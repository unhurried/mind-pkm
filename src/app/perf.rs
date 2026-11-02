//! Benchmarks of what the user waits for: the application handling a message
//! and then, as iced does after messages, building, comparing, laying out and
//! drawing every window - on a large map with a long page open, without
//! windows (a headless renderer, so the GPU's part is left out). They are
//! ignored by default; run them in release mode (iced's test renderer, which
//! other tests use, needs debug assertions in `iced_core`):
//!
//! ```text
//! cargo test --release --lib --config 'profile.release.package.iced_core.debug-assertions=true' \
//!     perf -- --ignored --nocapture --test-threads=1
//! ```
//!
//! Also here: tests of what makes the windows rebuild (pointer moves must not).

use super::command::Command;
use super::editor::EditorMsg;
use super::profile::{self, Phase};
use super::test_support::{headless_renderer, key_event};
use super::viewport::NEAR_MARGIN;
use super::{App, MAP_SCROLL_ID, MapMsg, Message, WindowEvent};
use crate::mindmap::NodeId;
use crate::pkm::testutil::Fixture;
use iced::advanced::clipboard;
use iced::advanced::renderer;
use iced::advanced::widget::operation::scrollable::AbsoluteOffset;
use iced::advanced::widget::{self, Operation, operation};
use iced::keyboard::{self, Key, Modifiers, key::Named};
use iced::widget::text_editor::{Action, Edit};
use iced::{Color, Event, Point, Rectangle, Size, Vector, mouse, window};
use iced_runtime::user_interface::{Cache, UserInterface};
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::fmt::Write as _;
use std::time::{Duration, Instant};

const WINDOW: Size = Size::new(1280.0, 800.0);
/// How many times each step is repeated.
const STEPS: usize = 20;

/// The application with its windows' interfaces, rebuilt as iced rebuilds them.
struct Bench {
    app: App,
    renderer: iced::Renderer,
    caches: BTreeMap<window::Id, Cache>,
    editor: window::Id,
    nodes: Vec<NodeId>,
    _fixture: Fixture,
}

/// What a step took, by phase and window (`update` for the messages), and
/// the messages it handled.
#[derive(Default)]
struct Totals {
    took: HashMap<(Phase, &'static str), Duration>,
    wall: Duration,
    messages: usize,
    /// The messages handled, by name.
    names: BTreeSet<String>,
    steps: usize,
}

/// The columns of the report: a phase in a window (any window for None).
const COLUMNS: &[(&str, Phase, Option<&str>)] = &[
    ("update", Phase::Update, None),
    ("view:map", Phase::View, Some("map")),
    ("view:ed", Phase::View, Some("editor")),
    ("diff", Phase::Diff, None),
    ("lay:map", Phase::Layout, Some("map")),
    ("lay:ed", Phase::Layout, Some("editor")),
    ("event", Phase::Event, None),
    ("draw", Phase::Draw, None),
];

impl Totals {
    fn add(&mut self, timings: Vec<profile::Timing>, wall: Duration, messages: usize) {
        for t in timings {
            let window = match t.what.as_str() {
                "map" => "map",
                "editor" => "editor",
                _ => {
                    self.names.insert(t.what.clone());
                    ""
                }
            };
            *self.took.entry((t.phase, window)).or_default() += t.took;
        }
        self.wall += wall;
        self.messages += messages;
        self.steps += 1;
    }

    fn total(&self, phase: Phase, window: Option<&str>) -> Duration {
        self.took.iter().filter(|((p, w), _)| *p == phase && window.is_none_or(|x| x == *w)).map(|(_, d)| *d).sum()
    }

