//! GitHub-flavored Markdown tables as the editor edits them: parsed into
//! cells (Markdown source each), changed by the table operations, and written
//! back with aligned columns.

use std::ops::RangeInclusive;
use unicode_width::UnicodeWidthStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Align {
    #[default]
    None,
    Left,
    Center,
    Right,
}

/// A table: row 0 is the header; every row has one cell per column.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Table {
    pub align: Vec<Align>,
    pub rows: Vec<Vec<String>>,
}

/// A rectangle of cells, both ends included.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CellRange {
    pub top: usize,
    pub left: usize,
    pub bottom: usize,
    pub right: usize,
}

impl CellRange {
    /// The rectangle spanned by two cells.
    pub fn new(a: (usize, usize), b: (usize, usize)) -> CellRange {
        CellRange { top: a.0.min(b.0), left: a.1.min(b.1), bottom: a.0.max(b.0), right: a.1.max(b.1) }
    }

    pub fn rows(&self) -> RangeInclusive<usize> {
        self.top..=self.bottom
    }

    pub fn cols(&self) -> RangeInclusive<usize> {
        self.left..=self.right
    }

    pub fn contains(&self, cell: (usize, usize)) -> bool {
        self.rows().contains(&cell.0) && self.cols().contains(&cell.1)
    }
}

/// The cells of a table row, as written (escaped pipes kept), trimmed.
pub fn split_row(line: &str) -> Vec<String> {
    let t = line.trim();
    let t = t.strip_prefix('|').unwrap_or(t);
    let t = if t.ends_with('|') && !t.ends_with("\\|") { &t[..t.len() - 1] } else { t };
    let mut cells = Vec::new();
    let mut cell = String::new();
    let mut chars = t.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '\\' => {
                cell.push(c);
                if let Some(n) = chars.next() {
                    cell.push(n);
                }
            }
            '|' => cells.push(std::mem::take(&mut cell).trim().to_string()),
            _ => cell.push(c),
        }
    }
    cells.push(cell.trim().to_string());
    cells
}

fn parse_align(cell: &str) -> Option<Align> {
    let c = cell.trim();
    let body = c.trim_start_matches(':').trim_end_matches(':');
    if body.is_empty() || !body.chars().all(|ch| ch == '-') {
        return None;
    }
    Some(match (c.starts_with(':'), c.ends_with(':') && c.len() > 1) {
        (true, true) => Align::Center,
        (true, false) => Align::Left,
        (false, true) => Align::Right,
        (false, false) => Align::None,
    })
}

impl Table {
    /// A table's source (header, delimiter row, body rows).
    pub fn parse(source: &str) -> Option<Table> {
        let lines: Vec<&str> = source.lines().filter(|l| !l.trim().is_empty()).collect();
        let header = split_row(lines.first()?);
        let align: Vec<Align> = split_row(lines.get(1)?).iter().map(|c| parse_align(c)).collect::<Option<_>>()?;
        let columns = header.len();
        let mut align = align;
        align.resize(columns, Align::None);
        let mut rows = vec![header];
        for line in &lines[2..] {
            let mut cells = split_row(line);
            cells.resize(columns, String::new());
            rows.push(cells);
        }
        if rows.len() == 1 {
            rows.push(vec![String::new(); columns]);
        }
        Some(Table { align, rows })
    }

    /// An empty table with a header row.
    pub fn new(body_rows: usize, columns: usize) -> Table {
        let columns = columns.max(1);
        Table { align: vec![Align::None; columns], rows: vec![vec![String::new(); columns]; body_rows.max(1) + 1] }
    }

    pub fn columns(&self) -> usize {
        self.align.len()
    }

    /// The number of body rows.
    pub fn body_rows(&self) -> usize {
        self.rows.len() - 1
    }

