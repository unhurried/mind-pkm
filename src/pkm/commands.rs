//! The PKM commands (one per menu entry of the former Freeplane add-on),
//! independent of the UI: an `Err` is the message to show the user, and the
//! commands that ask something first are split into a check and an execution.

use super::*;
use crate::mindmap::ICON_CHECKMARK;
use chrono::NaiveDate;

/// The page template shipped with the application; a `template.md` in the
/// configuration directory takes its place (see `crate::settings`).
pub const DEFAULT_TEMPLATE: &str = include_str!("../../assets/template.md");

const UNLINKED_NODE_NAME: &str = "unlinked";

fn doc_dir_if_linked(map: &MindMap, id: NodeId) -> Result<Option<PathBuf>, String> {
    if linked_file(map, id).is_none() { Ok(None) } else { load_doc_dir(map).map(Some) }
}

// --- Open (F1) ---

/// The files of the selected page and directory nodes; other nodes are skipped.
pub fn open_targets(map: &MindMap, selection: &[NodeId]) -> Result<Vec<PathBuf>, String> {
    let doc_dir = load_doc_dir(map)?;
    Ok(selection
        .iter()
        .filter(|&&n| doc_type(map, n, &doc_dir).is_some())
        .filter_map(|&n| linked_file(map, n))
        .collect())
}

// --- Edit (F2) ---

/// What Edit does with a node: a page or directory node is renamed on disk
/// (editing the text alone would desynchronize it from the file system);
/// any other node just gets its text edited.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditKind {
    Text,
    Rename(DocType),
}

pub fn edit_kind(map: &MindMap, selection: &[NodeId], id: NodeId) -> Result<EditKind, String> {
    if selection.len() > 1 {
        return Err("select a single node to edit.".into());
    }
    let Some(doc_dir) = doc_dir_if_linked(map, id)? else { return Ok(EditKind::Text) };
    Ok(doc_type(map, id, &doc_dir).map(EditKind::Rename).unwrap_or(EditKind::Text))
}

/// Renames the page or directory of a node to `new_name`, with the node.
pub fn rename(map: &mut MindMap, id: NodeId, kind: DocType, new_name: &str) -> Result<(), String> {
    let valid = match kind {
        DocType::Page => is_valid_name(new_name),
        DocType::Directory => is_valid_directory_name(new_name),
    };
    if !valid {
        return Err("new name includes invalid characters".into());
    }
    let doc_dir = load_doc_dir(map)?;
    match kind {
        DocType::Page => rename_page(map, &doc_dir, id, new_name),
        DocType::Directory => rename_directory(map, &doc_dir, id, new_name),
    }
}

fn rename_page(map: &mut MindMap, doc_dir: &Path, id: NodeId, new_name: &str) -> Result<(), String> {
    let old_name = map.text(id);
    let page = page_file(doc_dir, &old_name);
    let assets = assets_dir(doc_dir, &old_name);
    let new_page = page_file(doc_dir, new_name);
    let new_assets = assets_dir(doc_dir, new_name);

    // A case-only rename on a case-insensitive file system names the same file.
    let same = paths::same_file(&page, &new_page);
    if new_page.exists() && !same {
        return Err("new page file already exists.".into());
    }
    if new_assets.exists() && !paths::same_file(&assets, &new_assets) {
        return Err("new page assets directory already exists.".into());
    }
    std::fs::rename(&page, &new_page).map_err(|_| "failed to rename page file.")?;
    if assets.exists() && std::fs::rename(&assets, &new_assets).is_err() {
        let _ = std::fs::rename(&new_page, &page);
        return Err("failed to rename page assets directory.".into());
    }

    // Literal replace, not regex: page names may contain metacharacters ("C++ Memo").
    let text = read_page(&new_page).map_err(|e| e.to_string())?;
    let text = text.replace(&file_name(&page), &file_name(&new_page));
    let text = text.replace(&file_name(&assets), &file_name(&new_assets));
    write_page(&new_page, &text).map_err(|e| e.to_string())?;

    map.set_text(id, new_name);
    map.set_link_file(id, &new_page);
    // The ToDo items copied from this page name it too.
    rename_todo_items(map, doc_dir, &old_name, new_name);
    Ok(())
}

