//! The math and diagrams of open pages, rendered in the background and
//! cached by their source.

use crate::app::{App, Message, blocking};
use crate::markdown::content::{self, Inline, Node};
use crate::markdown::{BlockKind, Document, render};
use iced::Task;
use iced::widget::svg;
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum RenderKey {
    Math { tex: String, display: bool, dark: bool },
    Mermaid(String),
}

/// A rendered SVG, its size in pixels at 100%.
#[derive(Debug, Clone)]
pub struct Svg {
    pub handle: svg::Handle,
    pub width: f32,
    pub height: f32,
}

#[derive(Debug, Clone)]
pub enum Rendered {
    Pending,
    Done(Result<Svg, String>),
}

#[derive(Debug, Default)]
pub struct RenderCache {
    entries: HashMap<RenderKey, Rendered>,
}

impl RenderCache {
    pub fn get(&self, key: &RenderKey) -> Option<&Rendered> {
        self.entries.get(key)
    }

    /// The keys not rendered or being rendered yet, marked as being rendered now.
    fn take_missing(&mut self, keys: HashSet<RenderKey>) -> Vec<RenderKey> {
        let missing: Vec<RenderKey> = keys.into_iter().filter(|k| !self.entries.contains_key(k)).collect();
        for k in &missing {
            self.entries.insert(k.clone(), Rendered::Pending);
        }
        missing
    }

    pub fn insert(&mut self, key: RenderKey, svg: Result<String, String>) {
        let done = svg.and_then(|svg| {
            let (width, height) = render::svg_size(&svg).ok_or("the renderer gave no size")?;
            Ok(Svg { handle: svg::Handle::from_memory(svg.into_bytes()), width, height })
        });
        self.entries.insert(key, Rendered::Done(done));
    }
}

/// The math and diagrams of a page.
pub fn keys_of(doc: &Document, dark: bool) -> HashSet<RenderKey> {
    let mut keys = HashSet::new();
    for (i, block) in doc.blocks.iter().enumerate() {
        match &block.kind {
            k if k.is_mermaid() => {
                let source = content::nodes(doc, doc.block_events(i)).into_iter().find_map(|n| match n {
                    Node::Code { text, .. } => Some(text),
                    _ => None,
                });
                keys.insert(RenderKey::Mermaid(source.unwrap_or_else(|| code_of(doc, i))));
            }
            BlockKind::Code { .. } | BlockKind::FrontMatter | BlockKind::Other => {}
            _ => collect_math(&content::nodes(doc, doc.block_events(i)), dark, &mut keys),
        }
    }
    keys
}

/// The code of a code block.
pub fn code_of(doc: &Document, block: usize) -> String {
    doc.block_events(block)
        .iter()
        .filter_map(|(e, _)| match e {
            pulldown_cmark::Event::Text(t) => Some(t.to_string()),
            _ => None,
        })
        .collect()
}

fn collect_math(nodes: &[Node], dark: bool, keys: &mut HashSet<RenderKey>) {
    let inline = |inlines: &[Inline], keys: &mut HashSet<RenderKey>| {
        for i in inlines {
            if let Inline::Math { tex, display, .. } = i {
                keys.insert(RenderKey::Math { tex: tex.clone(), display: *display, dark });
            }
        }
    };
    for n in nodes {
        match n {
            Node::Paragraph(i) | Node::Heading(_, i) | Node::Html(i) => inline(i, keys),
            Node::Math(tex) => {
                keys.insert(RenderKey::Math { tex: tex.trim().to_string(), display: true, dark });
            }
            Node::Quote(c) | Node::Footnote { children: c, .. } => collect_math(c, dark, keys),
            Node::List { items, .. } => items.iter().for_each(|it| collect_math(&it.children, dark, keys)),
            Node::Table(t) => {
                for cell in t.header.iter().chain(t.rows.iter().flatten()) {
                    inline(cell, keys);
                }
            }
            Node::Code { .. } | Node::Rule | Node::FrontMatter(_) => {}
        }
    }
}

/// Renders a key (on the blocking pool).
pub fn render_key(key: &RenderKey) -> Result<String, String> {
    match key {
        RenderKey::Math { tex, display, dark } => {
            let rgb = if *dark { (0xe3, 0xe6, 0xea) } else { (0x1b, 0x1f, 0x24) };
            render::math_svg(tex, *display, rgb)
        }
        RenderKey::Mermaid(source) => render::mermaid_svg(source),
    }
}

impl App {
    /// Starts rendering what the open pages show and is not rendered yet.
    pub(in crate::app) fn request_renders(&mut self) -> Task<Message> {
        let dark = self.editor_prefs.dark;
        let mut keys = HashSet::new();
        for editor in self.editors.values() {
            keys.extend(keys_of(&editor.page.doc, dark));
        }
        let missing = self.renders.take_missing(keys);
        Task::batch(missing.into_iter().map(|key| {
            let k = key.clone();
            Task::perform(blocking(move || render_key(&k)), move |r| Message::Rendered(key.clone(), r))
        }))
    }
}
