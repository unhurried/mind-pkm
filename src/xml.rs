//! A minimal XML DOM, just enough to read a Freeplane `.mm` file and write it
//! back without losing anything this application does not understand
//! (styles, hooks, attributes, rich content, ...).
//!
//! The writer follows Freeplane's own serialization: no XML declaration,
//! newlines/tabs inside attribute values and every non-ASCII character
//! written as character references, empty elements self-closed.

use quick_xml::Reader;
use quick_xml::events::Event;

#[derive(Debug, Clone, PartialEq)]
pub enum XNode {
    Element(Element),
    Text(String),
    /// An entity reference other than the predefined/character ones, kept verbatim (`&name;`).
    EntityRef(String),
    Comment(String),
    CData(String),
    /// Processing instruction, declaration or doctype: written back verbatim (the content between `<` and `>`).
    Raw(String),
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Element {
    pub name: String,
    pub attrs: Vec<(String, String)>,
    pub children: Vec<XNode>,
}

impl Element {
    pub fn new(name: &str) -> Self {
        Element { name: name.to_string(), attrs: Vec::new(), children: Vec::new() }
    }

    pub fn attr(&self, name: &str) -> Option<&str> {
        get_attr(&self.attrs, name)
    }

    pub fn set_attr(&mut self, name: &str, value: &str) {
        set_attr(&mut self.attrs, name, value);
    }

    pub fn with_attr(mut self, name: &str, value: &str) -> Self {
        self.set_attr(name, value);
        self
    }
}

/// The value of an attribute in an attribute list.
pub fn get_attr<'a>(attrs: &'a [(String, String)], name: &str) -> Option<&'a str> {
    attrs.iter().find(|(k, _)| k == name).map(|(_, v)| v.as_str())
}

/// Sets an attribute in an attribute list: in place when present, else appended.
pub fn set_attr(attrs: &mut Vec<(String, String)>, name: &str, value: &str) {
    match attrs.iter_mut().find(|(k, _)| k == name) {
        Some((_, v)) => *v = value.to_string(),
        None => attrs.push((name.to_string(), value.to_string())),
    }
}

/// A parsed document: everything before, the root element, everything after.
#[derive(Debug, Clone, PartialEq)]
pub struct Document {
    pub prolog: Vec<XNode>,
    pub root: Element,
    pub epilog: Vec<XNode>,
}

pub fn parse(text: &str) -> Result<Document, String> {
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let mut reader = Reader::from_str(text);
    reader.config_mut().trim_text(false);
    reader.config_mut().check_end_names = true;

    let mut tree = TreeBuilder::default();
    loop {
        let event = reader.read_event().map_err(|e| format!("XML error at {}: {e}", reader.buffer_position()))?;
        if !matches!(event, Event::Text(_) | Event::GeneralRef(_)) {
            tree.flush_text();
        }
        match event {
            Event::Start(e) => tree.stack.push(start_element(&e)?),
            Event::Empty(e) => {
                let el = start_element(&e)?;
                if tree.stack.is_empty() && tree.root.is_none() {
                    tree.root = Some(el);
                } else {
                    tree.push(XNode::Element(el));
                }
            }
            Event::End(_) => {
                let el = tree.stack.pop().ok_or("unbalanced end tag")?;
                if tree.stack.is_empty() {
                    if tree.root.is_some() {
                        return Err("multiple root elements".into());
                    }
                    tree.root = Some(el);
                } else {
                    tree.push(XNode::Element(el));
                }
            }
            Event::Text(t) => tree.push_raw_text(&t),
            Event::GeneralRef(r) => tree.push_raw_text(&format!("&{};", &*r)),
            Event::CData(t) => tree.push(XNode::CData(t.into_inner().into_owned())),
            Event::Comment(t) => tree.push(XNode::Comment(t.into_inner().into_owned())),
            Event::Decl(d) => tree.push(XNode::Raw(format!("?{}?", &*d))),
            Event::PI(p) => tree.push(XNode::Raw(format!("?{}?", &*p))),
            Event::DocType(d) => tree.push(XNode::Raw(format!("!DOCTYPE {}", &*d))),
            Event::Eof => break,
        }
    }
    if !tree.stack.is_empty() {
        return Err("unexpected end of file".into());
    }
    let root = tree.root.ok_or("no root element")?;
    Ok(Document { prolog: tree.prolog, root, epilog: tree.epilog })
}

