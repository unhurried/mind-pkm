//! The commands of the menus and keyboard shortcuts.

use super::dialog::{ConfirmAction, InputAction};
use super::file::{AfterUnsaved, FileAction};
use super::ui::zoom_step;
use super::{App, MapMsg, Message, blocking, settings, viewport};
use crate::mindmap::{MindMap, NodeId, Subtree};
use crate::pkm::commands::{self, EditKind};
use crate::pkm::filter::FILTER_HELP_MESSAGE;
use crate::pkm::{self, next_steps};
use chrono::Local;
use iced::Task;
use std::path::{Path, PathBuf};

const NEW_TEXT_PROMPT: &str = "New Text";
const NEW_NAME_PROMPT: &str = "New Name";
const LINK_PROMPT: &str = "Link (a file path, URL, or #ID of a node; empty removes it)";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Command {
    New,
    Open,
    Save,
    SaveAs,
    Exit,
    Undo,
    Redo,
    NewChild,
    NewSibling,
    EditText,
    Cut,
    Copy,
    Paste,
    MoveUp,
    MoveDown,
    ToggleFold,
    SetLink,
    CenterSelected,
    ZoomIn,
    ZoomOut,
    ZoomReset,
    // PKM
    PkmOpen,
    PkmEdit,
    FoldOneLevel,
    UnfoldOneLevel,
    Filter,
    Search,
    ToggleCheckmark,
    AddToToDo,
    NewPage,
    NewDirectory,
    PkmDelete,
    PkmDeleteNow,
    Update,
    AddAssets,
    FindUnlinked,
    OpenExternally,
    ToggleBuiltinEditor,
}

/// What was last cut or copied here: the subtrees, and the plain text put on
/// the system clipboard.
pub struct Clipboard {
    text: String,
    subtrees: Vec<Subtree>,
}

impl Clipboard {
    /// The subtrees, unless the system clipboard (`text`, None when it can't
    /// be read) holds something else by now.
    fn subtrees_for(&self, text: Option<&str>) -> Option<&[Subtree]> {
        match text {
            Some(t) if t.replace("\r\n", "\n").trim_end() != self.text.trim_end() => None,
            _ => Some(&self.subtrees),
        }
    }
}

/// Shown when nodes are put under a page node.
pub(super) const UNDER_PAGE_WARNING: &str = "a page node's children are replaced by its Next Steps on the next refresh";

