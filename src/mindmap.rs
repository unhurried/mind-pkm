//! The mind map model: a Freeplane `.mm` file as a tree of nodes.
//!
//! Every node keeps all of its XML attributes and non-node child elements
//! (styles, icons, rich content, attributes, hooks, ...) in their original
//! order, so a map read and written back by this application stays a valid,
//! unchanged Freeplane map apart from the edits actually made.

use crate::paths;
use crate::xml::{self, Element, XNode};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

pub type NodeId = usize;

/// Freeplane's checkmark icon, toggled by Toggle Checkmark.
pub const ICON_CHECKMARK: &str = "button_ok";

const FREEPLANE_VERSION: &str = "freeplane 1.11.1";
const FILE_COMMENT: &str =
    "To view this file, download free mind mapping software Freeplane from https://www.freeplane.org ";

#[derive(Debug, Clone, PartialEq)]
pub enum Item {
    Xml(XNode),
    Child(NodeId),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Node {
    pub attrs: Vec<(String, String)>,
    pub items: Vec<Item>,
    pub parent: Option<NodeId>,
}

impl Node {
    /// A node with an empty text and nothing else.
    fn empty(parent: Option<NodeId>) -> Self {
        Node { attrs: vec![("TEXT".into(), String::new())], items: Vec::new(), parent }
    }

    fn attr(&self, name: &str) -> Option<&str> {
        xml::get_attr(&self.attrs, name)
    }

    fn set_attr(&mut self, name: &str, value: &str) {
        xml::set_attr(&mut self.attrs, name, value);
    }

    fn remove_attr(&mut self, name: &str) {
        self.attrs.retain(|(k, _)| k != name);
    }

    pub fn children(&self) -> impl Iterator<Item = NodeId> + '_ {
        self.items.iter().filter_map(|i| match i {
            Item::Child(c) => Some(*c),
            Item::Xml(_) => None,
        })
    }

    fn elements(&self) -> impl Iterator<Item = &Element> {
        self.items.iter().filter_map(|i| match i {
            Item::Xml(XNode::Element(e)) => Some(e),
            _ => None,
        })
    }
}

/// A detached copy of a subtree (for cut/copy and paste).
#[derive(Debug, Clone, PartialEq)]
pub struct Subtree {
    attrs: Vec<(String, String)>,
    items: Vec<SubtreeItem>,
}

#[derive(Debug, Clone, PartialEq)]
enum SubtreeItem {
    Xml(XNode),
    Child(Subtree),
}

/// Nodes and the ID index are shared between copies of a map (undo keeps a
/// copy per step) and copied only when changed, so a copy costs a pointer
/// per node rather than the whole tree.
#[derive(Debug, Clone)]
pub struct MindMap {
    nodes: Vec<Option<Arc<Node>>>,
    root: NodeId,
    prolog: Vec<XNode>,
    map_attrs: Vec<(String, String)>,
    /// The `<map>` element's content; `Item::Child` marks where the root node goes.
    map_items: Vec<Item>,
    epilog: Vec<XNode>,
    ids: Arc<HashMap<String, NodeId>>,
    path: Option<PathBuf>,
}

fn now_millis() -> String {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis()).unwrap_or(0).to_string()
}

impl MindMap {
    /// A new, unsaved map with the given root text.
    pub fn new(root_text: &str) -> Self {
        let mut map = MindMap {
            nodes: Vec::new(),
            root: 0,
            prolog: Vec::new(),
            map_attrs: vec![("version".into(), FREEPLANE_VERSION.into())],
            map_items: vec![Item::Xml(XNode::Comment(FILE_COMMENT.into()))],
            epilog: Vec::new(),
            ids: Arc::default(),
            path: None,
        };
        let root = map.alloc(Node::empty(None));
        map.root = root;
        map.init_new_node(root);
        map.map_items.push(Item::Child(root));
        map.set_text(root, root_text);
        map
    }

