//! Laying out the text of the block being edited: measured runs of styled
//! text and objects (images, formulas) broken into lines of a given width,
//! with the geometry the editing widget needs - where a shown offset's caret
//! is, which offset a point hits, the rectangles of a selection, and where
//! Up, Down, Home and End go.
//!
//! Everything here works on measurements (cluster positions, object sizes),
//! so that it can be tested without fonts.

use std::ops::Range;

/// A grapheme cluster of a measured text run: its shown bytes and its x
/// position and width within the run.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Cluster {
    pub start: usize,
    pub end: usize,
    pub x: f32,
    pub w: f32,
    /// A space or tab (a line may break after it; it may hang past the end).
    pub space: bool,
    /// A line may break before it (CJK, or after a space).
    pub break_before: bool,
    /// A line may not break after it (opening brackets).
    pub no_break_after: bool,
    /// A line may not start with it (closing punctuation, small kana).
    pub no_break_before: bool,
}

/// A measured piece of the shown text.
#[derive(Debug, Clone, PartialEq)]
pub enum Measured {
    /// Text: its clusters, the height above and below its baseline, and the
    /// part of that taken by the glyphs themselves (the rest is the line
    /// height's padding, split evenly above and below them).
    Text { clusters: Vec<Cluster>, ascent: f32, descent: f32, glyph: (f32, f32) },
    /// One object over `shown` (an image, a formula, a label); a `centered`
    /// one alone on its line (display math) in the middle of it.
    Object { shown: Range<usize>, width: f32, ascent: f32, descent: f32, centered: bool },
    /// A line break at the shown byte `at`.
    Newline { at: usize },
    /// Something drawn that takes no place in the text (a preview) at `at`;
    /// a `block` one on a line of its own.
    Decoration { at: usize, width: f32, ascent: f32, descent: f32, block: bool },
}

