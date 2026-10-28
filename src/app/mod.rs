//! The application: state, messages and their handling.
//!
//! It has one map window and an editor window per open page (`editor`).
//! Window events carry the window's id and are routed to the map or to the
//! editor of that window first. `App::update` handles a map message, then
//! recomputes the layout. The handling itself lives with its topic: documents
//! (`file`), dialogs (`dialog`), commands (`command`), keyboard input (`keys`),
//! scrolling and zooming (`viewport`) and the search panel (`search_panel`);
//! `doc` holds an open map and its edits.

mod arrow_cursor;
mod command;
mod ctrl_wheel;
mod dialog;
mod doc;
mod drag;
mod editor;
mod file;
mod ime;
mod keys;
mod layout;
mod search_panel;
pub mod settings;
mod view;
mod viewport;

#[cfg(test)]
mod tests;

use crate::mindmap::NodeId;
use crate::pkm::next_steps;
use command::{Clipboard, Command};
use dialog::Dialog;
use doc::Doc;
use drag::NodeDrag;
use editor::{Editor, EditorMsg, EditorPrefs, RenderCache, RenderKey};
use file::{AfterUnsaved, FileAction};
use iced::keyboard::{self, Key, Modifiers, key::Named};
use iced::widget::scrollable;
use iced::{Point, Size, Subscription, Task, Vector, window};
use search_panel::{PanelFocus, SearchMsg, SearchPanel};
use settings::WindowGeometry;
use std::collections::{BTreeMap, HashMap, VecDeque};
use std::path::PathBuf;
use std::time::{Duration, Instant};
use viewport::Pan;

/// The periodic Next Steps / search index refresh runs at most this often, and
/// only after the map changed.
const UPDATE_INTERVAL: Duration = Duration::from_secs(5 * 60);
const TICK: Duration = Duration::from_secs(20);
/// How long a status bar message is shown at least (it goes on the first
/// message, or tick, after that).
const STATUS_TIME: Duration = Duration::from_secs(10);
/// The window when none is remembered.
const INITIAL_WINDOW: WindowGeometry =
    WindowGeometry { size: Size::new(1280.0, 800.0), position: None, maximized: false };
const MAP_SCROLL_ID: &str = "map";
const DIALOG_INPUT_ID: &str = "dialog-input";

pub fn run(initial: Option<PathBuf>) -> iced::Result {
    let geometry = settings::window().unwrap_or(INITIAL_WINDOW);
    iced::daemon(move || App::boot(initial.clone(), geometry), App::update, App::view)
        .title(App::title)
        .subscription(App::subscription)
        .theme(App::theme)
        .default_font(view::APP_FONT)
        .run()
}

/// The settings of a window of the application: closing it is up to `update`.
fn window_settings(size: Size, position: Option<Point>, maximized: bool) -> window::Settings {
    window::Settings {
        size,
        position: position.map_or(window::Position::Default, window::Position::Specific),
        maximized,
        icon: window_icon(),
        exit_on_close_request: false,
        ..Default::default()
    }
}

/// The window (title bar / taskbar) icon, drawn by `tools/make_icon.py`.
fn window_icon() -> Option<window::Icon> {
    const SIZE: u32 = 64;
    window::icon::from_rgba(include_bytes!("../../assets/icon-64.rgba").to_vec(), SIZE, SIZE).ok()
}

/// Runs blocking work (file I/O, indexing) on the blocking thread pool.
pub(crate) async fn blocking<T: Send + 'static>(f: impl FnOnce() -> T + Send + 'static) -> T {
    tokio::task::spawn_blocking(f).await.expect("background task panicked")
}