fn rename_directory(map: &mut MindMap, doc_dir: &Path, id: NodeId, new_name: &str) -> Result<(), String> {
    let old = directory_dir(doc_dir, &map.text(id));
    let new = directory_dir(doc_dir, new_name);
    if new.exists() && !paths::same_file(&old, &new) {
        return Err("new directory already exists.".into());
    }
    std::fs::rename(&old, &new).map_err(|_| "failed to rename directory.")?;
    map.set_text(id, new_name);
    map.set_link_file(id, &new);
    Ok(())
}

// --- New Page (F9) / New Directory (F10) ---

/// Creates the page `<docDir>/<text>.md` from the template, with its assets
/// directory, and links the node to it. Returns the page file (to be opened).
pub fn new_page(map: &mut MindMap, id: NodeId, template: &str, today: NaiveDate) -> Result<PathBuf, String> {
    let doc_dir = load_doc_dir(map)?;
    let name = map.text(id);
    if !is_valid_name(&name) {
        return Err("page name includes invalid characters".into());
    }
    let page = page_file(&doc_dir, &name);
    if page.exists() {
        map.set_link_file(id, &page);
        return Err("page file already exists.".into());
    }
    let assets = assets_dir(&doc_dir, &name);
    if assets.exists() {
        return Err("page assets directory already exists.".into());
    }
    std::fs::create_dir(&assets).map_err(|e| e.to_string())?;

    let template = template.strip_prefix(BOM).unwrap_or(template);
    let text = template
        .replace("${page_assets_name}", &file_name(&assets))
        .replace("${today}", &today.format(DATE_FORMAT).to_string());
    write_page(&page, &text).map_err(|e| e.to_string())?;
    map.set_link_file(id, &page);
    Ok(page)
}

/// Creates the directory `<docDir>/<text>/` and links the node to it.
pub fn new_directory(map: &mut MindMap, id: NodeId) -> Result<PathBuf, String> {
    let doc_dir = load_doc_dir(map)?;
    let name = map.text(id);
    if !is_valid_directory_name(&name) {
        return Err("directory name includes invalid characters".into());
    }
    let dir = directory_dir(&doc_dir, &name);
    if dir.exists() {
        return Err("directory already exists.".into());
    }
    std::fs::create_dir(&dir).map_err(|e| e.to_string())?;
    map.set_link_file(id, &dir);
    Ok(dir)
}

// --- Delete (F11) ---

/// The nodes to delete (a node under another selected node goes with that
/// node; the root is never deleted), each with the kind of document it links
/// when it is a page or directory node: their files go too, any other node is
/// deleted alone.
pub fn delete_targets(map: &MindMap, selection: &[NodeId]) -> Vec<(NodeId, Option<DocType>)> {
    let doc_dir = load_doc_dir(map).ok();
    let nodes: Vec<NodeId> = selection.iter().copied().filter(|&n| n != map.root()).collect();
    sorted_selection(map, &nodes)
        .into_iter()
        .map(|n| (n, doc_dir.as_deref().and_then(|d| doc_type(map, n, d))))
        .collect()
}

pub fn delete_question(map: &MindMap, targets: &[(NodeId, Option<DocType>)]) -> String {
    let describe = |(n, t): &(NodeId, Option<DocType>)| match t {
        Some(DocType::Page) => format!("\"{}\"", map.text(*n)),
        Some(DocType::Directory) => format!("directory \"{}\"", map.text(*n)),
        None => format!("node \"{}\"", map.text(*n)),
    };
    if targets.len() == 1 {
        format!("Are you sure to delete {}?", describe(&targets[0]))
    } else {
        let list: Vec<String> = targets.iter().map(describe).collect();
        format!("Are you sure to delete the following?\n{}", list.join("\n"))
    }
}

