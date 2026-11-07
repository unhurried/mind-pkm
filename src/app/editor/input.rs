//! Keys: the editor's shortcuts and Escape, keys while whole blocks are
//! selected, the key bindings of the active region (Enter, Backspace, the
//! arrows... by the editing rules), the text editor's own actions, and the
//! messages of the widget editing a block in its rendered form.

use super::keys::{self, Bind};
use super::live_text::LiveMsg;
use super::page::content_sel;
use super::{Editor, EditorMsg, EditorPrefs, Effect};
use crate::markdown::edit::{self, Sel};
use crate::markdown::format;
use iced::keyboard::{self, Key, Modifiers, key::Named};
use iced::widget::text_editor::{Action, Edit, Motion};
use iced::{Task, window};

impl Editor {
    /// A key no widget took: the editor's shortcuts, and moving into the
    /// page when no block is active.
    pub(super) fn on_key(
        &mut self,
        event: keyboard::Event,
        prefs: &EditorPrefs,
        effects: &mut Vec<Effect>,
    ) -> Task<EditorMsg> {
        let (key, physical, modifiers, text) = match event {
            keyboard::Event::ModifiersChanged(m) => {
                self.modifiers = m;
                return Task::none();
            }
            keyboard::Event::KeyPressed { key, physical_key, modifiers, text, .. } => {
                (key, physical_key, modifiers, text)
            }
            _ => return Task::none(),
        };
        self.modifiers = modifiers;
        if let Some(dialog) = self.dialogs.front() {
            return match key {
                Key::Named(Named::Enter) => self.dialog_answered(dialog.default_answer(), effects),
                _ => Task::none(),
            };
        }
        if let Some(cmd) = keys::command_for_key(&key, physical, modifiers) {
            self.menu = None;
            return self.run(cmd, prefs, effects);
        }
        if self.page.selected_blocks().is_some() {
            return self.blocks_key(&key, modifiers, text.as_deref());
        }
        // Nothing active: arrow keys and typing start at the cursor, and
        // what was typed goes in.
        let typed: Option<String> = text
            .as_deref()
            .map(|t| t.chars().filter(|c| !c.is_control()).collect::<String>())
            .filter(|t| !t.is_empty() && !(modifiers.control() || modifiers.alt()));
        let starts =
            matches!(key, Key::Named(Named::ArrowUp | Named::ArrowDown | Named::Enter | Named::Home | Named::End))
                || typed.is_some();
        if starts && self.page.active().is_none() && self.source.is_none() {
            self.page.place(self.page.sel());
            self.page.snap_to_shown();
            match key {
                Key::Named(Named::Home) => self.page.perform(Action::Move(Motion::Home)),
                Key::Named(Named::End) => self.page.perform(Action::Move(Motion::End)),
                _ => {}
            }
            for c in typed.iter().flat_map(|t| t.chars()) {
                let _ = self.on_action(Action::Edit(Edit::Insert(c)));
            }
            return self.focus();
        }
        Task::none()
    }

    /// Whether a menu of the window is open: the menu bar's, a right-click
    /// menu (the page's, a cell's) or an item's of the assets pane.
    pub(super) fn menu_open(&self) -> bool {
        self.menu.is_some()
            || self.context_menu.is_some()
            || self.table_ui.menu.is_some()
            || self.assets.as_ref().is_some_and(|a| a.menu.is_some())
    }

    pub(super) fn on_escape(&mut self) -> Task<EditorMsg> {
        if !self.dialogs.is_empty() {
            self.dialogs.pop_front();
        } else if self.menu_open() {
            self.menu = None;
            self.context_menu = None;
            self.table_ui.menu = None;
            if let Some(pane) = &mut self.assets {
                pane.menu = None;
            }
        } else if self.find.is_some() {
            return self.close_find();
        } else if self.page.selected_blocks().is_some() {
            self.page.deselect_blocks();
        } else if self.fullscreen {
            self.fullscreen = false;
            if let Some(id) = self.window {
                return window::set_mode(id, window::Mode::Windowed);
            }
        }
        Task::none()
    }

