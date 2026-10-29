//! A page's assets folder as the editor's assets pane shows it: the tree of
//! its files and folders, renaming and deleting them - never anything that is
//! not strictly inside the document directory.

use super::{ASSETS_SUFFIX, file_name, is_valid_directory_name};
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
    let stem = file.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
    file.with_file_name(format!("{stem}{ASSETS_SUFFIX}"))
}

/// What the folder holds, folders before files, each in name order ignoring
/// case; None when it is no folder. Links are listed as files, never followed.
pub fn read_tree(dir: &Path) -> Option<Vec<Entry>> {
    dir.is_dir().then(|| read_dir(dir))
}

fn read_dir(dir: &Path) -> Vec<Entry> {
    let Ok(entries) = std::fs::read_dir(dir) else { return Vec::new() };
    let mut list: Vec<Entry> = entries
        .flatten()
        .map(|e| {
            let path = e.path();
            let folder = e.file_type().is_ok_and(|t| t.is_dir());
            let children = folder.then(|| read_dir(&path));
            Entry { name: e.file_name().to_string_lossy().into_owned(), path, children }
        })
        .collect();
    list.sort_by(|a, b| b.is_dir().cmp(&a.is_dir()).then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase())));
    list
}

/// Whether the pane may rename or delete `path`: it lies strictly inside the
/// document directory.
pub fn can_change(path: &Path, doc_dir: &Path) -> bool {
    paths::is_strictly_inside(path, doc_dir)
}

/// Renames `item` to `new_name` in its folder; the new path, or None when the
/// name is empty or unchanged.
pub fn rename(item: &Path, new_name: &str, doc_dir: &Path) -> Result<Option<PathBuf>, String> {
    let old_name = file_name(item);
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
    std::fs::rename(item, &target).map_err(|e| format!("failed to rename {old_name}: {e}"))?;
    Ok(Some(target))
}

/// Deletes `item` (a folder with all it holds) with `remove`; the application
/// moves it to the Recycle Bin.
pub fn delete(
    item: &Path,
    doc_dir: &Path,
    remove: &mut dyn FnMut(&[PathBuf]) -> Result<(), String>,
) -> Result<(), String> {
    let name = file_name(item);
    if !can_change(item, doc_dir) {
        return Err(format!("failed to delete {name}: not in the document directory"));
    }
    remove(&[item.to_path_buf()]).map_err(|e| format!("failed to delete {name}: {e}"))
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
        assert_eq!(read_tree(&assets), None);
        std::fs::create_dir_all(assets.join("raw/deep")).unwrap();
        std::fs::create_dir(assets.join("Bin")).unwrap();
        for f in ["map.png", "Notes.txt", "b.pdf", "raw/x.txt"] {
            std::fs::write(assets.join(f), "").unwrap();
        }
        let tree = read_tree(&assets).unwrap();
        assert_eq!(names(&tree), vec!["Bin", "raw", "b.pdf", "map.png", "Notes.txt"]);
        let raw = &tree[1];
        assert_eq!(raw.path, assets.join("raw"));
        assert_eq!(names(raw.children.as_ref().unwrap()), vec!["deep", "x.txt"]);
        assert!(tree[0].is_dir() && tree[0].children.as_ref().unwrap().is_empty());
        assert!(!tree[2].is_dir());
        // A file is no folder.
        assert_eq!(read_tree(&assets.join("map.png")), None);
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
}
