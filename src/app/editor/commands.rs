//! Running the commands of the menus and shortcuts (`EditorCommand`), with
//! the file dialogs and the export some of them start.

use super::keys::EditorCommand;
use super::{Editor, EditorDialog, EditorMsg, EditorPrefs, Effect};
use crate::app::file::file_name;
use crate::app::ui::zoom_step;
use crate::app::{blocking, settings};
use crate::markdown::edit::{self, Sel};
use crate::markdown::format::{self, InlineStyle};
use crate::markdown::{export, html, images};
use iced::{Task, window};
use std::path::{Path, PathBuf};

/// The zoom levels Zoom In and Zoom Out step through.
const ZOOM_STEPS: &[f32] = &[0.7, 0.8, 0.9, 1.0, 1.1, 1.25, 1.5, 1.75, 2.0];

impl Editor {
    pub(super) fn run(
        &mut self,
        cmd: EditorCommand,
        prefs: &EditorPrefs,
        effects: &mut Vec<Effect>,
    ) -> Task<EditorMsg> {
        use EditorCommand as C;
        self.select_all_again &= cmd == C::SelectAll;
        self.table_ui.menu = None;
        if self.source.is_none() && self.table_command(cmd) {
            return self.focus();
        }
        match cmd {
            C::Open => self.ask_file_to_open(),
            C::Save => self.save(),
            C::ExportHtml => self.ask_export_file(false),
            C::ExportPdf => {
                if export::find_browser().is_none() {
                    self.dialogs.push_back(EditorDialog::Message(export::NO_BROWSER.to_string()));
                    return Task::none();
                }
                self.ask_export_file(true)
            }
            C::Close => self.close_requested(effects),
            C::Undo => {
                self.leave_source_for_history();
                if self.page.undo() {
                    self.refresh_source();
                }
                self.focus()
            }
            C::Redo => {
                self.leave_source_for_history();
                if self.page.redo() {
                    self.refresh_source();
                }
                self.focus()
            }
            C::SelectAll => self.select_all(),
            C::SelectWord => self.with_active(|p| {
                if let Some(w) = format::word_at(p.text(), p.sel().head) {
                    p.place(Sel { anchor: w.start, head: w.end });
                }
            }),
            C::SelectLine => self.with_active(|p| {
                let line = edit::line_range(p.text(), p.sel().head);
                p.place(Sel { anchor: line.start, head: line.end });
            }),
            C::SelectSpan => self.with_active(|p| {
                if let Some(r) = format::styled_span_at(p.text(), p.doc(), p.sel().head) {
                    p.place(Sel { anchor: r.start, head: r.end });
                }
            }),
            C::ClearFormatting => {
                self.edit_selection_or_region(|text, doc, range, _| Some(format::clear_formatting(text, doc, range)))
            }
            C::ToggleAutoPair => {
                effects.push(Effect::SetPrefs(EditorPrefs { auto_pair: !prefs.auto_pair, ..prefs.clone() }));
                Task::none()
            }
            C::Heading(n) => self.edit_region(|text, doc, region, sel| format::set_heading(text, doc, region, sel, n)),
            C::Paragraph => self.edit_region(|text, doc, region, sel| format::set_heading(text, doc, region, sel, 0)),
            C::HeadingUp => {
                self.edit_region(|text, doc, region, sel| format::step_heading(text, doc, region, sel, true))
            }
            C::HeadingDown => {
                self.edit_region(|text, doc, region, sel| format::step_heading(text, doc, region, sel, false))
            }
            C::Quote => self.edit_blocks(false, |text, _doc, range, sel| Some(format::toggle_quote(text, range, sel))),
            C::OrderedList => {
                self.edit_blocks(true, |text, doc, range, _| Some(format::toggle_list(text, doc, range, true)))
            }
            C::BulletList => {
                self.edit_blocks(true, |text, doc, range, _| Some(format::toggle_list(text, doc, range, false)))
            }
            C::CodeBlock => self.insert_block("```\n", "\n```"),
            C::MathBlock => self.insert_block("$$\n", "\n$$"),
            C::HorizontalRule => self.insert_block("---", ""),
            C::MoveItemUp | C::MoveItemDown => self.move_item(cmd == C::MoveItemDown),
            // The cursor in the first header cell, between the spaces of `|  |`.
            C::InsertTable => self.insert_block_with_cursor("|  |  |\n| --- | --- |\n|  |  |\n|  |  |", 2),
            C::Bold => self.toggle(InlineStyle::Bold),
            C::Italic => self.toggle(InlineStyle::Italic),
            C::Underline => self.toggle(InlineStyle::Underline),
            C::Code => self.toggle(InlineStyle::Code),
            C::Strikethrough => self.toggle(InlineStyle::Strike),
            C::Highlight => self.toggle(InlineStyle::Highlight),
            C::InlineMath => self.toggle(InlineStyle::Math),
            // Inserted when the clipboard's text arrives (`LinkClipboard`).
            C::Link => {
                effects.push(Effect::ReadClipboardForLink);
                Task::none()
            }
            C::Copy | C::CopyMarkdown => {
                self.copy(cmd == C::CopyMarkdown, effects);
                Task::none()
            }
            C::Cut => {
                if self.copy(false, effects) {
                    self.delete_selection();
                }
                self.focus()
            }
            C::Paste | C::PastePlain => {
                effects.push(Effect::ReadClipboard(cmd == C::PastePlain));
                Task::none()
            }
            C::Image => {
                let dir = self.page.path().parent().map(Path::to_path_buf);
                Task::perform(
                    async move {
                        let mut dialog = rfd::AsyncFileDialog::new().add_filter("Images", images::IMAGE_EXTENSIONS);
                        if let Some(dir) = dir {
                            dialog = dialog.set_directory(dir);
                        }
                        dialog.pick_files().await.map(|files| files.iter().map(|f| f.path().to_path_buf()).collect())
                    },
                    EditorMsg::ImagesChosen,
                )
            }
            C::Find => self.open_find(false),
            C::Replace => self.open_find(true),
            C::FindNext => self.find_next(false),
            C::FindPrevious => self.find_next(true),
            C::SourceMode => self.toggle_source_mode(),
            C::Outline => {
                self.outline = !self.outline;
                effects.push(Effect::SetPrefs(EditorPrefs { outline: self.outline, ..prefs.clone() }));
                Task::none()
            }
            C::Assets => {
                self.toggle_assets();
                effects.push(Effect::SetPrefs(EditorPrefs { assets: self.assets.is_some(), ..prefs.clone() }));
                Task::none()
            }
            C::FullScreen => {
                self.fullscreen = !self.fullscreen;
                let mode = if self.fullscreen { window::Mode::Fullscreen } else { window::Mode::Windowed };
                self.window.map_or(Task::none(), |id| window::set_mode(id, mode))
            }
            C::ZoomIn | C::ZoomOut | C::ZoomReset => {
                let zoom = match cmd {
                    C::ZoomIn => zoom_step(ZOOM_STEPS, prefs.zoom, 1),
                    C::ZoomOut => zoom_step(ZOOM_STEPS, prefs.zoom, -1),
                    _ => 1.0,
                };
                effects.push(Effect::SetPrefs(EditorPrefs { zoom, ..prefs.clone() }));
                Task::none()
            }
            C::DarkTheme => {
                effects.push(Effect::SetPrefs(EditorPrefs { dark: !prefs.dark, ..prefs.clone() }));
                Task::none()
            }
            // The table's commands do nothing outside a table and in the
            // source mode; in a table, `table_command` ran them above.
            C::RowAbove
            | C::RowBelow
            | C::DeleteRow
            | C::ColumnLeft
            | C::ColumnRight
            | C::DeleteColumn
            | C::MoveRowUp
            | C::MoveRowDown
            | C::MoveColumnLeft
            | C::MoveColumnRight
            | C::AlignLeft
            | C::AlignCenter
            | C::AlignRight
            | C::DeleteTable => Task::none(),
        }
    }

