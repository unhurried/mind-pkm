//! The editor windows as the application keeps them: opening a page in one
//! (or bringing its window to the front), handing them their messages and
//! doing what they ask (`Effect`), the settings they share, and keeping them
//! in step with what the PKM commands do to their files.

use super::command::open_with_default;
use super::editor::{AssetsMsg, Editor, EditorDialog, EditorMsg, EditorPrefs, Effect, opens_in_editor};
use super::file::file_name;
use super::{App, Message, blocking, settings, window_settings};
use crate::pkm;
use iced::{Task, window};
use std::path::{Path, PathBuf};

impl App {
    /// Handles a message of the editor window `id`.
    pub(super) fn editor_message(&mut self, id: window::Id, msg: EditorMsg) -> Task<Message> {
        let focused = matches!(msg, EditorMsg::Focused);
        let Some(editor) = self.editors.get_mut(&id) else { return Task::none() };
        if focused {
            editor.check_disk();
            editor.assets_stale();
            editor.forget_image_files();
            return Task::none();
        }
        let mut effects = Vec::new();
        let task = editor.update(msg, &self.editor_prefs, &mut effects).map(move |m| Message::Editor(id, m));
        let mut tasks = vec![task, self.request_renders()];
        for effect in effects {
            tasks.push(self.editor_effect(id, effect));
        }
        Task::batch(tasks)
    }

    fn editor_effect(&mut self, id: window::Id, effect: Effect) -> Task<Message> {
        match effect {
            Effect::OpenPage(path) => {
                let opened = self.try_open_page(&path);
                self.report_in_editor(id, opened)
            }
            Effect::OpenInEditor(path) => {
                let opened = self.try_open_in_editor(&path);
                self.report_in_editor(id, opened)
            }
            Effect::OpenExternal(target) => {
                if let Err(e) = opener::open(&target)
                    && let Some(editor) = self.editors.get_mut(&id)
                {
                    editor.dialogs.push_back(EditorDialog::Message(format!("failed to open {target}: {e}")));
                }
                Task::none()
            }
            Effect::Close => self.close_editor(id),
            Effect::SetPrefs(prefs) => {
                self.set_editor_prefs(prefs);
                Task::none()
            }
            Effect::Copy { text, html } => {
                if let Ok(mut c) = self.system_clipboard.lock() {
                    c.write(&text, html.as_deref());
                }
                Task::none()
            }
            Effect::ReadClipboard(plain) => {
                let clipboard = self.system_clipboard.clone();
                Task::perform(
                    blocking(move || clipboard.lock().map(|mut c| c.read()).unwrap_or_default()),
                    move |content| Message::Editor(id, EditorMsg::Pasted(content, plain)),
                )
            }
            Effect::ReadClipboardForLink => {
                let clipboard = self.system_clipboard.clone();
                Task::perform(blocking(move || clipboard.lock().ok().and_then(|mut c| c.read_text())), move |text| {
                    Message::Editor(id, EditorMsg::LinkClipboard(text))
                })
            }
            Effect::AssetMenu(path) => {
                let can_change = self.asset_doc_dir(&path).is_ok();
                if let Some(editor) = self.editors.get_mut(&id) {
                    editor.show_asset_menu(path, can_change);
                }
                Task::none()
            }
            Effect::RenameAsset(path, name) => {
                let result = self.asset_doc_dir(&path).and_then(|d| pkm::assets::rename(&path, &name, &d));
                if let Some(editor) = self.editors.get_mut(&id) {
                    match result {
                        Ok(Some(new)) => editor.asset_renamed(&path, &new),
                        Ok(None) => {}
                        Err(e) => editor.dialogs.push_back(EditorDialog::Message(e)),
                    }
                }
                Task::none()
            }
            Effect::DeleteAsset(path) => {
                let remove = self.remover;
                let result = self.asset_doc_dir(&path).and_then(|d| pkm::assets::delete(&path, &d, &mut |f| remove(f)));
                if let Some(editor) = self.editors.get_mut(&id) {
                    match result {
                        Ok(()) => editor.asset_deleted(&path),
                        Err(e) => editor.dialogs.push_back(EditorDialog::Message(e)),
                    }
                }
                Task::none()
            }
        }
    }

