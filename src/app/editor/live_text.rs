//! The widget editing text in its rendered form: styled runs of text, images
//! and formulas laid out in lines (`flow`), with a caret and a selection
//! drawn over them. It owns no text: the editor gives it the runs and the
//! selection (offsets in the shown text) every time, and it turns clicks,
//! keys and the IME into messages - offsets where the layout is needed (a
//! click, Up/Down, Home/End), motions and insertions otherwise.

use super::flow::{self, Cluster, Flow, Measured};
use super::keys::Bind;
use super::{EditorCommand, EditorMsg};
use iced::advanced::Renderer as _;
use iced::advanced::graphics::text::Paragraph;
use iced::advanced::layout::{self, Layout};
use iced::advanced::renderer::{self, Quad};
use iced::advanced::text::{self as core_text, Paragraph as _, Renderer as _};
use iced::advanced::widget::{self, Tree, Widget, operation, tree};
use iced::advanced::{Clipboard, Shell, image as core_image, input_method, mouse, svg as core_svg};
use iced::time::{Duration, Instant};
use iced::widget::text::{LineHeight, Shaping, Wrapping};
use iced::widget::text_editor::{Binding, KeyPress, Motion, Status};
use iced::{Color, Element, Event, Font, Length, Pixels, Point, Rectangle, Renderer, Size, Theme, keyboard, window};
use std::collections::HashMap;
use std::ops::Range;

/// What the widget asks of the editor.
#[derive(Debug, Clone, PartialEq)]
pub enum LiveMsg {
    /// A click at a shown offset (Shift: extending the selection).
    Click {
        at: usize,
        extend: bool,
    },
    Drag(usize),
    SelectWord(usize),
    SelectAll,
    /// The cursor to a shown offset the layout found (Up, Down, Home, End).
    MoveTo {
        at: usize,
        select: bool,
    },
    /// A motion that needs no layout (characters, words, the block's ends).
    Motion {
        motion: Motion,
        select: bool,
    },
    Insert(String),
}

/// How a piece of text looks.
#[derive(Debug, Clone, PartialEq)]
pub struct TextLook {
    pub font: Font,
    pub size: f32,
    pub color: Color,
    pub underline: bool,
    pub strike: bool,
    pub background: Option<Color>,
    /// A link's destination and its source offset (Ctrl+click follows it).
    pub link: Option<(String, usize)>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum RunKind {
    /// Text; its characters are the shown text's.
    Text {
        text: String,
        look: TextLook,
    },
    /// One label standing for the run's shown range (a footnote number...).
    Label {
        text: String,
        look: TextLook,
    },
    Image {
        handle: core_image::Handle,
        max_width: Option<f32>,
    },
    /// An SVG file shown as an image.
    SvgFile {
        handle: core_svg::Handle,
        max_width: Option<f32>,
    },
    /// An SVG (a formula) of a size; `center`ed on the text, else on the
    /// baseline; `display` math in the middle of a line of its own.
    Svg {
        handle: core_svg::Handle,
        width: f32,
        height: f32,
        center: bool,
        display: bool,
    },
    Newline,
}

/// A run over the shown range `shown`; a preview (of an image or formula at
/// the cursor) has an empty range and takes no place in the text.
#[derive(Debug, Clone, PartialEq)]
pub struct Run {
    pub shown: Range<usize>,
    pub kind: RunKind,
}

pub struct LiveText<'a> {
    id: widget::Id,
    runs: Vec<Run>,
    len: usize,
    /// (anchor, head) in the shown text.
    sel: (usize, usize),
    /// Ranges drawn with a background (find matches).
    marks: Vec<(Range<usize>, Color)>,
    /// The font, size and line height of an empty line.
    base: (Font, f32),
    line_height: f32,
    selection: Color,
    caret: Color,
    key_binding: Box<dyn Fn(KeyPress) -> Option<Binding<EditorMsg>> + 'a>,
}

impl<'a> LiveText<'a> {
    pub fn new(id: widget::Id, runs: Vec<Run>, len: usize, sel: (usize, usize)) -> Self {
        LiveText {
            id,
            runs,
            len,
            sel,
            marks: Vec::new(),
            base: (Font::DEFAULT, 16.0),
            line_height: 1.6,
            selection: Color::from_rgba(0.25, 0.5, 0.95, 0.25),
            caret: Color::BLACK,
            key_binding: Box::new(Binding::from_key_press),
        }
    }

