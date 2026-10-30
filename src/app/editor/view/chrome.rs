//! The window's bars, menus, outline and dialogs.

use super::Ctx;
use super::theme::{OUTLINE_WIDTH, Palette};
use crate::app::editor::find::FindBar;
use crate::app::editor::keys::{EditorCommand, MENUS, TABLE_MENU};
use crate::app::editor::modal::modal;
use crate::app::editor::{AssetsMsg, DialogAnswer, Editor, EditorDialog, EditorMsg, EditorPrefs, FindMsg, TableMsg};
use crate::app::ui::{self};
use iced::advanced::text::Wrapping;
use iced::widget::{
    Column, button, column, container, mouse_area, pin, row, scrollable, space, stack, text, text_input,
};
use iced::{Alignment, Background, Element, Fill, Point, Theme, border};

pub(super) fn dialog_view(dialog: &EditorDialog, p: Palette, input_id: iced::widget::Id) -> Element<'_, EditorMsg> {
    let answer = |label: &'static str, a: DialogAnswer, primary: bool| -> Element<'_, EditorMsg> {
        ui::dialog_button(label, primary, p.dialog(), EditorMsg::Dialog(a))
    };
    let mut input = None;
    let (body, buttons): (String, Vec<Element<EditorMsg>>) = match dialog {
        EditorDialog::Message(m) => (m.clone(), vec![answer("OK", DialogAnswer::Ok, true)]),
        EditorDialog::RenameAsset { path, name } => {
            input = Some(
                iced::widget::text_input("New name", name)
                    .id(input_id)
                    .on_input(|s| EditorMsg::Assets(AssetsMsg::NewName(s)))
                    .on_submit(EditorMsg::Dialog(DialogAnswer::Ok))
                    .size(13)
                    .padding([4, 6]),
            );
            (
                format!("Rename {} to:", crate::paths::file_name(path)),
                vec![answer("Rename", DialogAnswer::Ok, true), answer("Cancel", DialogAnswer::Cancel, false)],
            )
        }
        EditorDialog::DeleteAsset(path) => (
            format!("Delete {}?", crate::paths::file_name(path)),
            vec![answer("Delete", DialogAnswer::Ok, true), answer("Cancel", DialogAnswer::Cancel, false)],
        ),
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
    let mut content = vec![text(body).size(13).color(p.text).into()];
    content.extend(input.map(Element::from));
    // The page's text widget keeps its keyboard focus under the dialog; the
    // dialog takes the keys, so that none of them edits the page.
    modal(ui::dialog(content, buttons, p.dialog()), EditorMsg::Key).into()
}

impl Editor {
    /// A cell's right-click menu at `at` in the window.
    pub(super) fn cell_menu_view(&self, at: Point, p: Palette) -> Element<'_, EditorMsg> {
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

    pub(super) fn menu_bar(&self, p: Palette) -> Element<'_, EditorMsg> {
        ui::menu_bar(MENUS, self.menu, p.menu(), EditorMsg::Menu)
    }

    pub(super) fn menu_dropdown(&self, index: usize, p: Palette, prefs: &EditorPrefs) -> Element<'_, EditorMsg> {
        let checked = |cmd| match cmd {
            EditorCommand::ToggleAutoPair => prefs.auto_pair,
            EditorCommand::DarkTheme => prefs.dark,
            EditorCommand::SourceMode => self.source.is_some(),
            EditorCommand::Outline => self.outline,
            EditorCommand::Assets => self.assets.is_some(),
            EditorCommand::FullScreen => self.fullscreen,
            _ => false,
        };
        ui::menu_dropdown(MENUS, index, p.menu(), checked, EditorMsg::Command, EditorMsg::Menu)
    }

    pub(super) fn find_bar<'a>(&'a self, bar: &'a FindBar, p: Palette) -> Element<'a, EditorMsg> {
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

    pub(super) fn status_bar(&self, p: Palette, prefs: &EditorPrefs) -> Element<'_, EditorMsg> {
        let (all, selected) = self.counts();
        let counts = match selected {
            Some(s) => format!("{} / {} words, {} / {} characters", s.words, all.words, s.characters, all.characters),
            None => format!("{} words, {} characters", all.words, all.characters),
        };
        let mode = if self.source.is_some() { "Source" } else { "" };
        container(
            row![
                container(text(self.status.text().to_string()).size(11).color(p.muted).wrapping(Wrapping::None))
                    .width(Fill)
                    .clip(true),
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

    pub(super) fn outline_view<'a>(&'a self, ctx: &Ctx<'a>) -> Element<'a, EditorMsg> {
        let p = ctx.p;
        let mut list = Column::new().spacing(2).padding(10);
        for (i, h) in self.page.doc().headings.iter().enumerate() {
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
}