/// Deletes the targets' files with `remove` (given the existing files of one
/// target at a time: a page and its assets directory, or a directory; the
/// application moves them to the Recycle Bin), then their nodes. A target
/// whose files can't be removed keeps its node and is reported; a node that
/// is neither a page nor a directory has no files and simply goes.
pub fn delete_execute(
    map: &mut MindMap,
    targets: &[(NodeId, Option<DocType>)],
    remove: &mut dyn FnMut(&[PathBuf]) -> Result<(), String>,
) -> Result<(), String> {
    let doc_dir = if targets.iter().any(|(_, t)| t.is_some()) { Some(load_doc_dir(map)?) } else { None };
    let (mut failed_pages, mut failed_dirs) = (Vec::new(), Vec::new());
    for &(n, t) in targets {
        if !map.exists(n) {
            continue;
        }
        let name = map.text(n);
        let (Some(t), Some(doc_dir)) = (t, &doc_dir) else {
            map.delete(n);
            continue;
        };
        let (files, failed) = match t {
            DocType::Page => (vec![page_file(doc_dir, &name), assets_dir(doc_dir, &name)], &mut failed_pages),
            DocType::Directory => (vec![directory_dir(doc_dir, &name)], &mut failed_dirs),
        };
        // Checked again, whatever the targets came from: nothing outside the
        // document directory, nor the directory itself, is ever removed.
        let inside = files.iter().all(|f| paths::is_strictly_inside(f, doc_dir));
        let files: Vec<PathBuf> = files.into_iter().filter(|f| f.exists()).collect();
        if !inside || (!files.is_empty() && remove(&files).is_err()) {
            failed.push(name);
            continue;
        }
        map.delete(n);
    }
    let mut errors = Vec::new();
    if !failed_pages.is_empty() {
        errors.push(format!("failed to delete page: {}", failed_pages.join(", ")));
    }
    if !failed_dirs.is_empty() {
        errors.push(format!("failed to delete directory: {}", failed_dirs.join(", ")));
    }
    if errors.is_empty() { Ok(()) } else { Err(errors.join("\n")) }
}

// --- Add Assets ---

/// Creates the page's assets directory and links it from the top of the page.
pub fn add_assets(map: &MindMap, selection: &[NodeId], id: NodeId) -> Result<(), String> {
    if selection.len() > 1 {
        return Err("select a single page to add assets to.".into());
    }
    let doc_dir = load_doc_dir(map)?;
    if doc_type(map, id, &doc_dir) != Some(DocType::Page) {
        return Err("target node is not a page.".into());
    }
    let name = map.text(id);
    let page = page_file(&doc_dir, &name);
    let assets = assets_dir(&doc_dir, &name);
    if assets.exists() {
        return Err("page assets directory already exists.".into());
    }
    std::fs::create_dir(&assets).map_err(|e| e.to_string())?;
    let text = read_page(&page).map_err(|e| e.to_string())?;
    write_page(&page, &format!("[assets]({})\n\n{text}", file_name(&assets))).map_err(|e| e.to_string())
}

// --- Add to ToDo (F8) ---

/// Copies the selected items (children of page nodes) to the top of root >
/// ToDo, each linked back to its page, keeping their order. Returns the new nodes.
pub fn add_to_todo(map: &mut MindMap, selection: &[NodeId]) -> Result<Vec<NodeId>, String> {
    let todo = map.find_child_by_text(map.root(), TODO_NODE_NAME).ok_or("ToDo node is missing.")?;
    let doc_dir = load_doc_dir(map)?;
    let others: Vec<String> = selection
        .iter()
        .filter(|&&n| map.parent(n).is_none_or(|p| doc_type(map, p, &doc_dir) != Some(DocType::Page)))
        .map(|&n| map.text(n))
        .collect();
    if !others.is_empty() {
        return Err(format!("not a todo item: {}", others.join(", ")));
    }
    let mut created = Vec::new();
    for (i, &item) in selection.iter().enumerate() {
        let page = map.parent(item).unwrap();
        let text = todo_item_text(&map.text(item), &map.text(page));
        let file = linked_file(map, page);
        let n = map.create_child(todo, Some(i));
        map.set_text(n, &text);
        if let Some(f) = file {
            map.set_link_file(n, &f);
        }
        created.push(n);
    }
    Ok(created)
}

