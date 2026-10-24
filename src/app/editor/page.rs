//! An open page: its text, the active region (the source shown in the text
//! editor where the cursor is), the selection and the undo history.
//!
//! The text is the truth. The active region's `Content` holds a copy of its
//! part of the text; every edit there is spliced back right away and the page
//! is parsed again. Rules that change more (Enter, joining blocks, formatting)
//! return a new text and selection, after which the region is derived anew.

use crate::markdown::edit::{self, Edited, Pending, Sel};
use crate::markdown::history::{History, Snapshot};
use crate::markdown::page::PageText;
use crate::markdown::table::{self, CellRange, Table};
use crate::markdown::{BlockKind, Document};
use iced::widget::text_editor::{Action, Content, Cursor, Edit, Position};
use std::ops::Range;
use std::path::{Path, PathBuf};
use std::time::{Instant, SystemTime};

/// The table being edited, cell by cell, while the cursor is in it.
#[derive(Debug, Clone)]
pub struct TableState {
    pub table: Table,
    /// The cell the text editor holds.
    pub cell: (usize, usize),
    /// A rectangle of selected cells.
    pub range: Option<CellRange>,
}

pub struct Active {
    /// Its bytes in the page text.
    pub region: Range<usize>,
    /// The region's text, or the table cell's text for a table.
    pub content: Content,
    /// The kind of the region parsed on its own (how its source is presented).
    pub kind: BlockKind,
    pub table: Option<TableState>,
}

impl Active {
    fn new(region: Range<usize>, text: &str) -> Active {
        Active { region, content: Content::with_text(text), kind: own_kind(text), table: None }
    }
}

/// The kind of a piece of source parsed on its own.
pub fn own_kind(source: &str) -> BlockKind {
    Document::parse(source).blocks.first().map_or(BlockKind::Paragraph, |b| b.kind.clone())
}

/// The byte offset of a text editor position in its text.
pub fn offset_of(text: &str, p: Position) -> usize {
    let mut offset = 0;
    for (i, line) in text.split('\n').enumerate() {
        if i == p.line {
            return offset + p.column.min(line.len());
        }
        offset += line.len() + 1;
    }
    text.len()
}

/// The text editor position of a byte offset of its text.
pub fn position_of(text: &str, offset: usize) -> Position {
    let offset = offset.min(text.len());
    let line = text[..offset].matches('\n').count();
    let start = text[..offset].rfind('\n').map_or(0, |i| i + 1);
    Position { line, column: offset - start }
}

/// The selection of a text editor's content, as offsets in its text.
fn content_sel(content: &Content, text: &str) -> Sel {
    let c = content.cursor();
    let head = offset_of(text, c.position);
    let anchor = c.selection.map_or(head, |p| offset_of(text, p));
    Sel { anchor, head }
}

fn move_content(content: &mut Content, text: &str, sel: Sel) {
    let position = position_of(text, sel.head);
    let selection = (!sel.is_empty()).then(|| position_of(text, sel.anchor));
    content.move_to(Cursor { position, selection });
}

pub struct Page {
    pub path: PathBuf,
    pub text: String,
    crlf: bool,
    pub doc: Document,
    /// The text as last loaded or saved.
    saved: String,
    /// The file's modification time when it was last loaded or saved.
    pub modified: Option<SystemTime>,
    /// The cursor and selection in `text`.
    pub sel: Sel,
    pub active: Option<Active>,
    /// The active region is a new empty paragraph without source yet.
    pub pending: Option<Pending>,
    /// Whole blocks selected (anchor, head), while no region is active.
    pub blocks: Option<(usize, usize)>,
    history: History,
}

fn modified_time(path: &Path) -> Option<SystemTime> {
    std::fs::metadata(path).and_then(|m| m.modified()).ok()
}

impl Page {
    pub fn open(path: &Path) -> Result<Page, String> {
        let page = PageText::read(path)?;
        let mut p = Page::new(path.to_path_buf(), page.text, page.crlf);
        p.modified = modified_time(path);
        Ok(p)
    }

    pub fn new(path: PathBuf, text: String, crlf: bool) -> Page {
        let doc = Document::parse(&text);
        Page {
            path,
            saved: text.clone(),
            text,
            crlf,
            doc,
            modified: None,
            sel: Sel::default(),
            active: None,
            pending: None,
            blocks: None,
            history: History::default(),
        }
    }