#[derive(Debug, Clone)]
pub enum Message {
    /// A key no widget took, in a window.
    KeyIn(window::Id, keyboard::Event),
    /// Escape in a window (also when a text box has it).
    EscapeIn(window::Id),
    /// A window was resized, moved, asked to close, got a dropped file, got the focus.
    Resized(window::Id, Size),
    Moved(window::Id, Point),
    CloseRequest(window::Id),
    FileDropped(window::Id, PathBuf),
    Focused(window::Id),
    /// A message of an editor window.
    Editor(window::Id, EditorMsg),
    /// Math or a diagram rendered in the background.
    Rendered(RenderKey, Result<String, String>),
    // The map window's messages.
    Key(keyboard::Event),
    Escape,
    Command(Command),
    Menu(Option<usize>),
    NodeClicked(NodeId),
    NodeDoubleClicked(NodeId),
    LinkClicked(NodeId),
    FoldClicked(NodeId),
    MapScrolled(scrollable::Viewport),
    /// The wheel was turned with Ctrl held over the map, at this point of the viewport.
    ZoomWheel(iced::mouse::ScrollDelta, iced::Point),
    /// The left button went down on empty map space: start panning.
    PanStart,
    /// The pointer moved while panning (window coordinates).
    PanMove(iced::Point),
    PanEnd,
    /// The pointer moved while a node drag is armed (map coordinates).
    NodeDragMove(iced::Point),
    /// The button was released: drop the dragged nodes.
    NodeDragEnd,
    NodeDragCancel,
    WindowResized(Size),
    /// The window moved to this position on the screen.
    WindowMoved(Point),
    /// Whether the window is maximized or minimized, asked after it was resized or moved.
    WindowState {
        maximized: bool,
        minimized: bool,
    },
    CloseRequested,
    Tick,
    DialogInput(String),
    DialogOk,
    DialogCancel,
    /// Unsaved changes: save first.
    DialogSave,
    /// Unsaved changes: discard them.
    DialogDiscard,
    FileChosen(FileAction, Option<PathBuf>),
    NextStepsComputed(u64, Box<next_steps::Update>),
    IndexUpdated(Result<(), String>),
    ClipboardRead(Option<String>),
    Search(SearchMsg),
}

impl Message {
    /// Whether handling it may change the map or what is shown of it, so that
    /// the layout is recomputed afterwards.
    fn may_change_layout(&self) -> bool {
        !matches!(
            self,
            Message::MapScrolled(_)
                | Message::ZoomWheel(..)
                | Message::Tick
                | Message::Menu(_)
                | Message::PanStart
                | Message::PanMove(_)
                | Message::PanEnd
                | Message::NodeDragMove(_)
                | Message::NodeDragCancel
                | Message::WindowMoved(_)
                | Message::WindowState { .. }
        )
    }

    /// Whether it is the user acting on the map, after which the focused node
    /// is scrolled into view.
    fn reveals_focus(&self) -> bool {
        matches!(
            self,
            Message::Key(_)
                | Message::Escape
                | Message::Command(_)
                | Message::Search(_)
                | Message::DialogOk
                | Message::FileChosen(..)
        )
    }
}

pub struct App {
    main_window: window::Id,
    /// The page editor windows.
    editors: BTreeMap<window::Id, Editor>,
    editor_prefs: EditorPrefs,
    /// The math and diagrams of open pages, rendered.
    renders: RenderCache,
    /// The clipboard with rich text, images and files, for the editors.
    system_clipboard: std::sync::Arc<std::sync::Mutex<Box<dyn editor::clipboard::ClipboardBackend>>>,
    doc: Option<Doc>,
    /// The generation given to the last opened document.
    next_generation: u64,
    /// The queued dialogs; the first one is shown.
    dialogs: VecDeque<Dialog>,
    search: SearchPanel,
    /// The open menu: an index into `view::MENUS`.
    menu: Option<usize>,
    modifiers: Modifiers,
    window_size: Size,
    /// The window's position as last reported.
    window_position: Option<Point>,
    /// The window as it is to be restored on the next start.
    window: WindowGeometry,
    /// The map's scroll position, as last scrolled to or reported.
    scroll_offset: Vector,
    /// Room around the map beyond its margin, for centering and panning.
    room: viewport::Room,
    /// Set when a message placed the view itself (adding room around the
    /// map): the focused node is not kept in place nor revealed after it.
    view_placed: bool,
    /// Set by a move among siblings: the parent stays in place on screen
    /// after the message, not the moved node.
    keep_parent_in_place: bool,
    /// The size of the map's scrollable as last reported; None when it may have changed.
    viewport_size: Option<Size>,
    /// The map's zoom level, one of `viewport::ZOOM_STEPS`.
    zoom: f32,
    /// Ctrl+wheel movement not yet turned into a zoom step, in notches.
    wheel_zoom: f32,
    /// Node text sizes at the current zoom, by text.
    measure_cache: HashMap<String, Size>,
    /// The drawn edges, cleared on every relayout.
    edges: iced::widget::canvas::Cache,
    clipboard: Option<Clipboard>,
    /// The status bar message.
    status: Status,
    pan: Option<Pan>,
    /// A node drag, armed by a press on a selected node.
    drag: Option<NodeDrag>,
}

