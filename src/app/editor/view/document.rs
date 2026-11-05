//! The page: its blocks in a column, with the active region in place of the
//! blocks it covers, or the whole source in source mode.

use super::Ctx;
use super::blocks::quote_view;
use super::theme::{
    BLOCK_GAP, LINE_HEIGHT, LIST_INDENT, OUTLINE_WIDTH, PAGE_PADDING, PAGE_WIDTH, Palette, QUOTE_BAR, QUOTE_PADDING,
    editor_style,
};
use crate::app::editor::assets::{DropTarget, content_end};
use crate::app::editor::column::{self, Entry, EntryKey, Gap, Shown};
use crate::app::editor::highlight::{self, MarkdownHighlighter};
use crate::app::editor::keys::{self, BindContext};
use crate::app::editor::near;
use crate::app::editor::press_relay::PressRelay;
use crate::app::editor::{AssetsMsg, Editor, EditorMsg, EditorPrefs, RightClick};
use crate::app::ime::ime_aligned;
use crate::app::keyed_column::keyed_column;
use crate::app::right_press::right_press_in;
use crate::app::scroll_watch::scroll_watch;
use crate::app::view::APP_FONT;
use crate::markdown::parse::ContainerKind;
use iced::advanced::text::Wrapping;
use iced::widget::{column, container, mouse_area, row, scrollable, space, stack, text_editor};
use iced::{Color, Element, Fill, Padding};