    pub fn is_dirty(&self) -> bool {
        self.text != self.saved
    }

    pub fn save(&mut self) -> Result<(), String> {
        PageText { text: self.text.clone(), crlf: self.crlf }.write(&self.path)?;
        self.saved = self.text.clone();
        self.modified = modified_time(&self.path);
        Ok(())
    }

    /// Whether the file changed on disk since it was loaded or saved.
    pub fn changed_on_disk(&self) -> bool {
        self.path.exists() && modified_time(&self.path) != self.modified
    }

    /// Reads the file again, keeping the cursor about where it was. The
    /// change can be undone.
    pub fn reload(&mut self) -> Result<(), String> {
        let page = PageText::read(&self.path)?;
        self.crlf = page.crlf;
        self.modified = modified_time(&self.path);
        self.saved = page.text.clone();
        if page.text != self.text {
            let at = self.sel.head.min(page.text.len());
            let at = (0..=at).rev().find(|&i| page.text.is_char_boundary(i)).unwrap_or(0);
            self.set_text(page.text, Sel::caret(at));
            self.history.break_typing();
        }
        Ok(())
    }

    /// Follows a rename of the file (its text may have changed too).
    pub fn moved_to(&mut self, path: PathBuf) -> Result<(), String> {
        self.path = path;
        self.reload()
    }

    fn reparse(&mut self) {
        self.doc = Document::parse(&self.text);
    }

    /// The source of the active region.
    pub fn region_text(&self) -> Option<&str> {
        self.active.as_ref().map(|a| &self.text[a.region.clone()])
    }

    fn snapshot(&self) -> Snapshot {
        Snapshot { text: self.text.clone(), sel: self.sel }
    }

    /// Puts the cursor (and selection) at `sel`, making the block there the
    /// active region.
    pub fn place(&mut self, sel: Sel) {
        self.blocks = None;
        let len = self.text.len();
        let clamp = |o: usize| {
            let o = o.min(len);
            (0..=o).rev().find(|&i| self.text.is_char_boundary(i)).unwrap_or(0)
        };
        let sel = Sel { anchor: clamp(sel.anchor), head: clamp(sel.head) };
        if let Some(a) = &mut self.active
            && a.table.is_none()
            && self.pending.is_none()
            && a.region.contains(&sel.head).then_some(()).or((sel.head == a.region.end).then_some(())).is_some()
            && a.region.start <= sel.anchor
            && sel.anchor <= a.region.end
        {
            let text = &self.text[a.region.clone()];
            let local = Sel { anchor: sel.anchor - a.region.start, head: sel.head - a.region.start };
            move_content(&mut a.content, text, local);
            self.sel = sel;
            return;
        }
        self.deactivate();
        let head = sel.head;
        match edit::region_at(&self.doc, head) {
            Some(region) => {
                let source = &self.text[region.clone()];
                let mut active = Active::new(region.clone(), source);
                if active.kind == BlockKind::Table
                    && let Some(t) = Table::parse(source)
                {
                    let cell = table::cell_at_offset(source, head - region.start);
                    let cell = (cell.0.min(t.rows.len() - 1), cell.1.min(t.columns() - 1));
                    active.content = Content::with_text(&table::cell_to_text(&t.rows[cell.0][cell.1]));
                    active.table = Some(TableState { table: t, cell, range: None });
                    self.sel = Sel::caret(head);
                } else {
                    let within = |o: usize| o.clamp(region.start, region.end) - region.start;
                    let local = Sel { anchor: within(sel.anchor), head: within(sel.head) };
                    move_content(&mut active.content, source, local);
                    self.sel = Sel { anchor: local.anchor + region.start, head: local.head + region.start };
                }
                self.active = Some(active);
            }
            None => self.start_pending(Pending::gap(&self.text, head)),
        }
    }

    /// Makes the active region a new empty paragraph.
    fn start_pending(&mut self, pending: Pending) {
        let at = pending.at;
        self.active = Some(Active { region: at..at, content: Content::new(), kind: BlockKind::Paragraph, table: None });
        self.pending = Some(pending);
        self.sel = Sel::caret(at);
    }

