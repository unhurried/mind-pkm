//! Rendering: the menu bar, the mind map, the search panel, dialogs.

use super::command::Command;
use super::dialog::Dialog;
use super::doc::Doc;
use super::drag::DropPlace;
use super::layout::{MapLayout, NodeBox};
use super::search_panel::{RESULTS_SCROLL_ID, ROW_HEIGHT, SEARCH_INPUT_ID, ScopeOption, SearchMsg, SortOption};
use super::{App, DIALOG_INPUT_ID, MAP_SCROLL_ID, Message};
use crate::mindmap::{ICON_CHECKMARK, MindMap, NodeId};
use crate::pkm;
use crate::search::{self, SearchHit, Snippet};
use iced::advanced::text::{Paragraph as _, Shaping, Wrapping};
use iced::widget::canvas::{self, Cache, Path, Stroke};
use iced::widget::text::{LineHeight, Span};
use iced::widget::{
    Column, Stack, button, canvas as canvas_widget, center, column, container, mouse_area, opaque, pick_list, pin,
    rich_text, row, scrollable, space, stack, text, text_input, tooltip,
};
use iced::{
    Alignment, Background, Border, Color, Element, Fill, Font, Length, Point, Rectangle, Renderer, Size, Theme, mouse,
};
use std::collections::HashMap;

/// The font of the whole application (the default font, and the one node
/// texts are measured with). Where it is missing, the system fallback fonts
/// are used.
pub const APP_FONT: Font = Font::with_name("Meiryo UI");

// The node sizes at 100% zoom.
const NODE_FONT_SIZE: f32 = 14.0;
/// The line height of node texts and icons, relative to the font size.
const NODE_LINE_HEIGHT: f32 = 1.2;
const NODE_MAX_TEXT_WIDTH: f32 = 360.0;
const NODE_PADDING: [f32; 2] = [1.0, 7.0];
const ICON_WIDTH: f32 = 18.0;
const ROW_SPACING: f32 = 3.0;
const LINK_FONT_SIZE: f32 = 13.0;
const NODE_RADIUS: f32 = 5.0;
const FOLD_MARKER_SIZE: f32 = 18.0;
const FOLD_MARKER_FONT_SIZE: f32 = 9.0;
const EDGE_WIDTH: f32 = 1.0;
/// Where the edges to a node's children branch: the share of the gap between
/// the node and its children.
const EDGE_BRANCH: f32 = 0.5;
/// The radius of the edges' corners (at 100% zoom).
const EDGE_RADIUS: f32 = 12.0;
const MENU_BUTTON_WIDTH: f32 = 64.0;

const EDGE_COLOR: Color = Color::from_rgb(0.55, 0.58, 0.62);
const SELECTED_BG: Color = Color::from_rgb(0.80, 0.88, 1.0);
const SELECTED_BORDER: Color = Color::from_rgb(0.23, 0.48, 0.90);
const MAP_BG: Color = Color::WHITE;
const MUTED: Color = Color::from_rgb(0.40, 0.42, 0.46);
const CHECK_COLOR: Color = Color::from_rgb(0.13, 0.60, 0.25);
const HIGHLIGHT_BG: Color = Color::from_rgb(1.0, 0.878, 0.4);
const LINK_COLOR: Color = Color::from_rgb(0.2, 0.4, 0.8);
const RED: Color = Color::from_rgb(0.8, 0.2, 0.2);
/// The menu and status bars.
const BAR_BG: Color = Color::from_rgb(0.95, 0.96, 0.97);
const BAR_BORDER: Color = Color::from_rgb(0.85, 0.86, 0.88);
const MENU_OPEN_BG: Color = Color::from_rgb(0.88, 0.90, 0.94);
const PANEL_BORDER: Color = Color::from_rgb(0.8, 0.82, 0.85);

type Menu = (&'static str, &'static [(&'static str, &'static str, Command)]);