/// A piece of a line: clusters `clusters` of item `item` (all of an object).
#[derive(Debug, Clone, PartialEq)]
pub struct Frag {
    pub item: usize,
    pub clusters: Range<usize>,
    /// Its left edge on the line.
    pub x: f32,
    pub width: f32,
    /// Where it starts within its run (the x of its first cluster).
    pub run_x: f32,
    pub shown: Range<usize>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Line {
    pub top: f32,
    pub height: f32,
    /// The baseline's y.
    pub baseline: f32,
    pub frags: Vec<Frag>,
    /// The shown bytes on the line (with its trailing spaces and line break).
    pub shown: Range<usize>,
    /// Ends with a line break (not wrapped).
    pub hard: bool,
    pub width: f32,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Flow {
    pub lines: Vec<Line>,
    pub height: f32,
    /// The shown text's length.
    pub len: usize,
}

/// One unit of line breaking.
#[derive(Debug, Clone)]
struct Atom {
    item: usize,
    cluster: usize,
    shown: Range<usize>,
    w: f32,
    ascent: f32,
    descent: f32,
    /// The glyphs' ascent and descent, for text.
    glyph: Option<(f32, f32)>,
    space: bool,
    newline: bool,
    decoration: bool,
    /// On a line of its own.
    block: bool,
    /// In the middle of its line when alone on it.
    centered: bool,
    break_before: bool,
    no_break_after: bool,
    no_break_before: bool,
}

/// Breaks `items` into lines of `width`; an empty line has the height of
/// `empty` (ascent, descent).
pub fn layout(items: &[Measured], width: f32, empty: (f32, f32), len: usize) -> Flow {
    let mut atoms: Vec<Atom> = Vec::new();
    for (i, item) in items.iter().enumerate() {
        match item {
            Measured::Text { clusters, ascent, descent, glyph } => {
                for (c, cl) in clusters.iter().enumerate() {
                    atoms.push(Atom {
                        item: i,
                        cluster: c,
                        shown: cl.start..cl.end,
                        w: cl.w,
                        ascent: *ascent,
                        descent: *descent,
                        glyph: Some(*glyph),
                        space: cl.space,
                        newline: false,
                        decoration: false,
                        block: false,
                        centered: false,
                        break_before: cl.break_before,
                        no_break_after: cl.no_break_after,
                        no_break_before: cl.no_break_before,
                    });
                }
            }
            Measured::Object { shown, width, ascent, descent, centered } => atoms.push(Atom {
                item: i,
                cluster: 0,
                shown: shown.clone(),
                w: *width,
                ascent: *ascent,
                descent: *descent,
                glyph: None,
                space: false,
                newline: false,
                decoration: false,
                block: false,
                centered: *centered,
                break_before: true,
                no_break_after: false,
                no_break_before: false,
            }),
            Measured::Decoration { at, width, ascent, descent, block } => atoms.push(Atom {
                item: i,
                cluster: 0,
                shown: *at..*at,
                w: *width,
                ascent: *ascent,
                descent: *descent,
                glyph: None,
                space: false,
                newline: false,
                decoration: true,
                block: *block,
                centered: false,
                break_before: true,
                no_break_after: false,
                no_break_before: false,
            }),
            Measured::Newline { at } => atoms.push(Atom {
                item: i,
                cluster: 0,
                shown: *at..*at + 1,
                w: 0.0,
                ascent: empty.0,
                descent: empty.1,
                glyph: None,
                space: false,
                newline: true,
                decoration: false,
                block: false,
                centered: false,
                break_before: false,
                no_break_after: false,
                no_break_before: false,
            }),
        }
    }
    // A break is allowed before atom `i` when...
    let can_break_before = |i: usize| {
        let (prev, next) = (&atoms[i - 1], &atoms[i]);
        if next.space || prev.no_break_after || next.no_break_before {
            return false;
        }
        // After a space, before or after CJK characters and objects.
        prev.space || next.break_before || prev.break_before
    };
    let mut lines: Vec<Vec<usize>> = Vec::new();
    let mut i = 0;
    while i < atoms.len() {
        let start = i;
        let mut x = 0.0;
        let mut last_break: Option<usize> = None;
        let mut end = atoms.len();
        while i < atoms.len() {
            let a = &atoms[i];
            if a.newline {
                end = i + 1;
                i += 1;
                break;
            }
            // A block decoration: alone on its line.
            if a.block {
                if i > start {
                    end = i;
                } else {
                    end = i + 1;
                    i += 1;
                }
                break;
            }
            if i > start && atoms[i - 1].block {
                end = i;
                break;
            }
            if i > start && can_break_before(i) {
                last_break = Some(i);
            }
            if x + a.w > width && i > start && !a.space {
                end = last_break.filter(|&b| b > start).unwrap_or(i);
                i = end;
                break;
            }
            x += a.w;
            i += 1;
            end = i;
        }
        lines.push((start..end).collect());
    }
    let ends_with_newline = atoms.last().is_some_and(|a| a.newline);
    if lines.is_empty() || ends_with_newline {
        lines.push(Vec::new());
    }

    let mut flow = Flow { lines: Vec::new(), height: 0.0, len };
    let mut top = 0.0;
    for line in lines {
        let (ascent, descent) = line_metrics(&atoms, &line, empty);
        let mut frags: Vec<Frag> = Vec::new();
        let mut x = 0.0;
        for &a in &line {
            let atom = &atoms[a];
            if atom.newline {
                continue;
            }
            match frags.last_mut() {
                Some(f) if f.item == atom.item && f.clusters.end == atom.cluster && !atom.decoration => {
                    f.clusters.end += 1;
                    f.width += atom.w;
                    f.shown.end = atom.shown.end;
                }
                _ => {
                    let run_x = match &items[atom.item] {
                        Measured::Text { clusters, .. } => clusters[atom.cluster].x,
                        _ => 0.0,
                    };
                    frags.push(Frag {
                        item: atom.item,
                        clusters: atom.cluster..atom.cluster + 1,
                        x,
                        width: atom.w,
                        run_x,
                        shown: atom.shown.clone(),
                    });
                }
            }
            x += atom.w;
        }
        // Display math alone on its line goes in the middle of it.
        let mut content = line.iter().filter(|&&a| !atoms[a].newline);
        if let (Some(&only), None) = (content.next(), content.next())
            && atoms[only].centered
            && width.is_finite()
        {
            let offset = ((width - x) / 2.0).max(0.0);
            for f in &mut frags {
                f.x += offset;
            }
            x += offset;
        }
        let start = line.first().map(|&a| atoms[a].shown.start);
        let end = line.last().map(|&a| atoms[a].shown.end);
        let prev_end = flow.lines.last().map_or(0, |l: &Line| l.shown.end);
        let shown = start.unwrap_or(prev_end)..end.unwrap_or(prev_end);
        let hard = line.last().is_some_and(|&a| atoms[a].newline);
        flow.lines.push(Line { top, height: ascent + descent, baseline: top + ascent, frags, shown, hard, width: x });
        top += ascent + descent;
    }
    flow.height = top;
    flow
}

/// The height above and below the baseline of a line of atoms. The text on
/// it is laid out like one paragraph of mixed fonts: the line is as tall as
/// its tallest run, and the glyphs - the tallest ascent and the deepest
/// descent among all fonts - are centered in it. Objects reach as far as they
/// do; an empty line has the height of `empty`.
fn line_metrics(atoms: &[Atom], line: &[usize], empty: (f32, f32)) -> (f32, f32) {
    let (mut ascent, mut descent) = (0.0f32, 0.0f32);
    let (mut height, mut glyph_ascent, mut glyph_descent) = (0.0f32, 0.0f32, 0.0f32);
    let mut text = false;
    let mut content = false;
    for a in line.iter().map(|&a| &atoms[a]).filter(|a| !a.newline) {
        content = true;
        if let Some((glyph_a, glyph_d)) = a.glyph {
            text = true;
            height = height.max(a.ascent + a.descent);
            glyph_ascent = glyph_ascent.max(glyph_a);
            glyph_descent = glyph_descent.max(glyph_d);
        } else {
            ascent = ascent.max(a.ascent);
            descent = descent.max(a.descent);
        }
    }
    if text {
        let above = (height - (glyph_ascent + glyph_descent)) / 2.0 + glyph_ascent;
        ascent = ascent.max(above);
        descent = descent.max(height - above);
    }
    if content { (ascent, descent) } else { empty }
}

/// Where a caret is: its x, and the line it is on.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Caret {
    pub x: f32,
    pub line: usize,
}

impl Flow {
    /// The line a shown offset is on: at a wrap, the line after it.
    pub fn line_of(&self, d: usize) -> usize {
        for (i, l) in self.lines.iter().enumerate() {
            let last = i + 1 == self.lines.len();
            if d < l.shown.end || last {
                return i;
            }
            if d == l.shown.end && !l.hard && self.lines.get(i + 1).is_none_or(|n| n.shown.start > d) {
                return i;
            }
        }
        0
    }

