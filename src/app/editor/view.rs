//! Rendering an editor window: the menu bar, the page as rendered blocks with
//! the active region as a text editor, the table being edited, the outline,
//! the status bar and dialogs.
//!
//! Text is laid out in pieces of one line (no line breaks inside), each in a
//! widget that reports where the pointer is in it, so that a click can be
//! mapped back to the source (`hit_test`).

use super::find::FindBar;
use super::highlight::{self, MarkdownHighlighter};
use super::keys::{self, BindContext, EditorCommand, MENUS, TABLE_MENU};
use super::live_text::{LiveText, Run, RunKind, TextLook};
use super::modal::modal;
use super::page::{Active, Live, TableState};
use super::render::{RenderCache, RenderKey, Rendered};
use super::{DialogAnswer, Editor, EditorDialog, EditorMsg, EditorPrefs, FindMsg, TableMsg};
use crate::app::arrow_cursor::arrow_cursor;
use crate::app::ime::ime_aligned;
use crate::app::view::APP_FONT;
use crate::markdown::content::{self, Inline, Link, Node, SourceMap, Style, TableNode};
use crate::markdown::edit::CodeBody;
use crate::markdown::live::{Display, Look, Object};
use crate::markdown::parse::ContainerKind;
use crate::markdown::{BlockKind, ListItem, images, table};
use iced::advanced::text::{Paragraph as _, Shaping, Wrapping};
use iced::widget::text::{LineHeight, Span};
use iced::widget::{
    Column, Row, Stack, button, center, column, container, image, mouse_area, opaque, pin, rich_text, row, rule,
    scrollable, space, stack, svg, text, text_editor, text_input,
};
use iced::{Alignment, Background, Border, Color, Element, Fill, Font, Length, Point, Size, Theme, border, font};
use std::sync::Arc;

pub const CODE_FONT: Font = if cfg!(windows) { Font::with_name("Consolas") } else { Font::MONOSPACE };
/// Body text size and line height at 100%.
pub const BODY_SIZE: f32 = 15.0;
const LINE_HEIGHT: f32 = 1.6;
/// The width of the page's column at 100%.
const PAGE_WIDTH: f32 = 820.0;
const PAGE_PADDING: [f32; 2] = [24.0, 36.0];
const HEADING_SCALE: [f32; 6] = [1.9, 1.55, 1.3, 1.15, 1.0, 0.95];
const LIST_INDENT: f32 = 26.0;
const BLOCK_GAP: f32 = 10.0;
const QUOTE_BAR: f32 = 4.0;
const QUOTE_PADDING: f32 = 14.0;
const CODE_PADDING: f32 = 10.0;
const OUTLINE_WIDTH: f32 = 220.0;
const MENU_BUTTON_WIDTH: f32 = 82.0;

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
}

/// A piece of rendered text (one line, no breaks) and where its characters
/// come from, for mapping a click to the source.
#[derive(Debug, Clone)]
pub struct HitInfo {
    pub block: usize,
    pub runs: Vec<HitRun>,
    /// The width the text was laid out in.
    pub width: f32,
}

#[derive(Debug, Clone)]
pub struct HitRun {
    pub text: String,
    pub size: f32,
    pub font: Font,
    /// None for text that is no source (a bullet, an emoji's image...).
    pub map: Option<SourceMap>,
}

/// The source offset a point in a piece of text stands for.
pub fn hit_test(info: &HitInfo, p: Point) -> Option<usize> {
    let spans: Vec<Span<'_, (), Font>> =
        info.runs.iter().map(|r| Span::new(r.text.as_str()).size(r.size).font(r.font)).collect();
    let size = info.runs.first().map_or(BODY_SIZE, |r| r.size);
    let paragraph = iced::advanced::graphics::text::Paragraph::with_spans(iced::advanced::Text {
        content: spans.as_slice(),
        bounds: Size::new(info.width, f32::INFINITY),
        size: size.into(),
        line_height: LineHeight::Relative(LINE_HEIGHT),
        font: APP_FONT,
        align_x: iced::advanced::text::Alignment::Default,
        align_y: iced::alignment::Vertical::Top,
        shaping: Shaping::Advanced,
        wrapping: Wrapping::WordOrGlyph,
    });
    let iced::advanced::text::Hit::CharOffset(index) = paragraph.hit_test(p)?;
    let mut start = 0;
    let mut last = None;
    for run in &info.runs {
        let end = start + run.text.len();
        if let Some(map) = &run.map {
            if index < end || (index == end && run.map.is_some()) {
                return Some(map.source_of(index.saturating_sub(start).min(run.text.len())));
            }
            last = Some(map.source_range().end);
        }
        start = end;
    }
    last
}

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

fn bold(f: Font) -> Font {
    Font { weight: font::Weight::Bold, ..f }
}

