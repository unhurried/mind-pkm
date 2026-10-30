//! The page editor: a window per open Markdown file, showing the page as a
//! live preview - every block rendered, the one with the cursor edited in its
//! rendered form (code-like blocks as source in a text editor).
//!
//! `page` holds the page and its editing state, `live_text` the widget
//! editing rendered text, `keys` the commands and key bindings, `highlight`
//! the styling of the source, `view` the rendering; `tables`, `find` and
//! `assets` handle the table being edited, the find bar and the assets pane.

mod assets;
pub mod clipboard;
mod column;
mod find;
mod flow;
mod highlight;
mod keys;
mod live_text;
mod modal;
mod page;
mod render;
mod scroll;
mod tables;
mod view;

#[cfg(test)]
mod tests;

use super::file::file_name;
use super::ui::zoom_step;
use super::{Message, Status, blocking, settings};
use crate::markdown::count;
use crate::markdown::edit::{self, Sel};
use crate::markdown::format::{self, InlineStyle};
use crate::markdown::images::{self, ImageOptions};
use crate::markdown::paste::{self, Paste};
use crate::markdown::{export, html, table};
pub use assets::AssetsMsg;
use clipboard::ClipboardContent;
pub use find::FindMsg;
use iced::keyboard::{self, Key, Modifiers, key::Named};
use iced::widget::text_editor::{Action, Content, Edit, Motion};
use iced::widget::{operation, scrollable};
use iced::{Point, Size, Subscription, Task, window};
pub use keys::{Bind, EditorCommand};
pub use live_text::LiveMsg;
use page::{Page, content_sel, move_content};
pub use render::{RenderCache, RenderKey};
use std::collections::{BTreeMap, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
pub use tables::TableMsg;
use tables::TableUi;
use view::HitInfo;

/// The size of the first editor window, when none was remembered.
pub const DEFAULT_WINDOW: Size = Size::new(900.0, 860.0);
pub const ZOOM_STEPS: &[f32] = &[0.7, 0.8, 0.9, 1.0, 1.1, 1.25, 1.5, 1.75, 2.0];

static NEXT_UID: AtomicU64 = AtomicU64::new(1);

/// The editor settings every editor window shares.
#[derive(Debug, Clone, PartialEq)]
pub struct EditorPrefs {
    /// Pages open in the built-in editor (else with the default application).
    pub builtin: bool,
    pub dark: bool,
    pub zoom: f32,
    pub auto_pair: bool,
    pub window: Size,
    pub images: ImageOptions,
}

impl EditorPrefs {
    pub fn load() -> EditorPrefs {
        EditorPrefs {
            builtin: settings::editor_builtin(),
            dark: settings::editor_dark(),
            zoom: settings::editor_zoom().unwrap_or(1.0),
            auto_pair: settings::editor_auto_pair(),
            window: settings::editor_window().unwrap_or(DEFAULT_WINDOW),
            images: settings::image_options(),
        }
    }
}

impl Default for EditorPrefs {
    fn default() -> Self {
        EditorPrefs {
            builtin: true,
            dark: false,
            zoom: 1.0,
            auto_pair: true,
            window: DEFAULT_WINDOW,
            images: ImageOptions::default(),
        }
    }
}

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
    /// A press on a source offset of a rendered block (a link, a task box...).
    PressedAt(usize),
    /// A link clicked: its destination and where its text is in the source.
    Link(String, usize),
    TaskToggled(usize),
    /// A heading of the outline or a table of contents.
    HeadingClicked(usize),
    Scrolled(scrollable::Viewport),
    Dialog(DialogAnswer),
    /// The clipboard was read for pasting (as plain text when `true`).
    Pasted(ClipboardContent, bool),
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
    fn default_answer(&self) -> DialogAnswer {
        match self {
            EditorDialog::Message(_) | EditorDialog::RenameAsset { .. } | EditorDialog::DeleteAsset(_) => {
                DialogAnswer::Ok
            }
            EditorDialog::Unsaved => DialogAnswer::Save,
            EditorDialog::ChangedOnDisk => DialogAnswer::Discard,
        }
    }
}

/// What an editor asks of the application.
#[derive(Debug, Clone, PartialEq)]
pub enum Effect {
    /// Open a page (a `.md` file) in the editor.
    OpenPage(PathBuf),
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
    /// Open the right-click menu of an item of the assets pane, saying whether it may be changed.
    AssetMenu(PathBuf),
    RenameAsset(PathBuf, String),
    DeleteAsset(PathBuf),
}

pub struct Editor {
    uid: u64,
    /// The editor's window, once it is open.
    pub window: Option<window::Id>,
    pub page: Page,
    menu: Option<usize>,
    pub dialogs: VecDeque<EditorDialog>,
    /// The whole page in a text editor, in source mode.
    source: Option<Content>,
    outline: bool,
    /// The assets pane, when shown.
    assets: Option<assets::AssetsPane>,
    fullscreen: bool,
    /// The status bar message.
    pub(super) status: Status,
    modifiers: Modifiers,
    /// The rendered text under the pointer and the pointer's position in it.
    pointer: Option<(Arc<HitInfo>, Point)>,
    window_size: Size,
    /// How the document is scrolled, as last reported.
    viewport: Option<scrollable::Viewport>,
    /// Ctrl+A pressed again selects all blocks.
    select_all_again: bool,
    /// The page revision and theme its math and diagrams were last requested for.
    pub rendered: Option<(u64, bool)>,
    /// The text this editor last put on the clipboard (pasted back as it is).
    copied: Option<String>,
    /// When and where the last inserted files went (files dropped together go on separate lines).
    last_insert: Option<(std::time::Instant, usize)>,
    pub find: Option<find::FindBar>,
    /// The table being edited, as the window shows it.
    table_ui: TableUi,
    /// The block a press started in, while the button is held: dragging onto
    /// another block selects the blocks between.
    block_drag: Option<usize>,
    /// The pointer in the window, while a table is edited or the assets pane
    /// is shown (where a menu opens, a dragged item is).
    pointer_at: Point,
}

impl Editor {
    /// An editor of the page at `path`, for a window of `window_size`.
    pub fn open(path: &Path, window_size: Size) -> Result<Editor, String> {
        Ok(Editor { window_size, ..Editor::new(Page::open(path)?) })
    }

    pub fn new(page: Page) -> Editor {
        Editor {
            uid: NEXT_UID.fetch_add(1, Ordering::Relaxed),
            window: None,
            page,
            menu: None,
            dialogs: VecDeque::new(),
            source: None,
            outline: false,
            assets: None,
            fullscreen: false,
            status: Status::default(),
            modifiers: Modifiers::default(),
            pointer: None,
            window_size: DEFAULT_WINDOW,
            viewport: None,
            select_all_again: false,
            rendered: None,
            copied: None,
            last_insert: None,
            find: None,
            table_ui: TableUi::default(),
            block_drag: None,
            pointer_at: Point::ORIGIN,
        }
    }

    pub fn title(&self) -> String {
        let name = crate::paths::file_name(self.page.path());
        format!("{name}{} - Mind PKM", if self.page.is_dirty() { " *" } else { "" })
    }