// --- Toggle Checkmark (F7) ---

pub fn toggle_checkmark(map: &mut MindMap, selection: &[NodeId]) {
    for &n in selection {
        if map.has_icon(n, ICON_CHECKMARK) {
            map.remove_icon(n, ICON_CHECKMARK);
        } else {
            map.add_icon(n, ICON_CHECKMARK);
        }
    }
}

// --- Fold One Level (F3) / Unfold One Level (F4) ---
// The same algorithm as Freeplane's FoldOneLevelAction / UnfoldOneLevelAction.

fn max_depth(map: &MindMap, n: NodeId) -> usize {
    if map.is_folded(n) || !map.has_children(n) {
        return map.depth(n);
    }
    map.children(n).iter().map(|&c| max_depth(map, c)).max().unwrap_or(0)
}

fn min_depth(map: &MindMap, n: NodeId) -> usize {
    if map.is_folded(n) {
        return map.depth(n);
    }
    map.children(n).iter().map(|&c| min_depth(map, c)).min().unwrap_or(usize::MAX)
}

fn fold_all(map: &mut MindMap, n: NodeId) {
    for d in map.preorder(n) {
        if map.has_children(d) {
            map.set_folded(d, true);
        }
    }
}

fn fold_stage(map: &mut MindMap, n: NodeId, stage: usize) {
    if map.depth(n) < stage {
        map.set_folded(n, false);
        for c in map.children(n) {
            fold_stage(map, c, stage);
        }
    } else {
        fold_all(map, n);
    }
}

pub fn fold_one_level(map: &mut MindMap, n: NodeId) {
    let stage = max_depth(map, n).saturating_sub(1);
    fold_stage(map, n, stage);
}

pub fn unfold_one_level(map: &mut MindMap, n: NodeId) {
    let stage = min_depth(map, n).saturating_add(1);
    fold_stage(map, n, stage);
}

// --- Find Unlinked ---

/// Lists the pages, directories and orphaned assets directories of the document
/// directory that no node links under root > unlinked (rebuilt on each run).
pub fn find_unlinked(map: &mut MindMap) -> Result<NodeId, String> {
    let doc_dir = load_doc_dir(map)?;
    let root = map.root();
    // Prepared before collecting the linked files, so that the previous
    // unlinked node's own links don't count as "linked".
    let unlinked = match map.find_child_by_text(root, UNLINKED_NODE_NAME) {
        Some(n) => {
            map.delete_children(n);
            n
        }
        // A child of the root goes to the right side.
        None => map.add_child(root, UNLINKED_NODE_NAME),
    };
    let linked = collect_nodes_by_linked_file(map, root);
    let is_linked = |p: &Path| linked.contains_key(&paths::key(p));

    let mut entries: Vec<PathBuf> =
        std::fs::read_dir(&doc_dir).map_err(|e| e.to_string())?.filter_map(|e| e.ok().map(|e| e.path())).collect();
    entries.sort();
    let (mut pages, mut dirs, mut assets) = (Vec::new(), Vec::new(), Vec::new());
    for entry in entries {
        // The search index is maintenance data, not a document - never report it.
        if entry.file_name().is_some_and(|n| n == INDEX_DIR_NAME) {
            continue;
        }
        if entry.is_file() {
            if let Some(name) = page_name_of(&entry)
                && !is_linked(&entry)
            {
                pages.push((name, entry));
            }
        } else if let Some(page) = page_file_of_assets_dir(&entry) {
            if !page.exists() {
                assets.push((file_name(&entry), entry));
            }
        } else if !is_linked(&entry) {
            dirs.push((file_name(&entry), entry));
        }
    }
    for (label, items) in [("page", pages), ("directory", dirs), ("asset", assets)] {
        let category = map.add_child(unlinked, label);
        for (name, path) in items {
            let c = map.add_child(category, &name);
            map.set_link_file(c, &path);
        }
    }
    Ok(unlinked)
}

