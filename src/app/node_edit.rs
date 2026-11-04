//! Editing a node's text in the node itself on the map: the text of a new
//! node and Edit Node Text. The node being edited shows a text box in place of
//! its text (`view::node_view`) and the layout is made with the text typed so
//! far (`Doc::relayout`'s `Draft`).

use super::doc::Draft;
use super::{App, MapMsg, Message};
use crate::mindmap::NodeId;
use iced::Task;
use iced::keyboard::{Key, key::Named};
use iced::widget::operation;
use iced::widget::text_editor::{self, Action, Binding, Content, Edit};
use std::sync::Arc;

/// The id of the text box of the node being edited.
pub const NODE_EDIT_ID: &str = "node-edit";

/// What the text being edited is for.
#[derive(Debug, Clone, Copy)]
pub enum EditFor {
    /// A node's text as it is.
    Text,
    /// A node just created, with the serial number of the checkpoint before
    /// it: cancelling or leaving it empty removes it again.
    NewNode(u64),
}

/// The node being edited and the text box that holds its text.
pub struct NodeEdit {
    id: NodeId,
    content: Content,
    kind: EditFor,
}

impl NodeEdit {
    fn new(id: NodeId, text: &str, kind: EditFor) -> Self {
        let mut content = Content::with_text(&one_line(text));
        content.perform(Action::SelectAll);
        NodeEdit { id, content, kind }
    }

    pub fn id(&self) -> NodeId {
        self.id
    }

    pub fn content(&self) -> &Content {
        &self.content
    }

    /// The text typed so far.
    pub fn text(&self) -> String {
        one_line(&self.content.text())
    }
}

/// The text on one line: line breaks become spaces.
fn one_line(text: &str) -> String {
    text.replace("\r\n", " ").replace(['\n', '\r'], " ")
}

/// The text box's key bindings: Enter confirms; Tab inserts nothing (the
/// map's keys don't act meanwhile, so it is ignored); the rest are the text
/// editor's own.
pub fn key_binding(press: text_editor::KeyPress) -> Option<Binding<MapMsg>> {
    match &press.key {
        Key::Named(Named::Enter) => Some(Binding::Custom(MapMsg::NodeEditConfirm)),
        Key::Named(Named::Tab) => None,
        _ => Binding::from_key_press(press),
    }
}

impl App {
    /// Starts editing the text of `id` in the node (none is edited already:
    /// the map's keys don't act meanwhile, and any other action confirms it).
    pub(super) fn start_node_edit(&mut self, id: NodeId, text: &str, kind: EditFor) -> Task<Message> {
        self.node_edit = Some(NodeEdit::new(id, text, kind));
        operation::focus(NODE_EDIT_ID)
    }

    /// The text being edited, for the layout.
    pub(super) fn draft(&self) -> Option<Draft> {
        self.node_edit.as_ref().map(|e| Draft { id: e.id, text: e.text() })
    }

    /// An action of the text box. Line breaks can't be entered; pasted ones
    /// become spaces.
    pub(super) fn node_edit_action(&mut self, action: Action) {
        let Some(edit) = &mut self.node_edit else { return };
        let action = match action {
            Action::Edit(Edit::Enter | Edit::Indent | Edit::Unindent) => return,
            Action::Edit(Edit::Insert('\n' | '\r' | '\t')) => return,
            Action::Edit(Edit::Paste(text)) => Action::Edit(Edit::Paste(Arc::new(one_line(&text)))),
            other => other,
        };
        edit.content.perform(action);
    }

    /// A key while a node is edited: Enter confirms (also when the text box
    /// does not have the keyboard); nothing else acts on the map.
    pub(super) fn node_edit_key(&mut self, key: &Key) {
        if *key == Key::Named(Named::Enter) {
            self.confirm_node_edit();
        }
    }

    /// Confirms the text being edited, if a node is: a changed text is one
    /// undo step; a new node's empty text removes the node again.
    pub(super) fn confirm_node_edit(&mut self) {
        if let Some(edit) = self.node_edit.take() {
            let text = edit.text();
            self.finish_node_edit(edit.id, edit.kind, Some(text));
        }
    }

    /// Cancels the edit: a new node goes, an existing node stays as it was.
    pub(super) fn cancel_node_edit(&mut self) {
        if let Some(edit) = self.node_edit.take() {
            self.finish_node_edit(edit.id, edit.kind, None);
        }
    }

    fn finish_node_edit(&mut self, id: NodeId, kind: EditFor, text: Option<String>) {
        let Some(doc) = &mut self.doc else { return };
        let non_empty = text.filter(|t| !t.is_empty());
        match kind {
            EditFor::Text => {
                if let Some(v) = non_empty
                    && doc.map().exists(id)
                    && doc.map().text(id) != v
                {
                    doc.change(|map| map.set_text(id, &v));
                }
            }
            EditFor::NewNode(serial) => match non_empty {
                Some(v) if doc.map().exists(id) => doc.set_new_node_text(id, &v),
                // The node goes, and the nodes selected before it come back;
                // changes made meanwhile (a refresh) stay.
                _ => doc.cancel_new_node(id, serial),
            },
        }
    }
}

impl MapMsg {
    /// Whether the message is the user acting elsewhere than in the text box
    /// being edited, which confirms the text first (`edited` is the node).
    pub(super) fn confirms_node_edit(&self, edited: NodeId) -> bool {
        match self {
            MapMsg::NodeClicked(id) => *id != edited,
            MapMsg::NodeDoubleClicked(_)
            | MapMsg::NodeRightClicked(_)
            | MapMsg::MapRightClicked
            | MapMsg::LinkClicked(_)
            | MapMsg::FoldClicked(_)
            | MapMsg::PanStart
            | MapMsg::Menu(Some(_))
            | MapMsg::Command(_)
            | MapMsg::CloseRequested => true,
            _ => false,
        }
    }
}
