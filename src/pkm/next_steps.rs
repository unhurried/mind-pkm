//! The Next Steps refresh.
//!
//! Up to 3 list items under a page's `### Next Steps` heading are synced into
//! the page node's children; a page whose children already match is left
//! untouched. The same refresh deletes the root > ToDo items (`<item> (<page>)`
//! linking the page file) whose text is no longer among *all* the page's Next
//! Steps items, or whose page is gone. It then fills root > ToDo > Due Today /
//! Past Due (created when missing) with the Next Steps items whose leading
//! `yy/MM/dd` date is today / before today, minus those already among ToDo's
//! direct children.
//!
//! The work is split in three so that file I/O stays off the UI thread:
//! [`snapshot`] copies what is needed from the map, [`compute`] reads the pages
//! (on a background thread) and [`apply`] makes the changes to the map.

use super::{
    DUE_TODAY_NODE_NAME, DocType, PAST_DUE_NODE_NAME, TODO_NODE_NAME, doc_type_of, page_file, page_name_of,
    todo_item_text,
};
use crate::markdown::page::read_page;
use crate::mindmap::{MindMap, NodeId};
use crate::paths;
use chrono::NaiveDate;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

const NEXT_STEPS_HEADING: &str = "### Next Steps";
const MAX_NEXT_STEPS: usize = 3;
// "* " / "- " plus at least 2 more characters, so "* x" isn't a list item.
const MIN_LIST_ITEM_LENGTH: usize = 3;

#[derive(Debug, Clone)]
pub struct Snapshot {
    doc_dir: PathBuf,
    /// Every node of the map with a linked file: (node, text, linked file).
    nodes: Vec<(NodeId, String, PathBuf)>,
    /// The direct children of root > ToDo: (node, text, directly linked file).
    todo_items: Vec<(NodeId, String, Option<PathBuf>)>,
}

#[derive(Debug, Clone, Default)]
pub struct Update {
    doc_dir: PathBuf,
    /// Page nodes (with their text when read) and their first Next Steps items.
    next_steps: Vec<(NodeId, String, Vec<String>)>,
    stale_todo_items: Vec<NodeId>,
    due_today: Vec<DueItem>,
    past_due: Vec<DueItem>,
}

#[derive(Debug, Clone, PartialEq)]
struct DueItem {
    page: String,
    item: String,
    due: NaiveDate,
}

pub fn snapshot(map: &MindMap, doc_dir: &Path) -> Snapshot {
    let nodes = map
        .preorder(map.root())
        .into_iter()
        .filter_map(|n| super::linked_file(map, n).map(|f| (n, map.text(n), f)))
        .collect();
    let todo_items = map
        .find_child_by_text(map.root(), TODO_NODE_NAME)
        .map(|todo| map.children(todo).into_iter().map(|c| (c, map.text(c), map.link_file(c))).collect())
        .unwrap_or_default();
    Snapshot { doc_dir: doc_dir.to_path_buf(), nodes, todo_items }
}

/// Every list item under the page's Next Steps heading, without the list
/// marker. None when the file can't be read, so one broken link doesn't
/// interrupt the scan of the rest of the map.
pub fn read_next_step_items(page_file: &Path) -> Option<Vec<String>> {
    let text = read_page(page_file).ok()?;
    let lines: Vec<&str> = text.lines().collect();
    let Some(start) = lines.iter().position(|l| *l == NEXT_STEPS_HEADING) else {
        return Some(Vec::new());
    };
    Some(
        lines[start + 1..]
            .iter()
            .take_while(|l| !l.starts_with('#'))
            .filter(|l| is_list_item(l))
            .map(|l| l[2..].to_string())
            .collect(),
    )
}

fn is_list_item(line: &str) -> bool {
    (line.starts_with("* ") || line.starts_with("- ")) && line.encode_utf16().count() > MIN_LIST_ITEM_LENGTH
}

