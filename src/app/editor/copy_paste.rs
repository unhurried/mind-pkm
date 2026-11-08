//! Copying, cutting and pasting, and inserting files - pasted, dropped on
//! the window or chosen with Format → Image - and pasted image data.

use super::clipboard::{self, ClipboardContent};
use super::page::content_sel;
use super::{Editor, EditorMsg, EditorPrefs, Effect};
use crate::app::blocking;
use crate::markdown::edit::{self, Sel};
use crate::markdown::paste::{self, Paste};
use crate::markdown::{html, images, table};
use iced::Task;
use iced::widget::text_editor::{Action, Edit};
use std::path::PathBuf;
use std::sync::Arc;

impl Editor {
    /// Copies the selection (Markdown, and HTML unless `markdown_only`).
    /// Returns whether there was something to copy.
    pub(super) fn copy(&mut self, markdown_only: bool, effects: &mut Vec<Effect>) -> bool {
        // A range of table cells: tab-separated text and an HTML table, or a Markdown table.
        if let Some(a) = self.page.active()
            && let Some(state) = a.table()
            && let Some(range) = state.range
        {
            let t = &state.table;
            let (text, html) = if markdown_only {
                (t.range_markdown(range), None)
            } else {
                (t.range_tsv(range), Some(t.range_html(range, cell_html)))
            };
            self.copied = Some(text.clone());
            effects.push(Effect::Copy { text, html });
            return true;
        }
        let text = match &self.source {
            Some(source) => source.selection().unwrap_or_default(),
            None => self.page.selected_source(),
        };
        if text.is_empty() {
            return false;
        }
        let html = (!markdown_only).then(|| html::fragment(&text));
        self.copied = Some(text.clone());
        effects.push(Effect::Copy { text, html });
        true
    }

    /// Removes what is selected (after a cut).
    pub(super) fn delete_selection(&mut self) {
        if self.page.selected_blocks().is_some() {
            self.delete_blocks(None);
        } else if let Some(source) = &mut self.source {
            source.perform(Action::Edit(Edit::Delete));
            let text = source.text();
            let sel = content_sel(source, &text);
            self.page.set_source_text(text, sel);
        } else if self.page.active().and_then(|a| a.table()).is_some_and(|t| t.range.is_some()) {
            self.clear_cells();
        } else {
            self.page.perform(Action::Edit(Edit::Delete));
        }
    }

    /// Pastes what the clipboard held: files and images inserted, HTML
    /// converted to Markdown (a table as a table), text as it is (see
    /// `markdown::paste`).
    pub(super) fn pasted(&mut self, content: ClipboardContent, plain: bool, prefs: &EditorPrefs) -> Task<EditorMsg> {
        let in_code = self.page.active().is_some_and(|a| a.kind.is_code());
        let text = paste::clipboard_text(content.text.as_deref(), content.html.as_deref(), in_code);
        if !plain {
            if !content.files.is_empty() {
                return self.insert_files(content.files, prefs);
            }
            if let Some(image) = content.image.filter(|_| text.trim().is_empty()) {
                return self.insert_image_data(image, prefs);
            }
        }
        match paste::convert(&text, content.html.as_deref(), in_code, plain, self.copied.as_deref()) {
            Paste::Cells(grid) if self.page.active().is_some_and(|a| a.table().is_some()) => {
                self.page.change_table(|t, cell| {
                    t.paste(cell, &grid);
                    cell
                });
            }
            Paste::Cells(grid) => return self.insert_block(&paste::cells_markdown(&grid), ""),
            Paste::Text(text) => self.insert_text(&text),
            Paste::Nothing => {}
        }
        self.focus()
    }

    /// Inserts text at the cursor (in place of the selected blocks).
    pub(super) fn insert_text(&mut self, text: &str) {
        if self.page.selected_blocks().is_some() {
            let Some(range) = self.page.blocks_range() else { return };
            let new = edit::splice(self.page.text(), range.clone(), text);
            self.page.deselect_blocks();
            self.page.apply(edit::Edited { text: new, sel: Sel::caret(range.start + text.len()), pending: None });
            return;
        }
        if let Some(source) = &mut self.source {
            source.perform(Action::Edit(Edit::Paste(Arc::new(text.to_string()))));
            let t = source.text();
            let sel = content_sel(source, &t);
            self.page.set_source_text(t, sel);
            return;
        }
        if self.page.active().is_none() {
            self.page.place(self.page.sel());
        }
        self.page.perform(Action::Edit(Edit::Paste(Arc::new(text.to_string()))));
    }

    /// Copies files into the image folder in the background and inserts them.
    pub(super) fn insert_files(&mut self, files: Vec<PathBuf>, prefs: &EditorPrefs) -> Task<EditorMsg> {
        if files.is_empty() {
            return Task::none();
        }
        let (page, text, opts) = (self.page.path().to_path_buf(), self.page.text().to_string(), prefs.images.clone());
        Task::perform(blocking(move || images::files_markdown(&page, &text, &files, &opts)), EditorMsg::Inserted)
    }

    /// Saves pasted image data as a PNG in the image folder and inserts it.
    fn insert_image_data(&mut self, image: clipboard::PastedImage, prefs: &EditorPrefs) -> Task<EditorMsg> {
        let (page, text, opts) = (self.page.path().to_path_buf(), self.page.text().to_string(), prefs.images.clone());
        let stamp = chrono::Local::now().format("%Y%m%d%H%M%S").to_string();
        Task::perform(
            blocking(move || {
                let file = images::save_png(&page, &text, &image.rgba, image.width, image.height, &stamp, &opts)?;
                Ok(vec![images::markdown_for(&page, &file, &opts)])
            }),
            EditorMsg::Inserted,
        )
    }
}

/// A table cell's Markdown as HTML, for copying cells.
fn cell_html(cell: &str) -> String {
    let html = html::fragment(&table::cell_to_text(cell).replace('\n', "  \n"));
    html.trim().trim_start_matches("<p>").trim_end_matches("</p>").to_string()
}