fn italic(f: Font) -> Font {
    Font { style: font::Style::Italic, ..f }
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
        let find: Element<EditorMsg> = match &self.find {
            Some(bar) => self.find_bar(bar, p),
            None => space().into(),
        };
        let body = row![outline, document];
        let main = column![self.menu_bar(p), find, container(body).height(Fill), self.status_bar(p, prefs)];
        let main = container(main).style(move |_| container::background(p.bg).color(p.text));
        let mut layers: Stack<EditorMsg> = stack![main].width(Fill).height(Fill);
        if let Some(i) = self.menu {
            layers = layers.push(self.menu_dropdown(i, p, prefs));
        }
        if let Some(at) = self.cell_menu {
            layers = layers.push(self.cell_menu_view(at, p));
        }
        if let Some(dialog) = self.dialogs.front() {
            layers = layers.push(dialog_view(dialog, p));
        }
        // While a table is edited, where the pointer is (a cell's menu opens there).
        let mut root = mouse_area(layers);
        if self.page.active.as_ref().is_some_and(|a| a.table.is_some()) {
            root = root.on_move(|p| EditorMsg::Table(TableMsg::PointerAt(p)));
        }
        arrow_cursor(root).into()
    }

    /// A cell's right-click menu at `at` in the window.
    fn cell_menu_view(&self, at: Point, p: Palette) -> Element<'_, EditorMsg> {
        const WIDTH: f32 = 220.0;
        const ITEM: f32 = 26.0;
        let mut list = Column::new().spacing(0).width(WIDTH);
        for &(label, item) in TABLE_MENU {
            list = list.push(
                button(text(label).size(12).color(p.text))
                    .width(Fill)
                    .padding([5, 10])
                    .style(move |_: &Theme, status| button::Style {
                        background: matches!(status, button::Status::Hovered).then_some(Background::Color(p.hover)),
                        text_color: p.text,
                        ..Default::default()
                    })
                    .on_press(EditorMsg::Table(TableMsg::MenuItem(item))),
            );
        }
        let height = TABLE_MENU.len() as f32 * ITEM + 8.0;
        let x = at.x.min(self.window_size.width - WIDTH - 12.0).max(0.0);
        let y = at.y.min(self.window_size.height - height - 4.0).max(0.0);
        let menu = container(list)
            .padding(4)
            .style(move |_| container::background(p.bg).border(border::color(p.muted).width(1).rounded(4)));
        stack![
            mouse_area(container(space()).width(Fill).height(Fill)).on_press(EditorMsg::Menu(None)),
            pin(menu).position(Point::new(x, y)),
        ]
        .into()
    }

    // --- Bars and menus ---

    fn menu_bar(&self, p: Palette) -> Element<'_, EditorMsg> {
        let mut bar = row![].spacing(0);
        for (i, (label, _)) in MENUS.iter().enumerate() {
            let open = self.menu == Some(i);
            bar = bar.push(
                button(text(*label).size(12).color(p.text))
                    .width(MENU_BUTTON_WIDTH)
                    .padding([4, 12])
                    .style(move |_: &Theme, status| button::Style {
                        background: (open || matches!(status, button::Status::Hovered))
                            .then_some(Background::Color(p.hover)),
                        text_color: p.text,
                        ..Default::default()
                    })
                    .on_press(EditorMsg::Menu(if open { None } else { Some(i) })),
            );
        }
        container(bar)
            .width(Fill)
            .style(move |_| container::background(p.bar_bg).border(border::color(p.border).width(1)))
            .into()
    }

    fn menu_dropdown(&self, index: usize, p: Palette, prefs: &EditorPrefs) -> Element<'_, EditorMsg> {
        let (_, items) = MENUS[index];
        let mut list = Column::new().spacing(0).width(320);
        for (label, shortcut, cmd) in items.iter() {
            let checked = match cmd {
                keys::EditorCommand::ToggleAutoPair => Some(prefs.auto_pair),
                keys::EditorCommand::DarkTheme => Some(prefs.dark),
                keys::EditorCommand::SourceMode => Some(self.source.is_some()),
                keys::EditorCommand::Outline => Some(self.outline),
                keys::EditorCommand::FullScreen => Some(self.fullscreen),
                _ => None,
            };
            let mark = match checked {
                Some(true) => "✓",
                _ => "",
            };
            list = list.push(
                button(
                    row![
                        text(mark).size(12).width(16).color(p.text),
                        text(*label).size(12).width(Fill).color(p.text),
                        text(*shortcut).size(11).color(p.muted)
                    ]
                    .align_y(Alignment::Center),
                )
                .width(Fill)
                .padding([5, 10])
                .style(move |_: &Theme, status| button::Style {
                    background: matches!(status, button::Status::Hovered).then_some(Background::Color(p.hover)),
                    text_color: p.text,
                    ..Default::default()
                })
                .on_press(EditorMsg::Command(*cmd)),
            );
        }
        let dropdown = container(list).padding(4).style(move |_| {
            // No shadow: the software renderer draws it over the box on partial redraws.
            container::background(p.bg).border(border::color(p.muted).width(1).rounded(4))
        });
        stack![
            mouse_area(container(space()).width(Fill).height(Fill)).on_press(EditorMsg::Menu(None)),
            pin(dropdown).position(Point::new(index as f32 * MENU_BUTTON_WIDTH, 30.0)),
        ]
        .into()
    }

    fn find_bar<'a>(&'a self, bar: &'a FindBar, p: Palette) -> Element<'a, EditorMsg> {
        let find = |m: FindMsg| EditorMsg::Find(m);
        let small = |label: &'static str, m: FindMsg| {
            button(text(label).size(12).center()).padding([3, 10]).style(button::secondary).on_press(find(m))
        };
        let query = text_input("Find", &bar.query)
            .id(self.id("find"))
            .on_input(move |q| find(FindMsg::Query(q)))
            .on_submit(find(FindMsg::Next))
            .size(13)
            .padding([3, 6])
            .width(260);
        let mut find_row = row![
            query,
            small("Previous", FindMsg::Previous),
            small("Next", FindMsg::Next),
            iced::widget::checkbox(bar.match_case)
                .label("Match case")
                .size(14)
                .text_size(12)
                .on_toggle(move |on| find(FindMsg::MatchCase(on))),
            text(self.find_status()).size(12).color(p.muted),
        ]
        .spacing(8)
        .align_y(Alignment::Center);
        find_row = find_row.push(space().width(Fill)).push(small("Close", FindMsg::Close));
        let mut rows = column![find_row].spacing(6);
        if bar.replacing {
            let replacement = text_input("Replace with", &bar.replacement)
                .id(self.id("replace"))
                .on_input(move |r| find(FindMsg::Replacement(r)))
                .on_submit(find(FindMsg::Replace))
                .size(13)
                .padding([3, 6])
                .width(260);
            rows = rows.push(
                row![replacement, small("Replace", FindMsg::Replace), small("Replace All", FindMsg::ReplaceAll)]
                    .spacing(8)
                    .align_y(Alignment::Center),
            );
        }
        container(rows)
            .padding([6, 10])
            .width(Fill)
            .style(move |_| container::background(p.bar_bg).border(border::color(p.border).width(1)))
            .into()
    }

    fn status_bar(&self, p: Palette, prefs: &EditorPrefs) -> Element<'_, EditorMsg> {
        let (all, selected) = self.counts();
        let counts = match selected {
            Some(s) => format!("{} / {} words, {} / {} characters", s.words, all.words, s.characters, all.characters),
            None => format!("{} words, {} characters", all.words, all.characters),
        };
        let mode = if self.source.is_some() { "Source" } else { "" };
        container(
            row![
                text(self.status.clone()).size(11).color(p.muted).width(Fill).wrapping(Wrapping::None),
                text(mode).size(11).color(p.muted),
                text(counts).size(11).color(p.muted),
                text(format!("Zoom: {:.0}%", prefs.zoom * 100.0)).size(11).color(p.muted),
            ]
            .spacing(16),
        )
        .padding([3, 8])
        .width(Fill)
        .clip(true)
        .style(move |_| container::background(p.bar_bg))
        .into()
    }

    fn outline_view<'a>(&'a self, ctx: &Ctx<'a>) -> Element<'a, EditorMsg> {
        let p = ctx.p;
        let mut list = Column::new().spacing(2).padding(10);
        for (i, h) in self.page.doc.headings.iter().enumerate() {
            list = list.push(
                button(text(h.text.clone()).size(12).color(p.text).wrapping(Wrapping::WordOrGlyph))
                    .padding(iced::Padding { left: 6.0 + 12.0 * f32::from(h.level - 1), ..iced::Padding::from([2, 6]) })
                    .width(Fill)
                    .style(move |_: &Theme, status| button::Style {
                        background: matches!(status, button::Status::Hovered).then_some(Background::Color(p.hover)),
                        text_color: p.text,
                        ..Default::default()
                    })
                    .on_press(EditorMsg::HeadingClicked(i)),
            );
        }
        container(scrollable(list).height(Fill))
            .width(OUTLINE_WIDTH)
            .height(Fill)
            .style(move |_| container::background(p.bar_bg).border(border::color(p.border).width(1)))
            .into()
    }

    // --- The page ---

    /// The width of the page's column in the window.
    fn column_width(&self, zoom: f32) -> f32 {
        let outline = if self.outline { OUTLINE_WIDTH } else { 0.0 };
        (self.window_size.width - outline - 2.0 * PAGE_PADDING[1]).clamp(120.0, PAGE_WIDTH * zoom)
    }

    fn document<'a>(&'a self, ctx: &Ctx<'a>, prefs: &EditorPrefs) -> Element<'a, EditorMsg> {
        let width = self.column_width(ctx.zoom);
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
                        let half = |at: usize| -> Element<EditorMsg> {
                            mouse_area(space().width(Fill).height(space_above / 2.0))
                                .on_press(EditorMsg::PressedAt(at))
                                .into()
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
                            mouse_area(boxed).on_press(EditorMsg::BlockPressed(i)).into()
                        }
                        (Entry::Active, None) => continue,
                    };
                    col = col.push(shown);
                    previous_end = Some(covers(entry).end);
                    previous = Some((kind, outer));
                }
                // Room to click below the last block.
                col = col.push(
                    mouse_area(container(space()).width(Fill).height(120.0 * ctx.zoom))
                        .on_press(EditorMsg::PressedAt(page.text.len())),
                );
                col.into()
            }
        };
        let padded = container(content).padding(PAGE_PADDING).center_x(Fill);
        scrollable(padded).id(self.id("doc")).on_scroll(EditorMsg::Scrolled).width(Fill).height(Fill).into()
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

    /// The active region: its source in a text editor (or the table editor).
    fn active_view<'a>(
        &'a self,
        a: &'a Active,
        ctx: &Ctx<'a>,
        prefs: &EditorPrefs,
        width: f32,
    ) -> Element<'a, EditorMsg> {
        let p = ctx.p;
        if let Some(state) = &a.table {
            return container(self.table_editor(a, state, ctx, prefs, width)).id(self.id("active-box")).into();
        }
        if let Some(live) = &a.live {
            return container(self.live_view(a, live, ctx, prefs, width)).id(self.id("active-box")).into();
        }
        if let Some(code) = &a.code {
            return self.code_editor(a, code, ctx, prefs);
        }
        let (size, font, padding) = region_text_style(&a.kind, ctx.zoom);
        let code = a.kind.is_code();
        let bind = BindContext { auto_pair: prefs.auto_pair, code, source_mode: false, table: false };
        let settings = highlight::Settings {
            dark: prefs.dark,
            page_start: a.region.start == 0,
            find: self.find_query(),
            code: None,
        };
        let background = if code { p.code_bg } else { Color::TRANSPARENT };
        let editor = text_editor(&a.content)
            .id(self.id("active"))
            .on_action(EditorMsg::Action)
            .key_binding(move |k| keys::binding(k, bind))
            .highlight_with::<MarkdownHighlighter>(settings, highlight::format)
            .size(size)
            .line_height(LINE_HEIGHT)
            .font(font)
            .padding(padding)
            .wrapping(Wrapping::WordOrGlyph)
            .style(move |_, _| editor_style(p, background));
        // Math and diagrams show what they become below their source.
        let preview: Option<Element<EditorMsg>> = match &a.kind {
            BlockKind::Math => {
                let tex = self.page.text[a.region.clone()].trim().trim_start_matches("$$").trim_end_matches("$$");
                Some(math_view(tex.trim(), true, ctx))
            }
            k if k.is_mermaid() => Some(mermaid_view(&self.active_diagram(a), ctx, width)),
            _ => None,
        };
        let shown: Element<EditorMsg> = match preview {
            Some(preview) => column![ime_aligned(editor), preview].spacing(8).into(),
            None => ime_aligned(editor).into(),
        };
        container(shown).id(self.id("active-box")).into()
    }

    /// The source of the Mermaid block being edited, keyed like the renders
    /// requested for the page's blocks.
    pub(super) fn active_diagram(&self, a: &Active) -> String {
        let block = self.page.active_block().filter(|&b| self.page.doc.blocks[b].kind.is_mermaid());
        if let Some(b) = block {
            return super::render::mermaid_source(&self.page.doc, b);
        }
        let src = &self.page.text[a.region.clone()];
        let body = src.split_once('\n').map_or("", |(_, b)| b);
        let body = body.rsplit_once('\n').map_or(body, |(b, last)| {
            if last.trim().starts_with("```") || last.trim().starts_with("~~~") { b } else { body }
        });
        format!("{body}\n")
    }

    /// The code block with the cursor: its code in its box, without the
    /// fences, and a field for its language.
    fn code_editor<'a>(
        &'a self,
        a: &'a Active,
        code: &'a CodeBody,
        ctx: &Ctx<'a>,
        prefs: &EditorPrefs,
    ) -> Element<'a, EditorMsg> {
        let p = ctx.p;
        let bind = BindContext { auto_pair: prefs.auto_pair, code: true, source_mode: false, table: false };
        let lang = match &a.kind {
            BlockKind::Code { lang, .. } => lang.clone().unwrap_or_default(),
            _ => String::new(),
        };
        let settings =
            highlight::Settings { dark: prefs.dark, page_start: false, find: self.find_query(), code: Some(lang) };
        let (size, font, padding) = code_text_style(ctx.zoom);
        let editor = text_editor(&a.content)
            .id(self.id("active"))
            .on_action(EditorMsg::Action)
            .key_binding(move |k| keys::binding(k, bind))
            .highlight_with::<MarkdownHighlighter>(settings, highlight::format)
            .size(size)
            .line_height(LINE_HEIGHT)
            .font(font)
            .padding(0)
            .wrapping(Wrapping::WordOrGlyph)
            .style(move |_, _| editor_style(p, Color::TRANSPARENT));
        let language = text_input("language", &code.info)
            .on_input(EditorMsg::CodeLanguage)
            .size(ctx.size(0.75))
            .padding([1, 6])
            .width(140)
            .style(move |_: &Theme, _| text_input::Style {
                background: Background::Color(Color::TRANSPARENT),
                border: border::color(p.border).width(1).rounded(3),
                icon: p.muted,
                placeholder: p.muted,
                value: p.text,
                selection: p.selection,
            });
        container(column![ime_aligned(editor), row![space().width(Fill), language]].spacing(4))
            .id(self.id("active-box"))
            .padding(padding)
            .width(Fill)
            .style(move |_| container::background(p.code_bg).border(border::rounded(4)))
            .into()
    }

    /// The block with the cursor in its rendered form, around the editing widget.
    fn live_view<'a>(
        &'a self,
        a: &'a Active,
        live: &'a Live,
        ctx: &Ctx<'a>,
        prefs: &EditorPrefs,
        width: f32,
    ) -> Element<'a, EditorMsg> {
        let p = ctx.p;
        let bind = BindContext { auto_pair: prefs.auto_pair, code: false, source_mode: false, table: false };
        let marks = {
            let region = a.region.clone();
            let in_region: Vec<std::ops::Range<usize>> =
                ctx.matches.iter().filter(|m| m.start >= region.start && m.end <= region.end).cloned().collect();
            let current = ctx.current.map(|(s, e)| s..e);
            live.display
                .shown_ranges(&in_region)
                .into_iter()
                .zip(in_region)
                .map(|(shown, source)| (shown, if Some(source) == current { p.find } else { p.find_bg }))
                .collect()
        };
        let body = |base: TextBase, w: f32, marks: Vec<(std::ops::Range<usize>, Color)>| -> Element<'a, EditorMsg> {
            self.live_text(live, base, ctx, w, marks).key_binding(move |k| keys::binding(k, bind)).into()
        };
        let base = TextBase { size: ctx.size(1.0), font: APP_FONT, color: p.text };
        match &a.kind {
            BlockKind::Heading(level) => {
                let scale = HEADING_SCALE[usize::from((*level).clamp(1, 6)) - 1];
                let heading =
                    body(TextBase { size: ctx.size(scale), font: bold(APP_FONT), color: p.heading }, width, marks);
                if *level <= 2 {
                    column![
                        heading,
                        rule::horizontal(1).style(move |_| rule::Style {
                            color: p.border,
                            radius: 0.0.into(),
                            fill_mode: rule::FillMode::Full,
                            snap: true,
                        })
                    ]
                    .spacing(4)
                    .into()
                } else {
                    heading
                }
            }
            BlockKind::ListItem(item) => {
                let indent = LIST_INDENT * ctx.zoom * item.depth as f32;
                let marker_w = LIST_INDENT * ctx.zoom;
                let block = self.page.active_block().unwrap_or(0);
                let marker: Element<EditorMsg> = if let Some(checked) = item.task {
                    container(
                        iced::widget::checkbox(checked)
                            .size(ctx.size(0.95))
                            .on_toggle(move |_| EditorMsg::TaskToggled(block)),
                    )
                    .padding(iced::Padding { top: ctx.size(0.25), ..Default::default() })
                    .width(marker_w)
                    .into()
                } else {
                    let glyph = match item.number {
                        Some(n) => format!("{n}."),
                        None => ["•", "◦", "▪"][item.depth.min(2)].to_string(),
                    };
                    text(glyph).size(ctx.size(1.0)).line_height(LINE_HEIGHT).color(p.text).width(marker_w).into()
                };
                let base = if item.task == Some(true) { TextBase { color: p.muted, ..base } } else { base };
                row![space().width(indent), marker, body(base, width - indent - marker_w, marks)].into()
            }
            BlockKind::Quote => quote_view(body(base, width - QUOTE_PADDING - QUOTE_BAR, marks), p),
            BlockKind::Footnote(label) => {
                let number = self.page.doc.footnotes.get(label).copied().unwrap_or(0);
                let label_w = 32.0 * ctx.zoom;
                row![
                    text(format!("[{number}]")).size(ctx.size(0.85)).color(p.muted).width(label_w),
                    body(base, width - label_w, marks),
                ]
                .into()
            }
            _ => body(base, width, marks),
        }
    }

    /// The editing widget for what a live region shows.
    fn live_text<'a>(
        &'a self,
        live: &'a Live,
        base: TextBase,
        ctx: &Ctx<'a>,
        width: f32,
        marks: Vec<(std::ops::Range<usize>, Color)>,
    ) -> LiveText<'a> {
        let p = ctx.p;
        let runs = live_runs(&live.display, base, ctx, &self.page, width);
        LiveText::new(self.id("active"), runs, live.display.text.len(), (live.sel.anchor, live.sel.head))
            .base(base.font, base.size, LINE_HEIGHT)
            .colors(p.selection, base.color)
            .marks(marks)
    }

    /// The table with the cursor, as a grid of cells; the cursor's cell is a text editor.
    fn table_editor<'a>(
        &'a self,
        a: &'a Active,
        state: &'a TableState,
        ctx: &Ctx<'a>,
        prefs: &EditorPrefs,
        width: f32,
    ) -> Element<'a, EditorMsg> {
        let p = ctx.p;
        let t = &state.table;
        let columns = t.columns().max(1);
        let cell_width = (width / columns as f32).max(40.0);
        let bind = BindContext { auto_pair: prefs.auto_pair, code: false, source_mode: false, table: true };
        let mut grid = Column::new();
        for (r, cells) in t.rows.iter().enumerate() {
            let mut contents = Row::new();
            let mut frames = Row::new();
            for (c, cell) in cells.iter().enumerate() {
                let align = t.align.get(c).copied().unwrap_or_default();
                let selected = state.range.is_some_and(|range| range.contains((r, c)));
                let base = TextBase {
                    size: ctx.size(1.0),
                    font: if r == 0 { bold(APP_FONT) } else { APP_FONT },
                    color: p.text,
                };
                let active = (r, c) == state.cell;
                let content: Element<EditorMsg> = if active && let Some(live) = &a.live {
                    let editor = self
                        .live_text(live, base, ctx, cell_width - 16.0, Vec::new())
                        .key_binding(move |k| keys::binding(k, bind));
                    mouse_area(editor)
                        .on_enter(EditorMsg::Table(TableMsg::Entered(r, c)))
                        .on_release(EditorMsg::Table(TableMsg::Released))
                        .on_right_press(EditorMsg::Table(TableMsg::Menu(r, c)))
                        .into()
                } else {
                    let doc = crate::markdown::Document::parse(&table::cell_to_text(cell));
                    let inlines =
                        if doc.blocks.is_empty() { Vec::new() } else { content::inlines(&doc, doc.block_events(0)) };
                    inline_text(&inlines, base, ctx, None, usize::MAX)
                };
                let align_x = match align {
                    table::Align::Center => Alignment::Center,
                    table::Align::Right => Alignment::End,
                    _ => Alignment::Start,
                };
                let mut cell_box = container(content).width(cell_width).padding([4, 8]).align_x(align_x);
                if active {
                    cell_box = cell_box.id(self.id("active-cell"));
                }
                contents = contents.push(cell_box);
                let bg = if selected {
                    p.selection
                } else if r == 0 {
                    p.header_bg
                } else {
                    Color::TRANSPARENT
                };
                // The whole cell (below its text too) takes the clicks.
                let mut frame = mouse_area(cell_frame(cell_width, bg, p))
                    .on_enter(EditorMsg::Table(TableMsg::Entered(r, c)))
                    .on_release(EditorMsg::Table(TableMsg::Released))
                    .on_right_press(EditorMsg::Table(TableMsg::Menu(r, c)));
                if !active {
                    frame = frame.on_press(EditorMsg::CellPressed(r, c));
                }
                frames = frames.push(frame);
            }
            grid = grid.push(table_row(contents, frames));
        }
        column![self.table_toolbar(p), grid].spacing(4).into()
    }

    /// Above the table being edited: alignment, size and deleting the table.
    fn table_toolbar(&self, p: Palette) -> Element<'_, EditorMsg> {
        let tool = |label: &'static str, cmd: EditorCommand| {
            button(text(label).size(11).center())
                .padding([2, 8])
                .style(move |_: &Theme, status| button::Style {
                    background: Some(Background::Color(if matches!(status, button::Status::Hovered) {
                        p.hover
                    } else {
                        p.bar_bg
                    })),
                    text_color: p.text,
                    border: border::color(p.border).width(1).rounded(3),
                    ..Default::default()
                })
                .on_press(EditorMsg::Command(cmd))
        };
        let field = |value: String, on_input: fn(String) -> TableMsg| {
            text_input("", &value)
                .on_input(move |s| EditorMsg::Table(on_input(s)))
                .on_submit(EditorMsg::Table(TableMsg::Resize))
                .size(11)
                .padding([2, 4])
                .width(44)
        };
        let (rows, cols) = self.resize_values();
        let label = |s: &'static str| text(s).size(11).color(p.muted);
        row![
            tool("Left", EditorCommand::AlignLeft),
            tool("Center", EditorCommand::AlignCenter),
            tool("Right", EditorCommand::AlignRight),
            space().width(12),
            label("Rows"),
            field(rows, TableMsg::Rows),
            label("Columns"),
            field(cols, TableMsg::Columns),
            button(text("Resize").size(11).center())
                .padding([2, 8])
                .style(button::secondary)
                .on_press(EditorMsg::Table(TableMsg::Resize)),
            space().width(Fill),
            tool("Delete Table", EditorCommand::DeleteTable),
        ]
        .spacing(6)
        .align_y(Alignment::Center)
        .into()
    }

    // --- Rendered blocks ---

    fn block_view<'a>(&'a self, i: usize, ctx: &Ctx<'a>, width: f32) -> Element<'a, EditorMsg> {
        let doc = &self.page.doc;
        let block = &doc.blocks[i];
        let p = ctx.p;
        let base = TextBase { size: ctx.size(1.0), font: APP_FONT, color: p.text };
        match &block.kind {
            BlockKind::Heading(level) => {
                let scale = HEADING_SCALE[usize::from((*level).clamp(1, 6)) - 1];
                let inlines = content::inlines(doc, doc.block_events(i));
                let base = TextBase { size: ctx.size(scale), font: bold(APP_FONT), color: p.heading };
                let heading = inline_text(&inlines, base, ctx, Some(width), i);
                if *level <= 2 {
                    column![
                        heading,
                        rule::horizontal(1).style(move |_| rule::Style {
                            color: p.border,
                            radius: 0.0.into(),
                            fill_mode: rule::FillMode::Full,
                            snap: true,
                        })
                    ]
                    .spacing(4)
                    .into()
                } else {
                    heading
                }
            }
            BlockKind::ThematicBreak => container(rule::horizontal(2).style(move |_| rule::Style {
                color: p.border,
                radius: 0.0.into(),
                fill_mode: rule::FillMode::Full,
                snap: true,
            }))
            .padding([8, 0])
            .into(),
            BlockKind::Code { lang, .. } => {
                if block.kind.is_mermaid() {
                    return mermaid_view(&super::render::code_of(doc, i), ctx, width);
                }
                code_view(doc, i, lang.as_deref(), ctx, width)
            }
            BlockKind::Math => {
                let tex = content::nodes(doc, doc.block_events(i))
                    .into_iter()
                    .find_map(|n| match n {
                        Node::Math(t) => Some(t),
                        Node::Paragraph(inl) => inl.into_iter().find_map(|x| match x {
                            Inline::Math { tex, .. } => Some(tex),
                            _ => None,
                        }),
                        _ => None,
                    })
                    .unwrap_or_default();
                math_view(&tex, true, ctx)
            }
            BlockKind::Table => table_view(&content::table_block(doc, i), ctx, i),
            BlockKind::FrontMatter => {
                let source = &self.page.text[block.range.clone()];
                container(text(source.to_string()).font(CODE_FONT).size(ctx.size(0.85)).color(p.muted))
                    .padding(CODE_PADDING)
                    .width(Fill)
                    .style(move |_| container::background(p.code_bg).border(border::rounded(4)))
                    .into()
            }
            BlockKind::Toc => {
                let mut list = Column::new().spacing(2);
                for (h, info) in doc.headings.iter().enumerate() {
                    list = list.push(
                        button(text(info.text.clone()).size(ctx.size(0.95)).color(p.link))
                            .padding(iced::Padding {
                                left: 4.0 + 18.0 * f32::from(info.level - 1),
                                ..iced::Padding::from([1, 4])
                            })
                            .style(|_, _| button::Style::default())
                            .on_press(EditorMsg::HeadingClicked(h)),
                    );
                }
                list.into()
            }
            BlockKind::Other => {
                let source = &self.page.text[block.range.clone()];
                text(source.to_string()).size(ctx.size(0.9)).color(p.muted).into()
            }
            BlockKind::ListItem(item) => self.list_item_view(i, item, ctx, width),
            BlockKind::Footnote(label) => {
                let number = doc.footnotes.get(label).copied().unwrap_or(0);
                let nodes = content::nodes(doc, doc.block_events(i));
                let label_w = 32.0 * ctx.zoom;
                row![
                    text(format!("[{number}]")).size(ctx.size(0.85)).color(p.muted).width(label_w),
                    nodes_view(&nodes, ctx, width - label_w, i),
                ]
                .into()
            }
            BlockKind::Quote | BlockKind::Html | BlockKind::Paragraph => {
                let nodes = content::nodes(doc, doc.block_events(i));
                if block.kind == BlockKind::Quote {
                    // An empty quote: a line of its bar.
                    let line = space().height(ctx.size(1.0) * LINE_HEIGHT);
                    quote_view(column![line, nodes_view(&nodes, ctx, width - QUOTE_PADDING - QUOTE_BAR, i)].into(), p)
                } else {
                    let _ = base;
                    nodes_view(&nodes, ctx, width, i)
                }
            }
        }
    }

    fn list_item_view<'a>(&'a self, i: usize, item: &ListItem, ctx: &Ctx<'a>, width: f32) -> Element<'a, EditorMsg> {
        let doc = &self.page.doc;
        let p = ctx.p;
        let indent = LIST_INDENT * ctx.zoom * item.depth as f32;
        let marker_w = LIST_INDENT * ctx.zoom;
        let nodes = content::nodes(doc, doc.block_events(i));
        // A checked task's text is muted.
        let done = Ctx { p: Palette { text: p.muted, link: p.muted, heading: p.muted, ..p }, ..*ctx };
        let body_ctx = if item.task == Some(true) { &done } else { ctx };
        let body = nodes_view(&nodes, body_ctx, width - indent - marker_w, i);
        let marker: Element<EditorMsg> = if let Some(checked) = item.task {
            container(
                iced::widget::checkbox(checked).size(ctx.size(0.95)).on_toggle(move |_| EditorMsg::TaskToggled(i)),
            )
            .padding(iced::Padding { top: ctx.size(0.25), ..Default::default() })
            .width(marker_w)
            .into()
        } else {
            let glyph = match item.number {
                Some(n) => format!("{n}."),
                None => ["•", "◦", "▪"][item.depth.min(2)].to_string(),
            };
            text(glyph).size(ctx.size(1.0)).line_height(LINE_HEIGHT).color(p.text).width(marker_w).into()
        };
        row![space().width(indent), marker, body].into()
    }
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
    (BODY_SIZE * zoom * 0.9, CODE_FONT, CODE_PADDING)
}