    /// A row of the report: per step, in milliseconds.
    fn row(&self, name: &str) -> String {
        let per_step = |d: Duration| d.as_secs_f64() * 1000.0 / self.steps.max(1) as f64;
        let mut row = format!("{name:<34} {:>5.1}", self.messages as f64 / self.steps.max(1) as f64);
        for &(_, phase, window) in COLUMNS {
            let _ = write!(row, " {:>8.2}", per_step(self.total(phase, window)));
        }
        let _ = write!(row, " {:>8.2}", per_step(self.wall));
        let names: Vec<&str> = self.names.iter().map(String::as_str).collect();
        let _ = write!(row, "  {}", names.join(", "));
        row
    }
}

fn header() -> String {
    let mut header = format!("{:<34} {:>5}", "ms per step", "msgs");
    for (name, ..) in COLUMNS {
        let _ = write!(header, " {name:>8}");
    }
    header + &format!(" {:>8}", "total")
}

/// A map of about `fanout^3` nodes under root > Projects, every tenth a page.
fn large_map(f: &mut Fixture, fanout: usize) -> Vec<NodeId> {
    let root = f.map.root();
    let projects = f.add_child(root, "Projects");
    let mut nodes = Vec::new();
    for a in 0..fanout {
        let na = f.add_child(projects, &format!("{a:02} プロジェクト Alpha {a} の資料と進め方"));
        nodes.push(na);
        for b in 0..fanout {
            let nb = f.add_child(na, &format!("{a:02}-{b:02} Meeting notes and decisions"));
            nodes.push(nb);
            for c in 0..fanout {
                let text = format!("{a:02}-{b:02}-{c:02} 調査メモ: research item number {c}");
                let n = if c % 10 == 0 { f.add_page_node(nb, &text) } else { f.add_child(nb, &text) };
                nodes.push(n);
            }
        }
    }
    nodes
}

/// A page of `sections` sections, each with a heading, paragraphs with
/// inline markup, a list and a code block of 25 lines.
fn long_page(sections: usize) -> String {
    let mut s = String::from("# Benchmark page\n\n");
    for i in 0..sections {
        let _ = write!(
            s,
            "## Section {i}\n\n\
             Paragraph {i} with **bold**, *emphasis*, `inline code` and a [link](https://example.com/{i}). \
             日本語の文章も含めて、折り返しが起きる程度の長さにしておく。The quick brown fox jumps over the lazy dog.\n\n\
             - item one of {i}\n- item two with `code`\n- item three\n\n\
             ```rust\n"
        );
        for l in 0..25 {
            let _ = writeln!(s, "fn function_{i}_{l}(x: u32) -> u32 {{ let y = x * {l} + 1; y.pow(2) }} // line {l}");
        }
        s.push_str("```\n\nAnother paragraph after the code, with a footnote-like remark and more text.\n\n");
    }
    s
}

impl Bench {
    fn new(fanout: usize, sections: usize) -> Bench {
        let mut f = Fixture::new();
        let nodes = large_map(&mut f, fanout);
        let root = f.map.root();
        f.write_page("Bench", &long_page(sections));
        let page = f.add_page_node(root, "Bench");
        let mut app = App::new();
        let _ = app.set_doc(f.map.clone(), None);
        app.doc.as_mut().unwrap().select(vec![page]);
        let _ = app.update(MapMsg::WindowResized(WINDOW).into());
        let _ = app.update(MapMsg::Key(key_event(Key::Named(Named::F1), Modifiers::empty())).into());
        let editor = *app.editors.keys().last().expect("the page opened in an editor");
        let _ = app.update(Message::Window(editor, WindowEvent::Resized(WINDOW)));
        let renderer = headless_renderer();
        let mut bench = Bench { app, renderer, caches: BTreeMap::new(), editor, nodes, _fixture: f };
        bench.rebuild();
        bench
    }

    fn windows(&self) -> Vec<window::Id> {
        std::iter::once(self.app.main_window).chain(self.app.editors.keys().copied()).collect()
    }

    /// What iced does after handling messages: builds every window's
    /// interface again (the view, compared with the last one, laid out) and
    /// draws it. The tasks the messages gave are not run here; of them, the
    /// map's scrolling is done: its scrollable is moved to where the
    /// application scrolled the map.
    fn rebuild(&mut self) {
        for id in self.windows() {
            let cache = self.caches.remove(&id).unwrap_or_default();
            let theme = self.app.theme(id);
            let mut ui = UserInterface::build(self.app.view(id), WINDOW, cache, &mut self.renderer);
            if id == self.app.main_window {
                let Vector { x, y } = self.app.viewport.scroll;
                let to = AbsoluteOffset { x, y }.into();
                ui.operate(&self.renderer, &mut operation::scrollable::scroll_to(widget::Id::new(MAP_SCROLL_ID), to));
            }
            let style = renderer::Style { text_color: Color::BLACK };
            ui.draw(&mut self.renderer, &theme, &style, mouse::Cursor::Unavailable);
            self.caches.insert(id, ui.into_cache());
        }
    }

    /// Handles a message and rebuilds the windows, untimed.
    fn unmeasured(&mut self, message: impl Into<Message>) {
        let _ = self.app.update(message.into());
        self.rebuild();
    }

    /// Handles a message and rebuilds the windows, timed.
    fn step(&mut self, totals: &mut Totals, message: impl Into<Message>) {
        profile::start_collecting();
        let start = Instant::now();
        let _ = self.app.update(message.into());
        self.rebuild();
        totals.add(profile::stop_collecting(), start.elapsed(), 1);
    }

