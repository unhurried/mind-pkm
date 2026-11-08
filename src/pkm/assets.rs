//! A page's assets folder as the editor's assets pane shows it: the tree of
//! its files and folders, renaming and deleting them - never anything that is
//! not strictly inside the document directory.

use super::{ASSETS_SUFFIX, is_valid_directory_name};
use crate::paths;
use std::path::{Path, PathBuf};

/// A file or folder of an assets folder.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub path: PathBuf,
    pub name: String,
    /// What a folder holds; None for a file.
    pub children: Option<Vec<Entry>>,
}

impl Entry {
    pub fn is_dir(&self) -> bool {
        self.children.is_some()
    }
}

/// The assets folder of the file `<name>.md`: `<name>.assets` next to it.
pub fn assets_dir_of(file: &Path) -> PathBuf {
    let stem = paths::file_stem(file);
    file.with_file_name(format!("{stem}{ASSETS_SUFFIX}"))
}

/// What the folder holds, folders before files, each in name order ignoring
/// case; None when it is no folder or leaves the document directory. Links
/// (including the assets root) are omitted, never followed.
pub fn read_tree(dir: &Path, doc_dir: &Path) -> Option<Vec<Entry>> {
    let resolved = paths::resolve_inside(dir, doc_dir).ok()?;
    resolved.is_dir().then(|| read_dir(dir, doc_dir))
}

fn read_dir(dir: &Path, doc_dir: &Path) -> Vec<Entry> {
    let Ok(resolved) = paths::resolve_inside(dir, doc_dir) else { return Vec::new() };
    let Ok(entries) = std::fs::read_dir(resolved) else { return Vec::new() };
    let mut list: Vec<Entry> = entries
        .flatten()
        .filter_map(|e| {
            // Retain the caller's spelling for editor selection/rename tracking.
            let path = dir.join(e.file_name());
            let resolved = paths::resolve_inside(&path, doc_dir).ok()?;
            let metadata = std::fs::symlink_metadata(resolved).ok()?;
            if paths::is_link(&metadata) || (!metadata.is_dir() && !metadata.is_file()) {
                return None;
            }
            let children = metadata.is_dir().then(|| read_dir(&path, doc_dir));
            Some(Entry { name: e.file_name().to_string_lossy().into_owned(), path, children })
        })
        .collect();
    list.sort_by(|a, b| b.is_dir().cmp(&a.is_dir()).then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase())));
    list
}

/// Whether the pane may rename or delete `path`: it lies strictly inside the
/// document directory.
pub fn can_change(path: &Path, doc_dir: &Path) -> bool {
    paths::resolve_inside(path, doc_dir).is_ok()
}

/// Renames `item` to `new_name` in its folder; the new path, or None when the
/// name is empty or unchanged.
pub fn rename(item: &Path, new_name: &str, doc_dir: &Path) -> Result<Option<PathBuf>, String> {
    let old_name = paths::file_name(item);
    if new_name.is_empty() || new_name == old_name {
        return Ok(None);
    }
    if !is_valid_directory_name(new_name) {
        return Err(format!("invalid name: {new_name}"));
    }
    let target = item.with_file_name(new_name);
    if !can_change(item, doc_dir) || !can_change(&target, doc_dir) {
        return Err(format!("failed to rename {old_name}: not in the document directory"));
    }
    // A name differing only in case is the same file on Windows: renamed, not taken.
    if target.symlink_metadata().is_ok() && !paths::same_file(item, &target) {
        return Err(format!("{new_name} already exists."));
    }
    let source = paths::resolve_inside(item, doc_dir).map_err(|e| format!("failed to rename {old_name}: {e}"))?;
    let destination =
        paths::resolve_inside(&target, doc_dir).map_err(|e| format!("failed to rename {old_name}: {e}"))?;
    std::fs::rename(source, destination).map_err(|e| format!("failed to rename {old_name}: {e}"))?;
    Ok(Some(target))
}

