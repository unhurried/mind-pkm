//! Editing the active table: moving between cells, its rows, columns and
//! alignment, ranges of cells, the toolbar and the cell's right-click menu.

use super::keys::{Bind, EditorCommand, TableMenuItem};
use super::{Editor, EditorMsg, Effect};
use crate::markdown::edit::Sel;
use crate::markdown::table::{Align, CellRange, Table};
use iced::Task;
use iced::widget::text_editor::{Action, Edit, Motion};

#[derive(Debug, Clone, PartialEq)]
pub enum TableMsg {
    /// The pointer entered a cell (extending a range while the button is down).
    Entered(usize, usize),
    /// The mouse button was released.
    Released,
    /// A right click on a cell: its menu.
    Menu(usize, usize),
    MenuItem(TableMenuItem),
    /// The toolbar's size fields.
    Rows(String),
    Columns(String),
    Resize,
}

impl Editor {
    fn table_cell(&self) -> Option<((usize, usize), usize, usize)> {
        let state = self.page.active.as_ref()?.table.as_ref()?;
        Some((state.cell, state.table.rows.len(), state.table.columns()))
    }

    fn range_mut(&mut self) -> Option<&mut Option<CellRange>> {
        Some(&mut self.page.active.as_mut()?.table.as_mut()?.range)
    }

    /// The selected range of cells, or the cursor's cell.
    fn cells(&self) -> Option<CellRange> {
        let state = self.page.active.as_ref()?.table.as_ref()?;
        Some(state.range.unwrap_or(CellRange::new(state.cell, state.cell)))
    }

    /// A table command; `false` when the cursor is not in a table.
    pub(super) fn table_command(&mut self, cmd: EditorCommand) -> bool {
        use EditorCommand as C;
        let Some(cells) = self.cells() else { return false };
        let align = |a: Align| {
            move |t: &mut Table, cell| {
                t.set_align(cells.cols(), a);
                cell
            }
        };
        match cmd {
            C::RowAbove => self.page.change_table(|t, (r, c)| {
                let at = r.max(1);
                t.insert_row(at);
                (at, c)
            }),
            C::RowBelow => self.page.change_table(|t, (r, c)| {
                t.insert_row(r + 1);
                (r + 1, c)
            }),
            C::DeleteRow => self.page.change_table(|t, (r, c)| {
                t.delete_row(r);
                (r, c)
            }),
            C::ColumnLeft => self.page.change_table(|t, (r, c)| {
                t.insert_column(c);
                (r, c)
            }),
            C::ColumnRight => self.page.change_table(|t, (r, c)| {
                t.insert_column(c + 1);
                (r, c + 1)
            }),
            C::DeleteColumn => self.page.change_table(|t, (r, c)| {
                t.delete_column(c);
                (r, c)
            }),
            C::MoveRowUp | C::MoveRowDown => {
                self.page.change_table(|t, (r, c)| (t.move_row(r, cmd == C::MoveRowUp), c))
            }
            C::MoveColumnLeft | C::MoveColumnRight => {
                self.page.change_table(|t, (r, c)| (r, t.move_column(c, cmd == C::MoveColumnLeft)))
            }
            C::AlignLeft => self.keep_range(|p| p.change_table(align(Align::Left))),
            C::AlignCenter => self.keep_range(|p| p.change_table(align(Align::Center))),
            C::AlignRight => self.keep_range(|p| p.change_table(align(Align::Right))),
            C::DeleteTable => self.delete_table(),
            C::SelectLine => {
                let (r, cols) = (cells.top, self.table_cell().map_or(1, |t| t.2));
                self.page.go_to_cell((r, 0));
                if let Some(range) = self.range_mut() {
                    *range = Some(CellRange::new((r, 0), (r, cols - 1)));
                }
            }
            C::SelectSpan => {
                if let Some(range) = self.range_mut() {
                    *range = None;
                }
                self.page.perform(Action::SelectAll);
            }
            _ => return false,
        }
        true
    }

    /// Runs a change that keeps the selected range.
    fn keep_range(&mut self, change: impl FnOnce(&mut super::Page)) {
        let range = self.range_mut().and_then(|r| *r);
        change(&mut self.page);
        if let Some(r) = self.range_mut() {
            *r = range;
        }
    }

    /// Delete or Backspace on a range of cells: they become empty.
    pub(super) fn clear_cells(&mut self) {
        let Some(range) = self.page.active.as_ref().and_then(|a| a.table.as_ref()).and_then(|s| s.range) else {
            return;
        };
        self.page.change_table(|t, _| {
            t.clear(range);
            (range.top, range.left)
        });
    }

