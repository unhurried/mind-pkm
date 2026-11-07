//! The active region: a block edited in its rendered form, a code block or
//! another code-like block as source, the table being edited.

use super::blocks::{cell_base, cell_frame, math_view, mermaid_view, quote_view, sideways, table_row};
use super::heading;
use super::inline::{cell_lines, inline_text, run_font};
use super::table_widths::{ColumnWidths, Piece};
use super::theme::{
    CELL_PADDING, CODE_FONT, HEADING_SCALE, LINE_HEIGHT, LIST_INDENT, Palette, QUOTE_BAR, QUOTE_PADDING,
    TABLE_COLUMN_MAX, bold, code_text_style, editor_style, region_text_style,
};
use super::{Ctx, TextBase};
use crate::app::editor::highlight::{self, MarkdownHighlighter};
use crate::app::editor::keys::{self, BindContext, EditorCommand};
use crate::app::editor::live_text::{LiveText, Run, RunKind, TextLook};
use crate::app::editor::page::{Active, Live, Mode, TableState};
use crate::app::editor::press_relay::PressRelay;
use crate::app::editor::render::{RenderKey, Rendered};
use crate::app::editor::{DataHandle, Editor, EditorMsg, EditorPrefs, RightClick, TableMsg};
use crate::app::floating::floating;
use crate::app::ime::ime_aligned;
use crate::app::right_press::right_press_in;
use crate::app::view::APP_FONT;
use crate::markdown::content;
use crate::markdown::edit::CodeBody;
use crate::markdown::live::{Display, Look, Object};
use crate::markdown::{BlockKind, images, table};
use iced::advanced::text::Wrapping;
use iced::widget::text_editor::Content;
use iced::widget::{
    Column, Row, button, column, container, image, mouse_area, row, space, svg, text, text_editor, text_input,
};
use iced::{Alignment, Background, Color, Element, Fill, Font, Padding, Theme, border};

/// What the cells of the table being edited show.
pub(super) struct EditedCells<'a> {
    /// The cursor's cell: its live region and the runs it shows.
    live_cell: Option<(&'a Live, Vec<Run>)>,
    /// Every cell's text (the cursor's cell's too, as it is in the file).
    shown: Vec<Vec<Vec<content::Inline>>>,
}

/// The most text a table's column takes on a line, at the zoom.
fn cell_max(ctx: &Ctx) -> f32 {
    TABLE_COLUMN_MAX * ctx.zoom
}

/// The lines (split at the line breaks) of the text the editing widget
/// shows, as pieces of text to measure.
fn run_lines(runs: &[Run]) -> Vec<Vec<Piece>> {
    let mut lines = vec![Vec::new()];
    for run in runs {
        match &run.kind {
            RunKind::Text { text, look } | RunKind::Label { text, look } => {
                if let Some(line) = lines.last_mut() {
                    line.push((text.clone(), look.size, look.font));
                }
            }
            RunKind::Newline => lines.push(Vec::new()),
            _ => {}
        }
    }
    lines
}

