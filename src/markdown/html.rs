//! Markdown as HTML: for page export and for copying rich text. The page's
//! events go through pulldown-cmark's HTML writer after the additions the
//! editor shows (highlight, emoji, intraword sub/superscript, bare URLs,
//! line breaks, the table of contents) are turned into HTML too.

use super::parse::{BlockKind, Document};
use pulldown_cmark::{CodeBlockKind, CowStr, Event, LinkType, Tag, TagEnd};
use std::ops::Range;

/// The style sheet of exported pages.
pub const DEFAULT_CSS: &str = include_str!("../../assets/export.css");

/// How math, diagrams and images are written.
pub struct HtmlOptions<'a> {
    /// Math as SVG (`tex`, `display`); None writes the LaTeX.
    pub math: &'a dyn Fn(&str, bool) -> Option<String>,
    /// A Mermaid diagram as SVG; None writes the code.
    pub mermaid: &'a dyn Fn(&str) -> Option<String>,
    /// The `src` to write for an image's `src`.
    pub image_src: &'a dyn Fn(&str) -> String,
}

impl HtmlOptions<'_> {
    /// LaTeX and code as they are, image sources unchanged (for the clipboard).
    pub fn plain() -> HtmlOptions<'static> {
        HtmlOptions { math: &|_, _| None, mermaid: &|_| None, image_src: &|s| s.to_string() }
    }
}

fn escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            _ => out.push(c),
        }
    }
    out
}

/// The HTML of a page's blocks.
pub fn body(doc: &Document, opts: &HtmlOptions) -> String {
    let mut events: Vec<Event<'static>> = Vec::new();
    let mut toc_done = false;
    let mut i = 0;
    let all = &doc.events;
    while i < all.len() {
        let (event, range) = &all[i];
        match event {
            Event::Start(Tag::MetadataBlock(_)) => {
                i = end_of(all, i) + 1;
                continue;
            }
            Event::Start(Tag::Paragraph) if is_toc(doc, range) => {
                events.push(Event::Html(toc_html(doc).into()));
                toc_done = true;
                i = end_of(all, i) + 1;
                continue;
            }
            Event::Start(Tag::CodeBlock(CodeBlockKind::Fenced(lang)))
                if lang.trim().eq_ignore_ascii_case("mermaid") =>
            {
                let end = end_of(all, i);
                let code: String = all[i + 1..end]
                    .iter()
                    .filter_map(|(e, _)| match e {
                        Event::Text(t) => Some(t.as_ref()),
                        _ => None,
                    })
                    .collect();
                match (opts.mermaid)(&code) {
                    Some(svg) => events.push(Event::Html(format!("<div class=\"mermaid\">{svg}</div>\n").into())),
                    None => events.extend(all[i..=end].iter().map(|(e, _)| e.clone())),
                }
                i = end + 1;
                continue;
            }
            Event::Start(Tag::Paragraph | Tag::Heading { .. } | Tag::TableCell | Tag::Item) => {
                // Inline content: highlight is paired within it.
                let end = end_of(all, i);
                events.push(event.clone());
                inline_events(&all[i + 1..end], opts, &mut events);
                events.push(all[end].0.clone());
                i = end + 1;
                continue;
            }
            _ => {}
        }
        events.push(map_event(event.clone(), opts));
        i += 1;
    }
    let _ = toc_done;
    let mut out = String::new();
    pulldown_cmark::html::push_html(&mut out, events.into_iter());
    out
}

/// A whole HTML document.
pub fn page(title: &str, body: &str, css: &str) -> String {
    format!(
        "<!DOCTYPE html>\n<html>\n<head>\n<meta charset=\"utf-8\">\n<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n<title>{}</title>\n<style>\n{css}\n</style>\n</head>\n<body>\n<article class=\"markdown-body\">\n{body}</article>\n</body>\n</html>\n",
        escape(title)
    )
}

/// Markdown source (a selection) as HTML, for the clipboard.
pub fn fragment(markdown: &str) -> String {
    body(&Document::parse(markdown), &HtmlOptions::plain())
}

