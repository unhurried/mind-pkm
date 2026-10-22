//! The PKM conventions on top of the mind map: the document directory, the
//! node <-> page/directory mapping, ToDo items and due dates.
//!
//! - The document directory is read from `root > config > docDirPath > <path>`.
//! - Page node: text links `<docDir>/<text>.md`; directory node: `<docDir>/<text>/`;
//!   attachments live in `<docDir>/<text>.assets/`. Pages are UTF-8 with a BOM.

pub mod commands;
pub mod filter;
pub mod next_steps;

use crate::mindmap::{MindMap, NodeId};
use crate::paths;
use chrono::NaiveDate;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

/// Date format used for due dates and journal entries throughout the map and pages.
pub const DATE_FORMAT: &str = "%y/%m/%d";
/// The node under the root that holds the task list.
pub const TODO_NODE_NAME: &str = "ToDo";
/// root > ToDo children collecting the Next Steps items due today / before today.
pub const DUE_TODAY_NODE_NAME: &str = "Due Today";
pub const PAST_DUE_NODE_NAME: &str = "Past Due";
/// The search index directory inside the document directory (never a document itself).
pub const INDEX_DIR_NAME: &str = ".search-index";

const CONFIG_NODE_NAME: &str = "config";
const DOC_DIR_PATH_KEY: &str = "docDirPath";
const PAGE_SUFFIX: &str = ".md";
const ASSETS_SUFFIX: &str = ".assets";
// File system reserved characters, rejected in page and directory names.
const INVALID_NAME_CHARS: &[char] = &['\\', '/', ':', '*', '?', '"', '>', '<', '|'];
// Windows device names: a file named like one (whatever its extension) is the device.
const DEVICE_NAMES: &[&str] = &[
    "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8", "COM9", "LPT1", "LPT2",
    "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
];
const BOM: char = '\u{feff}';

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DocType {
    Page,
    Directory,
}

/// A new map with the nodes PKM needs: root > config > docDirPath > `doc_dir`,
/// and root > ToDo.
pub fn new_map(root_text: &str, doc_dir: &str) -> MindMap {
    let mut map = MindMap::new(root_text);
    let root = map.root();
    let config = map.add_child(root, CONFIG_NODE_NAME);
    let key = map.add_child(config, DOC_DIR_PATH_KEY);
    map.add_child(key, doc_dir);
    map.add_child(root, TODO_NODE_NAME);
    map
}

/// The document directory from the map's config node: root > config > docDirPath > [path].
pub fn load_doc_dir(map: &MindMap) -> Result<PathBuf, String> {
    let config = map.find_child_by_text(map.root(), CONFIG_NODE_NAME).ok_or("config node is missing.")?;
    let path = map
        .find_child_by_text(config, DOC_DIR_PATH_KEY)
        .and_then(|n| map.children(n).first().map(|&c| map.text(c)))
        .filter(|p| !p.trim().is_empty())
        .ok_or("docDirPath node is missing.")?;
    let dir = PathBuf::from(path.trim());
    // A relative path is taken relative to the map file, which is where Freeplane's
    // working directory usually is not - so only absolute paths are reliable.
    let dir = if dir.is_relative() { map.base_dir().map(|b| b.join(&dir)).unwrap_or(dir) } else { dir };
    if !dir.exists() {
        return Err("document directory is missing.".into());
    }
    Ok(paths::normalize(&dir))
}

pub fn page_file(doc_dir: &Path, name: &str) -> PathBuf {
    doc_dir.join(format!("{name}{PAGE_SUFFIX}"))
}

pub fn assets_dir(doc_dir: &Path, name: &str) -> PathBuf {
    doc_dir.join(format!("{name}{ASSETS_SUFFIX}"))
}

pub fn directory_dir(doc_dir: &Path, name: &str) -> PathBuf {
    doc_dir.join(name)
}

fn file_name(path: &Path) -> String {
    path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default()
}

/// The page name of a `<name>.md` file, or None when the name has no ".md" suffix.
pub fn page_name_of(path: &Path) -> Option<String> {
    file_name(path).strip_suffix(PAGE_SUFFIX).map(str::to_string)
}

