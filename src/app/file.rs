//! The document lifecycle: new, open, save, and the unsaved changes guard.

use super::dialog::Dialog;
use super::doc::Doc;
use super::search_panel::SearchPanel;
use super::settings::{self, MapView};
use super::{App, Message};
use crate::mindmap::MindMap;
use crate::pkm;
use iced::{Task, Vector};
use std::path::{Path, PathBuf};

/// Root text and document directory of a new map.
const NEW_MAP_ROOT_TEXT: &str = "Map";
const NEW_MAP_DOC_DIR: &str = "C:\\path\\to\\documents";

/// What a file dialog was opened for.
#[derive(Debug, Clone, Copy)]
pub enum FileAction {
    Open,
    SaveAs,
    SaveAsThenExit,
}

/// What to do once the user dealt with unsaved changes.
#[derive(Debug, Clone, Copy)]
pub enum AfterUnsaved {
    New,
    Open,
    Exit,
}

impl App {
    pub(super) fn open_path(&mut self, path: PathBuf) -> Task<Message> {
        match MindMap::load(&path) {
            Ok(map) => {
                settings::set_last_file(&path);
                let view = settings::map_view(&path);
                self.set_doc(map, view)
            }
            Err(e) => {
                self.message(format!("failed to open the map: {e}"));
                Task::none()
            }
        }
    }

    /// Shows `map` in place of the open one (whose view is remembered first):
    /// as `view` says, else at the current zoom with the root centered.
    pub(super) fn set_doc(&mut self, map: MindMap, view: Option<MapView>) -> Task<Message> {
        self.remember_view();
        self.next_generation += 1;
        let mut doc = Doc::new(map, self.next_generation);
        if let Some(view) = &view {
            self.set_zoom(super::viewport::zoom_step(view.zoom, 0));
            // Nodes gone since are left out; hidden ones are handed to a shown ancestor by the relayout.
            doc.select(view.selection.iter().filter_map(|id| doc.map.find_by_freeplane_id(id)).collect());
        }
        self.doc = Some(doc);
        self.search = SearchPanel::default();
        self.relayout();
        match view {
            Some(view) => self.scroll_to(Vector::new(view.scroll.0.max(0.0), view.scroll.1.max(0.0))),
            None => self.center_on_focus(),
        }
    }

    /// How the open map is shown, if it has a file to be remembered by.
    pub(super) fn current_view(&self) -> Option<(PathBuf, MapView)> {
        let doc = self.doc.as_ref()?;
        let path = doc.map.path()?.to_path_buf();
        // The focused node last.
        let nodes = doc.selection.iter().copied().filter(|&n| n != doc.focus).chain([doc.focus]);
        let selection = nodes.map(|n| doc.map.freeplane_id(n).to_string()).collect();
        let scroll = (self.scroll_offset.x, self.scroll_offset.y);
        Some((path, MapView { zoom: self.zoom, scroll, selection }))
    }

    /// Remembers how the open map is shown, for when it is opened again.
    fn remember_view(&self) {
        if let Some((path, view)) = self.current_view() {
            settings::set_map_view(&path, view);
        }
    }

    /// Quits the application, remembering how the map and the window are shown.
    fn exit(&self) -> Task<Message> {
        self.remember_view();
        settings::set_window(&self.window);
        iced::exit()
    }

    /// Saves to the map's file (or asks for one first), then exits if `then_exit`.
    pub(super) fn save(&mut self, then_exit: bool) -> Task<Message> {
        let Some(doc) = &self.doc else { return Task::none() };
        let Some(path) = doc.map.path().map(Path::to_path_buf) else {
            return self.choose_file(if then_exit { FileAction::SaveAsThenExit } else { FileAction::SaveAs });
        };
        if self.save_to(&path) && then_exit { self.exit() } else { Task::none() }
    }

    /// Saves the map to `path`; a failure is shown to the user.
    fn save_to(&mut self, path: &Path) -> bool {
        let Some(doc) = &mut self.doc else { return false };
        match doc.map.save(path) {
            Ok(()) => {
                doc.dirty = false;
                settings::set_last_file(path);
                self.remember_view();
                self.status = format!("saved {}", path.display());
                true
            }
            Err(e) => {
                self.message(format!("failed to save the map: {e}"));
                false
            }
        }
    }

    pub(super) fn choose_file(&self, action: FileAction) -> Task<Message> {
        let dir = self.doc.as_ref().and_then(|d| d.map.path()).and_then(|p| p.parent()).map(|p| p.to_path_buf());
        Task::perform(
            async move {
                let mut dialog = rfd::AsyncFileDialog::new().add_filter("Mind map", &["mm"]);
                if let Some(dir) = dir {
                    dialog = dialog.set_directory(dir);
                }
                let handle = match action {
                    FileAction::Open => dialog.pick_file().await,
                    FileAction::SaveAs | FileAction::SaveAsThenExit => dialog.set_file_name("map.mm").save_file().await,
                };
                handle.map(|h| h.path().to_path_buf())
            },
            move |path| Message::FileChosen(action, path),
        )
    }

    /// The file dialog closed, with the chosen file unless it was cancelled.
    pub(super) fn file_chosen(&mut self, action: FileAction, path: Option<PathBuf>) -> Task<Message> {
        let Some(path) = path else { return Task::none() };
        match action {
            FileAction::Open => self.open_path(path),
            FileAction::SaveAs | FileAction::SaveAsThenExit => {
                let path = if path.extension().is_none() { path.with_extension("mm") } else { path };
                let exit = matches!(action, FileAction::SaveAsThenExit);
                if self.save_to(&path) && exit { self.exit() } else { Task::none() }
            }
        }
    }

    /// Runs `after` now, or once the user dealt with unsaved changes.
    pub(super) fn guard_unsaved(&mut self, after: AfterUnsaved) -> Task<Message> {
        if self.doc.as_ref().is_some_and(|d| d.dirty) {
            self.dialogs.push(Dialog::Unsaved(after));
            return Task::none();
        }
        self.after_unsaved(after)
    }

    pub(super) fn after_unsaved(&mut self, after: AfterUnsaved) -> Task<Message> {
        match after {
            AfterUnsaved::New => self.set_doc(pkm::new_map(NEW_MAP_ROOT_TEXT, NEW_MAP_DOC_DIR), None),
            AfterUnsaved::Open => self.choose_file(FileAction::Open),
            AfterUnsaved::Exit => self.exit(),
        }
    }
}