impl App {
    pub(super) fn run_command(&mut self, cmd: Command) -> Task<Message> {
        match cmd {
            Command::New => return self.guard_unsaved(AfterUnsaved::New),
            Command::Open => return self.guard_unsaved(AfterUnsaved::Open),
            Command::Exit => return self.guard_unsaved(AfterUnsaved::Exit),
            Command::ToggleBuiltinEditor => {
                let prefs =
                    super::editor::EditorPrefs { builtin: !self.editor_prefs.builtin, ..self.editor_prefs.clone() };
                self.set_editor_prefs(prefs);
                return Task::none();
            }
            _ => {}
        }
        let Some(doc) = &mut self.doc else { return Task::none() };
        let focus = doc.focus();
        match cmd {
            Command::New | Command::Open | Command::Exit | Command::ToggleBuiltinEditor => Task::none(),
            Command::Save => self.save(None),
            Command::SaveAs => self.choose_file(FileAction::SaveAs(None)),
            Command::Undo => {
                doc.undo();
                Task::none()
            }
            Command::Redo => {
                doc.redo();
                Task::none()
            }
            Command::NewChild => {
                let (n, serial) = doc.new_child();
                self.warn_if_under_page(&[n]);
                self.open_input(NEW_TEXT_PROMPT, String::new(), InputAction::NewNode(n, serial))
            }
            Command::NewSibling => {
                let (n, serial) = doc.new_sibling();
                self.warn_if_under_page(&[n]);
                self.open_input(NEW_TEXT_PROMPT, String::new(), InputAction::NewNode(n, serial))
            }
            Command::EditText => {
                let text = doc.map().text(focus);
                self.open_input(NEW_TEXT_PROMPT, text, InputAction::EditText(focus))
            }
            Command::Cut => self.cut_or_copy(true),
            Command::Copy => self.cut_or_copy(false),
            Command::Paste => iced::clipboard::read().map(|text| MapMsg::ClipboardRead(text).into()),
            Command::MoveUp | Command::MoveDown => {
                doc.move_focus(cmd == Command::MoveUp);
                self.viewport.keep_parent_in_place = true;
                Task::none()
            }
            Command::ToggleFold => {
                doc.toggle_fold_selected();
                Task::none()
            }
            Command::SetLink => {
                let link = doc.map().link(focus).unwrap_or("").to_string();
                self.open_input(LINK_PROMPT, link, InputAction::SetLink(focus))
            }
            Command::CenterSelected => self.center_on_focus(),
            // The relayout after the command keeps the focused node in place.
            Command::ZoomIn | Command::ZoomOut | Command::ZoomReset => {
                let zoom = match cmd {
                    Command::ZoomIn => zoom_step(viewport::ZOOM_STEPS, self.viewport.zoom(), 1),
                    Command::ZoomOut => zoom_step(viewport::ZOOM_STEPS, self.viewport.zoom(), -1),
                    _ => 1.0,
                };
                self.viewport.set_zoom(zoom);
                Task::none()
            }

            // --- PKM ---
            Command::PkmOpen | Command::OpenExternally => {
                let result = commands::open_targets(doc.map(), &doc.sorted_selection());
                let files = self.report(result).unwrap_or_default();
                if cmd == Command::OpenExternally {
                    files.iter().for_each(|f| self.open_in_desktop(f));
                    return Task::none();
                }
                Task::batch(files.iter().map(|f| self.open_page(f)).collect::<Vec<_>>())
            }

            Command::PkmEdit => self.pkm_edit(),
            Command::FoldOneLevel | Command::UnfoldOneLevel => {
                doc.fold_one_level(cmd == Command::FoldOneLevel);
                Task::none()
            }
            Command::Filter => self.open_input(FILTER_HELP_MESSAGE, String::new(), InputAction::Filter),
            Command::Search => self.open_search(),
            Command::ToggleCheckmark => {
                let selection = doc.sorted_selection();
                doc.change(|map| commands::toggle_checkmark(map, &selection));
                Task::none()
            }
            Command::AddToToDo => {
                // In the order the nodes were selected.
                let selection = doc.selection().to_vec();
                match doc.try_change(|map| commands::add_to_todo(map, &selection)) {
                    Ok(created) => doc.select(created),
                    Err(e) => self.message(e),
                }
                Task::none()
            }
            Command::NewPage => self.create_documents(true),
            Command::NewDirectory => self.create_documents(false),
            Command::PkmDelete | Command::PkmDeleteNow => {
                let targets = commands::delete_targets(doc.map(), &doc.sorted_selection());
                if targets.is_empty() {
                    return Task::none();
                }
                if cmd == Command::PkmDeleteNow {
                    return self.delete_docs(&targets);
                }
                let text = commands::delete_question(doc.map(), &targets);
                self.ask(text, ConfirmAction::DeleteDocs(targets));
                Task::none()
            }
            Command::Update => self.start_update(true),
            Command::AddAssets => {
                // The page gets a link to its assets: saved first, read again after.
                let page = self.doc_path_of(focus);
                if let Err(e) = self.save_editors_of(page.as_slice()) {
                    self.message(e);
                    return Task::none();
                }
                let Some(doc) = &self.doc else { return Task::none() };
                let result = commands::add_assets(doc.map(), &doc.sorted_selection(), focus);
                if let (Ok(()), Some(page)) = (&result, &page) {
                    self.editors_reload(page);
                }
                self.report(result);
                Task::none()
            }
            Command::FindUnlinked => {
                let unlinked = doc.try_change(|map| {
                    let n = commands::find_unlinked(map)?;
                    map.set_folded(n, false);
                    Ok::<_, String>(n)
                });
                match unlinked {
                    Ok(n) => doc.select(vec![n]),
                    Err(e) => self.message(e),
                }
                Task::none()
            }
        }
    }