/// Whether a path relative to the document directory ("/"-separated) is a page:
/// a `<name>.md` file outside any `<page>.assets/` directory.
pub fn is_page_path(rel_path: &str) -> bool {
    let segments: Vec<&str> = rel_path.split('/').filter(|s| !s.is_empty()).collect();
    match segments.split_last() {
        Some((last, init)) => last.ends_with(PAGE_SUFFIX) && !init.iter().any(|s| s.ends_with(ASSETS_SUFFIX)),
        None => false,
    }
}

/// The page file an assets directory belongs to, or None when the name has no ".assets" suffix.
pub fn page_file_of_assets_dir(dir: &Path) -> Option<PathBuf> {
    let name = file_name(dir);
    let page = name.strip_suffix(ASSETS_SUFFIX)?;
    Some(page_file(dir.parent()?, page))
}

/// Whether a page can be named `name`: not blank, `.` or `..`, without control
/// or reserved characters, and not a Windows device name.
pub fn is_valid_name(name: &str) -> bool {
    let stem = name.split('.').next().unwrap_or_default().trim_end_matches(' ');
    !name.trim().is_empty()
        && name != "."
        && name != ".."
        && !name.contains(INVALID_NAME_CHARS)
        && !name.contains(char::is_control)
        && !DEVICE_NAMES.iter().any(|d| d.eq_ignore_ascii_case(stem))
}

/// Whether a directory can be named `name`: a valid page name that doesn't end
/// with a dot or a space, which Windows would strip (a page's files end in
/// `.md` / `.assets`, so page names may).
pub fn is_valid_directory_name(name: &str) -> bool {
    is_valid_name(name) && !name.ends_with(['.', ' '])
}

/// Reads a page as UTF-8, without the byte-order mark `write_page` puts in front.
pub fn read_page(path: &Path) -> std::io::Result<String> {
    let bytes = std::fs::read(path)?;
    let text = String::from_utf8(bytes).map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    Ok(text.strip_prefix(BOM).map(str::to_string).unwrap_or(text))
}

/// Writes a page as UTF-8 with a byte-order mark (what Markdown editors on Windows expect).
pub fn write_page(path: &Path, text: &str) -> std::io::Result<()> {
    std::fs::write(path, format!("{BOM}{text}"))
}

/// The file linked from the node, via a linked node's link or directly.
pub fn linked_file(map: &MindMap, id: NodeId) -> Option<PathBuf> {
    map.link_node(id).and_then(|n| map.link_file(n)).or_else(|| map.link_file(id))
}

/// Page if the node links the existing file `<docDir>/<text>.md`, Directory if
/// it links the existing directory `<docDir>/<text>/`, else None. Either has
/// to lie inside the document directory: a text such as "", "." or ".." never
/// makes a node stand for the document directory itself or anything outside it.
pub fn doc_type(map: &MindMap, id: NodeId, doc_dir: &Path) -> Option<DocType> {
    doc_type_of(&map.text(id), &linked_file(map, id)?, doc_dir)
}

/// doc_type() for a node's text and linked file (usable off the UI thread).
pub fn doc_type_of(text: &str, linked: &Path, doc_dir: &Path) -> Option<DocType> {
    let links = |p: &Path| paths::same_file(linked, p) && paths::is_strictly_inside(p, doc_dir);
    if links(&page_file(doc_dir, text)) && linked.is_file() {
        Some(DocType::Page)
    } else if links(&directory_dir(doc_dir, text)) && linked.is_dir() {
        Some(DocType::Directory)
    } else {
        None
    }
}

/// The due date a text starts with ("yy/MM/dd ..."), or None when it has none
/// or it isn't a valid date. An overflowing day (02/30) is clamped to the last
/// day of the month, as java.time's default (smart) resolver does.
pub fn due_date_of(text: &str) -> Option<NaiveDate> {
    let b = text.as_bytes();
    if b.len() < 8 {
        return None;
    }
    let digit = |i: usize| b[i].is_ascii_digit();
    if !(digit(0) && digit(1) && b[2] == b'/' && digit(3) && digit(4) && b[5] == b'/' && digit(6) && digit(7)) {
        return None;
    }
    let num = |i: usize| ((b[i] - b'0') * 10 + (b[i + 1] - b'0')) as u32;
    let (year, month, day) = (2000 + num(0) as i32, num(3), num(6));
    if !(1..=12).contains(&month) || !(1..=31).contains(&day) {
        return None;
    }
    (1..=day).rev().find_map(|d| NaiveDate::from_ymd_opt(year, month, d))
}

