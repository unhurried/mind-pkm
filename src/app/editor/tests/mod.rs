//! Editor windows driven by their messages, without windows: opening pages
//! from the map, typing, saving, closing, and leaving the application. The
//! tests are grouped by topic in the submodules; the helpers several of them
//! use are here.

use super::keys::Bind;
use super::{
    AssetsMsg, DialogAnswer, EditorCommand, EditorDialog, EditorMsg, Effect, FindMsg, LiveMsg, TableMsg,
    opens_in_editor,
};
use crate::app::command::Command;
use crate::app::dialog::Dialog;
use crate::app::test_support::{key_event, send, typing_event};
use crate::app::{AfterUnsaved, App, Message};
use crate::mindmap::NodeId;
use crate::pkm::testutil::Fixture;
use iced::keyboard::key::{NativeCode, Physical};
use iced::keyboard::{self, Key, Location, Modifiers, key::Named};
use iced::widget::text_editor::{Action, Edit};
use iced::window;
use std::path::PathBuf;

mod assets;
mod clipboard;
mod editing;
mod find;
mod live;
mod pkm;
mod tables;
mod windows;

/// An application showing a map with a page node `Plan` whose page holds `text`.
fn app_with_page(text: &str) -> (App, Fixture, NodeId, PathBuf) {
    let mut f = Fixture::new();
    let root = f.map.root();
    let page = f.write_page("Plan", text);
    let node = f.add_page_node(root, "Plan");
    let mut app = App::new();
    let _ = app.set_doc(f.map.clone(), None);
    app.doc.as_mut().unwrap().select(vec![node]);
    (app, f, node, page)
}

/// Opens the selected page with F1; its window's id.
fn open_with_f1(app: &mut App) -> window::Id {
    send(app, Message::Key(key_event(Key::Named(Named::F1), Modifiers::empty())));
    *app.editors.keys().last().expect("an editor window")
}

fn editor(app: &mut App, id: window::Id, msg: EditorMsg) {
    send(app, Message::Editor(id, msg));
}

fn typing(app: &mut App, id: window::Id, s: &str) {
    for c in s.chars() {
        editor(app, id, EditorMsg::Action(Action::Edit(Edit::Insert(c))));
    }
}

fn page_text(app: &App, id: window::Id) -> String {
    app.editors[&id].page.text.clone()
}

/// Puts the cursor at the end of `after` in the page.
fn click_after(app: &mut App, id: window::Id, after: &str) {
    let at = page_text(app, id).find(after).unwrap() + after.len();
    editor(app, id, EditorMsg::PressedAt(at));
}

fn press(app: &mut App, id: window::Id, key: &str, modifiers: Modifiers) {
    send(app, Message::KeyIn(id, key_event(Key::Character(key.into()), modifiers)));
}

/// Selects `what` in the page (activating its block).
fn select(app: &mut App, id: window::Id, what: &str) {
    let text = page_text(app, id);
    let start = text.find(what).unwrap();
    editor(app, id, EditorMsg::PressedAt(start));
    let page = &mut app.editors.get_mut(&id).unwrap().page;
    page.place(crate::markdown::edit::Sel { anchor: start, head: start + what.len() });
}

fn clipboard(app: &App) -> super::clipboard::ClipboardContent {
    app.system_clipboard.lock().unwrap().read()
}

fn pasted(app: &mut App, id: window::Id, text: &str, html: Option<&str>, plain: bool) {
    let content = super::clipboard::ClipboardContent {
        text: Some(text.into()),
        html: html.map(str::to_string),
        ..Default::default()
    };
    editor(app, id, EditorMsg::Pasted(content, plain));
}

fn cell(app: &App, id: window::Id) -> (usize, usize) {
    app.editors[&id].page.active.as_ref().unwrap().table().unwrap().cell
}

fn live(app: &mut App, id: window::Id, msg: LiveMsg) {
    editor(app, id, EditorMsg::Live(msg));
}

/// The kind of the block being edited.
fn active_kind(app: &App, id: window::Id) -> crate::markdown::BlockKind {
    app.editors[&id].page.active.as_ref().map(|a| a.kind.clone()).unwrap_or(crate::markdown::BlockKind::Other)
}
