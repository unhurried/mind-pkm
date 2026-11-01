//! What the map window and the editor windows draw alike: the menu bar with
//! its menus, dialogs, and the zoom steps. Each window passes its own look
//! (`MenuStyle`, `DialogStyle`) and messages.

use super::view::APP_FONT;
use iced::advanced::text::{Paragraph as _, Shaping, Wrapping};
use iced::widget::text::LineHeight;
use iced::widget::{Column, button, center, container, mouse_area, opaque, pin, row, space, stack, text};
use iced::{Alignment, Background, Color, Element, Fill, Point, Shadow, Size, Theme, border};

const MENU_FONT_SIZE: f32 = 12.0;
const MENU_BAR_HEIGHT: f32 = 30.0;

/// A menu: its label in the menu bar, and its items' labels, shortcuts and commands.
pub type Menu<C> = (&'static str, &'static [(&'static str, &'static str, C)]);

/// How a window draws its menu bar and menus.
#[derive(Debug, Clone, Copy)]
pub struct MenuStyle {
    /// The padding on each side of a menu bar label: the gap between two
    /// labels is twice this, the same for every pair.
    pub padding: f32,
    /// The menu bar labels' color; None for the theme's (dimmed when hovered).
    pub label: Option<Color>,
    pub bar: Color,
    pub bar_border: Color,
    /// Behind the label of the open menu, or of the one hovered.
    pub open: Color,
    /// An open menu's width.
    pub width: f32,
    /// Around an item's label (vertical, horizontal).
    pub item_padding: [f32; 2],
    /// The width of the check mark before an item's label.
    pub mark_width: f32,
    pub item_text: Color,
    pub shortcut: Color,
    /// Behind the item hovered.
    pub item_hover: Color,
    pub background: Color,
    pub border: Color,
    pub shadow: Option<Shadow>,
}

/// The width of a menu bar label's button: the text and its padding.
fn menu_button_width(label: &str, padding: f32) -> f32 {
    let paragraph = iced::advanced::graphics::text::Paragraph::with_text(iced::advanced::Text {
        content: label,
        bounds: Size::new(f32::INFINITY, f32::INFINITY),
        size: MENU_FONT_SIZE.into(),
        line_height: LineHeight::default(),
        font: APP_FONT,
        align_x: iced::advanced::text::Alignment::Default,
        align_y: iced::alignment::Vertical::Top,
        shaping: Shaping::Basic,
        wrapping: Wrapping::None,
    });
    paragraph.min_bounds().width.ceil() + 2.0 * padding
}

/// Where a menu's button starts in the menu bar (and its menu opens).
fn menu_x<C>(menus: &[Menu<C>], index: usize, padding: f32) -> f32 {
    menus.iter().take(index).map(|(label, _)| menu_button_width(label, padding)).sum()
}

/// The menu bar: a button per menu, `open` the open one; a press opens or
/// closes a menu with `on_menu`.
pub fn menu_bar<'a, C, M: Clone + 'a>(
    menus: &[Menu<C>],
    open: Option<usize>,
    style: MenuStyle,
    on_menu: impl Fn(Option<usize>) -> M,
) -> Element<'a, M> {
    let mut bar = row![].spacing(0);
    for (i, (label, _)) in menus.iter().enumerate() {
        let is_open = open == Some(i);
        let label = text(*label).size(MENU_FONT_SIZE);
        let label = match style.label {
            Some(color) => label.color(color),
            None => label,
        };
        bar = bar.push(
            button(label)
                .padding([4.0, style.padding])
                .style(move |theme: &Theme, status| {
                    let mut s = button::text(theme, status);
                    if is_open || matches!(status, button::Status::Hovered) {
                        s.background = Some(Background::Color(style.open));
                    }
                    if let Some(color) = style.label {
                        s.text_color = color;
                    }
                    s
                })
                .on_press(on_menu(if is_open { None } else { Some(i) })),
        );
    }
    container(bar)
        .width(Fill)
        .style(move |_| container::background(style.bar).border(border::color(style.bar_border).width(1)))
        .into()
}

/// The open menu `index`, below its label, over the window: its items (a
/// check mark before those `checked` says are on), chosen with `on_command`;
/// a press beside it closes it with `on_menu`.
pub fn menu_dropdown<'a, C: Copy, M: Clone + 'a>(
    menus: &[Menu<C>],
    index: usize,
    style: MenuStyle,
    checked: impl Fn(C) -> bool,
    on_command: impl Fn(C) -> M,
    on_menu: impl Fn(Option<usize>) -> M,
) -> Element<'a, M> {
    let (_, items) = menus[index];
    let mut list = Column::new().spacing(0).width(style.width);
    for &(label, shortcut, cmd) in items {
        list = list.push(menu_item(label, shortcut, checked(cmd), style, on_command(cmd)));
    }
    let dropdown = menu_box(list, style);
    stack![
        mouse_area(container(space()).width(Fill).height(Fill)).on_press(on_menu(None)),
        pin(dropdown).position(Point::new(menu_x(menus, index, style.padding), MENU_BAR_HEIGHT)),
    ]
    .into()
}

