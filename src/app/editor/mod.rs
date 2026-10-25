//! The page editor: a window per open Markdown file, showing the page as a
//! live preview - every block rendered, but the one with the cursor, which
//! shows its source in a text editor.
//!
//! `page` holds the page and its editing state, `keys` the commands and key
//! bindings, `highlight` the styling of the source, `view` the rendering.

pub mod clipboard;
mod find;
mod highlight;
mod keys;
mod page;
mod render;
mod scroll;
mod tables;
mod view;

#[cfg(test)]
mod tests;

use super::{App, Message, blocking, settings};
use crate::markdown::count;
use crate::markdown::edit::{self, Sel};
use crate::markdown::format::{self, InlineStyle};
use crate::markdown::images::{self, ImageOptions};
use crate::markdown::{export, html, table};
use crate::pkm;
use clipboard::ClipboardContent;
pub use find::FindMsg;
use iced::keyboard::{self, Key, Modifiers, key::Named};
use iced::widget::text_editor::{Action, Content, Edit, Motion};
use iced::widget::{operation, scrollable};
use iced::{Point, Size, Subscription, Task, window};
pub use keys::{Bind, EditorCommand};
use page::Page;
pub use render::{RenderCache, RenderKey};
use std::collections::{BTreeMap, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
pub use tables::TableMsg;
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
    /// A press on the piece of text the pointer is over.
    TextPressed,
    /// A press on a rendered block outside its text.
    BlockPressed(usize),
    /// A press on a cell of the table being edited.
    CellPressed(usize, usize),
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
    fullscreen: bool,
    status: String,
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
    /// A press on a cell started dragging over cells.
    cell_drag: bool,
    /// Where the cells' range ends (the cursor's cell is where it starts).
    range_head: Option<(usize, usize)>,
    /// Where a cell's right-click menu is open.
    cell_menu: Option<Point>,
    /// The pointer in the window, while a table is edited.
    pointer_at: Point,
    /// The table toolbar's body rows and columns as typed.
    resize_fields: (Option<String>, Option<String>),
}

