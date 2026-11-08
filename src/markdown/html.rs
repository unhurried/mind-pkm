//! Markdown as HTML: for page export and for copying rich text. The page's
//! events go through pulldown-cmark's HTML writer after the additions the
//! editor shows (highlight, emoji, intraword sub/superscript, bare URLs,
//! line breaks, the table of contents) are turned into HTML too.

use super::parse::{BlockKind, Document, Events, matching_end};
use base64::Engine;
use pulldown_cmark::{CodeBlockKind, CowStr, Event, LinkType, Tag, TagEnd};
use std::ops::Range;
use std::sync::LazyLock;

/// The style sheet of exported pages.
pub const DEFAULT_CSS: &str = include_str!("../../assets/export.css");

// Sanitize the complete fragment, including raw HTML, Markdown destinations and
// generated markup. SVG is deliberately not an allowed namespace: renderers'
// output is embedded as an image, where scripts and external resources are inert.
static SANITIZER: LazyLock<ammonia::Builder<'static>> = LazyLock::new(|| {
    let mut builder = ammonia::Builder::default();
    builder
        .add_tags(&["input"])
        .add_tag_attributes("input", &["type", "disabled", "checked"])
        .add_generic_attributes(&["class", "id", "style"])
        .filter_style_properties(["color", "text-align"].into())
        .url_schemes(["http", "https", "mailto", "file", "data"].into())
        .attribute_filter(|tag, attr, value| {
            if !matches!(attr, "href" | "src" | "cite") {
                return Some(value.into());
            }
            // `data:` is only useful as an image source, never as a link to an
            // active document. Ammonia has already decoded HTML entities.
            let normalized: String =
                value.chars().filter(|c| !c.is_ascii_whitespace() && !c.is_ascii_control()).collect();
            let normalized = normalized.to_ascii_lowercase();
            if normalized.starts_with("data:")
                && !(tag == "img" && attr == "src" && normalized.starts_with("data:image/"))
            {
                return None;
            }
            Some(value.into())
        })
        .set_tag_attribute_values([("input", [("type", "checkbox"), ("disabled", "")].into())].into())
        .link_rel(None);
    builder
});

/// Export resources: HTML may show remote images; PDF printing stays local.
#[derive(Clone, Copy)]
pub(super) enum Resources {
    Html,
    Pdf,
}

fn content_security_policy(resources: Resources) -> String {
    let images = match resources {
        Resources::Html => "file: data: http: https:",
        Resources::Pdf => "file: data:",
    };
    format!(
        "default-src 'none'; script-src 'none'; style-src 'unsafe-inline'; img-src {images}; font-src file: data:; connect-src 'none'; object-src 'none'; frame-src 'none'; base-uri 'none'; form-action 'none'"
    )
}

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

