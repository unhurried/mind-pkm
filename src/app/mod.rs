//! The application: state, messages and their handling.
//!
//! It has one map window and an editor window per open page (`editor`).
//! Window events carry the window's id and are routed to the map or to the
//! editor of that window first. `App::update` handles a map message, then
//! recomputes the layout. The handling itself lives with its topic: documents
//! (`file`), dialogs (`dialog`), commands (`command`), keyboard input (`keys`),
//! scrolling and zooming (`viewport`), the search panel (`search_panel`) and
//! the editor windows (`editors`); `doc` holds an open map and its edits.

mod arrow_cursor;
mod command;
mod ctrl_wheel;
mod dialog;
mod doc;
mod drag;
mod editor;
mod editors;
mod file;
mod ime;
mod keys;
mod layout;
mod node_edit;
mod placed;
mod press_point;
pub mod profile;
mod right_press;
mod search_panel;
pub mod settings;
mod ui;
mod view;
mod viewport;

#[cfg(test)]
mod perf;
#[cfg(test)]
mod test_support;
#[cfg(test)]
mod tests;

use crate::mindmap::NodeId;
use crate::pkm::next_steps;
use command::{Clipboard, Command};
use dialog::Dialog;
use doc::Doc;
use drag::NodeDrag;
use editor::{Editor, EditorMsg, EditorPrefs, RenderCache, RenderKey, is_markdown};
use file::{AfterUnsaved, FileAction};
use iced::keyboard::{self, Key, Modifiers, key::Named};
use iced::widget::{scrollable, text_editor};
use iced::{Point, Size, Subscription, Task, window};
use node_edit::NodeEdit;
use search_panel::{PanelFocus, SearchMsg, SearchPanel};
use settings::WindowGeometry;
use std::collections::{BTreeMap, VecDeque};
use std::path::PathBuf;
use std::time::{Duration, Instant};
use viewport::MapViewport;

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

/// A message of the application: an event of one of its windows, a message
/// of an editor window or of the map window, or a render coming back.
#[derive(Debug, Clone)]
pub enum Message {
    /// An event of a window, for the map or for the editor it belongs to.
    Window(window::Id, WindowEvent),
    /// A message of an editor window.
    Editor(window::Id, EditorMsg),
    /// Math or a diagram rendered in the background.
    Rendered(RenderKey, Result<String, String>),
    /// A message of the map window.
    Map(MapMsg),
}

impl From<MapMsg> for Message {
    fn from(msg: MapMsg) -> Message {
        Message::Map(msg)
    }
}

/// What a window reports, whichever window it is.
#[derive(Debug, Clone)]
pub enum WindowEvent {
    /// A key no widget took.
    Key(keyboard::Event),
    /// Escape (also when a text box has it).
    Escape,
    Resized(Size),
    Moved(Point),
    CloseRequested,
    FileDropped(PathBuf),
    Focused,
}

impl WindowEvent {
    /// As a message of the map window; None for those it does nothing with.
    fn for_map(self) -> Option<MapMsg> {
        Some(match self {
            WindowEvent::Key(event) => MapMsg::Key(event),
            WindowEvent::Escape => MapMsg::Escape,
            WindowEvent::Resized(size) => MapMsg::WindowResized(size),
            WindowEvent::Moved(position) => MapMsg::WindowMoved(position),
            WindowEvent::CloseRequested => MapMsg::CloseRequested,
            WindowEvent::FileDropped(_) | WindowEvent::Focused => return None,
        })
    }

    /// As a message of an editor window; None for those it does nothing with.
    fn for_editor(self) -> Option<EditorMsg> {
        Some(match self {
            WindowEvent::Key(event) => EditorMsg::Key(event),
            WindowEvent::Escape => EditorMsg::Escape,
            WindowEvent::Resized(size) => EditorMsg::Resized(size),
            WindowEvent::Moved(_) => return None,
            WindowEvent::CloseRequested => EditorMsg::CloseRequested,
            WindowEvent::FileDropped(path) => EditorMsg::FileDropped(path),
            WindowEvent::Focused => EditorMsg::Focused,
        })
    }
}

