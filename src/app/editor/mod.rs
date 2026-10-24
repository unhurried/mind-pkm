//! The page editor: a window per open Markdown file, showing the page as a
//! live preview - every block rendered, but the one with the cursor, which
//! shows its source in a text editor.
//!
//! `page` holds the page and its editing state, `keys` the commands and key
//! bindings, `highlight` the styling of the source, `view` the rendering.

mod highlight;
mod keys;
mod page;
mod render;
mod scroll;
mod view;

#[cfg(test)]
mod tests;

use super::{App, Message, settings};
use crate::markdown::count;
use crate::markdown::edit::{self, Sel};
use crate::markdown::format::{self, InlineStyle};
use iced::keyboard::{self, Key, Modifiers, key::Named};
use iced::widget::text_editor::{Action, Content, Edit, Motion};
use iced::widget::{operation, scrollable};
use iced::{Point, Size, Subscription, Task, window};
pub use keys::{Bind, EditorCommand};
use page::Page;
pub use render::{RenderCache, RenderKey};
use std::collections::{BTreeMap, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use view::HitInfo;

/// The size of the first editor window, when none was remembered.
pub const DEFAULT_WINDOW: Size = Size::new(900.0, 860.0);
pub const ZOOM_STEPS: &[f32] = &[0.7, 0.8, 0.9, 1.0, 1.1, 1.25, 1.5, 1.75, 2.0];

static NEXT_UID: AtomicU64 = AtomicU64::new(1);

/// The editor settings every editor window shares.
#[derive(Debug, Clone, PartialEq)]
pub struct EditorPrefs {
    /// Pages open in the built-in editor (else with the default application).
    pub builtin: bool,
    pub dark: bool,
    pub zoom: f32,
    pub auto_pair: bool,
    pub window: Size,
}

impl EditorPrefs {
    pub fn load() -> EditorPrefs {
        EditorPrefs {
            builtin: settings::editor_builtin(),
            dark: settings::editor_dark(),
            zoom: settings::editor_zoom().unwrap_or(1.0),
            auto_pair: settings::editor_auto_pair(),
            window: settings::editor_window().unwrap_or(DEFAULT_WINDOW),
        }
    }
}

impl Default for EditorPrefs {
    fn default() -> Self {
        EditorPrefs { builtin: true, dark: false, zoom: 1.0, auto_pair: true, window: DEFAULT_WINDOW }
    }
}

#[derive(Debug, Clone)]
pub enum EditorMsg {
    Key(keyboard::Event),
    Escape,
    Resized(Size),
    CloseRequested,
    FileDropped(PathBuf),
    Focused,
    /// An action of the active region's text editor.
    Action(Action),
    /// A key the editor binds itself.
    Bind(Bind),
    Command(EditorCommand),
    Menu(Option<usize>),
    /// The pointer moved over a piece of rendered text, to this position in it.
    PointerOver(Arc<HitInfo>, Point),
    /// A press on the piece of text the pointer is over.
    TextPressed,
    /// A press on a rendered block outside its text.
    BlockPressed(usize),
    /// A press on a cell of the table being edited.
    CellPressed(usize, usize),
    /// A press on a source offset of a rendered block (a link, a task box...).
    PressedAt(usize),
    /// A link clicked: its destination and where its text is in the source.
    Link(String, usize),
    TaskToggled(usize),
    /// A heading of the outline or a table of contents.
    HeadingClicked(usize),
    Scrolled(scrollable::Viewport),
    Dialog(DialogAnswer),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DialogAnswer {
    Ok,
    Save,
    Discard,
    Cancel,
}

/// A question or message shown over the page.
#[derive(Debug, Clone, PartialEq)]
pub enum EditorDialog {
    Message(String),
    /// Closing with unsaved changes.
    Unsaved,
    /// The file changed on disk while there are unsaved changes.
    ChangedOnDisk,
}

/// What an editor asks of the application.
#[derive(Debug, Clone, PartialEq)]
pub enum Effect {
    /// Open a page (a `.md` file) in the editor.
    OpenPage(PathBuf),
    /// Open a file, directory or URL with its default application.
    OpenExternal(String),
    Close,
    SetPrefs(EditorPrefs),
}

pub struct Editor {
    uid: u64,
    /// The editor's window, once it is open.
    pub window: Option<window::Id>,
    pub page: Page,
    menu: Option<usize>,
    pub dialogs: VecDeque<EditorDialog>,
    /// The whole page in a text editor, in source mode.
    source: Option<Content>,
    outline: bool,
    fullscreen: bool,
    status: String,
    modifiers: Modifiers,
    /// The rendered text under the pointer and the pointer's position in it.
    pointer: Option<(Arc<HitInfo>, Point)>,
    window_size: Size,
    /// How the document is scrolled, as last reported.
    viewport: Option<scrollable::Viewport>,
    /// Ctrl+A pressed again selects all blocks.
    select_all_again: bool,
}

impl Editor {
    pub fn open(path: &Path) -> Result<Editor, String> {
        Ok(Editor::new(Page::open(path)?))
    }

    pub fn new(page: Page) -> Editor {
        Editor {
            uid: NEXT_UID.fetch_add(1, Ordering::Relaxed),
            window: None,
            page,
            menu: None,
            dialogs: VecDeque::new(),
            source: None,
            outline: false,
            fullscreen: false,
            status: String::new(),
            modifiers: Modifiers::default(),
            pointer: None,
            window_size: DEFAULT_WINDOW,
            viewport: None,
            select_all_again: false,
        }
    }

    pub fn title(&self) -> String {
        let name = self.page.path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        format!("{name}{} - Mind PKM", if self.page.is_dirty() { " *" } else { "" })
    }

    pub(super) fn id(&self, part: &str) -> iced::widget::Id {
        iced::widget::Id::from(format!("editor-{}-{part}", self.uid))
    }

    /// Focuses the text editor of the active region (or the source).
    fn focus(&self) -> Task<EditorMsg> {
        operation::focus(self.id("active"))
    }

    pub fn is_source_mode(&self) -> bool {
        self.source.is_some()
    }

    pub fn update(&mut self, msg: EditorMsg, prefs: &EditorPrefs, effects: &mut Vec<Effect>) -> Task<EditorMsg> {
        // After typing and moving with keys, the cursor stays in view.
        let reveals =
            matches!(msg, EditorMsg::Action(_) | EditorMsg::Bind(_) | EditorMsg::Command(_) | EditorMsg::Key(_));
        let task = self.handle(msg, prefs, effects);
        if reveals { Task::batch([task, self.reveal(prefs)]) } else { task }
    }

    /// Scrolls the page to show the cursor.
    fn reveal(&self, prefs: &EditorPrefs) -> Task<EditorMsg> {
        let (before, kind) = match (&self.source, &self.page.active) {
            (Some(_), _) => (
                self.page.text[..self.page.sel.head.min(self.page.text.len())].to_string(),
                crate::markdown::BlockKind::Paragraph,
            ),
            (None, Some(a)) if a.table.is_none() => {
                let text = &self.page.text[a.region.clone()];
                let at = self.page.sel.head.clamp(a.region.start, a.region.end) - a.region.start;
                (text[..at].to_string(), a.kind.clone())
            }
            _ => return Task::none(),
        };
        let (size, font, padding) = view::region_text_style(&kind, prefs.zoom);
        let caret = scroll::CaretText { before, size, font, line_height: view::TEXT_LINE_HEIGHT, padding };
        scroll::reveal_caret(self.id("active-box"), self.id("doc"), caret)
    }

    fn handle(&mut self, msg: EditorMsg, prefs: &EditorPrefs, effects: &mut Vec<Effect>) -> Task<EditorMsg> {
        match msg {
            EditorMsg::Key(event) => self.on_key(event, prefs, effects),
            EditorMsg::Escape => self.on_escape(),
            EditorMsg::Resized(size) => {
                self.window_size = size;
                if !self.fullscreen && size.width >= 200.0 && size.height >= 200.0 {
                    effects.push(Effect::SetPrefs(EditorPrefs { window: size, ..prefs.clone() }));
                }
                Task::none()
            }
            EditorMsg::CloseRequested => self.close_requested(effects),
            EditorMsg::FileDropped(_) | EditorMsg::Focused => Task::none(),
            EditorMsg::Action(action) => self.on_action(action),
            EditorMsg::Bind(b) => self.on_bind(b, prefs),
            EditorMsg::Command(cmd) => {
                self.menu = None;
                self.run(cmd, prefs, effects)
            }
            EditorMsg::Menu(m) => {
                self.menu = m;
                Task::none()
            }
            EditorMsg::PointerOver(info, p) => {
                self.pointer = Some((info, p));
                Task::none()
            }
            EditorMsg::TextPressed => self.text_pressed(),
            EditorMsg::BlockPressed(block) => self.block_pressed(block),
            EditorMsg::CellPressed(r, c) => {
                self.page.go_to_cell((r, c));
                self.focus()
            }
            EditorMsg::PressedAt(offset) => {
                self.menu = None;
                self.page.place(Sel::caret(offset));
                self.focus()
            }
            EditorMsg::Link(dest, at) => self.link_clicked(&dest, at, effects),
            EditorMsg::TaskToggled(block) => {
                let start = self.page.doc.blocks.get(block).map(|b| b.range.start);
                if let Some(e) = start.and_then(|s| format::toggle_task(&self.page.text, s)) {
                    let sel = self.page.sel;
                    self.page.apply(e);
                    self.page.deactivate();
                    self.page.sel = sel;
                }
                Task::none()
            }
            EditorMsg::HeadingClicked(i) => self.go_to_heading(i),
            EditorMsg::Scrolled(viewport) => {
                self.viewport = Some(viewport);
                Task::none()
            }
            EditorMsg::Dialog(answer) => self.dialog_answered(answer, effects),
        }
    }

    // --- Keys ---

    /// A key no widget took: the editor's shortcuts, and moving into the
    /// page when no block is active.
    fn on_key(&mut self, event: keyboard::Event, prefs: &EditorPrefs, effects: &mut Vec<Effect>) -> Task<EditorMsg> {
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
        if !self.dialogs.is_empty() {
            return match key {
                Key::Named(Named::Enter) => self.dialog_answered(DialogAnswer::Ok, effects),
                _ => Task::none(),
            };
        }
        if let Some(cmd) = keys::command_for_key(&key, physical, modifiers) {
            self.menu = None;
            return self.run(cmd, prefs, effects);
        }
        if self.page.blocks.is_some() {
            return self.blocks_key(&key, modifiers, text.as_deref());
        }
        // Nothing active: arrow keys and typing start at the cursor.
        let starts = matches!(key, Key::Named(Named::ArrowUp | Named::ArrowDown | Named::Enter))
            || text.as_deref().is_some_and(|t| t.chars().any(|c| !c.is_control()));
        if starts && self.page.active.is_none() && self.source.is_none() {
            self.page.place(self.page.sel);
            return self.focus();
        }
        Task::none()
    }

    fn on_escape(&mut self) -> Task<EditorMsg> {
        if !self.dialogs.is_empty() {
            self.dialogs.pop_front();
        } else if self.menu.is_some() {
            self.menu = None;
        } else if self.page.blocks.is_some() {
            self.page.blocks = None;
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
        let Some((anchor, head)) = self.page.blocks else { return Task::none() };
        let last = self.page.doc.blocks.len().saturating_sub(1);
        match key {
            Key::Named(Named::ArrowUp | Named::ArrowDown) if m.shift() => {
                let up = *key == Key::Named(Named::ArrowUp);
                let head = if up { head.saturating_sub(1) } else { (head + 1).min(last) };
                self.page.blocks = Some((anchor, head));
                Task::none()
            }
            Key::Named(Named::ArrowUp) | Key::Named(Named::ArrowLeft) => {
                let first = anchor.min(head);
                self.page.blocks = None;
                self.page.enter_block(first, true, None);
                self.focus()
            }
            Key::Named(Named::ArrowDown) | Key::Named(Named::ArrowRight) => {
                let end = anchor.max(head);
                self.page.blocks = None;
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

    /// Removes the selected blocks; typing `with` puts a paragraph of it in their place.
    fn delete_blocks(&mut self, with: Option<char>) {
        let Some(range) = self.page.blocks_range() else { return };
        let text = &self.page.text;
        let insert = with.map(String::from).unwrap_or_default();
        // Without a replacement, the blank lines after the blocks go too.
        let end = if insert.is_empty() {
            range.end + (text.len() - range.end - text[range.end..].trim_start().len())
        } else {
            range.end
        };
        let end = if end == text.len() { range.end } else { end };
        let new = edit::splice(text, range.start..end, &insert);
        let at = range.start + insert.len();
        self.page.blocks = None;
        self.page.set_text(new, Sel::caret(at));
    }

    // --- The active region ---

    fn on_action(&mut self, action: Action) -> Task<EditorMsg> {
        self.menu = None;
        self.select_all_again = false;
        if let Some(source) = &mut self.source {
            let is_edit = action.is_edit();
            source.perform(action);
            if is_edit {
                let text = source.text();
                let sel = source_sel(source, &text);
                self.page.set_source_text(text, sel);
            }
            return Task::none();
        }
        if matches!(action, Action::Click(_)) {
            self.page.blocks = None;
        }
        self.page.perform(action);
        Task::none()
    }

    fn on_bind(&mut self, bind: Bind, prefs: &EditorPrefs) -> Task<EditorMsg> {
        self.menu = None;
        self.select_all_again = false;
        if self.source.is_some() {
            return self.source_bind(bind, prefs);
        }
        let Some(active) = &self.page.active else { return Task::none() };
        let region = active.region.clone();
        let in_table = active.table.is_some();
        if in_table {
            return self.table_bind(bind);
        }
        let text = self.page.text.clone();
        let sel = self.page.sel;
        let doc = &self.page.doc;
        let pending = self.page.pending.is_some();
        match bind {
            Bind::Enter => {
                if pending {
                    return Task::none();
                }
                match edit::enter(&text, doc, region, sel) {
                    Some(e) => self.page.apply(e),
                    None => self.page.perform(Action::Edit(Edit::Enter)),
                }
            }
            Bind::ShiftEnter => {
                if pending {
                    return Task::none();
                }
                match edit::shift_enter(&text, doc, region, sel) {
                    Some(e) => self.page.apply(e),
                    None => self.page.perform(Action::Edit(Edit::Enter)),
                }
            }
            Bind::Tab | Bind::ShiftTab => {
                if let Some(e) = edit::indent(&text, doc, region, sel, bind == Bind::ShiftTab) {
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
                if !sel.is_empty() {
                    self.page.perform(Action::Edit(Edit::Backspace));
                } else if let Some(e) = prefs.auto_pair.then(|| format::pair_backspace(&text, sel.head)).flatten() {
                    self.page.apply(e);
                } else if let Some(e) = edit::backspace(&text, doc, region, sel.head) {
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
                match (sel.is_empty()).then(|| edit::delete(&text, doc, region, sel.head)).flatten() {
                    Some(e) => self.page.apply(e),
                    None => self.page.perform(Action::Edit(Edit::Delete)),
                }
            }
            Bind::Up | Bind::Down => return self.vertical(bind == Bind::Down),
            Bind::ShiftUp | Bind::ShiftDown => return self.extend_vertically(bind == Bind::ShiftDown),
            Bind::Left | Bind::Right => return self.horizontal(bind == Bind::Right),
            Bind::Type(c) => {
                let code = self.page.active.as_ref().is_some_and(|a| a.kind.is_code());
                match prefs.auto_pair.then(|| format::auto_pair(&text, sel, c, code)).flatten() {
                    Some(e) if !pending => self.page.apply(e),
                    _ => self.page.perform(Action::Edit(Edit::Insert(c))),
                }
            }
        }
        self.focus()
    }

    /// Up / Down: within the region, or into the block before / after it.
    fn vertical(&mut self, down: bool) -> Task<EditorMsg> {
        let motion = if down { Motion::Down } else { Motion::Up };
        let before = self.page.active.as_ref().map(|a| a.content.cursor());
        if self.page.pending.is_none() {
            self.page.perform(Action::Move(motion));
        }
        let after = self.page.active.as_ref().map(|a| a.content.cursor());
        if self.page.pending.is_some() || before == after {
            let column = self.page.column();
            if let Some(b) = self.page.neighbor(down) {
                self.page.enter_block(b, down, Some(column));
            }
        }
        self.focus()
    }

    /// Shift+Up / Shift+Down: selects within the region, then whole blocks.
    fn extend_vertically(&mut self, down: bool) -> Task<EditorMsg> {
        let motion = if down { Motion::Down } else { Motion::Up };
        let before = self.page.active.as_ref().map(|a| a.content.cursor());
        if self.page.pending.is_none() {
            self.page.perform(Action::Select(motion));
        }
        let after = self.page.active.as_ref().map(|a| a.content.cursor());
        if self.page.pending.is_some() || before == after {
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
        let Some(a) = &self.page.active else { return Task::none() };
        let sel = self.page.sel;
        let at_edge = sel.is_empty() && if right { sel.head >= a.region.end } else { sel.head <= a.region.start };
        if at_edge || self.page.pending.is_some() {
            if let Some(b) = self.page.neighbor(right) {
                self.page.enter_block(b, right, None);
            }
        } else {
            self.page.perform(Action::Move(if right { Motion::Right } else { Motion::Left }));
        }
        self.focus()
    }

    /// A press on rendered text: the cursor goes where the text came from.
    fn text_pressed(&mut self) -> Task<EditorMsg> {
        let Some((info, p)) = self.pointer.clone() else { return Task::none() };
        if self.shift_selects(info.block) {
            return Task::none();
        }
        self.menu = None;
        let offset = view::hit_test(&info, p).or_else(|| self.page.doc.blocks.get(info.block).map(|b| b.range.start));
        if let Some(offset) = offset {
            self.page.place(Sel::caret(offset));
        }
        self.focus()
    }

    fn block_pressed(&mut self, block: usize) -> Task<EditorMsg> {
        self.menu = None;
        if self.shift_selects(block) {
            return Task::none();
        }
        if let Some(b) = self.page.doc.blocks.get(block) {
            // Beside a list item's text (on its bullet): where its text starts.
            let at = match b.kind {
                crate::markdown::BlockKind::ListItem(_) => {
                    let region = edit::region_at(&self.page.doc, b.range.start).unwrap_or(b.range.clone());
                    edit::content_start(&self.page.text, b.range.start).min(region.end)
                }
                _ => b.range.start,
            };
            self.page.place(Sel::caret(at));
        }
        self.focus()
    }

    /// Shift+click: selects the blocks from the active one to `block`.
    fn shift_selects(&mut self, block: usize) -> bool {
        if !self.modifiers.shift() {
            return false;
        }
        let Some(anchor) = self.page.active_block().or(self.page.blocks.map(|b| b.0)) else { return false };
        self.page.select_blocks(anchor, block);
        true
    }

    fn link_clicked(&mut self, dest: &str, at: usize, effects: &mut Vec<Effect>) -> Task<EditorMsg> {
        if !(self.modifiers.control() || self.modifiers.command()) {
            self.page.place(Sel::caret(at));
            return self.focus();
        }
        if let Some(anchor) = dest.strip_prefix('#') {
            if let Some(h) = self.page.doc.heading_for_anchor(anchor) {
                let index = self.page.doc.headings.iter().position(|x| x.block == h.block).unwrap_or(0);
                return self.go_to_heading(index);
            }
            return Task::none();
        }
        match resolve_link(&self.page.path, dest) {
            LinkTarget::Page(path) => effects.push(Effect::OpenPage(path)),
            LinkTarget::External(target) => effects.push(Effect::OpenExternal(target)),
        }
        Task::none()
    }

    fn go_to_heading(&mut self, index: usize) -> Task<EditorMsg> {
        let Some(h) = self.page.doc.headings.get(index) else { return Task::none() };
        let block = h.block;
        self.page.deactivate();
        self.page.sel = Sel::caret(self.page.doc.blocks[block].range.start);
        self.scroll_to_block(block)
    }

    // --- Commands ---

    fn run(&mut self, cmd: EditorCommand, prefs: &EditorPrefs, effects: &mut Vec<Effect>) -> Task<EditorMsg> {
        use EditorCommand as C;
        self.select_all_again &= cmd == C::SelectAll;
        match cmd {
            C::Save => self.save(),
            C::Close => self.close_requested(effects),
            C::Undo => {
                self.leave_source_for_history();
                if self.page.undo() {
                    self.refresh_source();
                }
                self.focus()
            }
            C::Redo => {
                self.leave_source_for_history();
                if self.page.redo() {
                    self.refresh_source();
                }
                self.focus()
            }
            C::SelectAll => self.select_all(),
            C::SelectWord => self.with_active(|p| {
                if let Some(w) = format::word_at(&p.text, p.sel.head) {
                    p.place(Sel { anchor: w.start, head: w.end });
                }
            }),
            C::SelectLine => self.with_active(|p| {
                let line = edit::line_range(&p.text, p.sel.head);
                p.place(Sel { anchor: line.start, head: line.end });
            }),
            C::SelectSpan => self.with_active(|p| {
                if let Some(r) = format::styled_span_at(&p.text, &p.doc, p.sel.head) {
                    p.place(Sel { anchor: r.start, head: r.end });
                }
            }),
            C::ClearFormatting => {
                self.edit_selection_or_region(|text, doc, range, _| Some(format::clear_formatting(text, doc, range)))
            }
            C::ToggleAutoPair => {
                effects.push(Effect::SetPrefs(EditorPrefs { auto_pair: !prefs.auto_pair, ..prefs.clone() }));
                Task::none()
            }
            C::Heading(n) => self.edit_region(|text, doc, region, sel| format::set_heading(text, doc, region, sel, n)),
            C::Paragraph => self.edit_region(|text, doc, region, sel| format::set_heading(text, doc, region, sel, 0)),
            C::HeadingUp => {
                self.edit_region(|text, doc, region, sel| format::step_heading(text, doc, region, sel, true))
            }
            C::HeadingDown => {
                self.edit_region(|text, doc, region, sel| format::step_heading(text, doc, region, sel, false))
            }
            C::Quote => self.edit_blocks(|text, _doc, range, sel| Some(format::toggle_quote(text, range, sel))),
            C::OrderedList => self.edit_blocks(|text, doc, range, _| Some(format::toggle_list(text, doc, range, true))),
            C::BulletList => self.edit_blocks(|text, doc, range, _| Some(format::toggle_list(text, doc, range, false))),
            C::CodeBlock => self.insert_block("```\n", "\n```"),
            C::MathBlock => self.insert_block("$$\n", "\n$$"),
            C::HorizontalRule => self.insert_block("---", ""),
            C::InsertTable => self.insert_block("|  |  |\n| --- | --- |\n|  |  |\n|  |  |", ""),
            C::Bold => self.toggle(InlineStyle::Bold),
            C::Italic => self.toggle(InlineStyle::Italic),
            C::Underline => self.toggle(InlineStyle::Underline),
            C::Code => self.toggle(InlineStyle::Code),
            C::Strikethrough => self.toggle(InlineStyle::Strike),
            C::Highlight => self.toggle(InlineStyle::Highlight),
            C::InlineMath => self.toggle(InlineStyle::Math),
            C::Link => self.edit_selection(format::insert_link),
            C::SourceMode => self.toggle_source_mode(),
            C::Outline => {
                self.outline = !self.outline;
                Task::none()
            }
            C::FullScreen => {
                self.fullscreen = !self.fullscreen;
                let mode = if self.fullscreen { window::Mode::Fullscreen } else { window::Mode::Windowed };
                self.window.map_or(Task::none(), |id| window::set_mode(id, mode))
            }
            C::ZoomIn | C::ZoomOut | C::ZoomReset => {
                let i = ZOOM_STEPS.iter().position(|&z| z >= prefs.zoom - 0.001).unwrap_or(3) as i32;
                let zoom = match cmd {
                    C::ZoomIn => ZOOM_STEPS[((i + 1) as usize).min(ZOOM_STEPS.len() - 1)],
                    C::ZoomOut => ZOOM_STEPS[(i - 1).max(0) as usize],
                    _ => 1.0,
                };
                effects.push(Effect::SetPrefs(EditorPrefs { zoom, ..prefs.clone() }));
                Task::none()
            }
            C::DarkTheme => {
                effects.push(Effect::SetPrefs(EditorPrefs { dark: !prefs.dark, ..prefs.clone() }));
                Task::none()
            }
            _ => Task::none(),
        }
    }

    pub fn save(&mut self) -> Task<EditorMsg> {
        match self.page.save() {
            Ok(()) => self.status = format!("saved {}", self.page.path.display()),
            Err(e) => self.dialogs.push_back(EditorDialog::Message(e)),
        }
        Task::none()
    }

    fn close_requested(&mut self, effects: &mut Vec<Effect>) -> Task<EditorMsg> {
        if self.page.is_dirty() {
            if !self.dialogs.contains(&EditorDialog::Unsaved) {
                self.dialogs.push_back(EditorDialog::Unsaved);
            }
        } else {
            effects.push(Effect::Close);
        }
        Task::none()
    }

    fn dialog_answered(&mut self, answer: DialogAnswer, effects: &mut Vec<Effect>) -> Task<EditorMsg> {
        let Some(dialog) = self.dialogs.front().cloned() else { return Task::none() };
        match (dialog, answer) {
            (EditorDialog::Unsaved, DialogAnswer::Ok) => return Task::none(),
            (EditorDialog::Unsaved, DialogAnswer::Save) => {
                self.dialogs.pop_front();
                match self.page.save() {
                    Ok(()) => effects.push(Effect::Close),
                    Err(e) => self.dialogs.push_back(EditorDialog::Message(e)),
                }
            }
            (EditorDialog::Unsaved, DialogAnswer::Discard) => {
                self.dialogs.pop_front();
                effects.push(Effect::Close);
            }
            (EditorDialog::ChangedOnDisk, DialogAnswer::Discard) => {
                self.dialogs.pop_front();
                if let Err(e) = self.page.reload() {
                    self.dialogs.push_back(EditorDialog::Message(e));
                }
                self.refresh_source();
            }
            (EditorDialog::ChangedOnDisk, _) => {
                self.dialogs.pop_front();
                // Keep the editor's text: it is what is saved next.
                self.page.modified = std::fs::metadata(&self.page.path).and_then(|m| m.modified()).ok();
            }
            _ => {
                self.dialogs.pop_front();
            }
        }
        Task::none()
    }

    /// A check on the file (on focus and every tick): reloads it when it
    /// changed on disk, or asks when there are unsaved changes.
    pub fn check_disk(&mut self) {
        if !self.page.changed_on_disk() || self.dialogs.contains(&EditorDialog::ChangedOnDisk) {
            return;
        }
        if self.page.is_dirty() {
            self.dialogs.push_back(EditorDialog::ChangedOnDisk);
        } else if let Err(e) = self.page.reload() {
            self.dialogs.push_back(EditorDialog::Message(e));
        } else {
            self.refresh_source();
        }
    }

    // --- Editing helpers ---

    /// Runs `f` with the page when a region is active.
    fn with_active(&mut self, f: impl FnOnce(&mut Page)) -> Task<EditorMsg> {
        if self.page.active.is_none() {
            self.page.place(self.page.sel);
        }
        f(&mut self.page);
        self.focus()
    }

    fn toggle(&mut self, style: InlineStyle) -> Task<EditorMsg> {
        self.edit_selection(|text, sel| format::toggle_inline(text, sel, style))
    }

    fn edit_selection(&mut self, f: impl FnOnce(&str, Sel) -> edit::Edited) -> Task<EditorMsg> {
        if self.source.is_some() {
            let e = f(&self.page.text, self.page.sel);
            self.page.set_text(e.text, e.sel);
            self.refresh_source();
            return self.focus();
        }
        if self.page.active.is_none() || self.page.pending.is_some() {
            return Task::none();
        }
        let e = f(&self.page.text, self.page.sel);
        self.page.apply(e);
        self.focus()
    }

    /// An edit of the active region's block.
    fn edit_region(
        &mut self,
        f: impl FnOnce(&str, &crate::markdown::Document, std::ops::Range<usize>, Sel) -> Option<edit::Edited>,
    ) -> Task<EditorMsg> {
        let Some(a) = &self.page.active else { return Task::none() };
        if self.page.pending.is_some() {
            return Task::none();
        }
        if let Some(e) = f(&self.page.text, &self.page.doc, a.region.clone(), self.page.sel) {
            self.page.apply(e);
        }
        self.focus()
    }

    /// An edit of the selected blocks, or the active region.
    fn edit_blocks(
        &mut self,
        f: impl FnOnce(&str, &crate::markdown::Document, std::ops::Range<usize>, Sel) -> Option<edit::Edited>,
    ) -> Task<EditorMsg> {
        let range = match (self.page.blocks_range(), &self.page.active) {
            (Some(r), _) => r,
            (None, Some(a)) if self.page.pending.is_none() => a.region.clone(),
            _ => return Task::none(),
        };
        self.page.blocks = None;
        if let Some(e) = f(&self.page.text, &self.page.doc, range, self.page.sel) {
            self.page.apply(e);
        }
        self.focus()
    }

    /// An edit of the selection, else of the active region.
    fn edit_selection_or_region(
        &mut self,
        f: impl FnOnce(&str, &crate::markdown::Document, std::ops::Range<usize>, Sel) -> Option<edit::Edited>,
    ) -> Task<EditorMsg> {
        let range = if !self.page.sel.is_empty() {
            self.page.sel.range()
        } else if let Some(r) = self.page.blocks_range() {
            r
        } else if let Some(a) = &self.page.active {
            a.region.clone()
        } else {
            return Task::none();
        };
        self.page.blocks = None;
        if let Some(e) = f(&self.page.text, &self.page.doc, range, self.page.sel) {
            self.page.apply(e);
        }
        self.focus()
    }

    /// Inserts a new block after the active one (or at the end): `open`, the
    /// cursor, `close`.
    fn insert_block(&mut self, open: &str, close: &str) -> Task<EditorMsg> {
        let text = &self.page.text;
        let at = match (&self.page.active, &self.page.pending) {
            (Some(_), Some(p)) => p.at,
            (Some(a), None) => a.region.end,
            _ => self.page.blocks_range().map_or(text.len(), |r| r.end),
        };
        let pending = edit::Pending::gap(text, at);
        let (new, start) = pending.materialize(text, &format!("{open}{close}"));
        let cursor = start + open.len();
        self.page.blocks = None;
        self.page.set_text(new, Sel::caret(cursor));
        self.focus()
    }

    fn select_all(&mut self) -> Task<EditorMsg> {
        if let Some(source) = &mut self.source {
            source.perform(Action::SelectAll);
            return Task::none();
        }
        let whole_region = self.page.active.as_ref().is_some_and(|a| {
            let s = self.page.sel.range();
            s.start <= a.region.start && s.end >= a.region.end && !a.region.is_empty()
        });
        if self.select_all_again || whole_region || self.page.active.is_none() {
            let last = self.page.doc.blocks.len().saturating_sub(1);
            self.page.select_blocks(0, last);
            self.select_all_again = false;
            return Task::none();
        }
        if let Some(a) = &self.page.active {
            let r = a.region.clone();
            if a.table.is_some() {
                self.page.select_blocks(self.page.active_block().unwrap_or(0), self.page.active_block().unwrap_or(0));
                return Task::none();
            }
            self.page.place(Sel { anchor: r.start, head: r.end });
        }
        self.select_all_again = true;
        self.focus()
    }

    // --- Source mode ---

    fn toggle_source_mode(&mut self) -> Task<EditorMsg> {
        match self.source.take() {
            Some(content) => {
                let text = content.text();
                let sel = source_sel(&content, &text);
                self.page.active = None;
                self.page.pending = None;
                self.page.place(sel);
            }
            None => {
                let sel = self.page.sel;
                self.page.deactivate();
                self.page.blocks = None;
                let mut content = Content::with_text(&self.page.text);
                move_source(&mut content, &self.page.text, sel);
                self.page.sel = sel;
                self.source = Some(content);
            }
        }
        self.focus()
    }

    /// The source mode's text editor shows the page's text again (after undo or a reload).
    fn refresh_source(&mut self) {
        if let Some(source) = &mut self.source {
            *source = Content::with_text(&self.page.text);
            move_source(source, &self.page.text, self.page.sel);
            self.page.active = None;
            self.page.pending = None;
        }
    }

    fn leave_source_for_history(&mut self) {
        if let Some(source) = &self.source {
            let text = source.text();
            if text != self.page.text {
                let sel = source_sel(source, &text);
                self.page.set_source_text(text, sel);
            }
        }
    }

    fn source_bind(&mut self, bind: Bind, prefs: &EditorPrefs) -> Task<EditorMsg> {
        let Some(source) = &mut self.source else { return Task::none() };
        let action = match bind {
            Bind::Enter | Bind::ShiftEnter => Action::Edit(Edit::Enter),
            Bind::Tab => Action::Edit(Edit::Insert('\t')),
            Bind::ShiftTab => return Task::none(),
            Bind::Backspace => Action::Edit(Edit::Backspace),
            Bind::Delete => Action::Edit(Edit::Delete),
            Bind::Up | Bind::ShiftUp => Action::Move(Motion::Up),
            Bind::Down | Bind::ShiftDown => Action::Move(Motion::Down),
            Bind::Left => Action::Move(Motion::Left),
            Bind::Right => Action::Move(Motion::Right),
            Bind::Type(c) => {
                let text = source.text();
                let sel = source_sel(source, &text);
                if let Some(e) = prefs.auto_pair.then(|| format::auto_pair(&text, sel, c, false)).flatten() {
                    self.page.set_source_text(e.text, e.sel);
                    self.refresh_source();
                    return Task::none();
                }
                Action::Edit(Edit::Insert(c))
            }
        };
        self.on_action(action)
    }

    // --- Table keys (filled in by the table editing) ---

    fn table_bind(&mut self, bind: Bind) -> Task<EditorMsg> {
        match bind {
            Bind::Tab | Bind::ShiftTab | Bind::Enter => {
                let Some(state) = self.page.active.as_ref().and_then(|a| a.table.as_ref()) else {
                    return Task::none();
                };
                let (r, c) = state.cell;
                let (rows, cols) = (state.table.rows.len(), state.table.columns());
                match bind {
                    Bind::Tab if r + 1 == rows && c + 1 == cols => self.page.change_table(|t, (r, _)| {
                        t.insert_row(r + 1);
                        (r + 1, 0)
                    }),
                    Bind::Tab => {
                        let next = if c + 1 < cols { (r, c + 1) } else { (r + 1, 0) };
                        self.page.go_to_cell(next);
                    }
                    Bind::ShiftTab => {
                        let prev = if c > 0 {
                            (r, c - 1)
                        } else if r > 0 {
                            (r - 1, cols - 1)
                        } else {
                            (0, 0)
                        };
                        self.page.go_to_cell(prev);
                    }
                    _ if r + 1 == rows => self.page.change_table(|t, (r, c)| {
                        t.insert_row(r + 1);
                        (r + 1, c)
                    }),
                    _ => self.page.go_to_cell((r + 1, c)),
                }
            }
            Bind::ShiftEnter => self.page.perform(Action::Edit(Edit::Enter)),
            Bind::Backspace => self.page.perform(Action::Edit(Edit::Backspace)),
            Bind::Delete => self.page.perform(Action::Edit(Edit::Delete)),
            Bind::Type(c) => self.page.perform(Action::Edit(Edit::Insert(c))),
            Bind::Up | Bind::Down | Bind::ShiftUp | Bind::ShiftDown => {
                let down = matches!(bind, Bind::Down | Bind::ShiftDown);
                let Some(state) = self.page.active.as_ref().and_then(|a| a.table.as_ref()) else {
                    return Task::none();
                };
                let (r, c) = state.cell;
                let rows = state.table.rows.len();
                if down && r + 1 < rows {
                    self.page.go_to_cell((r + 1, c));
                } else if !down && r > 0 {
                    self.page.go_to_cell((r - 1, c));
                } else if let Some(b) = self.page.neighbor(down) {
                    self.page.enter_block(b, down, None);
                }
            }
            Bind::Left | Bind::Right => {
                let right = bind == Bind::Right;
                self.page.perform(Action::Move(if right { Motion::Right } else { Motion::Left }));
            }
        }
        self.focus()
    }

    // --- Scrolling ---

    fn scroll_to_block(&self, block: usize) -> Task<EditorMsg> {
        scroll::to_top(self.id(&format!("block-{block}")), self.id("doc"))
    }

    /// Words and characters for the status bar.
    pub fn counts(&self) -> (count::Counts, Option<count::Counts>) {
        let all = count::count(&count::shown_text(&self.page.doc));
        let selected = self.page.selected_source();
        let sel = (!selected.is_empty()).then(|| count::count_source(&selected));
        (all, sel)
    }
}

/// The selection of the source mode's text editor, as page offsets.
fn source_sel(content: &Content, text: &str) -> Sel {
    let c = content.cursor();
    let head = page::offset_of(text, c.position);
    let anchor = c.selection.map_or(head, |p| page::offset_of(text, p));
    Sel { anchor, head }
}

fn move_source(content: &mut Content, text: &str, sel: Sel) {
    let position = page::position_of(text, sel.head);
    let selection = (!sel.is_empty()).then(|| page::position_of(text, sel.anchor));
    content.move_to(iced::widget::text_editor::Cursor { position, selection });
}

impl Page {
    /// Takes the source mode's text as the page's.
    pub fn set_source_text(&mut self, text: String, sel: Sel) {
        if text != self.text {
            self.set_text(text, sel);
            self.active = None;
            self.pending = None;
        }
        self.sel = sel;
    }
}

/// Where a link in a page leads.
#[derive(Debug, Clone, PartialEq)]
pub enum LinkTarget {
    /// A Markdown file, opened in the editor.
    Page(PathBuf),
    /// A file, directory or URL for the default application.
    External(String),
}

/// A link of the page at `page` resolved: relative paths against the page's directory.
pub fn resolve_link(page: &Path, dest: &str) -> LinkTarget {
    let dest = dest.trim();
    let is_url = dest.contains("://") || dest.starts_with("mailto:");
    if is_url && !dest.starts_with("file:") {
        return LinkTarget::External(dest.to_string());
    }
    let path = crate::markdown::images::local_path(page, dest, None);
    match path {
        Some(p) if p.extension().is_some_and(|e| e.eq_ignore_ascii_case("md")) => LinkTarget::Page(p),
        Some(p) => LinkTarget::External(p.display().to_string()),
        None => LinkTarget::External(dest.to_string()),
    }
}

/// Whether a file opens in the built-in editor (rather than its default application).
pub fn opens_in_editor(path: &Path, builtin: bool) -> bool {
    builtin && !path.is_dir() && path.extension().is_some_and(|e| e.eq_ignore_ascii_case("md"))
}

/// The subscription of the editor windows: nothing of their own yet.
pub fn subscription(_editors: &BTreeMap<window::Id, Editor>) -> Subscription<Message> {
    Subscription::none()
}

impl App {
    /// Handles a message of the editor window `id`.
    pub(super) fn editor_message(&mut self, id: window::Id, msg: EditorMsg) -> Task<Message> {
        let focused = matches!(msg, EditorMsg::Focused);
        let Some(editor) = self.editors.get_mut(&id) else { return Task::none() };
        if focused {
            editor.check_disk();
            return Task::none();
        }
        let mut effects = Vec::new();
        let task = editor.update(msg, &self.editor_prefs, &mut effects).map(move |m| Message::Editor(id, m));
        let mut tasks = vec![task, self.request_renders()];
        for effect in effects {
            tasks.push(self.editor_effect(id, effect));
        }
        Task::batch(tasks)
    }

    fn editor_effect(&mut self, id: window::Id, effect: Effect) -> Task<Message> {
        match effect {
            Effect::OpenPage(path) => self.open_page(&path),
            Effect::OpenExternal(target) => {
                if let Err(e) = opener::open(&target)
                    && let Some(editor) = self.editors.get_mut(&id)
                {
                    editor.dialogs.push_back(EditorDialog::Message(format!("failed to open {target}: {e}")));
                }
                Task::none()
            }
            Effect::Close => self.close_editor(id),
            Effect::SetPrefs(prefs) => {
                self.set_editor_prefs(prefs);
                Task::none()
            }
        }
    }

    pub(super) fn set_editor_prefs(&mut self, prefs: EditorPrefs) {
        let old = std::mem::replace(&mut self.editor_prefs, prefs.clone());
        if old.builtin != prefs.builtin {
            settings::set_editor_builtin(prefs.builtin);
        }
        if old.dark != prefs.dark {
            settings::set_editor_dark(prefs.dark);
        }
        if old.zoom != prefs.zoom {
            settings::set_editor_zoom(prefs.zoom);
        }
        if old.auto_pair != prefs.auto_pair {
            settings::set_editor_auto_pair(prefs.auto_pair);
        }
        if old.window != prefs.window {
            settings::set_editor_window(prefs.window);
        }
    }

    pub(super) fn close_editor(&mut self, id: window::Id) -> Task<Message> {
        self.editors.remove(&id);
        window::close(id)
    }

    /// Opens a file: a Markdown file in the built-in editor (bringing its
    /// window to the front when it is open already), anything else - and
    /// everything when the built-in editor is off - with its default application.
    pub(super) fn open_page(&mut self, path: &Path) -> Task<Message> {
        if !opens_in_editor(path, self.editor_prefs.builtin) {
            self.open_in_desktop(path);
            return Task::none();
        }
        self.open_in_editor(path)
    }

    /// Opens a Markdown file in an editor window.
    pub(super) fn open_in_editor(&mut self, path: &Path) -> Task<Message> {
        let key = crate::paths::key(path);
        if let Some((&id, _)) = self.editors.iter().find(|(_, e)| crate::paths::key(&e.page.path) == key) {
            return window::gain_focus(id);
        }
        match Editor::open(path) {
            Ok(mut editor) => {
                editor.window_size = self.editor_prefs.window;
                let (id, opened) = window::open(super::window_settings(self.editor_prefs.window, None, false));
                editor.window = Some(id);
                self.editors.insert(id, editor);
                Task::batch([opened.discard(), self.request_renders()])
            }
            Err(e) => {
                self.message(e);
                Task::none()
            }
        }
    }

    /// The editors of files at or under `path` (a page, or a renamed directory).
    pub(super) fn editors_of(&self, path: &Path) -> Vec<window::Id> {
        let key = crate::paths::key(path);
        self.editors
            .iter()
            .filter(|(_, e)| {
                let k = crate::paths::key(&e.page.path);
                k == key || k.starts_with(&key) && k[key.len()..].starts_with(std::path::MAIN_SEPARATOR)
            })
            .map(|(&id, _)| id)
            .collect()
    }

    /// Every tick: editors notice files changed on disk.
    pub(super) fn editors_tick(&mut self) -> Task<Message> {
        for editor in self.editors.values_mut() {
            editor.check_disk();
        }
        Task::none()
    }

    /// The names of the pages with unsaved changes.
    pub(super) fn unsaved_pages(&self) -> Vec<String> {
        self.editors
            .values()
            .filter(|e| e.page.is_dirty())
            .map(|e| e.page.path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default())
            .collect()
    }

    /// Saves every page with unsaved changes; the errors of those that fail.
    pub(super) fn save_all_pages(&mut self) -> Vec<String> {
        let mut errors = Vec::new();
        for editor in self.editors.values_mut() {
            if editor.page.is_dirty()
                && let Err(e) = editor.page.save()
            {
                errors.push(e);
            }
        }
        errors
    }
}
