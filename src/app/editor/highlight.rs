//! How Markdown source looks in the text editor (the active region and the
//! source mode): markers dimmed, emphasis in bold and italic fonts, code in
//! the code font, links in the link color, fenced code highlighted for its
//! language. Also the syntax highlighting of rendered code blocks.

use super::view::{CODE_FONT, Palette};
use crate::app::view::APP_FONT;
use iced::advanced::text::highlighter::{self, Format};
use iced::{Color, Font, Theme, font};
use std::ops::Range;
use std::sync::LazyLock;
use syntect::highlighting::{HighlightIterator, HighlightState, Highlighter as SyntectHighlighter, Theme as CodeTheme};
use syntect::parsing::{ParseState, ScopeStack, SyntaxReference, SyntaxSet};

static SYNTAXES: LazyLock<SyntaxSet> = LazyLock::new(two_face::syntax::extra_newlines);
static THEMES: LazyLock<two_face::theme::EmbeddedLazyThemeSet> = LazyLock::new(two_face::theme::extra);

fn code_theme(dark: bool) -> &'static CodeTheme {
    use two_face::theme::EmbeddedThemeName;
    THEMES.get(if dark { EmbeddedThemeName::OneHalfDark } else { EmbeddedThemeName::InspiredGithub })
}

fn syntax(lang: &str) -> Option<&'static SyntaxReference> {
    let lang = lang.trim();
    if lang.is_empty() {
        return None;
    }
    SYNTAXES.find_syntax_by_token(lang).or_else(|| SYNTAXES.find_syntax_by_token(&lang.to_lowercase()))
}

fn color(c: syntect::highlighting::Color) -> Color {
    Color::from_rgba8(c.r, c.g, c.b, f32::from(c.a) / 255.0)
}

