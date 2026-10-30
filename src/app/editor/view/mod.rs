//! Rendering an editor window: the menu bar, the page as rendered blocks with
//! the active region as a text editor, the table being edited, the outline,
//! the status bar and dialogs.
//!
//! `document` lays out the page, `blocks` and `inline` render its blocks and
//! their text, `active` the region being edited, `chrome` the bars, menus,
//! outline and dialogs, and `theme` holds the colors, fonts and sizes.
//!
//! Text is laid out in pieces of one line (no line breaks inside), each in a
//! widget that reports where the pointer is in it, so that a click can be
//! mapped back to the source (`hit_test`).

mod active;
mod blocks;
mod chrome;
mod document;
mod hit;
mod inline;
mod theme;

#[cfg(test)]
pub use hit::HitRun;
pub use hit::{HitInfo, caret_in, hit_test, piece_key};
pub use theme::{BODY_SIZE, CODE_FONT, Palette, TEXT_LINE_HEIGHT, code_text_style, region_text_style};

use super::render::RenderCache;
use super::{Editor, EditorMsg, EditorPrefs};
use crate::app::arrow_cursor::arrow_cursor;
use chrome::dialog_view;
use iced::widget::{Stack, column, container, mouse_area, row, space, stack};
use iced::{Color, Element, Fill, Font};

/// The link of a span: its destination and the source offset of its text.
#[derive(Debug, Clone, PartialEq)]
struct SpanLink {
    dest: String,
    at: usize,
}

/// What rendering needs besides the editor.
#[derive(Clone, Copy)]
struct Ctx<'a> {
    editor: &'a Editor,
    p: Palette,
    zoom: f32,
    renders: &'a RenderCache,
    /// The source offsets the find bar highlights.
    matches: &'a [std::ops::Range<usize>],
    /// The match that is selected (start, end).
    current: Option<(usize, usize)>,
}

impl Ctx<'_> {
    fn size(&self, scale: f32) -> f32 {
        BODY_SIZE * self.zoom * scale
    }
}

/// How a piece of text is set: its base size, font and color.
#[derive(Debug, Clone, Copy)]
struct TextBase {
    size: f32,
    font: Font,
    color: Color,
}

impl Editor {
    pub fn view<'a>(&'a self, prefs: &'a EditorPrefs, renders: &'a RenderCache) -> Element<'a, EditorMsg> {
        let p = Palette::new(prefs.dark);
        let matches = self.find.as_ref().filter(|b| !b.query.is_empty()).map_or(&[][..], |b| &b.matches[..]);
        let current = self.current_match().map(|m| (m.start, m.end));
        let ctx = Ctx { editor: self, p, zoom: prefs.zoom, renders, matches, current };
        let document = self.document(&ctx, prefs);
        // The outline and the find bar keep their places when hidden, so that
        // the page's widgets (its scroll position) stay where they are.
        let outline: Element<EditorMsg> = if self.outline { self.outline_view(&ctx) } else { space().into() };
        let assets: Element<EditorMsg> = match &self.assets {
            Some(pane) => self.assets_view(pane, p),
            None => space().into(),
        };
        let find: Element<EditorMsg> = match &self.find {
            Some(bar) => self.find_bar(bar, p),
            None => space().into(),
        };
        let body = row![outline, document, assets];
        let main = column![self.menu_bar(p), find, container(body).height(Fill), self.status_bar(p, prefs)];
        let main = container(main).style(move |_| container::background(p.bg).color(p.text));
        let mut layers: Stack<EditorMsg> = stack![main].width(Fill).height(Fill);
        if let Some(i) = self.menu {
            layers = layers.push(self.menu_dropdown(i, p, prefs));
        }
        if let Some(at) = self.table_ui.menu {
            layers = layers.push(self.cell_menu_view(at, p));
        }
        if let Some(pane) = &self.assets {
            if let Some(menu) = &pane.menu {
                layers = layers.push(self.asset_menu_view(menu, p));
            }
            if let Some(drag) = pane.drag.as_ref().filter(|d| d.dragging) {
                layers = layers.push(self.drag_label(drag, p));
            }
        }
        if let Some(dialog) = self.dialogs.front() {
            layers = layers.push(dialog_view(dialog, p, self.id("rename")));
        }
        // While a table is edited or the assets pane is shown, where the
        // pointer is (a menu opens there, a dragged item follows it).
        let mut root = mouse_area(layers);
        if self.page.active.as_ref().is_some_and(|a| a.table.is_some()) || self.assets.is_some() {
            root = root.on_move(EditorMsg::PointerAt);
        }
        arrow_cursor(root).into()
    }
}
