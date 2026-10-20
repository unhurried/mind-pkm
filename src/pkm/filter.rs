//! The map filter (F5): which nodes are shown.

use crate::mindmap::{MindMap, NodeId};
use chrono::NaiveDate;
use std::collections::HashSet;

pub const FILTER_HELP_MESSAGE: &str =
    "Command:\n  [a] show all\n  [t] show todo list\n  [d] due today or overdue\n  (empty) hide archive";

#[derive(Debug, Clone, PartialEq)]
pub enum Filter {
    /// `a`: every node.
    All,
    /// Empty command (the default filter): anything under a node named `archive` is hidden.
    HideArchive,
    /// `t`: root > ToDo with its ancestors and descendants.
    ToDo,
    /// `d`: nodes whose leading yy/MM/dd date is on or before the given day.
    Due(NaiveDate),
    /// Anything else: nodes containing the keyword (case-insensitive).
    Keyword(String),
}

impl Filter {
    pub fn parse(command: &str, today: NaiveDate) -> Filter {
        match command {
            "" => Filter::HideArchive,
            "a" => Filter::All,
            "t" => Filter::ToDo,
            "d" => Filter::Due(today),
            k => Filter::Keyword(k.to_lowercase()),
        }
    }

    pub fn label(&self) -> String {
        match self {
            Filter::All => "show all".into(),
            Filter::HideArchive => "hide archive".into(),
            Filter::ToDo => "todo list".into(),
            Filter::Due(_) => "due today or overdue".into(),
            Filter::Keyword(k) => format!("\"{k}\""),
        }
    }

    fn matches(&self, map: &MindMap, id: NodeId) -> bool {
        match self {
            Filter::All => true,
            Filter::HideArchive => !std::iter::once(id).chain(map.ancestors(id)).any(|n| is_archive(map, n)),
            Filter::ToDo => map.parent(id) == Some(map.root()) && map.text(id) == super::TODO_NODE_NAME,
            Filter::Due(today) => super::due_date_of(&map.text(id)).is_some_and(|d| d <= *today),
            Filter::Keyword(k) => map.text(id).to_lowercase().contains(k.as_str()),
        }
    }

    /// The visible nodes. The root is always visible; a visible node whose
    /// parent is hidden is shown under its nearest visible ancestor.
    pub fn visible(&self, map: &MindMap) -> HashSet<NodeId> {
        let mut visible = match self {
            Filter::HideArchive => outside_archives(map),
            _ => self.matching_with_relatives(map),
        };
        visible.insert(map.root());
        visible
    }

    /// The matching nodes with their ancestors and descendants.
    fn matching_with_relatives(&self, map: &MindMap) -> HashSet<NodeId> {
        let matching: Vec<NodeId> = map.preorder(map.root()).into_iter().filter(|&n| self.matches(map, n)).collect();
        let mut visible: HashSet<NodeId> = matching.iter().copied().collect();
        for &n in &matching {
            // Stops at an ancestor already added, with all of its own.
            for p in map.ancestors(n) {
                if !visible.insert(p) {
                    break;
                }
            }
        }
        for &n in &matching {
            visible.extend(map.preorder(n));
        }
        visible
    }
}

/// Every node that neither is nor lies under a node named `archive`: the
/// walk doesn't enter an archive's subtree at all.
fn outside_archives(map: &MindMap) -> HashSet<NodeId> {
    let mut visible = HashSet::new();
    let mut stack = vec![map.root()];
    while let Some(n) = stack.pop() {
        if !is_archive(map, n) {
            visible.insert(n);
            stack.extend(map.children(n));
        }
    }
    visible
}

fn is_archive(map: &MindMap, id: NodeId) -> bool {
    map.text(id).to_lowercase() == "archive"
}

#[cfg(test)]
mod tests {
    use super::*;

    fn map() -> (MindMap, Vec<NodeId>) {
        let mut m = MindMap::new("root");
        let r = m.root();
        let add = |m: &mut MindMap, p, t: &str| m.add_child(p, t);
        let todo = add(&mut m, r, "ToDo");
        let item = add(&mut m, todo, "24/05/01 old task");
        let proj = add(&mut m, r, "Project");
        let archive = add(&mut m, proj, "Archive");
        let old = add(&mut m, archive, "old stuff");
        let future = add(&mut m, proj, "24/06/01 later");
        (m, vec![todo, item, proj, archive, old, future])
    }

    /// The visible nodes by the definition, node by node.
    fn visible_by_definition(filter: &Filter, m: &MindMap) -> HashSet<NodeId> {
        let all = m.preorder(m.root());
        let mut visible: HashSet<NodeId> = all.iter().copied().filter(|&n| filter.matches(m, n)).collect();
        if *filter != Filter::HideArchive {
            for &n in all.iter().filter(|&&n| filter.matches(m, n)) {
                visible.extend(m.ancestors(n));
                visible.extend(m.preorder(n));
            }
        }
        visible.insert(m.root());
        visible
    }

    #[test]
    fn visible_nodes_follow_the_definition() {
        let mut m = MindMap::new("root");
        let r = m.root();
        let mut parents = vec![r];
        // A few levels of nodes, some named archive, some matching, nested.
        for i in 0..200usize {
            let parent = parents[(i * 7) % parents.len()];
            let text = match i % 11 {
                0 => "Archive".to_string(),
                3 => format!("24/05/0{} item {i}", i % 9 + 1),
                5 => format!("key {i}"),
                _ => format!("node {i}"),
            };
            parents.push(m.add_child(parent, &text));
        }
        let today = NaiveDate::from_ymd_opt(2024, 5, 5).unwrap();
        for command in ["", "a", "t", "d", "key", "archive", "node 1", "none"] {
            let f = Filter::parse(command, today);
            assert_eq!(f.visible(&m), visible_by_definition(&f, &m), "{command:?}");
        }
        // A root named archive hides everything else.
        m.set_text(r, "archive");
        assert_eq!(Filter::HideArchive.visible(&m), HashSet::from([r]));
    }

    #[test]
    fn filters() {
        let (m, n) = map();
        let today = NaiveDate::from_ymd_opt(2024, 5, 10).unwrap();
        let [todo, item, proj, archive, old, future] = n[..] else { panic!() };
        let v = Filter::parse("", today).visible(&m);
        assert!(v.contains(&proj) && !v.contains(&archive) && !v.contains(&old) && v.contains(&todo));
        let v = Filter::parse("t", today).visible(&m);
        assert!(v.contains(&todo) && v.contains(&item) && !v.contains(&proj));
        let v = Filter::parse("d", today).visible(&m);
        assert!(v.contains(&item) && v.contains(&todo) && !v.contains(&future));
        let v = Filter::parse("STUFF", today).visible(&m);
        assert!(v.contains(&old) && v.contains(&archive) && v.contains(&proj) && !v.contains(&todo));
        assert_eq!(Filter::parse("a", today).visible(&m).len(), 7);
    }
}