    pub(super) fn id(&self, part: &str) -> iced::widget::Id {
        iced::widget::Id::from(format!("editor-{}-{part}", self.uid))
    }

    /// Focuses the text editor of the active region (or the source).
    fn focus(&self) -> Task<EditorMsg> {
        operation::focus(self.id("active"))
    }

    pub fn update(&mut self, msg: EditorMsg, prefs: &EditorPrefs, effects: &mut Vec<Effect>) -> Task<EditorMsg> {
        // After typing and moving with keys, the cursor stays in view; other
        // messages of these kinds (a key release, a modifier, the wheel) leave
        // the scrolling to the user.
        let may_reveal = matches!(
            msg,
            EditorMsg::Action(_) | EditorMsg::Bind(_) | EditorMsg::Key(_) | EditorMsg::Find(_) | EditorMsg::Live(_)
        );
        let command = matches!(msg, EditorMsg::Command(_));
        let press = self.source.is_none()
            && matches!(
                msg,
                EditorMsg::Live(LiveMsg::Click { .. })
                    | EditorMsg::Action(Action::Click(_))
                    | EditorMsg::TextPressed
                    | EditorMsg::BlockPressed(_)
                    | EditorMsg::PressedAt(_)
                    | EditorMsg::CellPressed(..)
                    | EditorMsg::TableCellPressed(..)
            );
        let before = self.cursor_state();
        let revision = self.page.revision();
        let shown = self.status.text().to_string();
        let task = self.handle(msg, prefs, effects);
        if press {
            self.block_drag = self.page.active_block().or(self.page.selected_blocks().map(|b| b.0));
        }
        // An edit clears the message about what came before it.
        if self.page.revision() != revision && self.status.text() == shown {
            self.status.clear();
        }
        self.refresh_find();
        if !self.page.active().is_some_and(|a| a.table().is_some()) {
            self.table_ui = TableUi::default();
        }
        let reveals = command || (may_reveal && self.cursor_state() != before);
        if reveals { Task::batch([task, self.reveal(prefs)]) } else { task }
    }

    /// What moving the cursor or editing changes: the selection (of the
    /// source mode's editor too), the table cell, the text.
    fn cursor_state(&self) -> CursorState {
        CursorState {
            sel: self.page.sel(),
            blocks: self.page.selected_blocks(),
            revision: self.page.revision(),
            region: self.page.active().map(|a| a.region.clone()),
            cell: self.page.active().and_then(|a| a.table()).map(|t| (t.cell, t.cell_sel)),
            source: self.source.as_ref().map(Content::cursor),
        }
    }

    /// Scrolls the page to show the cursor.
    fn reveal(&self, prefs: &EditorPrefs) -> Task<EditorMsg> {
        if self.source.is_none() && self.page.active().is_some_and(|a| a.live().is_some()) {
            return scroll::reveal_live(self.id("active"), self.id("doc"));
        }
        let (before, kind) = match (&self.source, self.page.active()) {
            (Some(_), _) => (
                self.page.text()[..self.page.sel().head.min(self.page.text().len())].to_string(),
                crate::markdown::BlockKind::Paragraph,
            ),
            (None, Some(a)) if a.table().is_some() => {
                return scroll::reveal_box(self.id("active-cell"), self.id("doc"));
            }
            (None, Some(a)) if a.code().is_some() => {
                let code = a.code().map_or("", |c| c.text());
                let at = a.code().map_or(0, |c| c.to_body(self.page.sel().head)).min(code.len());
                let (size, font, padding) = view::code_text_style(prefs.zoom);
                let caret = scroll::CaretText {
                    before: code[..at].to_string(),
                    size,
                    font,
                    line_height: view::TEXT_LINE_HEIGHT,
                    padding,
                };
                return scroll::reveal_caret(self.id("active-box"), self.id("doc"), caret);
            }
            (None, Some(a)) => {
                let text = &self.page.text()[a.region.clone()];
                let at = self.page.sel().head.clamp(a.region.start, a.region.end) - a.region.start;
                (text[..at].to_string(), a.kind.clone())
            }
            _ => return Task::none(),
        };
        let (size, font, padding) = view::region_text_style(&kind, prefs.zoom);
        let caret = scroll::CaretText { before, size, font, line_height: view::TEXT_LINE_HEIGHT, padding };
        scroll::reveal_caret(self.id("active-box"), self.id("doc"), caret)
    }

    fn handle(&mut self, msg: EditorMsg, prefs: &EditorPrefs, effects: &mut Vec<Effect>) -> Task<EditorMsg> {
        match msg {
            EditorMsg::Key(event) => self.on_key(event, prefs, effects),
            EditorMsg::Escape => self.on_escape(),
            EditorMsg::Resized(size) => {
                self.window_size = size;
                if !self.fullscreen && size.width >= 200.0 && size.height >= 200.0 {
                    effects.push(Effect::SetPrefs(EditorPrefs { window: size, ..prefs.clone() }));
                }
                Task::none()
            }
            EditorMsg::CloseRequested => self.close_requested(effects),
            EditorMsg::FileDropped(path) => self.insert_files(vec![path], prefs),
            EditorMsg::Focused => Task::none(),
            EditorMsg::Pasted(content, plain) => self.pasted(content, plain, prefs),
            EditorMsg::ImagesChosen(files) => self.insert_files(files.unwrap_or_default(), prefs),
            EditorMsg::Inserted(result) => {
                match result {
                    Ok(pieces) if !pieces.is_empty() => {
                        let recent = self
                            .last_insert
                            .is_some_and(|(t, at)| t.elapsed().as_secs() < 2 && at == self.page.sel().head);
                        let text = format!("{}{}", if recent { "\n" } else { "" }, pieces.join("\n"));
                        self.insert_text(&text);
                        self.last_insert = Some((std::time::Instant::now(), self.page.sel().head));
                        self.assets_stale();
                    }
                    Ok(_) => {}
                    Err(e) => self.dialogs.push_back(EditorDialog::Message(e)),
                }
                self.focus()
            }
            EditorMsg::Action(action) => self.on_action(action),
            EditorMsg::Bind(b) => self.on_bind(b, prefs),
            EditorMsg::Command(cmd) => {
                self.menu = None;
                self.run(cmd, prefs, effects)
            }
            EditorMsg::Menu(m) => {
                self.menu = m;
                self.table_ui.menu = None;
                Task::none()
            }
            EditorMsg::PointerOver(info, p) => {
                if self.asset_dragging() {
                    self.drag_over_text(&info, p);
                }
                self.pointer = Some((info, p));
                Task::none()
            }
            EditorMsg::PointerAt(p) => {
                self.pointer_at = p;
                self.assets_pointer(p);
                Task::none()
            }
            EditorMsg::TextPressed => self.text_pressed(),
            EditorMsg::BlockPressed(block) => self.block_pressed(block),
            EditorMsg::BlockEntered(block) if self.asset_dragging() => {
                self.assets_message(AssetsMsg::BlockHovered(block), effects)
            }
            EditorMsg::BlockEntered(block) => {
                if let Some(anchor) = self.block_drag
                    && (block != anchor || self.page.selected_blocks().is_some())
                {
                    self.menu = None;
                    self.page.select_blocks(anchor, block);
                }
                Task::none()
            }
            EditorMsg::Released => {
                (self.table_ui.drag, self.block_drag) = (false, None);
                self.assets_released(prefs)
            }
            EditorMsg::CodeLanguage(language) => {
                self.page.set_code_language(&language);
                Task::none()
            }
            EditorMsg::CellPressed(r, c) => self.cell_pressed((r, c)),
            EditorMsg::TableCellPressed(block, r, c) => self.table_cell_pressed(block, (r, c)),
            EditorMsg::PressedAt(offset) => {
                self.menu = None;
                self.page.place(Sel::caret(offset));
                self.focus()
            }
            EditorMsg::Link(dest, at) => self.link_clicked(&dest, at, effects),
            EditorMsg::TaskToggled(block) => {
                let start = self.page.doc().blocks.get(block).map(|b| b.range.start);
                if let Some(e) = start.and_then(|s| format::toggle_task(self.page.text(), s)) {
                    let sel = self.page.sel();
                    self.page.apply(e);
                    self.page.leave_at(sel);
                }
                Task::none()
            }
            EditorMsg::HeadingClicked(i) => self.go_to_heading(i),
            EditorMsg::Scrolled(viewport) => {
                self.viewport = Some(viewport);
                Task::none()
            }
            EditorMsg::Dialog(answer) => self.dialog_answered(answer, effects),
            EditorMsg::Find(msg) => self.find_message(msg),
            EditorMsg::Live(msg) => self.live_message(msg),
            EditorMsg::Table(msg) => self.table_message(msg, effects),
            EditorMsg::Assets(msg) => self.assets_message(msg, effects),
            EditorMsg::ExportTo(pdf, out) => self.export_to(pdf, out),
            EditorMsg::Exported(result) => {
                match result {
                    Ok(out) => self.status.set(format!("exported {}", file_name(&out))),
                    Err(e) => self.dialogs.push_back(EditorDialog::Message(e)),
                }
                Task::none()
            }
        }
    }

