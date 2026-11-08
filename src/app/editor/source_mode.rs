//! The source mode: the whole page as Markdown in a text editor, its edits
//! going into the page as they are made.

use super::keys::Bind;
use super::page::{content_sel, move_content};
use super::{Editor, EditorMsg, EditorPrefs};
use crate::markdown::format;
use iced::Task;
use iced::widget::text_editor::{Action, Content, Edit, Motion};

impl Editor {
    pub(super) fn toggle_source_mode(&mut self) -> Task<EditorMsg> {
        match self.source.take() {
            Some(content) => {
                let text = content.text();
                let sel = content_sel(&content, &text);
                self.page.forget_active();
                self.page.place(sel);
            }
            None => {
                let sel = self.page.sel();
                self.page.leave_at(sel);
                self.page.deselect_blocks();
                let mut content = Content::with_text(self.page.text());
                move_content(&mut content, self.page.text(), sel);
                self.source = Some(content);
            }
        }
        self.focus()
    }

    /// The source mode's text editor shows the page's text again (after undo or a reload).
    pub(super) fn refresh_source(&mut self) {
        if let Some(source) = &mut self.source {
            *source = Content::with_text(self.page.text());
            move_content(source, self.page.text(), self.page.sel());
            self.page.forget_active();
        }
    }

    pub(super) fn leave_source_for_history(&mut self) {
        if let Some(source) = &self.source {
            let text = source.text();
            if text != self.page.text() {
                let sel = content_sel(source, &text);
                self.page.set_source_text(text, sel);
            }
        }
    }

    pub(super) fn source_bind(&mut self, bind: Bind, prefs: &EditorPrefs) -> Task<EditorMsg> {
        let Some(source) = &mut self.source else { return Task::none() };
        let action = match bind {
            Bind::Enter | Bind::ShiftEnter => Action::Edit(Edit::Enter),
            Bind::Tab => Action::Edit(Edit::Insert('\t')),
            Bind::ShiftTab => return Task::none(),
            Bind::Backspace => Action::Edit(Edit::Backspace),
            Bind::Delete => Action::Edit(Edit::Delete),
            Bind::Up | Bind::ShiftUp => Action::Move(Motion::Up),
            Bind::Down | Bind::ShiftDown => Action::Move(Motion::Down),
            Bind::Left => Action::Move(Motion::Left),
            Bind::Right => Action::Move(Motion::Right),
            Bind::ShiftLeft => Action::Select(Motion::Left),
            Bind::ShiftRight => Action::Select(Motion::Right),
            Bind::Type(c) => {
                let text = source.text();
                let sel = content_sel(source, &text);
                if let Some(e) = prefs.auto_pair.then(|| format::auto_pair(&text, sel, c, false)).flatten() {
                    self.page.set_source_text(e.text, e.sel);
                    self.refresh_source();
                    return Task::none();
                }
                Action::Edit(Edit::Insert(c))
            }
        };
        self.on_action(action)
    }
}
