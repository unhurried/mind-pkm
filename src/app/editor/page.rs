//! An open page: its text, the active region (the block with the cursor), the
//! selection and the undo history.
//!
//! The text is the truth. A paragraph, heading, list item, quote or table
//! cell is edited in its rendered form (`Live`): what it shows is derived
//! from the text and the cursor, and every edit of the shown text becomes an
//! edit of the source, after which the page is parsed again. Code-like
//! blocks show their source in a text editor (`Content`), whose edits are
//! spliced back the same way. Rules that change more (Enter, joining blocks,
//! formatting) return a new text and selection, after which the region is
//! derived anew.

use crate::markdown::edit::{self, Edited, Pending, Sel};
use crate::markdown::history::{History, Snapshot};
use crate::markdown::live::{self, Display};
use crate::markdown::page::PageText;
use crate::markdown::table::{self, CellRange, Table};
use crate::markdown::{BlockKind, Document};
use iced::widget::text_editor::{Action, Content, Cursor, Edit, Motion, Position};
use std::ops::Range;
use std::path::{Path, PathBuf};
use std::time::{Instant, SystemTime};

/// The table being edited, cell by cell, while the cursor is in it.
#[derive(Debug, Clone)]
pub struct TableState {
    pub table: Table,
    /// The cell being edited.
    pub cell: (usize, usize),
    /// A rectangle of selected cells.
    pub range: Option<CellRange>,
    /// The cursor and selection in the cell's Markdown.
    pub cell_sel: Sel,
}

/// A block (or table cell) edited in its rendered form.
#[derive(Debug, Clone, Default)]
pub struct Live {
    pub display: Display,
    /// The selection in the shown text.
    pub sel: Sel,
}

pub struct Active {
    /// Its bytes in the page text.
    pub region: Range<usize>,
    /// The region's source, for a block shown as source.
    pub content: Content,
    /// The kind of the region.
    pub kind: BlockKind,
    pub table: Option<TableState>,
    /// For a block or table cell edited in its rendered form.
    pub live: Option<Live>,
}

impl Active {
    fn new(region: Range<usize>, text: &str) -> Active {
        Active { region, content: Content::with_text(text), kind: own_kind(text), table: None, live: None }
    }
}

/// Typed text as a table cell stores it: line breaks as `<br>`, `|` escaped.
fn cell_text(text: &str) -> String {
    text.replace('\n', "<br>").replace('|', "\\|")
}