    pub fn base(mut self, font: Font, size: f32, line_height: f32) -> Self {
        self.base = (font, size);
        self.line_height = line_height;
        self
    }

    pub fn colors(mut self, selection: Color, caret: Color) -> Self {
        self.selection = selection;
        self.caret = caret;
        self
    }

    pub fn marks(mut self, marks: Vec<(Range<usize>, Color)>) -> Self {
        self.marks = marks;
        self
    }

    pub fn key_binding(mut self, f: impl Fn(KeyPress) -> Option<Binding<EditorMsg>> + 'a) -> Self {
        self.key_binding = Box::new(f);
        self
    }
}

/// A shaped text: its paragraph (drawn), clusters and vertical metrics.
struct Shaped {
    paragraph: Paragraph,
    clusters: Vec<Cluster>,
    ascent: f32,
    descent: f32,
    width: f32,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct ShapeKey {
    text: String,
    font: Font,
    size: u32,
    line_height: u32,
}

struct Focus {
    updated_at: Instant,
    now: Instant,
    is_window_focused: bool,
}

impl Focus {
    const BLINK: u128 = 500;

    fn now() -> Focus {
        let now = Instant::now();
        Focus { updated_at: now, now, is_window_focused: true }
    }

    fn caret_visible(&self) -> bool {
        self.is_window_focused && ((self.now - self.updated_at).as_millis() / Self::BLINK).is_multiple_of(2)
    }
}

#[derive(Default)]
struct State {
    focus: Option<Focus>,
    last_click: Option<mouse::Click>,
    dragging: bool,
    modifiers: keyboard::Modifiers,
    preedit: Option<input_method::Preedit>,
    cache: HashMap<ShapeKey, Shaped>,
    /// The keys of the runs as last laid out (None: not text).
    keys: Vec<Option<ShapeKey>>,
    items: Vec<Measured>,
    flow: Flow,
    /// The x Up and Down keep.
    preferred_x: Option<f32>,
    last_head: usize,
}

impl operation::Focusable for State {
    fn is_focused(&self) -> bool {
        self.focus.is_some()
    }

    fn focus(&mut self) {
        self.focus = Some(Focus::now());
    }

    fn unfocus(&mut self) {
        self.focus = None;
    }
}

/// The caret's rectangle in the window, reported to the scrolling operation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CaretRect(pub Rectangle);

fn shape(text: &str, font: Font, size: f32, line_height: f32, shown_start: usize) -> Shaped {
    let paragraph = Paragraph::with_text(core_text::Text {
        content: text,
        bounds: Size::INFINITE,
        size: Pixels(size),
        line_height: LineHeight::Relative(line_height),
        font,
        align_x: core_text::Alignment::Left,
        align_y: iced::alignment::Vertical::Top,
        shaping: Shaping::Advanced,
        wrapping: Wrapping::None,
    });
    let mut clusters: Vec<Cluster> = Vec::new();
    let mut ascent = size * line_height * 0.6;
    let mut descent = size * line_height * 0.4;
    let mut width: f32 = 0.0;
    if let Some(run) = paragraph.buffer().layout_runs().next() {
        ascent = run.line_y - run.line_top;
        descent = run.line_height - ascent;
        width = run.line_w;
        for g in run.glyphs {
            match clusters.last_mut() {
                Some(c) if c.start == shown_start + g.start => c.w = (g.x + g.w - c.x).max(c.w),
                _ => {
                    let ch = text[g.start..].chars().next().unwrap_or(' ');
                    clusters.push(Cluster {
                        start: shown_start + g.start,
                        end: shown_start + g.end,
                        x: g.x,
                        w: g.w,
                        space: ch == ' ' || ch == '\t' || ch == '\u{3000}',
                        break_before: flow::is_cjk(ch),
                        no_break_after: flow::no_break_after(ch),
                        no_break_before: flow::no_break_before(ch),
                    })
                }
            }
        }
    }
    clusters.sort_by_key(|c| c.start);
    // Every byte in a cluster: each one reaches the next.
    let end = shown_start + text.len();
    for i in 0..clusters.len() {
        clusters[i].end = clusters.get(i + 1).map_or(end, |n| n.start);
    }
    Shaped { paragraph, clusters, ascent, descent, width }
}

impl LiveText<'_> {
    fn key_of(&self, run: &Run) -> Option<ShapeKey> {
        match &run.kind {
            RunKind::Text { text, look } | RunKind::Label { text, look } => Some(ShapeKey {
                text: text.clone(),
                font: look.font,
                size: look.size.to_bits(),
                line_height: self.line_height.to_bits(),
            }),
            _ => None,
        }
    }

