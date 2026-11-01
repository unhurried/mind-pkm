//! Timing of the application's work, to find out what makes it slow. With the
//! environment variable `MIND_PKM_PROFILE` naming a file, every message
//! handled - and after it every window's view built, compared with the last
//! one, laid out, drawn, and the events it handled - is written to that file
//! with how long it took. Without it nothing is timed or written.
//!
//! A line is `<seconds since start> <phase> <what> <microseconds>`, separated
//! by tabs; `what` is the message for `Update`, else the window (`map`,
//! `editor`).

use iced::advanced::layout::{self, Layout};
use iced::advanced::overlay;
use iced::advanced::renderer;
use iced::advanced::widget::{self, Operation, Tree, Widget};
use iced::advanced::{Clipboard, Shell};
use iced::{Element, Event, Length, Rectangle, Size, Vector, mouse};
use std::cell::RefCell;
use std::fmt;
use std::fs::File;
use std::io::{LineWriter, Write as _};
use std::sync::{LazyLock, Mutex};
use std::time::{Duration, Instant};

/// The environment variable naming the file the timings are written to.
pub const ENV_VAR: &str = "MIND_PKM_PROFILE";

/// What was timed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Phase {
    /// Handling a message.
    Update,
    /// Building a window's view.
    View,
    /// Comparing it with the last one (the widgets' state).
    Diff,
    /// Laying it out.
    Layout,
    /// Handling an event (the pointer, a key) in it.
    Event,
    /// Drawing it.
    Draw,
}

/// One timing.
#[derive(Debug, Clone)]
pub struct Timing {
    pub phase: Phase,
    pub what: String,
    pub took: Duration,
}

struct FileSink {
    start: Instant,
    out: LineWriter<File>,
}

static FILE: LazyLock<Option<Mutex<FileSink>>> = LazyLock::new(|| {
    let path = std::env::var_os(ENV_VAR).filter(|p| !p.is_empty())?;
    let out = LineWriter::new(File::create(path).ok()?);
    Some(Mutex::new(FileSink { start: Instant::now(), out }))
});

thread_local! {
    /// Where the timings go while a benchmark collects them, instead of the file.
    static COLLECTED: RefCell<Option<Vec<Timing>>> = const { RefCell::new(None) };
}

/// Whether timings are taken.
pub fn enabled() -> bool {
    COLLECTED.with(|c| c.borrow().is_some()) || FILE.is_some()
}

/// Runs `f`, timing it as `phase` of `what` (asked for only when timing).
pub fn time<T>(phase: Phase, what: impl FnOnce() -> String, f: impl FnOnce() -> T) -> T {
    if !enabled() {
        return f();
    }
    let what = what();
    let start = Instant::now();
    let out = f();
    record(Timing { phase, what, took: start.elapsed() });
    out
}

fn record(timing: Timing) {
    let collected = COLLECTED.with(|c| match c.borrow_mut().as_mut() {
        Some(timings) => {
            timings.push(timing.clone());
            true
        }
        None => false,
    });
    if collected {
        return;
    }
    if let Some(sink) = FILE.as_ref()
        && let Ok(mut sink) = sink.lock()
    {
        let at = sink.start.elapsed().as_secs_f64();
        let Timing { phase, what, took } = timing;
        let _ = writeln!(sink.out, "{at:.3}\t{phase:?}\t{what}\t{}", took.as_micros());
    }
}

/// Starts collecting the timings of this thread (a benchmark), instead of
/// writing them to the file.
#[cfg(test)]
pub fn start_collecting() {
    COLLECTED.with(|c| *c.borrow_mut() = Some(Vec::new()));
}

/// The timings collected since `start_collecting`; collecting stops.
#[cfg(test)]
pub fn stop_collecting() -> Vec<Timing> {
    COLLECTED.with(|c| c.borrow_mut().take()).unwrap_or_default()
}