impl App {
    fn boot(initial: Option<PathBuf>, window: WindowGeometry) -> (Self, Task<Message>) {
        let mut app = App::new();
        app.editor_prefs = EditorPrefs::load();
        app.window_size = window.size;
        app.window_position = window.position;
        app.window = window;
        let (id, opened) = window::open(window_settings(window.size, window.position, window.maximized));
        app.main_window = id;
        let explicit = initial.is_some();
        let task = match initial.or_else(settings::last_file) {
            Some(path) if explicit || path.exists() => app.open_path(path),
            _ => Task::none(),
        };
        (app, Task::batch([opened.discard(), task]))
    }

    /// The application without a map, before anything is opened.
    fn new() -> Self {
        App {
            main_window: window::Id::unique(),
            editors: BTreeMap::new(),
            editor_prefs: EditorPrefs::default(),
            renders: RenderCache::default(),
            system_clipboard: std::sync::Arc::new(std::sync::Mutex::new(editor::clipboard::default_backend())),
            doc: None,
            next_generation: 0,
            dialogs: VecDeque::new(),
            search: SearchPanel::default(),
            menu: None,
            modifiers: Modifiers::default(),
            window_size: INITIAL_WINDOW.size,
            window_position: None,
            window: INITIAL_WINDOW,
            scroll_offset: Vector::ZERO,
            room: viewport::Room::default(),
            view_placed: false,
            keep_parent_in_place: false,
            viewport_size: None,
            zoom: 1.0,
            wheel_zoom: 0.0,
            measure_cache: HashMap::new(),
            edges: Default::default(),
            clipboard: None,
            status: Status::default(),
            pan: None,
            drag: None,
        }
    }

    fn title(&self, window: window::Id) -> String {
        if let Some(editor) = self.editors.get(&window) {
            return editor.title();
        }
        match &self.doc {
            Some(doc) => {
                let name = doc.map.path().and_then(|p| p.file_name()).map(|n| n.to_string_lossy().into_owned());
                format!("{}{} - Mind PKM", name.unwrap_or_else(|| "Untitled".into()), if doc.dirty { " *" } else { "" })
            }
            None => "Mind PKM".into(),
        }
    }

    fn theme(&self, window: window::Id) -> iced::Theme {
        if self.editors.contains_key(&window) && self.editor_prefs.dark {
            iced::Theme::Dark
        } else {
            iced::Theme::Light
        }
    }

