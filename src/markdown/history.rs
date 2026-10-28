//! Undo and redo for a page: snapshots of the whole text with the selection
//! (pages are small), consecutive typing counting as one change.

use super::edit::Sel;
use std::time::{Duration, Instant};

/// At most this many changes are kept.
const MAX_STEPS: usize = 500;
/// Typing after a pause this long starts a new change.
const TYPING_PAUSE: Duration = Duration::from_secs(1);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Snapshot {
    pub text: String,
    pub sel: Sel,
}

#[derive(Debug, Default)]
pub struct History {
    undo: Vec<Snapshot>,
    redo: Vec<Snapshot>,
    /// When and where the last typed character went, and whether it ended a word.
    typing: Option<(Instant, usize, bool)>,
}

impl History {
    /// Records the state before a change. `typed` is the character typed
    /// and where it went, for typing that may join the previous change.
    pub fn record(&mut self, before: Snapshot, typed: Option<(char, usize)>, now: Instant) {
        let joins = match (typed, self.typing) {
            (Some((_, at)), Some((when, end, word_end))) => at == end && !word_end && now - when < TYPING_PAUSE,
            _ => false,
        };
        self.typing = typed.map(|(c, at)| (now, at + c.len_utf8(), c.is_whitespace()));
        self.redo.clear();
        if joins {
            return;
        }
        self.undo.push(before);
        if self.undo.len() > MAX_STEPS {
            self.undo.remove(0);
        }
    }

    /// Ends the current typing run (after a cursor move), so the next typing is a change of its own.
    pub fn break_typing(&mut self) {
        self.typing = None;
    }

    pub fn undo(&mut self, current: Snapshot) -> Option<Snapshot> {
        let previous = self.undo.pop()?;
        self.redo.push(current);
        self.typing = None;
        Some(previous)
    }

    pub fn redo(&mut self, current: Snapshot) -> Option<Snapshot> {
        let next = self.redo.pop()?;
        self.undo.push(current);
        self.typing = None;
        Some(next)
    }

    /// Forgets every change.
    pub fn clear(&mut self) {
        self.undo.clear();
        self.redo.clear();
        self.typing = None;
    }

    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snap(text: &str) -> Snapshot {
        Snapshot { text: text.into(), sel: Sel::caret(text.len()) }
    }

    #[test]
    fn typing_is_one_change_until_a_pause_or_a_space() {
        let mut h = History::default();
        let t0 = Instant::now();
        h.record(snap(""), Some(('h', 0)), t0);
        h.record(snap("h"), Some(('i', 1)), t0 + Duration::from_millis(200));
        h.record(snap("hi"), Some((' ', 2)), t0 + Duration::from_millis(400));
        // After a space: a new change.
        h.record(snap("hi "), Some(('x', 3)), t0 + Duration::from_millis(500));
        // After a pause: a new change.
        h.record(snap("hi x"), Some(('y', 4)), t0 + Duration::from_secs(3));
        assert_eq!(h.undo(snap("hi xy")).unwrap().text, "hi x");
        assert_eq!(h.undo(snap("hi x")).unwrap().text, "hi ");
        assert_eq!(h.undo(snap("hi ")).unwrap().text, "");
        assert!(h.undo(snap("")).is_none());
        assert_eq!(h.redo(snap("")).unwrap().text, "hi ");
    }

    #[test]
    fn other_changes_are_changes_of_their_own() {
        let mut h = History::default();
        let t0 = Instant::now();
        h.record(snap("a"), None, t0);
        h.record(snap("b"), None, t0);
        assert_eq!(h.undo(snap("c")).unwrap().text, "b");
        // A new change drops what could be redone.
        h.record(snap("b"), None, t0);
        assert!(h.redo(snap("d")).is_none());
    }
}
