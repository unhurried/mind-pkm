//! What an editor window gets and gives: its messages (`EditorMsg`), the
//! dialogs it shows over the page, where a right click landed, and what it
//! asks of the application (`Effect`).

use super::clipboard::ClipboardContent;
use super::view::HitInfo;
use super::{AssetsMsg, Bind, EditorCommand, EditorPrefs, FindMsg, LiveMsg, TableMsg};
use crate::app::scroll_watch::Visible;
use iced::keyboard;
use iced::widget::text_editor::Action;
use iced::{Point, Size};
use std::path::PathBuf;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub enum EditorMsg {
    Key(keyboard::Event),
    Escape,
    Resized(Size),
    CloseRequested,
    FileDropped(PathBuf),
    Focused,
    /// An action of the active region's text editor.
    Action(Action),
    /// A key the editor binds itself.
    Bind(Bind),
    Command(EditorCommand),
    Menu(Option<usize>),
    /// The pointer moved over a piece of rendered text, to this position in it.
    PointerOver(Arc<HitInfo>, Point),
    /// The pointer moved, in the window (where a menu opens, a dragged item is).
    PointerAt(Point),
    /// The left button went down at this point of the page.
    PressPoint(Point),
    /// A press on the piece of text the pointer is over.
    TextPressed,
    /// A press on a rendered block outside its text.
    BlockPressed(usize),
    /// The pointer entered a rendered block.
    BlockEntered(usize),
    /// The left mouse button was released, anywhere in the window.
    Released,
    /// The language field of the code block being edited changed.
    CodeLanguage(String),
    /// A press on a cell of the table being edited.
    CellPressed(usize, usize),
    /// A press on a cell (row, column) of a rendered table: the table block, the cell.
    TableCellPressed(usize, usize, usize),
    /// A table wider than the page was scrolled sideways: where the table
    /// starts in the source, and how far it is scrolled.
    TableScrolled(usize, f32),
    /// A press on a source offset of a rendered block (a link, a task box...).
    PressedAt(usize),
    /// A link clicked: its destination and where its text is in the source.
    Link(String, usize),
    TaskToggled(usize),
    /// A heading of the outline or a table of contents.
    HeadingClicked(usize),
    /// The arrow of a heading in the outline: fold or unfold its entries.
    OutlineFoldToggled(usize),
    /// The right button went down at this point of the window (before the
    /// widget under the pointer says what was right-clicked).
    RightPressedAt(Point),
    /// A right click on the page: where it landed.
    RightClicked(RightClick),
    CloseContextMenu,
    /// The visible part of the page came near the end of the blocks built
    /// around it (or left them): where it is now.
    PageScrolled(Visible),
    /// The page measures more of its blocks: nothing to do but show it again.
    Measure,
    Dialog(DialogAnswer),
    /// The clipboard was read for pasting (as plain text when `true`).
    Pasted(ClipboardContent, bool),
    /// The clipboard's text was read for Insert Link (a URL or a path in it
    /// becomes the link's destination).
    LinkClipboard(Option<String>),
    /// Image files picked with Format → Image.
    ImagesChosen(Option<Vec<PathBuf>>),
    /// The Markdown of inserted files (copied into the image folder), or why not.
    Inserted(Result<Vec<String>, String>),
    Find(FindMsg),
    Table(TableMsg),
    Assets(AssetsMsg),
    /// From the widget editing a block in its rendered form.
    Live(LiveMsg),
    /// Where to export to (as PDF when `true`), as chosen in the save dialog.
    ExportTo(bool, Option<PathBuf>),
    /// The file dialog of File → Open... closed: the chosen file, unless it was cancelled.
    OpenChosen(Option<PathBuf>),
    /// The file exported, or why not.
    Exported(Result<PathBuf, String>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DialogAnswer {
    Ok,
    Save,
    Discard,
    Cancel,
}

/// A question or message shown over the page.
#[derive(Debug, Clone, PartialEq)]
pub enum EditorDialog {
    Message(String),
    /// Closing with unsaved changes.
    Unsaved,
    /// The file changed on disk while there are unsaved changes.
    ChangedOnDisk,
    /// The new name of an item of the assets pane, as typed.
    RenameAsset {
        path: PathBuf,
        name: String,
    },
    /// Deleting an item of the assets pane.
    DeleteAsset(PathBuf),
}

impl EditorDialog {
    /// The answer of the highlighted button, which Enter chooses.
    pub(super) fn default_answer(&self) -> DialogAnswer {
        match self {
            EditorDialog::Message(_) | EditorDialog::RenameAsset { .. } | EditorDialog::DeleteAsset(_) => {
                DialogAnswer::Ok
            }
            EditorDialog::Unsaved => DialogAnswer::Save,
            EditorDialog::ChangedOnDisk => DialogAnswer::Discard,
        }
    }
}

/// Where a right click on the page landed, which says where it puts the
/// cursor (unless that is inside the selection).
#[derive(Debug, Clone, PartialEq)]
pub enum RightClick {
    /// The piece of rendered text the pointer is over.
    Text,
    /// A rendered block outside its text.
    Block(usize),
    /// A source offset: beside a line, in a gap, an image.
    At(usize),
    /// A cell (row, column) of a rendered table: the table block, the cell.
    TableCell(usize, usize, usize),
    /// A shown offset of the region edited as rendered text.
    Live(usize),
    /// A point of the text editor of the active region or of the source mode.
    Editor(Point),
    /// Elsewhere on the page: the cursor stays.
    Page,
}

/// What an editor asks of the application.
#[derive(Debug, Clone, PartialEq)]
pub enum Effect {
    /// Open a page (a `.md` file) in the editor - with the default
    /// application when the built-in editor is turned off.
    OpenPage(PathBuf),
    /// Open a Markdown file the user chose in the built-in editor, whether
    /// or not it is turned off.
    OpenInEditor(PathBuf),
    /// Open a file, directory or URL with its default application.
    OpenExternal(String),
    Close,
    SetPrefs(EditorPrefs),
    /// Put text (and its HTML) on the clipboard.
    Copy {
        text: String,
        html: Option<String>,
    },
    /// Read the clipboard to paste (as plain text when `true`).
    ReadClipboard(bool),
    /// Read the clipboard's text for Insert Link.
    ReadClipboardForLink,
    /// Open the right-click menu of an item of the assets pane, saying whether it may be changed.
    AssetMenu(PathBuf),
    RenameAsset(PathBuf, String),
    DeleteAsset(PathBuf),
}
