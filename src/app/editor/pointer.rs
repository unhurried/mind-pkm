//! Presses on the page: where a press on rendered text or beside it puts the
//! cursor, Shift+click selecting whole blocks, the button held for a drag,
//! right clicks opening the page's menu, and clicked links and headings.

use super::page::{content_sel, move_content};
use super::{Editor, EditorMsg, Effect, LinkTarget, RightClick, press_relay, resolve_link, view};
use crate::markdown::edit::{self, Sel};
use iced::Task;
use iced::widget::text_editor::Action;

impl Editor {
    /// A press on rendered text: the cursor goes where the text came from.
    pub(super) fn text_pressed(&mut self) -> Task<EditorMsg> {
        let Some((info, _)) = self.pointer.clone() else { return Task::none() };
        if self.shift_selects(info.block) {
            return Task::none();
        }
        self.menu = None;
        if let Some(offset) = self.pointer_offset() {
            self.page.place(Sel::caret(offset));
        }
        self.focus()
    }

    /// The source offset of the rendered text under the pointer (the start of
    /// its block when the text does not say).
    fn pointer_offset(&self) -> Option<usize> {
        let (info, p) = self.pointer.as_ref()?;
        view::hit_test(info, *p).or_else(|| self.page.doc().blocks.get(info.block).map(|b| b.range.start))
    }

    /// Whether the button is down from a press on the page.
    pub(in crate::app) fn pressed(&self) -> bool {
        self.block_drag.is_some()
    }

    /// The press the button is held from, for a text editor (with this
    /// padding) that replaced the block pressed.
    pub(in crate::app) fn held(&self, padding: iced::Padding) -> Option<press_relay::Held> {
        self.press_at.filter(|_| self.pressed()).map(|from| press_relay::Held { from, padding })
    }

    pub(super) fn block_pressed(&mut self, block: usize) -> Task<EditorMsg> {
        self.menu = None;
        if self.shift_selects(block) {
            return Task::none();
        }
        if let Some(at) = self.block_press_offset(block) {
            self.page.place(Sel::caret(at));
        }
        self.focus()
    }

    /// Where a press on a block outside its text puts the cursor: at its
    /// start, or beside a list item's text (on its bullet) where its text starts.
    fn block_press_offset(&self, block: usize) -> Option<usize> {
        let b = self.page.doc().blocks.get(block)?;
        Some(match b.kind {
            crate::markdown::BlockKind::ListItem(_) => {
                let region = edit::region_at(self.page.doc(), b.range.start).unwrap_or(b.range.clone());
                edit::content_start(self.page.text(), b.range.start).min(region.end)
            }
            _ => b.range.start,
        })
    }

    /// A right click on the page: the cursor goes where a click there would,
    /// unless that is inside the selection (text or whole blocks), and the
    /// page's menu opens there - the cell's menu in a table.
    pub(super) fn right_clicked(&mut self, target: RightClick) -> Task<EditorMsg> {
        self.menu = None;
        self.table_ui.menu = None;
        if let Some(pane) = &mut self.assets {
            pane.menu = None;
        }
        self.select_all_again = false;
        let blocks = self.page.selected_blocks().map(|(a, b)| a.min(b)..=a.max(b));
        let in_blocks = |block: usize| blocks.as_ref().is_some_and(|r| r.contains(&block));
        let sel = self.page.sel();
        let inside = |at: usize| !sel.is_empty() && sel.range().start <= at && at <= sel.range().end;
        let place = |editor: &mut Editor, at: Option<usize>| {
            if let Some(at) = at.filter(|&at| !inside(at)) {
                editor.page.place(Sel::caret(at));
            }
        };
        match target {
            RightClick::Text => {
                let block = self.pointer.as_ref().map(|(info, _)| info.block);
                if !block.is_some_and(in_blocks) {
                    place(self, self.pointer_offset());
                }
            }
            RightClick::Block(block) => {
                if !in_blocks(block) {
                    place(self, self.block_press_offset(block));
                }
            }
            RightClick::At(at) => place(self, Some(at)),
            RightClick::TableCell(block, r, c) => {
                let _ = self.table_cell_pressed(block, (r, c));
            }
            RightClick::Live(at) => self.page.live_right_click(at),
            RightClick::Editor(p) => match &mut self.source {
                Some(source) => {
                    let text = source.text();
                    let before = content_sel(source, &text);
                    source.perform(Action::Click(p));
                    let head = content_sel(source, &text).head;
                    if !before.is_empty() && before.range().start <= head && head <= before.range().end {
                        move_content(source, &text, before);
                    }
                }
                None => {
                    self.page.perform(Action::Click(p));
                    if inside(self.page.sel().head) {
                        self.page.place(sel);
                    }
                }
            },
            RightClick::Page => {}
        }
        let at = self.right_pressed_at;
        if self.source.is_none() && self.page.active().is_some_and(|a| a.table().is_some()) {
            self.table_ui.menu = Some(at);
        } else {
            self.context_menu = Some(at);
        }
        self.focus()
    }

    /// Shift+click: selects the blocks from the active one to `block`.
    pub(super) fn shift_selects(&mut self, block: usize) -> bool {
        if !self.modifiers.shift() {
            return false;
        }
        let Some(anchor) = self.page.active_block().or(self.page.selected_blocks().map(|b| b.0)) else { return false };
        self.page.select_blocks(anchor, block);
        true
    }

    pub(super) fn link_clicked(&mut self, dest: &str, at: usize, effects: &mut Vec<Effect>) -> Task<EditorMsg> {
        // A footnote reference goes to its definition.
        if let Some(label) = dest.strip_prefix('^') {
            let block = self
                .page
                .doc()
                .blocks
                .iter()
                .position(|b| b.kind == crate::markdown::BlockKind::Footnote(label.to_string()));
            return block.map_or(Task::none(), |b| {
                self.page.deactivate();
                self.scroll_to_block(b)
            });
        }
        if !(self.modifiers.control() || self.modifiers.command()) {
            self.page.place(Sel::caret(at));
            return self.focus();
        }
        if let Some(anchor) = dest.strip_prefix('#') {
            if let Some(h) = self.page.doc().heading_for_anchor(anchor) {
                let index = self.page.doc().headings.iter().position(|x| x.block == h.block).unwrap_or(0);
                return self.go_to_heading(index);
            }
            return Task::none();
        }
        match resolve_link(self.page.path(), dest) {
            LinkTarget::Page(path) => effects.push(Effect::OpenPage(path)),
            LinkTarget::External(target) => effects.push(Effect::OpenExternal(target)),
        }
        Task::none()
    }

    pub(super) fn go_to_heading(&mut self, index: usize) -> Task<EditorMsg> {
        let Some(h) = self.page.doc().headings.get(index) else { return Task::none() };
        let block = h.block;
        self.page.leave_at(Sel::caret(self.page.doc().blocks[block].range.start));
        self.scroll_to_block(block)
    }
}
