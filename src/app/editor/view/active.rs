//! The active region: a block edited in its rendered form, a code block or
//! another code-like block as source, the table being edited.

use super::blocks::{cell_frame, math_view, mermaid_view, quote_view, table_row};
use super::inline::{inline_text, run_font};
use super::theme::{
    CODE_FONT, HEADING_SCALE, LINE_HEIGHT, LIST_INDENT, Palette, QUOTE_BAR, QUOTE_PADDING, bold, code_text_style,
    editor_style, region_text_style,
};
use super::{Ctx, TextBase};
use crate::app::editor::highlight::{self, MarkdownHighlighter};
use crate::app::editor::keys::{self, BindContext, EditorCommand};
use crate::app::editor::live_text::{LiveText, Run, RunKind, TextLook};
use crate::app::editor::page::{Active, Live, Mode, TableState};
use crate::app::editor::render::{RenderKey, Rendered};
use crate::app::editor::{Editor, EditorMsg, EditorPrefs, TableMsg};
use crate::app::ime::ime_aligned;
use crate::app::view::APP_FONT;
use crate::markdown::content;
use crate::markdown::edit::CodeBody;
use crate::markdown::live::{Display, Look, Object};
use crate::markdown::{BlockKind, images, table};
use iced::advanced::text::Wrapping;
use iced::widget::text_editor::Content;
use iced::widget::{
    Column, Row, button, column, container, image, mouse_area, row, rule, space, svg, text, text_editor, text_input,
};
use iced::{Alignment, Background, Color, Element, Fill, Font, Theme, border};

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
                        let path = images::image_path(page.path(), page.text(), src);
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
                let content: Element<EditorMsg> = if active && let Some(live) = a.live() {
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
}
