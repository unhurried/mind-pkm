//! How an editor window looks: the colors of its themes, its fonts and sizes.

use crate::app::ui::{DialogStyle, MenuStyle};
use crate::app::view::APP_FONT;
use crate::markdown::BlockKind;
use iced::advanced::graphics::text::font_system;
use iced::widget::text_editor;
use iced::{Background, Border, Color, Font, Theme, font};
use std::sync::OnceLock;

/// The family code is set in where it is installed: a monospace font with
/// Japanese whose full-width characters are exactly two half-width ones wide.
const CODE_FAMILY: &str = "BIZ UDGothic";

/// The code font where `CODE_FAMILY` is not installed.
const FALLBACK_CODE_FONT: Font = if cfg!(windows) { Font::with_name("Consolas") } else { Font::MONOSPACE };

/// The font code is set in: `CODE_FAMILY` if the font system has it, else
/// `FALLBACK_CODE_FONT` - a font named but missing would leave every
/// character to the per-script fallback. Chosen once, the first time it is
/// asked (`app::run` asks before any text is drawn).
pub fn code_font() -> Font {
    static CHOSEN: OnceLock<Font> = OnceLock::new();
    if let Some(&font) = CHOSEN.get() {
        return font;
    }
    // iced holds the font system's lock while it formats highlighted text,
    // which asks for the code font: never wait for the lock here.
    let Ok(mut system) = font_system().try_write() else { return FALLBACK_CODE_FONT };
    let font =
        choose_code_font(system.raw().db().faces().flat_map(|f| f.families.iter().map(|(name, _)| name.as_str())));
    *CHOSEN.get_or_init(|| font)
}

/// The code font, given the family names of the installed fonts.
fn choose_code_font<'a>(mut families: impl Iterator<Item = &'a str>) -> Font {
    if families.any(|name| name == CODE_FAMILY) { Font::with_name(CODE_FAMILY) } else { FALLBACK_CODE_FONT }
}

/// Body text size and line height at 100%.
pub const BODY_SIZE: f32 = 15.0;

pub(super) const LINE_HEIGHT: f32 = 1.6;

/// The width of the page's column at 100%.
pub(super) const PAGE_WIDTH: f32 = 1024.0;

pub(super) const PAGE_PADDING: [f32; 2] = [24.0, 36.0];

pub(super) const HEADING_SCALE: [f32; 6] = [2.0, 1.5, 1.25, 1.125, 1.0, 1.0];

pub(super) const LIST_INDENT: f32 = 26.0;

pub(super) const BLOCK_GAP: f32 = 10.0;

pub(super) const QUOTE_BAR: f32 = 4.0;

pub(super) const QUOTE_PADDING: f32 = 14.0;

pub(super) const CODE_PADDING: f32 = 10.0;

pub(super) const OUTLINE_WIDTH: f32 = 220.0;

/// The most text a table's column takes on a line at 100%; longer text wraps.
pub(super) const TABLE_COLUMN_MAX: f32 = 320.0;

/// A table cell's padding: above and below, left and right.
pub(super) const CELL_PADDING: [f32; 2] = [4.0, 8.0];

/// The editor's colors in a theme.
#[derive(Debug, Clone, Copy)]
pub struct Palette {
    pub dark: bool,
    pub bg: Color,
    pub text: Color,
    pub muted: Color,
    pub heading: Color,
    pub link: Color,
    pub code: Color,
    pub code_bg: Color,
    pub html: Color,
    pub math: Color,
    pub mark: Color,
    pub mark_bg: Color,
    pub find: Color,
    pub find_bg: Color,
    pub quote_bar: Color,
    pub selection: Color,
    pub border: Color,
    pub bar_bg: Color,
    pub hover: Color,
    pub header_bg: Color,
    pub error: Color,
}

const LIGHT: Palette = Palette {
    dark: false,
    bg: Color::WHITE,
    text: Color::from_rgb(0.12, 0.13, 0.15),
    muted: Color::from_rgb(0.55, 0.57, 0.60),
    heading: Color::from_rgb(0.10, 0.11, 0.13),
    link: Color::from_rgb(0.15, 0.38, 0.78),
    code: Color::from_rgb(0.70, 0.22, 0.30),
    code_bg: Color::from_rgb(0.955, 0.96, 0.965),
    html: Color::from_rgb(0.50, 0.30, 0.65),
    math: Color::from_rgb(0.10, 0.45, 0.40),
    mark: Color::from_rgb(0.62, 0.45, 0.0),
    mark_bg: Color::from_rgb(1.0, 0.93, 0.55),
    find: Color::from_rgb(0.85, 0.35, 0.0),
    find_bg: Color::from_rgb(1.0, 0.80, 0.45),
    quote_bar: Color::from_rgb(0.82, 0.84, 0.87),
    selection: Color::from_rgba(0.25, 0.50, 0.95, 0.25),
    border: Color::from_rgb(0.85, 0.86, 0.88),
    bar_bg: Color::from_rgb(0.95, 0.96, 0.97),
    hover: Color::from_rgb(0.86, 0.90, 0.97),
    header_bg: Color::from_rgb(0.94, 0.95, 0.96),
    error: Color::from_rgb(0.80, 0.20, 0.20),
};