/// Deletes `item` (a folder with all it holds) with `remove`; the application
/// moves it to the Recycle Bin.
pub fn delete(
    item: &Path,
    doc_dir: &Path,
    remove: &mut dyn FnMut(&[PathBuf]) -> Result<(), String>,
) -> Result<(), String> {
    let name = paths::file_name(item);
    if !can_change(item, doc_dir) {
        return Err(format!("failed to delete {name}: not in the document directory"));
    }
    let item = paths::resolve_inside(item, doc_dir).map_err(|e| format!("failed to delete {name}: {e}"))?;
    remove(&[item]).map_err(|e| format!("failed to delete {name}: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(entries: &[Entry]) -> Vec<&str> {
        entries.iter().map(|e| e.name.as_str()).collect()
    }

    fn remove_files(files: &[PathBuf]) -> Result<(), String> {
        for f in files {
            let r = if f.is_dir() { std::fs::remove_dir_all(f) } else { std::fs::remove_file(f) };
            r.map_err(|e| e.to_string())?;
        }
        Ok(())
    }

    #[test]
    fn assets_folder_of_a_file() {
        assert_eq!(assets_dir_of(Path::new("/d/Trip.md")), Path::new("/d/Trip.assets"));
        assert_eq!(assets_dir_of(Path::new("/d/My Page.md")), Path::new("/d/My Page.assets"));
        assert_eq!(assets_dir_of(Path::new("/d/Plan..md")), Path::new("/d/Plan..assets"));
    }

    #[test]
    fn tree_lists_folders_first_by_name() {
        let dir = tempfile::tempdir().unwrap();
        let assets = dir.path().join("Trip.assets");
        assert_eq!(read_tree(&assets, dir.path()), None);
        std::fs::create_dir_all(assets.join("raw/deep")).unwrap();
        std::fs::create_dir(assets.join("Bin")).unwrap();
        for f in ["map.png", "Notes.txt", "b.pdf", "raw/x.txt"] {
            std::fs::write(assets.join(f), "").unwrap();
        }
        let tree = read_tree(&assets, dir.path()).unwrap();
        assert_eq!(names(&tree), vec!["Bin", "raw", "b.pdf", "map.png", "Notes.txt"]);
        let raw = &tree[1];
        assert_eq!(raw.path, assets.join("raw"));
        assert_eq!(names(raw.children.as_ref().unwrap()), vec!["deep", "x.txt"]);
        assert!(tree[0].is_dir() && tree[0].children.as_ref().unwrap().is_empty());
        assert!(!tree[2].is_dir());
        // A file is no folder.
        assert_eq!(read_tree(&assets.join("map.png"), dir.path()), None);
    }

    #[test]
    fn renaming() {
        let dir = tempfile::tempdir().unwrap();
        let doc_dir = dir.path().join("docs");
        let assets = doc_dir.join("Trip.assets");
        std::fs::create_dir_all(&assets).unwrap();
        let (a, b) = (assets.join("a.png"), assets.join("b.png"));
        std::fs::write(&a, "a").unwrap();
        std::fs::write(&b, "b").unwrap();
        assert_eq!(rename(&a, "", &doc_dir), Ok(None));
        assert_eq!(rename(&a, "a.png", &doc_dir), Ok(None));
        assert_eq!(rename(&a, "b.png", &doc_dir), Err("b.png already exists.".into()));
        for bad in ["x/y.png", "x\\y", "a?.png", "c:d", "dot.", "space ", "..", "aux"] {
            assert_eq!(rename(&a, bad, &doc_dir), Err(format!("invalid name: {bad}")), "{bad}");
        }
        assert_eq!(rename(&a, "route.png", &doc_dir), Ok(Some(assets.join("route.png"))));
        assert!(!a.exists() && std::fs::read_to_string(assets.join("route.png")).unwrap() == "a");
        assert_eq!(std::fs::read_to_string(&b).unwrap(), "b");
        let missing = rename(&a, "c.png", &doc_dir).unwrap_err();
        assert!(missing.starts_with("failed to rename a.png: "), "{missing}");
    }

    #[test]
    fn never_changes_anything_outside_the_document_directory() {
        let dir = tempfile::tempdir().unwrap();
        let parent = dir.path().to_path_buf();
        let doc_dir = parent.join("docs");
        let outside = parent.join("Other.assets");
        std::fs::create_dir_all(doc_dir.join("Trip.assets")).unwrap();
        std::fs::create_dir_all(&outside).unwrap();
        std::fs::write(outside.join("keep.png"), "").unwrap();
        let inside = doc_dir.join("Trip.assets").join("in.png");
        std::fs::write(&inside, "").unwrap();
        let items = [
            doc_dir.clone(),
            parent.clone(),
            outside.clone(),
            outside.join("keep.png"),
            doc_dir.join("."),
            doc_dir.join("..."),
            doc_dir.join("Trip.assets/../.."),
        ];
        for item in &items {
            assert!(delete(item, &doc_dir, &mut remove_files).is_err(), "{}", item.display());
            assert!(rename(item, "renamed", &doc_dir).is_err(), "{}", item.display());
        }
        // Renaming out of the document directory through the new name.
        assert!(rename(&inside, "..", &doc_dir).is_err());
        for kept in [&doc_dir, &outside.join("keep.png"), &inside] {
            assert!(kept.exists(), "{}", kept.display());
        }
        assert!(!parent.join("renamed").exists() && !doc_dir.join("renamed").exists());
        // Inside, deleting goes through the remover; a folder with what it holds.
        delete(&doc_dir.join("Trip.assets"), &doc_dir, &mut remove_files).unwrap();
        assert!(!doc_dir.join("Trip.assets").exists() && doc_dir.exists());
        let failed = delete(&doc_dir.join("gone.png"), &doc_dir, &mut |_| Err("no such file".into()));
        assert_eq!(failed, Err("failed to delete gone.png: no such file".into()));
    }

    #[cfg(unix)]
    #[test]
    fn never_lists_or_changes_linked_assets() {
        use std::os::unix::fs::symlink;
        let tmp = tempfile::tempdir().unwrap();
        let doc_dir = tmp.path().join("docs");
        let outside = tmp.path().join("outside");
        std::fs::create_dir(&doc_dir).unwrap();
        std::fs::create_dir(&outside).unwrap();
        let kept = outside.join("keep.txt");
        std::fs::write(&kept, "keep").unwrap();
        let assets = doc_dir.join("Trip.assets");
        symlink(&outside, &assets).unwrap();
        assert!(read_tree(&assets, &doc_dir).is_none());
        for item in [&assets, &assets.join("keep.txt"), &assets.join("new.txt")] {
            assert!(!can_change(item, &doc_dir));
            assert!(rename(item, "renamed.txt", &doc_dir).is_err());
            assert!(delete(item, &doc_dir, &mut |_| panic!("must not call remover")).is_err());
        }
        std::fs::remove_file(&assets).unwrap();
        std::fs::create_dir(&assets).unwrap();
        symlink(&outside, assets.join("nested")).unwrap();
        symlink(&kept, assets.join("linked.txt")).unwrap();
        symlink(outside.join("missing"), assets.join("dangling.txt")).unwrap();
        std::fs::write(assets.join("real.txt"), "real").unwrap();
        assert_eq!(names(&read_tree(&assets, &doc_dir).unwrap()), vec!["real.txt"]);
        assert!(rename(&assets.join("real.txt"), "dangling.txt", &doc_dir).is_err());
        assert!(delete(&assets.join("nested/keep.txt"), &doc_dir, &mut |_| panic!("must not call remover")).is_err());
        assert_eq!(std::fs::read_to_string(&kept).unwrap(), "keep");
        assert!(!outside.join("renamed.txt").exists());
    }
}
