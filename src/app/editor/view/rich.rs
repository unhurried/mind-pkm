//! Rich text - spans of text in their own sizes, fonts, colors and
//! backgrounds, some of them links - as iced's `rich_text` shows it, except
//! for where the underlines and strikethroughs go: where the widget editing
//! the active block draws them (`flow::decoration_tops`), so that they stay in
//! place when a block is edited. Adapted from `iced_widget::text::rich`
//! (iced 0.14, MIT).

use super::super::flow::{self, DECORATION_THICKNESS};
use iced::advanced::graphics::text::Paragraph;
use iced::advanced::layout::{self, Layout};
use iced::advanced::renderer::{self, Quad};
use iced::advanced::text::{self as core_text, Paragraph as _, Renderer as _, Span};
use iced::advanced::widget::{Tree, Widget, tree};
use iced::advanced::{Clipboard, Renderer as _, Shell, mouse};
use iced::widget::text::{Alignment, LineHeight, Shaping, Wrapping};
use iced::{Color, Element, Event, Font, Length, Pixels, Point, Rectangle, Renderer, Size, Theme, Vector};

/// Rich text of the spans.
pub(super) fn rich<'a, Link, Message>(spans: Vec<Span<'static, Link, Font>>) -> Rich<'a, Link, Message>
where
    Link: Clone + 'static,
{
    Rich {
        spans,
        size: None,
        line_height: LineHeight::default(),
        width: Length::Shrink,
        font: None,
        wrapping: Wrapping::default(),
        hovered_link: None,
        on_link_click: None,
    }
}

pub(super) struct Rich<'a, Link, Message> {
    spans: Vec<Span<'static, Link, Font>>,
    size: Option<Pixels>,
    line_height: LineHeight,
    width: Length,
    font: Option<Font>,
    wrapping: Wrapping,
    hovered_link: Option<usize>,
    on_link_click: Option<Box<dyn Fn(Link) -> Message + 'a>>,
}

impl<'a, Link, Message> Rich<'a, Link, Message> {
    pub(super) fn size(mut self, size: impl Into<Pixels>) -> Self {
        self.size = Some(size.into());
        self
    }

    pub(super) fn line_height(mut self, line_height: impl Into<LineHeight>) -> Self {
        self.line_height = line_height.into();
        self
    }

    pub(super) fn font(mut self, font: Font) -> Self {
        self.font = Some(font);
        self
    }

    pub(super) fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = width.into();
        self
    }

    pub(super) fn wrapping(mut self, wrapping: Wrapping) -> Self {
        self.wrapping = wrapping;
        self
    }

    pub(super) fn on_link_click(mut self, f: impl Fn(Link) -> Message + 'a) -> Self {
        self.on_link_click = Some(Box::new(f));
        self
    }
}

/// What the paragraph is shaped from, besides its content.
struct Text {
    bounds: Size,
    size: Pixels,
    font: Font,
    line_height: LineHeight,
    wrapping: Wrapping,
}

impl Text {
    fn with<C>(&self, content: C) -> core_text::Text<C, Font> {
        core_text::Text {
            content,
            bounds: self.bounds,
            size: self.size,
            line_height: self.line_height,
            font: self.font,
            align_x: Alignment::Default,
            align_y: iced::alignment::Vertical::Top,
            shaping: Shaping::Advanced,
            wrapping: self.wrapping,
        }
    }
}

struct State<Link> {
    spans: Vec<Span<'static, Link, Font>>,
    span_pressed: Option<usize>,
    paragraph: Paragraph,
}

