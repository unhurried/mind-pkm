//! The assets pane: the files and folders of the open file's `<name>.assets/`
//! folder beside the page, opened, renamed and deleted from there, and
//! dragged onto the page as links. The folder is read in the background
//! (the application starts the reads it asks for); renaming and deleting go
//! through the application, which knows the document directory and the
//! other editor windows.

use super::view::{HitInfo, Palette, caret_in, hit_test, piece_key};
use super::{Editor, EditorDialog, EditorMsg, EditorPrefs, Effect};
use crate::markdown::edit::{self, Sel};
use crate::markdown::images;
use crate::paths;
use crate::pkm::assets::{self, Entry};
use iced::widget::text::Wrapping;
use iced::widget::{Column, button, column, container, mouse_area, pin, row, scrollable, space, stack, text};
use iced::{Alignment, Background, Element, Fill, Point, Task, Theme, border};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// The pane's width.
pub const ASSETS_WIDTH: f32 = 240.0;
/// How far the pointer moves, with the button held on an item, before it drags.
const DRAG_DISTANCE: f32 = 4.0;
const ROW_INDENT: f32 = 14.0;
/// How far right of the nearest caret the pointer is beside the text, not in it.
const BESIDE_TEXT: f32 = 24.0;

/// The pane of a window showing it.
#[derive(Debug, Default)]
pub struct AssetsPane {
    /// The folder as last read and what it holds (None: there is no such
    /// folder); None before the first read.
    read: Option<(PathBuf, Option<Vec<Entry>>)>,
    /// The folder is to be read again.
    stale: bool,
    /// A read is under way.
    reading: bool,
    /// The expanded folders, by `paths::key`.
    expanded: HashSet<String>,
    selected: Option<PathBuf>,
    pub(super) menu: Option<AssetMenu>,
    pub(super) drag: Option<AssetDrag>,
}

/// An item's right-click menu.
#[derive(Debug, Clone, PartialEq)]
pub struct AssetMenu {
    pub path: PathBuf,
    /// Where it opens in the window.
    pub at: Point,
    /// Rename and Delete are offered.
    pub can_change: bool,
}

/// An item pressed, dragged once the pointer has moved far enough.
#[derive(Debug, Clone, PartialEq)]
pub struct AssetDrag {
    pub path: PathBuf,
    from: Point,
    pub dragging: bool,
    pub target: Option<DropTarget>,
    /// The text under the pointer, until the block around it resolves the target.
    text_hit: Option<DropTarget>,
    /// The target was set during the current pointer move.
    fresh: bool,
}

/// Where a dragged item goes.
#[derive(Debug, Clone, PartialEq)]
pub enum DropTarget {
    /// Into text at a source offset.
    Text { at: usize, mark: DropMark },
    /// A paragraph of its own after the block whose content ends at `at`.
    Paragraph { at: usize },
    /// At the cursor of the source mode's editor.
    Cursor,
}

/// Where a text target is marked: in a piece of rendered text (`view::piece_key`), at a point in it.
#[derive(Debug, Clone, PartialEq)]
pub struct DropMark(pub (usize, usize), pub Point);

#[derive(Debug, Clone)]
pub enum AssetsMsg {
    /// The folder read: what it holds, None when there is no such folder.
    Read(PathBuf, Option<Vec<Entry>>),
    Refresh,
    /// An arrow of a folder pressed.
    Toggle(PathBuf),
    Pressed(PathBuf),
    DoubleClicked(PathBuf),
    RightClicked(PathBuf),
    CloseMenu,
    Rename,
    Delete,
    /// The name typed in the rename dialog.
    NewName(String),
    // The pointer, while an item is dragged (the page all rendered):
    /// Over a rendered block (after the pieces of text in it reported).
    BlockHovered(usize),
    /// Over the space between blocks (or below the last one): a paragraph after the block ending at `at`.
    GapHovered(usize),
    /// Anywhere over the page (after everything in it reported).
    PageHovered,
    LeftPage,
}