/// The document as it is being parsed: the elements still open, and where a
/// finished node goes (the innermost open element, else the prolog before the
/// root element or the epilog after it).
#[derive(Default)]
struct TreeBuilder {
    prolog: Vec<XNode>,
    root: Option<Element>,
    epilog: Vec<XNode>,
    stack: Vec<Element>,
    /// Raw text with its references, decoded once complete (a surrogate pair
    /// arrives as two references).
    raw_text: String,
}

impl TreeBuilder {
    fn push(&mut self, node: XNode) {
        if let Some(top) = self.stack.last_mut() {
            // Adjacent text pieces (split around entity references) are merged.
            if let (XNode::Text(new), Some(XNode::Text(prev))) = (&node, top.children.last_mut()) {
                prev.push_str(new);
                return;
            }
            top.children.push(node);
        } else if self.root.is_none() {
            self.prolog.push(node);
        } else {
            self.epilog.push(node);
        }
    }

    fn push_raw_text(&mut self, raw: &str) {
        // Outside the root element only whitespace may appear; drop it.
        if !self.stack.is_empty() {
            self.raw_text.push_str(raw);
        }
    }

    fn flush_text(&mut self) {
        if !self.raw_text.is_empty() {
            for node in unescape_nodes(&std::mem::take(&mut self.raw_text)) {
                self.push(node);
            }
        }
    }
}

fn start_element(e: &quick_xml::events::BytesStart) -> Result<Element, String> {
    let name = e.name().as_ref().to_string();
    let mut attrs = Vec::new();
    for a in e.attributes().with_checks(false) {
        let a = a.map_err(|e| e.to_string())?;
        attrs.push((a.key.as_ref().to_string(), unescape(&a.value)));
    }
    Ok(Element { name, attrs, children: Vec::new() })
}

enum Ref {
    Text(&'static str),
    Code(u32),
}

fn parse_ref(name: &str) -> Option<Ref> {
    Some(match name {
        "lt" => Ref::Text("<"),
        "gt" => Ref::Text(">"),
        "amp" => Ref::Text("&"),
        "quot" => Ref::Text("\""),
        "apos" => Ref::Text("'"),
        _ => {
            if let Some(hex) = name.strip_prefix("#x").or_else(|| name.strip_prefix("#X")) {
                Ref::Code(u32::from_str_radix(hex, 16).ok()?)
            } else {
                Ref::Code(name.strip_prefix('#')?.parse().ok()?)
            }
        }
    })
}

/// Decodes raw XML text: predefined and character references are resolved
/// (a UTF-16 surrogate pair written as two references, as Freeplane does for
/// characters outside the BMP, becomes one character); any other entity
/// reference is kept as an `EntityRef` node.
fn unescape_nodes(raw: &str) -> Vec<XNode> {
    let mut nodes = Vec::new();
    let mut text = String::new();
    let mut rest = raw;
    let mut high: Option<u32> = None;
    while let Some(pos) = rest.find('&') {
        let before = &rest[..pos];
        let after = &rest[pos + 1..];
        if !before.is_empty() {
            high = None;
            text.push_str(before);
        }
        let Some(end) = after.find(';').filter(|&e| e <= 12) else {
            text.push('&');
            rest = after;
            continue;
        };
        let name = &after[..end];
        rest = &after[end + 1..];
        match parse_ref(name) {
            Some(Ref::Text(t)) => {
                high = None;
                text.push_str(t)
            }
            Some(Ref::Code(c)) if (0xd800..0xdc00).contains(&c) => high = Some(c),
            Some(Ref::Code(c)) if (0xdc00..0xe000).contains(&c) => {
                if let Some(h) = high.take()
                    && let Some(ch) = char::from_u32(0x10000 + ((h - 0xd800) << 10) + (c - 0xdc00))
                {
                    text.push(ch);
                }
            }
            Some(Ref::Code(c)) => {
                high = None;
                text.extend(char::from_u32(c));
            }
            None => {
                high = None;
                if !text.is_empty() {
                    nodes.push(XNode::Text(std::mem::take(&mut text)));
                }
                nodes.push(XNode::EntityRef(name.to_string()));
            }
        }
    }
    text.push_str(rest);
    if !text.is_empty() {
        nodes.push(XNode::Text(text));
    }
    nodes
}

/// Resolves the predefined and character references; anything else stays as written.
pub fn unescape(raw: &str) -> String {
    if !raw.contains('&') {
        return raw.to_string();
    }
    let mut out = String::new();
    for n in unescape_nodes(raw) {
        match n {
            XNode::Text(t) => out.push_str(&t),
            XNode::EntityRef(name) => {
                out.push('&');
                out.push_str(&name);
                out.push(';');
            }
            _ => {}
        }
    }
    out
}

fn escape_into(out: &mut String, s: &str, attr: bool) {
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' if attr => out.push_str("&quot;"),
            '\n' | '\r' | '\t' if attr => out.push_str(&format!("&#x{:x};", c as u32)),
            // Freeplane's writer: every non-ASCII UTF-16 unit as a reference.
            c if (c as u32) > 0x7e => {
                let mut units = [0u16; 2];
                for u in c.encode_utf16(&mut units) {
                    out.push_str(&format!("&#x{:x};", u));
                }
            }
            c => out.push(c),
        }
    }
}