    /// The document directory, when the assets pane may rename or delete
    /// `path`: inside it, and holding no file open in an editor window.
    fn asset_doc_dir(&self, path: &Path) -> Result<PathBuf, String> {
        let name = file_name(path);
        let doc = self.doc.as_ref().ok_or_else(|| format!("{name} is not in the document directory."))?;
        let doc_dir = pkm::load_doc_dir(doc.map())?;
        if !pkm::assets::can_change(path, &doc_dir) {
            return Err(format!("{name} is not in the document directory."));
        }
        if !self.editors_of(path).is_empty() {
            return Err(format!("{name} is open in an editor."));
        }
        Ok(doc_dir)
    }

    /// Reads the folders the assets panes ask for, in the background.
    pub(super) fn read_assets(&mut self) -> Task<Message> {
        let mut tasks = Vec::new();
        for (&id, editor) in self.editors.iter_mut() {
            if let Some(dir) = editor.start_assets_read() {
                tasks.push(Task::perform(
                    blocking(move || {
                        let tree = pkm::assets::read_tree(&dir);
                        (dir, tree)
                    }),
                    move |(dir, tree)| Message::Editor(id, EditorMsg::Assets(AssetsMsg::Read(dir, tree))),
                ));
            }
        }
        Task::batch(tasks)
    }

    pub(super) fn set_editor_prefs(&mut self, prefs: EditorPrefs) {
        let old = std::mem::replace(&mut self.editor_prefs, prefs.clone());
        if old.builtin != prefs.builtin {
            settings::set_editor_builtin(prefs.builtin);
        }
        if old.dark != prefs.dark {
            settings::set_editor_dark(prefs.dark);
        }
        if old.zoom != prefs.zoom {
            settings::set_editor_zoom(prefs.zoom);
        }
        if old.auto_pair != prefs.auto_pair {
            settings::set_editor_auto_pair(prefs.auto_pair);
        }
        if old.window != prefs.window {
            settings::set_editor_window(prefs.window);
        }
        if old.outline != prefs.outline {
            settings::set_editor_outline(prefs.outline);
        }
        if old.assets != prefs.assets {
            settings::set_editor_assets(prefs.assets);
        }
    }

    /// Closes the editor window `id`; without a map window, the last one
    /// ends the application.
    pub(super) fn close_editor(&mut self, id: window::Id) -> Task<Message> {
        self.editors.remove(&id);
        let closed = window::close(id);
        if self.editors_only && self.editors.is_empty() {
            return Task::batch([closed, self.exit()]);
        }
        closed
    }

    /// Opens a file: a Markdown file in the built-in editor (bringing its
    /// window to the front when it is open already), anything else - and
    /// everything when the built-in editor is off - with its default
    /// application. A failure is shown in the map window.
    pub(super) fn open_page(&mut self, path: &Path) -> Task<Message> {
        let opened = self.try_open_page(path);
        self.report(opened).unwrap_or_else(Task::none)
    }

    /// Opens a file as `open_page` does; the error if it can't be opened.
    fn try_open_page(&mut self, path: &Path) -> Result<Task<Message>, String> {
        if !opens_in_editor(path, self.editor_prefs.builtin) {
            open_with_default(path)?;
            return Ok(Task::none());
        }
        self.try_open_in_editor(path)
    }

    /// Opens a Markdown file in an editor window; a failure is shown in the map window.
    pub(super) fn open_in_editor(&mut self, path: &Path) -> Task<Message> {
        let opened = self.try_open_in_editor(path);
        self.report(opened).unwrap_or_else(Task::none)
    }

    /// Opens a Markdown file in an editor window (or brings its window to
    /// the front); the error if it can't be opened.
    pub(super) fn try_open_in_editor(&mut self, path: &Path) -> Result<Task<Message>, String> {
        let key = crate::paths::key(path);
        if let Some((&id, _)) = self.editors.iter().find(|(_, e)| crate::paths::key(e.page.path()) == key) {
            return Ok(window::gain_focus(id));
        }
        let mut editor = Editor::open(path, &self.editor_prefs)?;
        let (id, opened) = window::open(window_settings(self.editor_prefs.window, None, false));
        editor.window = Some(id);
        self.editors.insert(id, editor);
        // Opened at the start too, before any message: the assets pane it
        // opens with reads its folder now.
        Ok(Task::batch([opened.discard(), self.request_renders(), self.read_assets()]))
    }