    /// Puts the selected subtrees (but the root, for Cut) on the clipboard,
    /// and as indented text on the system clipboard.
    fn cut_or_copy(&mut self, cut: bool) -> Task<Message> {
        let Some(doc) = &mut self.doc else { return Task::none() };
        let root = doc.map().root();
        let nodes: Vec<NodeId> = doc.sorted_selection().into_iter().filter(|&n| !cut || n != root).collect();
        if nodes.is_empty() {
            return Task::none();
        }
        let subtrees = nodes.iter().map(|&n| doc.map().extract_subtree(n)).collect();
        let text = plain_text_outline(doc.map(), &nodes);
        if cut {
            let parent = doc.map().parent(nodes[0]);
            doc.change(|map| nodes.iter().for_each(|&n| map.delete(n)));
            if let Some(p) = parent {
                doc.select(vec![p]);
            }
        }
        self.clipboard = Some(Clipboard { text: text.clone(), subtrees });
        iced::clipboard::write(text)
    }

    /// Pastes the subtrees last cut/copied here, or else the clipboard text,
    /// one node per line (indentation nests them), under the focused node.
    pub(super) fn paste(&mut self, text: Option<String>) -> Task<Message> {
        let subtrees = self.clipboard.as_ref().and_then(|c| c.subtrees_for(text.as_deref())).map(<[Subtree]>::to_vec);
        let text = text.filter(|t| t.lines().any(|l| !l.trim().is_empty()));
        let Some(doc) = &mut self.doc else { return Task::none() };
        if subtrees.is_none() && text.is_none() {
            return Task::none();
        }
        let focus = doc.focus();
        let pasted = doc.change(|map| {
            map.set_folded(focus, false);
            match subtrees {
                Some(subtrees) => subtrees.iter().map(|s| map.insert_subtree(focus, None, s)).collect(),
                None => insert_outline(map, focus, text.as_deref().unwrap_or_default()),
            }
        });
        doc.select(pasted.clone());
        self.warn_if_under_page(&pasted);
        Task::none()
    }

    /// Tells that nodes just put under a page node don't stay there: the
    /// Next Steps refresh makes a page node's children its Next Steps items.
    pub(super) fn warn_if_under_page(&mut self, nodes: &[NodeId]) {
        let Some(doc_dir) = self.doc_dir(false) else { return };
        let Some(doc) = &self.doc else { return };
        let map = &doc.map();
        let under_page = nodes
            .iter()
            .filter(|&&n| map.exists(n))
            .filter_map(|&n| map.parent(n))
            .any(|p| pkm::doc_type(map, p, &doc_dir) == Some(pkm::DocType::Page));
        if under_page {
            self.status.set(UNDER_PAGE_WARNING);
        }
    }

    /// Edit (F2): renames a page or directory node with its file, else edits the text.
    fn pkm_edit(&mut self) -> Task<Message> {
        let Some(doc) = &self.doc else { return Task::none() };
        let focus = doc.focus();
        let text = doc.map().text(focus);
        match commands::edit_kind(doc.map(), doc.selection(), focus) {
            Ok(EditKind::Text) => self.open_input(NEW_TEXT_PROMPT, text, InputAction::EditText(focus)),
            Ok(EditKind::Rename(kind)) => self.open_input(NEW_NAME_PROMPT, text, InputAction::Rename(focus, kind)),
            Err(e) => {
                self.message(e);
                Task::none()
            }
        }
    }

    /// New Page (F9) / New Directory (F10) for every selected node; opens what
    /// was created.
    fn create_documents(&mut self, pages: bool) -> Task<Message> {
        let Some(doc) = &mut self.doc else { return Task::none() };
        let template = settings::page_template();
        let today = Local::now().date_naive();
        let selection = doc.sorted_selection();
        let create =
            |map: &mut MindMap| -> Vec<Result<PathBuf, String>> {
                selection
                    .into_iter()
                    .map(|n| {
                        if pages {
                            commands::new_page(map, n, &template, today)
                        } else {
                            commands::new_directory(map, n)
                        }
                    })
                    .collect()
            };
        // An undo step only when something was created.
        let results = doc.change_recorded_if(create, |_, _, results| results.iter().any(Result::is_ok));
        let mut tasks = Vec::new();
        for result in results {
            if let Some(path) = self.report(result) {
                tasks.push(self.open_page(&path));
            }
        }
        Task::batch(tasks)
    }

    pub(super) fn open_in_desktop(&mut self, path: &Path) {
        if let Err(e) = opener::open(path) {
            self.message(format!("failed to open {}: {e}", path.display()));
        }
    }