/// Where a block's content ends: before the line breaks closing its range.
pub fn content_end(text: &str, range: std::ops::Range<usize>) -> usize {
    let bytes = text.as_bytes();
    let mut end = range.end.min(text.len());
    while end > range.start && matches!(bytes[end - 1], b'\n' | b'\r') {
        end -= 1;
    }
    end
}

impl AssetsPane {
    fn new() -> AssetsPane {
        AssetsPane { stale: true, ..AssetsPane::default() }
    }

    fn is_expanded(&self, path: &Path) -> bool {
        self.expanded.contains(&paths::key(path))
    }

    fn entry(&self, path: &Path) -> Option<&Entry> {
        fn find<'e>(entries: &'e [Entry], key: &str) -> Option<&'e Entry> {
            entries.iter().find_map(|e| {
                if paths::key(&e.path) == key { Some(e) } else { find(e.children.as_deref().unwrap_or_default(), key) }
            })
        }
        let (_, Some(entries)) = self.read.as_ref()? else { return None };
        find(entries, &paths::key(path))
    }

    /// The paths of the pane at or under `old` now under `new`.
    fn moved(&mut self, old: &Path, new: &Path) {
        let moved = |p: &Path| -> Option<PathBuf> { p.strip_prefix(old).ok().map(|rest| new.join(rest)) };
        if let Some(path) = self.selected.as_deref().and_then(moved) {
            self.selected = Some(path);
        }
        let old_key = paths::key(old);
        let renamed: Vec<String> = self
            .expanded
            .iter()
            .filter(|k| *k == &old_key || k.starts_with(&format!("{old_key}{}", std::path::MAIN_SEPARATOR)))
            .cloned()
            .collect();
        for k in renamed {
            self.expanded.remove(&k);
            self.expanded.insert(format!("{}{}", paths::key(new), &k[old_key.len()..]));
        }
    }
}

impl Editor {
    /// The folder the pane shows.
    pub fn assets_dir(&self) -> PathBuf {
        assets::assets_dir_of(self.page.path())
    }

    /// The folder to read now, if the pane wants it read: it was asked for,
    /// or the file moved (its folder is another). Marks the read as started.
    pub fn start_assets_read(&mut self) -> Option<PathBuf> {
        let dir = self.assets_dir();
        let pane = self.assets.as_mut()?;
        let moved = pane.read.as_ref().is_none_or(|(d, _)| !paths::same_file(d, &dir));
        if pane.reading || !(pane.stale || moved) {
            return None;
        }
        (pane.stale, pane.reading) = (false, true);
        Some(dir)
    }

    /// The pane is to read its folder again (when shown).
    pub fn assets_stale(&mut self) {
        if let Some(pane) = &mut self.assets {
            pane.stale = true;
        }
    }

    pub(super) fn toggle_assets(&mut self) {
        self.assets = match self.assets.take() {
            Some(_) => None,
            None => Some(AssetsPane::new()),
        };
    }

    /// Whether an item is dragged (not just pressed) from the pane.
    pub(super) fn asset_dragging(&self) -> bool {
        self.assets.as_ref().and_then(|p| p.drag.as_ref()).is_some_and(|d| d.dragging)
    }

    /// Whether an item of the pane is pressed (the release ends it, wherever it is).
    pub(super) fn asset_pressed(&self) -> bool {
        self.assets.as_ref().is_some_and(|p| p.drag.is_some())
    }

    pub(super) fn drop_target(&self) -> Option<&DropTarget> {
        self.assets.as_ref()?.drag.as_ref().filter(|d| d.dragging)?.target.as_ref()
    }

    fn drag_mut(&mut self) -> Option<&mut AssetDrag> {
        self.assets.as_mut()?.drag.as_mut().filter(|d| d.dragging)
    }