    /// The messages an event in a window gives: the window's interface as
    /// the last rebuild left it (iced keeps it between events) takes it.
    /// `timed`: the event is timed (not building the interface again), and
    /// the timings are being collected when this returns; also returns when
    /// the event was given.
    fn messages(&mut self, id: window::Id, event: Event, cursor: Point, timed: bool) -> (Vec<Message>, Instant) {
        if timed {
            // The interface with its timing wrapper.
            profile::start_collecting();
        }
        let cache = self.caches.remove(&id).unwrap_or_default();
        let mut ui = UserInterface::build(self.app.view(id), WINDOW, cache, &mut self.renderer);
        if timed {
            profile::stop_collecting();
            profile::start_collecting();
        }
        let start = Instant::now();
        let mut messages = Vec::new();
        ui.update(&[event], mouse::Cursor::Available(cursor), &mut self.renderer, &mut clipboard::Null, &mut messages);
        self.caches.insert(id, ui.into_cache());
        (messages, start)
    }

    /// An event in a window, as iced handles it: the messages it gives are
    /// handled and the windows rebuilt; untimed.
    fn deliver(&mut self, id: window::Id, event: Event, cursor: Point) {
        let (messages, _) = self.messages(id, event, cursor, false);
        for m in messages {
            let _ = self.app.update(m);
        }
        self.rebuild();
    }

    /// What a window shows: the map's scrollable and the texts.
    fn shown(&mut self, id: window::Id) -> Shown {
        let cache = self.caches.remove(&id).unwrap_or_default();
        let mut ui = UserInterface::build(self.app.view(id), WINDOW, cache, &mut self.renderer);
        let mut shown = Shown::default();
        ui.operate(&self.renderer, &mut shown);
        self.caches.insert(id, ui.into_cache());
        shown
    }