fn is_toc(doc: &Document, range: &Range<usize>) -> bool {
    doc.blocks.iter().any(|b| b.kind == BlockKind::Toc && b.range.start <= range.start && range.start <= b.range.end)
}

fn toc_html(doc: &Document) -> String {
    let mut out = String::from("<nav class=\"toc\"><ul>\n");
    for h in &doc.headings {
        out.push_str(&format!(
            "<li class=\"toc-h{}\"><a href=\"#{}\">{}</a></li>\n",
            h.level,
            escape(&h.anchor),
            escape(&h.text)
        ));
    }
    out.push_str("</ul></nav>\n");
    out
}

fn end_of(events: &[(Event<'static>, Range<usize>)], start: usize) -> usize {
    let mut depth = 0;
    for (i, (e, _)) in events.iter().enumerate().skip(start) {
        match e {
            Event::Start(_) => depth += 1,
            Event::End(_) => {
                depth -= 1;
                if depth == 0 {
                    return i;
                }
            }
            _ => {}
        }
    }
    events.len() - 1
}

/// An event outside inline content.
fn map_event(event: Event<'static>, opts: &HtmlOptions) -> Event<'static> {
    match event {
        Event::Start(Tag::Image { link_type, dest_url, title, id }) => {
            let src = (opts.image_src)(&dest_url);
            Event::Start(Tag::Image { link_type, dest_url: src.into(), title, id })
        }
        e => e,
    }
}

/// The inline events of a paragraph, heading, list item or cell, with the editor's additions.
fn inline_events(inner: &[(Event<'static>, Range<usize>)], opts: &HtmlOptions, out: &mut Vec<Event<'static>>) {
    // `==` pairs (an odd last one stays text).
    let total: usize = inner
        .iter()
        .map(|(e, _)| match e {
            Event::Text(t) => t.matches("==").count(),
            _ => 0,
        })
        .sum();
    let mut marks_left = total - total % 2;
    let mut in_mark = false;
    let mut in_link = 0;
    let mut i = 0;
    while i < inner.len() {
        let event = inner[i].0.clone();
        match event {
            Event::Start(Tag::Link { .. }) => {
                in_link += 1;
                out.push(event);
            }
            Event::End(TagEnd::Link) => {
                in_link -= 1;
                out.push(event);
            }
            Event::Text(t) => {
                // Adjacent text events as one.
                let mut text = t.to_string();
                while let Some((Event::Text(next), _)) = inner.get(i + 1) {
                    text.push_str(next);
                    i += 1;
                }
                let mut pos = 0;
                while marks_left > 0
                    && let Some(p) = text[pos..].find("==").map(|p| p + pos)
                {
                    text_events(&text[pos..p], in_link > 0, out);
                    out.push(Event::InlineHtml(if in_mark { "</mark>" } else { "<mark>" }.into()));
                    in_mark = !in_mark;
                    marks_left -= 1;
                    pos = p + 2;
                }
                text_events(&text[pos..], in_link > 0, out);
            }
            Event::SoftBreak => out.push(Event::HardBreak),
            Event::InlineMath(tex) => match (opts.math)(&tex, false) {
                Some(svg) => out.push(Event::InlineHtml(format!("<span class=\"math\">{svg}</span>").into())),
                None => {
                    out.push(Event::InlineHtml(format!("<span class=\"math\">\\({}\\)</span>", escape(&tex)).into()))
                }
            },
            Event::DisplayMath(tex) => match (opts.math)(tex.trim(), true) {
                Some(svg) => out.push(Event::InlineHtml(format!("<div class=\"math\">{svg}</div>").into())),
                None => out
                    .push(Event::InlineHtml(format!("<div class=\"math\">\\[{}\\]</div>", escape(tex.trim())).into())),
            },
            e => out.push(map_event(e, opts)),
        }
        i += 1;
    }
    if in_mark {
        out.push(Event::InlineHtml("</mark>".into()));
    }
}

/// Text with emoji shortcodes, intraword sub/superscript and (outside links) bare URLs.
fn text_events(text: &str, in_link: bool, out: &mut Vec<Event<'static>>) {
    if text.is_empty() {
        return;
    }
    // The editor's recognition, on one run of text.
    let map = super::content::source_map(text.len(), 0..text.len());
    let run = super::content::Inline::Text {
        text: text.to_string(),
        style: Default::default(),
        link: in_link.then(|| super::content::Link { dest: String::new(), title: String::new() }),
        map,
    };
    for inline in super::content::recognize(vec![run]) {
        let super::content::Inline::Text { text, style, link, .. } = inline else { continue };
        let tag = if style.sub {
            Some("sub")
        } else if style.sup {
            Some("sup")
        } else {
            None
        };
        if let Some(tag) = tag {
            out.push(Event::InlineHtml(format!("<{tag}>").into()));
        }
        match link {
            Some(l) if !in_link && !l.dest.is_empty() => {
                out.push(Event::Start(Tag::Link {
                    link_type: LinkType::Autolink,
                    dest_url: CowStr::from(l.dest),
                    title: "".into(),
                    id: "".into(),
                }));
                out.push(Event::Text(text.into()));
                out.push(Event::End(TagEnd::Link));
            }
            _ => out.push(Event::Text(text.into())),
        }
        if let Some(tag) = tag {
            out.push(Event::InlineHtml(format!("</{tag}>").into()));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn html(md: &str) -> String {
        body(&Document::parse(md), &HtmlOptions::plain())
    }

    #[test]
    fn the_editor_s_additions() {
        assert_eq!(html("a ==b== c"), "<p>a <mark>b</mark> c</p>\n");
        assert_eq!(html("H~2~O :smile:"), "<p>H<sub>2</sub>O 😄</p>\n");
        assert_eq!(html("see https://x.org/a."), "<p>see <a href=\"https://x.org/a\">https://x.org/a</a>.</p>\n");
        assert_eq!(html("one\ntwo"), "<p>one<br />\ntwo</p>\n");
    }

    #[test]
    fn front_matter_is_left_out_and_toc_listed() {
        let out = html("---\ntitle: x\n---\n\n[toc]\n\n# A\n\n## B\n");
        assert!(!out.contains("title: x"));
        assert!(out.contains("<nav class=\"toc\">"), "{out}");
        assert!(out.contains("<a href=\"#b\">B</a>"), "{out}");
    }

    #[test]
    fn lenient_links_tables_and_tasks() {
        let out = html("[assets](My Page.assets)\n\n| a |\n|:-:|\n| 1 |\n\n- [x] done\n");
        assert!(out.contains("<a href=\"My%20Page.assets\">assets</a>"), "{out}");
        assert!(out.contains("<th style=\"text-align: center\">a</th>"), "{out}");
        assert!(out.contains("checked=\"\""), "{out}");
    }

    #[test]
    fn math_diagrams_and_images_by_the_options() {
        let math = |tex: &str, display: bool| Some(format!("<svg>{tex}{display}</svg>"));
        let mermaid = |code: &str| Some(format!("<svg>{}</svg>", code.trim()));
        let src = |s: &str| format!("../{s}");
        let opts = HtmlOptions { math: &math, mermaid: &mermaid, image_src: &src };
        let out = body(&Document::parse("$x$\n\n$$\ny\n$$\n\n```mermaid\nA-->B\n```\n\n![i](a.png)\n"), &opts);
        assert!(out.contains("<span class=\"math\"><svg>xfalse</svg></span>"), "{out}");
        assert!(out.contains("<div class=\"math\"><svg>ytrue</svg></div>"), "{out}");
        assert!(out.contains("<div class=\"mermaid\"><svg>A-->B</svg></div>"), "{out}");
        assert!(out.contains("src=\"../a.png\""), "{out}");
        // Without renderers, the LaTeX is kept.
        assert!(html("$x$").contains("\\(x\\)"));
    }

    #[test]
    fn a_whole_page() {
        let p = page("A <page>", "<p>x</p>\n", "body {}");
        assert!(p.starts_with("<!DOCTYPE html>"));
        assert!(p.contains("<title>A &lt;page&gt;</title>"));
        assert!(p.contains("<style>\nbody {}\n</style>"));
    }
}