pub const MENUS: &[Menu] = &[
    (
        "File",
        &[
            ("New", "Ctrl+N", Command::New),
            ("Open...", "Ctrl+O", Command::Open),
            ("Save", "Ctrl+S", Command::Save),
            ("Save As...", "Ctrl+Shift+S", Command::SaveAs),
            ("Exit", "Ctrl+Q", Command::Exit),
        ],
    ),
    (
        "Edit",
        &[
            ("Undo", "Ctrl+Z", Command::Undo),
            ("Redo", "Ctrl+Y", Command::Redo),
            ("New Child Node", "Tab / Insert", Command::NewChild),
            ("New Sibling Node", "Enter", Command::NewSibling),
            ("Edit Node Text", "Ctrl+E", Command::EditText),
            ("Delete Node", "Delete", Command::DeleteNode),
            ("Cut", "Ctrl+X", Command::Cut),
            ("Copy", "Ctrl+C", Command::Copy),
            ("Paste", "Ctrl+V", Command::Paste),
            ("Move Up", "Ctrl+Up", Command::MoveUp),
            ("Move Down", "Ctrl+Down", Command::MoveDown),
            ("Fold / Unfold", "Space", Command::ToggleFold),
            ("Set Link...", "Ctrl+K", Command::SetLink),
            ("Center Selected Node", "Esc", Command::CenterSelected),
        ],
    ),
    (
        "View",
        &[
            ("Zoom In", "Ctrl++", Command::ZoomIn),
            ("Zoom Out", "Ctrl+-", Command::ZoomOut),
            ("Reset Zoom", "Ctrl+0", Command::ZoomReset),
        ],
    ),
    (
        "PKM",
        &[
            ("Open", "F1", Command::PkmOpen),
            ("Edit", "F2", Command::PkmEdit),
            ("Fold One Level", "F3", Command::FoldOneLevel),
            ("Unfold One Level", "F4", Command::UnfoldOneLevel),
            ("Filter", "F5", Command::Filter),
            ("Search", "F6", Command::Search),
            ("Toggle Checkmark", "F7", Command::ToggleCheckmark),
            ("Add to ToDo", "F8", Command::AddToToDo),
            ("New Page", "F9", Command::NewPage),
            ("New Directory", "F10", Command::NewDirectory),
            ("Delete", "F11", Command::PkmDelete),
            ("Update Next Steps and Search Index", "F12", Command::Update),
            ("Add Assets", "", Command::AddAssets),
            ("Find Unlinked", "", Command::FindUnlinked),
        ],
    ),
];

// --- Measuring ---

/// The size of a node's text as the text widget will lay it out at the zoom
/// (the same font size for every node, the root included).
fn measure_text(content: &str, zoom: f32) -> Size {
    let content = if content.is_empty() { " " } else { content };
    let p = iced::advanced::graphics::text::Paragraph::with_text(iced::advanced::Text {
        content,
        bounds: Size::new(NODE_MAX_TEXT_WIDTH * zoom, f32::INFINITY),
        size: (NODE_FONT_SIZE * zoom).into(),
        line_height: LineHeight::Relative(NODE_LINE_HEIGHT),
        font: APP_FONT,
        align_x: iced::advanced::text::Alignment::Default,
        align_y: iced::alignment::Vertical::Top,
        shaping: Shaping::Advanced,
        wrapping: Wrapping::WordOrGlyph,
    });
    let b = p.min_bounds();
    Size::new(b.width.ceil() + 1.0, b.height.ceil())
}

/// `cache` holds the sizes at this zoom.
fn text_size(cache: &mut HashMap<String, Size>, text: &str, zoom: f32) -> Size {
    *cache.entry(text.to_string()).or_insert_with(|| measure_text(text, zoom))
}

/// The size of a node's box at the zoom: its text, icons and link marker,
/// with padding and the border.
pub fn node_size(cache: &mut HashMap<String, Size>, map: &MindMap, id: NodeId, zoom: f32) -> Size {
    let t = text_size(cache, &map.text(id), zoom);
    let extras = map.icons(id).len() + usize::from(map.link(id).is_some());
    let pad = NODE_PADDING;
    Size::new(
        t.width + extras as f32 * (ICON_WIDTH + ROW_SPACING) * zoom + 2.0 * pad[1] * zoom + 2.0,
        t.height + 2.0 * pad[0] * zoom + 2.0,
    )
}

// --- The window ---