    /// The caret at the shown offset `d`.
    pub fn caret(&self, items: &[Measured], d: usize) -> Caret {
        let line = self.line_of(d);
        let l = &self.lines[line];
        for f in &l.frags {
            if d < f.shown.start {
                return Caret { x: f.x, line };
            }
            if d <= f.shown.end {
                return Caret { x: f.x + offset_in(items, f, d), line };
            }
        }
        Caret { x: l.frags.last().map_or(0.0, |f| f.x + f.width), line }
    }

    /// The shown offset at the point (`x`, `y`) of the layout.
    pub fn hit(&self, items: &[Measured], x: f32, y: f32) -> usize {
        let line = self.lines.iter().position(|l| y < l.top + l.height).unwrap_or(self.lines.len().saturating_sub(1));
        self.hit_line(items, line, x)
    }

    /// The shown offset at `x` on line `line`.
    pub fn hit_line(&self, items: &[Measured], line: usize, x: f32) -> usize {
        let Some(l) = self.lines.get(line) else { return self.len };
        for f in &l.frags {
            if f.shown.is_empty() {
                continue;
            }
            if x < f.x + f.width {
                match &items[f.item] {
                    Measured::Text { clusters, .. } => {
                        for c in &clusters[f.clusters.clone()] {
                            let cx = f.x + (c.x - f.run_x);
                            if x < cx + c.w / 2.0 {
                                return c.start;
                            }
                            if x < cx + c.w {
                                return c.end;
                            }
                        }
                        return f.shown.end;
                    }
                    _ => return if x < f.x + f.width / 2.0 { f.shown.start } else { f.shown.end },
                }
            }
        }
        self.line_end(items, line)
    }