/// Reads the pages; runs off the UI thread.
pub fn compute(snapshot: &Snapshot, today: NaiveDate) -> Update {
    let dir = &snapshot.doc_dir;
    // Every page is read once, whether page nodes or ToDo items (or both) refer to it.
    let mut items_by_file: HashMap<String, Option<Vec<String>>> = HashMap::new();
    let mut items_of = |file: &Path| -> Option<Vec<String>> {
        items_by_file.entry(paths::key(file)).or_insert_with(|| read_next_step_items(file)).clone()
    };

    let mut update = Update { doc_dir: dir.clone(), ..Update::default() };
    let mut pages: Vec<String> = Vec::new();
    for (node, text, linked) in &snapshot.nodes {
        if doc_type_of(text, linked, dir) != Some(DocType::Page) {
            continue;
        }
        if let Some(items) = items_of(&page_file(dir, text)) {
            update.next_steps.push((*node, text.clone(), items.into_iter().take(MAX_NEXT_STEPS).collect()));
            if !pages.contains(text) {
                pages.push(text.clone());
            }
        }
    }

    for (node, text, linked) in &snapshot.todo_items {
        let Some(linked) = linked else { continue };
        let Some(page) = page_name_of(linked) else { continue };
        if !paths::same_file(linked, &page_file(dir, &page)) {
            continue;
        }
        let stale = match items_of(linked) {
            None => true,
            Some(items) => !items.iter().any(|i| todo_item_text(i, &page) == *text),
        };
        if stale {
            update.stale_todo_items.push(*node);
        }
    }

    for page in pages {
        for item in items_of(&page_file(dir, &page)).unwrap_or_default() {
            let Some(due) = super::due_date_of(&item) else { continue };
            let entry = DueItem { page: page.clone(), item, due };
            if due == today {
                update.due_today.push(entry);
            } else if due < today {
                update.past_due.push(entry);
            }
        }
    }
    update.past_due.sort_by_key(|e| e.due);
    update
}

/// Applies the update to the map; nodes deleted or renamed since the snapshot
/// are skipped. Returns whether anything changed.
pub fn apply(map: &mut MindMap, update: &Update) -> bool {
    let mut changed = false;
    for (node, text, lines) in &update.next_steps {
        if !map.exists(*node) || map.text(*node) != *text {
            continue;
        }
        if map.child_texts(*node) != *lines {
            sync_children(map, *node, lines.iter().map(|l| (l.as_str(), None)));
            changed = true;
        }
    }
    for &stale in &update.stale_todo_items {
        if map.exists(stale) {
            map.delete(stale);
            changed = true;
        }
    }
    changed | apply_due_items(map, update)
}

fn apply_due_items(map: &mut MindMap, update: &Update) -> bool {
    let doc_dir = &update.doc_dir;
    let Some(todo) = map.find_child_by_text(map.root(), TODO_NODE_NAME) else { return false };
    let listed: HashSet<(String, Option<String>)> =
        map.children(todo).into_iter().map(|c| (map.text(c), map.link_file(c).map(|f| paths::key(&f)))).collect();
    let mut changed = false;
    for (name, items) in [(DUE_TODAY_NODE_NAME, &update.due_today), (PAST_DUE_NODE_NAME, &update.past_due)] {
        // The items not listed yet, as ToDo items: the text and the page linked.
        let pending: Vec<(String, PathBuf)> = items
            .iter()
            .map(|i| (todo_item_text(&i.item, &i.page), page_file(doc_dir, &i.page)))
            .filter(|(text, page)| !listed.contains(&(text.clone(), Some(paths::key(page)))))
            .collect();
        let holder = match map.find_child_by_text(todo, name) {
            Some(h) => h,
            None => {
                changed = true;
                map.add_child(todo, name)
            }
        };
        if map.child_texts(holder).iter().eq(pending.iter().map(|(text, _)| text)) {
            continue;
        }
        sync_children(map, holder, pending.iter().map(|(text, page)| (text.as_str(), Some(page.as_path()))));
        changed = true;
    }
    changed
}

