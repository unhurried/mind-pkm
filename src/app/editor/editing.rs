//! The edits the commands and keys share: of the selection, of the active
//! region or of the selected blocks, inserting a new block, Select All, and
//! removing the selected blocks.

use super::page::{Page, content_sel};
use super::{Editor, EditorMsg};
use crate::markdown::edit::{self, Sel};
use crate::markdown::format::{self, InlineStyle};
use iced::Task;
use iced::widget::text_editor::Action;

impl Editor {
    /// Runs `f` with the page when a region is active.
    pub(super) fn with_active(&mut self, f: impl FnOnce(&mut Page)) -> Task<EditorMsg> {
        if self.page.active().is_none() {
            self.page.place(self.page.sel());
        }
        f(&mut self.page);
        self.focus()
    }

    pub(super) fn toggle(&mut self, style: InlineStyle) -> Task<EditorMsg> {
        self.edit_selection(|text, sel| format::toggle_inline(text, sel, style))
    }

    pub(super) fn edit_selection(&mut self, f: impl FnOnce(&str, Sel) -> edit::Edited) -> Task<EditorMsg> {
        if self.source.is_some() {
            let e = f(self.page.text(), self.page.sel());
            self.page.set_text(e.text, e.sel);
            self.refresh_source();
            return self.focus();
        }
        if self.page.active().is_none() || self.page.pending().is_some() {
            return Task::none();
        }
        let e = f(self.page.text(), self.page.sel());
        self.page.apply(e);
        self.focus()
    }

    /// An edit of the active region's block.
    pub(super) fn edit_region(
        &mut self,
        f: impl FnOnce(&str, &crate::markdown::Document, std::ops::Range<usize>, Sel) -> Option<edit::Edited>,
    ) -> Task<EditorMsg> {
        let Some(a) = self.page.active() else { return Task::none() };
        if self.page.pending().is_some() {
            return Task::none();
        }
        if let Some(e) = edit::nested(self.page.text(), self.page.doc(), a.region.clone(), self.page.sel(), f) {
            self.page.apply(e);
        }
        self.focus()
    }

    /// Move Item Up / Down: the list item at the cursor, also in the source mode.
    pub(super) fn move_item(&mut self, down: bool) -> Task<EditorMsg> {
        let rule = |t: &str, d: &crate::markdown::Document, r, s| edit::move_item(t, d, r, s, down);
        let Some(source) = &self.source else { return self.edit_region(rule) };
        let text = source.text();
        let sel = content_sel(source, &text);
        let doc = crate::markdown::Document::parse(&text);
        if let Some(e) = edit::region_at(&doc, sel.head).and_then(|region| edit::nested(&text, &doc, region, sel, rule))
        {
            self.page.set_text(e.text, e.sel);
            self.refresh_source();
        }
        self.focus()
    }

    /// An edit of the selected blocks, or the active region (inside their
    /// containers when `nested`).
    pub(super) fn edit_blocks(
        &mut self,
        nested: bool,
        f: impl FnOnce(&str, &crate::markdown::Document, std::ops::Range<usize>, Sel) -> Option<edit::Edited>,
    ) -> Task<EditorMsg> {
        let range = match (self.page.blocks_range(), self.page.active()) {
            (Some(r), _) => r,
            (None, Some(a)) if self.page.pending().is_none() => a.region.clone(),
            _ => return Task::none(),
        };
        self.page.deselect_blocks();
        let (text, doc, sel) = (self.page.text(), self.page.doc(), self.page.sel());
        let e = if nested { edit::nested_blocks(text, doc, range, sel, f) } else { f(text, doc, range, sel) };
        if let Some(e) = e {
            self.page.apply(e);
        }
        self.focus()
    }

    /// An edit of the selection, else of the active region.
    pub(super) fn edit_selection_or_region(
        &mut self,
        f: impl FnOnce(&str, &crate::markdown::Document, std::ops::Range<usize>, Sel) -> Option<edit::Edited>,
    ) -> Task<EditorMsg> {
        let range = if !self.page.sel().is_empty() {
            self.page.sel().range()
        } else if let Some(r) = self.page.blocks_range() {
            r
        } else if let Some(a) = self.page.active() {
            a.region.clone()
        } else {
            return Task::none();
        };
        self.page.deselect_blocks();
        if let Some(e) = f(self.page.text(), self.page.doc(), range, self.page.sel()) {
            self.page.apply(e);
        }
        self.focus()
    }