fn editor_style(p: Palette, background: Color) -> text_editor::Style {
    text_editor::Style {
        background: Background::Color(background),
        border: Border { radius: 4.0.into(), ..Border::default() },
        placeholder: p.muted,
        value: p.text,
        selection: p.selection,
    }
}

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

fn quote_view<'a>(content: Element<'a, EditorMsg>, p: Palette) -> Element<'a, EditorMsg> {
    row![
        container(space()).width(QUOTE_BAR).height(Fill).style(move |_| container::background(p.quote_bar)),
        container(content).padding(iced::Padding { left: QUOTE_PADDING - QUOTE_BAR, ..Default::default() }),
    ]
    .height(Length::Shrink)
    .into()
}

/// Nested content: paragraphs, headings, lists, quotes, code...
fn nodes_view<'a>(nodes: &[Node], ctx: &Ctx<'a>, width: f32, block: usize) -> Element<'a, EditorMsg> {
    let p = ctx.p;
    let base = TextBase { size: ctx.size(1.0), font: APP_FONT, color: p.text };
    let mut col = Column::new().spacing(BLOCK_GAP * ctx.zoom * 0.6);
    for node in nodes {
        col = col.push(match node {
            Node::Paragraph(inlines) | Node::Html(inlines) => paragraph_view(inlines, base, ctx, width, block),
            Node::Heading(level, inlines) => {
                let scale = HEADING_SCALE[usize::from((*level).clamp(1, 6)) - 1];
                let b = TextBase { size: ctx.size(scale), font: bold(APP_FONT), color: p.heading };
                inline_text(inlines, b, ctx, Some(width), block)
            }
            Node::Code { lang, text: code } => code_text_view(code, lang.as_deref(), None, ctx, width, block),
            Node::Math(tex) => math_view(tex, true, ctx),
            Node::Quote(children) => quote_view(nodes_view(children, ctx, width - QUOTE_PADDING, block), p),
            Node::List { start, items } => {
                let mut list = Column::new().spacing(2);
                for (n, item) in items.iter().enumerate() {
                    let marker_w = LIST_INDENT * ctx.zoom;
                    let marker: Element<EditorMsg> = match (item.task, start) {
                        (Some(c), _) => text(if c { "☑" } else { "☐" }).size(ctx.size(1.0)).width(marker_w).into(),
                        (None, Some(s)) => {
                            text(format!("{}.", s + n as u64)).size(ctx.size(1.0)).width(marker_w).into()
                        }
                        (None, None) => text("•").size(ctx.size(1.0)).width(marker_w).into(),
                    };
                    list = list.push(row![marker, nodes_view(&item.children, ctx, width - marker_w, block)]);
                }
                list.into()
            }
            Node::Rule => rule::horizontal(1).into(),
            Node::Table(t) => table_view(t, ctx, block),
            Node::FrontMatter(t) => text(t.clone()).font(CODE_FONT).color(p.muted).into(),
            Node::Footnote { number, children } => {
                row![text(format!("[{number}]")).color(p.muted), nodes_view(children, ctx, width - 32.0, block)].into()
            }
        });
    }
    col.into()
}