    /// An event in a window, as iced handles it: the messages it gives are
    /// handled, and if there are any, the windows are rebuilt; timed.
    fn event(&mut self, totals: &mut Totals, id: window::Id, event: Event, cursor: Point) {
        let (messages, start) = self.messages(id, event, cursor, true);
        let count = messages.len();
        for m in messages {
            let _ = self.app.update(m);
        }
        if count > 0 {
            self.rebuild();
        }
        totals.add(profile::stop_collecting(), start.elapsed(), count);
    }
}

fn key(named: Named) -> keyboard::Event {
    key_event(Key::Named(named), Modifiers::empty())
}

fn release(named: Named) -> keyboard::Event {
    keyboard::Event::KeyReleased {
        key: Key::Named(named),
        modified_key: Key::Named(named),
        physical_key: keyboard::key::Physical::Unidentified(keyboard::key::NativeCode::Unidentified),
        location: keyboard::Location::Standard,
        modifiers: Modifiers::empty(),
    }
}

/// Runs every scenario on a map of about `fanout^3` nodes with a page of
/// `sections` sections open, and prints the report.
fn report(fanout: usize, sections: usize) {
    let mut b = Bench::new(fanout, sections);
    let editor = b.editor;
    let mut rows = vec![
        format!(
            "map: {} nodes, page: {} bytes in {} blocks",
            b.nodes.len() + 1,
            b.app.editors[&editor].page.text().len(),
            b.app.editors[&editor].page.doc().blocks.len()
        ),
        header(),
    ];
    let mut run = |name: &str, b: &mut Bench, scenario: &mut dyn FnMut(&mut Bench, &mut Totals, usize)| {
        let mut totals = Totals::default();
        for i in 0..STEPS {
            scenario(b, &mut totals, i);
        }
        rows.push(totals.row(name));
    };

    // The map window.
    let start = b.nodes[0];
    b.app.doc.as_mut().unwrap().select(vec![start]);
    run("map: Down key (press)", &mut b, &mut |b, t, _| b.step(t, MapMsg::Key(key(Named::ArrowDown))));
    run("map: key release", &mut b, &mut |b, t, _| b.step(t, MapMsg::Key(release(Named::ArrowDown))));
    run("map: click a node", &mut b, &mut |b, t, i| {
        let node = b.nodes[(i * 97) % b.nodes.len()];
        b.step(t, MapMsg::NodeClicked(node));
        // The button released: no drag.
        b.unmeasured(MapMsg::NodeDragEnd);
    });
    run("map: zoom in/out", &mut b, &mut |b, t, i| {
        b.step(t, MapMsg::Command(if i % 2 == 0 { Command::ZoomIn } else { Command::ZoomOut }))
    });
    b.unmeasured(MapMsg::Command(Command::EditText));
    run("map: typing in a dialog", &mut b, &mut |b, t, i| b.step(t, MapMsg::DialogInput("x".repeat(i + 1))));
    b.unmeasured(MapMsg::DialogCancel);
    run("map: tick (any message)", &mut b, &mut |b, t, _| b.step(t, MapMsg::Tick));

    // The editor window.
    let text = b.app.editors[&editor].page.text().to_string();
    let at = text.find("Paragraph 3 with").unwrap() + "Paragraph 3".len();
    b.unmeasured(Message::Editor(editor, EditorMsg::PressedAt(at)));
    b.unmeasured(Message::Editor(editor, EditorMsg::Released));
    run("editor: typing a character", &mut b, &mut |b, t, _| {
        b.step(t, Message::Editor(editor, EditorMsg::Action(Action::Edit(Edit::Insert('a')))))
    });
    run("editor: pointer moves over text", &mut b, &mut |b, t, i| {
        let at = Point::new(260.0 + (i % 5) as f32 * 7.0, 120.0 + i as f32 * 23.0);
        b.event(t, editor, Event::Mouse(mouse::Event::CursorMoved { position: at }), at)
    });
    run("map: wheel scroll", &mut b, &mut |b, t, i| {
        let at = Point::new(640.0, 400.0);
        let main = b.app.main_window;
        // Down the map, a little sideways now and then.
        let delta = mouse::ScrollDelta::Lines { x: if i % 4 == 3 { -1.0 } else { 0.0 }, y: -2.0 };
        b.event(t, main, Event::Mouse(mouse::Event::WheelScrolled { delta }), at)
    });
    run("map: pointer moves over the map", &mut b, &mut |b, t, i| {
        let at = Point::new(200.0 + i as f32 * 31.0, 150.0 + i as f32 * 19.0);
        let main = b.app.main_window;
        b.event(t, main, Event::Mouse(mouse::Event::CursorMoved { position: at }), at)
    });

    println!("\n{}", rows.join("\n"));
}

#[test]
#[ignore = "a benchmark: run in release mode with --ignored --nocapture"]
fn perf_large_map_and_long_page() {
    report(15, 40);
}

/// What a window shows, as an operation finds it: where the map's scrollable
/// is and how far it is scrolled, and the texts of the text widgets.
#[derive(Default)]
struct Shown {
    map: Option<(Rectangle, Vector)>,
    texts: HashSet<String>,
}

impl Operation for Shown {
    fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation)) {
        operate(self);
    }

    fn scrollable(
        &mut self,
        id: Option<&widget::Id>,
        bounds: Rectangle,
        _content_bounds: Rectangle,
        translation: Vector,
        _state: &mut dyn operation::Scrollable,
    ) {
        if id == Some(&widget::Id::new(MAP_SCROLL_ID)) {
            self.map = Some((bounds, translation));
        }
    }

    fn text(&mut self, _id: Option<&widget::Id>, _bounds: Rectangle, text: &str) {
        self.texts.insert(text.to_string());
    }
}

/// The map builds widgets for the nodes near the part it shows, wherever it
/// is scrolled to - and those take presses.
#[test]
fn the_map_builds_widgets_for_the_nodes_near_what_it_shows() {
    let mut b = Bench::new(6, 2);
    let main = b.app.main_window;
    let check = |b: &mut Bench| {
        let shown = b.shown(main);
        let (bounds, scroll) = shown.map.expect("the map");
        let visible = Rectangle::new(Point::ORIGIN + scroll, bounds.size());
        let near = visible.expand(NEAR_MARGIN + 1.0);
        let doc = b.app.doc.as_ref().unwrap();
        let (mut built, mut left_out) = (0, 0);
        for node in &doc.layout().boxes {
            let text = doc.map().text(node.id);
            if node.rect.intersects(&visible) {
                assert!(shown.texts.contains(&text), "{text:?} is visible");
                built += 1;
            } else if !node.rect.intersects(&near) {
                assert!(!shown.texts.contains(&text), "{text:?} is far from the view");
                left_out += 1;
            }
        }
        assert!(built > 10 && left_out > 100, "{built} built, {left_out} left out");
        // A press on a node in view takes it.
        let node = doc.layout().boxes.iter().rev().find(|n| visible.contains(n.rect.center())).unwrap();
        let (id, at) = (node.id, bounds.position() + (n_center(node.rect) - scroll));
        let _ = b.messages(main, moved(at), at, false);
        let press = Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left));
        let (messages, _) = b.messages(main, press, at, false);
        assert!(messages.iter().any(|m| matches!(m, Message::Map(MapMsg::NodeClicked(n)) if *n == id)), "{messages:?}");
        scroll
    };
    // Shown scrolled down to the selected page node at the bottom.
    let start = check(&mut b);
    assert!(start.y > 1000.0);
    // Up the map with the wheel.
    let at = Point::new(640.0, 400.0);
    for _ in 0..40 {
        let delta = mouse::ScrollDelta::Lines { x: 0.0, y: 3.0 };
        b.deliver(main, Event::Mouse(mouse::Event::WheelScrolled { delta }), at);
    }
    assert!(check(&mut b).y < start.y - 1000.0);
}