/// Code highlighted for `lang`: per line, colored pieces (none when the
/// language is unknown).
pub fn highlight_code(code: &str, lang: Option<&str>, dark: bool) -> Option<Vec<Vec<(String, Color)>>> {
    let syntax = syntax(lang?)?;
    let theme = code_theme(dark);
    let mut lines = syntect::easy::HighlightLines::new(syntax, theme);
    let mut out = Vec::new();
    for line in syntect::util::LinesWithEndings::from(code) {
        let pieces = lines.highlight_line(line, &SYNTAXES).ok()?;
        out.push(
            pieces
                .into_iter()
                .map(|(style, text)| (text.trim_end_matches('\n').to_string(), color(style.foreground)))
                .filter(|(t, _)| !t.is_empty())
                .collect(),
        );
    }
    Some(out)
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Highlight {
    Marker,
    /// A marker set in the font of what it marks: the text layout picks one
    /// font per word, from the word's first character, so `**bold**` is bold
    /// only when its first `*` is.
    MarkerIn(Font),
    Heading,
    Strong,
    Emphasis,
    StrongEmphasis,
    Code,
    Link,
    Url,
    Html,
    Math,
    Strike,
    Mark,
    Fence,
    CodeText,
    Syntax(Color),
    Match,
    FrontMatter,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Settings {
    pub dark: bool,
    /// Whether the first line is the start of the page (front matter may start there).
    pub page_start: bool,
    /// The find bar's query and whether it matches case.
    pub find: Option<(String, bool)>,
    /// The text is the code of a code block in this language (empty: none),
    /// not Markdown.
    pub code: Option<String>,
}

impl Settings {
    /// The state the first line starts in: plain code of a language is the
    /// inside of a fence that never closes.
    fn start(&self) -> State {
        match &self.code {
            Some(lang) => State::Fence {
                ch: '`',
                len: usize::MAX,
                code: syntax(lang).map(|s| {
                    let highlighter = SyntectHighlighter::new(code_theme(self.dark));
                    Box::new((ParseState::new(s), HighlightState::new(&highlighter, ScopeStack::new())))
                }),
            },
            None => State::Normal,
        }
    }
}

pub fn format(h: &Highlight, theme: &Theme) -> Format<Font> {
    let p = Palette::of(theme);
    let bold = Font { weight: font::Weight::Bold, ..APP_FONT };
    let italic = Font { style: font::Style::Italic, ..APP_FONT };
    let (color, font) = match h {
        Highlight::Marker => (Some(p.muted), None),
        Highlight::MarkerIn(f) => (Some(p.muted), Some(*f)),
        Highlight::Heading => (Some(p.heading), Some(bold)),
        Highlight::Strong => (None, Some(bold)),
        Highlight::Emphasis => (None, Some(italic)),
        Highlight::StrongEmphasis => (None, Some(Font { style: font::Style::Italic, ..bold })),
        Highlight::Code => (Some(p.code), Some(CODE_FONT)),
        Highlight::Link => (Some(p.link), None),
        Highlight::Url => (Some(p.muted), None),
        Highlight::Html => (Some(p.html), None),
        Highlight::Math => (Some(p.math), Some(CODE_FONT)),
        Highlight::Strike => (Some(p.muted), None),
        Highlight::Mark => (Some(p.mark), None),
        Highlight::Fence => (Some(p.muted), Some(CODE_FONT)),
        Highlight::CodeText => (None, Some(CODE_FONT)),
        Highlight::Syntax(c) => (Some(*c), Some(CODE_FONT)),
        Highlight::Match => (Some(p.find), Some(bold)),
        Highlight::FrontMatter => (Some(p.muted), Some(CODE_FONT)),
    };
    Format { color, font }
}

/// What the lines before a line leave open.
#[derive(Clone)]
enum State {
    Normal,
    Fence { ch: char, len: usize, code: Option<Box<(ParseState, HighlightState)>> },
    Math,
    FrontMatter,
}

pub struct MarkdownHighlighter {
    settings: Settings,
    /// The state at the start of each line highlighted so far.
    states: Vec<State>,
    current: usize,
}

impl highlighter::Highlighter for MarkdownHighlighter {
    type Settings = Settings;
    type Highlight = Highlight;
    type Iterator<'a> = std::vec::IntoIter<(Range<usize>, Highlight)>;

    fn new(settings: &Settings) -> Self {
        MarkdownHighlighter { settings: settings.clone(), states: vec![settings.start()], current: 0 }
    }

    fn update(&mut self, new_settings: &Settings) {
        self.settings = new_settings.clone();
        self.states = vec![self.settings.start()];
        self.change_line(0);
    }

    fn change_line(&mut self, line: usize) {
        let line = line.min(self.states.len() - 1);
        self.states.truncate(line + 1);
        self.current = line;
    }

    fn highlight_line(&mut self, line: &str) -> Self::Iterator<'_> {
        let state = self.states[self.current.min(self.states.len() - 1)].clone();
        let (spans, next) = highlight_line(line, state, self.current == 0 && self.settings.page_start, &self.settings);
        self.states.truncate(self.current + 1);
        self.states.push(next);
        self.current += 1;
        spans.into_iter()
    }

    fn current_line(&self) -> usize {
        self.current
    }
}

/// The opening of a code fence: its character, length and info string.
fn fence(line: &str) -> Option<(char, usize, &str)> {
    let t = line.trim_start_matches(' ');
    if line.len() - t.len() > 3 {
        return None;
    }
    let ch = t.chars().next().filter(|c| *c == '`' || *c == '~')?;
    let len = t.len() - t.trim_start_matches(ch).len();
    let info = t[len..].trim();
    (len >= 3 && !(ch == '`' && info.contains('`'))).then_some((ch, len, info))
}

fn highlight_line(
    line: &str,
    state: State,
    page_start: bool,
    settings: &Settings,
) -> (Vec<(Range<usize>, Highlight)>, State) {
    let whole = |h: Highlight| vec![(0..line.len(), h)];
    let (mut spans, next) = match state {
        State::Fence { ch, len, code } => {
            let t = line.trim();
            let closes = t.len() >= len && t.chars().all(|c| c == ch);
            if closes {
                (whole(Highlight::Fence), State::Normal)
            } else {
                match code {
                    Some(boxed) => {
                        let (mut parse, mut hl) = *boxed;
                        let theme = code_theme(settings.dark);
                        let highlighter = SyntectHighlighter::new(theme);
                        let with_newline = format!("{line}\n");
                        let mut spans = Vec::new();
                        if let Ok(ops) = parse.parse_line(&with_newline, &SYNTAXES) {
                            let mut pos = 0;
                            for (style, piece) in HighlightIterator::new(&mut hl, &ops, &with_newline, &highlighter) {
                                let end = (pos + piece.len()).min(line.len());
                                if pos < end {
                                    spans.push((pos..end, Highlight::Syntax(color(style.foreground))));
                                }
                                pos += piece.len();
                            }
                        }
                        (spans, State::Fence { ch, len, code: Some(Box::new((parse, hl))) })
                    }
                    None => (whole(Highlight::CodeText), State::Fence { ch, len, code: None }),
                }
            }
        }
        State::Math => {
            if line.trim() == "$$" {
                (whole(Highlight::Marker), State::Normal)
            } else {
                (whole(Highlight::Math), State::Math)
            }
        }
        State::FrontMatter => {
            let t = line.trim_end();
            if t == "---" || t == "..." {
                (whole(Highlight::Marker), State::Normal)
            } else {
                (whole(Highlight::FrontMatter), State::FrontMatter)
            }
        }
        State::Normal => {
            if let Some((ch, len, info)) = fence(line) {
                let code = info.split_whitespace().next().and_then(syntax).map(|s| {
                    let highlighter = SyntectHighlighter::new(code_theme(settings.dark));
                    Box::new((ParseState::new(s), HighlightState::new(&highlighter, ScopeStack::new())))
                });
                (whole(Highlight::Fence), State::Fence { ch, len, code })
            } else if line.trim() == "$$" {
                (whole(Highlight::Marker), State::Math)
            } else if page_start && line.trim_end() == "---" {
                (whole(Highlight::Marker), State::FrontMatter)
            } else {
                (normal_line(line), State::Normal)
            }
        }
    };
    if let Some((query, case)) = &settings.find {
        for m in crate::markdown::find::find_all(line, query, *case) {
            spans.push((m, Highlight::Match));
        }
    }
    (spans, next)
}

/// A line outside code: block markers, then inline syntax.
fn normal_line(line: &str) -> Vec<(Range<usize>, Highlight)> {
    let mut spans = Vec::new();
    let t = line.trim();
    // Thematic breaks and setext underlines.
    let rule = |c: char| t.chars().filter(|x| !x.is_whitespace()).count() >= 3 && t.chars().all(|x| x == c || x == ' ');
    if !t.is_empty() && (rule('-') || rule('*') || rule('_') || t.chars().all(|c| c == '=')) {
        return vec![(0..line.len(), Highlight::Marker)];
    }
    let mut pos = 0;
    let mut heading = false;
    // Quote markers, list markers, a task box, a heading marker.
    loop {
        let rest = &line[pos..];
        let spaces = rest.len() - rest.trim_start_matches([' ', '\t']).len();
        let after = &rest[spaces..];
        if after.starts_with('>') {
            spans.push((pos + spaces..pos + spaces + 1, Highlight::Marker));
            pos += spaces + 1;
            continue;
        }
        if let Some(m) = crate::markdown::edit::line_marker(after) {
            use crate::markdown::edit::MarkerKind;
            match m.kind {
                MarkerKind::Heading(_) => {
                    spans.push((pos + spaces..pos + spaces + m.end, Highlight::Marker));
                    heading = true;
                    pos += spaces + m.end;
                }
                MarkerKind::Bullet(_) | MarkerKind::Ordered(..) => {
                    spans.push((pos + spaces..pos + spaces + m.end, Highlight::Marker));
                    pos += spaces + m.end;
                    continue;
                }
                MarkerKind::Quote => {}
            }
        }
        break;
    }
    let base = heading.then_some(Highlight::Heading);
    if let Some(b) = base {
        spans.push((pos..line.len(), b));
    }
    inline(line, pos, line.len(), base, &mut spans);
    spans
}

/// The marker of text highlighted `h`: in the font `h` is set in.
fn marker_for(h: Highlight) -> Highlight {
    let bold = Font { weight: font::Weight::Bold, ..APP_FONT };
    match h {
        Highlight::Strong | Highlight::Heading => Highlight::MarkerIn(bold),
        Highlight::Emphasis => Highlight::MarkerIn(Font { style: font::Style::Italic, ..APP_FONT }),
        Highlight::StrongEmphasis => Highlight::MarkerIn(Font { style: font::Style::Italic, ..bold }),
        Highlight::Code | Highlight::Math => Highlight::MarkerIn(CODE_FONT),
        _ => Highlight::Marker,
    }
}

/// The inline syntax of `line[from..to]`.
fn inline(line: &str, from: usize, to: usize, base: Option<Highlight>, spans: &mut Vec<(Range<usize>, Highlight)>) {
    let b = line.as_bytes();
    let mut i = from;
    let strong = |base: Option<Highlight>| match base {
        Some(Highlight::Emphasis) => Highlight::StrongEmphasis,
        Some(Highlight::Heading) => Highlight::Heading,
        _ => Highlight::Strong,
    };
    let emphasis = |base: Option<Highlight>| match base {
        Some(Highlight::Strong) => Highlight::StrongEmphasis,
        _ => Highlight::Emphasis,
    };
    while i < to {
        let c = b[i];
        match c {
            b'\\' if i + 1 < to && b[i + 1].is_ascii_punctuation() => {
                spans.push((i..i + 1, Highlight::Marker));
                i += 2;
            }
            b'`' => {
                let n = line[i..to].len() - line[i..to].trim_start_matches('`').len();
                let ticks = &line[i..i + n];
                match find_run(line, i + n, to, ticks) {
                    Some(close) => {
                        spans.push((i..i + n, marker_for(Highlight::Code)));
                        spans.push((i + n..close, Highlight::Code));
                        spans.push((close..close + n, marker_for(Highlight::Code)));
                        i = close + n;
                    }
                    None => i += n,
                }
            }
            b'$' if i + 1 < to && b[i + 1] != b'$' && b[i + 1] != b' ' => match line[i + 1..to].find('$') {
                Some(e) => {
                    let close = i + 1 + e;
                    spans.push((i..i + 1, marker_for(Highlight::Math)));
                    spans.push((i + 1..close, Highlight::Math));
                    spans.push((close..close + 1, marker_for(Highlight::Math)));
                    i = close + 1;
                }
                None => i += 1,
            },
            b'*' | b'_' | b'~' | b'=' | b'^' => {
                let ch = c as char;
                let n = line[i..to].len() - line[i..to].trim_start_matches(ch).len();
                let run = &line[i..i + n];
                // `_` only opens at a word start.
                let word_before = i > 0 && line[..i].chars().next_back().is_some_and(char::is_alphanumeric);
                let opens = i + n < to && !b[i + n].is_ascii_whitespace() && !(ch == '_' && word_before);
                let close = if opens { find_run(line, i + n, to, run) } else { None };
                match (close, ch, n) {
                    (Some(close), '*' | '_', _) => {
                        let h = match n {
                            1 => emphasis(base),
                            2 => strong(base),
                            _ => Highlight::StrongEmphasis,
                        };
                        spans.push((i..i + n, marker_for(h)));
                        spans.push((i + n..close, h));
                        inline(line, i + n, close, Some(h), spans);
                        spans.push((close..close + n, marker_for(h)));
                        i = close + n;
                    }
                    (Some(close), '~' | '=' | '^', 1 | 2) if !(ch == '=' && n == 1) => {
                        let h = match (ch, n) {
                            ('~', 2) => Highlight::Strike,
                            ('=', 2) => Highlight::Mark,
                            _ => base.unwrap_or(Highlight::Marker),
                        };
                        spans.push((i..i + n, base.map_or(Highlight::Marker, marker_for)));
                        if h != Highlight::Marker {
                            spans.push((i + n..close, h));
                        }
                        inline(line, i + n, close, Some(h).filter(|h| *h != Highlight::Marker).or(base), spans);
                        spans.push((close..close + n, base.map_or(Highlight::Marker, marker_for)));
                        i = close + n;
                    }
                    _ => i += n,
                }
            }
            b'!' | b'[' => {
                let start = if c == b'!' && line[i + 1..to].starts_with('[') { i + 1 } else { i };
                if b[start] != b'[' {
                    i += 1;
                    continue;
                }
                match link_end(line, start, to) {
                    Some((text_end, dest)) => {
                        spans.push((i..start + 1, Highlight::Marker));
                        spans.push((start + 1..text_end, Highlight::Link));
                        inline(line, start + 1, text_end, Some(Highlight::Link), spans);
                        spans.push((text_end..text_end + 2, Highlight::Marker));
                        spans.push((text_end + 2..dest, Highlight::Url));
                        spans.push((dest..dest + 1, Highlight::Marker));
                        i = dest + 1;
                    }
                    None => i += 1,
                }
            }
            b'<' => match line[i..to].find('>') {
                Some(e) if e > 1 && !line[i + 1..i + e].contains(' ') || line[i..].starts_with("<span") => {
                    let end = i + e + 1;
                    let inner = &line[i + 1..i + e];
                    let h = if inner.contains("://") || inner.contains('@') { Highlight::Url } else { Highlight::Html };
                    spans.push((i..end, h));
                    i = end;
                }
                Some(e) if line[i + 1..].starts_with(|c: char| c.is_ascii_alphabetic() || c == '/') => {
                    spans.push((i..i + e + 1, Highlight::Html));
                    i += e + 1;
                }
                _ => i += 1,
            },
            b'|' => {
                spans.push((i..i + 1, Highlight::Marker));
                i += 1;
            }
            _ => i += line[i..].chars().next().map_or(1, char::len_utf8),
        }
    }
}

/// The start of the next occurrence of exactly `run` (not part of a longer run) in `line[from..to]`.
fn find_run(line: &str, from: usize, to: usize, run: &str) -> Option<usize> {
    let ch = run.chars().next()?;
    let mut i = from;
    while let Some(p) = line[i..to].find(run).map(|p| p + i) {
        let n = line[p..to].len() - line[p..to].trim_start_matches(ch).len();
        let before_ok = p == 0 || !line[..p].ends_with(ch);
        if n == run.len() && before_ok && p > from && !line.as_bytes()[p - 1].is_ascii_whitespace() {
            return Some(p);
        }
        i = p + n.max(1);
    }
    None
}

/// For `[` at `start`: the `]` of `[...](...)` and the destination's `)`.
fn link_end(line: &str, start: usize, to: usize) -> Option<(usize, usize)> {
    let mut depth = 0;
    let b = line.as_bytes();
    let mut i = start;
    let mut close = None;
    while i < to {
        match b[i] {
            b'\\' => i += 1,
            b'[' => depth += 1,
            b']' => {
                depth -= 1;
                if depth == 0 {
                    close = Some(i);
                    break;
                }
            }
            _ => {}
        }
        i += 1;
    }
    let close = close?;
    if !line[close..to].starts_with("](") {
        return None;
    }
    let mut depth = 0;
    let mut j = close + 2;
    while j < to {
        match b[j] {
            b'\\' => j += 1,
            b'(' => depth += 1,
            b')' if depth == 0 => return Some((close, j)),
            b')' => depth -= 1,
            _ => {}
        }
        j += 1;
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use iced::advanced::text::highlighter::Highlighter as _;

    fn spans(lines: &[&str]) -> Vec<Vec<(String, Highlight)>> {
        let settings = Settings { dark: false, page_start: true, find: None, code: None };
        let mut h = MarkdownHighlighter::new(&settings);
        lines.iter().map(|l| h.highlight_line(l).map(|(r, hl)| (l[r].to_string(), hl)).collect()).collect()
    }

    fn has(line: &[(String, Highlight)], text: &str, h: Highlight) -> bool {
        line.iter().any(|(t, x)| t == text && *x == h)
    }

    #[test]
    fn markers_and_emphasis() {
        let s = spans(&["## Plan **now**", "- [ ] a *b* `c` ~~d~~ ==e== [f](g) <u>h</u> $x$"]);
        assert!(has(&s[0], "## ", Highlight::Marker));
        assert!(has(&s[0], "Plan **now**", Highlight::Heading));
        assert!(s[0].iter().any(|(t, h)| t == "**" && matches!(h, Highlight::MarkerIn(_))));
        let l = &s[1];
        assert!(has(l, "- [ ] ", Highlight::Marker));
        assert!(has(l, "b", Highlight::Emphasis));
        assert!(has(l, "c", Highlight::Code));
        assert!(has(l, "d", Highlight::Strike));
        assert!(has(l, "e", Highlight::Mark));
        assert!(has(l, "f", Highlight::Link));
        assert!(has(l, "g", Highlight::Url));
        assert!(has(l, "<u>", Highlight::Html));
        assert!(has(l, "x", Highlight::Math));
    }

    #[test]
    fn snake_case_is_not_emphasis() {
        let s = spans(&["a snake_case_name here"]);
        assert!(!s[0].iter().any(|(_, h)| *h == Highlight::Emphasis));
    }

    #[test]
    fn fenced_code_is_highlighted_for_its_language() {
        let s = spans(&["```rust", "fn main() {}", "```", "after *x*"]);
        assert!(has(&s[0], "```rust", Highlight::Fence));
        assert!(s[1].iter().any(|(t, h)| t == "fn" && matches!(h, Highlight::Syntax(_))));
        assert!(has(&s[2], "```", Highlight::Fence));
        assert!(has(&s[3], "x", Highlight::Emphasis));
        let s = spans(&["```", "plain", "```"]);
        assert!(has(&s[1], "plain", Highlight::CodeText));
    }

    #[test]
    fn the_code_of_a_code_block_alone() {
        let code = |lang: &str, lines: &[&str]| {
            let settings = Settings { dark: false, page_start: true, find: None, code: Some(lang.into()) };
            let mut h = MarkdownHighlighter::new(&settings);
            lines
                .iter()
                .map(|l| h.highlight_line(l).map(|(r, hl)| (l[r].to_string(), hl)).collect())
                .collect::<Vec<Vec<(String, Highlight)>>>()
        };
        // No fence opens it, and a line of backticks doesn't close it.
        let s = code("rust", &["fn main() {}", "```", "# not a heading"]);
        assert!(s[0].iter().any(|(t, h)| t == "fn" && matches!(h, Highlight::Syntax(_))));
        assert!(!s[2].iter().any(|(_, h)| *h == Highlight::Heading));
        let s = code("", &["# plain", "**x**"]);
        assert!(has(&s[0], "# plain", Highlight::CodeText));
        assert!(has(&s[1], "**x**", Highlight::CodeText));
    }

    #[test]
    fn math_and_front_matter_blocks() {
        let s = spans(&["---", "title: x", "---", "$$", "x^2", "$$"]);
        assert!(has(&s[1], "title: x", Highlight::FrontMatter));
        assert!(has(&s[4], "x^2", Highlight::Math));
    }

    #[test]
    fn changing_a_line_restarts_from_its_state() {
        let settings = Settings { dark: false, page_start: false, find: Some(("an".into(), false)), code: None };
        let mut h = MarkdownHighlighter::new(&settings);
        let _ = h.highlight_line("```").count();
        let _ = h.highlight_line("code").count();
        h.change_line(1);
        let again: Vec<_> = h.highlight_line("code").collect();
        assert_eq!(again, vec![(0..4, Highlight::CodeText)]);
        let found: Vec<_> = h.highlight_line("```").chain(h.highlight_line("An apple")).collect();
        assert!(found.contains(&(0..2, Highlight::Match)));
    }

    #[test]
    fn code_of_a_known_language() {
        let lines = highlight_code("let x = 1;\n", Some("rust"), false).unwrap();
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].iter().map(|(t, _)| t.as_str()).collect::<String>(), "let x = 1;");
        assert!(highlight_code("x", Some("no-such-language"), false).is_none());
    }
}