    fn subscription(&self) -> Subscription<Message> {
        Subscription::batch([
            // Keys no widget took, as `keyboard::listen` would give them, with their window.
            iced::event::listen_with(|event, status, window| match (event, status) {
                (iced::Event::Keyboard(event), iced::event::Status::Ignored) => Some(Message::KeyIn(window, event)),
                _ => None,
            }),
            // Escape also when a text input has it (closing a dialog from its input).
            iced::event::listen_with(|event, _status, window| match event {
                iced::Event::Keyboard(keyboard::Event::KeyPressed { key: Key::Named(Named::Escape), .. }) => {
                    Some(Message::EscapeIn(window))
                }
                iced::Event::Window(window::Event::Moved(position)) => Some(Message::Moved(window, position)),
                iced::Event::Window(window::Event::Resized(size)) => Some(Message::Resized(window, size)),
                iced::Event::Window(window::Event::FileDropped(path)) => Some(Message::FileDropped(window, path)),
                iced::Event::Window(window::Event::Focused) => Some(Message::Focused(window)),
                _ => None,
            }),
            window::close_requests().map(Message::CloseRequest),
            iced::time::every(TICK).map(|_| Message::Tick),
            // Pointer tracking only while panning, so ordinary moves cost nothing.
            if self.pan.is_some() {
                iced::event::listen_with(|event, _status, _window| match event {
                    iced::Event::Mouse(iced::mouse::Event::CursorMoved { position }) => {
                        Some(Message::PanMove(position))
                    }
                    iced::Event::Mouse(iced::mouse::Event::ButtonReleased(iced::mouse::Button::Left))
                    | iced::Event::Mouse(iced::mouse::Event::CursorLeft) => Some(Message::PanEnd),
                    _ => None,
                })
            } else {
                Subscription::none()
            },
            if self.drag.is_some() { drag::drag_events() } else { Subscription::none() },
            editor::subscription(&self.editors),
        ])
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        self.status.expire();
        for editor in self.editors.values_mut() {
            editor.status.expire();
        }
        let message = match self.route(message) {
            Ok(message) => message,
            Err(task) => return task,
        };
        if !message.may_change_layout() {
            return self.handle(message);
        }
        let reveal = message.reveals_focus();
        // Keep the focused node (or, after a move among siblings, its parent)
        // in place on screen when the layout shifts around it.
        let (focus_anchor, parent_anchor) = (self.focus_anchor(false), self.focus_anchor(true));
        let task = self.handle(message);
        self.relayout();
        let anchor = if std::mem::take(&mut self.keep_parent_in_place) { parent_anchor } else { focus_anchor };
        if std::mem::take(&mut self.view_placed) {
            return task;
        }
        let mut tasks = vec![task, self.keep_in_place(anchor)];
        if reveal {
            tasks.push(self.reveal_focus());
        }
        Task::batch(tasks)
    }

    /// Window events of the map window as its own messages; those of editor
    /// windows (and editor messages) handled here (`Err` with their task).
    fn route(&mut self, message: Message) -> Result<Message, Task<Message>> {
        let main = self.main_window;
        match message {
            Message::KeyIn(id, event) if id == main => Ok(Message::Key(event)),
            Message::EscapeIn(id) if id == main => Ok(Message::Escape),
            Message::Resized(id, size) if id == main => Ok(Message::WindowResized(size)),
            Message::Moved(id, position) if id == main => Ok(Message::WindowMoved(position)),
            Message::CloseRequest(id) if id == main => Ok(Message::CloseRequested),
            Message::FileDropped(id, _) | Message::Focused(id) if id == main => Err(Task::none()),
            Message::KeyIn(id, event) => Err(self.editor_message(id, EditorMsg::Key(event))),
            Message::EscapeIn(id) => Err(self.editor_message(id, EditorMsg::Escape)),
            Message::Resized(id, size) => Err(self.editor_message(id, EditorMsg::Resized(size))),
            Message::Moved(..) => Err(Task::none()),
            Message::CloseRequest(id) => Err(self.editor_message(id, EditorMsg::CloseRequested)),
            Message::FileDropped(id, path) => Err(self.editor_message(id, EditorMsg::FileDropped(path))),
            Message::Focused(id) => Err(self.editor_message(id, EditorMsg::Focused)),
            Message::Editor(id, msg) => Err(self.editor_message(id, msg)),
            Message::Rendered(key, svg) => {
                self.renders.insert(key, svg);
                Err(Task::none())
            }
            other => Ok(other),
        }
    }