    // --- Keys ---

    /// A key no widget took: the editor's shortcuts, and moving into the
    /// page when no block is active.
    fn on_key(&mut self, event: keyboard::Event, prefs: &EditorPrefs, effects: &mut Vec<Effect>) -> Task<EditorMsg> {
        let (key, physical, modifiers, text) = match event {
            keyboard::Event::ModifiersChanged(m) => {
                self.modifiers = m;
                return Task::none();
            }
            keyboard::Event::KeyPressed { key, physical_key, modifiers, text, .. } => {
                (key, physical_key, modifiers, text)
            }
            _ => return Task::none(),
        };
        self.modifiers = modifiers;
        if let Some(dialog) = self.dialogs.front() {
            return match key {
                Key::Named(Named::Enter) => self.dialog_answered(dialog.default_answer(), effects),
                _ => Task::none(),
            };
        }
        if let Some(cmd) = keys::command_for_key(&key, physical, modifiers) {
            self.menu = None;
            return self.run(cmd, prefs, effects);
        }
        if self.page.selected_blocks().is_some() {
            return self.blocks_key(&key, modifiers, text.as_deref());
        }
        // Nothing active: arrow keys and typing start at the cursor, and
        // what was typed goes in.
        let typed: Option<String> = text
            .as_deref()
            .map(|t| t.chars().filter(|c| !c.is_control()).collect::<String>())
            .filter(|t| !t.is_empty() && !(modifiers.control() || modifiers.alt()));
        let starts =
            matches!(key, Key::Named(Named::ArrowUp | Named::ArrowDown | Named::Enter | Named::Home | Named::End))
                || typed.is_some();
        if starts && self.page.active().is_none() && self.source.is_none() {
            self.page.place(self.page.sel());
            self.page.snap_to_shown();
            match key {
                Key::Named(Named::Home) => self.page.perform(Action::Move(Motion::Home)),
                Key::Named(Named::End) => self.page.perform(Action::Move(Motion::End)),
                _ => {}
            }
            for c in typed.iter().flat_map(|t| t.chars()) {
                let _ = self.on_action(Action::Edit(Edit::Insert(c)));
            }
            return self.focus();
        }
        Task::none()
    }

    fn on_escape(&mut self) -> Task<EditorMsg> {
        if !self.dialogs.is_empty() {
            self.dialogs.pop_front();
        } else if self.menu.is_some()
            || self.table_ui.menu.is_some()
            || self.assets.as_ref().is_some_and(|a| a.menu.is_some())
        {
            self.menu = None;
            self.table_ui.menu = None;
            if let Some(pane) = &mut self.assets {
                pane.menu = None;
            }
        } else if self.find.is_some() {
            return self.close_find();
        } else if self.page.selected_blocks().is_some() {
            self.page.deselect_blocks();
        } else if self.fullscreen {
            self.fullscreen = false;
            if let Some(id) = self.window {
                return window::set_mode(id, window::Mode::Windowed);
            }
        }
        Task::none()
    }

    /// Keys while whole blocks are selected.
    fn blocks_key(&mut self, key: &Key, m: Modifiers, text: Option<&str>) -> Task<EditorMsg> {
        let Some((anchor, head)) = self.page.selected_blocks() else { return Task::none() };
        let last = self.page.doc().blocks.len().saturating_sub(1);
        match key {
            Key::Named(Named::ArrowUp | Named::ArrowDown) if m.shift() => {
                let up = *key == Key::Named(Named::ArrowUp);
                let head = if up { head.saturating_sub(1) } else { (head + 1).min(last) };
                self.page.set_blocks_head(head);
                Task::none()
            }
            Key::Named(Named::ArrowUp) | Key::Named(Named::ArrowLeft) => {
                let first = anchor.min(head);
                self.page.deselect_blocks();
                self.page.enter_block(first, true, None);
                self.focus()
            }
            Key::Named(Named::ArrowDown) | Key::Named(Named::ArrowRight) => {
                let end = anchor.max(head);
                self.page.deselect_blocks();
                self.page.enter_block(end, false, None);
                self.focus()
            }
            Key::Named(Named::Delete | Named::Backspace) => {
                self.delete_blocks(None);
                self.focus()
            }
            _ => match text.and_then(|t| t.chars().find(|c| !c.is_control())) {
                Some(c) if !(m.control() || m.alt()) => {
                    self.delete_blocks(Some(c));
                    self.focus()
                }
                _ => Task::none(),
            },
        }
    }

    /// Removes the selected blocks; typing `with` puts a paragraph of it in their place.
    fn delete_blocks(&mut self, with: Option<char>) {
        let Some(range) = self.page.blocks_range() else { return };
        let text = self.page.text();
        let insert = with.map(String::from).unwrap_or_default();
        let (new, at) = if !insert.is_empty() {
            (edit::splice(text, range.clone(), &insert), range.start + insert.len())
        } else if text[range.end..].trim().is_empty() {
            // The last blocks: the blank lines before them go too.
            let before = text[..range.start].trim_end().len();
            let end = if before == 0 { "" } else { "\n" };
            (format!("{}{end}", &text[..before]), before)
        } else {
            // The blank lines after the blocks go with them.
            let end = range.end + (text.len() - range.end - text[range.end..].trim_start().len());
            (edit::splice(text, range.start..end, ""), range.start)
        };
        self.page.deselect_blocks();
        self.page.set_text(new, Sel::caret(at));
    }