    fn empty_metrics(&self, state: &mut State) -> (f32, f32) {
        let key = ShapeKey {
            text: " ".into(),
            font: self.base.0,
            size: self.base.1.to_bits(),
            line_height: self.line_height.to_bits(),
        };
        let shaped =
            state.cache.entry(key).or_insert_with(|| shape(" ", self.base.0, self.base.1, self.line_height, 0));
        (shaped.ascent, shaped.descent)
    }

    fn measure(&self, state: &mut State, renderer: &Renderer, width: f32) {
        let mut keys = Vec::with_capacity(self.runs.len());
        let mut items = Vec::with_capacity(self.runs.len());
        for run in &self.runs {
            let key = self.key_of(run);
            let item = match &run.kind {
                RunKind::Text { text, look } => {
                    let k = key.clone().unwrap_or_else(|| unreachable!());
                    let shaped =
                        state.cache.entry(k).or_insert_with(|| shape(text, look.font, look.size, self.line_height, 0));
                    let clusters = shaped
                        .clusters
                        .iter()
                        .map(|c| Cluster { start: c.start + run.shown.start, end: c.end + run.shown.start, ..*c })
                        .collect();
                    Measured::Text { clusters, ascent: shaped.ascent, descent: shaped.descent }
                }
                RunKind::Label { text, look } => {
                    let k = key.clone().unwrap_or_else(|| unreachable!());
                    let shaped =
                        state.cache.entry(k).or_insert_with(|| shape(text, look.font, look.size, self.line_height, 0));
                    object_or_decoration(&run.shown, shaped.width, shaped.ascent, shaped.descent, false, false)
                }
                RunKind::Image { handle, max_width } => {
                    let natural = core_image::Renderer::measure_image(renderer, handle)
                        .map_or(Size::new(0.0, 0.0), |s| Size::new(s.width as f32, s.height as f32));
                    let w = natural.width.min(max_width.unwrap_or(f32::INFINITY)).min(width.max(1.0));
                    let h = if natural.width > 0.0 { natural.height * w / natural.width } else { 0.0 };
                    object_or_decoration(&run.shown, w, h, 0.0, true, false)
                }
                RunKind::SvgFile { handle, max_width } => {
                    let natural = core_svg::Renderer::measure_svg(renderer, handle);
                    let natural = Size::new(natural.width as f32, natural.height as f32);
                    let w = natural.width.min(max_width.unwrap_or(f32::INFINITY)).min(width.max(1.0));
                    let h = if natural.width > 0.0 { natural.height * w / natural.width } else { 0.0 };
                    object_or_decoration(&run.shown, w, h, 0.0, true, false)
                }
                RunKind::Svg { width: w, height: h, center, display, .. } => {
                    // A formula sits around the middle of the text's lowercase letters.
                    let axis = if *center { self.base.1 * 0.3 } else { 0.0 };
                    let ascent = if *center { h / 2.0 + axis } else { *h };
                    object_or_decoration(&run.shown, *w, ascent, (h - ascent).max(0.0), false, *display)
                }
                RunKind::Newline => Measured::Newline { at: run.shown.start },
            };
            keys.push(key);
            items.push(item);
        }
        let empty = self.empty_metrics(state);
        // Forget shapes no run uses any more (the empty line's stays).
        state.cache.retain(|k, _| k.text == " " || keys.iter().any(|x| x.as_ref() == Some(k)));
        state.flow = flow::layout(&items, width, empty, self.len);
        state.items = items;
        state.keys = keys;
    }