    fn handle(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::KeyIn(..)
            | Message::EscapeIn(_)
            | Message::Resized(..)
            | Message::Moved(..)
            | Message::CloseRequest(_)
            | Message::FileDropped(..)
            | Message::Focused(_)
            | Message::Editor(..)
            | Message::Rendered(..) => Task::none(),
            Message::Key(event) => self.on_key(event),
            Message::Escape => self.on_escape(),
            Message::Command(cmd) => {
                self.menu = None;
                self.run_command(cmd)
            }
            Message::Menu(m) => {
                self.menu = m;
                Task::none()
            }
            Message::NodeClicked(id) => {
                self.node_clicked(id);
                Task::none()
            }
            Message::NodeDoubleClicked(id) => {
                if let Some(doc) = &mut self.doc {
                    doc.select(vec![id]);
                }
                self.run_command(Command::PkmEdit)
            }
            Message::LinkClicked(id) => self.follow_link(id),
            Message::FoldClicked(id) => {
                if let Some(doc) = &mut self.doc {
                    doc.toggle_folded(id);
                }
                Task::none()
            }
            Message::PanStart => {
                self.pan_start();
                Task::none()
            }
            Message::PanMove(position) => self.pan_move(position),
            Message::PanEnd => self.pan_end(),
            Message::NodeDragMove(position) => {
                self.drag_move(position);
                Task::none()
            }
            Message::NodeDragEnd => {
                self.drag_end();
                Task::none()
            }
            Message::NodeDragCancel => {
                self.drag = None;
                Task::none()
            }
            Message::MapScrolled(viewport) => self.map_scrolled(viewport),
            Message::ZoomWheel(delta, at) => self.zoom_wheel(delta, at),
            Message::WindowResized(size) => self.window_resized(size),
            Message::WindowMoved(position) => self.window_moved(position),
            Message::WindowState { maximized, minimized } => {
                self.window_state(maximized, minimized);
                Task::none()
            }
            Message::CloseRequested => self.guard_unsaved(AfterUnsaved::Exit),
            Message::Tick => {
                let pages = self.editors_tick();
                let due = self
                    .doc
                    .as_ref()
                    .is_some_and(|d| d.edited_since_refresh && d.last_refresh.elapsed() >= UPDATE_INTERVAL);
                Task::batch([pages, if due { self.start_update(false) } else { Task::none() }])
            }
            Message::DialogInput(value) => {
                self.dialog_input(value);
                Task::none()
            }
            Message::DialogOk => self.dialog_ok(),
            Message::DialogCancel => self.dialog_cancel(),
            Message::DialogSave => self.dialog_save(),
            Message::DialogDiscard => self.dialog_discard(),
            Message::FileChosen(action, path) => self.file_chosen(action, path),
            Message::NextStepsComputed(generation, update) => {
                self.next_steps_computed(generation, &update);
                Task::none()
            }
            Message::IndexUpdated(result) => {
                self.index_updated(result);
                Task::none()
            }
            Message::ClipboardRead(text) => self.paste(text),
            Message::Search(msg) => self.on_search(msg),
        }
    }

    /// A click selects the node; Ctrl adds or removes it, Shift adds it. A
    /// press on a node of a multiple selection keeps the selection, so that it
    /// can be dragged, and selects just the node if released without a drag.
    fn node_clicked(&mut self, id: NodeId) {
        self.menu = None;
        self.search.focus = PanelFocus::Map;
        let Some(doc) = &mut self.doc else { return };
        let mut click = None;
        if self.modifiers.control() {
            doc.toggle_selected(id);
        } else if self.modifiers.shift() {
            doc.extend_selection(id);
        } else if doc.selection.len() > 1 && doc.selection.contains(&id) && id != doc.map.root() {
            doc.focus = id;
            click = Some(id);
        } else {
            doc.select(vec![id]);
        }
        self.drag_arm(id, click);
    }
}

/// A status bar message, shown for a while ([`STATUS_TIME`]) or until it is
/// replaced or cleared.
#[derive(Debug, Default)]
pub struct Status {
    text: String,
    since: Option<Instant>,
}

impl Status {
    pub fn set(&mut self, text: impl Into<String>) {
        self.text = text.into();
        self.since = Some(Instant::now());
    }

    pub fn clear(&mut self) {
        self.text.clear();
        self.since = None;
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    /// Clears the message once it has been shown long enough.
    fn expire(&mut self) {
        if self.since.is_some_and(|t| t.elapsed() >= STATUS_TIME) {
            self.clear();
        }
    }
}