    /// Inserts a new block after the active one (or at the end): `open`, the
    /// cursor, `close`.
    pub(super) fn insert_block(&mut self, open: &str, close: &str) -> Task<EditorMsg> {
        self.insert_block_with_cursor(&format!("{open}{close}"), open.len())
    }

    /// Inserts `block` as a new block after the active one (or at the end),
    /// with the cursor `cursor` bytes into it.
    pub(super) fn insert_block_with_cursor(&mut self, block: &str, cursor: usize) -> Task<EditorMsg> {
        let text = self.page.text();
        let at = match (self.page.active(), self.page.pending()) {
            (Some(_), Some(p)) => p.at,
            (Some(a), None) => a.region.end,
            _ => self.page.blocks_range().map_or(text.len(), |r| r.end),
        };
        // Below a list item's text: inside the item.
        let in_item = match (self.page.active(), self.page.pending()) {
            (Some(a), None) if matches!(a.kind, crate::markdown::BlockKind::ListItem(_)) => {
                edit::nested(text, self.page.doc(), a.region.clone(), self.page.sel(), edit::block_below_item)
                    .and_then(|e| e.pending)
            }
            _ => None,
        };
        let pending = match (in_item, self.page.pending()) {
            (Some(p), _) => p,
            (None, Some(p)) if self.page.active().is_some() => p.clone(),
            _ => edit::gap_pending(text, self.page.doc(), at, None),
        };
        let (new, start) = pending.materialize(text, block);
        let cursor = start + pending.prefixed(&block[..cursor]).len();
        self.page.deselect_blocks();
        self.page.apply(edit::Edited { text: new, sel: Sel::caret(cursor), pending: None });
        self.focus()
    }

    pub(super) fn select_all(&mut self) -> Task<EditorMsg> {
        if let Some(source) = &mut self.source {
            source.perform(Action::SelectAll);
            return Task::none();
        }
        let whole_region = self.page.active().is_some_and(|a| {
            let s = self.page.sel().range();
            s.start <= a.region.start && s.end >= a.region.end && !a.region.is_empty()
        });
        if self.select_all_again || whole_region || self.page.active().is_none() {
            let last = self.page.doc().blocks.len().saturating_sub(1);
            self.page.select_blocks(0, last);
            self.select_all_again = false;
            return Task::none();
        }
        if let Some(a) = self.page.active() {
            let r = a.region.clone();
            if a.table().is_some() {
                self.page.select_blocks(self.page.active_block().unwrap_or(0), self.page.active_block().unwrap_or(0));
                return Task::none();
            }
            self.page.place(Sel { anchor: r.start, head: r.end });
        }
        self.select_all_again = true;
        self.focus()
    }

    /// Removes the selected blocks; typing `with` puts a paragraph of it in their place.
    pub(super) fn delete_blocks(&mut self, with: Option<char>) {
        let Some(range) = self.page.blocks_range() else { return };
        let text = self.page.text();
        let insert = with.map(String::from).unwrap_or_default();
        let (new, at) = if !insert.is_empty() {
            (edit::splice(text, range.clone(), &insert), range.start + insert.len())
        } else if text[range.end..].trim().is_empty() {
            // The last blocks: the blank lines before them go too.
            let before = text[..range.start].trim_end().len();
            let end = if before == 0 { "" } else { "\n" };
            (format!("{}{end}", &text[..before]), before)
        } else {
            // The blank lines after the blocks go with them.
            let end = range.end + (text.len() - range.end - text[range.end..].trim_start().len());
            (edit::splice(text, range.start..end, ""), range.start)
        };
        self.page.deselect_blocks();
        self.page.apply(edit::Edited { text: new, sel: Sel::caret(at), pending: None });
    }
}