/// A paragraph: lines of text, or images, math and embeds among the text.
fn paragraph_view<'a>(
    inlines: &[Inline],
    base: TextBase,
    ctx: &Ctx<'a>,
    width: f32,
    block: usize,
) -> Element<'a, EditorMsg> {
    let p = ctx.p;
    let is_text = |i: &Inline| matches!(i, Inline::Text { .. } | Inline::Break { .. } | Inline::FootnoteRef { .. });
    if inlines.iter().all(is_text) {
        return inline_text(inlines, base, ctx, Some(width), block);
    }
    // Lines (split at breaks) of flowing pieces.
    let mut col = Column::new().spacing(4);
    for line in inlines.split(|i| matches!(i, Inline::Break { .. })) {
        let blank = |i: &Inline| matches!(i, Inline::Text { text, .. } if text.trim().is_empty());
        let only_images = line.iter().all(|i| matches!(i, Inline::Image { .. }) || blank(i));
        if only_images && line.iter().any(|i| matches!(i, Inline::Image { .. })) {
            for i in line {
                if let Inline::Image { .. } = i {
                    col = col.push(image_view(i, ctx, width, block));
                }
            }
            continue;
        }
        let line_end = line.last().map(|i| i.source().end);
        let mut flow = Row::new().align_y(Alignment::Center);
        let mut texts: Vec<Inline> = Vec::new();
        let flush = |texts: &mut Vec<Inline>, flow: Row<'a, EditorMsg>| -> Row<'a, EditorMsg> {
            let mut flow = flow;
            for word in split_words(texts) {
                flow = flow.push(inline_text(&word, base, ctx, None, block));
            }
            texts.clear();
            flow
        };
        for i in line {
            match i {
                Inline::Text { .. } | Inline::FootnoteRef { .. } => texts.push(i.clone()),
                Inline::Math { tex, display, .. } => {
                    flow = flush(&mut texts, flow);
                    flow = flow.push(math_view(tex, *display, ctx));
                }
                Inline::Image { .. } => {
                    flow = flush(&mut texts, flow);
                    flow = flow.push(image_view(i, ctx, width, block));
                }
                Inline::Embed { tag, src, .. } => {
                    flow = flush(&mut texts, flow);
                    flow = flow.push(embed_view(tag, src.as_deref(), p, ctx));
                }
                Inline::Break { .. } => {}
            }
        }
        flow = flush(&mut texts, flow);
        // A click to the right of the line puts the cursor at its end.
        let line = container(flow.wrap()).width(Fill);
        col = col.push(match line_end {
            Some(end) => mouse_area(line).on_press(EditorMsg::PressedAt(end)).into(),
            None => Element::from(line),
        });
    }
    col.into()
}