/// Escapes a string the way Freeplane writes attribute values.
#[cfg(test)]
fn escape_attr(s: &str) -> String {
    let mut out = String::new();
    escape_into(&mut out, s, true);
    out
}

/// Which elements get one child per line (whitespace between their children is
/// not preserved but regenerated); the content of every other element is
/// written exactly as read.
pub type LineBreakPolicy = fn(&Element) -> bool;

pub fn write_document(doc: &Document, line_break: LineBreakPolicy) -> String {
    let mut out = String::new();
    for n in &doc.prolog {
        write_node(&mut out, n, line_break);
        out.push('\n');
    }
    write_element(&mut out, &doc.root, line_break);
    out.push('\n');
    for n in &doc.epilog {
        write_node(&mut out, n, line_break);
        out.push('\n');
    }
    out
}

pub fn write_node(out: &mut String, node: &XNode, line_break: LineBreakPolicy) {
    match node {
        XNode::Element(e) => write_element(out, e, line_break),
        XNode::Text(t) => escape_into(out, t, false),
        XNode::EntityRef(name) => {
            out.push('&');
            out.push_str(name);
            out.push(';');
        }
        XNode::Comment(c) => {
            out.push_str("<!--");
            out.push_str(c);
            out.push_str("-->");
        }
        XNode::CData(c) => {
            out.push_str("<![CDATA[");
            out.push_str(c);
            out.push_str("]]>");
        }
        XNode::Raw(r) => {
            out.push('<');
            out.push_str(r);
            out.push('>');
        }
    }
}

pub fn write_element(out: &mut String, e: &Element, line_break: LineBreakPolicy) {
    out.push('<');
    out.push_str(&e.name);
    for (k, v) in &e.attrs {
        out.push(' ');
        out.push_str(k);
        out.push_str("=\"");
        escape_into(out, v, true);
        out.push('"');
    }
    let breaking = line_break(e);
    let children: Vec<&XNode> =
        if breaking { e.children.iter().filter(|c| !is_blank(c)).collect() } else { e.children.iter().collect() };
    if children.is_empty() {
        out.push_str("/>");
        return;
    }
    out.push('>');
    if breaking {
        out.push('\n');
    }
    for c in children {
        write_node(out, c, line_break);
        if breaking {
            out.push('\n');
        }
    }
    out.push_str("</");
    out.push_str(&e.name);
    out.push('>');
}

pub fn is_blank(n: &XNode) -> bool {
    matches!(n, XNode::Text(t) if t.trim().is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn never(_: &Element) -> bool {
        false
    }

    #[test]
    fn unescapes_character_and_predefined_references() {
        assert_eq!(unescape("a&amp;b&#x65e5;&#26412;&lt;&unknown;"), "a&b日本<&unknown;");
    }

    #[test]
    fn escapes_like_freeplane() {
        assert_eq!(escape_attr("日本 \"x\"\n<&>\t"), "&#x65e5;&#x672c; &quot;x&quot;&#xa;&lt;&amp;&gt;&#x9;");
        assert_eq!(escape_attr("😀"), "&#xd83d;&#xde00;");
    }

    #[test]
    fn surrogate_pairs_round_trip() {
        assert_eq!(unescape("a&#xd83d;&#xde00;b"), "a😀b");
        let doc = parse("<a t=\"&#xd83d;&#xde00;\">x&#xd83d;&#xde00;&foo;y</a>").unwrap();
        assert_eq!(doc.root.attr("t"), Some("😀"));
        assert_eq!(
            doc.root.children,
            vec![XNode::Text("x😀".into()), XNode::EntityRef("foo".into()), XNode::Text("y".into())]
        );
    }

    #[test]
    fn round_trips_content_verbatim() {
        let src = "<map version=\"1\"><!--c--><node TEXT=\"&#x65e5;\"><richcontent TYPE=\"NODE\">\n<html>\n  <body>\n    <p>a&amp;b &nbsp;x</p>\n  </body>\n</html>\n</richcontent></node></map>";
        let doc = parse(src).unwrap();
        let out = write_document(&doc, never);
        assert_eq!(out.trim_end(), src);
    }
}