    fn delete_table(&mut self) {
        let Some(block) = self.page.active_block() else { return };
        self.page.deactivate();
        self.page.blocks = Some((block, block));
        self.delete_blocks(None);
    }

    /// Keys bound in a table's cell.
    pub(super) fn table_bind(&mut self, bind: Bind) -> Task<EditorMsg> {
        let Some(((r, c), rows, cols)) = self.table_cell() else { return Task::none() };
        let range = self.page.active.as_ref().and_then(|a| a.table.as_ref()).and_then(|s| s.range);
        if range.is_some() {
            match bind {
                Bind::Delete | Bind::Backspace => {
                    self.clear_cells();
                    return self.focus();
                }
                Bind::ShiftUp | Bind::ShiftDown | Bind::ShiftLeft | Bind::ShiftRight => {}
                _ => {
                    if let Some(r) = self.range_mut() {
                        *r = None;
                    }
                }
            }
        }
        match bind {
            Bind::Tab if r + 1 == rows && c + 1 == cols => self.page.change_table(|t, (r, _)| {
                t.insert_row(r + 1);
                (r + 1, 0)
            }),
            Bind::Tab => self.page.go_to_cell(if c + 1 < cols { (r, c + 1) } else { (r + 1, 0) }),
            Bind::ShiftTab => self.page.go_to_cell(if c > 0 {
                (r, c - 1)
            } else if r > 0 {
                (r - 1, cols - 1)
            } else {
                (0, 0)
            }),
            Bind::Enter if r + 1 == rows => self.page.change_table(|t, (r, c)| {
                t.insert_row(r + 1);
                (r + 1, c)
            }),
            Bind::Enter => self.page.go_to_cell((r + 1, c)),
            Bind::ShiftEnter => self.page.perform(Action::Edit(Edit::Enter)),
            Bind::Backspace => self.page.perform(Action::Edit(Edit::Backspace)),
            Bind::Delete => self.page.perform(Action::Edit(Edit::Delete)),
            Bind::Type(ch) => self.page.perform(Action::Edit(Edit::Insert(ch))),
            Bind::Up | Bind::Down => {
                let down = bind == Bind::Down;
                if !self.cell_moves(if down { Motion::Down } else { Motion::Up }) {
                    if down && r + 1 < rows {
                        self.page.go_to_cell((r + 1, c));
                    } else if !down && r > 0 {
                        self.page.go_to_cell((r - 1, c));
                    } else if let Some(b) = self.page.neighbor(down) {
                        self.page.enter_block(b, down, None);
                    }
                }
            }
            Bind::Left | Bind::Right => {
                let right = bind == Bind::Right;
                if !self.cell_moves(if right { Motion::Right } else { Motion::Left }) {
                    let next = if right && c + 1 < cols {
                        Some((r, c + 1))
                    } else if right && r + 1 < rows {
                        Some((r + 1, 0))
                    } else if !right && c > 0 {
                        Some((r, c - 1))
                    } else if !right && r > 0 {
                        Some((r - 1, cols - 1))
                    } else {
                        None
                    };
                    match next {
                        Some(cell) => self.page.go_to_cell_at(cell, !right),
                        None => {
                            if let Some(b) = self.page.neighbor(right) {
                                self.page.enter_block(b, right, None);
                            }
                        }
                    }
                }
            }
            Bind::ShiftUp | Bind::ShiftDown | Bind::ShiftLeft | Bind::ShiftRight => {
                let motion = match bind {
                    Bind::ShiftUp => Motion::Up,
                    Bind::ShiftDown => Motion::Down,
                    Bind::ShiftLeft => Motion::Left,
                    _ => Motion::Right,
                };
                // Within the cell's text first, then over cells.
                if range.is_some() || !self.cell_selects(motion) {
                    let head = self.range_head.filter(|_| range.is_some()).unwrap_or((r, c));
                    let head = match motion {
                        Motion::Up => (head.0.saturating_sub(1), head.1),
                        Motion::Down => ((head.0 + 1).min(rows - 1), head.1),
                        Motion::Left => (head.0, head.1.saturating_sub(1)),
                        _ => (head.0, (head.1 + 1).min(cols - 1)),
                    };
                    self.range_head = Some(head);
                    if let Some(range) = self.range_mut() {
                        *range = Some(CellRange::new((r, c), head));
                    }
                }
            }
        }
        self.focus()
    }

    /// Moves the cursor in the cell's text; whether it moved.
    fn cell_moves(&mut self, motion: Motion) -> bool {
        let before = self.page.cursor_token();
        let had_selection = !self.page.live_sel().is_empty();
        self.page.perform(Action::Move(motion));
        had_selection || before != self.page.cursor_token()
    }

