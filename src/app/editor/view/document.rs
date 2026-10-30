//! The page: its blocks in a column, with the active region in place of the
//! blocks it covers, or the whole source in source mode.

use super::Ctx;
use super::blocks::quote_view;
use super::theme::{
    BLOCK_GAP, LINE_HEIGHT, LIST_INDENT, OUTLINE_WIDTH, PAGE_PADDING, PAGE_WIDTH, Palette, QUOTE_BAR, QUOTE_PADDING,
    editor_style,
};
use crate::app::editor::assets::{DropTarget, content_end};
use crate::app::editor::column::{self, Gap, Shown};
use crate::app::editor::highlight::{self, MarkdownHighlighter};
use crate::app::editor::keys::{self, BindContext};
use crate::app::editor::{AssetsMsg, Editor, EditorMsg, EditorPrefs};
use crate::app::ime::ime_aligned;
use crate::app::view::APP_FONT;
use crate::markdown::parse::ContainerKind;
use iced::advanced::text::Wrapping;
use iced::widget::{Column, column, container, mouse_area, row, scrollable, space, text_editor};
use iced::{Color, Element, Fill};

/// The height of the gap above an entry of the column.
fn gap_height(gap: Gap, zoom: f32) -> f32 {
    let unit = BLOCK_GAP * zoom;
    match gap {
        Gap::None => 0.0,
        Gap::Tight => 2.0 * zoom,
        Gap::Heading => unit * 1.6,
        Gap::Normal => unit,
    }
}

/// `content` inside the containers `kinds` (outermost first): a quote's bar,
/// a list item's indentation.
fn in_containers<'a>(
    content: Element<'a, EditorMsg>,
    kinds: &[ContainerKind],
    p: Palette,
    zoom: f32,
) -> Element<'a, EditorMsg> {
    kinds.iter().rev().fold(content, |inner, kind| match kind {
        ContainerKind::Quote => quote_view(inner, p),
        ContainerKind::Item { depth, .. } => {
            row![space().width(LIST_INDENT * zoom * (*depth + 1) as f32), inner].into()
        }
    })
}

/// The width left for a block inside the containers `kinds`.
fn inner_width(width: f32, kinds: &[ContainerKind], zoom: f32) -> f32 {
    kinds.iter().fold(width, |w, kind| match kind {
        ContainerKind::Quote => w - QUOTE_PADDING - QUOTE_BAR,
        ContainerKind::Item { depth, .. } => w - LIST_INDENT * zoom * (*depth + 1) as f32,
    })
}

impl Editor {
    /// The width of the page's column in the window.
    fn column_width(&self, zoom: f32) -> f32 {
        let outline = if self.outline { OUTLINE_WIDTH } else { 0.0 };
        let assets = if self.assets.is_some() { crate::app::editor::assets::ASSETS_WIDTH } else { 0.0 };
        (self.window_size.width - outline - assets - 2.0 * PAGE_PADDING[1]).clamp(120.0, PAGE_WIDTH * zoom)
    }