impl<Link, Message> Widget<Message, Theme, Renderer> for Rich<'_, Link, Message>
where
    Link: Clone + 'static,
{
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State<Link>>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(State::<Link> { spans: Vec::new(), span_pressed: None, paragraph: Paragraph::default() })
    }

    fn size(&self) -> Size<Length> {
        Size { width: self.width, height: Length::Shrink }
    }

    fn layout(&mut self, tree: &mut Tree, renderer: &Renderer, limits: &layout::Limits) -> layout::Node {
        let state = tree.state.downcast_mut::<State<Link>>();
        layout::sized(limits, self.width, Length::Shrink, |limits| {
            let bounds = limits.max();
            let size = self.size.unwrap_or_else(|| renderer.default_size());
            let font = self.font.unwrap_or_else(|| renderer.default_font());
            let text = Text { bounds, size, font, line_height: self.line_height, wrapping: self.wrapping };
            if state.spans != self.spans {
                state.paragraph = Paragraph::with_spans(text.with(self.spans.as_slice()));
                state.spans.clone_from(&self.spans);
            } else {
                match state.paragraph.compare(text.with(())) {
                    core_text::Difference::None => {}
                    core_text::Difference::Bounds => state.paragraph.resize(bounds),
                    core_text::Difference::Shape => {
                        state.paragraph = Paragraph::with_spans(text.with(self.spans.as_slice()))
                    }
                }
            }
            state.paragraph.min_bounds()
        })
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        _theme: &Theme,
        defaults: &renderer::Style,
        layout: Layout<'_>,
        _cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        if !layout.bounds().intersects(viewport) {
            return;
        }
        let state = tree.state.downcast_ref::<State<Link>>();
        let translation = layout.position() - Point::ORIGIN;
        let quad = |renderer: &mut Renderer, bounds: Rectangle, color: Color| {
            renderer.fill_quad(Quad { bounds, ..Default::default() }, color);
        };
        for (index, span) in self.spans.iter().enumerate() {
            // A link is underlined while the pointer is over it.
            let hovered = self.on_link_click.is_some() && Some(index) == self.hovered_link;
            let underline = span.underline || hovered;
            if span.highlight.is_none() && !underline && !span.strikethrough {
                continue;
            }
            let regions = state.paragraph.span_bounds(index);
            if let Some(highlight) = span.highlight {
                for bounds in &regions {
                    let bounds = Rectangle::new(
                        bounds.position() - Vector::new(span.padding.left, span.padding.top),
                        bounds.size() + Size::new(span.padding.x(), span.padding.y()),
                    );
                    renderer.fill_quad(
                        Quad { bounds: bounds + translation, border: highlight.border, ..Default::default() },
                        highlight.background,
                    );
                }
            }
            if !underline && !span.strikethrough {
                continue;
            }
            let size = span.size.or(self.size).unwrap_or_else(|| renderer.default_size()).0;
            let line_height = span.line_height.unwrap_or(self.line_height).to_absolute(Pixels(size)).0 / size;
            let color = span.color.unwrap_or(defaults.text_color);
            // Each region is a part of the span on one line, from its top.
            let (underline_top, strike_top) = flow::decoration_tops(0.0, size, line_height);
            for bounds in &regions {
                for (drawn, y) in [(underline, underline_top), (span.strikethrough, strike_top)] {
                    if drawn {
                        let at = bounds.position() + translation + Vector::new(0.0, y);
                        // On whole pixels: sharp, however the line falls.
                        let at = Point::new(at.x, at.y.round());
                        quad(renderer, Rectangle::new(at, Size::new(bounds.width, DECORATION_THICKNESS)), color);
                    }
                }
            }
        }
        renderer.fill_paragraph(&state.paragraph, layout.bounds().position(), defaults.text_color, *viewport);
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _renderer: &Renderer,
        _clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        _viewport: &Rectangle,
    ) {
        let Some(on_link_click) = &self.on_link_click else { return };
        let was_hovered = self.hovered_link.is_some();
        let state = tree.state.downcast_mut::<State<Link>>();
        self.hovered_link = cursor.position_in(layout.bounds()).and_then(|position| {
            state.paragraph.hit_span(position).filter(|&span| self.spans.get(span).is_some_and(|s| s.link.is_some()))
        });
        if was_hovered != self.hovered_link.is_some() {
            shell.request_redraw();
        }
        match event {
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                if self.hovered_link.is_some() {
                    state.span_pressed = self.hovered_link;
                    shell.capture_event();
                }
            }
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                if let Some(span) = state.span_pressed.take()
                    && Some(span) == self.hovered_link
                    && let Some(link) = self.spans.get(span).and_then(|s| s.link.clone())
                {
                    shell.publish(on_link_click(link));
                }
            }
            _ => {}
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
        if self.hovered_link.is_some() { mouse::Interaction::Pointer } else { mouse::Interaction::None }
    }
}

impl<'a, Link, Message> From<Rich<'a, Link, Message>> for Element<'a, Message, Theme, Renderer>
where
    Link: Clone + 'static,
    Message: 'a,
{
    fn from(rich: Rich<'a, Link, Message>) -> Self {
        Element::new(rich)
    }
}
