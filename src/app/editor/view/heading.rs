//! How a heading is decorated (monochrome, drawn from the palette):
//!
//! - H1: a 3px rule below the text, in the text color
//! - H2: a light band behind the text with an 8px chip at its left edge
//! - H3: an 8px chip at the left of the text and a 1px rule below it
//! - H4: a 4px chip at the left of the text, in the muted color
//! - H5, H6: undecorated (H6 in the muted color)
//!
//! The same frame goes around a rendered heading, the heading being edited
//! and a heading inside a quote or list item. The text is made by the caller,
//! which is told the width left for it.

use super::EditorMsg;
use super::theme::{LINE_HEIGHT, Palette};
use iced::widget::{column, container, row, space};
use iced::{Color, Element, Fill, Length, Padding, border};

/// The line height the decoration was designed for. Text is set with
/// [`LINE_HEIGHT`], which is taller; half the difference is taken off the
/// distances from the text so that the decoration keeps its proportions.
const DESIGNED_LINE_HEIGHT: f32 = 1.4;

/// The color of a heading's text.
pub(super) fn text_color(level: u8, p: Palette) -> Color {
    if level >= 6 { p.muted } else { p.heading }
}

/// A rectangle of a single color.
fn block<'a>(width: Length, height: Length, color: Color, radius: border::Radius) -> Element<'a, EditorMsg> {
    container(space())
        .width(width)
        .height(height)
        .style(move |_| container::background(color).border(border::rounded(radius)))
        .into()
}

/// The text of a heading of level `level` (set at `em` pixels) inside its
/// decoration. `text` is given the width left for the text of a heading in
/// `width`.
pub(super) fn framed<'a>(
    level: u8,
    em: f32,
    width: f32,
    zoom: f32,
    p: Palette,
    text: impl FnOnce(f32) -> Element<'a, EditorMsg>,
) -> Element<'a, EditorMsg> {
    let half_leading = (LINE_HEIGHT - DESIGNED_LINE_HEIGHT) / 2.0 * em;
    match level {
        1 => {
            let rule = 3.0 * zoom;
            column![
                text(width),
                space().height((0.3 * em - half_leading).max(0.0)),
                block(Fill, rule.into(), p.heading, 0.0.into())
            ]
            .into()
        }
        2 => {
            let chip = 8.0 * zoom;
            let (vertical, side) = ((0.35 * em - half_leading).max(0.0), 0.6 * em);
            let inner = (width - chip - 2.0 * side).max(1.0);
            let content = container(text(inner)).width(inner + 2.0 * side).padding(Padding {
                top: vertical,
                bottom: vertical,
                left: side,
                right: side,
            });
            let band = row![block(chip.into(), Fill, p.heading, border::left(2.0)), content].height(Length::Shrink);
            container(band)
                .width(Fill)
                .style(move |_| container::background(p.header_bg).border(border::rounded(2.0)))
                .into()
        }
        3 | 4 => {
            let (chip, inset, color) = if level == 3 {
                (8.0 * zoom, 0.15 * em + half_leading, p.heading)
            } else {
                (4.0 * zoom, 0.22 * em + half_leading, p.muted)
            };
            let gap = 0.55 * em;
            // A fixed width: the row's height is that of the text, which the
            // chip takes (a text of fill width would be laid out after it).
            let inner = (width - chip - gap).max(1.0);
            let marked = row![
                container(block(chip.into(), Fill, color, 1.0.into())).height(Fill).padding(Padding {
                    top: inset,
                    bottom: inset,
                    ..Padding::ZERO
                }),
                space().width(gap),
                container(text(inner)).width(inner),
            ]
            .height(Length::Shrink);
            if level == 3 {
                column![
                    marked,
                    space().height((0.3 * em - half_leading).max(0.0)),
                    block(Fill, (1.0 * zoom).max(1.0).into(), p.border, 0.0.into())
                ]
                .into()
            } else {
                marked.into()
            }
        }
        _ => text(width),
    }
}