    pub(super) fn document<'a>(&'a self, ctx: &Ctx<'a>, prefs: &EditorPrefs) -> Element<'a, EditorMsg> {
        let width = self.column_width(ctx.zoom);
        let dragging = self.asset_dragging();
        let p = ctx.p;
        let content: Element<EditorMsg> = match &self.source {
            Some(source) => self.source_view(source, ctx, prefs),
            None => {
                let mut col = Column::new().width(width);
                let page = &self.page;
                let active = page.active.as_ref();
                let selected = page.blocks.map(|(a, b)| a.min(b)..=a.max(b));
                let mut previous_end = None;
                for entry in column::entries(page) {
                    let space_above = gap_height(entry.gap, ctx.zoom);
                    if space_above > 0.0 {
                        // A click in the gap goes to the nearer block: the upper
                        // half to the end of the one above, the lower half to the
                        // start of the one below.
                        let after = previous_end.map_or(0, |end| content_end(&page.text, 0..end));
                        let half = |at: usize| -> Element<EditorMsg> {
                            let mut area = mouse_area(space().width(Fill).height(space_above / 2.0))
                                .on_press(EditorMsg::PressedAt(at));
                            if dragging {
                                area = area.on_move(move |_| EditorMsg::Assets(AssetsMsg::GapHovered(after)));
                            }
                            area.into()
                        };
                        let gap_view: Element<EditorMsg> = match previous_end {
                            Some(end) => column![half(end), half(entry.covers.start)].into(),
                            None => half(entry.covers.start),
                        };
                        // Inside the containers both sides share: a quote's bar goes on.
                        col = col.push(in_containers(gap_view, &entry.shared, p, ctx.zoom));
                    }
                    let kinds = entry.kinds();
                    let inner = inner_width(width, &kinds, ctx.zoom);
                    let shown: Element<EditorMsg> = match (entry.shown, active) {
                        (Shown::Active, Some(a)) => {
                            in_containers(self.active_view(a, ctx, prefs, inner), &kinds, p, ctx.zoom)
                        }
                        (Shown::Block(i), _) => {
                            let rendered = self.block_view(i, ctx, inner);
                            let is_selected = selected.as_ref().is_some_and(|s| s.contains(&i));
                            let rendered: Element<EditorMsg> = if is_selected {
                                container(rendered)
                                    .width(Fill)
                                    .style(move |_| container::background(p.selection))
                                    .into()
                            } else {
                                rendered
                            };
                            // As wide as the page, so that a click beside the block's
                            // content (an empty item, a line of math) reaches it too.
                            let boxed = container(in_containers(rendered, &kinds, p, ctx.zoom))
                                .width(Fill)
                                .id(self.id(&format!("block-{i}")));
                            let mut area = mouse_area(boxed)
                                .on_press(EditorMsg::BlockPressed(i))
                                .on_enter(EditorMsg::BlockEntered(i));
                            if dragging {
                                area = area.on_move(move |_| EditorMsg::Assets(AssetsMsg::BlockHovered(i)));
                            }
                            area.into()
                        }
                        (Shown::Active, None) => continue,
                    };
                    col = col.push(shown);
                    // Where a dragged item would go as a paragraph of its own.
                    if let Some(DropTarget::Paragraph { at }) = ctx.editor.drop_target()
                        && content_end(&page.text, entry.covers.clone()) == *at
                    {
                        col = col.push(
                            container(space().height(2)).width(Fill).style(move |_| container::background(p.link)),
                        );
                    }
                    previous_end = Some(entry.covers.end);
                }
                // Room to click below the last block.
                let last = previous_end.map_or(0, |end| content_end(&page.text, 0..end));
                let mut room = mouse_area(container(space()).width(Fill).height(120.0 * ctx.zoom))
                    .on_press(EditorMsg::PressedAt(page.text.len()));
                if dragging {
                    room = room.on_move(move |_| EditorMsg::Assets(AssetsMsg::GapHovered(last)));
                }
                col = col.push(room);
                col.into()
            }
        };
        let padded = container(content).padding(PAGE_PADDING).center_x(Fill);
        let page = scrollable(padded).id(self.id("doc")).on_scroll(EditorMsg::Scrolled).width(Fill).height(Fill);
        // While an item of the assets pane is dragged, the page tells where it is.
        let mut area = mouse_area(page);
        if dragging {
            area = area
                .on_move(|_| EditorMsg::Assets(AssetsMsg::PageHovered))
                .on_exit(EditorMsg::Assets(AssetsMsg::LeftPage));
        }
        area.into()
    }

    fn source_view<'a>(
        &'a self,
        source: &'a text_editor::Content,
        ctx: &Ctx<'a>,
        prefs: &EditorPrefs,
    ) -> Element<'a, EditorMsg> {
        let p = ctx.p;
        let bind = BindContext { auto_pair: prefs.auto_pair, code: false, source_mode: true, table: false };
        let settings = highlight::Settings { dark: prefs.dark, page_start: true, find: self.find_query(), code: None };
        let editor = text_editor(source)
            .id(self.id("active"))
            .on_action(EditorMsg::Action)
            .key_binding(move |k| keys::binding(k, bind))
            .highlight_with::<MarkdownHighlighter>(settings, highlight::format)
            .size(ctx.size(1.0))
            .line_height(LINE_HEIGHT)
            .font(APP_FONT)
            .padding(0)
            .wrapping(Wrapping::WordOrGlyph)
            .style(move |_, _| editor_style(p, Color::TRANSPARENT));
        container(ime_aligned(editor)).id(self.id("active-box")).into()
    }
}