    // --- The active region ---

    fn on_action(&mut self, action: Action) -> Task<EditorMsg> {
        self.menu = None;
        self.select_all_again = false;
        if let Some(source) = &mut self.source {
            let is_edit = action.is_edit();
            source.perform(action);
            if is_edit {
                let text = source.text();
                let sel = content_sel(source, &text);
                self.page.set_source_text(text, sel);
            }
            return Task::none();
        }
        if matches!(action, Action::Click(_)) {
            self.page.deselect_blocks();
        }
        self.page.perform(action);
        Task::none()
    }

    fn on_bind(&mut self, bind: Bind, prefs: &EditorPrefs) -> Task<EditorMsg> {
        self.menu = None;
        self.select_all_again = false;
        if self.source.is_some() {
            return self.source_bind(bind, prefs);
        }
        let Some(active) = self.page.active() else { return Task::none() };
        let region = active.region.clone();
        let in_table = active.table().is_some();
        if in_table {
            return self.table_bind(bind);
        }
        let text = self.page.text().to_string();
        // The rules work on the source: at the start / end of what a block
        // shows, that is its content's start / end.
        let sel = self.page.rule_sel();
        let doc = self.page.doc();
        let pending = self.page.pending().is_some();
        match bind {
            Bind::Enter => {
                if pending {
                    // An empty new paragraph in a quote or list item leaves it.
                    self.page.leave_pending_container();
                    return self.focus();
                }
                match edit::nested(&text, doc, region, sel, edit::enter) {
                    Some(e) => self.page.apply(e),
                    None => self.page.perform(Action::Edit(Edit::Enter)),
                }
            }
            Bind::ShiftEnter => {
                if pending {
                    return Task::none();
                }
                match edit::nested(&text, doc, region, sel, edit::shift_enter) {
                    Some(e) => self.page.apply(e),
                    None => self.page.perform(Action::Edit(Edit::Enter)),
                }
            }
            Bind::Tab | Bind::ShiftTab => {
                let outdent = bind == Bind::ShiftTab;
                if let Some(e) = edit::nested(&text, doc, region, sel, |t, d, r, s| edit::indent(t, d, r, s, outdent)) {
                    self.page.apply(e);
                }
            }
            Bind::Backspace => {
                if pending {
                    // An empty new paragraph goes; the cursor to the end of the block before.
                    let before = self.page.neighbor(false);
                    self.page.deactivate();
                    match before {
                        Some(b) => self.page.enter_block(b, false, None),
                        None => self.page.place(Sel::caret(0)),
                    }
                    return self.focus();
                }
                let code = self.page.active().and_then(|a| a.code());
                if let Some(code) = code.filter(|c| sel.is_empty() && c.to_body(sel.head) == 0) {
                    // At the start of the code: an empty block goes, else to the block before.
                    if code.text().is_empty() {
                        self.page.remove_empty_code();
                    } else if let Some(e) = edit::backspace_nested(&text, doc, region.clone(), region.start) {
                        self.page.apply(e);
                    }
                } else if !sel.is_empty() {
                    self.page.perform(Action::Edit(Edit::Backspace));
                } else if let Some(e) = prefs.auto_pair.then(|| format::pair_backspace(&text, sel.head)).flatten() {
                    self.page.apply(e);
                } else if let Some(e) = edit::backspace_nested(&text, doc, region, sel.head) {
                    self.page.apply(e);
                } else {
                    self.page.perform(Action::Edit(Edit::Backspace));
                }
            }
            Bind::Delete => {
                if pending {
                    let after = self.page.neighbor(true);
                    self.page.deactivate();
                    if let Some(b) = after {
                        self.page.enter_block(b, true, None);
                    }
                    return self.focus();
                }
                let delete = |t: &str, d: &crate::markdown::Document, r, s: Sel| edit::delete(t, d, r, s.head);
                match (sel.is_empty()).then(|| edit::nested(&text, doc, region, sel, delete)).flatten() {
                    Some(e) => self.page.apply(e),
                    None => self.page.perform(Action::Edit(Edit::Delete)),
                }
            }
            Bind::Up | Bind::Down => return self.vertical(bind == Bind::Down),
            Bind::ShiftUp | Bind::ShiftDown => return self.extend_vertically(bind == Bind::ShiftDown),
            Bind::Left | Bind::Right => return self.horizontal(bind == Bind::Right),
            Bind::ShiftLeft | Bind::ShiftRight => {
                self.page.perform(Action::Select(if bind == Bind::ShiftRight { Motion::Right } else { Motion::Left }));
            }
            Bind::Type(c) => {
                let code = self.page.active().is_some_and(|a| a.kind.is_code());
                match prefs.auto_pair.then(|| format::auto_pair(&text, sel, c, code)).flatten() {
                    Some(e) if !pending => self.page.apply_typed(e, c),
                    _ => self.page.perform(Action::Edit(Edit::Insert(c))),
                }
            }
        }
        self.focus()
    }

    /// Up / Down: within the region, or into the block before / after it.
    fn vertical(&mut self, down: bool) -> Task<EditorMsg> {
        let motion = if down { Motion::Down } else { Motion::Up };
        let before = self.page.cursor_token();
        if self.page.pending().is_none() {
            self.page.perform(Action::Move(motion));
        }
        let after = self.page.cursor_token();
        if self.page.pending().is_some() || before == after {
            let column = self.page.column();
            match self.page.neighbor(down) {
                // Below code ending a quote or list item: a new paragraph in it.
                Some(_) if down && self.page.pending().is_none() && self.page.ends_container() => {
                    self.page.new_paragraph_after()
                }
                Some(b) => self.page.enter_block(b, down, Some(column)),
                // Below the last block, when it is code: a new paragraph.
                None if down && self.page.pending().is_none() => self.page.new_paragraph_after(),
                None => {}
            }
        }
        self.focus()
    }

    /// Shift+Up / Shift+Down: selects within the region, then whole blocks.
    fn extend_vertically(&mut self, down: bool) -> Task<EditorMsg> {
        let motion = if down { Motion::Down } else { Motion::Up };
        let before = self.page.cursor_token();
        if self.page.pending().is_none() {
            self.page.perform(Action::Select(motion));
        }
        let after = self.page.cursor_token();
        if self.page.pending().is_some() || before == after {
            let anchor = self.page.active_block().or_else(|| self.page.neighbor(down));
            if let Some(anchor) = anchor {
                let head = self.page.neighbor(down).unwrap_or(anchor);
                self.page.select_blocks(anchor, head);
            }
        }
        Task::none()
    }

