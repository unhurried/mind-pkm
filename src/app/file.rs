//! The document lifecycle: new, open, save, and the unsaved changes guard.

use super::dialog::Dialog;
use super::doc::Doc;
use super::search_panel::SearchPanel;
use super::{App, Message, settings};
use crate::mindmap::MindMap;
use crate::pkm;
use iced::Task;
use std::path::{Path, PathBuf};
use std::time::Instant;

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
                self.set_doc(map)
            }
            Err(e) => {
                self.message(format!("failed to open the map: {e}"));
                Task::none()
            }
        }
    }

    pub(super) fn set_doc(&mut self, map: MindMap) -> Task<Message> {
        self.next_generation += 1;
        self.doc = Some(Doc::new(map, self.next_generation));
        self.search = SearchPanel::default();
        self.last_update = Instant::now();
        self.changed_since_update = false;
        self.relayout();
        self.center_on_focus()
    }

    /// Saves to the map's file (or asks for one first), then exits if `then_exit`.
    pub(super) fn save(&mut self, then_exit: bool) -> Task<Message> {
        let Some(doc) = &self.doc else { return Task::none() };
        let Some(path) = doc.map.path().map(Path::to_path_buf) else {
            return self.choose_file(if then_exit { FileAction::SaveAsThenExit } else { FileAction::SaveAs });
        };
        if self.save_to(&path) && then_exit { iced::exit() } else { Task::none() }
    }

    /// Saves the map to `path`; a failure is shown to the user.
    fn save_to(&mut self, path: &Path) -> bool {
        let Some(doc) = &mut self.doc else { return false };
        match doc.map.save(path) {
            Ok(()) => {
                doc.dirty = false;
                settings::set_last_file(path);
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
                if self.save_to(&path) && exit { iced::exit() } else { Task::none() }
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
            AfterUnsaved::New => self.set_doc(pkm::new_map(NEW_MAP_ROOT_TEXT, NEW_MAP_DOC_DIR)),
            AfterUnsaved::Open => self.choose_file(FileAction::Open),
            AfterUnsaved::Exit => iced::exit(),
        }
    }
}
