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
use super::editor::{Bind, EditorCommand, EditorMsg};
use super::profile::{self, Phase};
use super::scroll_watch::Visible;
use super::test_support::{headless_renderer, key_event, remove_in_temp};
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
/// inline markup, a list, a code block of 25 lines and a table.
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
        let _ = write!(
            s,
            "| Key | Value | Note |\n| --- | ---: | --- |\n| a{i} | 1 | short |\n\
             | b{i} | 22 | 日本語の少し長めのメモで、列の最大幅を超えて折り返すくらいの長さにしておく |\n\n"
        );
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
        let mut app = App::new(remove_in_temp);
        let _ = app.set_doc(f.map.clone(), None);
        app.doc.as_mut().unwrap().select(vec![page]);
        let _ = app.update(MapMsg::WindowResized(WINDOW).into());
        let _ = app.update(MapMsg::Key(key_event(Key::Named(Named::F1), Modifiers::empty())).into());
        let editor = *app.editors.keys().last().expect("the page opened in an editor");
        let _ = app.update(Message::Window(editor, WindowEvent::Resized(WINDOW)));
        let renderer = headless_renderer();
        let mut bench = Bench { app, renderer, caches: BTreeMap::new(), editor, nodes, _fixture: f };
        bench.rebuild();
        // As after a while: every block of the page measured.
        bench.measure();
        bench
    }

    /// What the running application does while the page has blocks left to
    /// measure: the page asks for a message when it is drawn, which builds
    /// the windows again. Returns how many it took.
    fn measure(&mut self) -> usize {
        let mut messages = 0;
        while self.app.editors[&self.editor].measuring() {
            assert!(messages < 10_000, "the page never got measured");
            self.unmeasured(Message::Editor(self.editor, EditorMsg::Measure));
            messages += 1;
        }
        messages
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
                let Vector { x, y } = self.app.viewport.scroll();
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

    /// The messages of events that come in one batch (as a quick click's
    /// press and release can), taken by the window's interface at once.
    fn batch(&mut self, id: window::Id, events: &[Event], cursor: Point) -> Vec<Message> {
        let cache = self.caches.remove(&id).unwrap_or_default();
        let mut ui = UserInterface::build(self.app.view(id), WINDOW, cache, &mut self.renderer);
        let mut messages = Vec::new();
        ui.update(events, mouse::Cursor::Available(cursor), &mut self.renderer, &mut clipboard::Null, &mut messages);
        self.caches.insert(id, ui.into_cache());
        messages
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
    run("map: typing in a node", &mut b, &mut |b, t, _| {
        b.step(
            t,
            MapMsg::NodeEdit(iced::widget::text_editor::Action::Edit(iced::widget::text_editor::Edit::Insert('x'))),
        )
    });
    b.unmeasured(MapMsg::Escape);
    run("map: tick (any message)", &mut b, &mut |b, t, _| b.step(t, MapMsg::Tick));

    // The editor window.
    let text = b.app.editors[&editor].page.text().to_string();
    let at = text.find("Paragraph 3 with").unwrap() + "Paragraph 3".len();
    b.unmeasured(Message::Editor(editor, EditorMsg::PressedAt(at)));
    b.unmeasured(Message::Editor(editor, EditorMsg::Released));
    run("editor: typing a character", &mut b, &mut |b, t, _| {
        b.step(t, Message::Editor(editor, EditorMsg::Action(Action::Edit(Edit::Insert('a')))))
    });
    // A block added and joined again, with most of the page after it.
    run("editor: Enter / Backspace", &mut b, &mut |b, t, i| {
        let bind = if i % 2 == 0 { Bind::Enter } else { Bind::Backspace };
        b.step(t, Message::Editor(editor, EditorMsg::Bind(bind)))
    });
    run("editor: pointer moves over text", &mut b, &mut |b, t, i| {
        let at = Point::new(260.0 + (i % 5) as f32 * 7.0, 120.0 + i as f32 * 23.0);
        b.event(t, editor, Event::Mouse(mouse::Event::CursorMoved { position: at }), at)
    });
    run("editor: wheel scroll", &mut b, &mut |b, t, _| {
        let at = Point::new(400.0, 400.0);
        let delta = mouse::ScrollDelta::Lines { x: 0.0, y: -2.0 };
        b.event(t, editor, Event::Mouse(mouse::Event::WheelScrolled { delta }), at)
    });
    // Every block measured again: those near the visible part at once, the
    // others with the messages after (not timed).
    run("editor: zoom in/out", &mut b, &mut |b, t, i| {
        let cmd = if i % 2 == 0 { EditorCommand::ZoomIn } else { EditorCommand::ZoomOut };
        b.step(t, Message::Editor(editor, EditorMsg::Command(cmd)));
        b.measure();
    });
    run("editor: resize (narrower page)", &mut b, &mut |b, t, i| {
        let size = Size::new(700.0 + (i % 2) as f32 * 40.0, WINDOW.height);
        b.step(t, Message::Window(editor, WindowEvent::Resized(size)));
        b.measure();
    });
    b.unmeasured(Message::Window(editor, WindowEvent::Resized(WINDOW)));
    b.measure();
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

#[test]
#[ignore = "a benchmark: run in release mode with --ignored --nocapture"]
fn perf_very_long_page() {
    report(3, 160);
}

/// What a window shows, as an operation finds it: where the map's scrollable
/// is and how far it is scrolled, the scrollables and containers with an id
/// (bounds, the content's bounds and how far it is scrolled; bounds), and the
/// texts of the text widgets.
#[derive(Default)]
struct Shown {
    map: Option<(Rectangle, Vector)>,
    scrollables: HashMap<widget::Id, (Rectangle, Rectangle, Vector)>,
    containers: Vec<(widget::Id, Rectangle)>,
    texts: HashSet<String>,
}

impl Operation for Shown {
    fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation)) {
        operate(self);
    }

    fn container(&mut self, id: Option<&widget::Id>, bounds: Rectangle) {
        if let Some(id) = id {
            self.containers.push((id.clone(), bounds));
        }
    }

    fn scrollable(
        &mut self,
        id: Option<&widget::Id>,
        bounds: Rectangle,
        content_bounds: Rectangle,
        translation: Vector,
        _state: &mut dyn operation::Scrollable,
    ) {
        if let Some(id) = id {
            self.scrollables.insert(id.clone(), (bounds, content_bounds, translation));
        }
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
        // Built: the nodes within the margin of where the map was last told
        // to be, which the visible part is within half a margin of.
        let near = visible.expand(1.5 * NEAR_MARGIN + 1.0);
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

/// Scrolling the map with the wheel tells the application where it is only
/// now and then - when the visible part comes near the edge of the nodes
/// built (each message builds every window again) - and the application
/// follows every step all the same (zooming and panning start from there).
#[test]
fn scrolling_the_map_tells_where_it_is_only_now_and_then() {
    let mut b = Bench::new(10, 1);
    let main = b.app.main_window;
    let at = Point::new(640.0, 400.0);
    let _ = b.messages(main, moved(at), at, false);
    let start = b.app.viewport.scroll();
    let mut sent = 0;
    for step in 0..40 {
        let delta = mouse::ScrollDelta::Lines { x: 0.0, y: 1.0 };
        let (messages, _) = b.messages(main, Event::Mouse(mouse::Event::WheelScrolled { delta }), at, false);
        assert!(messages.iter().all(|m| matches!(m, Message::Map(MapMsg::MapScrolled(_)))), "{messages:?}");
        sent += messages.len();
        if !messages.is_empty() {
            for m in messages {
                let _ = b.app.update(m);
            }
            b.rebuild();
        }
        // Followed at every step.
        if step % 8 == 7 {
            let (_, scroll) = b.shown(main).map.expect("the map");
            assert_eq!(b.app.viewport.scroll(), scroll);
        }
    }
    assert!(b.app.viewport.scroll().y < start.y - 1000.0, "{:?} from {start:?}", b.app.viewport.scroll());
    assert!((3..=20).contains(&sent), "{sent} messages in 40 steps");
}

/// After the map was scrolled by the wheel (followed without a message),
/// Ctrl+wheel zooms keeping the point under the pointer where it is.
#[test]
fn ctrl_wheel_zooms_at_the_pointer_after_scrolling() {
    let mut b = Bench::new(10, 1);
    let main = b.app.main_window;
    let at = Point::new(700.0, 400.0);
    let _ = b.messages(main, moved(at), at, false);
    for _ in 0..7 {
        let delta = mouse::ScrollDelta::Lines { x: 0.0, y: 1.0 };
        b.deliver(main, Event::Mouse(mouse::Event::WheelScrolled { delta }), at);
    }
    // The map point under the pointer, from where the map is shown.
    let point = |b: &mut Bench| {
        let (bounds, scroll) = b.shown(main).map.expect("the map");
        (Vector::new(at.x - bounds.x, at.y - bounds.y) + scroll, b.app.viewport.zoom())
    };
    let (before, zoom) = point(&mut b);
    let ctrl = Event::Keyboard(keyboard::Event::ModifiersChanged(Modifiers::CTRL));
    let _ = b.messages(main, ctrl, at, false);
    let wheel = Event::Mouse(mouse::Event::WheelScrolled { delta: mouse::ScrollDelta::Lines { x: 0.0, y: 1.0 } });
    b.deliver(main, wheel, at);
    let (after, zoomed) = point(&mut b);
    assert!(zoomed > zoom, "{zoom} to {zoomed}");
    // The same point of the map, at the zoom (beside the margin around it) -
    // but for the node sizes, rounded up to whole pixels at every zoom, which
    // add up along the nodes to the left (scrolled from where it was before
    // the wheel, it would be hundreds of pixels away).
    let margin = Vector::new(40.0, 40.0);
    let expected = margin + (before - margin) * (zoomed / zoom);
    assert!((after.x - expected.x).abs() < 8.0 && (after.y - expected.y).abs() < 8.0, "{after:?}, not {expected:?}");
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

/// The visible part of the page (in the coordinates the widgets are laid
/// out in), the height of its content, and what the window shows.
fn page_view(b: &mut Bench) -> (Rectangle, f32, Shown) {
    let doc = b.app.editors[&b.editor].id("doc");
    let shown = b.shown(b.editor);
    let (bounds, content, scroll) = shown.scrollables[&doc];
    (Rectangle::new(bounds.position() + scroll, bounds.size()), content.height, shown)
}

/// Whether the page's block `block` has widgets.
fn built(b: &Bench, shown: &Shown, block: usize) -> bool {
    let id = b.app.editors[&b.editor].id(&format!("block-{block}"));
    shown.containers.iter().any(|(i, _)| *i == id)
}

/// The block of the page whose source starts with `start`.
fn block_of(b: &Bench, start: &str) -> usize {
    let page = &b.app.editors[&b.editor].page;
    page.doc().blocks.iter().position(|bl| page.text()[bl.range.clone()].starts_with(start)).unwrap()
}

/// Nothing of the visible part of the page is left without widgets.
fn assert_all_in_view_built(b: &mut Bench) {
    let (visible, _, shown) = page_view(b);
    let far = b.app.editors[&b.editor].id("far");
    for (id, bounds) in &shown.containers {
        assert!(*id != far || !bounds.intersects(&visible), "{bounds:?} has no widgets, in view {visible:?}");
    }
}

/// Scrolls the page by `steps` steps of the wheel (down when positive),
/// handling the messages that sends; how many there were.
fn scroll_page(b: &mut Bench, steps: i32) -> usize {
    let at = Point::new(400.0, 400.0);
    let editor = b.editor;
    let _ = b.messages(editor, moved(at), at, false);
    let mut sent = 0;
    for _ in 0..steps.abs() {
        let delta = mouse::ScrollDelta::Lines { x: 0.0, y: -2.0 * steps.signum() as f32 };
        let (messages, _) = b.messages(editor, Event::Mouse(mouse::Event::WheelScrolled { delta }), at, false);
        sent += messages.len();
        if !messages.is_empty() {
            for m in messages {
                let _ = b.app.update(m);
            }
            b.rebuild();
        }
    }
    sent
}

/// The height of the page's content with every block built (as when the
/// visible part were taller than the page), leaving the page as it was.
fn height_built_whole(b: &mut Bench) -> f32 {
    let editor = b.editor;
    let (visible, _, _) = page_view(b);
    let told = |height: f32| {
        let visible = Visible { top: visible.y, height, left: 0.0, width: visible.width };
        Message::Editor(editor, EditorMsg::PageScrolled(visible))
    };
    b.unmeasured(told(1.0e7));
    let (_, height, shown) = page_view(b);
    assert!(!shown.containers.iter().any(|(id, _)| *id == b.app.editors[&editor].id("far")));
    b.unmeasured(told(visible.height));
    height
}

/// A page builds widgets for the blocks near the part it shows, wherever it
/// is scrolled to, and is as tall as with every block built.
#[test]
fn the_page_builds_widgets_for_the_blocks_near_what_it_shows() {
    let mut b = Bench::new(3, 40);
    let (first, last) = (block_of(&b, "## Section 0"), block_of(&b, "## Section 39"));
    let height = height_built_whole(&mut b);
    // Those near the visible part only.
    let (_, near_top, shown) = page_view(&mut b);
    assert!(built(&b, &shown, first) && !built(&b, &shown, last));
    assert!((near_top - height).abs() < 0.5, "{near_top} tall, built whole {height}");
    assert_all_in_view_built(&mut b);
    // Down the page: what comes into view is built, what is far behind is not.
    let sent = scroll_page(&mut b, 150);
    assert!((3..75).contains(&sent), "{sent} messages");
    let (visible, further, shown) = page_view(&mut b);
    assert!(visible.y > 10000.0, "{visible:?}");
    assert!(!built(&b, &shown, first) && !built(&b, &shown, last));
    assert!((further - height).abs() < 0.5, "{further} tall, built whole {height}");
    assert_all_in_view_built(&mut b);
    // Back up.
    scroll_page(&mut b, -200);
    let doc = b.app.editors[&b.editor].id("doc");
    let shown = b.shown(b.editor);
    assert_eq!(shown.scrollables[&doc].2.y, 0.0, "at the top");
    assert!(built(&b, &shown, first));
    assert_all_in_view_built(&mut b);
}

/// At another zoom (or width, or when just opened) the blocks' heights are
/// not known: those near the visible part are built at once, the others
/// take a guess and are measured a few at a time, with the messages after -
/// and the page ends up as tall as with every block built.
#[test]
fn blocks_of_unknown_height_are_measured_a_few_at_a_time() {
    let mut b = Bench::new(3, 40);
    let editor = b.editor;
    let (first, last) = (block_of(&b, "## Section 0"), block_of(&b, "## Section 39"));
    b.unmeasured(Message::Editor(editor, EditorMsg::Command(EditorCommand::ZoomIn)));
    let (_, _, shown) = page_view(&mut b);
    assert!(built(&b, &shown, first) && !built(&b, &shown, last));
    assert!(b.app.editors[&editor].measuring());
    assert_all_in_view_built(&mut b);
    let messages = b.measure();
    assert!((5..200).contains(&messages), "{messages} messages");
    let (_, measured, shown) = page_view(&mut b);
    assert!(built(&b, &shown, first) && !built(&b, &shown, last));
    let height = height_built_whole(&mut b);
    assert!((measured - height).abs() < 0.5, "{measured} tall, built whole {height}");
}

/// Scrolls the page to `y` (from the top of its content) as an operation
/// does, and lets the page tell where it is.
fn scroll_page_to(b: &mut Bench, y: f32) {
    let editor = b.editor;
    let doc = b.app.editors[&editor].id("doc");
    let cache = b.caches.remove(&editor).unwrap_or_default();
    let mut ui = UserInterface::build(b.app.view(editor), WINDOW, cache, &mut b.renderer);
    ui.operate(&b.renderer, &mut operation::scrollable::scroll_to(doc, AbsoluteOffset { x: 0.0, y }.into()));
    b.caches.insert(editor, ui.into_cache());
    let redraw = Event::Window(window::Event::RedrawRequested(iced::time::Instant::now()));
    b.deliver(editor, redraw, Point::new(400.0, 400.0));
}

/// Where the top of the page's block `block` is below the top of the
/// visible part (None: it has no widgets).
fn below_top(b: &mut Bench, block: usize) -> Option<f32> {
    let (visible, _, shown) = page_view(b);
    let id = b.app.editors[&b.editor].id(&format!("block-{block}"));
    shown.containers.iter().find(|(i, _)| *i == id).map(|(_, bounds)| bounds.y - visible.y)
}

/// As blocks above the visible part are measured (and grow or shrink from
/// their guesses), and when the page is zoomed, the block at the top of the
/// visible part stays where it is.
#[test]
fn the_block_at_the_top_stays_in_place() {
    let mut b = Bench::new(3, 40);
    let editor = b.editor;
    // Scrolled to put a heading far down the page at the top.
    let block = block_of(&b, "## Section 30");
    let heading = b.app.editors[&editor].page.doc().headings.iter().position(|h| h.block == block).unwrap();
    b.unmeasured(Message::Editor(editor, EditorMsg::HeadingClicked(heading)));
    let at = below_top(&mut b, block).expect("the heading scrolled to has widgets");
    let (visible, _, _) = page_view(&mut b);
    let doc = b.app.editors[&editor].id("doc");
    let top = visible.y - b.shown(editor).scrollables[&doc].0.y;
    scroll_page_to(&mut b, top + at);
    let at = below_top(&mut b, block).expect("the heading in view");
    assert!(at.abs() < 1.0, "the heading at the top: {at}");
    // Zoomed: the heights above are guesses, then measured.
    for cmd in [EditorCommand::ZoomIn, EditorCommand::ZoomOut] {
        b.unmeasured(Message::Editor(editor, EditorMsg::Command(cmd)));
        let at = below_top(&mut b, block).expect("the heading in view");
        assert!(at.abs() < 1.0, "zoomed: the heading at {at}");
        assert!(b.app.editors[&editor].measuring());
        b.measure();
        let at = below_top(&mut b, block).expect("the heading in view");
        assert!(at.abs() < 1.0, "measured: the heading at {at}");
    }
}

/// Scrolling a page tells the application where it is only when the visible
/// part comes near the end of the blocks built around it (each message
/// builds every window again).
#[test]
fn scrolling_a_page_tells_where_it_is_only_near_the_end_of_what_was_built() {
    let mut b = Bench::new(3, 40);
    let editor = b.editor;
    b.unmeasured(Message::Editor(editor, EditorMsg::CloseContextMenu));
    let at = Point::new(400.0, 400.0);
    let _ = b.messages(editor, moved(at), at, false);
    let wheel = || Event::Mouse(mouse::Event::WheelScrolled { delta: mouse::ScrollDelta::Lines { x: 0.0, y: -2.0 } });
    // The steps of the wheel that send nothing, and the messages of the next.
    let quiet = |b: &mut Bench| {
        for steps in 0..100 {
            let (messages, _) = b.messages(editor, wheel(), at, false);
            if !messages.is_empty() {
                return (steps, messages);
            }
        }
        panic!("scrolled without telling where to");
    };
    // Some steps send nothing, then one tells where the page is.
    let (steps, messages) = quiet(&mut b);
    assert!(steps >= 2, "{steps}");
    assert!(matches!(messages[..], [Message::Editor(_, EditorMsg::PageScrolled(_))]), "{messages:?}");
    for m in messages {
        let _ = b.app.update(m);
    }
    b.rebuild();
    // Built around where it is now: quiet again for a while.
    assert!(quiet(&mut b).0 >= 2);
}

/// A heading far from the visible part that the page is scrolled to (from
/// the outline, a link) gets its widgets, for the scroll to find it.
#[test]
fn a_block_scrolled_to_gets_widgets_wherever_it_is() {
    let mut b = Bench::new(3, 40);
    let editor = b.editor;
    b.unmeasured(Message::Editor(editor, EditorMsg::CloseContextMenu));
    let block = block_of(&b, "## Section 35");
    let shown = b.shown(editor);
    assert!(!built(&b, &shown, block));
    let heading = b.app.editors[&editor].page.doc().headings.iter().position(|h| h.block == block).unwrap();
    b.unmeasured(Message::Editor(editor, EditorMsg::HeadingClicked(heading)));
    let shown = b.shown(editor);
    assert!(built(&b, &shown, block));
}

/// The block being edited keeps its widgets (its focus, what is being
/// composed) when the page is scrolled far from it.
#[test]
fn the_block_being_edited_keeps_its_widgets_far_from_view() {
    let mut b = Bench::new(3, 40);
    let editor = b.editor;
    let text = b.app.editors[&editor].page.text().to_string();
    let at = text.find("Paragraph 3 with").unwrap() + "Paragraph 3".len();
    b.unmeasured(Message::Editor(editor, EditorMsg::PressedAt(at)));
    b.unmeasured(Message::Editor(editor, EditorMsg::Released));
    let active = b.app.editors[&editor].id("active-box");
    let has_active = |b: &mut Bench| b.shown(editor).containers.iter().any(|(id, _)| *id == active);
    assert!(has_active(&mut b));
    scroll_page(&mut b, 150);
    let (visible, _, _) = page_view(&mut b);
    assert!(visible.y > 10000.0, "{visible:?}");
    assert!(has_active(&mut b));
    b.unmeasured(Message::Editor(editor, EditorMsg::Live(super::editor::LiveMsg::Insert("x".into()))));
    assert!(b.app.editors[&editor].page.text().contains("Paragraph 3x with"));
    assert!(has_active(&mut b));
}

/// A quick click on a page's text, its press and release in one batch of
/// events (before the subscription waiting for the release runs), leaves no
/// button held: a text editor shown later (a block turned into a math block
/// by typing) is not handed that click.
#[test]
fn a_quick_click_on_a_page_leaves_no_button_held() {
    let mut b = Bench::new(3, 6);
    let editor = b.editor;
    let press = Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left));
    let release = Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left));
    let clicked = (0..60).find_map(|i| {
        let at = Point::new(160.0, 40.0 + i as f32 * 12.0);
        let _ = b.messages(editor, moved(at), at, false);
        let messages = b.batch(editor, &[press.clone(), release.clone()], at);
        messages.iter().any(|m| matches!(m, Message::Editor(_, EditorMsg::TextPressed))).then_some(messages)
    });
    let messages = clicked.expect("a click on text");
    let released = messages.iter().rposition(|m| matches!(m, Message::Editor(_, EditorMsg::Released)));
    let pressed = messages.iter().position(|m| matches!(m, Message::Editor(_, EditorMsg::TextPressed)));
    assert!(released > pressed, "{messages:?}");
    for m in messages {
        let _ = b.app.update(m);
    }
    let e = &b.app.editors[&editor];
    assert!(!e.pressed() && e.held(iced::Padding::ZERO).is_none());
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