const DARK: Palette = Palette {
    dark: true,
    bg: Color::from_rgb(0.12, 0.13, 0.15),
    text: Color::from_rgb(0.87, 0.89, 0.91),
    muted: Color::from_rgb(0.52, 0.55, 0.60),
    heading: Color::from_rgb(0.94, 0.95, 0.96),
    link: Color::from_rgb(0.45, 0.66, 0.98),
    code: Color::from_rgb(0.93, 0.55, 0.60),
    code_bg: Color::from_rgb(0.17, 0.18, 0.21),
    html: Color::from_rgb(0.75, 0.58, 0.90),
    math: Color::from_rgb(0.45, 0.80, 0.72),
    mark: Color::from_rgb(0.95, 0.80, 0.35),
    mark_bg: Color::from_rgb(0.45, 0.38, 0.10),
    find: Color::from_rgb(1.0, 0.62, 0.30),
    find_bg: Color::from_rgb(0.55, 0.35, 0.08),
    quote_bar: Color::from_rgb(0.30, 0.32, 0.36),
    selection: Color::from_rgba(0.35, 0.55, 0.95, 0.35),
    border: Color::from_rgb(0.26, 0.28, 0.31),
    bar_bg: Color::from_rgb(0.15, 0.16, 0.18),
    hover: Color::from_rgb(0.22, 0.26, 0.34),
    header_bg: Color::from_rgb(0.17, 0.18, 0.20),
    error: Color::from_rgb(0.95, 0.45, 0.45),
};

impl Palette {
    pub fn new(dark: bool) -> Palette {
        if dark { DARK } else { LIGHT }
    }

    pub fn of(theme: &Theme) -> Palette {
        Palette::new(theme.extended_palette().is_dark)
    }

    /// The menu bar and menus in these colors. No shadow under a menu: the
    /// software renderer draws it over the box on partial redraws.
    pub(super) fn menu(self) -> MenuStyle {
        MenuStyle {
            padding: 12.0,
            label: Some(self.text),
            bar: self.bar_bg,
            bar_border: self.border,
            open: self.hover,
            width: 320.0,
            item_padding: [5.0, 10.0],
            mark_width: 16.0,
            item_text: self.text,
            shortcut: self.muted,
            item_hover: self.hover,
            background: self.bg,
            border: self.muted,
            shadow: None,
        }
    }

    /// Right-click menus in these colors: narrower than the menu bar's.
    pub(super) fn context_menu(self) -> MenuStyle {
        MenuStyle { width: 300.0, ..self.menu() }
    }

    /// Dialogs in these colors.
    pub(super) fn dialog(self) -> DialogStyle {
        DialogStyle { background: self.bg, border: self.muted, shadow: None, text: None, button_width: 110.0 }
    }
}

pub(super) fn bold(f: Font) -> Font {
    Font { weight: font::Weight::Bold, ..f }
}

pub(super) fn italic(f: Font) -> Font {
    Font { style: font::Style::Italic, ..f }
}

/// The size, font and padding of the active region's source for a kind of block.
pub fn region_text_style(kind: &BlockKind, zoom: f32) -> (f32, Font, f32) {
    match kind {
        BlockKind::Heading(level) => {
            (BODY_SIZE * zoom * HEADING_SCALE[usize::from((*level).clamp(1, 6)) - 1], bold(APP_FONT), 0.0)
        }
        k if k.is_code() => (BODY_SIZE * zoom, APP_FONT, CODE_PADDING),
        _ => (BODY_SIZE * zoom, APP_FONT, 0.0),
    }
}

pub const TEXT_LINE_HEIGHT: f32 = LINE_HEIGHT;

/// The size, font and padding of code in its box (as rendered).
pub fn code_text_style(zoom: f32) -> (f32, Font, f32) {
    (BODY_SIZE * zoom * 0.9, code_font(), CODE_PADDING)
}

pub(super) fn editor_style(p: Palette, background: Color) -> text_editor::Style {
    text_editor::Style {
        background: Background::Color(background),
        border: Border { radius: 4.0.into(), ..Border::default() },
        placeholder: p.muted,
        value: p.text,
        selection: p.selection,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn code_font_is_biz_udgothic_where_it_is_installed() {
        let installed = ["Meiryo UI", "BIZ UDPGothic", "BIZ UDGothic", "Consolas"];
        assert_eq!(choose_code_font(installed.into_iter()), Font::with_name("BIZ UDGothic"));
    }

    #[test]
    fn code_font_falls_back_where_biz_udgothic_is_missing() {
        // The proportional family of the same font file does not count.
        let installed = ["Meiryo UI", "BIZ UDPGothic", "Consolas"];
        assert_eq!(choose_code_font(installed.into_iter()), FALLBACK_CODE_FONT);
        assert_eq!(choose_code_font(std::iter::empty()), FALLBACK_CODE_FONT);
    }
}