#[cfg(test)]
mod tests {
    use super::super::testutil::Fixture;
    use super::*;

    fn today() -> NaiveDate {
        NaiveDate::from_ymd_opt(2024, 5, 10).unwrap()
    }

    /// Removes the files for good (the application moves them to the Recycle Bin).
    fn remove_files(files: &[PathBuf]) -> Result<(), String> {
        for f in files {
            let result = if f.is_dir() { std::fs::remove_dir_all(f) } else { std::fs::remove_file(f) };
            result.map_err(|e| e.to_string())?;
        }
        Ok(())
    }

    #[test]
    fn new_page_creates_page_from_template() {
        let mut f = Fixture::new();
        let root = f.map.root();
        let n = f.add_child(root, "My Page");
        let page = new_page(&mut f.map, n, DEFAULT_TEMPLATE, today()).unwrap();
        assert!(f.doc_dir.join("My Page.assets").is_dir());
        let text = read_page(&page).unwrap();
        assert!(text.starts_with("[assets](My Page.assets)\r\n"));
        assert!(text.contains("### 24/05/10"));
        assert_eq!(doc_type(&f.map, n, &f.doc_dir), Some(DocType::Page));
    }

    #[test]
    fn new_page_links_an_existing_page_instead_of_overwriting() {
        let mut f = Fixture::new();
        let root = f.map.root();
        f.write_page("P", "keep");
        let n = f.add_child(root, "P");
        assert_eq!(new_page(&mut f.map, n, DEFAULT_TEMPLATE, today()).unwrap_err(), "page file already exists.");
        assert_eq!(read_page(&page_file(&f.doc_dir, "P")).unwrap(), "keep");
        assert_eq!(doc_type(&f.map, n, &f.doc_dir), Some(DocType::Page));
        let bad = f.add_child(root, "a/b");
        assert!(new_page(&mut f.map, bad, DEFAULT_TEMPLATE, today()).is_err());
    }

    #[test]
    fn rename_page_moves_files_and_rewrites_references() {
        let mut f = Fixture::new();
        let root = f.map.root();
        f.write_page("C++ Memo", "[assets](C++ Memo.assets)\nsee C++ Memo.md\n");
        std::fs::create_dir(assets_dir(&f.doc_dir, "C++ Memo")).unwrap();
        let n = f.add_page_node(root, "C++ Memo");
        let todo = f.todo;
        let item = f.add_page_node(todo, "C++ Memo");
        f.map.set_text(item, "step (C++ Memo)");
        let sel = [n];
        assert_eq!(edit_kind(&f.map, &sel, n).unwrap(), EditKind::Rename(DocType::Page));
        rename(&mut f.map, n, DocType::Page, "Rust Memo").unwrap();
        assert!(!page_file(&f.doc_dir, "C++ Memo").exists());
        assert!(assets_dir(&f.doc_dir, "Rust Memo").is_dir());
        assert_eq!(
            read_page(&page_file(&f.doc_dir, "Rust Memo")).unwrap(),
            "[assets](Rust Memo.assets)\nsee Rust Memo.md\n"
        );
        assert_eq!(f.map.text(n), "Rust Memo");
        assert_eq!(doc_type(&f.map, n, &f.doc_dir), Some(DocType::Page));
        assert_eq!(f.map.text(item), "step (Rust Memo)");
    }