    pub fn load(path: &Path) -> Result<Self, String> {
        let bytes = std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
        let text = String::from_utf8_lossy(&bytes);
        let mut map = Self::parse(&text)?;
        map.path = Some(path.to_path_buf());
        Ok(map)
    }

    pub fn parse(text: &str) -> Result<Self, String> {
        let doc = xml::parse(text)?;
        if doc.root.name != "map" {
            return Err("not a mind map file (no <map> element)".into());
        }
        let mut map = MindMap {
            nodes: Vec::new(),
            root: 0,
            prolog: doc.prolog,
            map_attrs: doc.root.attrs,
            map_items: Vec::new(),
            epilog: doc.epilog,
            ids: Arc::default(),
            path: None,
        };
        let mut root = None;
        for child in doc.root.children {
            match child {
                XNode::Element(e) if e.name == "node" && root.is_none() => {
                    let id = map.import_element(e, None);
                    root = Some(id);
                    map.map_items.push(Item::Child(id));
                }
                c if xml::is_blank(&c) => {}
                c => map.map_items.push(Item::Xml(c)),
            }
        }
        map.root = root.ok_or("the map has no root node")?;
        Ok(map)
    }

    fn alloc(&mut self, node: Node) -> NodeId {
        self.nodes.push(Some(Arc::new(node)));
        self.nodes.len() - 1
    }

    fn import_element(&mut self, e: Element, parent: Option<NodeId>) -> NodeId {
        let id = self.alloc(Node { attrs: e.attrs, items: Vec::new(), parent });
        let mut items = Vec::new();
        for child in e.children {
            match child {
                XNode::Element(ce) if ce.name == "node" => items.push(Item::Child(self.import_element(ce, Some(id)))),
                c if xml::is_blank(&c) => {}
                c => items.push(Item::Xml(c)),
            }
        }
        self.node_mut(id).items = items;
        self.register_id(id);
        id
    }

    /// Indexes the node by its Freeplane ID, or gives it a new one when it has
    /// none or another node already has it.
    fn register_id(&mut self, id: NodeId) {
        match self.node(id).attr("ID").map(str::to_string) {
            Some(fid) if !self.ids.contains_key(&fid) => {
                Arc::make_mut(&mut self.ids).insert(fid, id);
            }
            _ => self.assign_new_id(id),
        }
    }

    fn assign_new_id(&mut self, id: NodeId) {
        let fid = loop {
            let candidate = format!("ID_{}", fastrand::u32(1..i32::MAX as u32));
            if !self.ids.contains_key(&candidate) {
                break candidate;
            }
        };
        self.node_mut(id).set_attr("ID", &fid);
        Arc::make_mut(&mut self.ids).insert(fid, id);
    }

    fn init_new_node(&mut self, id: NodeId) {
        self.assign_new_id(id);
        let now = now_millis();
        let node = self.node_mut(id);
        node.set_attr("CREATED", &now);
        node.set_attr("MODIFIED", &now);
    }

    pub fn to_xml(&self) -> String {
        let mut root = Element::new("map");
        root.attrs = self.map_attrs.clone();
        for item in &self.map_items {
            root.children.push(match item {
                Item::Xml(x) => x.clone(),
                Item::Child(c) => XNode::Element(self.export_element(*c)),
            });
        }
        let doc = xml::Document { prolog: self.prolog.clone(), root, epilog: self.epilog.clone() };
        xml::write_document(&doc, |e| e.name == "map" || e.name == "node")
    }

    fn export_element(&self, id: NodeId) -> Element {
        let node = self.node(id);
        let mut e = Element::new("node");
        e.attrs = node.attrs.clone();
        for item in &node.items {
            e.children.push(match item {
                Item::Xml(x) => x.clone(),
                Item::Child(c) => XNode::Element(self.export_element(*c)),
            });
        }
        e
    }

