//! The full-text search panel (F6), docked under the map. Closing it only
//! hides it, so the next F6 shows the same query and results again.

use super::{App, Message, blocking};
use crate::mindmap::NodeId;
use crate::pkm::{self, filter::Filter};
use crate::search::{self, SearchHit, SearchScope, SearchSort};
use iced::Task;
use iced::widget::operation;
use std::fmt;
use std::path::PathBuf;

pub const UPDATING_INDEX_STATUS: &str = "Updating search index...";
pub const SEARCH_INPUT_ID: &str = "search-input";
pub const RESULTS_SCROLL_ID: &str = "search-results";
pub const ROW_HEIGHT: f32 = 22.0;

/// The orders and scopes the panel offers.
pub const SORTS: [SearchSort; 2] = [SearchSort::Modified, SearchSort::Relevance];
pub const SCOPES: [SearchScope; 3] = [SearchScope::Pages, SearchScope::Assets, SearchScope::All];

// Their labels in the panel's drop-downs.
impl fmt::Display for SearchSort {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(match self {
            SearchSort::Modified => "Last modified",
            SearchSort::Relevance => "Relevance",
        })
    }
}

impl fmt::Display for SearchScope {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(match self {
            SearchScope::Pages => "Pages",
            SearchScope::Assets => "Attachments",
            SearchScope::All => "Both",
        })
    }
}

/// What the keys act on, as far as can be told.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PanelFocus {
    /// The map: the panel is closed, or a node was clicked.
    #[default]
    Map,
    /// The keyword box, since it was focused or typed in: only the function
    /// keys reach the map.
    Input,
    /// The result list, since a result was clicked: arrow keys and Enter act on it.
    Results,
}

#[derive(Debug, Clone)]
pub enum SearchMsg {
    QueryChanged(String),
    Submit,
    Sort(SearchSort),
    Scope(SearchScope),
    Done(u64, Result<Vec<SearchHit>, String>),
    RowClicked(usize),
    RowDoubleClicked(usize),
    Move(i32),
    OpenFile,
    SelectNode,
    Close,
}

pub struct SearchPanel {
    pub open: bool,
    pub doc_dir: Option<PathBuf>,
    pub query: String,
    pub sort: SearchSort,
    pub scope: SearchScope,
    pub hits: Vec<SearchHit>,
    /// The node each hit resolves to (see `pkm::find_node_for_file`).
    pub nodes: Vec<Option<NodeId>>,
    pub selected: Option<usize>,
    pub status: String,
    pub searching: bool,
    pub updating: bool,
    pub focus: PanelFocus,
    search_id: u64,
}

impl Default for SearchPanel {
    fn default() -> Self {
        SearchPanel {
            open: false,
            doc_dir: None,
            query: String::new(),
            sort: SearchSort::Modified,
            scope: SearchScope::Pages,
            hits: Vec::new(),
            nodes: Vec::new(),
            selected: None,
            status: " ".into(),
            searching: false,
            updating: false,
            focus: PanelFocus::Map,
            search_id: 0,
        }
    }
}

impl App {
    pub(super) fn open_search(&mut self) -> Task<Message> {
        let Some(doc_dir) = self.doc_dir(true) else { return Task::none() };
        self.search.open = true;
        self.search.focus = PanelFocus::Input;
        self.search.doc_dir = Some(doc_dir.clone());
        self.viewport_size = None;
        let focus = Task::batch([operation::focus(SEARCH_INPUT_ID), operation::select_all(SEARCH_INPUT_ID)]);
        // Brings the index up to date in the background; searching meanwhile
        // works against what is on disk. Also loads the dictionary early.
        if self.search.updating {
            return focus;
        }
        self.search.updating = true;
        self.search.status = UPDATING_INDEX_STATUS.into();
        Task::batch([
            focus,
            Task::perform(
                blocking(move || {
                    search::analyzer::warm_up();
                    search::update_index(&doc_dir)
                }),
                Message::IndexUpdated,
            ),
        ])
    }

    /// A background index update (from opening the panel, or the Next Steps
    /// refresh) finished.
    pub(super) fn index_updated(&mut self, result: Result<(), String>) {
        self.search.updating = false;
        if let Err(e) = result {
            self.status.set(format!("search index update failed: {e}"));
        } else if self.search.status == UPDATING_INDEX_STATUS {
            self.search.status = "Ready".into();
        }
    }