/// An item of a menu: a check mark when `checked`, its label and its shortcut.
fn menu_item<'a, M: Clone + 'a>(
    label: &'a str,
    shortcut: &'a str,
    checked: bool,
    style: MenuStyle,
    on_press: M,
) -> Element<'a, M> {
    button(
        row![
            text(if checked { "✓" } else { "" }).size(MENU_FONT_SIZE).width(style.mark_width).color(style.item_text),
            text(label).size(MENU_FONT_SIZE).width(Fill).color(style.item_text),
            text(shortcut).size(11).color(style.shortcut)
        ]
        .align_y(Alignment::Center),
    )
    .width(Fill)
    .padding(style.item_padding)
    .style(move |theme: &Theme, status| {
        let mut s = button::text(theme, status);
        s.text_color = style.item_text;
        if matches!(status, button::Status::Hovered) {
            s.background = Some(Background::Color(style.item_hover));
        }
        s
    })
    .on_press(on_press)
    .into()
}

/// The box around a menu's items.
fn menu_box<'a, M: 'a>(list: Column<'a, M>, style: MenuStyle) -> container::Container<'a, M> {
    container(list).padding(MENU_BOX_PADDING).style(move |_| {
        let s = container::background(style.background).border(border::color(style.border).width(1).rounded(4));
        match style.shadow {
            Some(shadow) => s.shadow(shadow),
            None => s,
        }
    })
}

const MENU_BOX_PADDING: f32 = 4.0;
const SEPARATOR_HEIGHT: f32 = 9.0;

/// An entry of a context menu: an item's label, its shortcut and what it
/// does, or None for a separator.
pub type ContextEntry<'a, C> = Option<(&'a str, &'a str, C)>;

/// The label and shortcut of the menu bar's item running `cmd`, as an entry
/// of a context menu.
pub fn entry_of<C: Copy + PartialEq>(menus: &[Menu<C>], cmd: C) -> Option<(&'static str, &'static str)> {
    menus
        .iter()
        .flat_map(|(_, items)| items.iter())
        .find(|item| item.2 == cmd)
        .map(|&(label, shortcut, _)| (label, shortcut))
}

/// The entries of a context menu listing commands of the menu bar (None
/// for a separator), with their labels and shortcuts there.
pub fn context_entries<C: Copy + PartialEq>(
    menus: &[Menu<C>],
    commands: &[Option<C>],
) -> Vec<ContextEntry<'static, C>> {
    commands
        .iter()
        .map(|cmd| {
            cmd.map(|c| {
                let (label, shortcut) = entry_of(menus, c).unwrap_or(("?", ""));
                (label, shortcut, c)
            })
        })
        .collect()
}

/// The height of a context menu with `entries`.
fn context_menu_height<C>(entries: &[ContextEntry<'_, C>], style: MenuStyle) -> f32 {
    let item = (MENU_FONT_SIZE * 1.3).ceil() + 2.0 * style.item_padding[0];
    let rows: f32 = entries.iter().map(|e| if e.is_some() { item } else { SEPARATOR_HEIGHT }).sum();
    rows + 2.0 * MENU_BOX_PADDING + 2.0
}

/// Where a menu of `size` opening at `at` goes so that it fits in `window`:
/// at the point, else (reaching past the right / bottom edge) ending at it,
/// else as far from the window's top left corner as fits.
pub fn context_menu_position(at: Point, size: Size, window: Size) -> Point {
    let place = |at: f32, length: f32, room: f32| {
        if at + length <= room {
            at
        } else if at >= length {
            at - length
        } else {
            (room - length).max(0.0)
        }
    };
    Point::new(place(at.x, size.width, window.width), place(at.y, size.height, window.height))
}

/// A context menu opened at `at` in a window of `window` size, over the
/// window: its entries, chosen with `on_item`; a press beside it closes it
/// with `on_close`.
pub fn context_menu<'a, C: Copy, M: Clone + 'a>(
    entries: &[ContextEntry<'a, C>],
    at: Point,
    window: Size,
    style: MenuStyle,
    on_item: impl Fn(C) -> M,
    on_close: M,
) -> Element<'a, M> {
    let mut list = Column::new().spacing(0).width(style.width);
    for entry in entries {
        list = list.push(match *entry {
            Some((label, shortcut, item)) => menu_item(label, shortcut, false, style, on_item(item)),
            None => {
                container(container(space()).width(Fill).height(1).style(move |_| container::background(style.border)))
                    .padding([(SEPARATOR_HEIGHT - 1.0) / 2.0, style.item_padding[1]])
                    .into()
            }
        });
    }
    let size = Size::new(style.width + 2.0 * MENU_BOX_PADDING + 2.0, context_menu_height(entries, style));
    stack![
        mouse_area(container(space()).width(Fill).height(Fill)).on_press(on_close.clone()).on_right_press(on_close),
        pin(menu_box(list, style)).position(context_menu_position(at, size, window)),
    ]
    .into()
}

/// How a window draws its dialogs.
#[derive(Debug, Clone, Copy)]
pub struct DialogStyle {
    pub background: Color,
    pub border: Color,
    pub shadow: Option<Shadow>,
    /// The color of the dialog's text; None for the window's.
    pub text: Option<Color>,
    /// The width of every button.
    pub button_width: f32,
}

/// A dialog's button, `primary` for the one Enter chooses.
pub fn dialog_button<'a, M: Clone + 'a>(
    label: &'a str,
    primary: bool,
    style: DialogStyle,
    on_press: M,
) -> Element<'a, M> {
    button(text(label).size(13).center())
        .width(style.button_width)
        .style(if primary { button::primary } else { button::secondary })
        .on_press(on_press)
        .into()
}