    /// Left at the start / Right at the end of the region move into the block before / after.
    fn horizontal(&mut self, right: bool) -> Task<EditorMsg> {
        if self.page.active().is_none() {
            return Task::none();
        }
        if self.page.at_edge(right) || self.page.pending().is_some() {
            match self.page.neighbor(right) {
                Some(b) => self.page.enter_block(b, right, None),
                None if right && self.page.pending().is_none() => self.page.new_paragraph_after(),
                None => {}
            }
        } else {
            self.page.perform(Action::Move(if right { Motion::Right } else { Motion::Left }));
        }
        self.focus()
    }

    /// A press on rendered text: the cursor goes where the text came from.
    fn text_pressed(&mut self) -> Task<EditorMsg> {
        let Some((info, p)) = self.pointer.clone() else { return Task::none() };
        if self.shift_selects(info.block) {
            return Task::none();
        }
        self.menu = None;
        let offset = view::hit_test(&info, p).or_else(|| self.page.doc().blocks.get(info.block).map(|b| b.range.start));
        if let Some(offset) = offset {
            self.page.place(Sel::caret(offset));
        }
        self.focus()
    }

    fn block_pressed(&mut self, block: usize) -> Task<EditorMsg> {
        self.menu = None;
        if self.shift_selects(block) {
            return Task::none();
        }
        if let Some(b) = self.page.doc().blocks.get(block) {
            // Beside a list item's text (on its bullet): where its text starts.
            let at = match b.kind {
                crate::markdown::BlockKind::ListItem(_) => {
                    let region = edit::region_at(self.page.doc(), b.range.start).unwrap_or(b.range.clone());
                    edit::content_start(self.page.text(), b.range.start).min(region.end)
                }
                _ => b.range.start,
            };
            self.page.place(Sel::caret(at));
        }
        self.focus()
    }

    /// Shift+click: selects the blocks from the active one to `block`.
    fn shift_selects(&mut self, block: usize) -> bool {
        if !self.modifiers.shift() {
            return false;
        }
        let Some(anchor) = self.page.active_block().or(self.page.selected_blocks().map(|b| b.0)) else { return false };
        self.page.select_blocks(anchor, block);
        true
    }

    fn link_clicked(&mut self, dest: &str, at: usize, effects: &mut Vec<Effect>) -> Task<EditorMsg> {
        // A footnote reference goes to its definition.
        if let Some(label) = dest.strip_prefix('^') {
            let block = self
                .page
                .doc()
                .blocks
                .iter()
                .position(|b| b.kind == crate::markdown::BlockKind::Footnote(label.to_string()));
            return block.map_or(Task::none(), |b| {
                self.page.deactivate();
                self.scroll_to_block(b)
            });
        }
        if !(self.modifiers.control() || self.modifiers.command()) {
            self.page.place(Sel::caret(at));
            return self.focus();
        }
        if let Some(anchor) = dest.strip_prefix('#') {
            if let Some(h) = self.page.doc().heading_for_anchor(anchor) {
                let index = self.page.doc().headings.iter().position(|x| x.block == h.block).unwrap_or(0);
                return self.go_to_heading(index);
            }
            return Task::none();
        }
        match resolve_link(self.page.path(), dest) {
            LinkTarget::Page(path) => effects.push(Effect::OpenPage(path)),
            LinkTarget::External(target) => effects.push(Effect::OpenExternal(target)),
        }
        Task::none()
    }

    fn go_to_heading(&mut self, index: usize) -> Task<EditorMsg> {
        let Some(h) = self.page.doc().headings.get(index) else { return Task::none() };
        let block = h.block;
        self.page.leave_at(Sel::caret(self.page.doc().blocks[block].range.start));
        self.scroll_to_block(block)
    }

    // --- Commands ---

    fn run(&mut self, cmd: EditorCommand, prefs: &EditorPrefs, effects: &mut Vec<Effect>) -> Task<EditorMsg> {
        use EditorCommand as C;
        self.select_all_again &= cmd == C::SelectAll;
        self.table_ui.menu = None;
        if self.source.is_none() && self.table_command(cmd) {
            return self.focus();
        }
        match cmd {
            C::Save => self.save(),
            C::ExportHtml => self.ask_export_file(false),
            C::ExportPdf => {
                if export::find_browser().is_none() {
                    self.dialogs.push_back(EditorDialog::Message(export::NO_BROWSER.to_string()));
                    return Task::none();
                }
                self.ask_export_file(true)
            }
            C::Close => self.close_requested(effects),
            C::Undo => {
                self.leave_source_for_history();
                if self.page.undo() {
                    self.refresh_source();
                }
                self.focus()
            }
            C::Redo => {
                self.leave_source_for_history();
                if self.page.redo() {
                    self.refresh_source();
                }
                self.focus()
            }
            C::SelectAll => self.select_all(),
            C::SelectWord => self.with_active(|p| {
                if let Some(w) = format::word_at(p.text(), p.sel().head) {
                    p.place(Sel { anchor: w.start, head: w.end });
                }
            }),
            C::SelectLine => self.with_active(|p| {
                let line = edit::line_range(p.text(), p.sel().head);
                p.place(Sel { anchor: line.start, head: line.end });
            }),
            C::SelectSpan => self.with_active(|p| {
                if let Some(r) = format::styled_span_at(p.text(), p.doc(), p.sel().head) {
                    p.place(Sel { anchor: r.start, head: r.end });
                }
            }),
            C::ClearFormatting => {
                self.edit_selection_or_region(|text, doc, range, _| Some(format::clear_formatting(text, doc, range)))
            }
            C::ToggleAutoPair => {
                effects.push(Effect::SetPrefs(EditorPrefs { auto_pair: !prefs.auto_pair, ..prefs.clone() }));
                Task::none()
            }
            C::Heading(n) => self.edit_region(|text, doc, region, sel| format::set_heading(text, doc, region, sel, n)),
            C::Paragraph => self.edit_region(|text, doc, region, sel| format::set_heading(text, doc, region, sel, 0)),
            C::HeadingUp => {
                self.edit_region(|text, doc, region, sel| format::step_heading(text, doc, region, sel, true))
            }
            C::HeadingDown => {
                self.edit_region(|text, doc, region, sel| format::step_heading(text, doc, region, sel, false))
            }
            C::Quote => self.edit_blocks(false, |text, _doc, range, sel| Some(format::toggle_quote(text, range, sel))),
            C::OrderedList => {
                self.edit_blocks(true, |text, doc, range, _| Some(format::toggle_list(text, doc, range, true)))
            }
            C::BulletList => {
                self.edit_blocks(true, |text, doc, range, _| Some(format::toggle_list(text, doc, range, false)))
            }
            C::CodeBlock => self.insert_block("```\n", "\n```"),
            C::MathBlock => self.insert_block("$$\n", "\n$$"),
            C::HorizontalRule => self.insert_block("---", ""),
            C::InsertTable => self.insert_block("|  |  |\n| --- | --- |\n|  |  |\n|  |  |", ""),
            C::Bold => self.toggle(InlineStyle::Bold),
            C::Italic => self.toggle(InlineStyle::Italic),
            C::Underline => self.toggle(InlineStyle::Underline),
            C::Code => self.toggle(InlineStyle::Code),
            C::Strikethrough => self.toggle(InlineStyle::Strike),
            C::Highlight => self.toggle(InlineStyle::Highlight),
            C::InlineMath => self.toggle(InlineStyle::Math),
            C::Link => self.edit_selection(format::insert_link),
            C::Copy | C::CopyMarkdown => {
                self.copy(cmd == C::CopyMarkdown, effects);
                Task::none()
            }
            C::Cut => {
                if self.copy(false, effects) {
                    self.delete_selection();
                }
                self.focus()
            }
            C::Paste | C::PastePlain => {
                effects.push(Effect::ReadClipboard(cmd == C::PastePlain));
                Task::none()
            }
            C::Image => {
                let dir = self.page.path().parent().map(Path::to_path_buf);
                Task::perform(
                    async move {
                        let mut dialog = rfd::AsyncFileDialog::new().add_filter("Images", images::IMAGE_EXTENSIONS);
                        if let Some(dir) = dir {
                            dialog = dialog.set_directory(dir);
                        }
                        dialog.pick_files().await.map(|files| files.iter().map(|f| f.path().to_path_buf()).collect())
                    },
                    EditorMsg::ImagesChosen,
                )
            }
            C::Find => self.open_find(false),
            C::Replace => self.open_find(true),
            C::FindNext => self.find_next(false),
            C::FindPrevious => self.find_next(true),
            C::SourceMode => self.toggle_source_mode(),
            C::Outline => {
                self.outline = !self.outline;
                Task::none()
            }
            C::Assets => {
                self.toggle_assets();
                Task::none()
            }
            C::FullScreen => {
                self.fullscreen = !self.fullscreen;
                let mode = if self.fullscreen { window::Mode::Fullscreen } else { window::Mode::Windowed };
                self.window.map_or(Task::none(), |id| window::set_mode(id, mode))
            }
            C::ZoomIn | C::ZoomOut | C::ZoomReset => {
                let zoom = match cmd {
                    C::ZoomIn => zoom_step(ZOOM_STEPS, prefs.zoom, 1),
                    C::ZoomOut => zoom_step(ZOOM_STEPS, prefs.zoom, -1),
                    _ => 1.0,
                };
                effects.push(Effect::SetPrefs(EditorPrefs { zoom, ..prefs.clone() }));
                Task::none()
            }
            C::DarkTheme => {
                effects.push(Effect::SetPrefs(EditorPrefs { dark: !prefs.dark, ..prefs.clone() }));
                Task::none()
            }
            _ => Task::none(),
        }
    }