/// Makes the children of `parent` the given texts, in order: a child whose
/// text is wanted stays (with its id, icons and attributes) and is moved into
/// place, the others are deleted, and the missing texts are added (linking
/// the file given).
fn sync_children<'a>(map: &mut MindMap, parent: NodeId, wanted: impl Iterator<Item = (&'a str, Option<&'a Path>)>) {
    let mut old = map.children(parent);
    for (index, (text, file)) in wanted.enumerate() {
        let child = match old.iter().position(|&c| map.text(c) == text) {
            Some(i) => {
                let c = old.remove(i);
                map.move_node(c, parent, Some(index));
                c
            }
            None => {
                let c = map.create_child(parent, Some(index));
                map.set_text(c, text);
                if let Some(file) = file {
                    map.set_link_file(c, file);
                }
                c
            }
        };
        debug_assert_eq!(map.index_in_parent(child), Some(index));
    }
    for c in old {
        map.delete(c);
    }
}

/// The whole refresh, synchronously.
#[cfg(test)]
pub fn refresh(map: &mut MindMap, doc_dir: &Path, today: NaiveDate) -> bool {
    let snap = snapshot(map, doc_dir);
    let update = compute(&snap, today);
    apply(map, &update)
}

#[cfg(test)]
mod tests {
    use super::super::testutil::Fixture;
    use super::*;
    use chrono::Days;

    fn today() -> NaiveDate {
        NaiveDate::from_ymd_opt(2024, 5, 10).unwrap()
    }

    fn day(offset: i64) -> String {
        let d = if offset >= 0 {
            today().checked_add_days(Days::new(offset as u64))
        } else {
            today().checked_sub_days(Days::new((-offset) as u64))
        };
        d.unwrap().format(super::super::DATE_FORMAT).to_string()
    }

    fn refresh_fixture(f: &mut Fixture) -> bool {
        let dir = f.doc_dir.clone();
        refresh(&mut f.map, &dir, today())
    }

    fn page(f: &mut Fixture, name: &str, text: &str, children: &[&str]) -> NodeId {
        f.write_page(name, text);
        let root = f.map.root();
        let n = f.add_page_node(root, name);
        for c in children {
            f.add_child(n, c);
        }
        n
    }

    fn todo_item(f: &mut Fixture, item: &str, page_name: &str, parent: Option<NodeId>) -> NodeId {
        let parent = parent.unwrap_or(f.todo);
        let n = f.add_page_node(parent, page_name);
        f.map.set_text(n, &todo_item_text(item, page_name));
        n
    }

    fn todo_items(f: &Fixture) -> Vec<NodeId> {
        f.map
            .children(f.todo)
            .into_iter()
            .filter(|&c| ![DUE_TODAY_NODE_NAME, PAST_DUE_NODE_NAME].contains(&f.map.text(c).as_str()))
            .collect()
    }

    #[test]
    fn syncs_up_to_three_items_under_the_heading() {
        let mut f = Fixture::new();
        let text = "[assets](Task.assets)\n\n## Contents\n\n### Next Steps\n\n* First step\n- Second step\n* x\n* Third step\n* Fourth step\n\n## Journal\n";
        let n = page(&mut f, "Task", text, &[]);
        refresh_fixture(&mut f);
        assert_eq!(f.texts(n), vec!["First step", "Second step", "Third step"]);
    }

    #[test]
    fn stops_at_the_next_heading_and_handles_crlf() {
        let mut f = Fixture::new();
        let n = page(&mut f, "Task", "### Next Steps\r\n* One\r\n#### Sub\r\n* Two\r\n", &["old"]);
        refresh_fixture(&mut f);
        assert_eq!(f.texts(n), vec!["One"]);
    }

    #[test]
    fn leaves_a_matching_page_alone() {
        let mut f = Fixture::new();
        let n = page(&mut f, "Task", "### Next Steps\n* One\n", &["One"]);
        let child = f.map.children(n)[0];
        refresh_fixture(&mut f);
        assert_eq!(f.map.children(n), vec![child]);
    }

    #[test]
    fn children_still_wanted_keep_their_nodes() {
        let mut f = Fixture::new();
        let n = page(&mut f, "Task", "### Next Steps\n* step B\n* step C\n* step A\n", &["step A", "old", "step B"]);
        let [a, old, b] = f.map.children(n)[..] else { panic!() };
        f.map.add_icon(a, "button_ok");
        refresh_fixture(&mut f);
        assert_eq!(f.texts(n), vec!["step B", "step C", "step A"]);
        let children = f.map.children(n);
        assert_eq!((children[0], children[2]), (b, a));
        assert!(f.map.has_icon(a, "button_ok"));
        assert!(!f.map.exists(old));
    }