/// Text runs split into words (and single CJK characters), so that they can wrap in a row.
fn split_words(runs: &[Inline]) -> Vec<Vec<Inline>> {
    let mut out = Vec::new();
    for run in runs {
        let Inline::Text { text, style, link, map } = run else {
            out.push(vec![run.clone()]);
            continue;
        };
        let mut start = 0;
        let chars: Vec<(usize, char)> = text.char_indices().collect();
        for (n, (i, c)) in chars.iter().enumerate() {
            let next = chars.get(n + 1).map(|(j, _)| *j).unwrap_or(text.len());
            let cjk = *c as u32 >= 0x2e80;
            let boundary = c.is_whitespace() || cjk || chars.get(n + 1).is_some_and(|(_, d)| *d as u32 >= 0x2e80);
            if boundary || next == text.len() {
                let piece = &text[start..next];
                if !piece.is_empty() {
                    out.push(vec![Inline::Text {
                        text: piece.to_string(),
                        style: *style,
                        link: link.clone(),
                        map: map_slice(map, start..next),
                    }]);
                }
                start = next;
            }
            let _ = i;
        }
    }
    out
}

fn map_slice(map: &SourceMap, range: std::ops::Range<usize>) -> SourceMap {
    map.slice(range)
}

/// The spans of text runs (no images), as one text per line; each reports
/// the pointer for mapping clicks.
fn inline_text<'a>(
    inlines: &[Inline],
    base: TextBase,
    ctx: &Ctx<'a>,
    width: Option<f32>,
    block: usize,
) -> Element<'a, EditorMsg> {
    let mut lines: Vec<Element<'a, EditorMsg>> = Vec::new();
    for line in inlines.split(|i| matches!(i, Inline::Break { .. })) {
        lines.push(text_line(line, base, ctx, width, block));
    }
    if lines.len() == 1 {
        return lines.pop().unwrap_or_else(|| space().into());
    }
    Column::with_children(lines).into()
}