impl App {
    pub(super) fn view(&self) -> Element<'_, Message> {
        let body: Element<Message> = match &self.doc {
            None => self.welcome(),
            Some(doc) => {
                let map = self.map_view(doc);
                if self.search.open {
                    column![
                        container(map).height(Length::FillPortion(3)),
                        container(self.search_view()).height(Length::FillPortion(2))
                    ]
                    .into()
                } else {
                    map
                }
            }
        };
        let main = column![self.menu_bar(), body, self.status_bar()];
        let mut layers: Stack<Message> = stack![main].width(Fill).height(Fill);
        if let Some(i) = self.menu {
            layers = layers.push(self.menu_dropdown(i));
        }
        if let Some(dialog) = self.dialogs.first() {
            layers = layers.push(self.dialog_view(dialog));
        }
        // The pointer stays the normal arrow everywhere.
        super::arrow_cursor::arrow_cursor(layers).into()
    }

    fn welcome(&self) -> Element<'_, Message> {
        center(
            column![
                text("Mind PKM").size(28),
                text("Open a Freeplane mind map (.mm), or start a new one.").color(MUTED),
                row![
                    button(text("Open...").size(12)).on_press(Message::Command(Command::Open)),
                    button(text("New").size(12)).on_press(Message::Command(Command::New)),
                ]
                .spacing(8),
            ]
            .spacing(14)
            .align_x(Alignment::Center),
        )
        .into()
    }

    fn menu_bar(&self) -> Element<'_, Message> {
        let mut bar = row![].spacing(0);
        for (i, (label, _)) in MENUS.iter().enumerate() {
            let open = self.menu == Some(i);
            bar = bar.push(
                button(text(*label).size(12))
                    .width(MENU_BUTTON_WIDTH)
                    .padding([4, 10])
                    .style(move |theme: &Theme, status| {
                        let mut s = button::text(theme, status);
                        if open || matches!(status, button::Status::Hovered) {
                            s.background = Some(Background::Color(MENU_OPEN_BG));
                        }
                        s
                    })
                    .on_press(Message::Menu(if open { None } else { Some(i) })),
            );
        }
        container(bar)
            .width(Fill)
            .style(|_| container::Style {
                background: Some(Background::Color(BAR_BG)),
                border: Border { color: BAR_BORDER, width: 1.0, radius: 0.0.into() },
                ..Default::default()
            })
            .into()
    }

    fn menu_dropdown(&self, index: usize) -> Element<'_, Message> {
        let (_, items) = MENUS[index];
        let mut list = Column::new().spacing(0).width(340);
        for (label, shortcut, cmd) in items.iter() {
            list = list.push(
                button(
                    row![text(*label).size(12).width(Fill), text(*shortcut).size(11).color(MUTED)]
                        .align_y(Alignment::Center),
                )
                .width(Fill)
                .padding([5, 12])
                .style(button::text)
                .style(|theme: &Theme, status| {
                    let mut s = button::text(theme, status);
                    s.text_color = Color::BLACK;
                    if matches!(status, button::Status::Hovered) {
                        s.background = Some(Background::Color(SELECTED_BG));
                    }
                    s
                })
                .on_press(Message::Command(*cmd)),
            );
        }
        let dropdown = container(list).padding(4).style(|_| container::Style {
            background: Some(Background::Color(Color::WHITE)),
            border: Border { color: Color::from_rgb(0.75, 0.77, 0.80), width: 1.0, radius: 4.0.into() },
            shadow: iced::Shadow {
                color: Color::from_rgba(0.0, 0.0, 0.0, 0.2),
                offset: iced::Vector::new(0.0, 2.0),
                blur_radius: 8.0,
            },
            ..Default::default()
        });
        stack![
            mouse_area(container(space()).width(Fill).height(Fill)).on_press(Message::Menu(None)),
            pin(dropdown).position(Point::new(index as f32 * MENU_BUTTON_WIDTH, 30.0)),
        ]
        .into()
    }

    fn status_bar(&self) -> Element<'_, Message> {
        let (filter, path) = match &self.doc {
            Some(doc) => (
                format!("Filter: {}", doc.filter.label()),
                if doc.map.exists(doc.focus) { pkm::node_path_text(&doc.map, doc.focus) } else { String::new() },
            ),
            None => (String::new(), String::new()),
        };
        container(
            row![
                text(path).size(11).color(MUTED).width(Fill).wrapping(Wrapping::None),
                text(self.status.clone()).size(11).color(MUTED),
                text(filter).size(11).color(MUTED),
                text(format!("Zoom: {:.0}%", self.zoom * 100.0)).size(11).color(MUTED),
            ]
            .spacing(16),
        )
        .padding([3, 8])
        .width(Fill)
        .clip(true)
        .style(|_| container::Style { background: Some(Background::Color(BAR_BG)), ..Default::default() })
        .into()
    }

    // --- The map ---

    fn map_view<'a>(&'a self, doc: &'a Doc) -> Element<'a, Message> {
        let size = doc.layout.size;
        let mut layers: Stack<Message> = Stack::new().width(size.width).height(size.height);
        layers = layers.push(
            canvas_widget(EdgeLayer { layout: &doc.layout, cache: &self.edges, zoom: self.zoom })
                .width(size.width)
                .height(size.height),
        );
        // The branch point handles go under the nodes: a wide node next to a
        // narrow parent keeps its own clicks where it reaches into a handle.
        let root = doc.map.root();
        for b in doc.layout.boxes.iter().filter(|b| b.id != root) {
            if let Some(point) = branch_point(&doc.layout, b) {
                layers = layers.push(branch_handle(b.id, point, self.zoom));
            }
        }
        for b in &doc.layout.boxes {
            layers = layers.push(pin(self.node_view(doc, b)).position(b.rect.position()));
            if b.folded {
                layers = layers.push(fold_marker(b, doc.map.children(b.id).len(), self.zoom));
            }
        }
        if let Some(target) = self.drop_indicator()
            && let Some(b) = doc.layout.get(target.node)
        {
            layers = layers.push(drop_indicator(b.rect, target.place));
        }
        // While a node drag is armed, pointer moves are reported in map
        // coordinates. A release here ends it even if it comes before the
        // drag subscription is in place (a quick click).
        let mut layers = mouse_area(layers).on_release(Message::NodeDragEnd);
        if self.drag.is_some() {
            layers = layers.on_move(Message::NodeDragMove);
        }
        let map = scrollable(layers)
            .id(MAP_SCROLL_ID)
            .direction(scrollable::Direction::Both {
                vertical: scrollable::Scrollbar::default(),
                horizontal: scrollable::Scrollbar::default(),
            })
            .on_scroll(Message::MapScrolled)
            .width(Fill)
            .height(Fill);
        // Nodes, markers and scrollbars capture their own presses, so only a
        // press on empty map space reaches this area and starts a pan.
        let map = mouse_area(map).on_press(Message::PanStart);
        // Ctrl+wheel zooms instead of scrolling.
        container(super::ctrl_wheel::ctrl_wheel(map, Message::ZoomWheel))
            .style(|_| container::Style { background: Some(Background::Color(MAP_BG)), ..Default::default() })
            .width(Fill)
            .height(Fill)
            .into()
    }

    fn node_view<'a>(&'a self, doc: &'a Doc, b: &NodeBox) -> Element<'a, Message> {
        let id = b.id;
        let map = &doc.map;
        let label = map.text(id);
        let zoom = self.zoom;
        let tsize = self.measure_cache.get(&label).copied().unwrap_or(Size::new(NODE_MAX_TEXT_WIDTH, 20.0) * zoom);
        let (font_size, icon_width) = (NODE_FONT_SIZE * zoom, ICON_WIDTH * zoom);
        let mut content = row![].spacing(ROW_SPACING * zoom).align_y(Alignment::Center);
        for icon in map.icons(id) {
            let (glyph, color) = icon_glyph(&icon);
            content = content.push(
                text(glyph).size(font_size).line_height(NODE_LINE_HEIGHT).color(color).width(icon_width).center(),
            );
        }
        content = content.push(
            text(label)
                .size(font_size)
                .line_height(NODE_LINE_HEIGHT)
                .font(APP_FONT)
                .shaping(Shaping::Advanced)
                .wrapping(Wrapping::WordOrGlyph)
                .width(tsize.width)
                .color(if map.has_icon(id, ICON_CHECKMARK) { MUTED } else { Color::BLACK }),
        );
        if map.link(id).is_some() {
            let glyph = if map.link_node(id).is_some() { "→" } else { "↗" };
            content = content.push(super::arrow_cursor::hand_cursor(
                button(text(glyph).size(LINK_FONT_SIZE * zoom).center())
                    .width(icon_width)
                    .padding(0)
                    .style(|_, status| button::Style {
                        text_color: if matches!(status, button::Status::Hovered) {
                            SELECTED_BORDER
                        } else {
                            LINK_COLOR
                        },
                        ..Default::default()
                    })
                    .on_press(Message::LinkClicked(id)),
            ));
        }
        let selected = doc.selection.contains(&id);
        let focused = doc.focus == id;
        let boxed = container(content)
            .padding(NODE_PADDING.map(|p| p * zoom))
            .width(b.rect.width)
            .height(b.rect.height)
            .align_y(Alignment::Center)
            .style(move |_| {
                let (bg, border) = if selected {
                    (
                        Some(SELECTED_BG),
                        Border {
                            color: SELECTED_BORDER,
                            width: if focused { 2.0 } else { 1.0 },
                            radius: (NODE_RADIUS * zoom).into(),
                        },
                    )
                } else {
                    (None, Border { color: Color::TRANSPARENT, width: 1.0, radius: (NODE_RADIUS * zoom).into() })
                };
                container::Style { background: bg.map(Background::Color), border, ..Default::default() }
            });
        mouse_area(boxed).on_press(Message::NodeClicked(id)).on_double_click(Message::NodeDoubleClicked(id)).into()
    }

    // --- Search panel ---

    fn search_view(&self) -> Element<'_, Message> {
        let s = &self.search;
        let search = |m: SearchMsg| Message::Search(m);
        let controls = row![
            text_input("Keyword", &s.query)
                .id(SEARCH_INPUT_ID)
                .on_input(|q| Message::Search(SearchMsg::QueryChanged(q)))
                .on_submit(search(SearchMsg::Submit))
                .size(12)
                .padding(4)
                .width(Fill),
            pick_list(ScopeOption::ALL, Some(s.scope), |v| Message::Search(SearchMsg::Scope(v))).text_size(12),
            pick_list(SortOption::ALL, Some(s.sort), |v| Message::Search(SearchMsg::Sort(v))).text_size(12),
            icon_button(
                PanelIcon::Search,
                "Search",
                button::primary,
                (!s.searching).then(|| search(SearchMsg::Submit))
            ),
            icon_button(PanelIcon::Close, "Close", button::secondary, Some(search(SearchMsg::Close))),
        ]
        .spacing(6)
        .align_y(Alignment::Center);

        let header = row![
            text("File").size(11).color(MUTED).width(FILE_COLUMN),
            text("Modified").size(11).color(MUTED).width(DATE_COLUMN),
            text("Snippet").size(11).color(MUTED),
        ]
        .spacing(8)
        .padding([0, 6]);

        let mut rows = Column::new();
        for (i, hit) in s.hits.iter().enumerate() {
            rows = rows.push(result_row(i, hit, s.selected == Some(i)));
        }
        let selected = s.selected.is_some();
        let has_node = s.selected.and_then(|i| s.nodes.get(i).copied().flatten()).is_some();
        let footer = row![
            text(s.status.clone()).size(12).width(Fill),
            button(text("Open File").size(12)).on_press_maybe(selected.then(|| search(SearchMsg::OpenFile))),
            button(text("Select Node").size(12)).on_press_maybe(has_node.then(|| search(SearchMsg::SelectNode))),
        ]
        .spacing(6)
        .align_y(Alignment::Center);

        container(
            column![
                controls,
                header,
                container(scrollable(rows).id(RESULTS_SCROLL_ID).width(Fill).height(Fill))
                    .style(|_| container::Style {
                        background: Some(Background::Color(Color::WHITE)),
                        border: Border { color: PANEL_BORDER, width: 1.0, radius: 2.0.into() },
                        ..Default::default()
                    })
                    .height(Fill),
                footer,
            ]
            .spacing(6),
        )
        .padding(8)
        .style(|_| container::Style {
            background: Some(Background::Color(Color::from_rgb(0.97, 0.97, 0.98))),
            border: Border { color: PANEL_BORDER, width: 1.0, radius: 0.0.into() },
            ..Default::default()
        })
        .into()
    }

    // --- Dialogs ---

    fn dialog_view(&self, dialog: &Dialog) -> Element<'_, Message> {
        let ok = |label: &'static str| button(text(label).size(13).center()).width(90).on_press(Message::DialogOk);
        let cancel = |label: &'static str| {
            button(text(label).size(13).center()).width(90).style(button::secondary).on_press(Message::DialogCancel)
        };
        let body: Element<Message> = match dialog {
            Dialog::Message(msg) => {
                column![text(msg.clone()).size(13), row![space().width(Fill), ok("OK")]].spacing(16).into()
            }
            Dialog::Confirm { text: question, .. } => {
                column![text(question.clone()).size(13), row![space().width(Fill), ok("Yes"), cancel("No")].spacing(8)]
                    .spacing(16)
                    .into()
            }
            Dialog::Input { prompt, value, .. } => column![
                text(prompt.clone()).size(13).font(if prompt.contains('\n') { Font::MONOSPACE } else { APP_FONT }),
                text_input("", value)
                    .id(DIALOG_INPUT_ID)
                    .on_input(Message::DialogInput)
                    .on_submit(Message::DialogOk)
                    .size(13)
                    .padding(6),
                row![space().width(Fill), ok("OK"), cancel("Cancel")].spacing(8)
            ]
            .spacing(12)
            .into(),
            Dialog::Unsaved(_) => column![
                text("The map has unsaved changes. Save them?").size(13),
                row![
                    space().width(Fill),
                    button(text("Save").size(13).center()).width(90).on_press(Message::DialogSave),
                    button(text("Don't Save").size(13).center())
                        .width(100)
                        .style(button::secondary)
                        .on_press(Message::DialogDiscard),
                    cancel("Cancel"),
                ]
                .spacing(8)
            ]
            .spacing(16)
            .into(),
        };
        let card = container(body).padding(20).max_width(560).style(|_| container::Style {
            background: Some(Background::Color(Color::WHITE)),
            border: Border { color: Color::from_rgb(0.7, 0.72, 0.75), width: 1.0, radius: 8.0.into() },
            shadow: iced::Shadow {
                color: Color::from_rgba(0.0, 0.0, 0.0, 0.3),
                offset: iced::Vector::new(0.0, 4.0),
                blur_radius: 16.0,
            },
            text_color: Some(Color::BLACK),
            ..Default::default()
        });
        opaque(center(card).style(|_| container::Style {
            background: Some(Background::Color(Color::from_rgba(0.0, 0.0, 0.0, 0.25))),
            ..Default::default()
        }))
    }
}