/// A message of the map window.
#[derive(Debug, Clone)]
pub enum MapMsg {
    Key(keyboard::Event),
    Escape,
    Command(Command),
    Menu(Option<usize>),
    NodeClicked(NodeId),
    NodeDoubleClicked(NodeId),
    LinkClicked(NodeId),
    FoldClicked(NodeId),
    /// The right button went down at this point of the window (before the
    /// widget under the pointer says what was right-clicked).
    RightPressedAt(iced::Point),
    /// A right click on a node: its menu.
    NodeRightClicked(NodeId),
    /// A right click on empty map space: its menu.
    MapRightClicked,
    CloseContextMenu,
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
    /// An action of the text box of the node being edited.
    NodeEdit(text_editor::Action),
    /// Enter in the text box of the node being edited.
    NodeEditConfirm,
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

impl MapMsg {
    /// Whether handling it may change the map or what is shown of it, so that
    /// the layout is recomputed afterwards.
    fn may_change_layout(&self) -> bool {
        !matches!(
            self,
            MapMsg::MapScrolled(_)
                | MapMsg::ZoomWheel(..)
                | MapMsg::Tick
                | MapMsg::Menu(_)
                | MapMsg::RightPressedAt(_)
                | MapMsg::MapRightClicked
                | MapMsg::CloseContextMenu
                | MapMsg::PanStart
                | MapMsg::PanMove(_)
                | MapMsg::PanEnd
                | MapMsg::NodeDragMove(_)
                | MapMsg::NodeDragCancel
                | MapMsg::WindowMoved(_)
                | MapMsg::WindowState { .. }
        )
    }

    /// Whether it is the user acting on the map, after which the focused node
    /// is scrolled into view.
    fn reveals_focus(&self) -> bool {
        matches!(
            self,
            MapMsg::Key(_)
                | MapMsg::Escape
                | MapMsg::Command(_)
                | MapMsg::NodeEdit(_)
                | MapMsg::NodeEditConfirm
                | MapMsg::Search(_)
                | MapMsg::DialogOk
                | MapMsg::FileChosen(..)
        )
    }
}

pub struct App {
    main_window: window::Id,
    /// Started with a Markdown file: there is no map window, and the
    /// application ends with its last editor window.
    editors_only: bool,
    /// The application is ending.
    exiting: bool,
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
    /// The node whose text is being edited on the map.
    node_edit: Option<NodeEdit>,
    search: SearchPanel,
    /// The open menu: an index into `view::MENUS`.
    menu: Option<usize>,
    /// The open right-click menu.
    context_menu: Option<MapContextMenu>,
    /// Where the right mouse button last went down, in the window.
    right_pressed_at: Point,
    modifiers: Modifiers,
    window_size: Size,
    /// The window's position as last reported.
    window_position: Option<Point>,
    /// The window as it is to be restored on the next start.
    window: WindowGeometry,
    /// What the map window shows of the map: zoom, scrolling, panning.
    viewport: MapViewport,
    clipboard: Option<Clipboard>,
    /// The status bar message.
    status: Status,
    /// A node drag, armed by a press on a selected node.
    drag: Option<NodeDrag>,
    /// Removes what the editors' assets panes delete: to the Recycle Bin.
    asset_remover: fn(&[PathBuf]) -> Result<(), String>,
}

/// Moves files to the Recycle Bin; in tests, which work in temporary
/// directories, removes them.
fn recycle(files: &[PathBuf]) -> Result<(), String> {
    if cfg!(test) {
        for f in files {
            let removed = if f.is_dir() { std::fs::remove_dir_all(f) } else { std::fs::remove_file(f) };
            removed.map_err(|e| e.to_string())?;
        }
        return Ok(());
    }
    trash::delete_all(files).map_err(|e| e.to_string())
}

impl App {
    fn boot(initial: Option<PathBuf>, window: WindowGeometry) -> (Self, Task<Message>) {
        let mut app = App::new();
        app.editor_prefs = EditorPrefs::load();
        let task = app.start(initial, window);
        (app, task)
    }