fn n_center(r: Rectangle) -> Vector {
    Vector::new(r.center_x(), r.center_y())
}

fn moved(at: Point) -> Event {
    Event::Mouse(mouse::Event::CursorMoved { position: at })
}

/// Moving the pointer over a page or the map sends no message (each would
/// build every window again); a press on a page's text still tells where in
/// the text it was, before the press itself.
#[test]
fn pointer_moves_send_no_messages() {
    let mut b = Bench::new(3, 6);
    let (editor, map) = (b.editor, b.app.main_window);
    for i in 0..60 {
        let at = Point::new(120.0 + (i % 7) as f32 * 40.0, 40.0 + i as f32 * 12.0);
        for window in [editor, map] {
            let (messages, _) = b.messages(window, moved(at), at, false);
            assert!(messages.is_empty(), "{window:?} at {at:?}: {messages:?}");
        }
    }
    // A press on text somewhere down the page.
    let pressed = (0..60).find_map(|i| {
        let at = Point::new(160.0, 40.0 + i as f32 * 12.0);
        let _ = b.messages(editor, moved(at), at, false);
        let press = Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left));
        let (messages, _) = b.messages(editor, press, at, false);
        messages.iter().any(|m| matches!(m, Message::Editor(_, EditorMsg::TextPressed))).then_some(messages)
    });
    let messages = pressed.expect("a press on text");
    let pointer = messages.iter().position(|m| matches!(m, Message::Editor(_, EditorMsg::PointerOver(..))));
    let text = messages.iter().position(|m| matches!(m, Message::Editor(_, EditorMsg::TextPressed)));
    assert!(pointer.is_some() && pointer < text, "{messages:?}");
}

/// The fold buttons of the nodes near the view take presses: the branch
/// handle of a node with children folds it, its fold marker unfolds it.
#[test]
fn the_fold_buttons_near_the_view_take_presses() {
    let mut b = Bench::new(3, 1);
    let main = b.app.main_window;
    let click = |b: &mut Bench, point: Point| {
        let (bounds, scroll) = b.shown(main).map.expect("the map");
        let at = bounds.position() + (Vector::new(point.x, point.y) - scroll);
        let _ = b.messages(main, moved(at), at, false);
        let _ = b.messages(main, Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)), at, false);
        let (messages, _) =
            b.messages(main, Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)), at, false);
        for m in &messages {
            let _ = b.app.update(m.clone());
        }
        b.rebuild();
        messages
    };
    let (bounds, scroll) = b.shown(main).map.expect("the map");
    let visible = Rectangle::new(Point::ORIGIN + scroll, bounds.size());
    let layout = b.app.doc.as_ref().unwrap().layout().clone();
    let node = layout
        .boxes
        .iter()
        .find(|n| n.parent.is_some() && !layout.visible_children(n.id).is_empty() && visible.contains(n.rect.center()))
        .expect("a node with children in view");
    // The branch handle: halfway to the children, at the node's underline.
    let first = layout.get(layout.visible_children(node.id)[0]).unwrap();
    let end = node.rect.x + node.rect.width;
    let handle = Point::new(end + (first.rect.x - end) / 2.0, node.rect.y + node.rect.height);
    let messages = click(&mut b, handle);
    assert!(matches!(messages[..], [Message::Map(MapMsg::FoldClicked(n))] if n == node.id), "{messages:?}");
    assert!(b.app.doc.as_ref().unwrap().map().is_folded(node.id));
    // The fold marker, right of the folded node.
    let folded = b.app.doc.as_ref().unwrap().layout().get(node.id).unwrap().rect;
    let marker = Point::new(folded.x + folded.width + 11.0, folded.center_y());
    let messages = click(&mut b, marker);
    assert!(matches!(messages[..], [Message::Map(MapMsg::FoldClicked(n))] if n == node.id), "{messages:?}");
    assert!(!b.app.doc.as_ref().unwrap().map().is_folded(node.id));
}