    /// The source, with a pipe at both ends of every row and the columns
    /// padded to equal width (East Asian wide characters counting double).
    pub fn to_markdown(&self) -> String {
        let widths: Vec<usize> =
            (0..self.columns()).map(|c| self.rows.iter().map(|r| r[c].width()).max().unwrap_or(0).max(3)).collect();
        let row = |cells: &[String]| {
            let mut s = String::from("|");
            for (cell, w) in cells.iter().zip(&widths) {
                s.push(' ');
                s.push_str(cell);
                s.push_str(&" ".repeat(w - cell.width()));
                s.push_str(" |");
            }
            s
        };
        let mut out = row(&self.rows[0]);
        out.push_str("\n|");
        for (a, w) in self.align.iter().zip(&widths) {
            let dashes = |n: usize| "-".repeat(n);
            let d = match a {
                Align::None => dashes(*w),
                Align::Left => format!(":{}", dashes(w - 1)),
                Align::Center => format!(":{}:", dashes(w - 2)),
                Align::Right => format!("{}:", dashes(w - 1)),
            };
            out.push(' ');
            out.push_str(&d);
            out.push_str(" |");
        }
        for r in &self.rows[1..] {
            out.push('\n');
            out.push_str(&row(r));
        }
        out
    }

    /// Inserts an empty body row at `at` (1 = first body row).
    pub fn insert_row(&mut self, at: usize) {
        let at = at.clamp(1, self.rows.len());
        self.rows.insert(at, vec![String::new(); self.columns()]);
    }

    /// Deletes a body row; the last one is only emptied.
    pub fn delete_row(&mut self, row: usize) {
        if row == 0 || row >= self.rows.len() {
            return;
        }
        if self.body_rows() == 1 {
            self.rows[1].iter_mut().for_each(String::clear);
        } else {
            self.rows.remove(row);
        }
    }

    /// Moves a body row one up or down; returns its new index.
    pub fn move_row(&mut self, row: usize, up: bool) -> usize {
        let target = if up { row.saturating_sub(1) } else { row + 1 };
        if row == 0 || target == 0 || target >= self.rows.len() {
            return row;
        }
        self.rows.swap(row, target);
        target
    }

    pub fn insert_column(&mut self, at: usize) {
        let at = at.min(self.columns());
        self.align.insert(at, Align::None);
        for r in &mut self.rows {
            r.insert(at, String::new());
        }
    }

    /// Deletes a column; the last one is only emptied.
    pub fn delete_column(&mut self, col: usize) {
        if col >= self.columns() {
            return;
        }
        if self.columns() == 1 {
            self.rows.iter_mut().for_each(|r| r[0].clear());
            return;
        }
        self.align.remove(col);
        for r in &mut self.rows {
            r.remove(col);
        }
    }

    /// Moves a column (with its alignment) one left or right; returns its new index.
    pub fn move_column(&mut self, col: usize, left: bool) -> usize {
        let target = if left { col.wrapping_sub(1) } else { col + 1 };
        if target >= self.columns() || col >= self.columns() {
            return col;
        }
        self.align.swap(col, target);
        for r in &mut self.rows {
            r.swap(col, target);
        }
        target
    }

    pub fn set_align(&mut self, cols: RangeInclusive<usize>, align: Align) {
        for c in cols {
            if let Some(a) = self.align.get_mut(c) {
                *a = align;
            }
        }
    }

    /// Changes the number of body rows and columns, keeping the cells that fit.
    pub fn resize(&mut self, body_rows: usize, columns: usize) {
        let columns = columns.max(1);
        self.align.resize(columns, Align::None);
        for r in &mut self.rows {
            r.resize(columns, String::new());
        }
        self.rows.resize(body_rows.max(1) + 1, vec![String::new(); columns]);
    }

    pub fn clear(&mut self, range: CellRange) {
        for r in range.rows() {
            for c in range.cols() {
                if let Some(cell) = self.rows.get_mut(r).and_then(|row| row.get_mut(c)) {
                    cell.clear();
                }
            }
        }
    }