/// The sanitized, passive HTML of a page's blocks (also safe without a CSP,
/// as when copied to the clipboard).
pub fn body(doc: &Document, opts: &HtmlOptions) -> String {
    let mut events: Vec<Event<'static>> = Vec::new();
    let mut toc_done = false;
    let mut i = 0;
    let all = &doc.events;
    while i < all.len() {
        let (event, range) = &all[i];
        match event {
            Event::Start(Tag::MetadataBlock(_)) => {
                i = matching_end(all, i) + 1;
                continue;
            }
            Event::Start(Tag::Paragraph) if is_toc(doc, range) => {
                events.push(Event::Html(toc_html(doc).into()));
                toc_done = true;
                i = matching_end(all, i) + 1;
                continue;
            }
            Event::Start(Tag::CodeBlock(CodeBlockKind::Fenced(lang)))
                if lang.trim().eq_ignore_ascii_case("mermaid") =>
            {
                let end = matching_end(all, i);
                let code: String = all[i + 1..end]
                    .iter()
                    .filter_map(|(e, _)| match e {
                        Event::Text(t) => Some(t.as_ref()),
                        _ => None,
                    })
                    .collect();
                match (opts.mermaid)(&code) {
                    Some(svg) => events
                        .push(Event::Html(format!("<div class=\"mermaid\">{}</div>\n", svg_image(&svg, &code)).into())),
                    None => events.extend(all[i..=end].iter().map(|(e, _)| e.clone())),
                }
                i = end + 1;
                continue;
            }
            Event::Start(Tag::Paragraph | Tag::Heading { .. } | Tag::TableCell | Tag::Item) => {
                // Inline content: highlight is paired within it.
                let end = matching_end(all, i);
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
    SANITIZER.clean(&out).to_string()
}

/// A whole HTML document.
pub fn page(title: &str, body: &str, css: &str) -> String {
    page_with_resources(title, body, css, Resources::Html)
}

pub(super) fn page_with_resources(title: &str, body: &str, css: &str, resources: Resources) -> String {
    let csp = content_security_policy(resources);
    format!(
        "<!DOCTYPE html>\n<html>\n<head>\n<meta charset=\"utf-8\">\n<meta http-equiv=\"Content-Security-Policy\" content=\"{csp}\">\n<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n<title>{}</title>\n<style>\n{css}\n</style>\n</head>\n<body>\n<article class=\"markdown-body\">\n{body}</article>\n</body>\n</html>\n",
        escape(title)
    )
}

/// SVG in an `<img>` is passive even if a renderer emits active SVG markup.
fn svg_image(svg: &str, alt: &str) -> String {
    let encoded = base64::engine::general_purpose::STANDARD.encode(svg);
    format!("<img src=\"data:image/svg+xml;base64,{encoded}\" alt=\"{}\">", escape(alt))
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
fn inline_events(inner: &Events, opts: &HtmlOptions, out: &mut Vec<Event<'static>>) {
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
                Some(svg) => {
                    out.push(Event::InlineHtml(format!("<span class=\"math\">{}</span>", svg_image(&svg, &tex)).into()))
                }
                None => {
                    out.push(Event::InlineHtml(format!("<span class=\"math\">\\({}\\)</span>", escape(&tex)).into()))
                }
            },
            Event::DisplayMath(tex) => match (opts.math)(tex.trim(), true) {
                Some(svg) => out.push(Event::InlineHtml(
                    format!("<div class=\"math\">{}</div>", svg_image(&svg, tex.trim())).into(),
                )),
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

/// The text of an HTML fragment as it reads: tags dropped, the end of a block
/// element or a `<br>` a line break, entities decoded (what a code block
/// receives when the clipboard holds HTML only).
pub fn to_plain_text(html: &str) -> String {
    let mut out = String::new();
    let mut rest = html;
    while let Some(lt) = rest.find('<') {
        push_decoded(&mut out, &rest[..lt]);
        rest = &rest[lt..];
        if let Some(comment) = rest.strip_prefix("<!--") {
            rest = comment.find("-->").map_or("", |e| &comment[e + 3..]);
            continue;
        }
        let Some(gt) = rest.find('>') else {
            // Not a tag after all.
            push_decoded(&mut out, rest);
            rest = "";
            break;
        };
        let tag = rest[1..gt].trim_start_matches('/').split(|c: char| c.is_whitespace() || c == '/').next();
        let name = tag.unwrap_or("").to_ascii_lowercase();
        let closing = rest[1..].starts_with('/');
        if name == "br" || (closing && is_block(&name)) {
            out.push('\n');
        }
        rest = &rest[gt + 1..];
        if matches!(name.as_str(), "style" | "script") && !closing {
            let end = format!("</{name}");
            rest = rest.to_ascii_lowercase().find(&end).map_or("", |e| &rest[e..]);
        }
    }
    push_decoded(&mut out, rest);
    out.trim_end_matches('\n').to_string()
}

fn is_block(name: &str) -> bool {
    matches!(
        name,
        "div"
            | "p"
            | "li"
            | "tr"
            | "pre"
            | "ul"
            | "ol"
            | "table"
            | "blockquote"
            | "h1"
            | "h2"
            | "h3"
            | "h4"
            | "h5"
            | "h6"
    )
}

/// Appends `text` with its character references decoded.
fn push_decoded(out: &mut String, text: &str) {
    let mut rest = text;
    while let Some(amp) = rest.find('&') {
        out.push_str(&rest[..amp]);
        rest = &rest[amp..];
        let decoded = rest.find(';').filter(|&semi| semi <= 10).and_then(|semi| {
            let name = &rest[1..semi];
            let c = match name {
                "lt" => '<',
                "gt" => '>',
                "amp" => '&',
                "quot" => '"',
                "apos" => '\'',
                "nbsp" => '\u{a0}',
                _ => {
                    let n = name.strip_prefix('#')?;
                    let code = match n.strip_prefix(['x', 'X']) {
                        Some(hex) => u32::from_str_radix(hex, 16).ok()?,
                        None => n.parse().ok()?,
                    };
                    char::from_u32(code)?
                }
            };
            Some((c, semi + 1))
        });
        match decoded {
            Some((c, len)) => {
                out.push(c);
                rest = &rest[len..];
            }
            None => {
                out.push('&');
                rest = &rest[1..];
            }
        }
    }
    out.push_str(rest);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn html_as_plain_text() {
        assert_eq!(
            to_plain_text("<div><div><span>a_b = c*d;</span></div><div><span>x &lt; y</span></div></div>"),
            "a_b = c*d;\nx < y"
        );
        assert_eq!(to_plain_text("a<br>b<!-- c -->&#x41;&#66;&amp;lt; &bogus; & x"), "a\nbAB&lt; &bogus; & x");
        assert_eq!(to_plain_text("<style>p { x: y }</style><p>one</p><p>two</p>"), "one\ntwo");
        assert_eq!(to_plain_text("1 < 2"), "1 < 2");
    }

    fn html(md: &str) -> String {
        body(&Document::parse(md), &HtmlOptions::plain())
    }

    #[test]
    fn the_editor_s_additions() {
        assert_eq!(html("a ==b== c"), "<p>a <mark>b</mark> c</p>\n");
        assert_eq!(html("H~2~O :smile:"), "<p>H<sub>2</sub>O 😄</p>\n");
        assert_eq!(html("see https://x.org/a."), "<p>see <a href=\"https://x.org/a\">https://x.org/a</a>.</p>\n");
        assert_eq!(html("one\ntwo"), "<p>one<br>\ntwo</p>\n");
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
        assert!(out.contains("<th style=\"text-align:center\">a</th>"), "{out}");
        assert!(out.contains("checked=\"\""), "{out}");
    }

    #[test]
    fn math_diagrams_and_images_by_the_options() {
        let math = |tex: &str, display: bool| Some(format!("<svg>{tex}{display}</svg>"));
        let mermaid = |code: &str| Some(format!("<svg>{}</svg>", code.trim()));
        let src = |s: &str| format!("../{s}");
        let opts = HtmlOptions { math: &math, mermaid: &mermaid, image_src: &src };
        let out = body(&Document::parse("$x$\n\n$$\ny\n$$\n\n```mermaid\nA-->B\n```\n\n![i](a.png)\n"), &opts);
        assert!(out.contains(&format!("<span class=\"math\">{}</span>", svg_image("<svg>xfalse</svg>", "x"))), "{out}");
        assert!(out.contains(&format!("<div class=\"math\">{}</div>", svg_image("<svg>ytrue</svg>", "y"))), "{out}");
        assert!(
            out.contains(&format!("<div class=\"mermaid\">{}</div>", svg_image("<svg>A-->B</svg>", "A-->B\n"))),
            "{out}"
        );
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
        assert!(p.contains("script-src 'none'"));
        assert!(p.find("Content-Security-Policy").unwrap() < p.find("<style>").unwrap());
    }

    #[test]
    fn untrusted_html_is_passive_in_blocks_inline_content_and_containers() {
        for md in [
            "<script>SEC02_EXECUTED</script><p>safe</p>",
            "safe <img src=x onerror=SEC02_EXECUTED> <b onclick=SEC02_EXECUTED>bold</b>",
            "> safe <svg onload=SEC02_EXECUTED><script>SEC02_EXECUTED</script></svg>",
            "- safe <a href='javascript:SEC02_EXECUTED'>link</a>",
            "| cell |\n|---|\n| safe <img src=x onerror=SEC02_EXECUTED> |",
            "<iframe srcdoc='<script>SEC02_EXECUTED</script>'></iframe><p>safe</p>",
        ] {
            let out = fragment(md);
            assert!(out.contains("safe"), "{md}: {out}");
            for active in ["SEC02_EXECUTED", "<script", "onerror", "onclick", "onload", "<iframe", "<svg", "<math"] {
                assert!(!out.contains(active), "{md}: {out}");
            }
        }
    }

    #[test]
    fn malformed_foreign_content_cannot_escape_sanitization() {
        // The trailing text is inside malformed foreign/raw-text content, not
        // ordinary visible text. Reparse the result as a browser would do.
        let out = fragment(
            "<math><mtext><table><mglyph><style><!--</style><img title='--><img src=x onerror=SEC02_EXECUTED>'>",
        );
        assert!(!out.contains("SEC02_EXECUTED"), "{out}");
        assert!(!out.contains("<math"), "{out}");
        assert_eq!(SANITIZER.clean(&out).to_string(), out);
    }

    #[test]
    fn unsafe_destinations_and_document_controls_are_removed() {
        for md in [
            "[safe](javascript:alert%281%29)",
            "[safe](JaVaScRiPt:alert%281%29)",
            "[safe](vbscript:msgbox%281%29)",
            "[safe](data:text/html;base64,PHNjcmlwdD4=)",
            "<a href='jav&#x61;script:alert(1)'>safe</a>",
            "<a href='java&#9;script:alert(1)'>safe</a>",
            "<a href='d&#97;ta:text/html,test'>safe</a>",
        ] {
            let out = fragment(md);
            assert_eq!(out.trim(), "<p><a>safe</a></p>", "{md}: {out}");
        }
        let out = fragment(
            "<base href='https://example.com'><meta http-equiv=refresh content='0;url=https://example.com'>\n\
             <form action='https://example.com'><button formaction='https://example.com'>safe</button></form>\n\
             <object data='https://example.com'><embed src='https://example.com'></object>\n\
             <link rel=stylesheet href='https://example.com/a.css'>\n\
             <style>@import 'https://example.com/a.css';</style>",
        );
        for tag in ["base", "meta", "form", "button", "object", "embed", "link", "style"] {
            assert!(!out.contains(&format!("<{tag}")), "{out}");
        }
        assert!(!out.contains("https://example.com"), "{out}");
        let out = fragment("![safe](data:text/html;base64,PHNjcmlwdD4=)");
        assert!(!out.contains("src="), "{out}");
    }

    #[test]
    fn passive_html_links_and_embedded_images_still_work() {
        let out = fragment(
            "<u>under</u><br><span style='color: red; background: url(https://example.com/track); position: fixed' onclick='alert(1)'>red</span>\n\n\
             [web](https://example.com) [mail](mailto:a@example.com) [file](file:///d/a.pdf) [relative](../a.md) [anchor](#a)\n\n\
             ![data:text/html](data:image/png;base64,aGVsbG8=)",
        );
        assert!(out.contains("<u>under</u><br><span style=\"color:red\">red</span>"), "{out}");
        for href in ["https://example.com", "mailto:a@example.com", "file:///d/a.pdf", "../a.md", "#a"] {
            assert!(out.contains(&format!("href=\"{href}\"")), "{out}");
        }
        assert!(out.contains("src=\"data:image/png;base64,aGVsbG8=\""), "{out}");
        assert!(out.contains("alt=\"data:text/html\""), "{out}");
        assert!(!out.contains("background"), "{out}");
        assert!(!out.contains("onclick"), "{out}");
    }

    #[test]
    fn renderer_output_cannot_insert_active_svg_into_the_page() {
        let svg = "<svg xmlns='http://www.w3.org/2000/svg' onload='alert(1)'><script>alert(1)</script><foreignObject><iframe src='https://example.com'></iframe></foreignObject></svg>";
        let opts =
            HtmlOptions { math: &|_, _| Some(svg.into()), mermaid: &|_| Some(svg.into()), image_src: &str::to_string };
        let out = body(&Document::parse("$x$\n\n```mermaid\nA-->B\n```"), &opts);
        assert_eq!(out.matches("src=\"data:image/svg+xml;base64,").count(), 2, "{out}");
        assert!(!out.contains("<svg"), "{out}");
        assert!(!out.contains("<script"), "{out}");
        for img in out.split("data:image/svg+xml;base64,").skip(1) {
            let encoded = img.split('"').next().unwrap();
            assert_eq!(base64::engine::general_purpose::STANDARD.decode(encoded).unwrap(), svg.as_bytes());
        }
    }
}