    /// Writes the map to `path` (through a temporary file, so a failed write
    /// never leaves a truncated map behind) and makes it the map's path.
    pub fn save(&mut self, path: &Path) -> Result<(), String> {
        let tmp = path.with_extension("mm.tmp");
        std::fs::write(&tmp, self.to_xml()).map_err(|e| format!("{}: {e}", tmp.display()))?;
        std::fs::rename(&tmp, path).map_err(|e| format!("{}: {e}", path.display()))?;
        self.path = Some(path.to_path_buf());
        Ok(())
    }

    pub fn path(&self) -> Option<&Path> {
        self.path.as_deref()
    }

    /// The directory relative links are resolved against.
    pub fn base_dir(&self) -> Option<PathBuf> {
        self.path.as_ref().and_then(|p| p.parent()).map(paths::normalize)
    }

    // --- Tree structure ---

    pub fn root(&self) -> NodeId {
        self.root
    }

    pub fn exists(&self, id: NodeId) -> bool {
        self.nodes.get(id).is_some_and(Option::is_some)
    }

    /// The node ids handed out so far are all below this.
    pub fn id_bound(&self) -> usize {
        self.nodes.len()
    }

    /// Makes nodes created from now on get ids at or above `bound`, so that
    /// the ids another version of this map handed out (one replaced by an
    /// earlier copy on undo) are never reused.
    pub fn reserve_ids(&mut self, bound: usize) {
        if self.nodes.len() < bound {
            self.nodes.resize_with(bound, || None);
        }
    }

    pub fn node(&self, id: NodeId) -> &Node {
        self.nodes[id].as_deref().expect("deleted node")
    }

    fn node_mut(&mut self, id: NodeId) -> &mut Node {
        Arc::make_mut(self.nodes[id].as_mut().expect("deleted node"))
    }

    pub fn children(&self, id: NodeId) -> Vec<NodeId> {
        self.node(id).children().collect()
    }

    pub fn has_children(&self, id: NodeId) -> bool {
        self.node(id).children().next().is_some()
    }

    pub fn parent(&self, id: NodeId) -> Option<NodeId> {
        self.node(id).parent
    }