    /// Writes `grid` (cells of Markdown) from `at` to the right and down,
    /// adding rows and columns as needed.
    pub fn paste(&mut self, at: (usize, usize), grid: &[Vec<String>]) {
        let height = grid.len();
        let width = grid.iter().map(Vec::len).max().unwrap_or(0);
        let rows = (at.0 + height).max(self.rows.len());
        let cols = (at.1 + width).max(self.columns());
        self.resize(rows - 1, cols);
        for (i, line) in grid.iter().enumerate() {
            for (j, cell) in line.iter().enumerate() {
                self.rows[at.0 + i][at.1 + j] = cell.clone();
            }
        }
    }

    fn range_cells(&self, range: CellRange) -> Vec<Vec<String>> {
        range
            .rows()
            .filter_map(|r| self.rows.get(r))
            .map(|row| range.cols().filter_map(|c| row.get(c).cloned()).collect())
            .collect()
    }

    /// The cells as tab-separated text (cells shown as text: line breaks as spaces).
    pub fn range_tsv(&self, range: CellRange) -> String {
        self.range_cells(range)
            .iter()
            .map(|r| r.iter().map(|c| cell_to_text(c).replace(['\n', '\t'], " ")).collect::<Vec<_>>().join("\t"))
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// The cells as a Markdown table (their first row as its header).
    pub fn range_markdown(&self, range: CellRange) -> String {
        let rows = self.range_cells(range);
        let align = range.cols().filter_map(|c| self.align.get(c).copied()).collect();
        let mut t = Table { align, rows };
        if t.rows.len() == 1 {
            t.rows.push(vec![String::new(); t.columns()]);
        }
        t.to_markdown()
    }

    /// The cells as an HTML table, each cell rendered by `cell_html`.
    pub fn range_html(&self, range: CellRange, cell_html: impl Fn(&str) -> String) -> String {
        let mut out = String::from("<table>");
        for (i, r) in self.range_cells(range).iter().enumerate() {
            out.push_str("<tr>");
            let tag = if i == 0 && range.top == 0 { "th" } else { "td" };
            for c in r {
                out.push_str(&format!("<{tag}>{}</{tag}>", cell_html(c)));
            }
            out.push_str("</tr>");
        }
        out.push_str("</table>");
        out
    }
}

/// The cell (row, column) at a byte offset of a table's source; the
/// delimiter row counts as the first body row.
pub fn cell_at_offset(source: &str, offset: usize) -> (usize, usize) {
    let offset = offset.min(source.len());
    let line_index = source[..offset].matches('\n').count();
    let line_start = source[..offset].rfind('\n').map_or(0, |i| i + 1);
    let row = if line_index == 0 { 0 } else { line_index.max(2) - 1 };
    // Unescaped pipes before the offset on the line, the leading one not counting.
    let line = &source[line_start..offset];
    let mut pipes: usize = 0;
    let mut escaped = false;
    for c in line.chars() {
        match c {
            '\\' if !escaped => escaped = true,
            '|' if !escaped => pipes += 1,
            _ => escaped = false,
        }
    }
    let leading = line.trim_start().starts_with('|');
    (row, pipes.saturating_sub(usize::from(leading)))
}

/// Where a cell's text starts in the source `Table::to_markdown` writes.
pub fn cell_offset(source: &str, cell: (usize, usize)) -> usize {
    let line_index = if cell.0 == 0 { 0 } else { cell.0 + 1 };
    let mut pos = 0;
    for (i, line) in source.split('\n').enumerate() {
        if i == line_index {
            let mut pipes = 0;
            let mut escaped = false;
            for (j, c) in line.char_indices() {
                match c {
                    '\\' if !escaped => escaped = true,
                    '|' if !escaped => {
                        if pipes == cell.1 {
                            let rest = &line[j + 1..];
                            return pos + j + 1 + (rest.len() - rest.trim_start().len()).min(1);
                        }
                        pipes += 1;
                    }
                    _ => escaped = false,
                }
            }
            return pos + line.len();
        }
        pos += line.len() + 1;
    }
    source.len()
}

/// A cell's source as it is edited: `<br>` as a line break, `\|` as `|`.
pub fn cell_to_text(cell: &str) -> String {
    let mut out = String::with_capacity(cell.len());
    let mut rest = cell;
    while let Some(i) = rest.find(['<', '\\']) {
        out.push_str(&rest[..i]);
        rest = &rest[i..];
        if rest.starts_with("\\|") {
            out.push('|');
            rest = &rest[2..];
        } else if rest.starts_with("\\\\") {
            // An escaped backslash stays as it is.
            out.push_str("\\\\");
            rest = &rest[2..];
        } else if let Some(len) = br_length(rest) {
            out.push('\n');
            rest = &rest[len..];
        } else {
            out.push_str(&rest[..1]);
            rest = &rest[1..];
        }
    }
    out.push_str(rest);
    out
}

/// The length of a `<br>` tag at the start of `s`.
fn br_length(s: &str) -> Option<usize> {
    let lower = s.get(..3)?.to_ascii_lowercase();
    if lower != "<br" {
        return None;
    }
    let end = s.find('>')?;
    s[3..end].trim().trim_end_matches('/').trim().is_empty().then_some(end + 1)
}

/// An edited cell's text as stored: line breaks as `<br>`, `|` escaped
/// (unless an odd number of backslashes escapes it already).
pub fn text_to_cell(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut backslashes = 0;
    for c in text.chars() {
        match c {
            '\n' => out.push_str("<br>"),
            '\r' => {}
            '|' if backslashes % 2 == 0 => out.push_str("\\|"),
            _ => out.push(c),
        }
        backslashes = if c == '\\' { backslashes + 1 } else { 0 };
    }
    out
}

/// Tab-separated text (as spreadsheets copy it) as a grid; quoted cells may
/// hold tabs, line breaks and doubled quotes. None unless it has a tab.
pub fn parse_tsv(text: &str) -> Option<Vec<Vec<String>>> {
    if !text.contains('\t') {
        return None;
    }
    let mut grid = Vec::new();
    let mut row = Vec::new();
    let mut cell = String::new();
    let mut chars = text.trim_end_matches(['\n', '\r']).chars().peekable();
    let mut quoted = false;
    let mut at_cell_start = true;
    while let Some(c) = chars.next() {
        match c {
            '"' if at_cell_start => quoted = true,
            '"' if quoted => {
                if chars.peek() == Some(&'"') {
                    cell.push('"');
                    chars.next();
                } else {
                    quoted = false;
                }
            }
            '\t' if !quoted => row.push(std::mem::take(&mut cell)),
            '\r' if !quoted => {}
            '\n' if !quoted => {
                row.push(std::mem::take(&mut cell));
                grid.push(std::mem::take(&mut row));
            }
            _ => cell.push(c),
        }
        at_cell_start = matches!(c, '\t' | '\n') && !quoted;
    }
    row.push(cell);
    grid.push(row);
    Some(grid.into_iter().map(|r| r.iter().map(|c| text_to_cell(c.trim())).collect()).collect())
}

/// The cells of the first `<table>` of HTML, each cell's inner HTML turned
/// into Markdown by `cell`. None without a table.
pub fn parse_html_table(html: &str, cell: impl Fn(&str) -> String) -> Option<Vec<Vec<String>>> {
    let lower = html.to_ascii_lowercase();
    let start = lower.find("<table")?;
    let end = lower[start..].find("</table").map_or(lower.len(), |e| start + e);
    let mut grid = Vec::new();
    let mut pos = start;
    while let Some(tr) = lower[pos..end].find("<tr").map(|p| p + pos) {
        let tr_end = lower[tr..end].find("</tr").map_or(end, |e| tr + e);
        let mut row = Vec::new();
        let mut cpos = tr;
        loop {
            let td = [lower[cpos..tr_end].find("<td"), lower[cpos..tr_end].find("<th")]
                .into_iter()
                .flatten()
                .min()
                .map(|p| p + cpos);
            let Some(td) = td else { break };
            let Some(open_end) = lower[td..tr_end].find('>').map(|p| td + p + 1) else { break };
            let close = ["</td", "</th", "<td", "<th"]
                .iter()
                .filter_map(|t| lower[open_end..tr_end].find(t))
                .min()
                .map_or(tr_end, |p| open_end + p);
            let text = cell(&html[open_end..close]);
            row.push(text_to_cell(text.trim()));
            cpos = close.max(open_end);
            if cpos >= tr_end {
                break;
            }
            // Past this cell's end tag.
            if lower[cpos..].starts_with("</") {
                cpos = lower[cpos..tr_end].find('>').map_or(tr_end, |p| cpos + p + 1);
            }
        }
        grid.push(row);
        pos = tr_end.max(tr + 3);
    }
    (!grid.is_empty()).then_some(grid)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_and_write() {
        let t = Table::parse("|a|b|\n|:-|-:|\n|1|2|\n").unwrap();
        assert_eq!(t.align, vec![Align::Left, Align::Right]);
        assert_eq!(t.rows, vec![vec!["a", "b"], vec!["1", "2"]]);
        let mut t2 = t.clone();
        t2.rows[0][0] = "xyz".into();
        assert_eq!(t2.to_markdown(), "| xyz | b   |\n| :-- | --: |\n| 1   | 2   |");
    }

    #[test]
    fn spec_example_is_written_with_aligned_pipes() {
        let mut t = Table::parse("|a|b|\n|-|-|\n|1|2|").unwrap();
        t.rows[0][0] = "xyz".into();
        assert_eq!(t.to_markdown(), "| xyz | b   |\n| --- | --- |\n| 1   | 2   |");
    }

    #[test]
    fn wide_characters_count_double() {
        let t = Table::parse("| 日本 | a |\n|---|:-:|\n| x | y |").unwrap();
        assert_eq!(t.to_markdown(), "| 日本 | a   |\n| ---- | :-: |\n| x    | y   |");
    }

    #[test]
    fn escaped_pipes_and_breaks() {
        let t = Table::parse("| a \\| b | c<br>d |\n|---|---|\n| 1 | 2 |").unwrap();
        assert_eq!(t.rows[0], vec!["a \\| b", "c<br>d"]);
        assert_eq!(cell_to_text(&t.rows[0][0]), "a | b");
        assert_eq!(cell_to_text("c<br/>d<BR >e"), "c\nd\ne");
        assert_eq!(text_to_cell("x|y\nz"), "x\\|y<br>z");
        assert_eq!(text_to_cell(&cell_to_text("a \\| b<br>c")), "a \\| b<br>c");
        assert_eq!(text_to_cell("x\\\\|"), "x\\\\\\|");
        assert_eq!(cell_to_text("x\\\\\\|"), "x\\\\|");
    }

    #[test]
    fn missing_and_extra_cells() {
        let t = Table::parse("| a | b |\n|---|---|\n| 1 |\n| 1 | 2 | 3 |").unwrap();
        assert_eq!(t.rows[1], vec!["1", ""]);
        assert_eq!(t.rows[2], vec!["1", "2"]);
        assert!(Table::parse("| a |\nnot a delimiter").is_none());
        // A header alone gets an empty body row.
        assert_eq!(Table::parse("| a |\n|---|").unwrap().body_rows(), 1);
    }

    #[test]
    fn rows_and_columns() {
        let mut t = Table::parse("| a | b |\n|:-:|---|\n| 1 | 2 |\n| 3 | 4 |\n| 5 | 6 |").unwrap();
        t.delete_row(2);
        assert_eq!(t.rows[2], vec!["5", "6"]);
        t.insert_row(2);
        assert_eq!(t.rows[2], vec!["", ""]);
        assert_eq!(t.move_row(3, true), 2);
        assert_eq!(t.rows[2], vec!["5", "6"]);
        // The header row stays where it is.
        assert_eq!(t.move_row(1, true), 1);
        assert_eq!(t.move_column(0, false), 1);
        assert_eq!(t.align, vec![Align::None, Align::Center]);
        assert_eq!(t.rows[0], vec!["b", "a"]);
        t.insert_column(1);
        assert_eq!(t.columns(), 3);
        t.delete_column(1);
        assert_eq!(t.rows[0], vec!["b", "a"]);
    }

    #[test]
    fn a_table_keeps_a_body_row_and_a_column() {
        let mut t = Table::parse("| a |\n|---|\n| 1 |").unwrap();
        t.delete_row(1);
        assert_eq!(t.rows, vec![vec!["a"], vec![""]]);
        t.delete_column(0);
        assert_eq!(t.columns(), 1);
    }

    #[test]
    fn resize_and_clear() {
        let mut t = Table::parse("| a | b |\n|---|---|\n| 1 | 2 |\n| 3 | 4 |").unwrap();
        t.resize(3, 3);
        assert_eq!(t.rows, vec![vec!["a", "b", ""], vec!["1", "2", ""], vec!["3", "4", ""], vec!["", "", ""]]);
        t.clear(CellRange::new((1, 0), (2, 1)));
        assert_eq!(t.rows[1], vec!["", "", ""]);
        assert_eq!(t.rows[0], vec!["a", "b", ""]);
    }

    #[test]
    fn pasting_grows_the_table() {
        let mut t = Table::parse("| a | b |\n|---|---|\n| 1 | 2 |").unwrap();
        let grid = parse_tsv("x\ty\nz\tw\nu\tv\n").unwrap();
        t.paste((1, 0), &grid);
        assert_eq!(t.body_rows(), 3);
        assert_eq!(t.rows[3], vec!["u", "v"]);
        t.paste((1, 1), &[vec!["p".into(), "q".into()]]);
        assert_eq!(t.rows[1], vec!["x", "p", "q"]);
    }

    #[test]
    fn copying_ranges() {
        let t = Table::parse("| a | b |\n|:-:|---|\n| 1 | 2<br>3 |").unwrap();
        let all = CellRange::new((0, 0), (1, 1));
        assert_eq!(t.range_tsv(all), "a\tb\n1\t2 3");
        assert_eq!(
            t.range_markdown(CellRange::new((1, 0), (1, 1))),
            "| 1   | 2<br>3 |\n| :-: | ------ |\n|     |        |"
        );
        assert_eq!(
            t.range_html(all, |c| c.to_string()),
            "<table><tr><th>a</th><th>b</th></tr><tr><td>1</td><td>2<br>3</td></tr></table>"
        );
    }

    #[test]
    fn cells_and_offsets() {
        let t = Table::parse("| a | bb |\n|---|---|\n| 1 | 2 |").unwrap();
        let src = t.to_markdown();
        for cell in [(0, 0), (0, 1), (1, 0), (1, 1)] {
            let o = cell_offset(&src, cell);
            assert_eq!(cell_at_offset(&src, o), cell, "{cell:?} at {o} in {src}");
        }
        assert_eq!(&src[cell_offset(&src, (1, 1))..cell_offset(&src, (1, 1)) + 1], "2");
        // The delimiter row counts as the first body row.
        assert_eq!(cell_at_offset(&src, src.find("---").unwrap()).0, 1);
    }

    #[test]
    fn spreadsheet_and_html_grids() {
        assert_eq!(parse_tsv("no tabs"), None);
        assert_eq!(
            parse_tsv("\"a\tb\"\t\"c\"\"d\"\n1\t\"x\ny\"").unwrap(),
            vec![vec!["a\tb", "c\"d"], vec!["1", "x<br>y"]]
        );
        let html = "<meta><table><tr><th>A</th><th><b>B</b></th></tr><tr><td>1</td><td>2|3</td></tr></table>";
        let grid = parse_html_table(html, |s| s.replace("<b>", "**").replace("</b>", "**")).unwrap();
        assert_eq!(grid, vec![vec!["A", "**B**"], vec!["1", "2\\|3"]]);
        assert_eq!(parse_html_table("<p>x</p>", |s| s.to_string()), None);
    }
}