    /// Leaves the active region: an empty new paragraph disappears, and so do
    /// the blank lines of a block whose text was all deleted.
    pub fn deactivate(&mut self) {
        let Some(active) = self.active.take() else { return };
        if self.pending.take().is_some() {
            return;
        }
        if active.region.is_empty() && !self.text.is_empty() {
            let (text, at) = edit::collapse_blank_lines(&self.text, active.region.start);
            if text != self.text {
                self.text = text;
                self.reparse();
            }
            self.sel = Sel::caret(at);
        }
    }

    /// Applies an action of the text editor to the active region.
    pub fn perform(&mut self, action: Action) {
        let Some(active) = &mut self.active else { return };
        let is_edit = action.is_edit();
        let typed = match &action {
            Action::Edit(Edit::Insert(c)) => Some(*c),
            _ => None,
        };
        let before = Snapshot { text: self.text.clone(), sel: self.sel };
        active.content.perform(action);
        if active.table.is_some() {
            if is_edit {
                self.table_cell_changed(before);
            }
            return;
        }
        let new = active.content.text();
        if !is_edit {
            let local = content_sel(&active.content, &new);
            self.sel = Sel { anchor: local.anchor + active.region.start, head: local.head + active.region.start };
            self.history.break_typing();
            return;
        }
        if let Some(pending) = &self.pending {
            if new.is_empty() {
                return;
            }
            let (text, start) = pending.materialize(&self.text, &new);
            self.text = text;
            active.region = start..start + new.len();
            self.pending = None;
        } else {
            self.text = edit::splice(&self.text, active.region.clone(), &new);
            active.region.end = active.region.start + new.len();
        }
        active.kind = own_kind(&new);
        let local = content_sel(&active.content, &new);
        self.sel = Sel { anchor: local.anchor + active.region.start, head: local.head + active.region.start };
        self.reparse();
        self.history.record(before, typed.map(|c| (c, self.sel.head.saturating_sub(c.len_utf8()))), Instant::now());
    }

    /// The table's source after its cell text changed.
    fn table_cell_changed(&mut self, before: Snapshot) {
        let Some(active) = &mut self.active else { return };
        let Some(state) = &mut active.table else { return };
        let cell_text = active.content.text();
        state.table.rows[state.cell.0][state.cell.1] = table::text_to_cell(&cell_text);
        let source = state.table.to_markdown();
        self.text = edit::splice(&self.text, active.region.clone(), &source);
        active.region.end = active.region.start + source.len();
        self.sel = Sel::caret(active.region.start + table::cell_offset(&source, state.cell));
        self.reparse();
        self.history.record(before, None, Instant::now());
    }

    /// Changes the table being edited by `change`, as one undoable change;
    /// the cursor goes to the cell `change` returns.
    pub fn change_table(&mut self, change: impl FnOnce(&mut Table, (usize, usize)) -> (usize, usize)) {
        let before = self.snapshot();
        let Some(active) = &mut self.active else { return };
        let Some(state) = &mut active.table else { return };
        let cell = change(&mut state.table, state.cell);
        let rows = state.table.rows.len();
        let columns = state.table.columns();
        state.cell = (cell.0.min(rows - 1), cell.1.min(columns - 1));
        state.range = None;
        let source = state.table.to_markdown();
        let cell_text = table::cell_to_text(&state.table.rows[state.cell.0][state.cell.1]);
        active.content = Content::with_text(&cell_text);
        self.text = edit::splice(&self.text, active.region.clone(), &source);
        active.region.end = active.region.start + source.len();
        self.sel = Sel::caret(active.region.start + table::cell_offset(&source, state.cell));
        self.reparse();
        self.history.record(before, None, Instant::now());
    }

    /// Moves to another cell of the table being edited, selecting its text.
    pub fn go_to_cell(&mut self, cell: (usize, usize)) {
        let Some(active) = &mut self.active else { return };
        let Some(state) = &mut active.table else { return };
        let cell = (cell.0.min(state.table.rows.len() - 1), cell.1.min(state.table.columns() - 1));
        state.cell = cell;
        state.range = None;
        let text = table::cell_to_text(&state.table.rows[cell.0][cell.1]);
        active.content = Content::with_text(&text);
        if !text.is_empty() {
            active.content.perform(Action::SelectAll);
        }
        let source = &self.text[active.region.clone()];
        self.sel = Sel::caret(active.region.start + table::cell_offset(source, cell));
        self.history.break_typing();
    }