    /// The save dialog for an export: the page's name with `.html` / `.pdf`, next to it.
    fn ask_export_file(&self, pdf: bool) -> Task<EditorMsg> {
        let (ext, filter) = if pdf { ("pdf", "PDF") } else { ("html", "HTML") };
        let stem = crate::paths::file_stem(self.page.path());
        let dir = self.page.path().parent().map(Path::to_path_buf);
        Task::perform(
            async move {
                let mut dialog =
                    rfd::AsyncFileDialog::new().add_filter(filter, &[ext]).set_file_name(format!("{stem}.{ext}"));
                if let Some(dir) = dir {
                    dialog = dialog.set_directory(dir);
                }
                dialog.save_file().await.map(|f| f.path().to_path_buf())
            },
            move |out| EditorMsg::ExportTo(pdf, out),
        )
    }

    /// Exports the page in the background.
    fn export_to(&mut self, pdf: bool, out: Option<PathBuf>) -> Task<EditorMsg> {
        let Some(out) = out else { return Task::none() };
        let (page, text) = (self.page.path().to_path_buf(), self.page.text().to_string());
        self.status.set(format!("exporting {}", file_name(&out)));
        Task::perform(
            blocking(move || {
                let css = settings::export_css().unwrap_or_else(|| html::DEFAULT_CSS.to_string());
                if pdf {
                    let browser = export::find_browser().ok_or_else(|| export::NO_BROWSER.to_string())?;
                    export::export_pdf(&browser, &page, &text, &out, &css)?;
                } else {
                    export::export_html(&page, &text, &out, &css)?;
                }
                Ok(out)
            }),
            EditorMsg::Exported,
        )
    }

    pub fn save(&mut self) -> Task<EditorMsg> {
        match self.page.save() {
            Ok(()) => self.status.set(format!("saved {}", file_name(self.page.path()))),
            Err(e) => self.dialogs.push_back(EditorDialog::Message(e)),
        }
        Task::none()
    }

    fn close_requested(&mut self, effects: &mut Vec<Effect>) -> Task<EditorMsg> {
        if self.page.is_dirty() {
            if !self.dialogs.contains(&EditorDialog::Unsaved) {
                self.dialogs.push_back(EditorDialog::Unsaved);
            }
        } else {
            effects.push(Effect::Close);
        }
        Task::none()
    }