const FILE_COLUMN: f32 = 260.0;
const DATE_COLUMN: f32 = 110.0;

fn result_row(i: usize, hit: &SearchHit, selected: bool) -> Element<'_, Message> {
    let modified = chrono::DateTime::from_timestamp_millis(hit.last_modified)
        .map(|d| d.with_timezone(&chrono::Local).format("%y/%m/%d %H:%M").to_string())
        .unwrap_or_default();
    let spans = highlighted_spans(&search::single_line(&hit.snippet));
    let cells = row![
        container(text(hit.relative_path.clone()).size(12).wrapping(Wrapping::None)).width(FILE_COLUMN).clip(true),
        container(text(modified).size(12).wrapping(Wrapping::None)).width(DATE_COLUMN).clip(true),
        container(rich_text(spans).size(12).wrapping(Wrapping::None)).width(Fill).clip(true),
    ]
    .spacing(8)
    .align_y(Alignment::Center);
    mouse_area(container(cells).padding([0, 6]).height(ROW_HEIGHT).width(Fill).align_y(Alignment::Center).style(
        move |_| container::Style {
            background: selected.then_some(Background::Color(SELECTED_BG)),
            ..Default::default()
        },
    ))
    .on_press(Message::Search(SearchMsg::RowClicked(i)))
    .on_double_click(Message::Search(SearchMsg::RowDoubleClicked(i)))
    .into()
}