    /// Applies the outcome of an editing rule as one undoable change.
    pub fn apply(&mut self, edited: Edited) {
        let before = self.snapshot();
        let changed = edited.text != self.text;
        self.active = None;
        self.pending = None;
        self.text = edited.text;
        self.reparse();
        match edited.pending {
            Some(p) => self.start_pending(p),
            None => self.place(edited.sel),
        }
        if changed {
            self.history.record(before, None, Instant::now());
        }
        self.history.break_typing();
    }

    /// Replaces the text (one undoable change) and puts the cursor at `sel`.
    pub fn set_text(&mut self, text: String, sel: Sel) {
        self.apply(Edited { text, sel, pending: None });
    }

    pub fn undo(&mut self) -> bool {
        self.time_travel(true)
    }

    pub fn redo(&mut self) -> bool {
        self.time_travel(false)
    }

    fn time_travel(&mut self, back: bool) -> bool {
        let current = self.snapshot();
        let step = if back { self.history.undo(current) } else { self.history.redo(current) };
        let Some(s) = step else { return false };
        self.active = None;
        self.pending = None;
        self.blocks = None;
        self.text = s.text;
        self.reparse();
        self.place(s.sel);
        true
    }

    /// The text of the selection (in the region, or the selected blocks).
    pub fn selected_source(&self) -> String {
        if let Some((a, b)) = self.blocks {
            let (first, last) = (a.min(b), a.max(b));
            let r = self.doc.blocks[first].range.start..self.doc.blocks[last].range.end;
            return self.text[r].to_string();
        }
        if let Some(active) = &self.active
            && active.table.is_some()
        {
            return active.content.selection().unwrap_or_default();
        }
        self.text[self.sel.range()].to_string()
    }

    /// The range of the selected blocks.
    pub fn blocks_range(&self) -> Option<Range<usize>> {
        let (a, b) = self.blocks?;
        let (first, last) = (a.min(b), a.max(b));
        Some(self.doc.blocks.get(first)?.range.start..self.doc.blocks.get(last)?.range.end)
    }

    /// The index of the first block the active region covers (or that follows a new paragraph).
    pub fn active_block(&self) -> Option<usize> {
        let a = self.active.as_ref()?;
        if self.pending.is_some() {
            return None;
        }
        self.doc.blocks.iter().position(|b| b.range.end >= a.region.start && b.range.start <= a.region.end)
    }

    /// The block before / after the active region (or a new paragraph).
    pub fn neighbor(&self, after: bool) -> Option<usize> {
        let (start, end) = match &self.active {
            Some(a) => (a.region.start, a.region.end),
            None => (self.sel.head, self.sel.head),
        };
        if after {
            self.doc.blocks.iter().position(|b| b.range.start > end || (b.range.start >= end && b.range.end > end))
        } else {
            self.doc.blocks.iter().rposition(|b| b.range.end < start || (b.range.end <= start && b.range.start < start))
        }
    }

    /// The cursor's column on its line within the active region.
    pub fn column(&self) -> usize {
        let line_start = self.text[..self.sel.head].rfind('\n').map_or(0, |i| i + 1);
        self.text[line_start..self.sel.head].chars().count()
    }

    /// Moves into block `index`: at its first line (`at_start`) or last line,
    /// at `column` characters where the line is long enough.
    pub fn enter_block(&mut self, index: usize, at_start: bool, column: Option<usize>) {
        let Some(b) = self.doc.blocks.get(index) else { return };
        let r = b.range.clone();
        let line = if at_start { edit::line_range(&self.text, r.start) } else { edit::line_range(&self.text, r.end) };
        let at = match column {
            Some(col) => {
                let l = &self.text[line.clone()];
                line.start + l.char_indices().nth(col).map_or(l.len(), |(i, _)| i)
            }
            None if at_start => r.start,
            None => r.end,
        };
        self.place(Sel::caret(at));
    }