    /// What an open the editor window `id` asked for gives: its task, or
    /// nothing with the error shown in that window.
    fn report_in_editor(&mut self, id: window::Id, opened: Result<Task<Message>, String>) -> Task<Message> {
        opened.unwrap_or_else(|e| {
            if let Some(editor) = self.editors.get_mut(&id) {
                editor.dialogs.push_back(EditorDialog::Message(e));
            }
            Task::none()
        })
    }

    /// The page file or directory of a page or directory node.
    pub(super) fn doc_path_of(&self, id: crate::mindmap::NodeId) -> Option<PathBuf> {
        let map = &self.doc.as_ref()?.map();
        let doc_dir = pkm::load_doc_dir(map).ok()?;
        let name = map.text(id);
        Some(match pkm::doc_type(map, id, &doc_dir)? {
            pkm::DocType::Page => pkm::page_file(&doc_dir, &name),
            pkm::DocType::Directory => pkm::directory_dir(&doc_dir, &name),
        })
    }

    /// Before a PKM command moves, changes or deletes `paths` (files, or
    /// directories with everything in them): their editors' unsaved changes
    /// are saved. The error of the first that can't be.
    pub(super) fn save_editors_of(&mut self, paths: &[PathBuf]) -> Result<(), String> {
        let ids: Vec<window::Id> = paths.iter().flat_map(|p| self.editors_of(p)).collect();
        for id in ids {
            if let Some(editor) = self.editors.get_mut(&id)
                && editor.page.is_dirty()
            {
                editor.page.save()?;
            }
        }
        Ok(())
    }

    /// After `old` (a page file or a directory) was renamed to `new`: its
    /// editors show the files at their new place, with their new text.
    pub(super) fn editors_renamed(&mut self, old: &Path, new: &Path) {
        let depth = old.components().count();
        for id in self.editors_of(old) {
            let Some(editor) = self.editors.get_mut(&id) else { continue };
            let rest: PathBuf = editor.page.path().components().skip(depth).collect();
            editor.moved_to(if rest.as_os_str().is_empty() { new.to_path_buf() } else { new.join(rest) });
        }
    }

    /// After a PKM command changed `path`: its editors read it again.
    pub(super) fn editors_reload(&mut self, path: &Path) {
        for id in self.editors_of(path) {
            if let Some(editor) = self.editors.get_mut(&id) {
                editor.reload();
            }
        }
    }

    /// After a deletion: the editors of `paths` whose files are gone close.
    pub(super) fn close_deleted_editors(&mut self, paths: &[PathBuf]) -> Task<Message> {
        let ids: Vec<window::Id> = paths.iter().flat_map(|p| self.editors_of(p)).collect();
        let gone: Vec<window::Id> =
            ids.into_iter().filter(|id| self.editors.get(id).is_some_and(|e| !e.page.path().exists())).collect();
        Task::batch(gone.into_iter().map(|id| self.close_editor(id)))
    }

    /// The editors of files at or under `path` (a page, or a renamed directory).
    pub(super) fn editors_of(&self, path: &Path) -> Vec<window::Id> {
        let key = crate::paths::key(path);
        self.editors
            .iter()
            .filter(|(_, e)| {
                let k = crate::paths::key(e.page.path());
                k == key || k.starts_with(&key) && k[key.len()..].starts_with(std::path::MAIN_SEPARATOR)
            })
            .map(|(&id, _)| id)
            .collect()
    }

    /// Every tick: editors notice files changed on disk.
    pub(super) fn editors_tick(&mut self) -> Task<Message> {
        for editor in self.editors.values_mut() {
            editor.check_disk();
            editor.forget_image_files();
        }
        Task::none()
    }

    /// The names of the pages with unsaved changes.
    pub(super) fn unsaved_pages(&self) -> Vec<String> {
        self.editors.values().filter(|e| e.page.is_dirty()).map(|e| crate::paths::file_name(e.page.path())).collect()
    }

    /// Saves every page with unsaved changes; the errors of those that fail.
    pub(super) fn save_all_pages(&mut self) -> Vec<String> {
        let mut errors = Vec::new();
        for editor in self.editors.values_mut() {
            if editor.page.is_dirty()
                && let Err(e) = editor.page.save()
            {
                errors.push(e);
            }
        }
        errors
    }
}