    #[test]
    fn rename_refuses_conflicts_and_bad_names() {
        let mut f = Fixture::new();
        let root = f.map.root();
        f.write_page("A", "");
        f.write_page("B", "");
        let n = f.add_page_node(root, "A");
        assert_eq!(rename(&mut f.map, n, DocType::Page, "B").unwrap_err(), "new page file already exists.");
        std::fs::create_dir(assets_dir(&f.doc_dir, "C")).unwrap();
        assert_eq!(rename(&mut f.map, n, DocType::Page, "C").unwrap_err(), "new page assets directory already exists.");
        assert_eq!(rename(&mut f.map, n, DocType::Page, "x:y").unwrap_err(), "new name includes invalid characters");
        let plain = f.add_child(root, "plain");
        assert_eq!(edit_kind(&f.map, &[plain], plain).unwrap(), EditKind::Text);
        assert!(edit_kind(&f.map, &[plain, n], plain).is_err());
    }

    #[test]
    fn rename_directory() {
        let mut f = Fixture::new();
        let root = f.map.root();
        std::fs::create_dir(f.doc_dir.join("D")).unwrap();
        let n = f.add_dir_node(root, "D");
        rename(&mut f.map, n, DocType::Directory, "E").unwrap();
        assert!(f.doc_dir.join("E").is_dir());
        assert_eq!(doc_type(&f.map, n, &f.doc_dir), Some(DocType::Directory));
    }

    #[test]
    fn delete_pages_and_directories() {
        let mut f = Fixture::new();
        let root = f.map.root();
        f.write_page("P", "");
        std::fs::create_dir(assets_dir(&f.doc_dir, "P")).unwrap();
        std::fs::create_dir_all(f.doc_dir.join("D")).unwrap();
        f.write_page("Q", "");
        let p = f.add_page_node(root, "P");
        let d = f.add_dir_node(root, "D");
        let q = f.add_page_node(d, "Q");
        let plain = f.add_child(root, "plain");
        let targets = delete_targets(&f.map, &[q, p, d, root]);
        assert_eq!(targets, vec![(p, Some(DocType::Page)), (d, Some(DocType::Directory))]);
        assert_eq!(delete_question(&f.map, &targets), "Are you sure to delete the following?\n\"P\"\ndirectory \"D\"");
        let mut removed = Vec::new();
        delete_execute(&mut f.map, &targets, &mut |files| {
            removed.push(files.to_vec());
            remove_files(files)
        })
        .unwrap();
        // One removal per target: a page with its assets directory, a directory.
        let d = &f.doc_dir;
        assert_eq!(removed, vec![vec![d.join("P.md"), d.join("P.assets")], vec![d.join("D")]]);
        assert!(!page_file(&f.doc_dir, "P").exists() && !assets_dir(&f.doc_dir, "P").exists());
        assert!(!f.doc_dir.join("D").exists());
        assert!(!f.map.exists(p) && !f.map.exists(q) && f.map.exists(plain));
    }

    #[test]
    fn delete_plain_nodes() {
        let mut f = Fixture::new();
        let root = f.map.root();
        f.write_page("P", "");
        let p = f.add_page_node(root, "P");
        let plain = f.add_child(root, "plain");
        let below = f.add_child(plain, "below");
        let targets = delete_targets(&f.map, &[plain, below, p]);
        assert_eq!(targets, vec![(p, Some(DocType::Page)), (plain, None)]);
        assert_eq!(delete_question(&f.map, &targets), "Are you sure to delete the following?\n\"P\"\nnode \"plain\"");
        assert_eq!(delete_question(&f.map, &targets[1..]), "Are you sure to delete node \"plain\"?");
        let mut removed = Vec::new();
        delete_execute(&mut f.map, &targets, &mut |files| {
            removed.push(files.to_vec());
            remove_files(files)
        })
        .unwrap();
        // Only the page has files.
        assert_eq!(removed.len(), 1);
        assert!(!f.map.exists(p) && !f.map.exists(plain) && !f.map.exists(below));
        // Without a document directory every node is a plain one.
        let mut g = Fixture::new();
        let config = g.map.children(g.map.root())[0];
        g.map.delete(config);
        let n = g.add_child(g.map.root(), "n");
        let targets = delete_targets(&g.map, &[n]);
        assert_eq!(targets, vec![(n, None)]);
        delete_execute(&mut g.map, &targets, &mut remove_files).unwrap();
        assert!(!g.map.exists(n));
        assert!(delete_targets(&g.map, &[g.map.root()]).is_empty());
    }