    fn dialog_answered(&mut self, answer: DialogAnswer, effects: &mut Vec<Effect>) -> Task<EditorMsg> {
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

    // --- Editing helpers ---

    /// Runs `f` with the page when a region is active.
    fn with_active(&mut self, f: impl FnOnce(&mut Page)) -> Task<EditorMsg> {
        if self.page.active().is_none() {
            self.page.place(self.page.sel());
        }
        f(&mut self.page);
        self.focus()
    }

    fn toggle(&mut self, style: InlineStyle) -> Task<EditorMsg> {
        self.edit_selection(|text, sel| format::toggle_inline(text, sel, style))
    }

    fn edit_selection(&mut self, f: impl FnOnce(&str, Sel) -> edit::Edited) -> Task<EditorMsg> {
        if self.source.is_some() {
            let e = f(self.page.text(), self.page.sel());
            self.page.set_text(e.text, e.sel);
            self.refresh_source();
            return self.focus();
        }
        if self.page.active().is_none() || self.page.pending().is_some() {
            return Task::none();
        }
        let e = f(self.page.text(), self.page.sel());
        self.page.apply(e);
        self.focus()
    }

    /// An edit of the active region's block.
    fn edit_region(
        &mut self,
        f: impl FnOnce(&str, &crate::markdown::Document, std::ops::Range<usize>, Sel) -> Option<edit::Edited>,
    ) -> Task<EditorMsg> {
        let Some(a) = self.page.active() else { return Task::none() };
        if self.page.pending().is_some() {
            return Task::none();
        }
        if let Some(e) = edit::nested(self.page.text(), self.page.doc(), a.region.clone(), self.page.sel(), f) {
            self.page.apply(e);
        }
        self.focus()
    }

    /// An edit of the selected blocks, or the active region (inside their
    /// containers when `nested`).
    fn edit_blocks(
        &mut self,
        nested: bool,
        f: impl FnOnce(&str, &crate::markdown::Document, std::ops::Range<usize>, Sel) -> Option<edit::Edited>,
    ) -> Task<EditorMsg> {
        let range = match (self.page.blocks_range(), self.page.active()) {
            (Some(r), _) => r,
            (None, Some(a)) if self.page.pending().is_none() => a.region.clone(),
            _ => return Task::none(),
        };
        self.page.deselect_blocks();
        let (text, doc, sel) = (self.page.text(), self.page.doc(), self.page.sel());
        let e = if nested { edit::nested_blocks(text, doc, range, sel, f) } else { f(text, doc, range, sel) };
        if let Some(e) = e {
            self.page.apply(e);
        }
        self.focus()
    }

    /// An edit of the selection, else of the active region.
    fn edit_selection_or_region(
        &mut self,
        f: impl FnOnce(&str, &crate::markdown::Document, std::ops::Range<usize>, Sel) -> Option<edit::Edited>,
    ) -> Task<EditorMsg> {
        let range = if !self.page.sel().is_empty() {
            self.page.sel().range()
        } else if let Some(r) = self.page.blocks_range() {
            r
        } else if let Some(a) = self.page.active() {
            a.region.clone()
        } else {
            return Task::none();
        };
        self.page.deselect_blocks();
        if let Some(e) = f(self.page.text(), self.page.doc(), range, self.page.sel()) {
            self.page.apply(e);
        }
        self.focus()
    }

    /// Inserts a new block after the active one (or at the end): `open`, the
    /// cursor, `close`.
    fn insert_block(&mut self, open: &str, close: &str) -> Task<EditorMsg> {
        let text = self.page.text();
        let at = match (self.page.active(), self.page.pending()) {
            (Some(_), Some(p)) => p.at,
            (Some(a), None) => a.region.end,
            _ => self.page.blocks_range().map_or(text.len(), |r| r.end),
        };
        let pending = match self.page.pending() {
            Some(p) if self.page.active().is_some() => p.clone(),
            _ => edit::gap_pending(text, self.page.doc(), at, None),
        };
        let (new, start) = pending.materialize(text, &format!("{open}{close}"));
        let cursor = start + pending.prefixed(open).len();
        self.page.deselect_blocks();
        self.page.set_text(new, Sel::caret(cursor));
        self.focus()
    }

    fn select_all(&mut self) -> Task<EditorMsg> {
        if let Some(source) = &mut self.source {
            source.perform(Action::SelectAll);
            return Task::none();
        }
        let whole_region = self.page.active().is_some_and(|a| {
            let s = self.page.sel().range();
            s.start <= a.region.start && s.end >= a.region.end && !a.region.is_empty()
        });
        if self.select_all_again || whole_region || self.page.active().is_none() {
            let last = self.page.doc().blocks.len().saturating_sub(1);
            self.page.select_blocks(0, last);
            self.select_all_again = false;
            return Task::none();
        }
        if let Some(a) = self.page.active() {
            let r = a.region.clone();
            if a.table().is_some() {
                self.page.select_blocks(self.page.active_block().unwrap_or(0), self.page.active_block().unwrap_or(0));
                return Task::none();
            }
            self.page.place(Sel { anchor: r.start, head: r.end });
        }
        self.select_all_again = true;
        self.focus()
    }

    // --- Clipboard and inserting files ---

    /// Copies the selection (Markdown, and HTML unless `markdown_only`).
    /// Returns whether there was something to copy.
    fn copy(&mut self, markdown_only: bool, effects: &mut Vec<Effect>) -> bool {
        // A range of table cells: tab-separated text and an HTML table, or a Markdown table.
        if let Some(a) = self.page.active()
            && let Some(state) = a.table()
            && let Some(range) = state.range
        {
            let t = &state.table;
            let (text, html) = if markdown_only {
                (t.range_markdown(range), None)
            } else {
                (t.range_tsv(range), Some(t.range_html(range, cell_html)))
            };
            self.copied = Some(text.clone());
            effects.push(Effect::Copy { text, html });
            return true;
        }
        let text = match &self.source {
            Some(source) => source.selection().unwrap_or_default(),
            None => self.page.selected_source(),
        };
        if text.is_empty() {
            return false;
        }
        let html = (!markdown_only).then(|| html::fragment(&text));
        self.copied = Some(text.clone());
        effects.push(Effect::Copy { text, html });
        true
    }

    /// Removes what is selected (after a cut).
    fn delete_selection(&mut self) {
        if self.page.selected_blocks().is_some() {
            self.delete_blocks(None);
        } else if let Some(source) = &mut self.source {
            source.perform(Action::Edit(Edit::Delete));
            let text = source.text();
            let sel = content_sel(source, &text);
            self.page.set_source_text(text, sel);
        } else if self.page.active().and_then(|a| a.table()).is_some_and(|t| t.range.is_some()) {
            self.clear_cells();
        } else {
            self.page.perform(Action::Edit(Edit::Delete));
        }
    }

    /// Pastes what the clipboard held: files and images inserted, HTML
    /// converted to Markdown (a table as a table), text as it is (see
    /// `markdown::paste`).
    fn pasted(&mut self, content: ClipboardContent, plain: bool, prefs: &EditorPrefs) -> Task<EditorMsg> {
        let in_code = self.page.active().is_some_and(|a| a.kind.is_code());
        let text = paste::clipboard_text(content.text.as_deref(), content.html.as_deref(), in_code);
        if !plain {
            if !content.files.is_empty() {
                return self.insert_files(content.files, prefs);
            }
            if let Some(image) = content.image.filter(|_| text.trim().is_empty()) {
                return self.insert_image_data(image, prefs);
            }
        }
        match paste::convert(&text, content.html.as_deref(), in_code, plain, self.copied.as_deref()) {
            Paste::Cells(grid) if self.page.active().is_some_and(|a| a.table().is_some()) => {
                self.page.change_table(|t, cell| {
                    t.paste(cell, &grid);
                    cell
                });
            }
            Paste::Cells(grid) => return self.insert_block(&paste::cells_markdown(&grid), ""),
            Paste::Text(text) => self.insert_text(&text),
            Paste::Nothing => {}
        }
        self.focus()
    }

    /// Inserts text at the cursor (in place of the selected blocks).
    fn insert_text(&mut self, text: &str) {
        if self.page.selected_blocks().is_some() {
            let Some(range) = self.page.blocks_range() else { return };
            let new = edit::splice(self.page.text(), range.clone(), text);
            self.page.deselect_blocks();
            self.page.set_text(new, Sel::caret(range.start + text.len()));
            return;
        }
        if let Some(source) = &mut self.source {
            source.perform(Action::Edit(Edit::Paste(Arc::new(text.to_string()))));
            let t = source.text();
            let sel = content_sel(source, &t);
            self.page.set_source_text(t, sel);
            return;
        }
        if self.page.active().is_none() {
            self.page.place(self.page.sel());
        }
        self.page.perform(Action::Edit(Edit::Paste(Arc::new(text.to_string()))));
    }

    /// Copies files into the image folder in the background and inserts them.
    fn insert_files(&mut self, files: Vec<PathBuf>, prefs: &EditorPrefs) -> Task<EditorMsg> {
        if files.is_empty() {
            return Task::none();
        }
        let (page, text, opts) = (self.page.path().to_path_buf(), self.page.text().to_string(), prefs.images.clone());
        Task::perform(blocking(move || images::files_markdown(&page, &text, &files, &opts)), EditorMsg::Inserted)
    }

    /// Saves pasted image data as a PNG in the image folder and inserts it.
    fn insert_image_data(&mut self, image: clipboard::PastedImage, prefs: &EditorPrefs) -> Task<EditorMsg> {
        let (page, text, opts) = (self.page.path().to_path_buf(), self.page.text().to_string(), prefs.images.clone());
        let stamp = chrono::Local::now().format("%Y%m%d%H%M%S").to_string();
        Task::perform(
            blocking(move || {
                let file = images::save_png(&page, &text, &image.rgba, image.width, image.height, &stamp, &opts)?;
                Ok(vec![images::markdown_for(&page, &file, &opts)])
            }),
            EditorMsg::Inserted,
        )
    }

    // --- Source mode ---

    fn toggle_source_mode(&mut self) -> Task<EditorMsg> {
        match self.source.take() {
            Some(content) => {
                let text = content.text();
                let sel = content_sel(&content, &text);
                self.page.forget_active();
                self.page.place(sel);
            }
            None => {
                let sel = self.page.sel();
                self.page.leave_at(sel);
                self.page.deselect_blocks();
                let mut content = Content::with_text(self.page.text());
                move_content(&mut content, self.page.text(), sel);
                self.source = Some(content);
            }
        }
        self.focus()
    }

    /// The source mode's text editor shows the page's text again (after undo or a reload).
    fn refresh_source(&mut self) {
        if let Some(source) = &mut self.source {
            *source = Content::with_text(self.page.text());
            move_content(source, self.page.text(), self.page.sel());
            self.page.forget_active();
        }
    }

    fn leave_source_for_history(&mut self) {
        if let Some(source) = &self.source {
            let text = source.text();
            if text != self.page.text() {
                let sel = content_sel(source, &text);
                self.page.set_source_text(text, sel);
            }
        }
    }

    fn source_bind(&mut self, bind: Bind, prefs: &EditorPrefs) -> Task<EditorMsg> {
        let Some(source) = &mut self.source else { return Task::none() };
        let action = match bind {
            Bind::Enter | Bind::ShiftEnter => Action::Edit(Edit::Enter),
            Bind::Tab => Action::Edit(Edit::Insert('\t')),
            Bind::ShiftTab => return Task::none(),
            Bind::Backspace => Action::Edit(Edit::Backspace),
            Bind::Delete => Action::Edit(Edit::Delete),
            Bind::Up | Bind::ShiftUp => Action::Move(Motion::Up),
            Bind::Down | Bind::ShiftDown => Action::Move(Motion::Down),
            Bind::Left => Action::Move(Motion::Left),
            Bind::Right => Action::Move(Motion::Right),
            Bind::ShiftLeft => Action::Select(Motion::Left),
            Bind::ShiftRight => Action::Select(Motion::Right),
            Bind::Type(c) => {
                let text = source.text();
                let sel = content_sel(source, &text);
                if let Some(e) = prefs.auto_pair.then(|| format::auto_pair(&text, sel, c, false)).flatten() {
                    self.page.set_source_text(e.text, e.sel);
                    self.refresh_source();
                    return Task::none();
                }
                Action::Edit(Edit::Insert(c))
            }
        };
        self.on_action(action)
    }

    // --- Rendered editing ---

    /// A message of the widget editing the active block in its rendered form.
    fn live_message(&mut self, msg: LiveMsg) -> Task<EditorMsg> {
        self.menu = None;
        self.select_all_again = false;
        let page = &mut self.page;
        match msg {
            LiveMsg::Click { at, extend } => page.live_move_to(at, extend),
            LiveMsg::Drag(at) => page.live_move_to(at, true),
            LiveMsg::MoveTo { at, select } => page.live_move_to(at, select),
            LiveMsg::SelectWord(at) => page.live_select_word(at),
            LiveMsg::SelectAll => page.live_select_shown(None),
            LiveMsg::Motion { motion, select } => {
                page.perform(if select { Action::Select(motion) } else { Action::Move(motion) })
            }
            LiveMsg::Insert(text) => {
                let mut chars = text.chars();
                let typed = chars.next().filter(|_| chars.next().is_none());
                page.live_insert(&text, typed);
            }
        }
        // An edit can put the block elsewhere among the widgets (a new
        // paragraph getting its text): the focus follows it.
        self.focus()
    }

    // --- Scrolling ---

    fn scroll_to_block(&self, block: usize) -> Task<EditorMsg> {
        scroll::to_top(self.id(&format!("block-{block}")), self.id("doc"))
    }

    // --- Status bar ---

    /// Words and characters for the status bar.
    pub fn counts(&self) -> (count::Counts, Option<count::Counts>) {
        let all = count::count(&count::shown_text(self.page.doc()));
        let selected = self.page.selected_source();
        let sel = (!selected.is_empty()).then(|| count::count_source(&selected));
        (all, sel)
    }
}

/// A table cell's Markdown as HTML, for copying cells.
fn cell_html(cell: &str) -> String {
    let html = html::fragment(&table::cell_to_text(cell).replace('\n', "  \n"));
    html.trim().trim_start_matches("<p>").trim_end_matches("</p>").to_string()
}

/// See [`Editor::cursor_state`].
#[derive(PartialEq)]
struct CursorState {
    sel: Sel,
    blocks: Option<(usize, usize)>,
    revision: u64,
    region: Option<std::ops::Range<usize>>,
    cell: Option<((usize, usize), Sel)>,
    source: Option<iced::widget::text_editor::Cursor>,
}

/// Where a link in a page leads.
#[derive(Debug, Clone, PartialEq)]
pub enum LinkTarget {
    /// A Markdown file, opened in the editor.
    Page(PathBuf),
    /// A file, directory or URL for the default application.
    External(String),
}

/// A link of the page at `page` resolved: relative paths against the page's directory.
pub fn resolve_link(page: &Path, dest: &str) -> LinkTarget {
    let dest = dest.trim();
    let is_url = dest.contains("://") || dest.starts_with("mailto:");
    if is_url && !dest.starts_with("file:") {
        return LinkTarget::External(dest.to_string());
    }
    let path = crate::markdown::images::local_path(page, dest, None);
    match path {
        Some(p) if p.extension().is_some_and(|e| e.eq_ignore_ascii_case("md")) => LinkTarget::Page(p),
        Some(p) => LinkTarget::External(p.display().to_string()),
        None => LinkTarget::External(dest.to_string()),
    }
}

/// Whether a file opens in the built-in editor (rather than its default application).
pub fn opens_in_editor(path: &Path, builtin: bool) -> bool {
    builtin && !path.is_dir() && path.extension().is_some_and(|e| e.eq_ignore_ascii_case("md"))
}

/// The subscription of the editor windows: while cells, blocks or an item of
/// the assets pane are dragged, the release of the button, wherever it is.
pub fn subscription(editors: &BTreeMap<window::Id, Editor>) -> Subscription<Message> {
    if editors.values().any(|e| e.table_ui.drag || e.block_drag.is_some() || e.asset_pressed()) {
        iced::event::listen_with(|event, _status, window| match event {
            iced::Event::Mouse(iced::mouse::Event::ButtonReleased(iced::mouse::Button::Left)) => {
                Some(Message::Editor(window, EditorMsg::Released))
            }
            _ => None,
        })
    } else {
        Subscription::none()
    }
}
