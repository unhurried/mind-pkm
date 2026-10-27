//! Where the text being composed with an IME shows. The shell draws it over
//! the window, its bottom on the bottom of the caret rectangle the focused
//! widget reports and its line `LineHeight::default()` tall, with the text in
//! the middle of that line. On a taller line (the page editor's) that puts it
//! below the text around it; `align` gives the IME a caret rectangle whose
//! bottom puts the composed text's middle on the text's, and `ImeAligned`
//! does that for a widget that reports its whole line (iced's text editor).

use iced::advanced::layout::{self, Layout};
use iced::advanced::overlay;
use iced::advanced::renderer;
use iced::advanced::text;
use iced::advanced::widget::{Operation, Tree, Widget};
use iced::advanced::{Clipboard, Shell, input_method::InputMethod};
use iced::widget::text::LineHeight;
use iced::{Element, Event, Length, Pixels, Rectangle, Size, Vector, mouse};

/// The caret rectangle `cursor` with its bottom where the composed text, at
/// `size`, has its middle at `middle` (y).
pub fn align(cursor: Rectangle, middle: f32, size: f32) -> Rectangle {
    let half = LineHeight::default().to_absolute(Pixels(size)).0 / 2.0;
    Rectangle { height: (middle + half - cursor.y).max(1.0), ..cursor }
}

/// Wraps a widget whose text sits in the middle of the caret rectangle it
/// reports.
pub struct ImeAligned<'a, Message, Theme, Renderer> {
    content: Element<'a, Message, Theme, Renderer>,
}

pub fn ime_aligned<'a, Message, Theme, Renderer>(
    content: impl Into<Element<'a, Message, Theme, Renderer>>,
) -> ImeAligned<'a, Message, Theme, Renderer> {
    ImeAligned { content: content.into() }
}

impl<Message, Theme, Renderer> Widget<Message, Theme, Renderer> for ImeAligned<'_, Message, Theme, Renderer>
where
    Renderer: text::Renderer,
{
    fn size(&self) -> Size<Length> {
        self.content.as_widget().size()
    }

    fn size_hint(&self) -> Size<Length> {
        self.content.as_widget().size_hint()
    }

    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.content)]
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(std::slice::from_ref(&self.content));
    }

    fn layout(&mut self, tree: &mut Tree, renderer: &Renderer, limits: &layout::Limits) -> layout::Node {
        self.content.as_widget_mut().layout(&mut tree.children[0], renderer, limits)
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
        self.content.as_widget().draw(&tree.children[0], renderer, theme, style, layout, cursor, viewport)
    }

    fn operate(&mut self, tree: &mut Tree, layout: Layout<'_>, renderer: &Renderer, operation: &mut dyn Operation) {
        self.content.as_widget_mut().operate(&mut tree.children[0], layout, renderer, operation)
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
        // Only a request of the content: another widget's came first.
        let before = shell.input_method().is_enabled();
        self.content.as_widget_mut().update(
            &mut tree.children[0],
            event,
            layout,
            cursor,
            renderer,
            clipboard,
            shell,
            viewport,
        );
        if before {
            return;
        }
        if let InputMethod::Enabled { cursor, preedit: Some(preedit), .. } = shell.input_method_mut() {
            let size = preedit.text_size.unwrap_or_else(|| renderer.default_size()).0;
            *cursor = align(*cursor, cursor.center_y(), size);
        }
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        self.content.as_widget().mouse_interaction(&tree.children[0], layout, cursor, viewport, renderer)
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'b>,
        renderer: &Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, Message, Theme, Renderer>> {
        self.content.as_widget_mut().overlay(&mut tree.children[0], layout, renderer, viewport, translation)
    }
}

impl<'a, Message, Theme, Renderer> From<ImeAligned<'a, Message, Theme, Renderer>>
    for Element<'a, Message, Theme, Renderer>
where
    Message: 'a,
    Theme: 'a,
    Renderer: text::Renderer + 'a,
{
    fn from(widget: ImeAligned<'a, Message, Theme, Renderer>) -> Self {
        Element::new(widget)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use iced::Point;

    /// The middle of the composed text as the shell draws it: its line ends at
    /// the caret rectangle's bottom.
    fn drawn_middle(cursor: Rectangle, size: f32) -> f32 {
        cursor.y + cursor.height - LineHeight::default().to_absolute(Pixels(size)).0 / 2.0
    }

    #[test]
    fn composed_text_sits_in_the_middle_of_a_taller_line() {
        // A 16 px text on a 1.6 line: 25.6 px tall, its middle 12.8 px down.
        let line = Rectangle::new(Point::new(30.0, 100.0), Size::new(1.0, 25.6));
        assert!(drawn_middle(line, 16.0) > line.center_y() + 2.0, "unaligned, it shows below the text");
        let aligned = align(line, line.center_y(), 16.0);
        assert!((drawn_middle(aligned, 16.0) - line.center_y()).abs() < 1e-3);
        assert_eq!((aligned.x, aligned.y, aligned.width), (line.x, line.y, line.width));
    }

    #[test]
    fn a_line_as_tall_as_the_composed_one_stays() {
        let line = Rectangle::new(Point::new(0.0, 10.0), Size::new(1.0, 13.0));
        let aligned = align(line, line.center_y(), 10.0);
        assert!((aligned.height - line.height).abs() < 1e-3);
    }

    /// The baseline of `text` shaped at `size` on a `line_height` line, from
    /// the line's top.
    fn baseline(text: &str, size: f32, line_height: LineHeight) -> f32 {
        use iced::advanced::graphics::text::Paragraph;
        use iced::advanced::text::Paragraph as _;
        let paragraph = Paragraph::with_text(text::Text {
            content: text,
            bounds: Size::INFINITE,
            size: Pixels(size),
            line_height,
            font: iced::Font::DEFAULT,
            align_x: text::Alignment::Default,
            align_y: iced::alignment::Vertical::Top,
            shaping: text::Shaping::Advanced,
            wrapping: text::Wrapping::None,
        });
        let run = paragraph.buffer().layout_runs().next().expect("one line");
        run.line_y - run.line_top
    }

    #[test]
    fn composed_text_shares_the_baseline_of_the_text_around_it() {
        let size = 16.0;
        let line = LineHeight::Relative(1.6);
        let height = line.to_absolute(Pixels(size)).0;
        let ascent = baseline("あいう", size, line);
        let caret = Rectangle::new(Point::new(0.0, 200.0), Size::new(1.0, height));
        let aligned = align(caret, caret.y + ascent + (height - 2.0 * ascent) / 2.0, size);
        // The shell draws the composed text's line ending at the caret's bottom.
        let composed_line = LineHeight::default();
        let top = aligned.y + aligned.height - composed_line.to_absolute(Pixels(size)).0;
        let drawn = top + baseline("いいい", size, composed_line);
        assert!((drawn - (caret.y + ascent)).abs() < 0.5, "{drawn} vs {}", caret.y + ascent);
    }
}