    pub(super) fn assets_message(&mut self, msg: AssetsMsg, effects: &mut Vec<Effect>) -> Task<EditorMsg> {
        let dir = self.assets_dir();
        let pointer = self.pointer_at;
        let source_mode = self.source.is_some();
        let Some(pane) = &mut self.assets else { return Task::none() };
        match msg {
            AssetsMsg::Read(read, tree) => {
                pane.reading = false;
                // A read of the folder the file had before it moved: read again.
                if paths::same_file(&read, &dir) {
                    let folders: HashSet<String> = tree.iter().flatten().flat_map(folder_keys).collect();
                    pane.expanded.retain(|k| folders.contains(k));
                    pane.read = Some((read, tree));
                }
            }
            AssetsMsg::Refresh => pane.stale = true,
            AssetsMsg::Toggle(path) => toggle(pane, &path),
            AssetsMsg::Pressed(path) => {
                self.menu = None;
                pane.menu = None;
                pane.selected = Some(path.clone());
                pane.drag = Some(AssetDrag {
                    path,
                    from: pointer,
                    dragging: false,
                    target: None,
                    text_hit: None,
                    fresh: false,
                });
            }
            AssetsMsg::DoubleClicked(path) => {
                pane.drag = None;
                match pane.entry(&path) {
                    Some(e) if e.is_dir() => toggle(pane, &path),
                    _ if path.extension().is_some_and(|e| e.eq_ignore_ascii_case("md")) => {
                        effects.push(Effect::OpenPage(path))
                    }
                    _ => effects.push(Effect::OpenExternal(path.display().to_string())),
                }
            }
            AssetsMsg::RightClicked(path) => {
                self.menu = None;
                pane.selected = Some(path.clone());
                // The application says whether it may be changed (`show_asset_menu`).
                effects.push(Effect::AssetMenu(path));
            }
            AssetsMsg::CloseMenu => pane.menu = None,
            AssetsMsg::Rename => {
                if let Some(menu) = pane.menu.take().filter(|m| m.can_change) {
                    let name = paths::file_name(&menu.path);
                    self.dialogs.push_back(EditorDialog::RenameAsset { path: menu.path, name });
                    return iced::widget::operation::focus(self.id("rename"));
                }
            }
            AssetsMsg::Delete => {
                if let Some(menu) = pane.menu.take().filter(|m| m.can_change) {
                    self.dialogs.push_back(EditorDialog::DeleteAsset(menu.path));
                }
            }
            AssetsMsg::NewName(new) => {
                if let Some(EditorDialog::RenameAsset { name, .. }) = self.dialogs.front_mut() {
                    *name = new;
                }
            }
            AssetsMsg::BlockHovered(block) => self.block_hovered(block),
            AssetsMsg::GapHovered(at) => {
                if let Some(drag) = self.drag_mut() {
                    (drag.target, drag.fresh, drag.text_hit) = (Some(DropTarget::Paragraph { at }), true, None);
                }
            }
            AssetsMsg::PageHovered => {
                if let Some(drag) = self.drag_mut() {
                    if source_mode {
                        drag.target = Some(DropTarget::Cursor);
                    } else if !drag.fresh {
                        // Beside the page's column: nowhere.
                        drag.target = None;
                    }
                    (drag.fresh, drag.text_hit) = (false, None);
                }
            }
            AssetsMsg::LeftPage => {
                if let Some(drag) = self.drag_mut() {
                    (drag.target, drag.text_hit) = (None, None);
                }
            }
        }
        Task::none()
    }

    /// The application's answer to a right-click on an item.
    pub fn show_asset_menu(&mut self, path: PathBuf, can_change: bool) {
        let at = self.pointer_at;
        if let Some(pane) = &mut self.assets {
            pane.menu = Some(AssetMenu { path, at, can_change });
        }
    }