    /// Extends the selection in the cell's text; whether it changed.
    fn cell_selects(&mut self, motion: Motion) -> bool {
        let before = self.page.cursor_token();
        self.page.perform(Action::Select(motion));
        before != self.page.cursor_token()
    }

    /// A press on a cell that is not being edited (Shift: a range from the cursor's cell).
    pub(super) fn cell_pressed(&mut self, cell: (usize, usize)) -> Task<EditorMsg> {
        self.menu = None;
        self.cell_menu = None;
        if self.modifiers.shift()
            && let Some((at, _, _)) = self.table_cell()
        {
            self.range_head = Some(cell);
            if let Some(range) = self.range_mut() {
                *range = (cell != at).then(|| CellRange::new(at, cell));
            }
            return self.focus();
        }
        self.page.go_to_cell(cell);
        self.cell_drag = true;
        self.focus()
    }

    /// A press on a cell of a rendered table outside its text: the cursor goes
    /// to the end of that cell's text.
    pub(super) fn table_cell_pressed(&mut self, block: usize, cell: (usize, usize)) -> Task<EditorMsg> {
        self.menu = None;
        self.cell_menu = None;
        if self.shift_selects(block) {
            return Task::none();
        }
        // A table inside another block (a footnote...): as a press on that block.
        let Some(b) = self.page.doc.blocks.get(block).filter(|b| b.kind == crate::markdown::BlockKind::Table) else {
            return self.block_pressed(block);
        };
        self.page.place(Sel::caret(b.range.start));
        self.page.go_to_cell_at(cell, true);
        self.focus()
    }

    pub(super) fn table_message(&mut self, msg: TableMsg, effects: &mut Vec<Effect>) -> Task<EditorMsg> {
        match msg {
            TableMsg::Entered(r, c) => {
                if self.cell_drag
                    && let Some((at, _, _)) = self.table_cell()
                {
                    self.range_head = Some((r, c));
                    if let Some(range) = self.range_mut() {
                        *range = ((r, c) != at).then(|| CellRange::new(at, (r, c)));
                    }
                }
                Task::none()
            }
            TableMsg::Released => {
                self.cell_drag = false;
                Task::none()
            }
            TableMsg::Menu(r, c) => {
                let inside = self.cells().is_some_and(|range| range.contains((r, c)));
                if !inside {
                    self.page.go_to_cell((r, c));
                }
                self.menu = None;
                self.cell_menu = Some(self.pointer_at);
                Task::none()
            }
            TableMsg::MenuItem(item) => {
                self.cell_menu = None;
                match item {
                    TableMenuItem::Command(cmd) => {
                        self.table_command(cmd);
                    }
                    TableMenuItem::CopyRow | TableMenuItem::CopyColumn => {
                        if let Some(((r, c), rows, cols)) = self.table_cell() {
                            let range = if item == TableMenuItem::CopyRow {
                                CellRange::new((r, 0), (r, cols - 1))
                            } else {
                                CellRange::new((0, c), (rows - 1, c))
                            };
                            let old = self.range_mut().and_then(|r| r.replace(range));
                            self.copy(false, effects);
                            if let Some(r) = self.range_mut() {
                                *r = old;
                            }
                        }
                    }
                }
                self.focus()
            }
            TableMsg::Rows(s) => {
                self.resize_fields.0 = Some(s);
                Task::none()
            }
            TableMsg::Columns(s) => {
                self.resize_fields.1 = Some(s);
                Task::none()
            }
            TableMsg::Resize => {
                let Some((_, rows, cols)) = self.table_cell() else { return Task::none() };
                let field = |s: &Option<String>, current: usize| {
                    s.as_deref().map_or(Some(current), |s| s.trim().parse::<usize>().ok().filter(|&n| n >= 1))
                };
                let (body, columns) = (field(&self.resize_fields.0, rows - 1), field(&self.resize_fields.1, cols));
                self.resize_fields = (None, None);
                if let (Some(body), Some(columns)) = (body, columns)
                    && (body, columns) != (rows - 1, cols)
                {
                    self.page.change_table(|t, cell| {
                        t.resize(body, columns);
                        cell
                    });
                }
                self.focus()
            }
        }
    }

    /// The toolbar's size fields: what is typed, or the table's size.
    pub(super) fn resize_values(&self) -> (String, String) {
        let (_, rows, cols) = self.table_cell().unwrap_or(((0, 0), 2, 1));
        (
            self.resize_fields.0.clone().unwrap_or_else(|| (rows - 1).to_string()),
            self.resize_fields.1.clone().unwrap_or_else(|| cols.to_string()),
        )
    }
}