    #[test]
    fn a_failed_removal_keeps_the_node() {
        let mut f = Fixture::new();
        let root = f.map.root();
        f.write_page("P", "");
        std::fs::create_dir(f.doc_dir.join("D")).unwrap();
        let p = f.add_page_node(root, "P");
        let d = f.add_dir_node(root, "D");
        let targets = delete_targets(&f.map, &[p, d]);
        let err = delete_execute(&mut f.map, &targets, &mut |_| Err("in use".into())).unwrap_err();
        assert_eq!(err, "failed to delete page: P\nfailed to delete directory: D");
        assert!(f.map.exists(p) && f.map.exists(d));
        assert!(page_file(&f.doc_dir, "P").exists() && f.doc_dir.join("D").is_dir());
    }

    /// Whatever the nodes' texts and links, F11 never removes the document
    /// directory or anything outside it - not even when handed targets directly.
    #[test]
    fn delete_never_leaves_the_document_directory() {
        // Texts that, joined to the document directory, resolve to it, its
        // parent or a sibling (Windows strips trailing dots and spaces).
        let texts = ["", " ", ".", "..", "...", ". ", "../outside", "../docs", "a/../..", "CON"];
        for text in texts {
            let mut f = Fixture::new();
            let (doc_dir, parent) = (f.doc_dir.clone(), f.dir.path().to_path_buf());
            let map_file = parent.join("map.mm");
            let outside = parent.join("outside");
            std::fs::create_dir(&outside).unwrap();
            std::fs::write(outside.join("keep.md"), "").unwrap();
            std::fs::write(parent.join("outside.md"), "").unwrap();
            let kept_page = f.write_page("kept", "");
            let root = f.map.root();
            let links = [
                doc_dir.clone(),
                parent.clone(),
                outside.clone(),
                parent.join("outside.md"),
                page_file(&doc_dir, text),
                directory_dir(&doc_dir, text),
            ];
            let nodes: Vec<NodeId> = links
                .iter()
                .map(|link| {
                    let n = f.add_child(root, text);
                    f.map.set_link_file(n, link);
                    n
                })
                .collect();
            for n in nodes {
                let targets = delete_targets(&f.map, &[n]);
                let _ = delete_execute(&mut f.map, &targets, &mut remove_files);
                let forced = [(n, Some(DocType::Page)), (n, Some(DocType::Directory))];
                let _ = delete_execute(&mut f.map, &forced, &mut remove_files);
            }
            for kept in [&doc_dir, &kept_page, &outside.join("keep.md"), &parent.join("outside.md"), &map_file] {
                assert!(kept.exists(), "{text:?} removed {}", kept.display());
            }
        }
    }

    #[test]
    fn rejects_names_the_file_system_would_resolve_differently() {
        let mut f = Fixture::new();
        let root = f.map.root();
        for name in ["", "aux", "..", "a\nb"] {
            let n = f.add_child(root, name);
            assert_eq!(
                new_page(&mut f.map, n, DEFAULT_TEMPLATE, today()).unwrap_err(),
                "page name includes invalid characters"
            );
            assert_eq!(new_directory(&mut f.map, n).unwrap_err(), "directory name includes invalid characters");
        }
        let dotted = f.add_child(root, "Plan.");
        assert_eq!(new_directory(&mut f.map, dotted).unwrap_err(), "directory name includes invalid characters");
        // A page name may end with a dot: its files end in ".md" / ".assets".
        let page = new_page(&mut f.map, dotted, DEFAULT_TEMPLATE, today()).unwrap();
        assert_eq!(page, f.doc_dir.join("Plan..md"));
        std::fs::create_dir(f.doc_dir.join("D")).unwrap();
        let d = f.add_dir_node(root, "D");
        assert_eq!(
            rename(&mut f.map, d, DocType::Directory, "E.").unwrap_err(),
            "new name includes invalid characters"
        );
        assert_eq!(
            rename(&mut f.map, d, DocType::Directory, "..").unwrap_err(),
            "new name includes invalid characters"
        );
        assert!(f.doc_dir.join("D").is_dir());
        // Nothing but the two pages' files and D was created.
        let mut entries: Vec<String> =
            std::fs::read_dir(&f.doc_dir).unwrap().map(|e| e.unwrap().file_name().to_string_lossy().into()).collect();
        entries.sort();
        assert_eq!(entries, vec!["D", "Plan..assets", "Plan..md"]);
    }