/// The runs the editing widget draws for what a live region shows: styled
/// like the rendered blocks, with the Markdown at the cursor dimmed.
fn live_runs(
    display: &Display,
    base: TextBase,
    ctx: &Ctx,
    page: &crate::app::editor::page::Page,
    width: f32,
) -> Vec<Run> {
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
                        let max_width = w.map(|w| w * ctx.zoom);
                        let broken = |why: String| RunKind::Label {
                            text: format!("🖼 {alt} ({why})"),
                            look: plain(base.size * 0.9, base.font, p.error),
                        };
                        let path = || images::image_path(page.path(), page.text(), src);
                        match ctx.editor.data_image(src).ok_or_else(path) {
                            Ok(DataHandle::Svg(handle)) => RunKind::SvgFile { handle, max_width },
                            Ok(DataHandle::Raster(handle)) => RunKind::Image { handle, max_width },
                            Ok(DataHandle::Invalid(e)) => broken(format!("invalid data URL: {e}")),
                            Err(Some(path)) if ctx.editor.is_image_file(&path) => {
                                if path.extension().is_some_and(|e| e.eq_ignore_ascii_case("svg")) {
                                    RunKind::SvgFile { handle: svg::Handle::from_path(&path), max_width }
                                } else {
                                    RunKind::Image { handle: image::Handle::from_path(&path), max_width }
                                }
                            }
                            Err(Some(_)) => broken(format!("missing: {src}")),
                            Err(None) => RunKind::Label {
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

impl Editor {
    /// The active region: its source in a text editor (or the table editor).
    pub(super) fn active_view<'a>(
        &'a self,
        a: &'a Active,
        ctx: &Ctx<'a>,
        prefs: &EditorPrefs,
        width: f32,
    ) -> Element<'a, EditorMsg> {
        let p = ctx.p;
        let content = match &a.mode {
            Mode::Table(state, _) => {
                return container(self.table_editor(a, state, ctx, prefs, width)).id(self.id("active-box")).into();
            }
            Mode::Live(live) => {
                return container(self.live_view(a, live, ctx, prefs, width)).id(self.id("active-box")).into();
            }
            Mode::Code(code, content) => return self.code_editor(a, code, content, ctx, prefs),
            Mode::Source(content) => content,
        };
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
        let editor = text_editor(content)
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
        let editor = PressRelay::relaying(editor, self.held(Padding::from(padding)));
        let editor = right_press_in(editor, Padding::from(padding), |p| EditorMsg::RightClicked(RightClick::Editor(p)));
        // Math and diagrams show what they become below their source.
        let preview: Option<Element<EditorMsg>> = match &a.kind {
            BlockKind::Math => {
                let tex = self.page.text()[a.region.clone()].trim().trim_start_matches("$$").trim_end_matches("$$");
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
    pub(in crate::app::editor) fn active_diagram(&self, a: &Active) -> String {
        let block = self.page.active_block().filter(|&b| self.page.doc().blocks[b].kind.is_mermaid());
        if let Some(b) = block {
            return crate::app::editor::render::mermaid_source(self.page.doc(), b);
        }
        let src = &self.page.text()[a.region.clone()];
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
        content: &'a Content,
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
        let editor = text_editor(content)
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
        let editor = PressRelay::relaying(editor, self.held(Padding::ZERO));
        let editor = right_press_in(editor, Padding::ZERO, |p| EditorMsg::RightClicked(RightClick::Editor(p)));
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
                let level = (*level).clamp(1, 6);
                let scale = HEADING_SCALE[usize::from(level) - 1];
                let base =
                    TextBase { size: ctx.size(scale), font: bold(APP_FONT), color: heading::text_color(level, p) };
                heading::framed(level, base.size, width, ctx.zoom, p, |w| body(base, w, marks))
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
                let number = self.page.doc().footnotes.get(label).copied().unwrap_or(0);
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
        let runs = live_runs(&live.display, base, ctx, &self.page, width);
        self.live_widget(live, runs, base, ctx, marks)
    }

    /// The editing widget of a live region showing `runs`.
    fn live_widget<'a>(
        &'a self,
        live: &'a Live,
        runs: Vec<Run>,
        base: TextBase,
        ctx: &Ctx<'a>,
        marks: Vec<(std::ops::Range<usize>, Color)>,
    ) -> LiveText<'a> {
        let p = ctx.p;
        LiveText::new(self.id("active"), runs, live.display.text.len(), (live.sel.anchor, live.sel.head))
            .base(base.font, base.size, LINE_HEIGHT)
            .colors(p.selection, base.color)
            .held(self.pressed())
            .marks(marks)
    }

    /// The table with the cursor, as a grid of cells; the cursor's cell is a
    /// text editor. Its columns are as wide as when it is rendered, measured
    /// from what the cells show (the cursor's cell with its Markdown).
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
        let bind = BindContext { auto_pair: prefs.auto_pair, code: false, source_mode: false, table: true };
        let base = |r: usize| cell_base(r, ctx);
        let cells = self.edited_cells(a, state, ctx);
        let widths = self.edited_widths(state, &cells, ctx, width);
        let EditedCells { mut live_cell, shown } = cells;
        let mut grid = Column::new();
        // `shown` has a cell for every cell of the table.
        for (r, row_shown) in shown.iter().enumerate() {
            let mut contents = Row::new();
            let mut frames = Row::new();
            for (c, inlines) in row_shown.iter().enumerate() {
                let cell_width = widths.widths.get(c).copied().unwrap_or(40.0);
                let align = t.align.get(c).copied().unwrap_or_default();
                let selected = state.range.is_some_and(|range| range.contains((r, c)));
                let active = (r, c) == state.cell;
                let content: Element<EditorMsg> = match live_cell.take_if(|_| active) {
                    Some((live, runs)) => {
                        let editor = self
                            .live_widget(live, runs, base(r), ctx, Vec::new())
                            .key_binding(move |k| keys::binding(k, bind));
                        mouse_area(editor)
                            .on_enter(EditorMsg::Table(TableMsg::Entered(r, c)))
                            .on_release(EditorMsg::Table(TableMsg::Released))
                            .on_right_press(EditorMsg::Table(TableMsg::Menu(r, c)))
                            .into()
                    }
                    None => inline_text(inlines, base(r), ctx, None, usize::MAX),
                };
                let align_x = match align {
                    table::Align::Center => Alignment::Center,
                    table::Align::Right => Alignment::End,
                    _ => Alignment::Start,
                };
                let mut cell_box = container(content).width(cell_width).padding(CELL_PADDING).align_x(align_x);
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
        let start = a.region.start;
        let grid = sideways(grid.into(), widths.scrolls, Some(self.id("table-scroll")), move |x| {
            EditorMsg::TableScrolled(start, x)
        });
        // The toolbar floats above the table, taking no room: going into the
        // table moves nothing. Hidden while a menu or a dialog is open, which
        // it would otherwise cover.
        let shown = !self.menu_open() && self.dialogs.is_empty();
        floating(grid, self.table_toolbar(p), 4.0, shown).into()
    }

    /// What the cells of the table being edited show: the cursor's cell its
    /// live runs, the others their text.
    pub(super) fn edited_cells<'a>(&'a self, a: &'a Active, state: &TableState, ctx: &Ctx<'a>) -> EditedCells<'a> {
        let live_cell = a
            .live()
            .map(|live| (live, live_runs(&live.display, cell_base(state.cell.0, ctx), ctx, &self.page, cell_max(ctx))));
        let shown = state
            .table
            .rows
            .iter()
            .map(|cells| {
                cells
                    .iter()
                    .map(|cell| {
                        let doc = crate::markdown::Document::parse(&table::cell_to_text(cell));
                        if doc.blocks.is_empty() { Vec::new() } else { content::inlines(&doc, doc.block_events(0)) }
                    })
                    .collect()
            })
            .collect();
        EditedCells { live_cell, shown }
    }

    /// The widths of the columns of the table being edited, from what its
    /// cells show, in a page column of `width`.
    pub(super) fn edited_widths(&self, state: &TableState, cells: &EditedCells, ctx: &Ctx, width: f32) -> ColumnWidths {
        let widest = |r: usize, c: usize| match &cells.live_cell {
            Some((_, runs)) if (r, c) == state.cell => self.table_widths.widest(&run_lines(runs)),
            _ => cells.shown[r]
                .get(c)
                .map_or(0.0, |inlines| self.table_widths.widest(&cell_lines(inlines, cell_base(r, ctx)))),
        };
        let rows = state.table.rows.len();
        let natural: Vec<f32> =
            (0..state.table.columns().max(1)).map(|c| (0..rows).map(|r| widest(r, c)).fold(0.0, f32::max)).collect();
        ctx.column_widths(&natural, width)
    }

    /// Above the table being edited, over the page: alignment, size and
    /// deleting the table, on a background of its own.
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
        let tools = row![
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
            space().width(12),
            tool("Delete Table", EditorCommand::DeleteTable),
        ]
        .spacing(6)
        .align_y(Alignment::Center);
        container(tools)
            .padding([3, 6])
            .style(move |_| container::background(p.bar_bg).border(border::color(p.border).width(1).rounded(4)))
            .into()
    }
}