    /// Where End puts the caret on a line: before its line break, or before
    /// the spaces a wrapped line ends with.
    pub fn line_end(&self, items: &[Measured], line: usize) -> usize {
        let Some(l) = self.lines.get(line) else { return self.len };
        if l.hard {
            return l.shown.end - 1;
        }
        if line + 1 < self.lines.len() {
            // Wrapped: after the last character that is not a hanging space.
            for f in l.frags.iter().rev() {
                match &items[f.item] {
                    Measured::Text { clusters, .. } => {
                        if let Some(c) = clusters[f.clusters.clone()].iter().rev().find(|c| !c.space) {
                            return c.end;
                        }
                    }
                    Measured::Decoration { .. } => {}
                    _ => return f.shown.end,
                }
            }
            return l.shown.start;
        }
        l.shown.end
    }

    /// Where Home puts the caret on a line.
    pub fn line_start(&self, line: usize) -> usize {
        self.lines.get(line).map_or(0, |l| l.shown.start)
    }

    /// The offset one line above / below `d` at the horizontal position
    /// `x`; None from the first / last line.
    pub fn vertical(&self, items: &[Measured], d: usize, x: f32, down: bool) -> Option<usize> {
        let line = self.line_of(d);
        let target = if down { line + 1 } else { line.checked_sub(1)? };
        if target >= self.lines.len() {
            return None;
        }
        Some(self.hit_line(items, target, x))
    }