    /// Selects whole blocks from `anchor` to `head`, leaving the active region.
    pub fn select_blocks(&mut self, anchor: usize, head: usize) {
        self.deactivate();
        let last = self.doc.blocks.len().saturating_sub(1);
        self.blocks = (!self.doc.blocks.is_empty()).then_some((anchor.min(last), head.min(last)));
    }

    /// The last change could be part of typing; a move ends it.
    pub fn break_typing(&mut self) {
        self.history.break_typing();
    }

    /// The selection within the active region's content, as page offsets.
    pub fn sync_selection(&mut self) {
        if let Some(a) = &self.active
            && a.table.is_none()
        {
            let text = &self.text[a.region.clone()];
            let local = content_sel(&a.content, text);
            self.sel = Sel { anchor: local.anchor + a.region.start, head: local.head + a.region.start };
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn page(text: &str) -> Page {
        Page::new(PathBuf::from("p.md"), text.to_string(), false)
    }

    fn typing(p: &mut Page, s: &str) {
        for c in s.chars() {
            p.perform(Action::Edit(Edit::Insert(c)));
        }
    }

    #[test]
    fn positions_and_offsets() {
        let t = "ab\ncde\n";
        assert_eq!(offset_of(t, Position { line: 1, column: 2 }), 5);
        assert_eq!(position_of(t, 5), Position { line: 1, column: 2 });
        assert_eq!(position_of(t, 7), Position { line: 2, column: 0 });
    }

    #[test]
    fn typing_goes_into_the_text_of_the_active_block() {
        let mut p = page("one\n\ntwo\n");
        p.place(Sel::caret(8));
        assert_eq!(p.region_text(), Some("two"));
        typing(&mut p, "!!");
        assert_eq!(p.text, "one\n\ntwo!!\n");
        assert_eq!(p.sel, Sel::caret(10));
        assert!(p.is_dirty());
        assert!(p.undo());
        assert_eq!(p.text, "one\n\ntwo\n");
        assert!(!p.is_dirty());
    }

    #[test]
    fn a_new_paragraph_gets_text_when_typed_into() {
        let mut p = page("one");
        let e = edit::enter(&p.text.clone(), &p.doc.clone(), 0..3, Sel::caret(3)).unwrap();
        p.apply(e);
        assert!(p.pending.is_some());
        assert_eq!(p.text, "one");
        typing(&mut p, "x");
        assert_eq!(p.text, "one\n\nx");
        assert!(p.pending.is_none());
        assert_eq!(p.region_text(), Some("x"));
        // Leaving an empty new paragraph changes nothing.
        let mut q = page("one");
        let e = edit::enter(&q.text.clone(), &q.doc.clone(), 0..3, Sel::caret(3)).unwrap();
        q.apply(e);
        q.place(Sel::caret(0));
        assert_eq!(q.text, "one");
    }

    #[test]
    fn a_block_typed_empty_leaves_no_blank_lines() {
        let mut p = page("a\n\nb\n\nc");
        p.place(Sel::caret(4));
        p.perform(Action::Edit(Edit::Backspace));
        assert_eq!(p.text, "a\n\n\n\nc");
        p.place(Sel::caret(0));
        assert_eq!(p.text, "a\n\nc");
    }

    #[test]
    fn tables_are_edited_cell_by_cell() {
        let mut p = page("x\n\n| a | b |\n|---|---|\n| 1 | 2 |\n");
        let at = p.text.find('2').unwrap();
        p.place(Sel::caret(at));
        let state = p.active.as_ref().unwrap().table.clone().unwrap();
        assert_eq!(state.cell, (1, 1));
        p.perform(Action::Edit(Edit::Insert('|')));
        assert!(p.text.contains("| 2\\| |") || p.text.contains("| \\|2 |"), "{}", p.text);
        p.go_to_cell((0, 0));
        assert_eq!(p.active.as_ref().unwrap().content.selection().as_deref(), Some("a"));
        p.change_table(|t, (r, _)| {
            t.insert_row(r + 1);
            (r + 1, 0)
        });
        assert_eq!(p.active.as_ref().unwrap().table.as_ref().unwrap().table.rows.len(), 3);
        assert!(p.undo());
        assert!(p.undo());
        assert_eq!(p.text, "x\n\n| a | b |\n|---|---|\n| 1 | 2 |\n");
    }
}