    /// Keys while whole blocks are selected.
    fn blocks_key(&mut self, key: &Key, m: Modifiers, text: Option<&str>) -> Task<EditorMsg> {
        let Some((anchor, head)) = self.page.selected_blocks() else { return Task::none() };
        let last = self.page.doc().blocks.len().saturating_sub(1);
        match key {
            Key::Named(Named::ArrowUp | Named::ArrowDown) if m.shift() => {
                let up = *key == Key::Named(Named::ArrowUp);
                let head = if up { head.saturating_sub(1) } else { (head + 1).min(last) };
                self.page.set_blocks_head(head);
                Task::none()
            }
            Key::Named(Named::ArrowUp) | Key::Named(Named::ArrowLeft) => {
                let first = anchor.min(head);
                self.page.deselect_blocks();
                self.page.enter_block(first, true, None);
                self.focus()
            }
            Key::Named(Named::ArrowDown) | Key::Named(Named::ArrowRight) => {
                let end = anchor.max(head);
                self.page.deselect_blocks();
                self.page.enter_block(end, false, None);
                self.focus()
            }
            Key::Named(Named::Delete | Named::Backspace) => {
                self.delete_blocks(None);
                self.focus()
            }
            _ => match text.and_then(|t| t.chars().find(|c| !c.is_control())) {
                Some(c) if !(m.control() || m.alt()) => {
                    self.delete_blocks(Some(c));
                    self.focus()
                }
                _ => Task::none(),
            },
        }
    }

    pub(super) fn on_action(&mut self, action: Action) -> Task<EditorMsg> {
        self.menu = None;
        self.select_all_again = false;
        if let Some(source) = &mut self.source {
            let is_edit = action.is_edit();
            source.perform(action);
            if is_edit {
                let text = source.text();
                let sel = content_sel(source, &text);
                self.page.set_source_text(text, sel);
            }
            return Task::none();
        }
        if matches!(action, Action::Click(_)) {
            self.page.deselect_blocks();
        }
        self.page.perform(action);
        Task::none()
    }

    pub(super) fn on_bind(&mut self, bind: Bind, prefs: &EditorPrefs) -> Task<EditorMsg> {
        self.menu = None;
        self.select_all_again = false;
        if self.source.is_some() {
            return self.source_bind(bind, prefs);
        }
        let Some(active) = self.page.active() else { return Task::none() };
        let region = active.region.clone();
        let in_table = active.table().is_some();
        if in_table {
            return self.table_bind(bind);
        }
        let text = self.page.text().to_string();
        // The rules work on the source: at the start / end of what a block
        // shows, that is its content's start / end.
        let sel = self.page.rule_sel();
        let doc = self.page.doc();
        let pending = self.page.pending().is_some();
        match bind {
            Bind::Enter => {
                if pending {
                    // An empty new paragraph in a quote or list item leaves it.
                    self.page.leave_pending_container();
                    return self.focus();
                }
                match edit::nested(&text, doc, region, sel, edit::enter) {
                    Some(e) => self.page.apply(e),
                    None => self.page.perform(Action::Edit(Edit::Enter)),
                }
            }
            Bind::ShiftEnter => {
                if pending {
                    return Task::none();
                }
                match edit::nested(&text, doc, region, sel, edit::shift_enter) {
                    Some(e) => self.page.apply(e),
                    None => self.page.perform(Action::Edit(Edit::Enter)),
                }
            }
            Bind::Tab | Bind::ShiftTab => {
                let outdent = bind == Bind::ShiftTab;
                if let Some(e) = edit::nested(&text, doc, region, sel, |t, d, r, s| edit::indent(t, d, r, s, outdent)) {
                    self.page.apply(e);
                }
            }
            Bind::Backspace => {
                if pending {
                    // An empty new paragraph goes; the cursor to the end of the block before.
                    let before = self.page.neighbor(false);
                    self.page.deactivate();
                    match before {
                        Some(b) => self.page.enter_block(b, false, None),
                        None => self.page.place(Sel::caret(0)),
                    }
                    return self.focus();
                }
                let code = self.page.active().and_then(|a| a.code());
                if let Some(code) = code.filter(|c| sel.is_empty() && c.to_body(sel.head) == 0) {
                    // At the start of the code: an empty block goes, else to the block before.
                    if code.text().is_empty() {
                        self.page.remove_empty_code();
                    } else if let Some(e) = edit::backspace_nested(&text, doc, region.clone(), region.start) {
                        self.page.apply(e);
                    }
                } else if !sel.is_empty() {
                    self.page.perform(Action::Edit(Edit::Backspace));
                } else if let Some(e) = prefs.auto_pair.then(|| format::pair_backspace(&text, sel.head)).flatten() {
                    self.page.apply(e);
                } else if let Some(e) = edit::backspace_nested(&text, doc, region, sel.head) {
                    self.page.apply(e);
                } else {
                    self.page.perform(Action::Edit(Edit::Backspace));
                }
            }
            Bind::Delete => {
                if pending {
                    let after = self.page.neighbor(true);
                    self.page.deactivate();
                    if let Some(b) = after {
                        self.page.enter_block(b, true, None);
                    }
                    return self.focus();
                }
                let delete = |t: &str, d: &crate::markdown::Document, r, s: Sel| edit::delete(t, d, r, s.head);
                match (sel.is_empty()).then(|| edit::nested(&text, doc, region, sel, delete)).flatten() {
                    Some(e) => self.page.apply(e),
                    None => self.page.perform(Action::Edit(Edit::Delete)),
                }
            }
            Bind::Up | Bind::Down => return self.vertical(bind == Bind::Down),
            Bind::ShiftUp | Bind::ShiftDown => return self.extend_vertically(bind == Bind::ShiftDown),
            Bind::Left | Bind::Right => return self.horizontal(bind == Bind::Right),
            Bind::ShiftLeft | Bind::ShiftRight => {
                self.page.perform(Action::Select(if bind == Bind::ShiftRight { Motion::Right } else { Motion::Left }));
            }
            Bind::Type(c) => {
                let code = self.page.active().is_some_and(|a| a.kind.is_code());
                match prefs.auto_pair.then(|| format::auto_pair(&text, sel, c, code)).flatten() {
                    Some(e) if !pending => self.page.apply_typed(e, c),
                    _ => self.page.perform(Action::Edit(Edit::Insert(c))),
                }
            }
        }
        self.focus()
    }