    fn caret_rect(&self, state: &State, origin: Point) -> Rectangle {
        let c = state.flow.caret(&state.items, self.sel.1);
        let line = &state.flow.lines[c.line];
        Rectangle::new(Point::new(origin.x + c.x, origin.y + line.top), Size::new(1.5, line.height))
    }
}

fn object_or_decoration(
    shown: &Range<usize>,
    width: f32,
    ascent: f32,
    descent: f32,
    block: bool,
    centered: bool,
) -> Measured {
    if shown.is_empty() {
        Measured::Decoration { at: shown.start, width, ascent, descent, block }
    } else {
        Measured::Object { shown: shown.clone(), width, ascent, descent, centered }
    }
}

impl Widget<EditorMsg, Theme, Renderer> for LiveText<'_> {
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(State::default())
    }

    fn size(&self) -> Size<Length> {
        Size::new(Length::Fill, Length::Shrink)
    }

    fn layout(&mut self, tree: &mut Tree, renderer: &Renderer, limits: &layout::Limits) -> layout::Node {
        let state = tree.state.downcast_mut::<State>();
        let width = limits.max().width;
        let width = if width.is_finite() { width } else { 800.0 };
        self.measure(state, renderer, width);
        layout::Node::new(Size::new(width, state.flow.height))
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        _renderer: &Renderer,
        operation: &mut dyn widget::Operation,
    ) {
        let state = tree.state.downcast_mut::<State>();
        let mut caret = CaretRect(self.caret_rect(state, layout.bounds().position()));
        operation.custom(Some(&self.id), layout.bounds(), &mut caret);
        operation.focusable(Some(&self.id), layout.bounds(), state);
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _renderer: &Renderer,
        _clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, EditorMsg>,
        _viewport: &Rectangle,
    ) {
        let state = tree.state.downcast_mut::<State>();
        let bounds = layout.bounds();
        let hit = |state: &State, p: Point| {
            let local = p - bounds.position();
            state.flow.hit(&state.items, local.x, local.y.clamp(0.0, (state.flow.height - 1.0).max(0.0)))
        };
        match event {
            Event::Window(window::Event::Unfocused) => {
                if let Some(f) = &mut state.focus {
                    f.is_window_focused = false;
                }
            }
            Event::Window(window::Event::Focused) => {
                if let Some(f) = &mut state.focus {
                    f.is_window_focused = true;
                    f.updated_at = Instant::now();
                    shell.request_redraw();
                }
            }
            Event::Window(window::Event::RedrawRequested(now)) => {
                if state.last_head != self.sel.1 {
                    state.last_head = self.sel.1;
                    if let Some(f) = &mut state.focus {
                        f.updated_at = *now;
                    }
                }
                if let Some(f) = &mut state.focus
                    && f.is_window_focused
                {
                    f.now = *now;
                    let wait = Focus::BLINK - (f.now - f.updated_at).as_millis() % Focus::BLINK;
                    shell.request_redraw_at(f.now + Duration::from_millis(wait as u64));
                }
                let method = match &state.focus {
                    Some(Focus { is_window_focused: true, .. }) => input_method::InputMethod::Enabled {
                        cursor: self.caret_rect(state, bounds.position()),
                        purpose: input_method::Purpose::Normal,
                        preedit: state.preedit.as_ref().map(input_method::Preedit::as_ref),
                    },
                    _ => input_method::InputMethod::Disabled,
                };
                shell.request_input_method(&method);
            }
            Event::Keyboard(keyboard::Event::ModifiersChanged(m)) => state.modifiers = *m,
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                if let Some(p) = cursor.position_over(bounds) {
                    let click = mouse::Click::new(p, mouse::Button::Left, state.last_click);
                    state.last_click = Some(click);
                    state.focus = Some(Focus::now());
                    state.preferred_x = None;
                    let at = hit(state, p);
                    let msg = match click.kind() {
                        mouse::click::Kind::Single => {
                            let link = (state.modifiers.control() || state.modifiers.command())
                                .then(|| self.link_at(state, p - bounds.position()))
                                .flatten();
                            match link {
                                Some((dest, source)) => EditorMsg::Link(dest, source),
                                None => {
                                    state.dragging = true;
                                    EditorMsg::Live(LiveMsg::Click { at, extend: state.modifiers.shift() })
                                }
                            }
                        }
                        mouse::click::Kind::Double => EditorMsg::Live(LiveMsg::SelectWord(at)),
                        mouse::click::Kind::Triple => EditorMsg::Live(LiveMsg::SelectAll),
                    };
                    shell.publish(msg);
                    shell.capture_event();
                } else if state.focus.is_some() {
                    state.focus = None;
                }
            }
            Event::Mouse(mouse::Event::CursorMoved { .. }) if state.dragging => {
                if let Some(p) = cursor.position() {
                    let at = hit(state, p);
                    shell.publish(EditorMsg::Live(LiveMsg::Drag(at)));
                }
            }
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => state.dragging = false,
            Event::InputMethod(event) => match event {
                input_method::Event::Opened | input_method::Event::Closed => {
                    state.preedit = matches!(event, input_method::Event::Opened).then(input_method::Preedit::new);
                    shell.request_redraw();
                }
                input_method::Event::Preedit(content, selection) if state.focus.is_some() => {
                    state.preedit = Some(input_method::Preedit {
                        content: content.clone(),
                        selection: selection.clone(),
                        text_size: Some(Pixels(self.base.1)),
                    });
                    shell.request_redraw();
                }
                input_method::Event::Commit(text) if state.focus.is_some() => {
                    state.preferred_x = None;
                    shell.publish(EditorMsg::Live(LiveMsg::Insert(text.clone())));
                    shell.capture_event();
                }
                _ => {}
            },
            Event::Keyboard(keyboard::Event::KeyPressed {
                key, modified_key, physical_key, modifiers, text, ..
            }) if state.focus.is_some() => {
                state.modifiers = *modifiers;
                let press = KeyPress {
                    key: key.clone(),
                    modified_key: modified_key.clone(),
                    physical_key: *physical_key,
                    modifiers: *modifiers,
                    text: text.clone(),
                    status: Status::Focused { is_hovered: cursor.is_over(bounds) },
                };
                if let Some(binding) = (self.key_binding)(press) {
                    if !matches!(binding, Binding::Unfocus) {
                        shell.capture_event();
                    }
                    self.apply(binding, state, shell);
                    if let Some(f) = &mut state.focus {
                        f.updated_at = Instant::now();
                    }
                }
            }
            _ => {}
        }
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        _theme: &Theme,
        _style: &renderer::Style,
        layout: Layout<'_>,
        _cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        let state = tree.state.downcast_ref::<State>();
        let bounds = layout.bounds();
        let origin = bounds.position();
        let flow = &state.flow;
        let quad = |renderer: &mut Renderer, r: Rectangle, color: Color| {
            renderer.fill_quad(Quad { bounds: r, ..Quad::default() }, color);
        };
        // Backgrounds of styled text (code, highlight), then marks and the selection.
        for line in &flow.lines {
            for f in &line.frags {
                if let Some(Run { kind: RunKind::Text { look, .. } | RunKind::Label { look, .. }, .. }) =
                    self.runs.get(f.item)
                    && let Some(bg) = look.background
                {
                    let top = origin.y + line.baseline - look.size * 1.0;
                    quad(
                        renderer,
                        Rectangle::new(Point::new(origin.x + f.x, top), Size::new(f.width, look.size * 1.3)),
                        bg,
                    );
                }
            }
        }
        let rect = |(x, y, w, h): (f32, f32, f32, f32)| {
            Rectangle::new(Point::new(origin.x + x, origin.y + y), Size::new(w, h))
        };
        for (range, color) in &self.marks {
            for r in flow.rects(&state.items, range.clone()) {
                quad(renderer, rect(r), *color);
            }
        }
        let (a, h) = self.sel;
        for r in flow.rects(&state.items, a.min(h)..a.max(h)) {
            quad(renderer, rect(r), self.selection);
        }
        // Text, decorations and objects.
        for line in &flow.lines {
            let line_clip = Rectangle::new(
                Point::new(bounds.x - 1000.0, origin.y + line.top),
                Size::new(bounds.width + 2000.0, line.height.max(1.0)),
            );
            for f in &line.frags {
                let Some(run) = self.runs.get(f.item) else { continue };
                match &run.kind {
                    RunKind::Text { look, .. } | RunKind::Label { look, .. } => {
                        let Some(Some(key)) = state.keys.get(f.item) else { continue };
                        let Some(shaped) = state.cache.get(key) else { continue };
                        let continues_left = f.clusters.start > 0 && matches!(run.kind, RunKind::Text { .. });
                        let continues_right = match &state.items[f.item] {
                            Measured::Text { clusters, .. } => f.clusters.end < clusters.len(),
                            _ => false,
                        };
                        let left = if continues_left { origin.x + f.x } else { line_clip.x };
                        let right =
                            if continues_right { origin.x + f.x + f.width } else { line_clip.x + line_clip.width };
                        let clip = Rectangle::new(
                            Point::new(left, line_clip.y - line.height),
                            Size::new(right - left, line.height * 3.0),
                        );
                        let Some(clip) = clip.intersection(viewport) else { continue };
                        let position = Point::new(origin.x + f.x - f.run_x, origin.y + line.baseline - shaped.ascent);
                        renderer.fill_paragraph(&shaped.paragraph, position, look.color, clip);
                        let thickness = (look.size / 14.0).max(1.0);
                        if look.underline {
                            let y = origin.y + line.baseline + look.size * 0.12;
                            quad(
                                renderer,
                                Rectangle::new(Point::new(origin.x + f.x, y), Size::new(f.width, thickness)),
                                look.color,
                            );
                        }
                        if look.strike {
                            let y = origin.y + line.baseline - look.size * 0.3;
                            quad(
                                renderer,
                                Rectangle::new(Point::new(origin.x + f.x, y), Size::new(f.width, thickness)),
                                look.color,
                            );
                        }
                    }
                    RunKind::Image { handle, .. } => {
                        if let Measured::Object { ascent, .. } | Measured::Decoration { ascent, .. } =
                            state.items[f.item]
                        {
                            let r = Rectangle::new(
                                Point::new(origin.x + f.x, origin.y + line.baseline - ascent),
                                Size::new(f.width, ascent),
                            );
                            core_image::Renderer::draw_image(
                                renderer,
                                core_image::Image::new(handle.clone()),
                                r,
                                *viewport,
                            );
                        }
                    }
                    RunKind::Svg { handle, height, .. } => {
                        if let Measured::Object { ascent, .. } | Measured::Decoration { ascent, .. } =
                            state.items[f.item]
                        {
                            let r = Rectangle::new(
                                Point::new(origin.x + f.x, origin.y + line.baseline - ascent),
                                Size::new(f.width, *height),
                            );
                            core_svg::Renderer::draw_svg(renderer, core_svg::Svg::new(handle.clone()), r, *viewport);
                        }
                    }
                    RunKind::SvgFile { handle, .. } => {
                        if let Measured::Object { ascent, .. } | Measured::Decoration { ascent, .. } =
                            state.items[f.item]
                        {
                            let r = Rectangle::new(
                                Point::new(origin.x + f.x, origin.y + line.baseline - ascent),
                                Size::new(f.width, ascent),
                            );
                            core_svg::Renderer::draw_svg(renderer, core_svg::Svg::new(handle.clone()), r, *viewport);
                        }
                    }
                    RunKind::Newline => {}
                }
            }
        }
        // The caret.
        if a == h
            && let Some(focus) = &state.focus
            && focus.caret_visible()
            && state.preedit.as_ref().is_none_or(|p| p.content.is_empty())
        {
            quad(renderer, self.caret_rect(state, origin), self.caret);
        }
    }

    fn mouse_interaction(
        &self,
        _tree: &Tree,
        _layout: Layout<'_>,
        _cursor: mouse::Cursor,
        _viewport: &Rectangle,
        _renderer: &Renderer,
    ) -> mouse::Interaction {
        mouse::Interaction::None
    }
}