/// The snippet's text with the matched terms highlighted.
fn highlighted_spans(snippet: &Snippet) -> Vec<Span<'static, (), Font>> {
    let mut spans = Vec::new();
    let mut pos = 0;
    for r in &snippet.highlights {
        if r.start > pos {
            spans.push(Span::new(snippet.text[pos..r.start].to_string()));
        }
        spans.push(
            Span::new(snippet.text[r.clone()].to_string())
                .background(Background::Color(HIGHLIGHT_BG))
                .color(Color::BLACK),
        );
        pos = r.end;
    }
    if pos < snippet.text.len() {
        spans.push(Span::new(snippet.text[pos..].to_string()));
    }
    spans
}

/// The child count shown right of a folded node; clicking it unfolds the node.
fn fold_marker<'a>(b: &NodeBox, count: usize, zoom: f32) -> Element<'a, Message> {
    let size = FOLD_MARKER_SIZE * zoom;
    let marker = button(text(count.to_string()).size(FOLD_MARKER_FONT_SIZE * zoom).center())
        .width(size)
        .height(size)
        .padding(0)
        .style(move |_, _| button::Style {
            background: Some(Background::Color(Color::WHITE)),
            text_color: MUTED,
            border: Border { color: EDGE_COLOR, width: 1.0, radius: (size / 2.0).into() },
            ..Default::default()
        })
        .on_press(Message::FoldClicked(b.id));
    pin(super::arrow_cursor::hand_cursor(marker))
        .position(Point::new(b.rect.x + b.rect.width + 2.0 * zoom, b.rect.center_y() - size / 2.0))
        .into()
}