    /// After the pane's item `old` was renamed to `new`: the page's links to
    /// it point at the new name (one undoable change), the pane follows.
    pub fn asset_renamed(&mut self, old: &Path, new: &Path) {
        let name = paths::file_name(new);
        self.leave_source_for_history();
        let (text, n) = images::rename_destinations(self.page.path(), self.page.text(), old, &name);
        if n > 0 {
            let head = self.page.sel().head.min(text.len());
            self.page.set_text(text, Sel::caret(head));
            self.page.deactivate();
            self.refresh_source();
            self.status.set(format!("renamed {name}, {n} link{} changed", if n == 1 { "" } else { "s" }));
        } else {
            self.status.set(format!("renamed {name}"));
        }
        if let Some(pane) = &mut self.assets {
            pane.moved(old, new);
            pane.stale = true;
        }
    }

    /// After the pane's item at `path` was deleted.
    pub fn asset_deleted(&mut self, path: &Path) {
        if let Some(pane) = &mut self.assets {
            if pane.selected.as_deref().is_some_and(|s| s.starts_with(path)) {
                pane.selected = None;
            }
            pane.stale = true;
        }
    }

    // --- Dragging onto the page ---

    /// The pointer over a piece of rendered text while an item is dragged:
    /// into it, unless it is code or the pointer is well beside its text.
    pub(super) fn drag_over_text(&mut self, info: &HitInfo, p: Point) {
        let code = self.page.doc().blocks.get(info.block).is_some_and(|b| b.kind.is_code());
        let beside = caret_in(info, p).is_none_or(|caret| p.x > caret.x + BESIDE_TEXT);
        let target = (!code && !beside)
            .then(|| hit_test(info, p))
            .flatten()
            .map(|at| DropTarget::Text { at, mark: DropMark(piece_key(info), p) });
        if let Some(drag) = self.drag_mut() {
            drag.text_hit = target;
        }
    }

    fn block_hovered(&mut self, block: usize) {
        let Some(range) = self.page.doc().blocks.get(block).map(|b| b.range.clone()) else { return };
        let end = content_end(self.page.text(), range);
        if let Some(drag) = self.drag_mut() {
            drag.target = Some(drag.text_hit.take().unwrap_or(DropTarget::Paragraph { at: end }));
            drag.fresh = true;
        }
    }

    /// The pointer moved in the window: a press on an item becomes a drag,
    /// and the page shows every block rendered, so that what is dropped goes
    /// between the characters shown (never into an element's Markdown).
    pub(super) fn assets_pointer(&mut self, p: Point) {
        if let Some(drag) = self.assets.as_mut().and_then(|pane| pane.drag.as_mut())
            && !drag.dragging
            && p.distance(drag.from) > DRAG_DISTANCE
        {
            drag.dragging = true;
            if self.source.is_none() {
                self.page.deactivate();
            }
        }
    }

    /// The button released: a dragged item goes where it was dropped.
    pub(super) fn assets_released(&mut self, prefs: &EditorPrefs) -> Task<EditorMsg> {
        let Some(drag) = self.assets.as_mut().and_then(|pane| pane.drag.take()) else { return Task::none() };
        let (true, Some(target)) = (drag.dragging, drag.target) else { return Task::none() };
        let md = images::markdown_for(self.page.path(), &drag.path, &prefs.images);
        match target {
            DropTarget::Cursor => self.insert_text(&md),
            DropTarget::Text { at, .. } => {
                let new = edit::splice(self.page.text(), at..at, &md);
                self.page.set_text(new, Sel::caret(at + md.len()));
            }
            DropTarget::Paragraph { at } => {
                // In a quote or a list item, a paragraph of its own would end
                // it: the link goes at the end of the block's text instead.
                let nested = self
                    .page
                    .doc()
                    .blocks
                    .iter()
                    .find(|b| content_end(self.page.text(), b.range.clone()) == at)
                    .is_some_and(|b| !b.outer.is_empty() || matches!(b.kind, crate::markdown::BlockKind::ListItem(_)));
                let piece = match (at, nested) {
                    (0, _) if self.page.text().trim().is_empty() => md.clone(),
                    (_, true) => format!(" {md}"),
                    _ => format!("\n\n{md}"),
                };
                let new = edit::splice(self.page.text(), at..at, &piece);
                self.page.set_text(new, Sel::caret(at + piece.len()));
            }
        }
        self.focus()
    }