impl LiveText<'_> {
    /// The link under a point of the layout.
    fn link_at(&self, state: &State, p: iced::Vector) -> Option<(String, usize)> {
        let line = state.flow.lines.iter().find(|l| p.y >= l.top && p.y < l.top + l.height)?;
        let f = line.frags.iter().find(|f| p.x >= f.x && p.x < f.x + f.width)?;
        match &self.runs.get(f.item)?.kind {
            RunKind::Text { look, .. } | RunKind::Label { look, .. } => look.link.clone(),
            _ => None,
        }
    }

    /// Does what a key binding says.
    fn apply(&self, binding: Binding<EditorMsg>, state: &mut State, shell: &mut Shell<'_, EditorMsg>) {
        let vertical = |state: &mut State, down: bool| {
            let head = self.sel.1;
            let x = *state.preferred_x.get_or_insert_with(|| state.flow.caret(&state.items, head).x);
            state.flow.vertical(&state.items, head, x, down)
        };
        let line = |state: &State| state.flow.line_of(self.sel.1);
        let live = |m: LiveMsg| EditorMsg::Live(m);
        match binding {
            Binding::Custom(EditorMsg::Bind(b @ (Bind::Up | Bind::Down | Bind::ShiftUp | Bind::ShiftDown))) => {
                let down = matches!(b, Bind::Down | Bind::ShiftDown);
                let select = matches!(b, Bind::ShiftUp | Bind::ShiftDown);
                match vertical(state, down) {
                    Some(at) => shell.publish(live(LiveMsg::MoveTo { at, select })),
                    None => {
                        state.preferred_x = None;
                        shell.publish(EditorMsg::Bind(b));
                    }
                }
            }
            Binding::Custom(m) => {
                state.preferred_x = None;
                shell.publish(m);
            }
            Binding::Move(m) | Binding::Select(m) => {
                let select = matches!(binding, Binding::Select(_));
                match m {
                    Motion::Up | Motion::Down => {
                        let down = m == Motion::Down;
                        match vertical(state, down) {
                            Some(at) => shell.publish(live(LiveMsg::MoveTo { at, select })),
                            None => shell.publish(EditorMsg::Bind(match (down, select) {
                                (true, true) => Bind::ShiftDown,
                                (true, false) => Bind::Down,
                                (false, true) => Bind::ShiftUp,
                                (false, false) => Bind::Up,
                            })),
                        }
                    }
                    Motion::Home => {
                        state.preferred_x = None;
                        shell.publish(live(LiveMsg::MoveTo { at: state.flow.line_start(line(state)), select }));
                    }
                    Motion::End => {
                        state.preferred_x = None;
                        let at = state.flow.line_end(&state.items, line(state));
                        shell.publish(live(LiveMsg::MoveTo { at, select }));
                    }
                    Motion::PageUp => shell.publish(live(LiveMsg::MoveTo { at: 0, select })),
                    Motion::PageDown => shell.publish(live(LiveMsg::MoveTo { at: self.len, select })),
                    _ => {
                        state.preferred_x = None;
                        shell.publish(live(LiveMsg::Motion { motion: m, select }));
                    }
                }
            }
            Binding::Insert(c) => {
                state.preferred_x = None;
                shell.publish(live(LiveMsg::Insert(c.to_string())));
            }
            Binding::Enter => shell.publish(EditorMsg::Bind(Bind::Enter)),
            Binding::Backspace => shell.publish(EditorMsg::Bind(Bind::Backspace)),
            Binding::Delete => shell.publish(EditorMsg::Bind(Bind::Delete)),
            Binding::SelectAll => shell.publish(EditorMsg::Command(EditorCommand::SelectAll)),
            Binding::SelectWord => shell.publish(live(LiveMsg::SelectWord(self.sel.1))),
            Binding::SelectLine => shell.publish(live(LiveMsg::SelectAll)),
            Binding::Copy => shell.publish(EditorMsg::Command(EditorCommand::Copy)),
            Binding::Cut => shell.publish(EditorMsg::Command(EditorCommand::Cut)),
            Binding::Paste => shell.publish(EditorMsg::Command(EditorCommand::Paste)),
            Binding::Sequence(bindings) => {
                for b in bindings {
                    self.apply(b, state, shell);
                }
            }
            Binding::Unfocus => state.focus = None,
        }
    }
}

impl<'a> From<LiveText<'a>> for Element<'a, EditorMsg> {
    fn from(w: LiveText<'a>) -> Self {
        Element::new(w)
    }
}