/// One line of text runs.
fn text_line<'a>(
    runs: &[Inline],
    base: TextBase,
    ctx: &Ctx<'a>,
    width: Option<f32>,
    block: usize,
) -> Element<'a, EditorMsg> {
    let p = ctx.p;
    let mut spans: Vec<Span<'static, SpanLink, Font>> = Vec::new();
    let mut hits: Vec<HitRun> = Vec::new();
    for run in runs {
        match run {
            Inline::Text { text: t, style, link, map } => {
                let (size, font) = run_font(*style, base);
                let mut color = if link.is_some() { p.link } else { base.color };
                if let Some(c) = style.color {
                    color = Color::from_rgb8(c.0, c.1, c.2);
                }
                if style.dim {
                    color = p.muted;
                }
                if style.code {
                    color = p.code;
                }
                // Pieces in and out of find matches.
                for (piece, found) in match_pieces(t, map, ctx) {
                    let mut span = Span::new(t[piece].to_string()).size(size).font(font).color(color);
                    if style.underline || link.is_some() {
                        span = span.underline(true);
                    }
                    if style.strike {
                        span = span.strikethrough(true);
                    }
                    if let Some(current) = found {
                        span = span.background(if current { p.find } else { p.find_bg });
                    } else if style.highlight {
                        span = span.background(p.mark_bg);
                    } else if style.code || style.kbd {
                        span = span.background(p.code_bg);
                    }
                    if let Some(Link { dest, .. }) = link {
                        span = span.link(SpanLink { dest: dest.clone(), at: map.source_range().start });
                    }
                    spans.push(span);
                }
                hits.push(HitRun { text: t.clone(), size, font, map: Some(map.clone()) });
            }
            Inline::FootnoteRef { label, number, source } => {
                let size = base.size * 0.75;
                spans.push(
                    Span::new(format!("[{number}]"))
                        .size(size)
                        .color(p.link)
                        .link(SpanLink { dest: format!("^{label}"), at: source.start }),
                );
                hits.push(HitRun { text: format!("[{number}]"), size, font: base.font, map: None });
            }
            _ => {}
        }
    }
    if spans.is_empty() {
        // An empty line keeps its height.
        return text(" ").size(base.size).line_height(LINE_HEIGHT).into();
    }
    let mut rich = rich_text(spans)
        .size(base.size)
        .font(base.font)
        .line_height(LINE_HEIGHT)
        .wrapping(Wrapping::WordOrGlyph)
        .on_link_click(|l: SpanLink| EditorMsg::Link(l.dest, l.at));
    if let Some(w) = width {
        rich = rich.width(w);
    }
    let info = Arc::new(HitInfo { block, runs: hits, width: width.unwrap_or(f32::INFINITY) });
    mouse_area(rich).on_move(move |pt| EditorMsg::PointerOver(info.clone(), pt)).on_press(EditorMsg::TextPressed).into()
}

/// The size and font of a run's style.
/// A text run cut where find matches start and end: each piece, and whether
/// it is in a match (and in the selected one).
fn match_pieces(t: &str, map: &SourceMap, ctx: &Ctx) -> Vec<(std::ops::Range<usize>, Option<bool>)> {
    let source = map.source_range();
    if !ctx.matches.iter().any(|m| m.start < source.end && m.end > source.start) {
        return vec![(0..t.len(), None)];
    }
    let state = |i: usize| {
        let s = map.source_of(i);
        let k = ctx.matches.partition_point(|m| m.end <= s);
        ctx.matches.get(k).filter(|m| m.start <= s).map(|m| ctx.current == Some((m.start, m.end)))
    };
    let mut pieces: Vec<(std::ops::Range<usize>, Option<bool>)> = Vec::new();
    for (i, _) in t.char_indices() {
        let found = state(i);
        match pieces.last_mut() {
            Some((r, f)) if *f == found => r.end = i,
            _ => pieces.push((i..i, found)),
        }
    }
    for k in 0..pieces.len() {
        pieces[k].0.end = pieces.get(k + 1).map_or(t.len(), |p| p.0.start);
    }
    pieces
}