    // --- View ---

    pub(super) fn assets_view<'a>(&'a self, pane: &'a AssetsPane, p: Palette) -> Element<'a, EditorMsg> {
        let assets = |m: AssetsMsg| EditorMsg::Assets(m);
        let header = row![
            text("Assets").size(12).color(p.muted).width(Fill),
            button(text("⟳").size(13).color(p.text))
                .padding([0, 6])
                .style(move |_: &Theme, status| button::Style {
                    background: matches!(status, button::Status::Hovered).then_some(Background::Color(p.hover)),
                    text_color: p.text,
                    ..Default::default()
                })
                .on_press(assets(AssetsMsg::Refresh)),
        ]
        .align_y(Alignment::Center)
        .padding([6, 10]);
        let body: Element<EditorMsg> = match &pane.read {
            Some((_, Some(entries))) => {
                let mut list = Vec::new();
                rows(pane, entries, 0, p, &mut list);
                scrollable(Column::with_children(list).padding([0, 4])).height(Fill).into()
            }
            Some((_, None)) => container(text("No assets folder").size(12).color(p.muted)).padding([6, 10]).into(),
            None => space().into(),
        };
        container(column![header, body])
            .width(ASSETS_WIDTH)
            .height(Fill)
            .style(move |_| container::background(p.bar_bg).border(border::color(p.border).width(1)))
            .into()
    }

    /// An item's right-click menu at its place in the window.
    pub(super) fn asset_menu_view(&self, menu: &AssetMenu, p: Palette) -> Element<'_, EditorMsg> {
        const WIDTH: f32 = 140.0;
        let item = |label: &'static str, msg: AssetsMsg| {
            let color = if menu.can_change { p.text } else { p.muted };
            button(text(label).size(12).color(color))
                .width(Fill)
                .padding([5, 10])
                .style(move |_: &Theme, status| button::Style {
                    background: matches!(status, button::Status::Hovered).then_some(Background::Color(p.hover)),
                    text_color: color,
                    ..Default::default()
                })
                .on_press_maybe(menu.can_change.then_some(EditorMsg::Assets(msg)))
        };
        let list = column![item("Rename", AssetsMsg::Rename), item("Delete", AssetsMsg::Delete)].width(WIDTH);
        let x = menu.at.x.min(self.window_size.width - WIDTH - 12.0).max(0.0);
        let y = menu.at.y.min(self.window_size.height - 70.0).max(0.0);
        let menu = container(list)
            .padding(4)
            .style(move |_| container::background(p.bg).border(border::color(p.muted).width(1).rounded(4)));
        stack![
            mouse_area(container(space()).width(Fill).height(Fill))
                .on_press(EditorMsg::Assets(AssetsMsg::CloseMenu))
                .on_right_press(EditorMsg::Assets(AssetsMsg::CloseMenu)),
            pin(menu).position(Point::new(x, y)),
        ]
        .into()
    }

    /// The name of the item being dragged, beside the pointer.
    pub(super) fn drag_label(&self, drag: &AssetDrag, p: Palette) -> Element<'_, EditorMsg> {
        let name = paths::file_name(&drag.path);
        let label = container(text(name).size(12).color(p.text).wrapping(Wrapping::None))
            .padding([3, 8])
            .style(move |_| container::background(p.bg).border(border::color(p.muted).width(1).rounded(4)));
        // Off the pointer, so that what is under it still sees it.
        pin(label).position(Point::new(self.pointer_at.x + 14.0, self.pointer_at.y + 10.0)).into()
    }

    /// The mark of a text target in a piece of rendered text, if it is there.
    pub(super) fn drop_mark_in(&self, info: &HitInfo) -> Option<iced::Rectangle> {
        match self.drop_target()? {
            DropTarget::Text { mark: DropMark(key, at), .. } if *key == piece_key(info) => caret_in(info, *at),
            _ => None,
        }
    }
}