impl Editor {
    pub fn open(path: &Path) -> Result<Editor, String> {
        Ok(Editor::new(Page::open(path)?))
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
            fullscreen: false,
            status: String::new(),
            modifiers: Modifiers::default(),
            pointer: None,
            window_size: DEFAULT_WINDOW,
            viewport: None,
            select_all_again: false,
            rendered: None,
            copied: None,
            last_insert: None,
            find: None,
            cell_drag: false,
            range_head: None,
            cell_menu: None,
            pointer_at: Point::ORIGIN,
            resize_fields: (None, None),
        }
    }

    pub fn title(&self) -> String {
        let name = self.page.path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
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
        // After typing and moving with keys, the cursor stays in view.
        let reveals = matches!(
            msg,
            EditorMsg::Action(_) | EditorMsg::Bind(_) | EditorMsg::Command(_) | EditorMsg::Key(_) | EditorMsg::Find(_)
        );
        let task = self.handle(msg, prefs, effects);
        self.refresh_find();
        if !self.page.active.as_ref().is_some_and(|a| a.table.is_some()) {
            (self.cell_drag, self.range_head, self.cell_menu, self.resize_fields) = (false, None, None, (None, None));
        }
        if reveals { Task::batch([task, self.reveal(prefs)]) } else { task }
    }

    /// Scrolls the page to show the cursor.
    fn reveal(&self, prefs: &EditorPrefs) -> Task<EditorMsg> {
        let (before, kind) = match (&self.source, &self.page.active) {
            (Some(_), _) => (
                self.page.text[..self.page.sel.head.min(self.page.text.len())].to_string(),
                crate::markdown::BlockKind::Paragraph,
            ),
            (None, Some(a)) if a.table.is_none() => {
                let text = &self.page.text[a.region.clone()];
                let at = self.page.sel.head.clamp(a.region.start, a.region.end) - a.region.start;
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
                            .is_some_and(|(t, at)| t.elapsed().as_secs() < 2 && at == self.page.sel.head);
                        let text = format!("{}{}", if recent { "\n" } else { "" }, pieces.join("\n"));
                        self.insert_text(&text);
                        self.last_insert = Some((std::time::Instant::now(), self.page.sel.head));
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
                self.cell_menu = None;
                Task::none()
            }
            EditorMsg::PointerOver(info, p) => {
                self.pointer = Some((info, p));
                Task::none()
            }
            EditorMsg::TextPressed => self.text_pressed(),
            EditorMsg::BlockPressed(block) => self.block_pressed(block),
            EditorMsg::CellPressed(r, c) => self.cell_pressed((r, c)),
            EditorMsg::PressedAt(offset) => {
                self.menu = None;
                self.page.place(Sel::caret(offset));
                self.focus()
            }
            EditorMsg::Link(dest, at) => self.link_clicked(&dest, at, effects),
            EditorMsg::TaskToggled(block) => {
                let start = self.page.doc.blocks.get(block).map(|b| b.range.start);
                if let Some(e) = start.and_then(|s| format::toggle_task(&self.page.text, s)) {
                    let sel = self.page.sel;
                    self.page.apply(e);
                    self.page.deactivate();
                    self.page.sel = sel;
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
            EditorMsg::Table(msg) => self.table_message(msg, effects),
            EditorMsg::ExportTo(pdf, out) => self.export_to(pdf, out),
            EditorMsg::Exported(result) => {
                match result {
                    Ok(out) => self.status = format!("exported {}", out.display()),
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
        if !self.dialogs.is_empty() {
            return match key {
                Key::Named(Named::Enter) => self.dialog_answered(DialogAnswer::Ok, effects),
                _ => Task::none(),
            };
        }
        if let Some(cmd) = keys::command_for_key(&key, physical, modifiers) {
            self.menu = None;
            return self.run(cmd, prefs, effects);
        }
        if self.page.blocks.is_some() {
            return self.blocks_key(&key, modifiers, text.as_deref());
        }
        // Nothing active: arrow keys and typing start at the cursor.
        let starts = matches!(key, Key::Named(Named::ArrowUp | Named::ArrowDown | Named::Enter))
            || text.as_deref().is_some_and(|t| t.chars().any(|c| !c.is_control()));
        if starts && self.page.active.is_none() && self.source.is_none() {
            self.page.place(self.page.sel);
            return self.focus();
        }
        Task::none()
    }

    fn on_escape(&mut self) -> Task<EditorMsg> {
        if !self.dialogs.is_empty() {
            self.dialogs.pop_front();
        } else if self.menu.is_some() || self.cell_menu.is_some() {
            self.menu = None;
            self.cell_menu = None;
        } else if self.find.is_some() {
            return self.close_find();
        } else if self.page.blocks.is_some() {
            self.page.blocks = None;
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
        let Some((anchor, head)) = self.page.blocks else { return Task::none() };
        let last = self.page.doc.blocks.len().saturating_sub(1);
        match key {
            Key::Named(Named::ArrowUp | Named::ArrowDown) if m.shift() => {
                let up = *key == Key::Named(Named::ArrowUp);
                let head = if up { head.saturating_sub(1) } else { (head + 1).min(last) };
                self.page.blocks = Some((anchor, head));
                Task::none()
            }
            Key::Named(Named::ArrowUp) | Key::Named(Named::ArrowLeft) => {
                let first = anchor.min(head);
                self.page.blocks = None;
                self.page.enter_block(first, true, None);
                self.focus()
            }
            Key::Named(Named::ArrowDown) | Key::Named(Named::ArrowRight) => {
                let end = anchor.max(head);
                self.page.blocks = None;
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
        let text = &self.page.text;
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
        self.page.blocks = None;
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
                let sel = source_sel(source, &text);
                self.page.set_source_text(text, sel);
            }
            return Task::none();
        }
        if matches!(action, Action::Click(_)) {
            self.page.blocks = None;
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
        let Some(active) = &self.page.active else { return Task::none() };
        let region = active.region.clone();
        let in_table = active.table.is_some();
        if in_table {
            return self.table_bind(bind);
        }
        let text = self.page.text.clone();
        let sel = self.page.sel;
        let doc = &self.page.doc;
        let pending = self.page.pending.is_some();
        match bind {
            Bind::Enter => {
                if pending {
                    return Task::none();
                }
                match edit::enter(&text, doc, region, sel) {
                    Some(e) => self.page.apply(e),
                    None => self.page.perform(Action::Edit(Edit::Enter)),
                }
            }
            Bind::ShiftEnter => {
                if pending {
                    return Task::none();
                }
                match edit::shift_enter(&text, doc, region, sel) {
                    Some(e) => self.page.apply(e),
                    None => self.page.perform(Action::Edit(Edit::Enter)),
                }
            }
            Bind::Tab | Bind::ShiftTab => {
                if let Some(e) = edit::indent(&text, doc, region, sel, bind == Bind::ShiftTab) {
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
                if !sel.is_empty() {
                    self.page.perform(Action::Edit(Edit::Backspace));
                } else if let Some(e) = prefs.auto_pair.then(|| format::pair_backspace(&text, sel.head)).flatten() {
                    self.page.apply(e);
                } else if let Some(e) = edit::backspace(&text, doc, region, sel.head) {
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
                match (sel.is_empty()).then(|| edit::delete(&text, doc, region, sel.head)).flatten() {
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
                let code = self.page.active.as_ref().is_some_and(|a| a.kind.is_code());
                match prefs.auto_pair.then(|| format::auto_pair(&text, sel, c, code)).flatten() {
                    Some(e) if !pending => self.page.apply(e),
                    _ => self.page.perform(Action::Edit(Edit::Insert(c))),
                }
            }
        }
        self.focus()
    }

    /// Up / Down: within the region, or into the block before / after it.
    fn vertical(&mut self, down: bool) -> Task<EditorMsg> {
        let motion = if down { Motion::Down } else { Motion::Up };
        let before = self.page.active.as_ref().map(|a| a.content.cursor());
        if self.page.pending.is_none() {
            self.page.perform(Action::Move(motion));
        }
        let after = self.page.active.as_ref().map(|a| a.content.cursor());
        if self.page.pending.is_some() || before == after {
            let column = self.page.column();
            if let Some(b) = self.page.neighbor(down) {
                self.page.enter_block(b, down, Some(column));
            }
        }
        self.focus()
    }

    /// Shift+Up / Shift+Down: selects within the region, then whole blocks.
    fn extend_vertically(&mut self, down: bool) -> Task<EditorMsg> {
        let motion = if down { Motion::Down } else { Motion::Up };
        let before = self.page.active.as_ref().map(|a| a.content.cursor());
        if self.page.pending.is_none() {
            self.page.perform(Action::Select(motion));
        }
        let after = self.page.active.as_ref().map(|a| a.content.cursor());
        if self.page.pending.is_some() || before == after {
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
        let Some(a) = &self.page.active else { return Task::none() };
        let sel = self.page.sel;
        let at_edge = sel.is_empty() && if right { sel.head >= a.region.end } else { sel.head <= a.region.start };
        if at_edge || self.page.pending.is_some() {
            if let Some(b) = self.page.neighbor(right) {
                self.page.enter_block(b, right, None);
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
        let offset = view::hit_test(&info, p).or_else(|| self.page.doc.blocks.get(info.block).map(|b| b.range.start));
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
        if let Some(b) = self.page.doc.blocks.get(block) {
            // Beside a list item's text (on its bullet): where its text starts.
            let at = match b.kind {
                crate::markdown::BlockKind::ListItem(_) => {
                    let region = edit::region_at(&self.page.doc, b.range.start).unwrap_or(b.range.clone());
                    edit::content_start(&self.page.text, b.range.start).min(region.end)
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
        let Some(anchor) = self.page.active_block().or(self.page.blocks.map(|b| b.0)) else { return false };
        self.page.select_blocks(anchor, block);
        true
    }

    fn link_clicked(&mut self, dest: &str, at: usize, effects: &mut Vec<Effect>) -> Task<EditorMsg> {
        // A footnote reference goes to its definition.
        if let Some(label) = dest.strip_prefix('^') {
            let block = self
                .page
                .doc
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
            if let Some(h) = self.page.doc.heading_for_anchor(anchor) {
                let index = self.page.doc.headings.iter().position(|x| x.block == h.block).unwrap_or(0);
                return self.go_to_heading(index);
            }
            return Task::none();
        }
        match resolve_link(&self.page.path, dest) {
            LinkTarget::Page(path) => effects.push(Effect::OpenPage(path)),
            LinkTarget::External(target) => effects.push(Effect::OpenExternal(target)),
        }
        Task::none()
    }

    fn go_to_heading(&mut self, index: usize) -> Task<EditorMsg> {
        let Some(h) = self.page.doc.headings.get(index) else { return Task::none() };
        let block = h.block;
        self.page.deactivate();
        self.page.sel = Sel::caret(self.page.doc.blocks[block].range.start);
        self.scroll_to_block(block)
    }

    // --- Commands ---

    fn run(&mut self, cmd: EditorCommand, prefs: &EditorPrefs, effects: &mut Vec<Effect>) -> Task<EditorMsg> {
        use EditorCommand as C;
        self.select_all_again &= cmd == C::SelectAll;
        self.cell_menu = None;
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
                if let Some(w) = format::word_at(&p.text, p.sel.head) {
                    p.place(Sel { anchor: w.start, head: w.end });
                }
            }),
            C::SelectLine => self.with_active(|p| {
                let line = edit::line_range(&p.text, p.sel.head);
                p.place(Sel { anchor: line.start, head: line.end });
            }),
            C::SelectSpan => self.with_active(|p| {
                if let Some(r) = format::styled_span_at(&p.text, &p.doc, p.sel.head) {
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
            C::Quote => self.edit_blocks(|text, _doc, range, sel| Some(format::toggle_quote(text, range, sel))),
            C::OrderedList => self.edit_blocks(|text, doc, range, _| Some(format::toggle_list(text, doc, range, true))),
            C::BulletList => self.edit_blocks(|text, doc, range, _| Some(format::toggle_list(text, doc, range, false))),
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
                let dir = self.page.path.parent().map(Path::to_path_buf);
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
            C::FullScreen => {
                self.fullscreen = !self.fullscreen;
                let mode = if self.fullscreen { window::Mode::Fullscreen } else { window::Mode::Windowed };
                self.window.map_or(Task::none(), |id| window::set_mode(id, mode))
            }
            C::ZoomIn | C::ZoomOut | C::ZoomReset => {
                let i = ZOOM_STEPS.iter().position(|&z| z >= prefs.zoom - 0.001).unwrap_or(3) as i32;
                let zoom = match cmd {
                    C::ZoomIn => ZOOM_STEPS[((i + 1) as usize).min(ZOOM_STEPS.len() - 1)],
                    C::ZoomOut => ZOOM_STEPS[(i - 1).max(0) as usize],
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
        let stem = self.page.path.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
        let dir = self.page.path.parent().map(Path::to_path_buf);
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
        let (page, text) = (self.page.path.clone(), self.page.text.clone());
        self.status = format!("exporting {}", out.display());
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
            Ok(()) => self.status = format!("saved {}", self.page.path.display()),
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
            (EditorDialog::Unsaved, DialogAnswer::Ok) => return Task::none(),
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
                if let Err(e) = self.page.reload() {
                    self.dialogs.push_back(EditorDialog::Message(e));
                }
                self.refresh_source();
            }
            (EditorDialog::ChangedOnDisk, _) => {
                self.dialogs.pop_front();
                // Keep the editor's text: it is what is saved next.
                self.page.modified = std::fs::metadata(&self.page.path).and_then(|m| m.modified()).ok();
            }
            _ => {
                self.dialogs.pop_front();
            }
        }
        Task::none()
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
        if self.page.active.is_none() {
            self.page.place(self.page.sel);
        }
        f(&mut self.page);
        self.focus()
    }

    fn toggle(&mut self, style: InlineStyle) -> Task<EditorMsg> {
        self.edit_selection(|text, sel| format::toggle_inline(text, sel, style))
    }

    fn edit_selection(&mut self, f: impl FnOnce(&str, Sel) -> edit::Edited) -> Task<EditorMsg> {
        if self.source.is_some() {
            let e = f(&self.page.text, self.page.sel);
            self.page.set_text(e.text, e.sel);
            self.refresh_source();
            return self.focus();
        }
        if self.page.active.is_none() || self.page.pending.is_some() {
            return Task::none();
        }
        let e = f(&self.page.text, self.page.sel);
        self.page.apply(e);
        self.focus()
    }

    /// An edit of the active region's block.
    fn edit_region(
        &mut self,
        f: impl FnOnce(&str, &crate::markdown::Document, std::ops::Range<usize>, Sel) -> Option<edit::Edited>,
    ) -> Task<EditorMsg> {
        let Some(a) = &self.page.active else { return Task::none() };
        if self.page.pending.is_some() {
            return Task::none();
        }
        if let Some(e) = f(&self.page.text, &self.page.doc, a.region.clone(), self.page.sel) {
            self.page.apply(e);
        }
        self.focus()
    }

    /// An edit of the selected blocks, or the active region.
    fn edit_blocks(
        &mut self,
        f: impl FnOnce(&str, &crate::markdown::Document, std::ops::Range<usize>, Sel) -> Option<edit::Edited>,
    ) -> Task<EditorMsg> {
        let range = match (self.page.blocks_range(), &self.page.active) {
            (Some(r), _) => r,
            (None, Some(a)) if self.page.pending.is_none() => a.region.clone(),
            _ => return Task::none(),
        };
        self.page.blocks = None;
        if let Some(e) = f(&self.page.text, &self.page.doc, range, self.page.sel) {
            self.page.apply(e);
        }
        self.focus()
    }

    /// An edit of the selection, else of the active region.
    fn edit_selection_or_region(
        &mut self,
        f: impl FnOnce(&str, &crate::markdown::Document, std::ops::Range<usize>, Sel) -> Option<edit::Edited>,
    ) -> Task<EditorMsg> {
        let range = if !self.page.sel.is_empty() {
            self.page.sel.range()
        } else if let Some(r) = self.page.blocks_range() {
            r
        } else if let Some(a) = &self.page.active {
            a.region.clone()
        } else {
            return Task::none();
        };
        self.page.blocks = None;
        if let Some(e) = f(&self.page.text, &self.page.doc, range, self.page.sel) {
            self.page.apply(e);
        }
        self.focus()
    }

    /// Inserts a new block after the active one (or at the end): `open`, the
    /// cursor, `close`.
    fn insert_block(&mut self, open: &str, close: &str) -> Task<EditorMsg> {
        let text = &self.page.text;
        let at = match (&self.page.active, &self.page.pending) {
            (Some(_), Some(p)) => p.at,
            (Some(a), None) => a.region.end,
            _ => self.page.blocks_range().map_or(text.len(), |r| r.end),
        };
        let pending = edit::Pending::gap(text, at);
        let (new, start) = pending.materialize(text, &format!("{open}{close}"));
        let cursor = start + open.len();
        self.page.blocks = None;
        self.page.set_text(new, Sel::caret(cursor));
        self.focus()
    }

    fn select_all(&mut self) -> Task<EditorMsg> {
        if let Some(source) = &mut self.source {
            source.perform(Action::SelectAll);
            return Task::none();
        }
        let whole_region = self.page.active.as_ref().is_some_and(|a| {
            let s = self.page.sel.range();
            s.start <= a.region.start && s.end >= a.region.end && !a.region.is_empty()
        });
        if self.select_all_again || whole_region || self.page.active.is_none() {
            let last = self.page.doc.blocks.len().saturating_sub(1);
            self.page.select_blocks(0, last);
            self.select_all_again = false;
            return Task::none();
        }
        if let Some(a) = &self.page.active {
            let r = a.region.clone();
            if a.table.is_some() {
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
        if let Some(a) = &self.page.active
            && let Some(state) = &a.table
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
        if self.page.blocks.is_some() {
            self.delete_blocks(None);
        } else if let Some(source) = &mut self.source {
            source.perform(Action::Edit(Edit::Delete));
            let text = source.text();
            let sel = source_sel(source, &text);
            self.page.set_source_text(text, sel);
        } else if self.page.active.as_ref().and_then(|a| a.table.as_ref()).is_some_and(|t| t.range.is_some()) {
            self.clear_cells();
        } else {
            self.page.perform(Action::Edit(Edit::Delete));
        }
    }

    /// Pastes what the clipboard held: files and images inserted, HTML
    /// converted to Markdown (a table as a table), text as it is.
    fn pasted(&mut self, content: ClipboardContent, plain: bool, prefs: &EditorPrefs) -> Task<EditorMsg> {
        let text = content.text.clone().unwrap_or_default().replace("\r\n", "\n");
        let in_table = self.page.active.as_ref().is_some_and(|a| a.table.is_some());
        if !plain {
            if !content.files.is_empty() {
                return self.insert_files(content.files, prefs);
            }
            if let Some(image) = content.image.filter(|_| text.trim().is_empty()) {
                return self.insert_image_data(image, prefs);
            }
            if self.copied.as_deref() != Some(text.as_str()) || text.is_empty() {
                // A table (from a spreadsheet or a web page).
                let grid = content
                    .html
                    .as_deref()
                    .and_then(|h| table::parse_html_table(h, |cell| htmd::convert(cell).unwrap_or_default()))
                    .or_else(|| table::parse_tsv(&text))
                    .filter(|g| g.len() > 1 || g.first().is_some_and(|r| r.len() > 1));
                if let Some(grid) = grid {
                    if in_table {
                        self.page.change_table(|t, cell| {
                            t.paste(cell, &grid);
                            cell
                        });
                    } else {
                        let mut t = table::Table::new(
                            grid.len().saturating_sub(1),
                            grid.iter().map(Vec::len).max().unwrap_or(1),
                        );
                        t.paste((0, 0), &grid);
                        return self.insert_block(&t.to_markdown(), "");
                    }
                    return self.focus();
                }
                if let Some(html) = &content.html
                    && let Ok(markdown) = htmd::convert(html)
                    && !markdown.trim().is_empty()
                {
                    self.insert_text(markdown.trim());
                    return self.focus();
                }
            }
        }
        if !text.is_empty() {
            self.insert_text(&text);
        }
        self.focus()
    }

    /// Inserts text at the cursor (in place of the selected blocks).
    fn insert_text(&mut self, text: &str) {
        if self.page.blocks.is_some() {
            let Some(range) = self.page.blocks_range() else { return };
            let new = edit::splice(&self.page.text, range.clone(), text);
            self.page.blocks = None;
            self.page.set_text(new, Sel::caret(range.start + text.len()));
            return;
        }
        if let Some(source) = &mut self.source {
            source.perform(Action::Edit(Edit::Paste(Arc::new(text.to_string()))));
            let t = source.text();
            let sel = source_sel(source, &t);
            self.page.set_source_text(t, sel);
            return;
        }
        if self.page.active.is_none() {
            self.page.place(self.page.sel);
        }
        self.page.perform(Action::Edit(Edit::Paste(Arc::new(text.to_string()))));
    }

    /// Copies files into the image folder in the background and inserts them.
    fn insert_files(&mut self, files: Vec<PathBuf>, prefs: &EditorPrefs) -> Task<EditorMsg> {
        if files.is_empty() {
            return Task::none();
        }
        let (page, text, opts) = (self.page.path.clone(), self.page.text.clone(), prefs.images.clone());
        Task::perform(blocking(move || files_markdown(&page, &text, &files, &opts)), EditorMsg::Inserted)
    }

    /// Saves pasted image data as a PNG in the image folder and inserts it.
    fn insert_image_data(&mut self, image: clipboard::PastedImage, prefs: &EditorPrefs) -> Task<EditorMsg> {
        let (page, text, opts) = (self.page.path.clone(), self.page.text.clone(), prefs.images.clone());
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
                let sel = source_sel(&content, &text);
                self.page.active = None;
                self.page.pending = None;
                self.page.place(sel);
            }
            None => {
                let sel = self.page.sel;
                self.page.deactivate();
                self.page.blocks = None;
                let mut content = Content::with_text(&self.page.text);
                move_source(&mut content, &self.page.text, sel);
                self.page.sel = sel;
                self.source = Some(content);
            }
        }
        self.focus()
    }

    /// The source mode's text editor shows the page's text again (after undo or a reload).
    fn refresh_source(&mut self) {
        if let Some(source) = &mut self.source {
            *source = Content::with_text(&self.page.text);
            move_source(source, &self.page.text, self.page.sel);
            self.page.active = None;
            self.page.pending = None;
        }
    }

    fn leave_source_for_history(&mut self) {
        if let Some(source) = &self.source {
            let text = source.text();
            if text != self.page.text {
                let sel = source_sel(source, &text);
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
                let sel = source_sel(source, &text);
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

    // --- Table keys (filled in by the table editing) ---

    // --- Scrolling ---

    fn scroll_to_block(&self, block: usize) -> Task<EditorMsg> {
        scroll::to_top(self.id(&format!("block-{block}")), self.id("doc"))
    }

    /// Words and characters for the status bar.
    pub fn counts(&self) -> (count::Counts, Option<count::Counts>) {
        let all = count::count(&count::shown_text(&self.page.doc));
        let selected = self.page.selected_source();
        let sel = (!selected.is_empty()).then(|| count::count_source(&selected));
        (all, sel)
    }
}

/// The Markdown inserting files into the page at `page`: images copied into
/// its image folder, other files linked where they are.
pub fn files_markdown(page: &Path, text: &str, files: &[PathBuf], opts: &ImageOptions) -> Result<Vec<String>, String> {
    files
        .iter()
        .map(|f| {
            let target = if images::is_image(f) { images::copy_image(page, text, f, opts)? } else { f.clone() };
            Ok(images::markdown_for(page, &target, opts))
        })
        .collect()
}

/// A table cell's Markdown as HTML, for copying cells.
fn cell_html(cell: &str) -> String {
    let html = html::fragment(&table::cell_to_text(cell).replace('\n', "  \n"));
    html.trim().trim_start_matches("<p>").trim_end_matches("</p>").to_string()
}

/// The selection of the source mode's text editor, as page offsets.
fn source_sel(content: &Content, text: &str) -> Sel {
    let c = content.cursor();
    let head = page::offset_of(text, c.position);
    let anchor = c.selection.map_or(head, |p| page::offset_of(text, p));
    Sel { anchor, head }
}

fn move_source(content: &mut Content, text: &str, sel: Sel) {
    page::move_content(content, text, sel);
}

impl Page {
    /// Takes the source mode's text as the page's.
    pub fn set_source_text(&mut self, text: String, sel: Sel) {
        if text != self.text {
            self.set_text(text, sel);
            self.active = None;
            self.pending = None;
        }
        self.sel = sel;
    }
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

/// The subscription of the editor windows: nothing of their own yet.
pub fn subscription(editors: &BTreeMap<window::Id, Editor>) -> Subscription<Message> {
    // Dragging over cells ends where the button is released, in or out of the table.
    if editors.values().any(|e| e.cell_drag) {
        iced::event::listen_with(|event, _status, window| match event {
            iced::Event::Mouse(iced::mouse::Event::ButtonReleased(iced::mouse::Button::Left)) => {
                Some(Message::Editor(window, EditorMsg::Table(TableMsg::Released)))
            }
            _ => None,
        })
    } else {
        Subscription::none()
    }
}

impl App {
    /// Handles a message of the editor window `id`.
    pub(super) fn editor_message(&mut self, id: window::Id, msg: EditorMsg) -> Task<Message> {
        let focused = matches!(msg, EditorMsg::Focused);
        let Some(editor) = self.editors.get_mut(&id) else { return Task::none() };
        if focused {
            editor.check_disk();
            return Task::none();
        }
        let mut effects = Vec::new();
        let task = editor.update(msg, &self.editor_prefs, &mut effects).map(move |m| Message::Editor(id, m));
        let mut tasks = vec![task, self.request_renders()];
        for effect in effects {
            tasks.push(self.editor_effect(id, effect));
        }
        Task::batch(tasks)
    }

    fn editor_effect(&mut self, id: window::Id, effect: Effect) -> Task<Message> {
        match effect {
            Effect::OpenPage(path) => self.open_page(&path),
            Effect::OpenExternal(target) => {
                if let Err(e) = opener::open(&target)
                    && let Some(editor) = self.editors.get_mut(&id)
                {
                    editor.dialogs.push_back(EditorDialog::Message(format!("failed to open {target}: {e}")));
                }
                Task::none()
            }
            Effect::Close => self.close_editor(id),
            Effect::SetPrefs(prefs) => {
                self.set_editor_prefs(prefs);
                Task::none()
            }
            Effect::Copy { text, html } => {
                if let Ok(mut c) = self.system_clipboard.lock() {
                    c.write(&text, html.as_deref());
                }
                Task::none()
            }
            Effect::ReadClipboard(plain) => {
                let clipboard = self.system_clipboard.clone();
                Task::perform(
                    blocking(move || clipboard.lock().map(|mut c| c.read()).unwrap_or_default()),
                    move |content| Message::Editor(id, EditorMsg::Pasted(content, plain)),
                )
            }
        }
    }

    pub(super) fn set_editor_prefs(&mut self, prefs: EditorPrefs) {
        let old = std::mem::replace(&mut self.editor_prefs, prefs.clone());
        if old.builtin != prefs.builtin {
            settings::set_editor_builtin(prefs.builtin);
        }
        if old.dark != prefs.dark {
            settings::set_editor_dark(prefs.dark);
        }
        if old.zoom != prefs.zoom {
            settings::set_editor_zoom(prefs.zoom);
        }
        if old.auto_pair != prefs.auto_pair {
            settings::set_editor_auto_pair(prefs.auto_pair);
        }
        if old.window != prefs.window {
            settings::set_editor_window(prefs.window);
        }
    }

    pub(super) fn close_editor(&mut self, id: window::Id) -> Task<Message> {
        self.editors.remove(&id);
        window::close(id)
    }

    /// Opens a file: a Markdown file in the built-in editor (bringing its
    /// window to the front when it is open already), anything else - and
    /// everything when the built-in editor is off - with its default application.
    pub(super) fn open_page(&mut self, path: &Path) -> Task<Message> {
        if !opens_in_editor(path, self.editor_prefs.builtin) {
            self.open_in_desktop(path);
            return Task::none();
        }
        self.open_in_editor(path)
    }

    /// Opens a Markdown file in an editor window.
    pub(super) fn open_in_editor(&mut self, path: &Path) -> Task<Message> {
        let key = crate::paths::key(path);
        if let Some((&id, _)) = self.editors.iter().find(|(_, e)| crate::paths::key(&e.page.path) == key) {
            return window::gain_focus(id);
        }
        match Editor::open(path) {
            Ok(mut editor) => {
                editor.window_size = self.editor_prefs.window;
                let (id, opened) = window::open(super::window_settings(self.editor_prefs.window, None, false));
                editor.window = Some(id);
                self.editors.insert(id, editor);
                Task::batch([opened.discard(), self.request_renders()])
            }
            Err(e) => {
                self.message(e);
                Task::none()
            }
        }
    }

    /// The editors of files at or under `path` (a page, or a renamed directory).
    /// The page file or directory of a page or directory node.
    pub(super) fn doc_path_of(&self, id: crate::mindmap::NodeId) -> Option<PathBuf> {
        let map = &self.doc.as_ref()?.map;
        let doc_dir = pkm::load_doc_dir(map).ok()?;
        let name = map.text(id);
        Some(match pkm::doc_type(map, id, &doc_dir)? {
            pkm::DocType::Page => pkm::page_file(&doc_dir, &name),
            pkm::DocType::Directory => pkm::directory_dir(&doc_dir, &name),
        })
    }

    /// Before a PKM command moves, changes or deletes `paths` (files, or
    /// directories with everything in them): their editors' unsaved changes
    /// are saved. The error of the first that can't be.
    pub(super) fn save_editors_of(&mut self, paths: &[PathBuf]) -> Result<(), String> {
        let ids: Vec<window::Id> = paths.iter().flat_map(|p| self.editors_of(p)).collect();
        for id in ids {
            if let Some(editor) = self.editors.get_mut(&id)
                && editor.page.is_dirty()
            {
                editor.page.save()?;
            }
        }
        Ok(())
    }

    /// After `old` (a page file or a directory) was renamed to `new`: its
    /// editors show the files at their new place, with their new text.
    pub(super) fn editors_renamed(&mut self, old: &Path, new: &Path) {
        let depth = old.components().count();
        for id in self.editors_of(old) {
            let Some(editor) = self.editors.get_mut(&id) else { continue };
            let rest: PathBuf = editor.page.path.components().skip(depth).collect();
            let path = if rest.as_os_str().is_empty() { new.to_path_buf() } else { new.join(rest) };
            if let Err(e) = editor.page.moved_to(path) {
                editor.dialogs.push_back(EditorDialog::Message(e));
            }
            editor.refresh_source();
        }
    }

    /// After a PKM command changed `path`: its editors read it again.
    pub(super) fn editors_reload(&mut self, path: &Path) {
        for id in self.editors_of(path) {
            let Some(editor) = self.editors.get_mut(&id) else { continue };
            if let Err(e) = editor.page.reload() {
                editor.dialogs.push_back(EditorDialog::Message(e));
            }
            editor.refresh_source();
        }
    }

    /// After a deletion: the editors of `paths` whose files are gone close.
    pub(super) fn close_deleted_editors(&mut self, paths: &[PathBuf]) -> Task<Message> {
        let ids: Vec<window::Id> = paths.iter().flat_map(|p| self.editors_of(p)).collect();
        let gone: Vec<window::Id> =
            ids.into_iter().filter(|id| self.editors.get(id).is_some_and(|e| !e.page.path.exists())).collect();
        Task::batch(gone.into_iter().map(|id| self.close_editor(id)))
    }

    pub(super) fn editors_of(&self, path: &Path) -> Vec<window::Id> {
        let key = crate::paths::key(path);
        self.editors
            .iter()
            .filter(|(_, e)| {
                let k = crate::paths::key(&e.page.path);
                k == key || k.starts_with(&key) && k[key.len()..].starts_with(std::path::MAIN_SEPARATOR)
            })
            .map(|(&id, _)| id)
            .collect()
    }

    /// Every tick: editors notice files changed on disk.
    pub(super) fn editors_tick(&mut self) -> Task<Message> {
        for editor in self.editors.values_mut() {
            editor.check_disk();
        }
        Task::none()
    }

    /// The names of the pages with unsaved changes.
    pub(super) fn unsaved_pages(&self) -> Vec<String> {
        self.editors
            .values()
            .filter(|e| e.page.is_dirty())
            .map(|e| e.page.path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default())
            .collect()
    }

    /// Saves every page with unsaved changes; the errors of those that fail.
    pub(super) fn save_all_pages(&mut self) -> Vec<String> {
        let mut errors = Vec::new();
        for editor in self.editors.values_mut() {
            if editor.page.is_dirty()
                && let Err(e) = editor.page.save()
            {
                errors.push(e);
            }
        }
        errors
    }
}