    #[test]
    fn add_assets_links_directory_from_page() {
        let mut f = Fixture::new();
        let root = f.map.root();
        f.write_page("P", "body\n");
        let p = f.add_page_node(root, "P");
        add_assets(&f.map, &[p], p).unwrap();
        assert_eq!(read_page(&page_file(&f.doc_dir, "P")).unwrap(), "[assets](P.assets)\n\nbody\n");
        assert_eq!(add_assets(&f.map, &[p], p).unwrap_err(), "page assets directory already exists.");
    }

    #[test]
    fn add_to_todo_copies_items_to_the_top() {
        let mut f = Fixture::new();
        let root = f.map.root();
        let todo = f.todo;
        f.add_child(todo, "existing");
        f.write_page("P", "");
        let p = f.add_page_node(root, "P");
        let a = f.add_child(p, "a");
        let b = f.add_child(p, "b");
        let created = add_to_todo(&mut f.map, &[a, b]).unwrap();
        assert_eq!(created.len(), 2);
        assert_eq!(f.texts(todo), vec!["a (P)", "b (P)", "existing"]);
        assert!(add_to_todo(&mut f.map, &[p]).is_err());
    }

    #[test]
    fn fold_and_unfold_one_level() {
        let mut f = Fixture::new();
        let root = f.map.root();
        let a = f.add_child(root, "a");
        let b = f.add_child(a, "b");
        f.add_child(b, "c");
        fold_one_level(&mut f.map, root);
        assert!(f.map.is_folded(b) && !f.map.is_folded(a));
        fold_one_level(&mut f.map, root);
        assert!(f.map.is_folded(a));
        unfold_one_level(&mut f.map, root);
        assert!(!f.map.is_folded(a) && f.map.is_folded(b));
        unfold_one_level(&mut f.map, root);
        assert!(!f.map.is_folded(b));
    }

    #[test]
    fn find_unlinked_sorts_entries() {
        let mut f = Fixture::new();
        let root = f.map.root();
        f.write_page("Linked", "");
        f.add_page_node(root, "Linked");
        f.write_page("Lonely", "");
        std::fs::create_dir_all(f.doc_dir.join("Dir")).unwrap();
        std::fs::create_dir_all(f.doc_dir.join("Gone.assets")).unwrap();
        std::fs::create_dir_all(f.doc_dir.join("Linked.assets")).unwrap();
        std::fs::create_dir_all(f.doc_dir.join(INDEX_DIR_NAME)).unwrap();
        let unlinked = find_unlinked(&mut f.map).unwrap();
        let cats = f.map.children(unlinked);
        assert_eq!(f.texts(cats[0]), vec!["Lonely"]);
        assert_eq!(f.texts(cats[1]), vec!["Dir"]);
        assert_eq!(f.texts(cats[2]), vec!["Gone.assets"]);
        // Running again rebuilds it without counting its own links.
        let again = find_unlinked(&mut f.map).unwrap();
        assert_eq!(again, unlinked);
        assert_eq!(f.texts(f.map.children(unlinked)[0]), vec!["Lonely"]);
    }

    #[test]
    fn toggles_checkmark() {
        let mut f = Fixture::new();
        let root = f.map.root();
        let a = f.add_child(root, "a");
        toggle_checkmark(&mut f.map, &[a]);
        assert!(f.map.has_icon(a, ICON_CHECKMARK));
        toggle_checkmark(&mut f.map, &[a]);
        assert!(!f.map.has_icon(a, ICON_CHECKMARK));
    }
}