/// The source after replacing the shown range `shown` of `display` (or
/// inserting at the source offset `cursor` when it is empty) with `insert`,
/// and the cursor after it.
fn replace_shown(source: &str, display: &Display, shown: Range<usize>, cursor: usize, insert: &str) -> (String, usize) {
    let mut out = source.to_string();
    let at = if shown.is_empty() {
        cursor.min(out.len())
    } else {
        let deletions = display.deletion(shown);
        let at = deletions.first().map_or(cursor, |d| d.start);
        for d in deletions.iter().rev() {
            out.replace_range(d.clone(), "");
        }
        at
    };
    out.insert_str(at, insert);
    (out, at + insert.len())
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

/// Puts the cursor (and selection) of a text editor at `sel` in its `text`.
pub fn move_content(content: &mut Content, text: &str, sel: Sel) {
    if sel.is_empty() && content.selection().is_some() {
        // `move_to` keeps a selection; a new content has none.
        *content = Content::with_text(text);
    }
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
    /// Counts the changes of the text.
    pub revision: u64,
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
            revision: 0,
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
        self.revision += 1;
    }

    /// The source of the active region.
    #[cfg(test)]
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
            if a.live.is_some() {
                self.sel = sel;
                self.refresh_live();
                return;
            }
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
                let block = self.doc.blocks.iter().position(|b| b.range.start == region.start);
                if active.kind == BlockKind::Table
                    && let Some(t) = Table::parse(source)
                {
                    let cell = table::cell_at_offset(source, head - region.start);
                    let cell = (cell.0.min(t.rows.len() - 1), cell.1.min(t.columns() - 1));
                    // The cursor (a selection within the cell) where it is in the source.
                    let raw = &t.rows[cell.0][cell.1];
                    let start = region.start + table::cell_offset(source, cell);
                    let local =
                        |o: usize| Some(o.checked_sub(start)?.min(raw.len())).filter(|&i| raw.is_char_boundary(i));
                    let same_cell = table::cell_at_offset(source, sel.anchor.saturating_sub(region.start)) == cell;
                    let h = local(head).unwrap_or(0);
                    let a = if same_cell { local(sel.anchor).unwrap_or(h) } else { h };
                    active.table =
                        Some(TableState { table: t, cell, range: None, cell_sel: Sel { anchor: a, head: h } });
                    active.live = Some(Live::default());
                    self.active = Some(active);
                    self.sync_cell_sel();
                    return;
                } else if let Some(b) = block.filter(|&b| live::editable(&self.doc, b)) {
                    active.kind = self.doc.blocks[b].kind.clone();
                    active.live = Some(Live::default());
                    self.active = Some(active);
                    let within = |o: usize| o.clamp(region.start, region.end);
                    self.sel = Sel { anchor: within(sel.anchor), head: within(sel.head) };
                    self.refresh_live();
                    return;
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
        self.active = Some(Active {
            region: at..at,
            content: Content::new(),
            kind: BlockKind::Paragraph,
            table: None,
            live: Some(Live { display: Display::empty(at), sel: Sel::default() }),
        });
        self.pending = Some(pending);
        self.sel = Sel::caret(at);
    }

    /// What the live region shows, after the text or the cursor changed.
    fn refresh_live(&mut self) {
        let Some(a) = &mut self.active else { return };
        let Some(live) = &mut a.live else { return };
        let (display, sel) = if let Some(state) = &a.table {
            let raw = &state.table.rows[state.cell.0][state.cell.1];
            (Display::for_cell(raw, state.cell_sel.head.min(raw.len())), state.cell_sel)
        } else if self.pending.is_some() || a.region.is_empty() {
            (Display::empty(a.region.start), self.sel)
        } else {
            let Some(block) = self.doc.blocks.iter().position(|b| b.range.start == a.region.start) else { return };
            (Display::new(&self.doc, self.doc.block_events(block), a.region.clone(), self.sel.head), self.sel)
        };
        live.sel = Sel { anchor: display.to_shown(sel.anchor), head: display.to_shown(sel.head) };
        live.display = display;
    }

    /// The page selection of the cell selection, and what the cell shows.
    fn sync_cell_sel(&mut self) {
        let Some(a) = &self.active else { return };
        let Some(state) = &a.table else { return };
        let base = a.region.start + table::cell_offset(&self.text[a.region.clone()], state.cell);
        self.sel = Sel { anchor: base + state.cell_sel.anchor, head: base + state.cell_sel.head };
        self.refresh_live();
    }

    /// The selection the live region's motions work on: the page's, or the
    /// cell's in its Markdown.
    pub fn live_sel(&self) -> Sel {
        match self.active.as_ref().and_then(|a| a.table.as_ref()) {
            Some(state) => state.cell_sel,
            None => self.sel,
        }
    }

    fn set_live_sel(&mut self, sel: Sel) {
        if let Some(a) = &mut self.active
            && let Some(state) = &mut a.table
        {
            state.cell_sel = sel;
            self.sync_cell_sel();
        } else {
            self.sel = sel;
            self.refresh_live();
        }
    }

    fn live(&self) -> Option<&Live> {
        self.active.as_ref()?.live.as_ref()
    }

    /// Something to compare before and after a motion: whether the cursor moved.
    pub fn cursor_token(&self) -> Option<(usize, usize, usize, usize)> {
        let a = self.active.as_ref()?;
        Some(match &a.live {
            Some(_) => {
                let s = self.live_sel();
                (s.anchor, s.head, 0, 0)
            }
            None => {
                let c = a.content.cursor();
                let s = c.selection.unwrap_or(c.position);
                (c.position.line, c.position.column, s.line, s.column)
            }
        })
    }

    /// Whether the cursor is at the start (end) of the active region, with nothing selected.
    pub fn at_edge(&self, right: bool) -> bool {
        let Some(a) = &self.active else { return true };
        match &a.live {
            Some(l) => l.sel.is_empty() && if right { l.sel.head >= l.display.text.len() } else { l.sel.head == 0 },
            None => {
                self.sel.is_empty()
                    && if right { self.sel.head >= a.region.end } else { self.sel.head <= a.region.start }
            }
        }
    }

    /// The selection the editing rules (Enter, Backspace at the start...)
    /// work with: at the start of what a block shows, its content's start in
    /// the source (after its block syntax, before any inline Markdown).
    pub fn rule_sel(&self) -> Sel {
        if let Some(a) = &self.active
            && let Some(l) = &a.live
            && a.table.is_none()
            && l.sel.is_empty()
            && l.sel.head == 0
        {
            return Sel::caret(l.display.content.start);
        }
        self.sel
    }

    /// A text editor action on the live region.
    fn live_action(&mut self, action: Action) {
        match action {
            Action::Edit(Edit::Insert(c)) => self.live_insert(&c.to_string(), Some(c)),
            Action::Edit(Edit::Paste(s)) => self.live_insert(&s, None),
            Action::Edit(Edit::Enter) => self.live_insert("\n", None),
            Action::Edit(Edit::Backspace) => self.live_delete(false),
            Action::Edit(Edit::Delete) => self.live_delete(true),
            Action::Move(m) => self.live_motion(m, false),
            Action::Select(m) => self.live_motion(m, true),
            Action::SelectAll | Action::SelectLine => self.live_select_shown(None),
            Action::SelectWord => {
                let head = self.live().map_or(0, |l| l.sel.head);
                self.live_select_word(head);
            }
            _ => {}
        }
    }

    /// Inserts at the cursor of the live region, in place of the selection.
    pub fn live_insert(&mut self, text: &str, typed: Option<char>) {
        let Some(shown) = self.live().map(|l| l.sel.range()) else { return };
        self.live_replace(shown, text, typed);
    }

    /// Backspace / Delete: the selection, else the character before / after the cursor.
    fn live_delete(&mut self, forward: bool) {
        let Some(l) = self.live() else { return };
        let shown = if !l.sel.is_empty() {
            Some(l.sel.range())
        } else if forward {
            l.display.grapheme_after(l.sel.head)
        } else {
            l.display.grapheme_before(l.sel.head)
        };
        if let Some(shown) = shown {
            self.live_replace(shown, "", None);
        }
    }

    fn live_replace(&mut self, shown: Range<usize>, insert: &str, typed: Option<char>) {
        let before = self.snapshot();
        // A new paragraph gets its source.
        if let Some(pending) = &self.pending {
            if insert.is_empty() {
                return;
            }
            let (text, start) = pending.materialize(&self.text, insert);
            self.pending = None;
            self.text = text;
            self.reparse();
            self.active = None;
            self.place(Sel::caret(start + insert.len()));
            self.record(before, typed);
            return;
        }
        let Some(a) = &mut self.active else { return };
        let Some(live) = &a.live else { return };
        if let Some(state) = &mut a.table {
            let raw = state.table.rows[state.cell.0][state.cell.1].clone();
            let (new, cursor) = replace_shown(&raw, &live.display, shown, state.cell_sel.head, &cell_text(insert));
            state.table.rows[state.cell.0][state.cell.1] = new;
            state.cell_sel = Sel::caret(cursor);
            let source = state.table.to_markdown();
            self.text = edit::splice(&self.text, a.region.clone(), &source);
            a.region.end = a.region.start + source.len();
            self.reparse();
            self.sync_cell_sel();
            self.record(before, typed);
            return;
        }
        let (text, cursor) = replace_shown(&self.text, &live.display, shown, self.sel.head, insert);
        let start = a.region.start;
        let end = (a.region.end + text.len()).saturating_sub(self.text.len()).max(start);
        if end == start {
            // All of the block's text deleted: an empty block until it is left.
            a.region = start..start;
        }
        self.text = text;
        self.reparse();
        if end == start {
            self.sel = Sel::caret(start);
            self.refresh_live();
        } else {
            self.active = None;
            self.place(Sel::caret(cursor));
        }
        self.record(before, typed);
    }

    fn record(&mut self, before: Snapshot, typed: Option<char>) {
        let at = self.sel.head;
        self.history.record(before, typed.map(|c| (c, at.saturating_sub(c.len_utf8()))), Instant::now());
    }

    /// Moves the live region's cursor (`select`: its head only).
    fn live_motion(&mut self, motion: Motion, select: bool) {
        let Some(l) = self.live() else { return };
        let d = &l.display;
        let s = self.live_sel();
        let target = match motion {
            Motion::Left if !select && !s.is_empty() => Some(s.range().start),
            Motion::Right if !select && !s.is_empty() => Some(s.range().end),
            Motion::Left => d.left(s.head),
            Motion::Right => d.right(s.head),
            Motion::WordLeft => d.word_left(s.head),
            Motion::WordRight => d.word_right(s.head),
            Motion::Home | Motion::DocumentStart | Motion::PageUp => Some(d.to_source(0)),
            Motion::End | Motion::DocumentEnd | Motion::PageDown => Some(d.to_source(d.text.len())),
            // Lines are the widget's: here there is no line above or below.
            Motion::Up | Motion::Down => None,
        };
        if let Some(t) = target {
            self.set_live_sel(if select { Sel { anchor: s.anchor, head: t } } else { Sel::caret(t) });
        }
        self.history.break_typing();
    }

    /// The cursor to a shown offset of the live region (`extend`: the head only).
    pub fn live_move_to(&mut self, at: usize, extend: bool) {
        let Some(l) = self.live() else { return };
        let s = l.display.to_source(at.min(l.display.text.len()));
        let anchor = self.live_sel().anchor;
        self.set_live_sel(if extend { Sel { anchor, head: s } } else { Sel::caret(s) });
        self.history.break_typing();
    }

    /// Selects the shown range (all of it when None).
    pub fn live_select_shown(&mut self, range: Option<Range<usize>>) {
        let Some(l) = self.live() else { return };
        let r = range.unwrap_or(0..l.display.text.len());
        let sel = Sel { anchor: l.display.to_source(r.start), head: l.display.to_source(r.end) };
        self.set_live_sel(sel);
        self.history.break_typing();
    }

    /// Selects the word at a shown offset.
    pub fn live_select_word(&mut self, at: usize) {
        let Some(range) = self.live().and_then(|l| l.display.word_at(at)) else { return };
        self.live_select_shown(Some(range));
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
        if self.live().is_some() {
            self.live_action(action);
            return;
        }
        let Some(active) = &mut self.active else { return };
        let is_edit = action.is_edit();
        let typed = match &action {
            Action::Edit(Edit::Insert(c)) => Some(*c),
            _ => None,
        };
        let before = Snapshot { text: self.text.clone(), sel: self.sel };
        active.content.perform(action);
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

    /// Changes the table being edited by `change`, as one undoable change;
    /// the cursor goes to the cell `change` returns.
    pub fn change_table(&mut self, change: impl FnOnce(&mut Table, (usize, usize)) -> (usize, usize)) {
        let before = self.snapshot();
        let Some(active) = &mut self.active else { return };
        let Some(state) = &mut active.table else { return };
        let old = state.cell;
        let cell = change(&mut state.table, state.cell);
        let rows = state.table.rows.len();
        let columns = state.table.columns();
        state.cell = (cell.0.min(rows - 1), cell.1.min(columns - 1));
        state.range = None;
        // In the same cell the cursor stays (where the text still reaches); else at the start.
        let raw = &state.table.rows[state.cell.0][state.cell.1];
        let clamp = |o: usize| (0..=o.min(raw.len())).rev().find(|&i| raw.is_char_boundary(i)).unwrap_or(0);
        state.cell_sel = if state.cell == old {
            Sel { anchor: clamp(state.cell_sel.anchor), head: clamp(state.cell_sel.head) }
        } else {
            Sel::default()
        };
        let source = state.table.to_markdown();
        self.text = edit::splice(&self.text, active.region.clone(), &source);
        active.region.end = active.region.start + source.len();
        self.reparse();
        self.sync_cell_sel();
        self.history.record(before, None, Instant::now());
    }

    /// Moves to another cell of the table being edited, selecting its text.
    pub fn go_to_cell(&mut self, cell: (usize, usize)) {
        let Some(active) = &mut self.active else { return };
        let Some(state) = &mut active.table else { return };
        let cell = (cell.0.min(state.table.rows.len() - 1), cell.1.min(state.table.columns() - 1));
        state.cell = cell;
        state.range = None;
        state.cell_sel = Sel { anchor: 0, head: state.table.rows[cell.0][cell.1].len() };
        self.sync_cell_sel();
        self.history.break_typing();
    }

    /// Moves to another cell with the cursor at the start (or end) of its text.
    pub fn go_to_cell_at(&mut self, cell: (usize, usize), at_end: bool) {
        self.go_to_cell(cell);
        if let Some(state) = self.active.as_mut().and_then(|a| a.table.as_mut()) {
            state.cell_sel = Sel::caret(if at_end { state.cell_sel.head } else { 0 });
            self.sync_cell_sel();
        }
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
        if let Some(state) = self.active.as_ref().and_then(|a| a.table.as_ref()) {
            let raw = &state.table.rows[state.cell.0][state.cell.1];
            return raw.get(state.cell_sel.range()).unwrap_or_default().to_string();
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
        assert_eq!(p.selected_source(), "a");
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