/// Where the edges from a node to its shown children branch: halfway to the
/// children, at the height of the node's underline. None without children.
fn branch_point(layout: &MapLayout, b: &NodeBox) -> Option<Point> {
    let first = layout.get(*layout.visible_children(b.id).first()?)?;
    let start = b.rect.x + b.rect.width;
    Some(Point::new(branch_x(start, first.rect.x), b.rect.y + b.rect.height))
}

fn branch_x(start: f32, child: f32) -> f32 {
    start + (child - start) * EDGE_BRANCH
}

/// The clickable branch point of an unfolded node: invisible until hovered,
/// then a circle like the fold marker; clicking it folds the node.
fn branch_handle<'a>(id: NodeId, point: Point, zoom: f32) -> Element<'a, Message> {
    let size = FOLD_MARKER_SIZE * zoom;
    let handle = button(text("−").size(FOLD_MARKER_FONT_SIZE * zoom).center())
        .width(size)
        .height(size)
        .padding(0)
        .style(move |_, status| match status {
            button::Status::Hovered | button::Status::Pressed => button::Style {
                background: Some(Background::Color(Color::WHITE)),
                text_color: MUTED,
                border: Border { color: EDGE_COLOR, width: 1.0, radius: (size / 2.0).into() },
                ..Default::default()
            },
            _ => button::Style { text_color: Color::TRANSPARENT, ..Default::default() },
        })
        .on_press(Message::FoldClicked(id));
    pin(super::arrow_cursor::hand_cursor(handle))
        .position(Point::new(point.x - size / 2.0, point.y - size / 2.0))
        .into()
}