/// A short name for a message: its variants down to the first field that is
/// not one (`Key(KeyPressed`), without formatting all of it.
pub fn label(message: &impl fmt::Debug) -> String {
    /// Takes the start of what is written, up to the first character that
    /// ends a variant path, then stops the formatting.
    struct Head(String);
    impl fmt::Write for Head {
        fn write_str(&mut self, s: &str) -> fmt::Result {
            for c in s.chars() {
                if !(c.is_alphabetic() || c == '_' || c == '(') || self.0.len() > 60 {
                    return Err(fmt::Error);
                }
                self.0.push(c);
            }
            Ok(())
        }
    }
    let mut head = Head(String::new());
    let _ = fmt::write(&mut head, format_args!("{message:?}"));
    head.0.trim_end_matches('(').to_string()
}

/// `content`, its diffing, layout, drawing and event handling timed as the
/// window `name`'s.
pub fn timed<'a, Message: 'a, Theme: 'a, Renderer: renderer::Renderer + 'a>(
    name: &'static str,
    content: impl Into<Element<'a, Message, Theme, Renderer>>,
) -> Element<'a, Message, Theme, Renderer> {
    Element::new(Timed { name, content: content.into() })
}

struct Timed<'a, Message, Theme, Renderer> {
    name: &'static str,
    content: Element<'a, Message, Theme, Renderer>,
}

impl<Message, Theme, Renderer> Widget<Message, Theme, Renderer> for Timed<'_, Message, Theme, Renderer>
where
    Renderer: renderer::Renderer,
{
    fn size(&self) -> Size<Length> {
        self.content.as_widget().size()
    }

    fn size_hint(&self) -> Size<Length> {
        self.content.as_widget().size_hint()
    }

    fn layout(&mut self, tree: &mut Tree, renderer: &Renderer, limits: &layout::Limits) -> layout::Node {
        let name = self.name;
        time(Phase::Layout, || name.into(), || self.content.as_widget_mut().layout(tree, renderer, limits))
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        time(
            Phase::Draw,
            || self.name.into(),
            || self.content.as_widget().draw(tree, renderer, theme, style, layout, cursor, viewport),
        );
    }

    fn tag(&self) -> widget::tree::Tag {
        self.content.as_widget().tag()
    }

    fn state(&self) -> widget::tree::State {
        self.content.as_widget().state()
    }

    fn children(&self) -> Vec<Tree> {
        self.content.as_widget().children()
    }

    fn diff(&self, tree: &mut Tree) {
        time(Phase::Diff, || self.name.into(), || self.content.as_widget().diff(tree));
    }

    fn operate(&mut self, tree: &mut Tree, layout: Layout<'_>, renderer: &Renderer, operation: &mut dyn Operation) {
        self.content.as_widget_mut().operate(tree, layout, renderer, operation)
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        let name = self.name;
        time(
            Phase::Event,
            || name.into(),
            || self.content.as_widget_mut().update(tree, event, layout, cursor, renderer, clipboard, shell, viewport),
        );
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        self.content.as_widget().mouse_interaction(tree, layout, cursor, viewport, renderer)
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'b>,
        renderer: &Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, Message, Theme, Renderer>> {
        self.content.as_widget_mut().overlay(tree, layout, renderer, viewport, translation)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn labels_name_the_variants() {
        #[derive(Debug)]
        #[allow(dead_code)]
        enum Inner {
            Key { code: u32 },
            Moved(f32),
        }
        #[derive(Debug)]
        #[allow(dead_code)]
        enum Outer {
            Map(Inner),
            Tick,
        }
        assert_eq!(label(&Outer::Map(Inner::Key { code: 1 })), "Map(Key");
        assert_eq!(label(&Outer::Map(Inner::Moved(1.0))), "Map(Moved");
        assert_eq!(label(&Outer::Tick), "Tick");
    }

    #[test]
    fn timings_are_collected_only_while_collecting() {
        if FILE.is_some() {
            return; // Timing for the whole run.
        }
        assert_eq!(time(Phase::Update, || unreachable!(), || 1), 1);
        start_collecting();
        assert!(enabled());
        assert_eq!(time(Phase::View, || "map".into(), || 2), 2);
        let timings = stop_collecting();
        assert_eq!(timings.len(), 1);
        assert_eq!((timings[0].phase, timings[0].what.as_str()), (Phase::View, "map"));
        assert!(!enabled());
    }
}