fn todo_item_suffix(page: &str) -> String {
    format!(" ({page})")
}

/// The text of a root > ToDo item copied from a page: "item (page)".
pub fn todo_item_text(item: &str, page: &str) -> String {
    format!("{item}{}", todo_item_suffix(page))
}

/// Points the root > ToDo items copied from the page `old_name` (linking
/// `<docDir>/<old_name>.md`) at its new name; the next refresh would otherwise
/// drop them as items of a page that no longer exists.
pub fn rename_todo_items(map: &mut MindMap, doc_dir: &Path, old_name: &str, new_name: &str) {
    let Some(todo) = map.find_child_by_text(map.root(), TODO_NODE_NAME) else { return };
    let old_file = page_file(doc_dir, old_name);
    let old_suffix = todo_item_suffix(old_name);
    for item in map.children(todo) {
        if !map.link_file(item).is_some_and(|f| paths::same_file(&f, &old_file)) {
            continue;
        }
        let text = map.text(item);
        if let Some(stripped) = text.strip_suffix(&old_suffix) {
            map.set_text(item, &todo_item_text(stripped, new_name));
        }
        map.set_link_file(item, &page_file(doc_dir, new_name));
    }
}

/// Maps every file linked from the subtree at `id` (keyed by `paths::key`) to
/// the node linking it. A direct link wins over an indirect one (through a
/// node link), so a "see also" link never shadows the node representing the document.
pub fn collect_nodes_by_linked_file(map: &MindMap, id: NodeId) -> HashMap<String, NodeId> {
    let mut direct = HashMap::new();
    let mut indirect = HashMap::new();
    for n in map.preorder(id) {
        if let Some(f) = map.link_file(n) {
            direct.insert(paths::key(&f), n);
        } else if let Some(f) = linked_file(map, n) {
            indirect.insert(paths::key(&f), n);
        }
    }
    indirect.extend(direct);
    indirect
}

/// Resolves a file (e.g. a search hit) to its node: the node linking the file
/// itself, else the nearest ancestor directory's node, else - for a file under
/// `<page>.assets/` - the page's node.
pub fn find_node_for_file(nodes_by_file: &HashMap<String, NodeId>, file: &Path, doc_dir: &Path) -> Option<NodeId> {
    let doc_key = paths::key(doc_dir);
    for path in paths::normalize(file).ancestors() {
        let key = paths::key(path);
        let page_key = page_file_of_assets_dir(path).map(|page| paths::key(&page));
        if let Some(&n) = nodes_by_file.get(&key).or_else(|| nodes_by_file.get(page_key.as_ref()?)) {
            return Some(n);
        }
        if key == doc_key {
            break;
        }
    }
    None
}

/// "root > ... > text" for a node.
pub fn node_path_text(map: &MindMap, id: NodeId) -> String {
    let mut parts: Vec<String> = std::iter::once(id).chain(map.ancestors(id)).map(|n| map.text(n)).collect();
    parts.reverse();
    parts.join(" > ")
}

/// The selection in document order, without the nodes lying under another
/// selected node (Freeplane's `getSortedSelection(true)`).
pub fn sorted_selection(map: &MindMap, selection: &[NodeId]) -> Vec<NodeId> {
    let order: HashMap<NodeId, usize> = map.preorder(map.root()).into_iter().enumerate().map(|(i, n)| (n, i)).collect();
    let mut nodes: Vec<NodeId> = selection.iter().copied().filter(|n| order.contains_key(n)).collect();
    nodes.sort_by_key(|n| order[n]);
    nodes.dedup();
    let selected: HashSet<NodeId> = nodes.iter().copied().collect();
    nodes.retain(|&n| !map.ancestors(n).any(|a| selected.contains(&a)));
    nodes
}

#[cfg(test)]
pub(crate) mod testutil {
    use super::*;

    /// A map with root > config > docDirPath > [docDir] and root > ToDo, saved
    /// next to a fresh document directory.
    pub struct Fixture {
        pub dir: tempfile::TempDir,
        pub doc_dir: PathBuf,
        pub map: MindMap,
        pub todo: NodeId,
    }