    /// The node's parent, its parent, and so on up to the root.
    pub fn ancestors(&self, id: NodeId) -> impl Iterator<Item = NodeId> + '_ {
        std::iter::successors(self.parent(id), |&p| self.parent(p))
    }

    /// The node's distance from the root (the root is 0).
    pub fn depth(&self, id: NodeId) -> usize {
        self.ancestors(id).count()
    }

    pub fn is_ancestor(&self, ancestor: NodeId, id: NodeId) -> bool {
        self.ancestors(id).any(|p| p == ancestor)
    }

    /// The subtree at `id` in document order (pre-order).
    pub fn preorder(&self, id: NodeId) -> Vec<NodeId> {
        let mut out = Vec::new();
        let mut stack = vec![id];
        while let Some(n) = stack.pop() {
            out.push(n);
            let children: Vec<NodeId> = self.node(n).children().collect();
            stack.extend(children.into_iter().rev());
        }
        out
    }

    pub fn index_in_parent(&self, id: NodeId) -> Option<usize> {
        let parent = self.parent(id)?;
        self.node(parent).children().position(|c| c == id)
    }

    pub fn find_child_by_text(&self, parent: NodeId, text: &str) -> Option<NodeId> {
        self.node(parent).children().find(|&c| self.text(c) == text)
    }

    /// The texts of the node's children.
    pub fn child_texts(&self, id: NodeId) -> Vec<String> {
        self.node(id).children().map(|c| self.text(c)).collect()
    }

    /// Creates an empty child at `index` among the children (None appends).
    pub fn create_child(&mut self, parent: NodeId, index: Option<usize>) -> NodeId {
        let id = self.alloc(Node::empty(Some(parent)));
        self.init_new_node(id);
        self.attach(parent, index, id);
        id
    }

    /// Appends a new child with the given text.
    pub fn add_child(&mut self, parent: NodeId, text: &str) -> NodeId {
        let id = self.create_child(parent, None);
        self.set_text(id, text);
        id
    }

    /// Puts `child` at `index` among the children of `parent` (None appends);
    /// a child of the root goes to the right side.
    fn attach(&mut self, parent: NodeId, index: Option<usize>, child: NodeId) {
        let items = &mut self.node_mut(parent).items;
        let child_positions: Vec<usize> =
            items.iter().enumerate().filter(|(_, i)| matches!(i, Item::Child(_))).map(|(p, _)| p).collect();
        let pos = match index {
            Some(i) if i < child_positions.len() => child_positions[i],
            _ => child_positions.last().map(|p| p + 1).unwrap_or(items.len()),
        };
        items.insert(pos, Item::Child(child));
        if parent == self.root {
            self.set_right(child);
        }
    }

    fn detach(&mut self, id: NodeId) {
        if let Some(parent) = self.parent(id) {
            self.node_mut(parent).items.retain(|i| *i != Item::Child(id));
        }
    }

    /// Deletes the node and its subtree. The root cannot be deleted.
    pub fn delete(&mut self, id: NodeId) {
        if id == self.root || !self.exists(id) {
            return;
        }
        self.detach(id);
        for n in self.preorder(id) {
            if let Some(fid) = self.node(n).attr("ID").map(str::to_string)
                && self.ids.get(&fid) == Some(&n)
            {
                Arc::make_mut(&mut self.ids).remove(&fid);
            }
            self.nodes[n] = None;
        }
    }

    pub fn delete_children(&mut self, id: NodeId) {
        for c in self.children(id) {
            self.delete(c);
        }
    }

    /// Moves a node (with its subtree) under `new_parent` at `index`. Refuses to
    /// move the root or a node into its own subtree.
    pub fn move_node(&mut self, id: NodeId, new_parent: NodeId, index: Option<usize>) -> bool {
        if id == self.root || id == new_parent || self.is_ancestor(id, new_parent) {
            return false;
        }
        self.detach(id);
        self.node_mut(id).parent = Some(new_parent);
        self.attach(new_parent, index, id);
        true
    }

    pub fn extract_subtree(&self, id: NodeId) -> Subtree {
        let node = self.node(id);
        Subtree {
            attrs: node.attrs.clone(),
            items: node
                .items
                .iter()
                .map(|i| match i {
                    Item::Xml(x) => SubtreeItem::Xml(x.clone()),
                    Item::Child(c) => SubtreeItem::Child(self.extract_subtree(*c)),
                })
                .collect(),
        }
    }

    /// Inserts a copy of `subtree`; nodes keep their IDs where still free, so a
    /// cut and paste keeps node links working.
    pub fn insert_subtree(&mut self, parent: NodeId, index: Option<usize>, subtree: &Subtree) -> NodeId {
        let id = self.build_subtree(parent, subtree);
        self.attach(parent, index, id);
        id
    }

    fn build_subtree(&mut self, parent: NodeId, subtree: &Subtree) -> NodeId {
        let id = self.alloc(Node { attrs: subtree.attrs.clone(), items: Vec::new(), parent: Some(parent) });
        self.register_id(id);
        let mut items = Vec::new();
        for item in &subtree.items {
            items.push(match item {
                SubtreeItem::Xml(x) => Item::Xml(x.clone()),
                SubtreeItem::Child(c) => Item::Child(self.build_subtree(id, c)),
            });
        }
        self.node_mut(id).items = items;
        id
    }

    // --- Node content ---

    /// The node's plain text: its TEXT attribute, or the text of its HTML rich content.
    pub fn text(&self, id: NodeId) -> String {
        let node = self.node(id);
        if let Some(t) = node.attr("TEXT") {
            return t.to_string();
        }
        if let Some(rich) = node.elements().find(|e| e.name == "richcontent" && e.attr("TYPE") == Some("NODE")) {
            return html_to_plain(rich);
        }
        node.attr("LOCALIZED_TEXT").unwrap_or("").to_string()
    }

    pub fn set_text(&mut self, id: NodeId, text: &str) {
        if self.text(id) == text && self.node(id).attr("TEXT").is_some() {
            return;
        }
        let node = self.node_mut(id);
        node.items.retain(
            |i| !matches!(i, Item::Xml(XNode::Element(e)) if e.name == "richcontent" && e.attr("TYPE") == Some("NODE")),
        );
        if node.attr("TEXT").is_none() {
            node.attrs.insert(0, ("TEXT".into(), text.into()));
        } else {
            node.set_attr("TEXT", text);
        }
        self.touch(id);
    }

    fn touch(&mut self, id: NodeId) {
        let now = now_millis();
        self.node_mut(id).set_attr("MODIFIED", &now);
    }

    pub fn freeplane_id(&self, id: NodeId) -> &str {
        self.node(id).attr("ID").unwrap_or("")
    }

    pub fn find_by_freeplane_id(&self, fid: &str) -> Option<NodeId> {
        self.ids.get(fid).copied().filter(|&n| self.exists(n))
    }

    pub fn link(&self, id: NodeId) -> Option<&str> {
        self.node(id).attr("LINK").filter(|l| !l.is_empty())
    }

    pub fn set_link(&mut self, id: NodeId, link: Option<&str>) {
        match link {
            Some(l) => self.node_mut(id).set_attr("LINK", l),
            None => self.node_mut(id).remove_attr("LINK"),
        }
        self.touch(id);
    }

    /// The file the node links directly (`node.link.file` in Freeplane's API).
    pub fn link_file(&self, id: NodeId) -> Option<PathBuf> {
        paths::link_to_path(self.link(id)?, self.base_dir().as_deref())
    }

    pub fn set_link_file(&mut self, id: NodeId, file: &Path) {
        let link = paths::path_to_link(file, self.base_dir().as_deref());
        self.set_link(id, Some(&link));
    }

    /// The node a `#ID_...` link points at (`node.link.node`).
    pub fn link_node(&self, id: NodeId) -> Option<NodeId> {
        let target = self.link(id)?.strip_prefix('#')?;
        self.find_by_freeplane_id(target)
    }

    pub fn icons(&self, id: NodeId) -> Vec<String> {
        self.node(id)
            .elements()
            .filter(|e| e.name == "icon")
            .filter_map(|e| e.attr("BUILTIN"))
            .map(str::to_string)
            .collect()
    }

    pub fn has_icon(&self, id: NodeId, icon: &str) -> bool {
        self.icons(id).iter().any(|i| i == icon)
    }

    pub fn add_icon(&mut self, id: NodeId, icon: &str) {
        let node = self.node_mut(id);
        let el = XNode::Element(Element::new("icon").with_attr("BUILTIN", icon));
        // After the last icon, else before the first child node.
        let pos = node
            .items
            .iter()
            .rposition(|i| matches!(i, Item::Xml(XNode::Element(e)) if e.name == "icon"))
            .map(|p| p + 1)
            .or_else(|| node.items.iter().position(|i| matches!(i, Item::Child(_))))
            .unwrap_or(node.items.len());
        node.items.insert(pos, Item::Xml(el));
        self.touch(id);
    }

    /// Removes the first occurrence of the icon.
    pub fn remove_icon(&mut self, id: NodeId, icon: &str) {
        let node = self.node_mut(id);
        if let Some(pos) = node.items.iter().position(
            |i| matches!(i, Item::Xml(XNode::Element(e)) if e.name == "icon" && e.attr("BUILTIN") == Some(icon)),
        ) {
            node.items.remove(pos);
        }
        self.touch(id);
    }

    pub fn is_folded(&self, id: NodeId) -> bool {
        self.node(id).attr("FOLDED") == Some("true")
    }

    /// Unfolds every ancestor of the node, so that it can be shown.
    pub fn unfold_ancestors(&mut self, id: NodeId) {
        for p in self.ancestors(id).collect::<Vec<_>>() {
            self.set_folded(p, false);
        }
    }

    pub fn set_folded(&mut self, id: NodeId, folded: bool) {
        if self.is_folded(id) == folded {
            return;
        }
        let node = self.node_mut(id);
        if folded {
            node.set_attr("FOLDED", "true");
        } else {
            node.remove_attr("FOLDED");
        }
    }

    /// Places a child of the root on the right side (the only side this
    /// application lays out), in the map's dialect: Freeplane 1.12 writes
    /// bottom_or_right, earlier versions right.
    pub fn set_right(&mut self, id: NodeId) {
        let new_style = self
            .children(self.root)
            .iter()
            .any(|&c| matches!(self.node(c).attr("POSITION"), Some("top_or_left") | Some("bottom_or_right")));
        self.node_mut(id).set_attr("POSITION", if new_style { "bottom_or_right" } else { "right" });
    }
}

