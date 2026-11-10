//! The page editor: a window per open Markdown file, showing the page as a
//! live preview - every block rendered, the one with the cursor edited in its
//! rendered form (code-like blocks as source in a text editor).
//!
//! `page` holds the page and its editing state, `live_text` the widget
//! editing rendered text, `keys` the commands and key bindings, `highlight`
//! the styling of the source, `view` the rendering; `tables`, `find` and
//! `assets` handle the table being edited, the find bar and the assets pane.
//!
//! Here: the `Editor` - an editor window's state - and its update loop. Its
//! messages, dialogs and effects on the application are in `message`, the
//! settings the windows share in `prefs`. What the update loop does lives
//! with its topic: keys and the active region's key bindings in `input`,
//! presses and right clicks on the page in `pointer`, the commands in
//! `commands` and the edits they share in `editing`, the clipboard and
//! inserted files in `copy_paste`, the source mode in `source_mode`, and the
//! page's file - saving, closing, changes on disk - with the dialogs'
//! answers in `file`.

mod assets;
pub mod clipboard;
mod column;
mod commands;
mod copy_paste;
mod decoration;
mod editing;
mod file;
mod find;
mod flow;
mod highlight;
mod input;
mod keys;
mod live_text;
mod message;
mod modal;
mod near;
mod outline;
mod page;
mod pointer;
mod prefs;
mod press_relay;
mod render;
mod scroll;
mod source_mode;
mod tables;
mod view;

#[cfg(test)]
mod tests;

use super::scroll_watch::Visible;
use super::{Message, Status};
use crate::app::file::file_name;
use crate::markdown::count;
use crate::markdown::edit::Sel;
use crate::markdown::{format, images};
pub use assets::AssetsMsg;
pub use find::FindMsg;
use iced::keyboard::{self, Modifiers};
use iced::widget::operation;
use iced::widget::text_editor::{Action, Content};
use iced::{Point, Size, Subscription, Task, window};
pub use keys::{Bind, EditorCommand};
pub use live_text::LiveMsg;
pub use message::{DialogAnswer, EditorDialog, EditorMsg, Effect, RightClick};
use page::Page;
use prefs::DEFAULT_WINDOW;
pub use prefs::EditorPrefs;
pub use render::{RenderCache, RenderKey};
use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
pub use tables::TableMsg;
use tables::TableUi;
use view::HitInfo;
pub use view::code_font;

static NEXT_UID: AtomicU64 = AtomicU64::new(1);

/// The status bar's counts, as last counted: the page's for a revision of
/// it (and its blocks', so that an edit counts only the blocks it changed),
/// the selection's for its source.
#[derive(Default)]
struct Counted {
    page: Option<(u64, count::Counts)>,
    blocks: count::BlockCounts,
    selection: Option<(String, count::Counts)>,
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
    /// The anchors of the headings folded or unfolded against the outline's default.
    outline_toggled: HashSet<String>,
    /// The assets pane, when shown.
    assets: Option<assets::AssetsPane>,
    fullscreen: bool,
    /// The status bar message.
    pub(super) status: Status,
    modifiers: Modifiers,
    /// The rendered text under the pointer and the pointer's position in it.
    pointer: Option<(Arc<HitInfo>, Point)>,
    window_size: Size,
    decoration_grid: decoration::Grid,
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
    /// Where the left button last went down on the page, in the page.
    press_at: Option<Point>,
    /// The pointer in the window, while a table is edited or the assets pane
    /// is shown (where a dragged item is).
    pointer_at: Point,
    /// Where the right button last went down, in the window.
    right_pressed_at: Point,
    /// Where the page's right-click menu is open.
    context_menu: Option<Point>,
    /// Whether the image files the page shows exist, as looked up since the
    /// window last got the focus (`is_image_file`).
    image_files: RefCell<HashMap<PathBuf, bool>>,
    /// The images the page holds in `data:` URLs, decoded (`data_image`).
    data_images: RefCell<DataImages>,
    /// The status bar's counts, as last counted.
    counted: RefCell<Counted>,
    /// The entries of the page column, as last made.
    entries: RefCell<column::Entries>,
    /// The heights of the page's blocks as last laid out, for those that
    /// get no widgets.
    heights: RefCell<near::Heights>,
    /// Some blocks whose height is not known are left to be measured with
    /// the next messages (as the page last showed), and the page asks for
    /// one when it is drawn.
    unmeasured: Cell<bool>,
    /// How many of the page's blocks had a known height when it was last
    /// shown (none yet: `usize::MAX`): no more since, no further batch is
    /// asked for.
    known: Cell<usize>,
    /// Where the page's blocks were last laid out.
    placement: RefCell<view::Placement>,
    /// The block at the top of the visible part as last laid out or told,
    /// and how far below its top the visible part starts.
    top_entry: Cell<Option<(column::EntryKey, f32)>>,
    /// The visible part of the page, as last told: the blocks near it get
    /// widgets (none told yet: the top of the page).
    visible: Option<Visible>,
    /// A block the page is scrolled to, which gets widgets wherever it is
    /// until the page tells where it is again.
    scroll_target: Option<usize>,
    /// The widths of the lines of table cells, as measured for their columns.
    table_widths: view::WidthCache,
    /// How far the table that was last scrolled sideways is scrolled: where
    /// it starts in the source and its offset (kept when it is clicked).
    table_x: Option<(usize, f32)>,
}