/// The runs the editing widget draws for what a live region shows: styled
/// like the rendered blocks, with the Markdown at the cursor dimmed.
fn live_runs(display: &Display, base: TextBase, ctx: &Ctx, page: &super::page::Page, width: f32) -> Vec<Run> {
    let p = ctx.p;
    let mut runs = Vec::new();
    // Text with its line breaks as runs of their own.
    let push_text = |runs: &mut Vec<Run>, shown: std::ops::Range<usize>, look: TextLook| {
        let mut start = shown.start;
        for (i, _) in display.text[shown.clone()].match_indices('\n') {
            let at = shown.start + i;
            if at > start {
                runs.push(Run {
                    shown: start..at,
                    kind: RunKind::Text { text: display.text[start..at].to_string(), look: look.clone() },
                });
            }
            runs.push(Run { shown: at..at + 1, kind: RunKind::Newline });
            start = at + 1;
        }
        if start < shown.end {
            runs.push(Run {
                shown: start..shown.end,
                kind: RunKind::Text { text: display.text[start..shown.end].to_string(), look },
            });
        }
    };
    let plain = |size: f32, font: Font, color: Color| TextLook {
        font,
        size,
        color,
        underline: false,
        strike: false,
        background: None,
        link: None,
    };
    for u in &display.units {
        match &u.look {
            Look::Text { style, link } => {
                let (size, font) = run_font(*style, base);
                let mut color = if link.is_some() { p.link } else { base.color };
                if let Some(c) = style.color {
                    color = Color::from_rgb8(c.0, c.1, c.2);
                }
                if style.dim {
                    color = p.muted;
                }
                if style.code {
                    color = p.code;
                }
                let background = if style.highlight {
                    Some(p.mark_bg)
                } else if style.code || style.kbd {
                    Some(p.code_bg)
                } else {
                    None
                };
                let look = TextLook {
                    font,
                    size,
                    color,
                    underline: style.underline || link.is_some(),
                    strike: style.strike,
                    background,
                    link: link.as_ref().map(|l| (l.dest.clone(), u.source.start)),
                };
                push_text(&mut runs, u.shown.clone(), look);
            }
            Look::Marker => push_text(&mut runs, u.shown.clone(), plain(base.size, base.font, p.muted)),
            Look::Break => {
                for i in u.shown.clone() {
                    runs.push(Run { shown: i..i + 1, kind: RunKind::Newline });
                }
            }
            Look::Object(o) | Look::Preview(o) => {
                let kind = match o {
                    Object::Image { src, alt, width: w, link, .. } => {
                        let path = images::image_path(&page.path, &page.text, src);
                        let max_width = w.map(|w| w * ctx.zoom);
                        match path {
                            Some(path) if path.is_file() => {
                                if path.extension().is_some_and(|e| e.eq_ignore_ascii_case("svg")) {
                                    RunKind::SvgFile { handle: svg::Handle::from_path(&path), max_width }
                                } else {
                                    RunKind::Image { handle: image::Handle::from_path(&path), max_width }
                                }
                            }
                            Some(_) => RunKind::Label {
                                text: format!("🖼 {alt} (missing: {src})"),
                                look: plain(base.size * 0.9, base.font, p.error),
                            },
                            None => RunKind::Label {
                                text: format!("🖼 {} {src}", if alt.is_empty() { "image" } else { alt }),
                                look: TextLook {
                                    underline: true,
                                    link: Some((link.as_ref().map_or(src.clone(), |l| l.dest.clone()), u.source.start)),
                                    ..plain(base.size * 0.9, base.font, p.link)
                                },
                            },
                        }
                    }
                    Object::Math { tex, display: block } => {
                        let key = RenderKey::Math { tex: tex.trim().to_string(), display: *block, dark: p.dark };
                        match ctx.renders.get(&key) {
                            Some(Rendered::Done(Ok(s))) => {
                                let scale = ctx.size(1.0) / 16.0 * if *block { 1.15 } else { 1.0 };
                                RunKind::Svg {
                                    handle: s.handle.clone(),
                                    width: s.width * scale,
                                    height: s.height * scale,
                                    center: true,
                                    display: *block,
                                }
                            }
                            Some(Rendered::Done(Err(_))) => RunKind::Label {
                                text: format!("${tex}$"),
                                look: plain(base.size * 0.9, CODE_FONT, p.error),
                            },
                            _ => RunKind::Label { text: tex.clone(), look: plain(base.size * 0.9, CODE_FONT, p.muted) },
                        }
                    }
                    Object::Footnote { label, number } => RunKind::Label {
                        text: format!("[{number}]"),
                        look: TextLook {
                            link: Some((format!("^{label}"), u.source.start)),
                            ..plain(base.size * 0.75, base.font, p.link)
                        },
                    },
                    Object::Embed { tag, src } => RunKind::Label {
                        text: format!("▶ {tag}{}", src.as_ref().map(|s| format!(" {s}")).unwrap_or_default()),
                        look: plain(base.size * 0.9, base.font, p.muted),
                    },
                };
                runs.push(Run { shown: u.shown.clone(), kind });
            }
        }
    }
    let _ = width;
    runs
}

fn run_font(style: Style, base: TextBase) -> (f32, Font) {
    let mut font = if style.code || style.kbd { CODE_FONT } else { base.font };
    if style.bold {
        font = bold(font);
    }
    if style.italic {
        font = italic(font);
    }
    let size = if style.sub || style.sup { base.size * 0.75 } else { base.size };
    (size, font)
}

fn code_view<'a>(
    doc: &crate::markdown::Document,
    block: usize,
    lang: Option<&str>,
    ctx: &Ctx<'a>,
    width: f32,
) -> Element<'a, EditorMsg> {
    // The code with where each line starts in the source.
    let mut code = String::new();
    let mut starts = Vec::new();
    for (e, r) in doc.block_events(block) {
        if let pulldown_cmark::Event::Text(t) = e {
            let mut offset = r.start;
            for line in t.split_inclusive('\n') {
                starts.push(offset);
                offset += line.len();
            }
            code.push_str(t);
        }
    }
    code_text_view(&code, lang, Some(&starts), ctx, width, block)
}

/// Code in a box, highlighted for its language; `starts` are the source
/// offsets of its lines, when they are known.
fn code_text_view<'a>(
    code: &str,
    lang: Option<&str>,
    starts: Option<&[usize]>,
    ctx: &Ctx<'a>,
    width: f32,
    block: usize,
) -> Element<'a, EditorMsg> {
    let p = ctx.p;
    let code = code.strip_suffix('\n').unwrap_or(code);
    let size = ctx.size(0.9);
    let colored = highlight::highlight_code(code, lang, p.dark);
    let mut lines = Column::new();
    for (n, line) in code.split('\n').enumerate() {
        let pieces: Vec<(String, Color)> =
            colored.as_ref().and_then(|c| c.get(n).cloned()).unwrap_or_else(|| vec![(line.to_string(), p.text)]);
        let spans: Vec<Span<'static, SpanLink, Font>> =
            pieces.iter().map(|(t, c)| Span::new(t.clone()).color(*c).font(CODE_FONT).size(size)).collect();
        let rich = rich_text(spans).size(size).font(CODE_FONT).line_height(LINE_HEIGHT).wrapping(Wrapping::WordOrGlyph);
        let element: Element<EditorMsg> = match starts.and_then(|s| s.get(n)) {
            Some(&start) => {
                let map = content::source_map(line.len(), start..start + line.len());
                let info = Arc::new(HitInfo {
                    block,
                    runs: vec![HitRun { text: line.to_string(), size, font: CODE_FONT, map: Some(map) }],
                    width: width - 2.0 * CODE_PADDING,
                });
                mouse_area(rich.width(width - 2.0 * CODE_PADDING))
                    .on_move(move |pt| EditorMsg::PointerOver(info.clone(), pt))
                    .on_press(EditorMsg::TextPressed)
                    .into()
            }
            None => rich.into(),
        };
        lines =
            lines.push(if line.is_empty() { text(" ").size(size).line_height(LINE_HEIGHT).into() } else { element });
    }
    container(lines)
        .padding(CODE_PADDING)
        .width(Fill)
        .style(move |_| container::background(p.code_bg).border(border::rounded(4)))
        .into()
}

fn math_view<'a>(tex: &str, display: bool, ctx: &Ctx<'a>) -> Element<'a, EditorMsg> {
    let p = ctx.p;
    let key = RenderKey::Math { tex: tex.trim().to_string(), display, dark: p.dark };
    let rendered = ctx.renders.get(&key);
    let content: Element<EditorMsg> = match rendered {
        Some(Rendered::Done(Ok(s))) => {
            // Math is typeset at 12 pt (16 px); the body text is BODY_SIZE.
            let scale = ctx.size(1.0) / 16.0 * if display { 1.15 } else { 1.0 };
            svg(s.handle.clone()).width(s.width * scale).height(s.height * scale).into()
        }
        Some(Rendered::Done(Err(e))) => {
            let source = text(if display { tex.to_string() } else { format!("${tex}$") })
                .font(CODE_FONT)
                .size(ctx.size(0.9))
                .color(p.math);
            if display {
                column![source, text(e.clone()).size(ctx.size(0.8)).color(p.error)].spacing(4).into()
            } else {
                source.into()
            }
        }
        _ => text(tex.to_string()).font(CODE_FONT).size(ctx.size(0.9)).color(p.muted).into(),
    };
    if display { container(content).center_x(Fill).padding([4, 0]).into() } else { content }
}

fn mermaid_view<'a>(source: &str, ctx: &Ctx<'a>, width: f32) -> Element<'a, EditorMsg> {
    let p = ctx.p;
    let key = RenderKey::Mermaid(source.to_string());
    match ctx.renders.get(&key) {
        Some(Rendered::Done(Ok(s))) => {
            let scale = ctx.zoom.min(width / s.width.max(1.0));
            container(svg(s.handle.clone()).width(s.width * scale).height(s.height * scale)).center_x(Fill).into()
        }
        Some(Rendered::Done(Err(e))) => column![
            container(text(source.to_string()).font(CODE_FONT).size(ctx.size(0.9)).color(p.text))
                .padding(CODE_PADDING)
                .width(Fill)
                .style(move |_| container::background(p.code_bg).border(border::rounded(4))),
            text(e.clone()).size(ctx.size(0.8)).color(p.error),
        ]
        .spacing(4)
        .into(),
        _ => text("…").color(p.muted).into(),
    }
}

