//! Dragging in a text editor that took the place of the block pressed. A press
//! on a rendered block makes that block the active one, so the editor shown
//! instead never saw the press and a drag with the button held does nothing.
//! `PressRelay` tells where a press landed (`tracking`) and hands it, with the
//! moves that follow, to the editor as a click and drags (`relaying`).
//! `tracking` also takes a press in the margin beside the content for one at
//! the content's left edge at that height, so that it puts the cursor at the
//! start of the line there.

use iced::advanced::layout::{self, Layout};
use iced::advanced::overlay;
use iced::advanced::renderer;
use iced::advanced::widget::{Operation, Tree, tree};
use iced::advanced::{Clipboard, Shell, Widget};
use iced::widget::text_editor::Action;
use iced::{Element, Event, Length, Padding, Point, Rectangle, Renderer, Size, Theme, Vector, mouse};

use crate::app::editor::EditorMsg;

/// A press that began before the wrapped editor was shown, still held.
#[derive(Clone, Copy)]
pub struct Held {
    /// Where the button went down (in the page, like the widgets' bounds).
    pub from: Point,
    /// The padding of the wrapped text editor.
    pub padding: Padding,
}

pub struct PressRelay<'a> {
    content: Element<'a, EditorMsg>,
    /// Told where the left button goes down over the content.
    on_press: Option<fn(Point) -> EditorMsg>,
    held: Option<Held>,
}

#[derive(Default)]
struct State {
    /// The press was in the content itself: it handles its drag.
    own: bool,
    /// The press was handed to the editor.
    relayed: bool,
    /// The button was released since the page last said it was held.
    released: bool,
}

impl<'a> PressRelay<'a> {
    /// Publishes where the left button goes down over `content`.
    pub fn tracking(content: impl Into<Element<'a, EditorMsg>>, on_press: fn(Point) -> EditorMsg) -> Self {
        PressRelay { content: content.into(), on_press: Some(on_press), held: None }
    }

    /// Hands a text editor a press held from before it was shown (None: no
    /// button is held).
    pub fn relaying(content: impl Into<Element<'a, EditorMsg>>, held: Option<Held>) -> Self {
        PressRelay { content: content.into(), on_press: None, held }
    }
}

/// Where a press at the cursor stands for when it is beside `bounds` (to the
/// left or right of them) at a height inside them and inside the visible
/// `viewport`: the left edge of the bounds at that height.
fn margin_press(cursor: mouse::Cursor, bounds: Rectangle, viewport: &Rectangle) -> Option<Point> {
    let p = cursor.position()?;
    let beside = p.x < bounds.x || p.x >= bounds.x + bounds.width;
    let at_height = p.y >= bounds.y && p.y < bounds.y + bounds.height;
    (beside && at_height && viewport.contains(p)).then(|| Point::new(bounds.x + 1.0, p.y))
}

/// `p` in the editor's text, kept inside its bounds.
fn local(p: Point, bounds: Rectangle, padding: Padding) -> Point {
    Point::new(
        p.x.clamp(bounds.x, bounds.x + bounds.width) - bounds.x - padding.left,
        p.y.clamp(bounds.y, bounds.y + bounds.height) - bounds.y - padding.top,
    )
}

impl Widget<EditorMsg, Theme, Renderer> for PressRelay<'_> {
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(State::default())
    }

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
        shell: &mut Shell<'_, EditorMsg>,
        viewport: &Rectangle,
    ) {
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
        let bounds = layout.bounds();
        if let (Some(on_press), Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left))) = (self.on_press, event)
            && let Some(p) = cursor.position_over(bounds)
        {
            shell.publish(on_press(p));
        }
        // A press in the margin, replayed at the content's left edge.
        if let (Some(on_press), Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left))) = (self.on_press, event)
            && let Some(at) = margin_press(cursor, bounds, viewport)
        {
            let over = mouse::Cursor::Available(at);
            let moved = Event::Mouse(mouse::Event::CursorMoved { position: at });
            for replay in [&moved, event] {
                self.content.as_widget_mut().update(
                    &mut tree.children[0],
                    replay,
                    layout,
                    over,
                    renderer,
                    clipboard,
                    shell,
                    viewport,
                );
            }
            shell.publish(on_press(at));
        }
        let state = tree.state.downcast_mut::<State>();
        let Some(held) = self.held else {
            *state = State::default();
            return;
        };
        match event {
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) if cursor.is_over(bounds) => {
                state.own = true;
            }
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                *state = State { released: true, ..State::default() };
            }
            _ if state.own || state.released => {}
            _ => {
                // The press, as a click where it landed (when that is on the
                // editor: else the cursor stays where the page put it).
                if !state.relayed {
                    state.relayed = true;
                    if bounds.contains(held.from) {
                        shell.publish(EditorMsg::Action(Action::Click(local(held.from, bounds, held.padding))));
                    }
                }
                if let Event::Mouse(mouse::Event::CursorMoved { .. }) = event
                    && let Some(p) = cursor.position_over(bounds)
                {
                    shell.publish(EditorMsg::Action(Action::Drag(local(p, bounds, held.padding))));
                }
            }
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
    ) -> Option<overlay::Element<'b, EditorMsg, Theme, Renderer>> {
        self.content.as_widget_mut().overlay(&mut tree.children[0], layout, renderer, viewport, translation)
    }
}

impl<'a> From<PressRelay<'a>> for Element<'a, EditorMsg> {
    fn from(widget: PressRelay<'a>) -> Self {
        Element::new(widget)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_press_beside_the_content_stands_for_one_at_its_left_edge() {
        let bounds = Rectangle { x: 100.0, y: 20.0, width: 300.0, height: 500.0 };
        let viewport = Rectangle { x: 0.0, y: 0.0, width: 600.0, height: 400.0 };
        let at = |x: f32, y: f32| margin_press(mouse::Cursor::Available(Point::new(x, y)), bounds, &viewport);
        assert_eq!(at(10.0, 50.0), Some(Point::new(101.0, 50.0)));
        assert_eq!(at(450.0, 50.0), Some(Point::new(101.0, 50.0)));
        // On the content, above or below it, or out of the visible page.
        assert_eq!(at(200.0, 50.0), None);
        assert_eq!(at(10.0, 10.0), None);
        assert_eq!(at(10.0, 530.0), None);
        assert_eq!(at(10.0, 450.0), None);
        assert_eq!(margin_press(mouse::Cursor::Unavailable, bounds, &viewport), None);
    }

    #[test]
    fn a_point_is_taken_in_the_text_inside_the_bounds() {
        let bounds = Rectangle { x: 10.0, y: 20.0, width: 100.0, height: 50.0 };
        let padding = Padding::from(4.0);
        assert_eq!(local(Point::new(30.0, 40.0), bounds, padding), Point::new(16.0, 16.0));
        assert_eq!(local(Point::new(0.0, 500.0), bounds, padding), Point::new(-4.0, 46.0));
    }
}