/// An image a page holds in a `data:` URL, made into a handle once.
#[derive(Debug, Clone)]
pub(super) enum DataHandle {
    Raster(iced::widget::image::Handle),
    Svg(iced::widget::svg::Handle),
    /// It holds no image, or cannot be decoded: why.
    Invalid(String),
}

/// The handles of the `data:` URL images, by a hash of the URL, and those
/// shown since the page's last change.
#[derive(Default)]
struct DataImages {
    revision: u64,
    used: std::collections::HashSet<u64>,
    handles: HashMap<u64, DataHandle>,
}

impl Editor {
    /// An editor of the page at `path`, for a window of the size `prefs`
    /// remember, showing the outline and the assets pane if they say so.
    pub fn open(path: &Path, prefs: &EditorPrefs) -> Result<Editor, String> {
        Ok(Editor {
            window_size: prefs.window,
            outline: prefs.outline,
            assets: prefs.assets.then(assets::AssetsPane::new),
            ..Editor::new(Page::open(path)?)
        })
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
            outline_toggled: HashSet::new(),
            assets: None,
            fullscreen: false,
            status: Status::default(),
            modifiers: Modifiers::default(),
            pointer: None,
            window_size: DEFAULT_WINDOW,
            decoration_grid: decoration::Grid::default(),
            select_all_again: false,
            rendered: None,
            copied: None,
            last_insert: None,
            find: None,
            table_ui: TableUi::default(),
            block_drag: None,
            press_at: None,
            pointer_at: Point::ORIGIN,
            right_pressed_at: Point::ORIGIN,
            context_menu: None,
            image_files: RefCell::default(),
            data_images: RefCell::default(),
            counted: RefCell::default(),
            entries: RefCell::default(),
            heights: RefCell::default(),
            unmeasured: Cell::new(false),
            known: Cell::new(usize::MAX),
            placement: RefCell::default(),
            top_entry: Cell::new(None),
            visible: None,
            scroll_target: None,
            table_widths: view::WidthCache::default(),
            table_x: None,
        }
    }

    /// Whether `path`, an image the page shows, is an existing file: looked
    /// up once rather than in every view or with every edit, and again after
    /// the window got the focus, with the tick, after files were inserted or
    /// the assets pane was read (files may have changed meanwhile).
    fn is_image_file(&self, path: &Path) -> bool {
        *self.image_files.borrow_mut().entry(path.to_path_buf()).or_insert_with(|| path.is_file())
    }

    /// The image a `data:` URL `src` of the page holds (None when `src` is
    /// no `data:` URL). The handle is made once and kept, as iced decodes an
    /// image again for every new handle: kept while the page has the image -
    /// when the page changes, the handles not shown since the last change go.
    fn data_image(&self, src: &str) -> Option<DataHandle> {
        use std::hash::{Hash, Hasher};
        if !images::is_data_url(src) {
            return None;
        }
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        src.hash(&mut hasher);
        let key = hasher.finish();
        let mut cache = self.data_images.borrow_mut();
        let revision = self.page.revision();
        if cache.revision != revision {
            let used = std::mem::take(&mut cache.used);
            cache.handles.retain(|k, _| used.contains(k));
            cache.revision = revision;
        }
        cache.used.insert(key);
        let handle = cache.handles.entry(key).or_insert_with(|| {
            match images::data_image(src).unwrap_or_else(|| Err(String::new())) {
                Ok(data) if data.svg => DataHandle::Svg(iced::widget::svg::Handle::from_memory(data.bytes)),
                Ok(data) => DataHandle::Raster(iced::widget::image::Handle::from_bytes(data.bytes)),
                Err(e) => DataHandle::Invalid(e),
            }
        });
        Some(handle.clone())
    }

    /// Looks the images up again when they are next shown.
    pub(super) fn forget_image_files(&mut self) {
        self.image_files.get_mut().clear();
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
        let rendered_press = self.source.is_none()
            && matches!(msg, EditorMsg::TextPressed | EditorMsg::BlockPressed(_) | EditorMsg::TableCellPressed(..));
        // Typing, a command or the menu bar closes the right-click menu.
        if matches!(
            msg,
            EditorMsg::Action(_)
                | EditorMsg::Bind(_)
                | EditorMsg::Live(_)
                | EditorMsg::Command(_)
                | EditorMsg::Menu(_)
                | EditorMsg::Key(keyboard::Event::KeyPressed { .. })
        ) {
            self.context_menu = None;
        }
        if matches!(msg, EditorMsg::Assets(AssetsMsg::Read(..))) {
            self.forget_image_files();
        }
        let before = self.cursor_state();
        let table_before = self.active_table_start();
        let revision = self.page.revision();
        let shown = self.status.text().to_string();
        let task = self.handle(msg, prefs, effects);
        if press {
            self.block_drag = self.page.active_block().or(self.page.selected_blocks().map(|b| b.0));
        }
        // A press on a rendered table: dragging onto other cells selects them.
        if rendered_press && self.page.active().is_some_and(|a| a.table().is_some()) {
            self.table_ui.drag = true;
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
        let task = if reveals { Task::batch([task, self.reveal(prefs)]) } else { task };
        // A table that scrolls sideways shows the cursor's cell; one that was
        // rendered until now is scrolled as far as it was then.
        let table_after = self.active_table_start();
        match table_after {
            Some(start) if table_after != table_before || reveals => {
                let from = self.table_x.filter(|t| t.0 == start && table_after != table_before).map(|t| t.1);
                Task::batch([task, scroll::reveal_cell(self.id("active-cell"), self.id("table-scroll"), from)])
            }
            _ => task,
        }
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

    /// Where the table being edited starts in the source.
    fn active_table_start(&self) -> Option<usize> {
        self.page.active().filter(|a| a.table().is_some()).map(|a| a.region.start)
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
            EditorMsg::Rescaled(scale) => {
                self.decoration_grid.scale.set(scale);
                Task::none()
            }
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
            EditorMsg::LinkClipboard(text) => {
                let dest = text.as_deref().and_then(format::link_destination);
                self.edit_selection(|t, sel| format::insert_link(t, sel, dest.as_deref()))
            }
            EditorMsg::ImagesChosen(files) => self.insert_files(files.unwrap_or_default(), prefs),
            EditorMsg::Inserted(result) => {
                self.forget_image_files();
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
            EditorMsg::PressPoint(p) => {
                self.press_at = Some(p);
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
                (self.table_ui.drag, self.block_drag, self.press_at) = (false, None, None);
                self.assets_released(prefs)
            }
            EditorMsg::CodeLanguage(language) => {
                self.page.set_code_language(&language);
                Task::none()
            }
            EditorMsg::CellPressed(r, c) => self.cell_pressed((r, c)),
            EditorMsg::TableCellPressed(block, r, c) => self.table_cell_pressed(block, (r, c)),
            EditorMsg::TableScrolled(start, x) => {
                self.table_x = Some((start, x));
                Task::none()
            }
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
            EditorMsg::OutlineFoldToggled(i) => {
                if let Some(h) = self.page.doc().headings.get(i) {
                    outline::toggle(&mut self.outline_toggled, h);
                }
                Task::none()
            }
            EditorMsg::RightPressedAt(p) => {
                self.right_pressed_at = p;
                Task::none()
            }
            EditorMsg::RightClicked(target) => self.right_clicked(target),
            EditorMsg::CloseContextMenu => {
                self.context_menu = None;
                Task::none()
            }
            EditorMsg::Measure => Task::none(),
            EditorMsg::PageScrolled(visible) => {
                (self.visible, self.scroll_target) = (Some(visible), None);
                self.note_top(visible);
                Task::none()
            }
            EditorMsg::Dialog(answer) => self.dialog_answered(answer, effects),
            EditorMsg::Find(msg) => self.find_message(msg),
            EditorMsg::Live(msg) => self.live_message(msg),
            EditorMsg::Table(msg) => self.table_message(msg, effects),
            EditorMsg::Assets(msg) => self.assets_message(msg, effects),
            EditorMsg::ExportTo(pdf, out) => self.export_to(pdf, out),
            EditorMsg::OpenChosen(path) => {
                effects.extend(path.map(Effect::OpenInEditor));
                Task::none()
            }
            EditorMsg::Exported(result) => {
                match result {
                    Ok(out) => self.status.set(format!("exported {}", file_name(&out))),
                    Err(e) => self.dialogs.push_back(EditorDialog::Message(e)),
                }
                Task::none()
            }
        }
    }

    // --- Scrolling ---

    /// Scrolls the page to put `block` at the top: it gets widgets wherever
    /// it is, for the scroll to find it.
    fn scroll_to_block(&mut self, block: usize) -> Task<EditorMsg> {
        self.scroll_target = Some(block);
        scroll::to_top(self.id(&format!("block-{block}")), self.id("doc"))
    }

    /// The visible part of the page, as last told.
    fn visible(&self) -> Visible {
        let (height, width) = (self.window_size.height, self.window_size.width);
        self.visible.unwrap_or(Visible { top: 0.0, height, left: 0.0, width })
    }

    /// Whether some blocks whose height is not known are left to be
    /// measured: a block's widgets measure it, and those away from the visible
    /// part get them a few at a time, with every view (which asks for the
    /// next message when it is drawn).
    #[cfg(test)]
    pub fn measuring(&self) -> bool {
        self.unmeasured.get()
    }

    /// Math or diagrams were rendered: the blocks that may show them are
    /// measured again.
    pub(super) fn renders_came_in(&mut self) {
        self.heights.get_mut().rendered();
    }

    // --- Status bar ---

    /// Words and characters for the status bar: the page's, and the
    /// selection's if there is one. Counted again only when the page or the
    /// selection changed, not in every view - and then only the blocks that
    /// changed.
    pub fn counts(&self) -> (count::Counts, Option<count::Counts>) {
        let mut counted = self.counted.borrow_mut();
        let revision = self.page.revision();
        if counted.page.is_none_or(|(r, _)| r != revision) {
            let page = counted.blocks.count(self.page.doc());
            counted.page = Some((revision, page));
        }
        let all = counted.page.map(|(_, c)| c).unwrap_or_default();
        let selected = self.page.selected_source();
        if selected.is_empty() {
            return (all, None);
        }
        if counted.selection.as_ref().is_none_or(|(s, _)| *s != selected) {
            let c = count::count_source(&selected);
            counted.selection = Some((selected, c));
        }
        (all, counted.selection.as_ref().map(|(_, c)| *c))
    }
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
    builtin && is_markdown(path)
}

/// Whether a file is a Markdown file, which the built-in editor edits: a
/// `.md` file (in any case) that is not a directory.
pub fn is_markdown(path: &Path) -> bool {
    !path.is_dir() && path.extension().is_some_and(|e| e.eq_ignore_ascii_case("md"))
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