    /// The file dialog of File → Open...: a Markdown file, from the page's folder on.
    fn ask_file_to_open(&self) -> Task<EditorMsg> {
        let dir = self.page.path().parent().map(Path::to_path_buf);
        Task::perform(
            async move {
                let mut dialog = rfd::AsyncFileDialog::new().add_filter("Markdown", &["md"]);
                if let Some(dir) = dir {
                    dialog = dialog.set_directory(dir);
                }
                dialog.pick_file().await.map(|f| f.path().to_path_buf())
            },
            EditorMsg::OpenChosen,
        )
    }

    /// The save dialog for an export: the page's name with `.html` / `.pdf`, next to it.
    fn ask_export_file(&self, pdf: bool) -> Task<EditorMsg> {
        let (ext, filter) = if pdf { ("pdf", "PDF") } else { ("html", "HTML") };
        let stem = crate::paths::file_stem(self.page.path());
        let dir = self.page.path().parent().map(Path::to_path_buf);
        Task::perform(
            async move {
                let mut dialog =
                    rfd::AsyncFileDialog::new().add_filter(filter, &[ext]).set_file_name(format!("{stem}.{ext}"));
                if let Some(dir) = dir {
                    dialog = dialog.set_directory(dir);
                }
                dialog.save_file().await.map(|f| f.path().to_path_buf())
            },
            move |out| EditorMsg::ExportTo(pdf, out),
        )
    }

    /// Exports the page in the background.
    pub(super) fn export_to(&mut self, pdf: bool, out: Option<PathBuf>) -> Task<EditorMsg> {
        let Some(out) = out else { return Task::none() };
        let (page, text) = (self.page.path().to_path_buf(), self.page.text().to_string());
        self.status.set(format!("exporting {}", file_name(&out)));
        Task::perform(
            blocking(move || {
                let css = settings::export_css().unwrap_or_else(|| html::DEFAULT_CSS.to_string());
                if pdf {
                    let browser = export::find_browser().ok_or_else(|| export::NO_BROWSER.to_string())?;
                    export::export_pdf(&browser, &page, &text, &out, &css)?;
                } else {
                    export::export_html(&page, &text, &out, &css)?;
                }
                Ok(out)
            }),
            EditorMsg::Exported,
        )
    }
}
