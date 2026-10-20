//! Property tests for what must survive a round trip for any input: node
//! texts through the `.mm` writer and reader, and file paths through links.

use mind_pkm::mindmap::MindMap;
use mind_pkm::paths;
use proptest::prelude::*;
use std::path::PathBuf;

// Any text a node can hold: everything but the control characters XML 1.0
// forbids (tab, line feed and carriage return are allowed).
const NODE_TEXT: &str = "[^\u{0}-\u{8}\u{b}\u{c}\u{e}-\u{1f}]{0,40}";
// A file or directory name: no separators, nothing Windows rejects.
const NAME: &str = "[^/\\\\:*?\"<>|\u{0}-\u{1f}]{1,12}";

proptest! {
    #[test]
    fn node_texts_survive_saving_and_reading(root in NODE_TEXT, child in NODE_TEXT) {
        let mut map = MindMap::new(&root);
        let r = map.root();
        map.add_child(r, &child);
        let xml = map.to_xml();
        let reread = MindMap::parse(&xml).unwrap();
        prop_assert_eq!(reread.text(reread.root()), root);
        prop_assert_eq!(reread.child_texts(reread.root()), vec![child]);
        // What was read is written back unchanged.
        prop_assert_eq!(reread.to_xml(), xml);
    }

    #[test]
    fn files_survive_linking(names in prop::collection::vec(NAME, 1..4), up in 0usize..2) {
        prop_assume!(names.iter().all(|n| n != "." && n != ".."));
        let base = std::env::temp_dir().join("maps");
        let mut file = base.clone();
        for _ in 0..up {
            file.push("..");
        }
        file.extend(&names);
        let link = paths::path_to_link(&file, Some(&base));
        let expected: PathBuf = paths::normalize(&file);
        prop_assert_eq!(paths::link_to_path(&link, Some(&base)), Some(expected));
    }
}