/// Where dragged nodes would be dropped: an outline around the target for a
/// child, a line along its top or bottom edge for a sibling.
fn drop_indicator<'a>(r: Rectangle, place: DropPlace) -> Element<'a, Message> {
    const LINE: f32 = 3.0;
    let line = || {
        container(space())
            .width(r.width)
            .height(LINE)
            .style(|_| container::Style { background: Some(Background::Color(SELECTED_BORDER)), ..Default::default() })
    };
    let (marker, position): (Element<'a, Message>, Point) = match place {
        DropPlace::Child => (
            container(space())
                .width(r.width)
                .height(r.height)
                .style(|_| container::Style {
                    border: Border { color: SELECTED_BORDER, width: 2.0, radius: 5.0.into() },
                    ..Default::default()
                })
                .into(),
            r.position(),
        ),
        DropPlace::Before => (line().into(), Point::new(r.x, r.y - LINE / 2.0)),
        DropPlace::After => (line().into(), Point::new(r.x, r.y + r.height - LINE / 2.0)),
    };
    pin(marker).position(position).into()
}

/// How a Freeplane built-in icon is shown.
fn icon_glyph(icon: &str) -> (&'static str, Color) {
    match icon {
        "button_ok" => ("✔", CHECK_COLOR),
        "button_cancel" => ("✘", RED),
        "help" => ("?", LINK_COLOR),
        "messagebox_warning" | "yes" => ("!", Color::from_rgb(0.85, 0.5, 0.0)),
        "idea" => ("☀", Color::from_rgb(0.85, 0.65, 0.0)),
        "flag" | "flag-red" => ("⚑", RED),
        "full-1" => ("1", MUTED),
        "full-2" => ("2", MUTED),
        "full-3" => ("3", MUTED),
        "full-4" => ("4", MUTED),
        "full-5" => ("5", MUTED),
        _ => ("●", MUTED),
    }
}

/// The edges from a parent's underline end `start` to its children's
/// underline starts `ends` (top to bottom): horizontally to a branch point,
/// vertically to each child's height and horizontally into the child, with
/// rounded corners. The edges share the horizontal part and the vertical one;
/// every shared part is drawn once, so it looks like a single line however
/// many children there are (overlapping strokes would darken it).
fn edges(builder: &mut canvas::path::Builder, start: Point, ends: &[Point], zoom: f32) {
    let Some(first) = ends.first() else { return };
    let x = branch_x(start.x, first.x);
    // A corner takes at most half the height, so that a child nearly level
    // with its parent gets a gentle S instead of a loop.
    let radius = |dy: f32| (EDGE_RADIUS * zoom).min(dy.abs() / 2.0).min(x - start.x).max(0.0);
    let sign = |y: f32| if y < start.y { -1.0 } else { 1.0 };
    // The farthest child above and below the parent; their edges make up the
    // vertical line.
    let up = (first.y < start.y).then_some(0);
    let down = ends.iter().rposition(|e| e.y >= start.y);
    let side_radius = |i: Option<usize>| i.map_or(0.0, |i| radius(ends[i].y - start.y));
    let (r_up, r_down) = (side_radius(up), side_radius(down));
    let trunk_end = Point::new(x - r_up.max(r_down), start.y);
    builder.move_to(start);
    builder.line_to(trunk_end);
    for (i, e) in ends.iter().enumerate() {
        let s = sign(e.y);
        let side_r = if e.y < start.y { r_up } else { r_down };
        if Some(i) == up || Some(i) == down {
            let r = side_r;
            builder.move_to(trunk_end);
            builder.line_to(Point::new(x - r, start.y));
            builder.quadratic_curve_to(Point::new(x, start.y), Point::new(x, start.y + s * r));
            builder.line_to(Point::new(x, e.y - s * r));
            builder.quadratic_curve_to(Point::new(x, e.y), Point::new(x + r, e.y));
        } else if (e.y - start.y).abs() > side_r {
            // Branch off the vertical line below the corner it leaves the
            // trunk with.
            let r = radius(e.y - start.y).min((e.y - start.y).abs() - side_r);
            builder.move_to(Point::new(x, e.y - s * r));
            builder.quadratic_curve_to(Point::new(x, e.y), Point::new(x + r, e.y));
        } else {
            // Nearly level with the parent, between children above and below.
            builder.move_to(trunk_end);
            builder.bezier_curve_to(Point::new(x, start.y), Point::new(x, e.y), Point::new(x + r_up.max(r_down), e.y));
        }
        builder.line_to(*e);
    }
}