/// A dialog in the middle of the window: `body` above the `buttons`, which
/// line up on the right, over a shade that keeps the pointer from what is
/// behind it.
pub fn dialog<'a, M: 'a>(
    body: Vec<Element<'a, M>>,
    buttons: Vec<Element<'a, M>>,
    style: DialogStyle,
) -> Element<'a, M> {
    let buttons = row![space().width(Fill)].extend(buttons).spacing(8);
    let card =
        container(Column::with_children(body).push(buttons).spacing(16)).padding(20).max_width(560).style(move |_| {
            let s = container::background(style.background).border(border::color(style.border).width(1).rounded(8));
            let s = match style.shadow {
                Some(shadow) => s.shadow(shadow),
                None => s,
            };
            match style.text {
                Some(color) => s.color(color),
                None => s,
            }
        });
    opaque(center(card).style(|_| container::background(Color::from_rgba(0.0, 0.0, 0.0, 0.25))))
}

/// The zoom level `steps` steps away from `zoom` among `levels` (ascending),
/// within their limits; a zoom beyond them counts as the last level.
pub fn zoom_step(levels: &[f32], zoom: f32, steps: i32) -> f32 {
    let last = levels.len() - 1;
    let i = levels.iter().position(|&z| z >= zoom - 0.001).unwrap_or(last);
    levels[(i as i64 + i64::from(steps)).clamp(0, last as i64) as usize]
}

#[cfg(test)]
mod tests {
    use super::*;

    const LEVELS: &[f32] = &[0.5, 0.75, 1.0, 1.5, 2.0];

    #[test]
    fn zoom_steps_stop_at_the_limits() {
        assert_eq!(zoom_step(LEVELS, 1.0, 1), 1.5);
        assert_eq!(zoom_step(LEVELS, 1.0, -1), 0.75);
        assert_eq!(zoom_step(LEVELS, 0.5, -1), 0.5);
        assert_eq!(zoom_step(LEVELS, 2.0, 1), 2.0);
        assert_eq!(zoom_step(LEVELS, 1.0, -20), 0.5);
        // Between levels: the next one up; outside them: the nearest end.
        assert_eq!(zoom_step(LEVELS, 1.2, 0), 1.5);
        assert_eq!(zoom_step(LEVELS, 0.1, 1), 0.75);
        assert_eq!(zoom_step(LEVELS, 3.0, -1), 1.5);
    }

    #[test]
    fn context_menus_stay_inside_the_window() {
        let window = Size::new(800.0, 600.0);
        let menu = Size::new(200.0, 300.0);
        // Room on the right and below: at the pointer.
        assert_eq!(context_menu_position(Point::new(100.0, 100.0), menu, window), Point::new(100.0, 100.0));
        // Near the bottom right corner: to the left of and above the pointer.
        assert_eq!(context_menu_position(Point::new(700.0, 500.0), menu, window), Point::new(500.0, 200.0));
        // No room above or below: as low as it fits.
        assert_eq!(
            context_menu_position(Point::new(150.0, 250.0), Size::new(200.0, 500.0), window),
            Point::new(150.0, 100.0)
        );
        // Taller than the window: against its top.
        assert_eq!(
            context_menu_position(Point::new(150.0, 200.0), Size::new(200.0, 700.0), window),
            Point::new(150.0, 0.0)
        );
    }

    #[test]
    fn context_entries_take_the_menu_bars_labels_and_shortcuts() {
        const MENUS: &[Menu<u8>] = &[("File", &[("Save", "Ctrl+S", 1)]), ("Edit", &[("Copy", "Ctrl+C", 2)])];
        assert_eq!(
            context_entries(MENUS, &[Some(2), None, Some(1)]),
            vec![Some(("Copy", "Ctrl+C", 2)), None, Some(("Save", "Ctrl+S", 1))]
        );
    }
}
