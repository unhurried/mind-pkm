//! Modal dialogs: messages, confirmations, text input and the unsaved changes
//! question. They queue up; the first one is shown.

use super::file::AfterUnsaved;
use super::{App, DIALOG_INPUT_ID, Message};
use crate::mindmap::NodeId;
use crate::pkm::commands;
use crate::pkm::{DocType, filter::Filter};
use chrono::Local;
use iced::Task;
use iced::widget::operation;

#[derive(Debug, Clone)]
pub enum Dialog {
    Message(String),
    Confirm {
        text: String,
        action: ConfirmAction,
    },
    Input {
        prompt: String,
        value: String,
        action: InputAction,
    },
    Unsaved(AfterUnsaved),
    /// Ending the application with pages that have unsaved changes: their names.
    UnsavedPages(Vec<String>),
}

/// What a confirmation dialog does on Yes.
#[derive(Debug, Clone)]
pub enum ConfirmAction {
    DeleteDocs(Vec<(NodeId, DocType)>),
}

/// What an input dialog's value is for.
#[derive(Debug, Clone)]
pub enum InputAction {
    EditText(NodeId),
    /// A node just created, with the serial number of the checkpoint before
    /// it: cancelling or leaving it empty removes it again.
    NewNode(NodeId, u64),
    Rename(NodeId, DocType),
    Filter,
    SetLink(NodeId),
}

impl App {
    /// Shows a message to the user.
    pub(super) fn message(&mut self, text: impl Into<String>) {
        self.dialogs.push_back(Dialog::Message(text.into()));
    }

    /// Shows the error of a failed command, if it failed.
    pub(super) fn report<T>(&mut self, result: Result<T, String>) -> Option<T> {
        result.map_err(|e| self.message(e)).ok()
    }

    pub(super) fn ask(&mut self, text: String, action: ConfirmAction) {
        self.dialogs.push_back(Dialog::Confirm { text, action });
    }

    pub(super) fn open_input(&mut self, prompt: &str, value: String, action: InputAction) -> Task<Message> {
        self.dialogs.push_back(Dialog::Input { prompt: prompt.into(), value, action });
        if self.dialogs.len() == 1 { focus_dialog_input() } else { Task::none() }
    }

    /// Closes the unsaved changes question, if it is the shown dialog.
    fn take_unsaved_dialog(&mut self) -> Option<AfterUnsaved> {
        let &Dialog::Unsaved(after) = self.dialogs.front()? else { return None };
        self.dialogs.pop_front();
        Some(after)
    }

    /// Focus for the next dialog in the queue, if it has an input.
    fn after_dialog(&self) -> Task<Message> {
        match self.dialogs.front() {
            Some(Dialog::Input { .. }) => focus_dialog_input(),
            _ => Task::none(),
        }
    }

    pub(super) fn dialog_input(&mut self, value: String) {
        if let Some(Dialog::Input { value: v, .. }) = self.dialogs.front_mut() {
            *v = value;
        }
    }

    pub(super) fn dialog_ok(&mut self) -> Task<Message> {
        // The unsaved changes questions are answered only by their own buttons.
        if matches!(self.dialogs.front(), Some(Dialog::Unsaved(_) | Dialog::UnsavedPages(_))) {
            return Task::none();
        }
        let task = match self.dialogs.pop_front() {
            None | Some(Dialog::Message(_) | Dialog::Unsaved(_) | Dialog::UnsavedPages(_)) => Task::none(),
            Some(Dialog::Confirm { action: ConfirmAction::DeleteDocs(targets), .. }) => self.delete_docs(&targets),
            Some(Dialog::Input { value, action, .. }) => self.input_done(Some(value), action),
        };
        Task::batch([task, self.after_dialog()])
    }

    /// Cancel, No, or Esc: closes the shown dialog (the unsaved changes
    /// question keeping the changes).
    pub(super) fn dialog_cancel(&mut self) -> Task<Message> {
        let task = match self.dialogs.pop_front() {
            Some(Dialog::Input { action, .. }) => self.input_done(None, action),
            _ => Task::none(),
        };
        Task::batch([task, self.after_dialog()])
    }

    /// Unsaved changes: save them (choosing a file first for a new map), then
    /// go on unless saving failed or was cancelled.
    pub(super) fn dialog_save(&mut self) -> Task<Message> {
        if matches!(self.dialogs.front(), Some(Dialog::UnsavedPages(_))) {
            self.dialogs.pop_front();
            let errors = self.save_all_pages();
            if errors.is_empty() {
                return self.exit();
            }
            errors.into_iter().for_each(|e| self.message(e));
            return Task::none();
        }
        let Some(after) = self.take_unsaved_dialog() else { return Task::none() };
        self.save(Some(after))
    }