/// A search panel button showing a drawn icon (no font has to carry the
/// glyph), with its name as a tooltip.
fn icon_button<'a>(
    icon: PanelIcon,
    name: &'a str,
    style: fn(&Theme, button::Status) -> button::Style,
    on_press: Option<Message>,
) -> Element<'a, Message> {
    let button = button(canvas_widget(icon).width(ICON_SIZE).height(ICON_SIZE))
        .padding([5, 10])
        .style(style)
        .on_press_maybe(on_press);
    tooltip(
        button,
        container(text(name).size(12)).padding([2, 6]).style(container::bordered_box),
        tooltip::Position::Bottom,
    )
    .into()
}

const ICON_SIZE: f32 = 14.0;

#[derive(Debug, Clone, Copy)]
enum PanelIcon {
    /// A magnifying glass.
    Search,
    /// A cross.
    Close,
}

impl canvas::Program<Message> for PanelIcon {
    type State = ();

    fn draw(
        &self,
        _state: &(),
        renderer: &Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<canvas::Geometry> {
        let mut frame = canvas::Frame::new(renderer, bounds.size());
        let s = bounds.width.min(bounds.height);
        let path = match self {
            PanelIcon::Search => Path::new(|b| {
                b.circle(Point::new(s * 0.42, s * 0.42), s * 0.3);
                b.move_to(Point::new(s * 0.64, s * 0.64));
                b.line_to(Point::new(s * 0.92, s * 0.92));
            }),
            PanelIcon::Close => Path::new(|b| {
                b.move_to(Point::new(s * 0.18, s * 0.18));
                b.line_to(Point::new(s * 0.82, s * 0.82));
                b.move_to(Point::new(s * 0.82, s * 0.18));
                b.line_to(Point::new(s * 0.18, s * 0.82));
            }),
        };
        frame.stroke(&path, Stroke::default().with_width(1.8).with_color(Color::WHITE));
        vec![frame.into_geometry()]
    }
}

/// The edges between the nodes, drawn under them.
struct EdgeLayer<'a> {
    layout: &'a MapLayout,
    cache: &'a Cache,
    zoom: f32,
}

impl canvas::Program<Message> for EdgeLayer<'_> {
    type State = ();

    fn draw(
        &self,
        _state: &(),
        renderer: &Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<canvas::Geometry> {
        let geometry = self.cache.draw(renderer, bounds.size(), |frame| {
            let stroke = Stroke::default().with_width((EDGE_WIDTH * self.zoom).max(1.0)).with_color(EDGE_COLOR);
            let path = Path::new(|builder| {
                for b in &self.layout.boxes {
                    let c = b.rect;
                    let bottom = c.y + c.height;
                    // Every node is underlined; the edge from its parent ends
                    // at the underline's left end.
                    builder.move_to(Point::new(c.x, bottom));
                    builder.line_to(Point::new(c.x + c.width, bottom));
                    let ends: Vec<Point> = self
                        .layout
                        .visible_children(b.id)
                        .iter()
                        .filter_map(|&id| self.layout.get(id))
                        .map(|child| Point::new(child.rect.x, child.rect.y + child.rect.height))
                        .collect();
                    edges(builder, Point::new(c.x + c.width, bottom), &ends, self.zoom);
                }
            });
            frame.stroke(&path, stroke);
        });
        vec![geometry]
    }
}
