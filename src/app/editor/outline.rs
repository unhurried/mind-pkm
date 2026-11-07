//! The outline's folding: which headings it lists, given the ones whose
//! folding the user toggled. Pure.
//!
//! A heading's entries are the headings after it up to the next one of the
//! same or a higher (smaller) level. By default the outline is unfolded down
//! to H3: entries of H1 and H2 are unfolded, those of H3 and below folded. The
//! window keeps the headings toggled from that default by their anchors, which
//! stay the same while other parts of the page are edited.

use crate::markdown::HeadingInfo;
use std::collections::HashSet;

/// Headings of this level and below are folded by default.
const FOLDED_FROM_LEVEL: u8 = 3;

/// An outline entry that is listed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Entry {
    /// The heading's index in the page's headings.
    pub index: usize,
    /// Whether its entries are folded; None when it has none.
    pub folded: Option<bool>,
}

/// The entries listed, in order.
pub(super) fn entries(headings: &[HeadingInfo], toggled: &HashSet<String>) -> Vec<Entry> {
    let mut out = Vec::new();
    // The level of the folded heading whose entries are being skipped.
    let mut skipping: Option<u8> = None;
    for (index, h) in headings.iter().enumerate() {
        match skipping {
            Some(level) if h.level > level => continue,
            _ => skipping = None,
        }
        let has_entries = headings.get(index + 1).is_some_and(|next| next.level > h.level);
        let folded = has_entries.then(|| (h.level >= FOLDED_FROM_LEVEL) != toggled.contains(&h.anchor));
        if folded == Some(true) {
            skipping = Some(h.level);
        }
        out.push(Entry { index, folded });
    }
    out
}

/// Folds an unfolded heading or unfolds a folded one.
pub(super) fn toggle(toggled: &mut HashSet<String>, heading: &HeadingInfo) {
    if !toggled.remove(&heading.anchor) {
        toggled.insert(heading.anchor.clone());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::markdown::Document;

    /// The listed headings' texts, with `+` before a folded one and `-` before an unfolded one.
    fn listed(page: &str, toggled: &HashSet<String>) -> Vec<String> {
        let doc = Document::parse(page);
        entries(&doc.headings, toggled)
            .into_iter()
            .map(|e| {
                let mark = match e.folded {
                    None => "",
                    Some(true) => "+",
                    Some(false) => "-",
                };
                format!("{mark}{}", doc.headings[e.index].text)
            })
            .collect()
    }

    fn toggled(anchors: &[&str]) -> HashSet<String> {
        anchors.iter().map(|a| a.to_string()).collect()
    }

    #[test]
    fn unfolded_down_to_h3_by_default() {
        let page = "# A\n\n## B\n\n### C\n\n#### D\n\n##### E\n\n### F\n\n## G\n";
        assert_eq!(listed(page, &HashSet::new()), ["-A", "-B", "+C", "F", "G"]);
    }

    #[test]
    fn toggling_folds_and_unfolds() {
        let page = "# A\n\n## B\n\n### C\n\n#### D\n\n##### E\n\n## G\n";
        assert_eq!(listed(page, &toggled(&["c"])), ["-A", "-B", "-C", "+D", "G"]);
        assert_eq!(listed(page, &toggled(&["c", "d"])), ["-A", "-B", "-C", "-D", "E", "G"]);
        assert_eq!(listed(page, &toggled(&["b"])), ["-A", "+B", "G"]);
        assert_eq!(listed(page, &toggled(&["a"])), ["+A"]);
        // A folded heading hides its entries whatever was toggled under it.
        assert_eq!(listed(page, &toggled(&["a", "c"])), ["+A"]);
        let doc = Document::parse(page);
        let mut set = HashSet::new();
        toggle(&mut set, &doc.headings[0]);
        assert_eq!(set, toggled(&["a"]));
        toggle(&mut set, &doc.headings[0]);
        assert!(set.is_empty());
    }

    #[test]
    fn level_jumps_nest() {
        // H4 right under H1 is H1's entry, shown as H1 is unfolded.
        assert_eq!(listed("# A\n\n#### D\n\n##### E\n\n# B\n", &HashSet::new()), ["-A", "+D", "B"]);
        // A page starting below H1, and an H3 before an H2.
        assert_eq!(listed("### C\n\n#### D\n\n## B\n\n### F\n", &HashSet::new()), ["+C", "-B", "F"]);
    }

    #[test]
    fn repeated_headings_fold_on_their_own() {
        let page = "## X\n\n### Y\n\n#### Z\n\n## X\n\n### Y\n\n#### Z\n";
        assert_eq!(listed(page, &toggled(&["y-1"])), ["-X", "+Y", "-X", "-Y", "Z"]);
    }
}
