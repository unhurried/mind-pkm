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
    Confirm { text: String, action: ConfirmAction },
    Input { prompt: String, value: String, action: InputAction },
    Unsaved(AfterUnsaved),
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
        self.dialogs.push(Dialog::Message(text.into()));
    }

    /// Shows the error of a failed command, if it failed.
    pub(super) fn report<T>(&mut self, result: Result<T, String>) -> Option<T> {
        result.map_err(|e| self.message(e)).ok()
    }

    pub(super) fn ask(&mut self, text: String, action: ConfirmAction) {
        self.dialogs.push(Dialog::Confirm { text, action });
    }

    pub(super) fn open_input(&mut self, prompt: &str, value: String, action: InputAction) -> Task<Message> {
        self.dialogs.push(Dialog::Input { prompt: prompt.into(), value, action });
        if self.dialogs.len() == 1 { focus_dialog_input() } else { Task::none() }
    }

    /// The shown dialog asks about unsaved changes (answered only by its own buttons).
    pub(super) fn unsaved_dialog_shown(&self) -> bool {
        matches!(self.dialogs.first(), Some(Dialog::Unsaved(_)))
    }

    fn take_dialog(&mut self) -> Option<Dialog> {
        if self.dialogs.is_empty() { None } else { Some(self.dialogs.remove(0)) }
    }

    /// Closes the unsaved changes question, if it is the shown dialog.
    fn take_unsaved_dialog(&mut self) -> Option<AfterUnsaved> {
        let &Dialog::Unsaved(after) = self.dialogs.first()? else { return None };
        self.dialogs.remove(0);
        Some(after)
    }

    /// Focus for the next dialog in the queue, if it has an input.
    fn after_dialog(&self) -> Task<Message> {
        match self.dialogs.first() {
            Some(Dialog::Input { .. }) => focus_dialog_input(),
            _ => Task::none(),
        }
    }

    pub(super) fn dialog_input(&mut self, value: String) {
        if let Some(Dialog::Input { value: v, .. }) = self.dialogs.first_mut() {
            *v = value;
        }
    }

    pub(super) fn dialog_ok(&mut self) -> Task<Message> {
        if self.unsaved_dialog_shown() {
            return Task::none();
        }
        let Some(dialog) = self.take_dialog() else { return Task::none() };
        let task = match dialog {
            Dialog::Message(_) | Dialog::Unsaved(_) => Task::none(),
            Dialog::Confirm { action: ConfirmAction::DeleteDocs(targets), .. } => {
                self.delete_docs(&targets);
                Task::none()
            }
            Dialog::Input { value, action, .. } => self.input_done(Some(value), action),
        };
        Task::batch([task, self.after_dialog()])
    }

    pub(super) fn dialog_cancel(&mut self) -> Task<Message> {
        let Some(dialog) = self.take_dialog() else { return Task::none() };
        let task = match dialog {
            Dialog::Input { action, .. } => self.input_done(None, action),
            _ => Task::none(),
        };
        Task::batch([task, self.after_dialog()])
    }

    /// Unsaved changes: save them, then go on unless saving failed.
    pub(super) fn dialog_save(&mut self) -> Task<Message> {
        let Some(after) = self.take_unsaved_dialog() else { return Task::none() };
        if let AfterUnsaved::Exit = after {
            return self.save(true);
        }
        let saved = self.save(false);
        if self.doc.as_ref().is_some_and(|d| d.dirty) {
            // Not saved (yet): a failure was reported, or Save As asks for a file.
            saved
        } else {
            Task::batch([saved, self.after_unsaved(after)])
        }
    }

    /// Unsaved changes: discard them and go on.
    pub(super) fn dialog_discard(&mut self) -> Task<Message> {
        let Some(after) = self.take_unsaved_dialog() else { return Task::none() };
        if let Some(doc) = &mut self.doc {
            doc.dirty = false;
        }
        self.after_unsaved(after)
    }

    /// Closes the shown dialog with Escape: as Cancel would, except that the
    /// unsaved changes question is just dismissed.
    pub(super) fn dismiss_dialog(&mut self) -> Task<Message> {
        if self.unsaved_dialog_shown() {
            self.dialogs.remove(0);
            return Task::none();
        }
        self.dialog_cancel()
    }

    /// Deletes the pages and directories confirmed for deletion (F11), moving
    /// their files to the Recycle Bin.
    fn delete_docs(&mut self, targets: &[(NodeId, DocType)]) {
        let Some(doc) = &mut self.doc else { return };
        doc.checkpoint();
        let result = commands::delete_execute(&mut doc.map, targets, &mut |files| {
            trash::delete_all(files).map_err(|e| e.to_string())
        });
        self.report(result);
    }

    /// Acts on an input dialog's value; None when it was cancelled.
    fn input_done(&mut self, value: Option<String>, action: InputAction) -> Task<Message> {
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
            InputAction::Rename(id, kind) => {
                if let Some(v) = non_empty
                    && doc.map.exists(id)
                {
                    let result = doc.try_change(|map| commands::rename(map, id, kind, &v));
                    self.report(result);
                }
            }
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

fn focus_dialog_input() -> Task<Message> {
    Task::batch([operation::focus(DIALOG_INPUT_ID), operation::select_all(DIALOG_INPUT_ID)])
}