    impl Fixture {
        pub fn new() -> Self {
            let dir = tempfile::tempdir().unwrap();
            let doc_dir = paths::normalize(&dir.path().join("docs"));
            std::fs::create_dir(&doc_dir).unwrap();
            let mut map = new_map("root", &doc_dir.to_string_lossy());
            let todo = map.find_child_by_text(map.root(), TODO_NODE_NAME).unwrap();
            map.save(&dir.path().join("map.mm")).unwrap();
            Fixture { dir, doc_dir, map, todo }
        }

        pub fn write_page(&self, name: &str, text: &str) -> PathBuf {
            let f = page_file(&self.doc_dir, name);
            write_page(&f, text).unwrap();
            f
        }

        pub fn add_child(&mut self, parent: NodeId, text: &str) -> NodeId {
            self.map.add_child(parent, text)
        }

        /// A node named `name` linking `<docDir>/<name>.md` (the page file is not created).
        pub fn add_page_node(&mut self, parent: NodeId, name: &str) -> NodeId {
            let n = self.add_child(parent, name);
            let f = page_file(&self.doc_dir, name);
            self.map.set_link_file(n, &f);
            n
        }

        pub fn add_dir_node(&mut self, parent: NodeId, name: &str) -> NodeId {
            let n = self.add_child(parent, name);
            let f = directory_dir(&self.doc_dir, name);
            self.map.set_link_file(n, &f);
            n
        }

