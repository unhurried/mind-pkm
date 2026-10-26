//! The find bar: finding and replacing in the page's source.

use super::{Editor, EditorMsg, move_source, source_sel};
use crate::markdown::edit::{self, Sel};
use crate::markdown::find;
use iced::Task;
use iced::widget::operation;
use std::ops::Range;

#[derive(Debug, Clone, PartialEq)]
pub enum FindMsg {
    Query(String),
    Replacement(String),
    MatchCase(bool),
    Next,
    Previous,
    Replace,
    ReplaceAll,
    Close,
}

#[derive(Debug, Default)]
pub struct FindBar {
    pub query: String,
    pub replacement: String,
    /// Whether the replace field is shown.
    pub replacing: bool,
    pub match_case: bool,
    /// The matches in the page's source, found for `found`.
    pub matches: Vec<Range<usize>>,
    found: Option<(u64, String, bool)>,
}

impl FindBar {
    /// Finds the matches again if the page or the query changed.
    pub fn refresh(&mut self, text: &str, revision: u64) {
        let key = (revision, self.query.clone(), self.match_case);
        if self.found.as_ref() != Some(&key) {
            self.matches = find::find_all(text, &self.query, self.match_case);
            self.found = Some(key);
        }
    }
}

impl Editor {
    /// Ctrl+F / Ctrl+H: opens the bar with the selected text and puts the focus in it.
    pub(super) fn open_find(&mut self, replacing: bool) -> Task<EditorMsg> {
        let selected = self.selection_text();
        let bar = self.find.get_or_insert_with(FindBar::default);
        if !selected.is_empty() && !selected.contains('\n') {
            bar.query = selected;
        }
        bar.replacing = replacing;
        self.refresh_find();
        Task::batch([operation::focus(self.id("find")), operation::select_all(self.id("find"))])
    }

    /// The selected source (in source mode, of the source editor).
    fn selection_text(&self) -> String {
        match &self.source {
            Some(source) => source.selection().unwrap_or_default(),
            None => self.page.selected_source(),
        }
    }

    pub(super) fn refresh_find(&mut self) {
        let text = match &self.source {
            Some(source) => source.text(),
            None => self.page.text.clone(),
        };
        if let Some(bar) = &mut self.find {
            bar.refresh(&text, self.page.revision);
        }
    }

    /// The cursor and selection, in source mode those of the source editor.
    fn current_sel(&self) -> Sel {
        match &self.source {
            Some(source) => source_sel(source, &source.text()),
            None => self.page.sel,
        }
    }

    /// The match that is selected, if one is.
    pub(super) fn current_match(&self) -> Option<Range<usize>> {
        let bar = self.find.as_ref()?;
        let sel = self.current_sel().range();
        bar.matches.iter().find(|m| **m == sel).cloned()
    }

    pub(super) fn find_message(&mut self, msg: FindMsg) -> Task<EditorMsg> {
        match msg {
            FindMsg::Query(q) => {
                if let Some(bar) = &mut self.find {
                    bar.query = q;
                }
                self.refresh_find();
                Task::none()
            }
            FindMsg::Replacement(r) => {
                if let Some(bar) = &mut self.find {
                    bar.replacement = r;
                }
                Task::none()
            }
            FindMsg::MatchCase(on) => {
                if let Some(bar) = &mut self.find {
                    bar.match_case = on;
                }
                self.refresh_find();
                Task::none()
            }
            FindMsg::Next => self.find_next(false),
            FindMsg::Previous => self.find_next(true),
            FindMsg::Replace => self.replace_current(),
            FindMsg::ReplaceAll => self.replace_all(),
            FindMsg::Close => self.close_find(),
        }
    }

    /// F3 / Shift+F3 and Enter in the bar: selects the next (previous) match.
    pub(super) fn find_next(&mut self, back: bool) -> Task<EditorMsg> {
        if self.find.as_ref().is_none_or(|b| b.query.is_empty()) {
            return self.open_find(self.find.as_ref().is_some_and(|b| b.replacing));
        }
        self.refresh_find();
        let sel = self.current_sel().range();
        let Some(bar) = &self.find else { return Task::none() };
        let at = if back { sel.start } else { sel.end };
        let Some(i) = find::next_match(&bar.matches, at, back) else { return Task::none() };
        let m = bar.matches[i].clone();
        self.select_source(Sel { anchor: m.start, head: m.end });
        Task::none()
    }

    /// Selects a range of the source, in its block (or the source editor).
    fn select_source(&mut self, sel: Sel) {
        match &mut self.source {
            Some(source) => {
                let text = source.text();
                move_source(source, &text, sel);
                self.page.sel = sel;
            }
            None => self.page.place(sel),
        }
    }

    /// Replace: the selected match is replaced, then the next one selected.
    fn replace_current(&mut self) -> Task<EditorMsg> {
        let Some(m) = self.current_match() else { return self.find_next(false) };
        let with = self.find.as_ref().map(|b| b.replacement.clone()).unwrap_or_default();
        self.leave_source_for_history();
        let new = edit::splice(&self.page.text, m.clone(), &with);
        self.page.set_text(new, Sel::caret(m.start + with.len()));
        self.refresh_source();
        self.refresh_find();
        self.find_next(false)
    }

    /// Replace All: every match replaced, as one change.
    fn replace_all(&mut self) -> Task<EditorMsg> {
        let Some(bar) = &self.find else { return Task::none() };
        let (query, with, case) = (bar.query.clone(), bar.replacement.clone(), bar.match_case);
        self.leave_source_for_history();
        let (new, n) = find::replace_all(&self.page.text, &query, &with, case);
        if n > 0 {
            let head = self.page.sel.head.min(new.len());
            self.page.set_text(new, Sel::caret(head));
            self.page.deactivate();
            self.refresh_source();
        }
        self.status = format!("{n} replaced");
        self.refresh_find();
        Task::none()
    }

    /// Esc: the bar closes, the focus goes back to the page with the match selected.
    pub(super) fn close_find(&mut self) -> Task<EditorMsg> {
        self.find = None;
        if self.source.is_none() && self.page.active.is_none() {
            return Task::none();
        }
        self.focus()
    }

    /// What the highlighter marks: the query and whether case matters.
    pub(super) fn find_query(&self) -> Option<(String, bool)> {
        self.find.as_ref().filter(|b| !b.query.is_empty()).map(|b| (b.query.clone(), b.match_case))
    }

    /// "3 of 5" (or how many there are when none is selected).
    pub(super) fn find_status(&self) -> String {
        let Some(bar) = &self.find else { return String::new() };
        if bar.query.is_empty() {
            return String::new();
        }
        let n = bar.matches.len();
        match self.current_match().and_then(|m| bar.matches.iter().position(|x| *x == m)) {
            Some(i) => format!("{} of {n}", i + 1),
            None if n == 0 => "No results".to_string(),
            None => format!("{n} found"),
        }
    }
}