    fn run_search(&mut self) -> Task<Message> {
        let Some(doc_dir) = self.search.doc_dir.clone() else { return Task::none() };
        self.search.searching = true;
        self.search.status = "Searching...".into();
        self.search.search_id += 1;
        let id = self.search.search_id;
        let (query, sort, scope) = (self.search.query.clone(), self.search.sort, self.search.scope);
        Task::perform(
            blocking(move || search::search(&doc_dir, &query, sort, scope, search::DEFAULT_MAX_RESULTS)),
            move |r| Message::Search(SearchMsg::Done(id, r)),
        )
    }

    pub(super) fn on_search(&mut self, msg: SearchMsg) -> Task<Message> {
        match msg {
            SearchMsg::QueryChanged(q) => {
                self.search.query = q;
                self.search.focus = PanelFocus::Input;
                Task::none()
            }
            SearchMsg::Submit => self.run_search(),
            SearchMsg::Sort(s) => {
                self.search.sort = s;
                self.rerun_search()
            }
            SearchMsg::Scope(s) => {
                self.search.scope = s;
                self.rerun_search()
            }
            SearchMsg::Done(id, result) => {
                if id != self.search.search_id {
                    return Task::none();
                }
                self.search.searching = false;
                let hits = match result {
                    Ok(h) => {
                        self.search.status =
                            if h.is_empty() { "No results".into() } else { format!("{} result(s)", h.len()) };
                        h
                    }
                    Err(e) => {
                        self.search.status = format!("search failed: {e}");
                        Vec::new()
                    }
                };
                // Resolved per search, so results reflect links changed since the panel opened.
                self.search.nodes = match (&self.doc, &self.search.doc_dir) {
                    (Some(doc), Some(dir)) => {
                        let by_file = pkm::collect_nodes_by_linked_file(&doc.map, doc.map.root());
                        hits.iter().map(|h| pkm::find_node_for_file(&by_file, &h.file, dir)).collect()
                    }
                    _ => vec![None; hits.len()],
                };
                self.search.hits = hits;
                self.search.selected = None;
                operation::scroll_to(RESULTS_SCROLL_ID, operation::AbsoluteOffset { x: 0.0, y: 0.0 })
            }
            SearchMsg::RowClicked(i) => {
                self.search.selected = Some(i);
                self.search.focus = PanelFocus::Results;
                Task::none()
            }
            SearchMsg::RowDoubleClicked(i) => {
                self.search.selected = Some(i);
                self.open_selected_hit()
            }
            SearchMsg::Move(delta) => {
                if self.search.hits.is_empty() {
                    return Task::none();
                }
                let last = self.search.hits.len() as i32 - 1;
                let i = match self.search.selected {
                    Some(i) => (i as i32 + delta).clamp(0, last),
                    None => 0,
                } as usize;
                self.search.selected = Some(i);
                operation::scroll_to(
                    RESULTS_SCROLL_ID,
                    operation::AbsoluteOffset { x: None, y: Some((i as f32 - 3.0).max(0.0) * ROW_HEIGHT) },
                )
            }
            SearchMsg::OpenFile => self.open_selected_hit(),
            SearchMsg::SelectNode => self.select_hit_node(),
            SearchMsg::Close => {
                self.search.open = false;
                self.search.focus = PanelFocus::Map;
                self.viewport_size = None;
                Task::none()
            }
        }
    }

    /// Re-runs with the new order/filter, unless there is nothing to search for yet.
    fn rerun_search(&mut self) -> Task<Message> {
        if self.search.query.trim().is_empty() { Task::none() } else { self.run_search() }
    }

    /// Opens the selected result: a page in the editor, any other file with its application.
    fn open_selected_hit(&mut self) -> Task<Message> {
        let Some(hit) = self.search.selected.and_then(|i| self.search.hits.get(i)).cloned() else {
            return Task::none();
        };
        if !hit.file.exists() {
            self.search.status = format!("file no longer exists: {}", hit.relative_path);
            return Task::none();
        }
        self.open_page(&hit.file)
    }

    /// Selects and centers the node of the selected result.
    fn select_hit_node(&mut self) -> Task<Message> {
        let Some(i) = self.search.selected else { return Task::none() };
        let target = self.search.nodes.get(i).copied().flatten();
        let Some(doc) = &mut self.doc else { return Task::none() };
        let Some(node) = target.filter(|&n| doc.map.exists(n)) else {
            self.search.status = format!("no node links to this file: {}", self.search.hits[i].relative_path);
            return Task::none();
        };
        // A filter hiding the node would leave nothing to select.
        if !doc.filter.visible(&doc.map).contains(&node) {
            doc.filter = Filter::All;
        }
        doc.reveal(node);
        self.search.status = format!("selected: {}", pkm::node_path_text(&doc.map, node));
        self.relayout();
        self.center_on_focus()
    }
}
