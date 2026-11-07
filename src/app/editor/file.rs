//! The page's file: saving it, closing the window with unsaved changes,
//! following the file when it changes on disk or a PKM command renames it,
//! and the answers to the window's dialogs.

use super::{DialogAnswer, Editor, EditorDialog, EditorMsg, Effect};
use crate::app::file::file_name;
use iced::Task;
use std::path::PathBuf;

impl Editor {
    pub fn save(&mut self) -> Task<EditorMsg> {
        match self.page.save() {
            Ok(()) => self.status.set(format!("saved {}", file_name(self.page.path()))),
            Err(e) => self.dialogs.push_back(EditorDialog::Message(e)),
        }
        Task::none()
    }

    pub(super) fn close_requested(&mut self, effects: &mut Vec<Effect>) -> Task<EditorMsg> {
        if self.page.is_dirty() {
            if !self.dialogs.contains(&EditorDialog::Unsaved) {
                self.dialogs.push_back(EditorDialog::Unsaved);
            }
        } else {
            effects.push(Effect::Close);
        }
        Task::none()
    }

    pub(super) fn dialog_answered(&mut self, answer: DialogAnswer, effects: &mut Vec<Effect>) -> Task<EditorMsg> {
        let Some(dialog) = self.dialogs.front().cloned() else { return Task::none() };
        match (dialog, answer) {
            (EditorDialog::Unsaved, DialogAnswer::Save) => {
                self.dialogs.pop_front();
                match self.page.save() {
                    Ok(()) => effects.push(Effect::Close),
                    Err(e) => self.dialogs.push_back(EditorDialog::Message(e)),
                }
            }
            (EditorDialog::Unsaved, DialogAnswer::Discard) => {
                self.dialogs.pop_front();
                effects.push(Effect::Close);
            }
            (EditorDialog::ChangedOnDisk, DialogAnswer::Discard) => {
                self.dialogs.pop_front();
                self.reload();
            }
            (EditorDialog::RenameAsset { path, name }, DialogAnswer::Ok) => {
                self.dialogs.pop_front();
                effects.push(Effect::RenameAsset(path, name));
            }
            (EditorDialog::DeleteAsset(path), DialogAnswer::Ok) => {
                self.dialogs.pop_front();
                effects.push(Effect::DeleteAsset(path));
            }
            (EditorDialog::ChangedOnDisk, _) => {
                self.dialogs.pop_front();
                self.page.ignore_disk_change();
            }
            _ => {
                self.dialogs.pop_front();
            }
        }
        Task::none()
    }

    /// Reads the file again, losing unsaved changes (they can be undone).
    pub fn reload(&mut self) {
        if let Err(e) = self.page.reload() {
            self.dialogs.push_back(EditorDialog::Message(e));
        }
        self.refresh_source();
    }

    /// Follows the file to `path`, where a PKM command renamed it (or the
    /// directory it is in), reading it again.
    pub fn moved_to(&mut self, path: PathBuf) {
        if let Err(e) = self.page.moved_to(path) {
            self.dialogs.push_back(EditorDialog::Message(e));
        }
        self.refresh_source();
    }

    /// A check on the file (on focus and every tick): reloads it when it
    /// changed on disk, or asks when there are unsaved changes.
    pub fn check_disk(&mut self) {
        if !self.page.changed_on_disk() || self.dialogs.contains(&EditorDialog::ChangedOnDisk) {
            return;
        }
        if self.page.is_dirty() {
            self.dialogs.push_back(EditorDialog::ChangedOnDisk);
        } else if let Err(e) = self.page.reload() {
            self.dialogs.push_back(EditorDialog::Message(e));
        } else {
            self.refresh_source();
        }
    }
}