    /// The rectangles (x, top, width, height) covering the shown range: the
    /// height of the text (or object) they cover, through the end of a line
    /// whose break the range takes in.
    pub fn rects(&self, items: &[Measured], range: Range<usize>) -> Vec<(f32, f32, f32, f32)> {
        let mut out: Vec<(f32, f32, f32, f32)> = Vec::new();
        if range.is_empty() {
            return out;
        }
        for (i, l) in self.lines.iter().enumerate() {
            if l.shown.end <= range.start || l.shown.start >= range.end {
                continue;
            }
            let mut line_rects: Vec<(f32, f32, f32, f32)> = Vec::new();
            for f in &l.frags {
                if f.shown.is_empty() || f.shown.end <= range.start || f.shown.start >= range.end {
                    continue;
                }
                let x0 = f.x + offset_in(items, f, range.start.max(f.shown.start));
                let x1 = f.x + offset_in(items, f, range.end.min(f.shown.end));
                let (ascent, descent) = metrics(&items[f.item]);
                let rect = (x0, l.baseline - ascent, x1 - x0, ascent + descent);
                match line_rects.last_mut() {
                    Some(r) if r.1 == rect.1 && r.3 == rect.3 && (r.0 + r.2 - rect.0).abs() < 0.5 => r.2 += rect.2,
                    _ => line_rects.push(rect),
                }
            }
            let takes_break = range.end >= l.shown.end && (l.hard || i + 1 < self.lines.len());
            if takes_break {
                match line_rects.last_mut() {
                    Some(r) => r.2 += 4.0,
                    None => line_rects.push((l.width, l.top, 4.0, l.height)),
                }
            }
            out.extend(line_rects.into_iter().filter(|r| r.2 > 0.0));
        }
        out
    }
}

/// The height above and below the baseline of a measured piece.
fn metrics(item: &Measured) -> (f32, f32) {
    match item {
        Measured::Text { ascent, descent, .. }
        | Measured::Object { ascent, descent, .. }
        | Measured::Decoration { ascent, descent, .. } => (*ascent, *descent),
        Measured::Newline { .. } => (0.0, 0.0),
    }
}

/// The x of shown offset `d` within fragment `f`.
fn offset_in(items: &[Measured], f: &Frag, d: usize) -> f32 {
    match &items[f.item] {
        Measured::Text { clusters, .. } => {
            for c in &clusters[f.clusters.clone()] {
                if d <= c.start {
                    return c.x - f.run_x;
                }
                if d < c.end {
                    // Inside a ligature: in proportion.
                    let t = (d - c.start) as f32 / (c.end - c.start) as f32;
                    return c.x - f.run_x + c.w * t;
                }
            }
            f.width
        }
        _ => {
            if d <= f.shown.start {
                0.0
            } else {
                f.width
            }
        }
    }
}

/// Characters a line may not start with (Japanese kinsoku and closing punctuation).
pub fn no_break_before(c: char) -> bool {
    "、。，．,.)）]］｝}〕〉》」』】〙〗〟’”｠»ゝゞーァィゥェォッャュョヮヵヶぁぃぅぇぉっゃゅょゎゕゖㇰㇱㇲㇳㇴㇵㇶㇷㇸㇹㇺㇻㇼㇽㇾㇿ々〻‐゠–〜？！?!‼⁇⁈⁉・:;／"
        .contains(c)
}

/// Characters a line may not end with (opening brackets).
pub fn no_break_after(c: char) -> bool {
    "(（[［｛{〔〈《「『【〘〖〝‘“｟«".contains(c)
}

/// Characters lines may break around without spaces (CJK).
pub fn is_cjk(c: char) -> bool {
    matches!(c as u32, 0x2E80..=0x9FFF | 0xAC00..=0xD7AF | 0xF900..=0xFAFF | 0xFE30..=0xFE4F | 0xFF00..=0xFFEF | 0x20000..=0x2FFFF)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A text run of `text` starting at shown byte `at`, every character 10 wide.
    fn run(text: &str, at: usize) -> Measured {
        let mut clusters = Vec::new();
        let mut x = 0.0;
        for (i, c) in text.char_indices() {
            clusters.push(Cluster {
                start: at + i,
                end: at + i + c.len_utf8(),
                x,
                w: 10.0,
                space: c == ' ',
                break_before: is_cjk(c),
                no_break_after: no_break_after(c),
                no_break_before: no_break_before(c),
            });
            x += 10.0;
        }
        Measured::Text { clusters, ascent: 12.0, descent: 4.0, glyph: (10.0, 2.0) }
    }

    fn lines(flow: &Flow, text: &str) -> Vec<String> {
        flow.lines.iter().map(|l| text[l.shown.clone()].to_string()).collect()
    }

    #[test]
    fn breaks_after_spaces() {
        let text = "aaa bbb ccc";
        let items = [run(text, 0)];
        let flow = layout(&items, 75.0, (12.0, 4.0), text.len());
        assert_eq!(lines(&flow, text), vec!["aaa bbb ", "ccc"]);
        assert_eq!(flow.lines[1].top, 16.0);
        assert_eq!(flow.height, 32.0);
        // The space hangs: the first line is 70 wide without it... and 80 with it.
        assert_eq!(flow.lines[0].width, 80.0);
    }

    #[test]
    fn breaks_between_cjk_characters_but_not_before_closing_punctuation() {
        let text = "日本語の文章。です";
        let items = [run(text, 0)];
        let flow = layout(&items, 60.0, (12.0, 4.0), text.len());
        // Six fit on the first line, but `。` may not start the second.
        assert_eq!(lines(&flow, text), vec!["日本語の文", "章。です"]);
    }

    #[test]
    fn a_word_longer_than_the_line_breaks_anywhere() {
        let text = "abcdefgh";
        let items = [run(text, 0)];
        let flow = layout(&items, 35.0, (12.0, 4.0), text.len());
        assert_eq!(lines(&flow, text), vec!["abc", "def", "gh"]);
    }

    #[test]
    fn styled_runs_and_objects_share_lines_and_baselines() {
        // "ab" + bold "cd" + image + " ef"
        let items = [
            run("ab", 0),
            Measured::Text {
                clusters: match run("cd", 2) {
                    Measured::Text { clusters, .. } => clusters,
                    _ => unreachable!(),
                },
                ascent: 20.0,
                descent: 6.0,
                glyph: (18.0, 4.0),
            },
            Measured::Object { shown: 4..7, width: 30.0, ascent: 40.0, descent: 0.0, centered: false },
            run(" ef", 7),
        ];
        let flow = layout(&items, 1000.0, (12.0, 4.0), 10);
        assert_eq!(flow.lines.len(), 1);
        let l = &flow.lines[0];
        assert_eq!((l.baseline, l.height), (40.0, 46.0));
        assert_eq!(l.frags.len(), 4);
        assert_eq!(l.frags[2].x, 40.0);
        // Carets: inside text, before and after the object.
        assert_eq!(flow.caret(&items, 1).x, 10.0);
        assert_eq!(flow.caret(&items, 4).x, 40.0);
        assert_eq!(flow.caret(&items, 7).x, 70.0);
        // Hits: halves of characters and of the object.
        assert_eq!(flow.hit(&items, 14.0, 5.0), 1);
        assert_eq!(flow.hit(&items, 16.0, 5.0), 2);
        assert_eq!(flow.hit(&items, 50.0, 5.0), 4);
        assert_eq!(flow.hit(&items, 60.0, 5.0), 7);
        assert_eq!(flow.hit(&items, 500.0, 5.0), 10);
    }

    #[test]
    fn line_breaks_and_empty_lines() {
        let text = "ab\n\ncd\n";
        let items = [
            run("ab", 0),
            Measured::Newline { at: 2 },
            Measured::Newline { at: 3 },
            run("cd", 4),
            Measured::Newline { at: 6 },
        ];
        let flow = layout(&items, 100.0, (12.0, 4.0), text.len());
        assert_eq!(flow.lines.len(), 4);
        assert_eq!(flow.line_of(2), 0);
        assert_eq!(flow.line_of(3), 1);
        assert_eq!(flow.line_of(4), 2);
        assert_eq!(flow.line_of(7), 3);
        assert_eq!(flow.lines[1].height, 16.0);
        assert_eq!(flow.line_end(&items, 0), 2);
        assert_eq!(flow.line_start(2), 4);
        // Up and Down keep the x.
        assert_eq!(flow.vertical(&items, 5, 10.0, false), Some(3));
        assert_eq!(flow.vertical(&items, 1, 10.0, true), Some(3));
        assert_eq!(flow.vertical(&items, 1, 10.0, false), None);
        assert_eq!(flow.vertical(&items, 7, 0.0, true), None);
    }

    #[test]
    fn the_caret_at_a_wrap_is_on_the_next_line() {
        let text = "aaa bbb";
        let items = [run(text, 0)];
        let flow = layout(&items, 45.0, (12.0, 4.0), text.len());
        assert_eq!(lines(&flow, text), vec!["aaa ", "bbb"]);
        assert_eq!(flow.caret(&items, 4), Caret { x: 0.0, line: 1 });
        assert_eq!(flow.caret(&items, 3), Caret { x: 30.0, line: 0 });
        assert_eq!(flow.line_end(&items, 0), 3);
        assert_eq!(flow.caret(&items, 7), Caret { x: 30.0, line: 1 });
    }

    #[test]
    fn selection_rectangles() {
        let text = "aaa bbb";
        let items = [run(text, 0)];
        let flow = layout(&items, 45.0, (12.0, 4.0), text.len());
        assert_eq!(flow.rects(&items, 1..5), vec![(10.0, 0.0, 34.0, 16.0), (0.0, 16.0, 10.0, 16.0)]);
        assert!(flow.rects(&items, 2..2).is_empty());
    }

    #[test]
    fn a_block_preview_has_a_line_of_its_own() {
        let items = [
            run("ab", 0),
            Measured::Decoration { at: 2, width: 50.0, ascent: 40.0, descent: 0.0, block: true },
            run("cd", 2),
        ];
        let flow = layout(&items, 1000.0, (12.0, 4.0), 4);
        assert_eq!(flow.lines.len(), 3);
        assert_eq!(flow.lines[1].height, 40.0);
        assert_eq!(flow.lines[2].frags[0].item, 2);
        // The caret skips the preview.
        assert_eq!(flow.caret(&items, 2).line, 2);
    }

    #[test]
    fn display_math_alone_on_its_line_is_centered() {
        let math = |at: usize| Measured::Object {
            shown: at..at + 3,
            width: 40.0,
            ascent: 20.0,
            descent: 10.0,
            centered: true,
        };
        let items = [run("ab", 0), Measured::Newline { at: 2 }, math(3), Measured::Newline { at: 6 }, run("cd", 7)];
        let flow = layout(&items, 200.0, (12.0, 4.0), 9);
        assert_eq!(flow.lines[1].frags[0].x, 80.0);
        assert_eq!(flow.caret(&items, 3).x, 80.0);
        // Among text it stays where it is.
        let items = [run("ab", 0), math(2)];
        assert_eq!(layout(&items, 200.0, (12.0, 4.0), 5).lines[0].frags[1].x, 20.0);
    }

    #[test]
    fn empty_text_has_one_line() {
        let flow = layout(&[], 100.0, (12.0, 4.0), 0);
        assert_eq!(flow.lines.len(), 1);
        assert_eq!(flow.height, 16.0);
        assert_eq!(flow.caret(&[], 0), Caret { x: 0.0, line: 0 });
        assert_eq!(flow.hit(&[], 50.0, 50.0), 0);
    }
}