/// Plain text of an HTML rich content element: block elements become lines,
/// whitespace runs collapse as a browser would show them.
fn html_to_plain(rich: &Element) -> String {
    fn walk(nodes: &[XNode], out: &mut String) {
        for n in nodes {
            match n {
                XNode::Text(t) | XNode::CData(t) => {
                    for c in t.chars() {
                        if c.is_whitespace() {
                            if !out.ends_with([' ', '\n']) && !out.is_empty() {
                                out.push(' ');
                            }
                        } else {
                            out.push(c);
                        }
                    }
                }
                XNode::EntityRef(name) if name == "nbsp" => out.push(' '),
                XNode::Element(e) => {
                    let name = e.name.to_ascii_lowercase();
                    if name == "head" || name == "style" || name == "script" {
                        continue;
                    }
                    let block = matches!(
                        name.as_str(),
                        "p" | "div"
                            | "br"
                            | "li"
                            | "tr"
                            | "h1"
                            | "h2"
                            | "h3"
                            | "h4"
                            | "h5"
                            | "h6"
                            | "pre"
                            | "ul"
                            | "ol"
                    );
                    if block {
                        end_line(out);
                    }
                    walk(&e.children, out);
                    if block {
                        end_line(out);
                    }
                }
                _ => {}
            }
        }
    }
    /// Ends the current line (if any), without trailing spaces.
    fn end_line(out: &mut String) {
        while out.ends_with(' ') {
            out.pop();
        }
        if !out.is_empty() && !out.ends_with('\n') {
            out.push('\n');
        }
    }
    let mut out = String::new();
    walk(&rich.children, &mut out);
    out.trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r##"<map version="freeplane 1.11.5">
<!--To view this file, download free mind mapping software Freeplane from https://www.freeplane.org -->
<bookmarks/>
<node TEXT="Root" FOLDED="false" ID="ID_1" CREATED="1" MODIFIED="2" STYLE="oval">
<font SIZE="18"/>
<hook NAME="MapStyle">
    <properties edgeColorConfiguration="#808080ff" show_icon_for_attributes="true"/>

<map_styles>
<stylenode LOCALIZED_TEXT="styles.root_node" STYLE="oval">
<stylenode TEXT="Default"/>
</stylenode>
</map_styles>
</hook>
<node TEXT="&#x65e5;&#x672c;&#xa;line2" POSITION="right" ID="ID_2" CREATED="1" MODIFIED="2" LINK="docs/a%20b.md">
<icon BUILTIN="button_ok"/>
<attribute NAME="k" VALUE="v"/>
<node ID="ID_3" CREATED="1" MODIFIED="2">
<richcontent TYPE="NODE">

<html>
  <head>

  </head>
  <body>
    <p>
      Rich &amp; text
    </p>
    <p>second</p>
  </body>
</html>
</richcontent>
</node>
</node>
<node TEXT="left" POSITION="left" ID="ID_4" CREATED="1" MODIFIED="2" LINK="#ID_2"/>
</node>
</map>
"##;

    #[test]
    fn round_trips_a_freeplane_map_unchanged() {
        let map = MindMap::parse(SAMPLE).unwrap();
        assert_eq!(map.to_xml(), SAMPLE);
    }

    #[test]
    fn reads_node_content() {
        let map = MindMap::parse(SAMPLE).unwrap();
        let root = map.root();
        assert_eq!(map.text(root), "Root");
        let kids = map.children(root);
        assert_eq!(kids.len(), 2);
        assert_eq!(map.text(kids[0]), "日本\nline2");
        assert_eq!(map.icons(kids[0]), vec!["button_ok"]);
        assert_eq!(map.link_node(kids[1]), Some(kids[0]));
        let rich = map.children(kids[0])[0];
        assert_eq!(map.text(rich), "Rich & text\nsecond");
    }

    #[test]
    fn edits_keep_unrelated_content() {
        let mut map = MindMap::parse(SAMPLE).unwrap();
        let first = map.children(map.root())[0];
        let rich = map.children(first)[0];
        map.set_text(rich, "plain");
        assert_eq!(map.text(rich), "plain");
        map.remove_icon(first, ICON_CHECKMARK);
        assert!(map.icons(first).is_empty());
        map.add_icon(first, ICON_CHECKMARK);
        let c = map.create_child(first, Some(0));
        map.set_text(c, "new");
        let xml = map.to_xml();
        assert!(xml.contains("<attribute NAME=\"k\" VALUE=\"v\"/>"));
        assert!(xml.contains("<hook NAME=\"MapStyle\">"));
        assert!(!xml.contains("richcontent"));
        let reread = MindMap::parse(&xml).unwrap();
        let first2 = reread.children(reread.root())[0];
        let kids: Vec<String> = reread.children(first2).iter().map(|&k| reread.text(k)).collect();
        assert_eq!(kids, vec!["new", "plain"]);
    }

    #[test]
    fn cut_and_paste_keeps_ids() {
        let mut map = MindMap::parse(SAMPLE).unwrap();
        let first = map.children(map.root())[0];
        let left = map.children(map.root())[1];
        let sub = map.extract_subtree(first);
        map.delete(first);
        assert_eq!(map.link_node(left), None);
        let pasted = map.insert_subtree(left, None, &sub);
        assert_eq!(map.link_node(left), Some(pasted));
        // A second paste gets fresh IDs.
        let again = map.insert_subtree(left, None, &sub);
        assert_ne!(map.freeplane_id(again), map.freeplane_id(pasted));
    }

    #[test]
    fn copies_share_unchanged_nodes() {
        let mut map = MindMap::parse(SAMPLE).unwrap();
        let copy = map.clone();
        let first = map.children(map.root())[0];
        map.set_text(first, "changed");
        let shared = |id: NodeId| Arc::ptr_eq(map.nodes[id].as_ref().unwrap(), copy.nodes[id].as_ref().unwrap());
        assert!(!shared(first));
        assert!(map.preorder(map.root()).into_iter().filter(|&n| n != first).all(shared));
        // The copy is unaffected.
        assert_eq!(copy.text(first), "日本\nline2");
        assert_eq!(copy.to_xml(), SAMPLE);
    }

    #[test]
    fn new_map_is_readable() {
        let mut map = MindMap::new("Root");
        let c = map.create_child(map.root(), None);
        map.set_text(c, "child");
        map.set_folded(c, true);
        let reread = MindMap::parse(&map.to_xml()).unwrap();
        let rc = reread.children(reread.root())[0];
        assert_eq!(reread.text(rc), "child");
        assert!(reread.is_folded(rc));
    }
}
