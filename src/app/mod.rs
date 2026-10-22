//! The application: state, messages and their handling.
//!
//! `App::update` handles a message, then recomputes the layout. The handling
//! itself lives with its topic: documents (`file`), dialogs (`dialog`),
//! commands (`command`), keyboard input (`keys`), scrolling and zooming
//! (`viewport`) and the
//! search panel (`search_panel`); `doc` holds an open map and its edits.

mod arrow_cursor;
mod command;
mod ctrl_wheel;
mod dialog;
mod doc;
mod drag;
mod file;
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
use file::{AfterUnsaved, FileAction};
use iced::keyboard::{self, Key, Modifiers, key::Named};
use iced::widget::scrollable;
use iced::{Point, Size, Subscription, Task, Vector, window};
use search_panel::{SearchMsg, SearchPanel};
use settings::WindowGeometry;
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;
use viewport::Pan;

/// The periodic Next Steps / search index refresh runs at most this often, and
/// only after the map changed.
const UPDATE_INTERVAL: Duration = Duration::from_secs(5 * 60);
const TICK: Duration = Duration::from_secs(20);
const INITIAL_WINDOW_SIZE: Size = Size::new(1280.0, 800.0);
const MAP_SCROLL_ID: &str = "map";
const DIALOG_INPUT_ID: &str = "dialog-input";

pub fn run(initial: Option<PathBuf>) -> iced::Result {
    let geometry =
        settings::window().unwrap_or(WindowGeometry { size: INITIAL_WINDOW_SIZE, position: None, maximized: false });
    let position = geometry.position.map_or(window::Position::Default, window::Position::Specific);
    iced::application(move || App::boot(initial.clone(), geometry), App::update, App::view)
        .title(App::title)
        .subscription(App::subscription)
        .theme(App::theme)
        .default_font(view::APP_FONT)
        .exit_on_close_request(false)
        .window(window::Settings {
            size: geometry.size,
            position,
            maximized: geometry.maximized,
            icon: window_icon(),
            ..Default::default()
        })
        .run()
}

/// The window (title bar / taskbar) icon, drawn by `tools/make_icon.py`.
fn window_icon() -> Option<window::Icon> {
    const SIZE: u32 = 64;
    window::icon::from_rgba(include_bytes!("../../assets/icon-64.rgba").to_vec(), SIZE, SIZE).ok()
}

/// Runs blocking work (file I/O, indexing) on the blocking thread pool.
async fn blocking<T: Send + 'static>(f: impl FnOnce() -> T + Send + 'static) -> T {
    tokio::task::spawn_blocking(f).await.expect("background task panicked")
}

#[derive(Debug, Clone)]
pub enum Message {
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
    doc: Option<Doc>,
    /// The generation given to the last opened document.
    next_generation: u64,
    /// The queued dialogs; the first one is shown.
    dialogs: Vec<Dialog>,
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
    status: String,
    pan: Option<Pan>,
    /// A node drag, armed by a press on a selected node.
    drag: Option<NodeDrag>,
}

impl App {
    fn boot(initial: Option<PathBuf>, window: WindowGeometry) -> (Self, Task<Message>) {
        let mut app = App::new();
        app.window_size = window.size;
        app.window_position = window.position;
        app.window = window;
        let explicit = initial.is_some();
        let task = match initial.or_else(settings::last_file) {
            Some(path) if explicit || path.exists() => app.open_path(path),
            _ => Task::none(),
        };
        (app, task)
    }

    /// The application without a map, before anything is opened.
    fn new() -> Self {
        App {
            doc: None,
            next_generation: 0,
            dialogs: Vec::new(),
            search: SearchPanel::default(),
            menu: None,
            modifiers: Modifiers::default(),
            window_size: INITIAL_WINDOW_SIZE,
            window_position: None,
            window: WindowGeometry { size: INITIAL_WINDOW_SIZE, position: None, maximized: false },
            scroll_offset: Vector::ZERO,
            viewport_size: None,
            zoom: 1.0,
            wheel_zoom: 0.0,
            measure_cache: HashMap::new(),
            edges: Default::default(),
            clipboard: None,
            status: String::new(),
            pan: None,
            drag: None,
        }
    }

    fn title(&self) -> String {
        match &self.doc {
            Some(doc) => {
                let name = doc.map.path().and_then(|p| p.file_name()).map(|n| n.to_string_lossy().into_owned());
                format!("{}{} - Mind PKM", name.unwrap_or_else(|| "Untitled".into()), if doc.dirty { " *" } else { "" })
            }
            None => "Mind PKM".into(),
        }
    }

    fn theme(&self) -> iced::Theme {
        iced::Theme::Light
    }

    fn subscription(&self) -> Subscription<Message> {
        Subscription::batch([
            keyboard::listen().map(Message::Key),
            // Escape also when a text input has it (closing a dialog from its input).
            iced::event::listen_with(|event, _status, _window| match event {
                iced::Event::Keyboard(keyboard::Event::KeyPressed { key: Key::Named(Named::Escape), .. }) => {
                    Some(Message::Escape)
                }
                _ => None,
            }),
            window::resize_events().map(|(_, size)| Message::WindowResized(size)),
            iced::event::listen_with(|event, _status, _window| match event {
                iced::Event::Window(window::Event::Moved(position)) => Some(Message::WindowMoved(position)),
                _ => None,
            }),
            window::close_requests().map(|_| Message::CloseRequested),
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
        ])
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        if !message.may_change_layout() {
            return self.handle(message);
        }
        let reveal = message.reveals_focus();
        // Keep the focused node in place on screen when the layout shifts around it.
        let anchor = self.focus_anchor();
        let task = self.handle(message);
        self.relayout();
        let mut tasks = vec![task, self.keep_in_place(anchor)];
        if reveal {
            tasks.push(self.reveal_focus());
        }
        Task::batch(tasks)
    }

    fn handle(&mut self, message: Message) -> Task<Message> {
        match message {
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
            Message::PanEnd => {
                self.pan_end();
                Task::none()
            }
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
            Message::MapScrolled(viewport) => {
                self.map_scrolled(viewport);
                Task::none()
            }
            Message::ZoomWheel(delta, at) => self.zoom_wheel(delta, at),
            Message::WindowResized(size) => self.window_resized(size),
            Message::WindowMoved(position) => self.window_moved(position),
            Message::WindowState { maximized, minimized } => {
                self.window_state(maximized, minimized);
                Task::none()
            }
            Message::CloseRequested => self.guard_unsaved(AfterUnsaved::Exit),
            Message::Tick => {
                let due = self
                    .doc
                    .as_ref()
                    .is_some_and(|d| d.edited_since_refresh && d.last_refresh.elapsed() >= UPDATE_INTERVAL);
                if due { self.start_update(false) } else { Task::none() }
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
        self.search.results_focused = false;
        self.search.input_focused = false;
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