    /// Unsaved changes: discard them and go on. The map stays marked as
    /// unsaved: going on may still be cancelled (the unsaved pages question),
    /// and a new or opened map replaces it anyway.
    pub(super) fn dialog_discard(&mut self) -> Task<Message> {
        if matches!(self.dialogs.front(), Some(Dialog::UnsavedPages(_))) {
            self.dialogs.pop_front();
            return self.exit();
        }
        let Some(after) = self.take_unsaved_dialog() else { return Task::none() };
        self.after_unsaved(after)
    }

    /// Deletes the pages and directories confirmed for deletion (F11), moving
    /// their files to the Recycle Bin.
    fn delete_docs(&mut self, targets: &[(NodeId, DocType)]) -> Task<Message> {
        // Open pages are saved first (to the Recycle Bin with their changes), their editors close.
        let paths: Vec<_> = targets.iter().filter_map(|&(n, _)| self.doc_path_of(n)).collect();
        if let Err(e) = self.save_editors_of(&paths) {
            self.message(e);
            return Task::none();
        }
        let Some(doc) = &mut self.doc else { return Task::none() };
        doc.checkpoint();
        let result = commands::delete_execute(&mut doc.map, targets, &mut |files| {
            trash::delete_all(files).map_err(|e| e.to_string())
        });
        self.report(result);
        self.close_deleted_editors(&paths)
    }

    /// Acts on an input dialog's value; None when it was cancelled.
    fn input_done(&mut self, value: Option<String>, action: InputAction) -> Task<Message> {
        if let InputAction::Rename(id, kind) = action {
            if let Some(v) = value.filter(|v| !v.is_empty()) {
                self.rename_doc(id, kind, &v);
            }
            return Task::none();
        }
        let Some(doc) = &mut self.doc else { return Task::none() };
        let non_empty = value.clone().filter(|v| !v.is_empty());
        match action {
            InputAction::EditText(id) => {
                if let Some(v) = non_empty
                    && doc.map.exists(id)
                    && doc.map.text(id) != v
                {
                    doc.checkpoint();
                    doc.map.set_text(id, &v);
                }
            }
            InputAction::NewNode(id, serial) => match non_empty {
                Some(v) if doc.map.exists(id) => doc.map.set_text(id, &v),
                _ => {
                    // The node goes; changes made meanwhile (a refresh) stay.
                    let parent = doc.map.exists(id).then(|| doc.map.parent(id)).flatten();
                    doc.cancel_new_node(id, serial);
                    if let Some(p) = parent.filter(|&p| doc.map.exists(p)) {
                        doc.select(vec![p]);
                    }
                }
            },
            InputAction::Rename(..) => {}
            InputAction::Filter => {
                // Cancelled: the current filter stays.
                if let Some(v) = value {
                    doc.filter = Filter::parse(&v, Local::now().date_naive());
                }
            }
            InputAction::SetLink(id) => {
                if let Some(v) = value
                    && doc.map.exists(id)
                {
                    doc.checkpoint();
                    let v = v.trim();
                    doc.map.set_link(id, if v.is_empty() { None } else { Some(v) });
                }
            }
        }
        Task::none()
    }
}

impl App {
    /// Edit (F2) on a page or directory node: renames it with its files; open
    /// pages are saved first and follow. The files stay renamed, so the map's
    /// undo history ends here.
    fn rename_doc(&mut self, id: NodeId, kind: DocType, name: &str) {
        if !self.doc.as_ref().is_some_and(|d| d.map.exists(id)) {
            return;
        }
        let old = self.doc_path_of(id);
        if let Err(e) = self.save_editors_of(old.as_slice()) {
            self.message(e);
            return;
        }
        let Some(doc) = &mut self.doc else { return };
        let result = doc.try_change(|map| commands::rename(map, id, kind, name));
        let renamed = result.is_ok();
        if renamed {
            doc.forget_undo();
        }
        self.report(result);
        if renamed && let (Some(old), Some(new)) = (old, self.doc_path_of(id)) {
            self.editors_renamed(&old, &new);
        }
    }
}

fn focus_dialog_input() -> Task<Message> {
    Task::batch([operation::focus(DIALOG_INPUT_ID), operation::select_all(DIALOG_INPUT_ID)])
}