    /// Opens the application's first windows: for a Markdown file `initial`,
    /// its editor window alone (in the built-in editor, whatever the
    /// preferences say); else the map window (`window`) with the map
    /// `initial`, or the last opened one - also when the Markdown file can't
    /// be opened, which the map window then says.
    fn start(&mut self, initial: Option<PathBuf>, window: WindowGeometry) -> Task<Message> {
        self.window_size = window.size;
        self.window_position = window.position;
        self.window = window;
        let initial = match initial {
            Some(path) if is_markdown(&path) => match self.try_open_in_editor(&path) {
                Ok(opened) => {
                    self.editors_only = true;
                    return opened;
                }
                Err(e) => {
                    self.message(e);
                    None
                }
            },
            initial => initial,
        };
        let (id, opened) = window::open(window_settings(window.size, window.position, window.maximized));
        self.main_window = id;
        let explicit = initial.is_some();
        let task = match initial.or_else(settings::last_file) {
            Some(path) if explicit || path.exists() => self.open_path(path),
            _ => Task::none(),
        };
        Task::batch([opened.discard(), task])
    }

    /// The application without a map, before anything is opened.
    fn new() -> Self {
        App {
            main_window: window::Id::unique(),
            editors_only: false,
            exiting: false,
            editors: BTreeMap::new(),
            editor_prefs: EditorPrefs::default(),
            renders: RenderCache::default(),
            system_clipboard: std::sync::Arc::new(std::sync::Mutex::new(editor::clipboard::default_backend())),
            doc: None,
            next_generation: 0,
            dialogs: VecDeque::new(),
            node_edit: None,
            search: SearchPanel::default(),
            menu: None,
            context_menu: None,
            right_pressed_at: Point::ORIGIN,
            modifiers: Modifiers::default(),
            window_size: INITIAL_WINDOW.size,
            window_position: None,
            window: INITIAL_WINDOW,
            viewport: MapViewport::default(),
            clipboard: None,
            status: Status::default(),
            drag: None,
            asset_remover: recycle,
        }
    }