    /// Opens what the node links: a file or directory, a URL, or another node.
    pub(super) fn follow_link(&mut self, id: NodeId) -> Task<Message> {
        let Some(doc) = &mut self.doc else { return Task::none() };
        if let Some(target) = doc.map().link_node(id) {
            doc.reveal(target);
            self.relayout();
            return self.center_on_focus();
        }
        if let Some(file) = doc.map().link_file(id) {
            return self.open_page(&file);
        } else if let Some(link) = doc.map().link(id).map(str::to_string)
            && let Err(e) = opener::open(&link)
        {
            self.message(format!("failed to open {link}: {e}"));
        }
        Task::none()
    }

    /// The document directory of the open map. When it can't be had, the
    /// reason is shown if `report`.
    pub(super) fn doc_dir(&mut self, report: bool) -> Option<PathBuf> {
        let result = pkm::load_doc_dir(self.doc.as_ref()?.map());
        if report { self.report(result) } else { result.ok() }
    }

    /// Refreshes the Next Steps children and the search index in the background.
    pub(super) fn start_update(&mut self, show_errors: bool) -> Task<Message> {
        let Some(doc) = &mut self.doc else { return Task::none() };
        doc.refresh_started();
        let Some(doc_dir) = self.doc_dir(show_errors) else { return Task::none() };
        let Some(doc) = &self.doc else { return Task::none() };
        let snapshot = next_steps::snapshot(doc.map(), &doc_dir);
        let generation = doc.generation();
        let today = Local::now().date_naive();
        self.search.updating = true;
        Task::batch([
            Task::perform(blocking(move || next_steps::compute(&snapshot, today)), move |u| {
                MapMsg::NextStepsComputed(generation, Box::new(u)).into()
            }),
            Task::perform(blocking(move || crate::search::update_index(&doc_dir)), |r| MapMsg::IndexUpdated(r).into()),
        ])
    }

    /// Applies a Next Steps refresh computed for the map, unless another map
    /// has been opened meanwhile.
    pub(super) fn next_steps_computed(&mut self, generation: u64, update: &next_steps::Update) {
        if let Some(doc) = &mut self.doc
            && doc.generation() == generation
        {
            doc.apply_refresh(|map| next_steps::apply(map, update));
        }
    }
}

/// The subtrees as indented text, one node per line (the format `insert_outline` reads).
fn plain_text_outline(map: &MindMap, nodes: &[NodeId]) -> String {
    fn walk(map: &MindMap, n: NodeId, depth: usize, out: &mut String) {
        out.push_str(&"\t".repeat(depth));
        out.push_str(&map.text(n).replace('\n', " "));
        out.push('\n');
        for c in map.children(n) {
            walk(map, c, depth + 1, out);
        }
    }
    let mut out = String::new();
    for &n in nodes {
        walk(map, n, 0, &mut out);
    }
    out
}

/// Creates a node under `parent` for every non-blank line of `text`, a line
/// indented deeper than the one before it becoming its child. Returns the new
/// direct children of `parent`.
fn insert_outline(map: &mut MindMap, parent: NodeId, text: &str) -> Vec<NodeId> {
    let mut top = Vec::new();
    // The nodes the next line may go under, with their indentation.
    let mut stack: Vec<(usize, NodeId)> = Vec::new();
    for line in text.lines().filter(|l| !l.trim().is_empty()) {
        let indent = line.chars().take_while(|c| c.is_whitespace()).count();
        while stack.last().is_some_and(|&(i, _)| i >= indent) {
            stack.pop();
        }
        let under = stack.last().map(|&(_, n)| n).unwrap_or(parent);
        let n = map.add_child(under, line.trim());
        if under == parent {
            top.push(n);
        }
        stack.push((indent, n));
    }
    top
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn outline_text_round_trips() {
        let mut map = MindMap::new("root");
        let root = map.root();
        let a = map.add_child(root, "a");
        let a1 = map.add_child(a, "a1");
        map.add_child(a1, "a11");
        let b = map.add_child(root, "b");
        let text = plain_text_outline(&map, &[a, b]);
        assert_eq!(text, "a\n\ta1\n\t\ta11\nb\n");

        let target = map.add_child(root, "target");
        let pasted = insert_outline(&mut map, target, &format!("{text}\n  \n"));
        assert_eq!(map.child_texts(target), vec!["a", "b"]);
        assert_eq!(plain_text_outline(&map, &pasted), text);
    }
}