/// A widget of the page column, for it to keep its state (its shaped text)
/// while blocks come and go before it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Part {
    /// The gap above an entry.
    Gap(EntryKey),
    /// An entry: a rendered block, the region being edited.
    Shown(EntryKey),
    /// Where a dragged item would go after an entry.
    Drop(EntryKey),
    /// Entries without widgets, from this one on: their room.
    Far(usize),
    /// The room below the last entry.
    End,
}

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

    /// What the column shows of `entries`: the room each takes (its gap and
    /// its height, as last laid out or guessed) and which get widgets.
    fn near_plan(&self, entries: &[Entry], width: f32, zoom: f32) -> (Vec<f32>, near::Plan) {
        let mut heights = self.heights.borrow_mut();
        heights.at(width, zoom);
        heights.prune(entries.iter().map(|e| &e.key));
        let text = self.page.text();
        let rooms: Vec<(f32, bool)> = entries
            .iter()
            .map(|e| {
                let source = &text[e.covers.clone()];
                let height = heights.get(e.key, near::may_render(e, source));
                let guess = || near::estimate(source, width, zoom);
                (gap_height(e.gap, zoom) + height.unwrap_or_else(guess), height.is_some())
            })
            .collect();
        // The visible part in the column (below the page's padding).
        let visible = self.visible();
        let top = visible.top - PAGE_PADDING[0];
        let margin = visible.height.max(near::MIN_MARGIN);
        let keep = |i: usize| match entries[i].shown {
            Shown::Active => true,
            Shown::Block(b) => Some(b) == self.scroll_target,
        };
        let plan = near::plan(&rooms, top..top + visible.height, margin, keep);
        (rooms.into_iter().map(|r| r.0).collect(), plan)
    }

    pub(super) fn document<'a>(&'a self, ctx: &Ctx<'a>, prefs: &EditorPrefs) -> Element<'a, EditorMsg> {
        let width = self.column_width(ctx.zoom);
        let dragging = self.asset_dragging();
        let p = ctx.p;
        let unbounded = (f32::NEG_INFINITY, f32::INFINITY);
        let (content, band): (Element<EditorMsg>, _) = match &self.source {
            Some(source) => (self.source_view(source, ctx, prefs), unbounded),
            None => {
                // Only the blocks near the visible part get widgets; those
                // laid out tell their heights, for when they get none.
                let entries = column::entries(&self.page);
                let (rooms, plan) = self.near_plan(&entries, width, ctx.zoom);
                let heights = &self.heights;
                let mut col = keyed_column().width(width).on_layout(move |part, size| {
                    if let Part::Shown(key) = part {
                        heights.borrow_mut().record(*key, size.height);
                    }
                });
                // Entries without widgets in a row: the first one and their room.
                let mut far: Option<(usize, f32)> = None;
                let far_view = |room: f32| container(space().width(Fill).height(room)).id(self.id("far"));
                let page = &self.page;
                let active = page.active();
                let selected = page.selected_blocks().map(|(a, b)| a.min(b)..=a.max(b));
                let is_selected = |i: usize| selected.as_ref().is_some_and(|s| s.contains(&i));
                // The highlight of selected blocks goes over them: one with a
                // background of its own (a level-2 heading's band, a code box, a
                // table's header) would hide it from behind. It is translucent,
                // takes no clicks and spans the column, containers included, so
                // that with the gaps between selected blocks it makes one band.
                let highlighted = move |content: Element<'a, EditorMsg>| -> Element<'a, EditorMsg> {
                    let layer =
                        container(space()).width(Fill).height(Fill).style(move |_| container::background(p.selection));
                    stack![container(content).width(Fill), layer].into()
                };
                let mut previous_end = None;
                let mut previous_block = None;
                for (n, entry) in entries.into_iter().enumerate() {
                    let block = match entry.shown {
                        Shown::Block(i) => Some(i),
                        Shown::Active => None,
                    };
                    if !plan.built[n] {
                        far.get_or_insert((n, 0.0)).1 += rooms[n];
                        previous_end = Some(entry.covers.end);
                        previous_block = block;
                        continue;
                    }
                    if let Some((first, room)) = far.take() {
                        col = col.push(Part::Far(first), far_view(room));
                    }
                    let space_above = gap_height(entry.gap, ctx.zoom);
                    if space_above > 0.0 {
                        // A click in the gap goes to the nearer block: the upper
                        // half to the end of the one above, the lower half to the
                        // start of the one below.
                        let after = previous_end.map_or(0, |end| content_end(page.text(), 0..end));
                        let half = |at: usize| -> Element<EditorMsg> {
                            let mut area = mouse_area(space().width(Fill).height(space_above / 2.0))
                                .on_press(EditorMsg::PressedAt(at))
                                .on_right_press(EditorMsg::RightClicked(RightClick::At(at)));
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
                        let gap_view = in_containers(gap_view, &entry.shared, p, ctx.zoom);
                        // Between two selected blocks the highlight goes on too.
                        let between_selected =
                            previous_block.zip(block).is_some_and(|(a, b)| is_selected(a) && is_selected(b));
                        col = col.push(
                            Part::Gap(entry.key),
                            if between_selected { highlighted(gap_view) } else { gap_view },
                        );
                    }
                    let kinds = entry.kinds();
                    let inner = inner_width(width, &kinds, ctx.zoom);
                    let shown: Element<EditorMsg> = match (entry.shown, active) {
                        (Shown::Active, Some(a)) => {
                            in_containers(self.active_view(a, ctx, prefs, inner), &kinds, p, ctx.zoom)
                        }
                        (Shown::Block(i), _) => {
                            let rendered = in_containers(self.block_view(i, ctx, inner), &kinds, p, ctx.zoom);
                            let rendered = if is_selected(i) { highlighted(rendered) } else { rendered };
                            // As wide as the page, so that a click beside the block's
                            // content (an empty item, a line of math) reaches it too.
                            let boxed = container(rendered).width(Fill).id(self.id(&format!("block-{i}")));
                            let mut area = mouse_area(boxed)
                                .on_press(EditorMsg::BlockPressed(i))
                                .on_right_press(EditorMsg::RightClicked(RightClick::Block(i)));
                            // Entering a block matters while the button is held from a
                            // press on the page (selecting blocks) or an item is dragged.
                            if self.pressed() || dragging {
                                area = area.on_enter(EditorMsg::BlockEntered(i));
                            }
                            if dragging {
                                area = area.on_move(move |_| EditorMsg::Assets(AssetsMsg::BlockHovered(i)));
                            }
                            area.into()
                        }
                        (Shown::Active, None) => continue,
                    };
                    col = col.push(Part::Shown(entry.key), shown);
                    // Where a dragged item would go as a paragraph of its own.
                    if let Some(DropTarget::Paragraph { at }) = ctx.editor.drop_target()
                        && content_end(page.text(), entry.covers.clone()) == *at
                    {
                        col = col.push(
                            Part::Drop(entry.key),
                            container(space().height(2)).width(Fill).style(move |_| container::background(p.link)),
                        );
                    }
                    previous_end = Some(entry.covers.end);
                    previous_block = block;
                }
                if let Some((first, room)) = far.take() {
                    col = col.push(Part::Far(first), far_view(room));
                }
                // Room to click below the last block.
                let last = previous_end.map_or(0, |end| content_end(page.text(), 0..end));
                let mut room = mouse_area(container(space()).width(Fill).height(120.0 * ctx.zoom))
                    .on_press(EditorMsg::PressedAt(page.text().len()))
                    .on_right_press(EditorMsg::RightClicked(RightClick::At(page.text().len())));
                if dragging {
                    room = room.on_move(move |_| EditorMsg::Assets(AssetsMsg::GapHovered(last)));
                }
                col = col.push(Part::End, room);
                // Where the visible part may move without other blocks needing
                // widgets, in the page (below its padding).
                let band = (plan.band.0 + PAGE_PADDING[0], plan.band.1 + PAGE_PADDING[0]);
                // Where a press lands, for the editor that takes the block's place.
                (PressRelay::tracking(col, EditorMsg::PressPoint).into(), band)
            }
        };
        let padded = container(content).padding(PAGE_PADDING).center_x(Fill);
        // Scrolling tells where the page is only when the visible part comes
        // near the end of the blocks built around it: a message for every
        // step of the wheel would build every window again.
        let page = scrollable(padded).id(self.id("doc")).width(Fill).height(Fill);
        let page = scroll_watch(page, self.id("doc"), band, EditorMsg::PageScrolled);
        // While an item of the assets pane is dragged, the page tells where it
        // is. A right click no block took opens the page's menu.
        let mut area = mouse_area(page).on_right_press(EditorMsg::RightClicked(RightClick::Page));
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
        let editor =
            right_press_in(ime_aligned(editor), Padding::ZERO, |p| EditorMsg::RightClicked(RightClick::Editor(p)));
        container(editor).id(self.id("active-box")).into()
    }
}