    fn title(&self, window: window::Id) -> String {
        if let Some(editor) = self.editors.get(&window) {
            return editor.title();
        }
        match &self.doc {
            Some(doc) => {
                let name = doc.map().path().and_then(|p| p.file_name()).map(|n| n.to_string_lossy().into_owned());
                format!(
                    "{}{} - Mind PKM",
                    name.unwrap_or_else(|| "Untitled".into()),
                    if doc.dirty() { " *" } else { "" }
                )
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
                (iced::Event::Keyboard(event), iced::event::Status::Ignored) => {
                    Some(Message::Window(window, WindowEvent::Key(event)))
                }
                _ => None,
            }),
            // Escape also when a text input has it (closing a dialog from its input).
            iced::event::listen_with(|event, _status, window| {
                match event {
                    iced::Event::Keyboard(keyboard::Event::KeyPressed { key: Key::Named(Named::Escape), .. }) => {
                        Some(WindowEvent::Escape)
                    }
                    iced::Event::Window(window::Event::Moved(position)) => Some(WindowEvent::Moved(position)),
                    iced::Event::Window(window::Event::Resized(size)) => Some(WindowEvent::Resized(size)),
                    iced::Event::Window(window::Event::FileDropped(path)) => Some(WindowEvent::FileDropped(path)),
                    iced::Event::Window(window::Event::Focused) => Some(WindowEvent::Focused),
                    _ => None,
                }
                .map(|event| Message::Window(window, event))
            }),
            window::close_requests().map(|id| Message::Window(id, WindowEvent::CloseRequested)),
            iced::time::every(TICK).map(|_| MapMsg::Tick.into()),
            // Pointer tracking only while panning, so ordinary moves cost nothing.
            if self.viewport.pan.is_some() {
                iced::event::listen_with(|event, _status, _window| match event {
                    iced::Event::Mouse(iced::mouse::Event::CursorMoved { position }) => {
                        Some(MapMsg::PanMove(position).into())
                    }
                    iced::Event::Mouse(iced::mouse::Event::ButtonReleased(iced::mouse::Button::Left))
                    | iced::Event::Mouse(iced::mouse::Event::CursorLeft) => Some(MapMsg::PanEnd.into()),
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
        let label = profile::enabled().then(|| self.message_label(&message));
        profile::time(
            profile::Phase::Update,
            || label.unwrap_or_default(),
            || {
                let task = self.update_message(message);
                Task::batch([task, self.read_assets()])
            },
        )
    }

    /// The message as the timings name it.
    fn message_label(&self, message: &Message) -> String {
        match message {
            Message::Window(id, event) => {
                format!("{}/{}", if *id == self.main_window { "Map" } else { "Editor" }, profile::label(event))
            }
            Message::Editor(_, msg) => format!("Editor/{}", profile::label(msg)),
            Message::Rendered(..) => "Rendered".into(),
            Message::Map(msg) => format!("Map/{}", profile::label(msg)),
        }
    }

    fn update_message(&mut self, message: Message) -> Task<Message> {
        self.status.expire();
        for editor in self.editors.values_mut() {
            editor.status.expire();
        }
        let message = match message {
            Message::Map(msg) => msg,
            Message::Window(id, event) if id == self.main_window => match event.for_map() {
                Some(msg) => msg,
                None => return Task::none(),
            },
            Message::Window(id, event) => {
                return event.for_editor().map_or(Task::none(), |msg| self.editor_message(id, msg));
            }
            Message::Editor(id, msg) => return self.editor_message(id, msg),
            Message::Rendered(key, svg) => {
                self.renders.insert(key, svg);
                return Task::none();
            }
        };
        // Acting elsewhere than in the text box confirms the text being edited.
        if let Some(edited) = self.node_edit.as_ref().map(NodeEdit::id)
            && message.confirms_node_edit(edited)
        {
            self.confirm_node_edit();
            self.relayout();
        }
        if !message.may_change_layout() {
            return self.handle(message);
        }
        let reveal = message.reveals_focus();
        // Keep the focused node (or, after a move among siblings, its parent)
        // in place on screen when the layout shifts around it.
        let (focus_anchor, parent_anchor) = (self.focus_anchor(false), self.focus_anchor(true));
        let task = self.handle(message);
        self.relayout();
        let anchor = if std::mem::take(&mut self.viewport.keep_parent_in_place) { parent_anchor } else { focus_anchor };
        let mut tasks = vec![task, self.keep_in_place(anchor)];
        if reveal {
            tasks.push(self.reveal_focus());
        }
        Task::batch(tasks)
    }

    /// Handles a message of the map window.
    fn handle(&mut self, message: MapMsg) -> Task<Message> {
        match message {
            MapMsg::Key(event) => self.on_key(event),
            MapMsg::Escape => self.on_escape(),
            MapMsg::Command(cmd) => {
                (self.menu, self.context_menu) = (None, None);
                self.run_command(cmd)
            }
            MapMsg::Menu(m) => {
                (self.menu, self.context_menu) = (m, None);
                Task::none()
            }
            MapMsg::RightPressedAt(at) => {
                self.right_pressed_at = at;
                Task::none()
            }
            MapMsg::NodeRightClicked(id) => {
                self.node_right_clicked(id);
                Task::none()
            }
            MapMsg::MapRightClicked => {
                self.menu = None;
                if self.doc.is_some() {
                    self.context_menu = Some(MapContextMenu { at: self.right_pressed_at, on_node: false });
                }
                Task::none()
            }
            MapMsg::CloseContextMenu => {
                self.context_menu = None;
                Task::none()
            }
            MapMsg::NodeClicked(id) => {
                self.node_clicked(id);
                Task::none()
            }
            MapMsg::NodeDoubleClicked(id) => {
                if let Some(doc) = &mut self.doc {
                    doc.select(vec![id]);
                }
                self.run_command(Command::PkmEdit)
            }
            MapMsg::LinkClicked(id) => self.follow_link(id),
            MapMsg::FoldClicked(id) => {
                if let Some(doc) = &mut self.doc {
                    doc.toggle_folded(id);
                }
                Task::none()
            }
            MapMsg::PanStart => {
                self.pan_start();
                Task::none()
            }
            MapMsg::PanMove(position) => self.pan_move(position),
            MapMsg::PanEnd => {
                self.pan_end();
                Task::none()
            }
            MapMsg::NodeDragMove(position) => {
                self.drag_move(position);
                Task::none()
            }
            MapMsg::NodeDragEnd => {
                self.drag_end();
                Task::none()
            }
            MapMsg::NodeDragCancel => {
                self.drag = None;
                Task::none()
            }
            MapMsg::MapScrolled(viewport) => {
                self.map_scrolled(viewport);
                Task::none()
            }
            MapMsg::ZoomWheel(delta, at) => self.zoom_wheel(delta, at),
            MapMsg::WindowResized(size) => self.window_resized(size),
            MapMsg::WindowMoved(position) => self.window_moved(position),
            MapMsg::WindowState { maximized, minimized } => {
                self.window_state(maximized, minimized);
                Task::none()
            }
            MapMsg::CloseRequested => self.guard_unsaved(AfterUnsaved::Exit),
            MapMsg::Tick => {
                let pages = self.editors_tick();
                let due = self.doc.as_ref().is_some_and(|d| d.refresh_due(UPDATE_INTERVAL));
                Task::batch([pages, if due { self.start_update(false) } else { Task::none() }])
            }
            MapMsg::NodeEdit(action) => {
                self.node_edit_action(action);
                Task::none()
            }
            MapMsg::NodeEditConfirm => {
                self.confirm_node_edit();
                Task::none()
            }
            MapMsg::DialogInput(value) => {
                self.dialog_input(value);
                Task::none()
            }
            MapMsg::DialogOk => self.dialog_ok(),
            MapMsg::DialogCancel => self.dialog_cancel(),
            MapMsg::DialogSave => self.dialog_save(),
            MapMsg::DialogDiscard => self.dialog_discard(),
            MapMsg::FileChosen(action, path) => self.file_chosen(action, path),
            MapMsg::NextStepsComputed(generation, update) => {
                self.next_steps_computed(generation, &update);
                Task::none()
            }
            MapMsg::IndexUpdated(result) => {
                self.index_updated(result);
                Task::none()
            }
            MapMsg::ClipboardRead(text) => self.paste(text),
            MapMsg::Search(msg) => self.on_search(msg),
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
        } else if doc.selection().len() > 1 && doc.selection().contains(&id) && id != doc.map().root() {
            doc.set_focus(id);
            click = Some(id);
        } else {
            doc.select(vec![id]);
        }
        self.drag_arm(id, click);
    }

    /// A right click on a node opens its menu. A node outside the selection
    /// becomes the only selected one; on a selected node the selection stays
    /// (the node becomes the focused one), so that the menu acts on all of it.
    fn node_right_clicked(&mut self, id: NodeId) {
        self.menu = None;
        self.search.focus = PanelFocus::Map;
        let Some(doc) = &mut self.doc else { return };
        if doc.selection().contains(&id) {
            doc.set_focus(id);
        } else {
            doc.select(vec![id]);
        }
        self.context_menu = Some(MapContextMenu { at: self.right_pressed_at, on_node: true });
    }
}

/// A right-click menu of the map: where it opens, and whether it is a node's
/// (else empty map space's).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MapContextMenu {
    pub at: Point,
    pub on_node: bool,
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
