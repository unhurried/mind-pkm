//! The page: its blocks in a column, with the active region in place of the
//! blocks it covers, or the whole source in source mode.

use super::Ctx;
use super::blocks::quote_view;
use super::theme::{
    BLOCK_GAP, LINE_HEIGHT, LIST_INDENT, OUTLINE_WIDTH, PAGE_PADDING, PAGE_WIDTH, Palette, QUOTE_BAR, QUOTE_PADDING,
    editor_style,
};
use crate::app::editor::assets::{DropTarget, content_end};
use crate::app::editor::highlight::{self, MarkdownHighlighter};
use crate::app::editor::keys::{self, BindContext};
use crate::app::editor::{AssetsMsg, Editor, EditorMsg, EditorPrefs};
use crate::app::ime::ime_aligned;
use crate::app::view::APP_FONT;
use crate::markdown::BlockKind;
use crate::markdown::parse::ContainerKind;
use iced::advanced::text::Wrapping;
use iced::widget::{Column, column, container, mouse_area, row, scrollable, space, text_editor};
use iced::{Color, Element, Fill};

/// What a block in the column is, and the containers around it (with their
/// index in the page's parse when known).
type Described = (BlockKind, Vec<(ContainerKind, Option<usize>)>);

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
                let doc = &page.doc;
                // Tight list items stay close together; headings get more room above.
                let gap = |prev: Option<&BlockKind>, kind: &BlockKind| -> f32 {
                    let unit = BLOCK_GAP * ctx.zoom;
                    match (prev, kind) {
                        (None, _) => 0.0,
                        (Some(BlockKind::ListItem(a)), BlockKind::ListItem(b)) if !b.loose && a.list == b.list => {
                            2.0 * ctx.zoom
                        }
                        (_, BlockKind::Heading(_)) => unit * 1.6,
                        _ => unit,
                    }
                };
                let active = page.active.as_ref();
                let selected = page.blocks.map(|(a, b)| a.min(b)..=a.max(b));
                // What the column shows: the blocks, with the active region in
                // place of the blocks it covers (a new paragraph before the
                // block after it).
                enum Entry {
                    Block(usize),
                    Active,
                }
                let mut entries = Vec::new();
                let mut active_block = None;
                for (i, block) in doc.blocks.iter().enumerate() {
                    if let Some(a) = active {
                        let overlaps = block.range.end >= a.region.start && block.range.start <= a.region.end;
                        let placed = entries.iter().any(|e| matches!(e, Entry::Active));
                        if page.pending.is_none() && overlaps {
                            if !placed {
                                entries.push(Entry::Active);
                                active_block = Some(i);
                            }
                            continue;
                        }
                        if !placed && page.pending.as_ref().is_some_and(|pd| pd.at <= block.range.start) {
                            entries.push(Entry::Active);
                        }
                    }
                    entries.push(Entry::Block(i));
                }
                if active.is_some() && !entries.iter().any(|e| matches!(e, Entry::Active)) {
                    entries.push(Entry::Active);
                }
                // The kind of what an entry shows, and the containers around it
                // (with their index, when known).
                let describe = |e: &Entry| -> Described {
                    let of_block = |i: usize| -> Described {
                        let b = &doc.blocks[i];
                        (b.kind.clone(), b.outer.iter().map(|&c| (doc.containers[c].kind, Some(c))).collect())
                    };
                    match (e, &page.pending, active_block) {
                        (Entry::Block(i), _, _) => of_block(*i),
                        (Entry::Active, Some(pd), _) => {
                            (BlockKind::Paragraph, pd.outer.iter().map(|&k| (k, None)).collect())
                        }
                        (Entry::Active, None, Some(i)) => {
                            (active.map_or(BlockKind::Paragraph, |a| a.kind.clone()), of_block(i).1)
                        }
                        (Entry::Active, None, None) => (BlockKind::Paragraph, Vec::new()),
                    }
                };
                // The source range an entry covers.
                let covers = |e: &Entry| -> std::ops::Range<usize> {
                    match (e, &page.pending, active) {
                        (Entry::Block(i), _, _) => doc.blocks[*i].range.clone(),
                        (Entry::Active, Some(pd), _) => pd.at..pd.at,
                        (Entry::Active, None, Some(a)) => a.region.clone(),
                        (Entry::Active, None, None) => 0..0,
                    }
                };
                let mut previous: Option<Described> = None;
                let mut previous_end = None;
                for entry in &entries {
                    let (kind, outer) = describe(entry);
                    let space_above = gap(previous.as_ref().map(|p| &p.0), &kind);
                    if space_above > 0.0 {
                        // Inside the containers both sides share: a quote's bar goes on.
                        let shared: Vec<ContainerKind> = previous
                            .as_ref()
                            .map(|(_, prev)| {
                                prev.iter()
                                    .zip(&outer)
                                    .take_while(|(a, b)| a.0 == b.0 && (a.1.is_none() || b.1.is_none() || a.1 == b.1))
                                    .map(|(a, _)| a.0)
                                    .collect()
                            })
                            .unwrap_or_default();
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
                            Some(end) => column![half(end), half(covers(entry).start)].into(),
                            None => half(covers(entry).start),
                        };
                        col = col.push(in_containers(gap_view, &shared, p, ctx.zoom));
                    }
                    let kinds: Vec<ContainerKind> = outer.iter().map(|(k, _)| *k).collect();
                    let inner = inner_width(width, &kinds, ctx.zoom);
                    let shown: Element<EditorMsg> = match (entry, active) {
                        (Entry::Active, Some(a)) => {
                            in_containers(self.active_view(a, ctx, prefs, inner), &kinds, p, ctx.zoom)
                        }
                        (Entry::Block(i), _) => {
                            let i = *i;
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
                        (Entry::Active, None) => continue,
                    };
                    col = col.push(shown);
                    // Where a dragged item would go as a paragraph of its own.
                    if let Some(DropTarget::Paragraph { at }) = ctx.editor.drop_target()
                        && content_end(&page.text, covers(entry)) == *at
                    {
                        col = col.push(
                            container(space().height(2)).width(Fill).style(move |_| container::background(p.link)),
                        );
                    }
                    previous_end = Some(covers(entry).end);
                    previous = Some((kind, outer));
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