    #[test]
    fn a_page_without_heading_loses_its_children() {
        let mut f = Fixture::new();
        let n = page(&mut f, "Task", "nothing\n", &["old"]);
        refresh_fixture(&mut f);
        assert!(f.texts(n).is_empty());
    }

    #[test]
    fn deletes_stale_todo_items_only() {
        let mut f = Fixture::new();
        page(&mut f, "Task", "### Next Steps\n\n* Kept step\n* One\n* Two\n* Four\n", &[]);
        let gone = todo_item(&mut f, "Gone step", "Task", None);
        f.map.add_icon(gone, "button_ok");
        let kept = todo_item(&mut f, "Kept step", "Task", None);
        let fourth = todo_item(&mut f, "Four", "Task", None);
        let todo = f.todo;
        let unlinked = f.add_child(todo, "call the plumber");
        let external = f.add_child(todo, "read it (notes)");
        let outside = f.dir.path().join("notes.md");
        f.map.set_link_file(external, &outside);
        let orphan = todo_item(&mut f, "Some step", "Removed Page", None);
        let note = todo_item(&mut f, "Gone step", "Task", Some(kept));
        refresh_fixture(&mut f);
        assert!(!f.map.exists(gone));
        assert!(!f.map.exists(orphan));
        assert!(f.map.exists(note));
        assert_eq!(todo_items(&f), vec![kept, fourth, unlinked, external]);
    }

    #[test]
    fn files_due_items() {
        let mut f = Fixture::new();
        let today_item = format!("{} send the report", day(0));
        let yesterday = format!("{} pay the bill", day(-1));
        let last_week = format!("{} call back", day(-7));
        page(
            &mut f,
            "Task",
            &format!("### Next Steps\n\n* {} review\n* {yesterday}\n* undated step\n* {today_item}\n", day(1)),
            &[],
        );
        page(&mut f, "Other", &format!("### Next Steps\n\n* {last_week}\n* 26/13/40 not a date\n"), &[]);
        assert!(refresh_fixture(&mut f));
        let due_today = f.map.find_child_by_text(f.todo, DUE_TODAY_NODE_NAME).unwrap();
        let past_due = f.map.find_child_by_text(f.todo, PAST_DUE_NODE_NAME).unwrap();
        assert_eq!(f.texts(due_today), vec![todo_item_text(&today_item, "Task")]);
        assert_eq!(f.texts(past_due), vec![todo_item_text(&last_week, "Other"), todo_item_text(&yesterday, "Task")]);
        let link = f.map.link_file(f.map.children(due_today)[0]).unwrap();
        assert!(paths::same_file(&link, &page_file(&f.doc_dir, "Task")));
        // A second run changes nothing.
        assert!(!refresh_fixture(&mut f));
    }

    #[test]
    fn leaves_out_due_items_already_in_todo() {
        let mut f = Fixture::new();
        let today_item = format!("{} send the report", day(0));
        let yesterday = format!("{} pay the bill", day(-1));
        page(&mut f, "Task", &format!("### Next Steps\n\n* {today_item}\n* {yesterday}\n"), &[]);
        todo_item(&mut f, &yesterday, "Task", None);
        refresh_fixture(&mut f);
        let past_due = f.map.find_child_by_text(f.todo, PAST_DUE_NODE_NAME).unwrap();
        assert!(f.texts(past_due).is_empty());
        let due_today = f.map.find_child_by_text(f.todo, DUE_TODAY_NODE_NAME).unwrap();
        assert_eq!(f.texts(due_today), vec![todo_item_text(&today_item, "Task")]);
    }

    #[test]
    fn creates_holders_when_missing() {
        let mut f = Fixture::new();
        assert!(refresh_fixture(&mut f));
        assert_eq!(f.texts(f.todo), vec![DUE_TODAY_NODE_NAME, PAST_DUE_NODE_NAME]);
    }
}