        pub fn texts(&self, id: NodeId) -> Vec<String> {
            self.map.child_texts(id)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::testutil::Fixture;
    use super::*;

    #[test]
    fn load_doc_dir_reports_each_missing_piece() {
        let mut f = Fixture::new();
        assert_eq!(load_doc_dir(&f.map).unwrap(), f.doc_dir);
        let config = f.map.children(f.map.root())[0];
        let value = f.map.children(f.map.children(config)[0])[0];
        f.map.set_text(value, "/nonexistent/path");
        assert_eq!(load_doc_dir(&f.map).unwrap_err(), "document directory is missing.");
        f.map.delete_children(config);
        assert_eq!(load_doc_dir(&f.map).unwrap_err(), "docDirPath node is missing.");
        f.map.delete(config);
        assert_eq!(load_doc_dir(&f.map).unwrap_err(), "config node is missing.");
    }

    #[test]
    fn classifies_page_and_directory_nodes() {
        let mut f = Fixture::new();
        let root = f.map.root();
        f.write_page("Page", "");
        std::fs::create_dir(f.doc_dir.join("Dir")).unwrap();
        let page = f.add_page_node(root, "Page");
        let dir = f.add_dir_node(root, "Dir");
        let missing = f.add_page_node(root, "Missing");
        let plain = f.add_child(root, "Plain");
        assert_eq!(doc_type(&f.map, page, &f.doc_dir), Some(DocType::Page));
        assert_eq!(doc_type(&f.map, dir, &f.doc_dir), Some(DocType::Directory));
        assert_eq!(doc_type(&f.map, missing, &f.doc_dir), None);
        assert_eq!(doc_type(&f.map, plain, &f.doc_dir), None);
        // A node linking a page through a node link counts too.
        let via = f.add_child(root, "Page");
        let target = format!("#{}", f.map.freeplane_id(page));
        f.map.set_link(via, Some(&target));
        assert_eq!(doc_type(&f.map, via, &f.doc_dir), Some(DocType::Page));
    }

    #[test]
    fn never_classifies_the_document_directory_or_beyond() {
        let mut f = Fixture::new();
        let root = f.map.root();
        let doc_dir = f.doc_dir.clone();
        let parent = f.dir.path().to_path_buf();
        for (text, link) in [("", &doc_dir), (".", &doc_dir), ("...", &doc_dir), ("..", &parent), ("../docs", &doc_dir)]
        {
            let n = f.add_child(root, text);
            f.map.set_link_file(n, link);
            assert_eq!(doc_type(&f.map, n, &f.doc_dir), None, "{text:?}");
        }
        // A page outside the document directory, named relative to it.
        std::fs::write(parent.join("outside.md"), "").unwrap();
        let n = f.add_child(root, "../outside");
        f.map.set_link_file(n, &parent.join("outside.md"));
        assert_eq!(doc_type(&f.map, n, &f.doc_dir), None);
    }

    #[test]
    fn name_conventions() {
        let d = Path::new("/d");
        assert_eq!(page_name_of(Path::new("/d/Page.md")).as_deref(), Some("Page"));
        assert_eq!(page_name_of(Path::new("/d/Page.assets")), None);
        assert_eq!(page_file_of_assets_dir(Path::new("/d/Page.assets")), Some(d.join("Page.md")));
        assert_eq!(page_file_of_assets_dir(Path::new("/d/Page")), None);
        for (p, expected) in [
            ("Page.md", true),
            ("Dir/Page.md", true),
            ("assets.md", true),
            ("Page.assets/Note.md", false),
            ("Page.assets/sub/x.md", false),
            ("Page.pdf", false),
        ] {
            assert_eq!(is_page_path(p), expected, "{p}");
        }
        for name in ["C++ Memo", "Mr.", "...", ".hidden", "conference", "COM10", "日本 語"] {
            assert!(is_valid_name(name), "{name}");
        }
        for name in ["", "  ", ".", "..", "a/b", "a\\b", "a?", "a\nb", "a\tb", "aux", "CON", "nul.txt", "Com1 .md"] {
            assert!(!is_valid_name(name), "{name:?}");
        }
        assert!(is_valid_directory_name("Plan"));
        for name in ["Plan.", "Plan ", "...", "aux"] {
            assert!(!is_valid_directory_name(name), "{name:?}");
        }
    }

    #[test]
    fn due_dates() {
        assert_eq!(due_date_of("24/03/05 do it"), NaiveDate::from_ymd_opt(2024, 3, 5));
        assert_eq!(due_date_of("23/02/30 x"), NaiveDate::from_ymd_opt(2023, 2, 28));
        assert_eq!(due_date_of("24/13/01 x"), None);
        assert_eq!(due_date_of("24-03-05"), None);
        assert_eq!(due_date_of("x 24/03/05"), None);
    }

    #[test]
    fn resolves_files_to_nodes() {
        let mut f = Fixture::new();
        let root = f.map.root();
        let page_file = f.write_page("Page", "");
        std::fs::create_dir_all(f.doc_dir.join("Sub")).unwrap();
        std::fs::create_dir_all(f.doc_dir.join("Page.assets")).unwrap();
        let page = f.add_page_node(root, "Page");
        let dir = f.add_dir_node(root, "Sub");
        let via = f.add_child(root, "via");
        let target = format!("#{}", f.map.freeplane_id(page));
        f.map.set_link(via, Some(&target));
        let by_file = collect_nodes_by_linked_file(&f.map, root);
        // The direct link wins over the indirect one.
        assert_eq!(by_file.get(&paths::key(&page_file)), Some(&page));
        let d = &f.doc_dir;
        assert_eq!(find_node_for_file(&by_file, &d.join("Sub/nested.pdf"), d), Some(dir));
        assert_eq!(find_node_for_file(&by_file, &d.join("Page.assets/image.png"), d), Some(page));
        assert_eq!(find_node_for_file(&by_file, &d.join("./Page.md"), d), Some(page));
        assert_eq!(find_node_for_file(&by_file, &d.join("orphan.md"), d), None);
    }

    #[test]
    fn node_path_and_sorted_selection() {
        let mut f = Fixture::new();
        let root = f.map.root();
        let a = f.add_child(root, "a");
        let b = f.add_child(a, "b");
        let c = f.add_child(root, "c");
        assert_eq!(node_path_text(&f.map, b), "root > a > b");
        assert_eq!(sorted_selection(&f.map, &[c, b, a]), vec![a, c]);
    }

    #[test]
    fn renames_todo_items() {
        let mut f = Fixture::new();
        let todo = f.todo;
        let item = f.add_page_node(todo, "Old");
        f.map.set_text(item, "do it (Old)");
        rename_todo_items(&mut f.map, &f.doc_dir.clone(), "Old", "New");
        assert_eq!(f.map.text(item), "do it (New)");
        assert!(paths::same_file(&f.map.link_file(item).unwrap(), &page_file(&f.doc_dir, "New")));
    }

    #[test]
    fn pages_round_trip_with_bom() {
        let f = Fixture::new();
        let p = f.doc_dir.join("x.md");
        write_page(&p, "hello").unwrap();
        assert_eq!(std::fs::read(&p).unwrap()[..3], [0xef, 0xbb, 0xbf]);
        assert_eq!(read_page(&p).unwrap(), "hello");
    }
}