/// An image of the page: the file, or its alt text when it can't be shown.
fn image_view<'a>(inline: &Inline, ctx: &Ctx<'a>, width: f32, block: usize) -> Element<'a, EditorMsg> {
    let p = ctx.p;
    let Inline::Image { src, alt, width: w, source, .. } = inline else { return space().into() };
    let editor = ctx.editor;
    let at = source.start;
    let path = images::image_path(&editor.page.path, &editor.page.text, src);
    let max = w.map_or(width, |w| (w * ctx.zoom).min(width));
    let shown: Element<EditorMsg> = match path {
        Some(path) if path.is_file() => {
            let is_svg = path.extension().is_some_and(|e| e.eq_ignore_ascii_case("svg"));
            if is_svg {
                svg(svg::Handle::from_path(&path)).width(Length::Shrink).height(Length::Shrink).into()
            } else {
                let img = image(image::Handle::from_path(&path));
                match w {
                    Some(_) => img.width(max).into(),
                    None => container(img).max_width(max).into(),
                }
            }
        }
        Some(_) => text(format!("🖼 {} (missing: {src})", alt)).size(ctx.size(0.9)).color(p.error).into(),
        None => rich_text(vec![
            Span::new(format!("🖼 {} ", if alt.is_empty() { "image" } else { alt })).color(p.muted),
            Span::new(src.clone()).color(p.link).underline(true).link(SpanLink { dest: src.clone(), at }),
        ])
        .size(ctx.size(0.9))
        .on_link_click(|l: SpanLink| EditorMsg::Link(l.dest, l.at))
        .into(),
    };
    let _ = block;
    mouse_area(shown).on_press(EditorMsg::PressedAt(at)).into()
}

fn embed_view<'a>(tag: &str, src: Option<&str>, p: Palette, ctx: &Ctx<'a>) -> Element<'a, EditorMsg> {
    let label = format!("▶ {tag}");
    let mut spans: Vec<Span<'static, SpanLink, Font>> = vec![Span::new(label).color(p.muted)];
    if let Some(src) = src {
        spans.push(
            Span::new(format!(" {src}")).color(p.link).underline(true).link(SpanLink { dest: src.to_string(), at: 0 }),
        );
    }
    container(rich_text(spans).size(ctx.size(0.9)).on_link_click(|l: SpanLink| EditorMsg::Link(l.dest, l.at)))
        .padding(12)
        .style(move |_| container::background(p.code_bg).border(border::color(p.border).width(1).rounded(4)))
        .into()
}

/// A table that is not being edited.
fn table_view<'a>(t: &TableNode, ctx: &Ctx<'a>, block: usize) -> Element<'a, EditorMsg> {
    let p = ctx.p;
    let columns = t.header.len().max(1);
    let mut grid = Column::new();
    let all_rows = std::iter::once(&t.header).chain(t.rows.iter());
    for (r, cells) in all_rows.enumerate() {
        let mut contents = Row::new();
        let mut frames = Row::new();
        for c in 0..columns {
            let inlines = cells.get(c).cloned().unwrap_or_default();
            let header = r == 0;
            let base =
                TextBase { size: ctx.size(1.0), font: if header { bold(APP_FONT) } else { APP_FONT }, color: p.text };
            let align_x = match t.align.get(c) {
                Some(pulldown_cmark::Alignment::Center) => Alignment::Center,
                Some(pulldown_cmark::Alignment::Right) => Alignment::End,
                _ => Alignment::Start,
            };
            contents = contents.push(
                container(inline_text(&inlines, base, ctx, None, block))
                    .width(Length::FillPortion(1))
                    .padding([4, 8])
                    .align_x(align_x),
            );
            // The whole cell (beside and below its text too) takes the clicks.
            frames = frames.push(
                mouse_area(cell_frame(
                    Length::FillPortion(1),
                    if header { p.header_bg } else { Color::TRANSPARENT },
                    p,
                ))
                .on_press(EditorMsg::TableCellPressed(block, r, c)),
            );
        }
        grid = grid.push(table_row(contents, frames));
    }
    grid.into()
}

/// A table row: the cells' contents over their frames (background and
/// border), which take the height of the row's tallest cell so that the
/// borders of a row line up when a cell wraps.
fn table_row<'a>(contents: Row<'a, EditorMsg>, frames: Row<'a, EditorMsg>) -> Element<'a, EditorMsg> {
    Stack::new().push(contents).push_under(frames).into()
}

/// A table cell's background and border, as tall as its row.
fn cell_frame<'a>(width: impl Into<Length>, bg: Color, p: Palette) -> Element<'a, EditorMsg> {
    container(space())
        .width(width)
        .height(Fill)
        .style(move |_| container::background(bg).border(border::color(p.border).width(1)))
        .into()
}

fn dialog_view(dialog: &EditorDialog, p: Palette) -> Element<'_, EditorMsg> {
    let answer = |label: &'static str, a: DialogAnswer, primary: bool| -> Element<'_, EditorMsg> {
        button(text(label).size(13).center())
            .width(110)
            .style(if primary { button::primary } else { button::secondary })
            .on_press(EditorMsg::Dialog(a))
            .into()
    };
    let (body, buttons): (String, Vec<Element<EditorMsg>>) = match dialog {
        EditorDialog::Message(m) => (m.clone(), vec![answer("OK", DialogAnswer::Ok, true)]),
        EditorDialog::Unsaved => (
            "The page has unsaved changes. Save them?".into(),
            vec![
                answer("Save", DialogAnswer::Save, true),
                answer("Don't Save", DialogAnswer::Discard, false),
                answer("Cancel", DialogAnswer::Cancel, false),
            ],
        ),
        EditorDialog::ChangedOnDisk => (
            "The file was changed by another application. Reload it, losing your changes?".into(),
            vec![answer("Reload", DialogAnswer::Discard, true), answer("Keep Mine", DialogAnswer::Cancel, false)],
        ),
    };
    let buttons = row![space().width(Fill)].extend(buttons).spacing(8);
    let card = container(column![text(body).size(13).color(p.text), buttons].spacing(16))
        .padding(20)
        .max_width(560)
        .style(move |_| container::background(p.bg).border(border::color(p.muted).width(1).rounded(8)));
    // The page's text widget keeps its keyboard focus under the dialog; the
    // dialog takes the keys, so that none of them edits the page.
    modal(opaque(center(card).style(|_| container::background(Color::from_rgba(0.0, 0.0, 0.0, 0.25)))), |event| {
        EditorMsg::Key(event)
    })
    .into()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(text: &str, source: std::ops::Range<usize>) -> HitRun {
        HitRun {
            text: text.into(),
            size: BODY_SIZE,
            font: APP_FONT,
            map: Some(content::source_map(text.len(), source)),
        }
    }

    #[test]
    fn a_click_maps_to_the_source_of_the_text_under_it() {
        // "hello world" rendered from `hello **world**`: "world" comes from 8..13.
        let info = HitInfo { block: 0, runs: vec![run("hello ", 0..6), run("world", 8..13)], width: 1000.0 };
        assert_eq!(hit_test(&info, Point::new(0.0, 5.0)), Some(0));
        assert_eq!(hit_test(&info, Point::new(999.0, 5.0)), Some(13));
        // Somewhere inside "world".
        let inside = hit_test(&info, Point::new(70.0, 5.0)).unwrap();
        assert!((8..=13).contains(&inside), "{inside}");
    }

    #[test]
    fn wrapped_lines_are_hit_by_their_height() {
        let text = "word ".repeat(30);
        let info = HitInfo { block: 0, runs: vec![run(&text, 100..100 + text.len())], width: 120.0 };
        let first = hit_test(&info, Point::new(1.0, 5.0)).unwrap();
        let third = hit_test(&info, Point::new(1.0, 2.5 * BODY_SIZE * LINE_HEIGHT)).unwrap();
        assert_eq!(first, 100);
        assert!(third > 110, "{third}");
    }
}