    /// Up / Down: within the region, or into the block before / after it.
    fn vertical(&mut self, down: bool) -> Task<EditorMsg> {
        let motion = if down { Motion::Down } else { Motion::Up };
        let before = self.page.cursor_token();
        if self.page.pending().is_none() {
            self.page.perform(Action::Move(motion));
        }
        let after = self.page.cursor_token();
        if self.page.pending().is_some() || before == after {
            let column = self.page.column();
            match self.page.neighbor(down) {
                // Below code ending a quote or list item: a new paragraph in it.
                Some(_) if down && self.page.pending().is_none() && self.page.ends_container() => {
                    self.page.new_paragraph_after()
                }
                Some(b) => self.page.enter_block(b, down, Some(column)),
                // Below the last block, when it is code: a new paragraph.
                None if down && self.page.pending().is_none() => self.page.new_paragraph_after(),
                None => {}
            }
        }
        self.focus()
    }

    /// Shift+Up / Shift+Down: selects within the region, then whole blocks.
    fn extend_vertically(&mut self, down: bool) -> Task<EditorMsg> {
        let motion = if down { Motion::Down } else { Motion::Up };
        let before = self.page.cursor_token();
        if self.page.pending().is_none() {
            self.page.perform(Action::Select(motion));
        }
        let after = self.page.cursor_token();
        if self.page.pending().is_some() || before == after {
            let anchor = self.page.active_block().or_else(|| self.page.neighbor(down));
            if let Some(anchor) = anchor {
                let head = self.page.neighbor(down).unwrap_or(anchor);
                self.page.select_blocks(anchor, head);
            }
        }
        Task::none()
    }

    /// Left at the start / Right at the end of the region move into the block before / after.
    fn horizontal(&mut self, right: bool) -> Task<EditorMsg> {
        if self.page.active().is_none() {
            return Task::none();
        }
        if self.page.at_edge(right) || self.page.pending().is_some() {
            match self.page.neighbor(right) {
                Some(b) => self.page.enter_block(b, right, None),
                None if right && self.page.pending().is_none() => self.page.new_paragraph_after(),
                None => {}
            }
        } else {
            self.page.perform(Action::Move(if right { Motion::Right } else { Motion::Left }));
        }
        self.focus()
    }

    /// A message of the widget editing the active block in its rendered form.
    pub(super) fn live_message(&mut self, msg: LiveMsg) -> Task<EditorMsg> {
        self.menu = None;
        self.select_all_again = false;
        let page = &mut self.page;
        match msg {
            LiveMsg::Click { at, extend } => page.live_move_to(at, extend),
            LiveMsg::Drag(at) => page.live_move_to(at, true),
            LiveMsg::MoveTo { at, select } => page.live_move_to(at, select),
            LiveMsg::SelectWord(at) => page.live_select_word(at),
            LiveMsg::SelectAll => page.live_select_shown(None),
            LiveMsg::Motion { motion, select } => {
                page.perform(if select { Action::Select(motion) } else { Action::Move(motion) })
            }
            LiveMsg::Insert(text) => {
                let mut chars = text.chars();
                let typed = chars.next().filter(|_| chars.next().is_none());
                page.live_insert(&text, typed);
            }
        }
        // An edit can put the block elsewhere among the widgets (a new
        // paragraph getting its text): the focus follows it.
        self.focus()
    }
}