fn toggle(pane: &mut AssetsPane, path: &Path) {
    let key = paths::key(path);
    if !pane.expanded.remove(&key) {
        pane.expanded.insert(key);
    }
}

fn folder_keys(e: &Entry) -> Vec<String> {
    match &e.children {
        Some(children) => std::iter::once(paths::key(&e.path)).chain(children.iter().flat_map(folder_keys)).collect(),
        None => Vec::new(),
    }
}

/// The rows of `entries` at `depth`, the expanded folders' after theirs.
fn rows<'a>(pane: &AssetsPane, entries: &'a [Entry], depth: usize, p: Palette, list: &mut Vec<Element<'a, EditorMsg>>) {
    for e in entries {
        let expanded = pane.is_expanded(&e.path);
        let arrow: Element<EditorMsg> = if e.is_dir() {
            button(text(if expanded { "▾" } else { "▸" }).size(11).color(p.muted))
                .padding([0, 2])
                .width(16)
                .style(|_: &Theme, _| button::Style::default())
                .on_press(EditorMsg::Assets(AssetsMsg::Toggle(e.path.clone())))
                .into()
        } else {
            space().width(16).into()
        };
        let selected = pane.selected.as_deref().is_some_and(|s| paths::same_file(s, &e.path));
        let name = text(e.name.clone()).size(12).color(p.text).wrapping(Wrapping::None);
        let line = container(row![space().width(ROW_INDENT * depth as f32), arrow, name].align_y(Alignment::Center))
            .padding([2, 4])
            .width(Fill)
            .clip(true)
            .style(move |_| if selected { container::background(p.hover) } else { container::Style::default() });
        list.push(
            mouse_area(line)
                .on_press(EditorMsg::Assets(AssetsMsg::Pressed(e.path.clone())))
                // A click may be over before the window-wide release is listened to.
                .on_release(EditorMsg::Released)
                .on_double_click(EditorMsg::Assets(AssetsMsg::DoubleClicked(e.path.clone())))
                .on_right_press(EditorMsg::Assets(AssetsMsg::RightClicked(e.path.clone())))
                .into(),
        );
        if expanded && let Some(children) = &e.children {
            rows(pane, children, depth + 1, p, list);
        }
    }
}

#[cfg(test)]
impl AssetsPane {
    /// The rows shown, indented by depth, folders with their arrow; None
    /// when there is no folder (or before it was read).
    pub fn shown(&self) -> Option<Vec<String>> {
        fn walk(pane: &AssetsPane, entries: &[Entry], depth: usize, out: &mut Vec<String>) {
            for e in entries {
                let expanded = pane.is_expanded(&e.path);
                let arrow = match (e.is_dir(), expanded) {
                    (false, _) => "",
                    (true, false) => "▸ ",
                    (true, true) => "▾ ",
                };
                out.push(format!("{}{arrow}{}", "  ".repeat(depth), e.name));
                if expanded {
                    walk(pane, e.children.as_deref().unwrap_or_default(), depth + 1, out);
                }
            }
        }
        let (_, Some(entries)) = self.read.as_ref()? else { return None };
        let mut out = Vec::new();
        walk(self, entries, 0, &mut out);
        Some(out)
    }

    pub fn has_no_folder(&self) -> bool {
        matches!(self.read, Some((_, None)))
    }

    pub fn reading(&self) -> bool {
        self.reading
    }

    pub fn selected(&self) -> Option<&Path> {
        self.selected.as_deref()
    }
}
