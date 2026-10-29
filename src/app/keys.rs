//! Keyboard input: the shortcuts and arrow key navigation.

use super::command::Command;
use super::search_panel::{PanelFocus, SearchMsg};
use super::{App, Message};
use crate::mindmap::NodeId;
use iced::Task;
use iced::keyboard::{self, Key, Modifiers, key::Named};

impl App {
    pub(super) fn on_key(&mut self, event: keyboard::Event) -> Task<Message> {
        let (key, modifiers) = match event {
            keyboard::Event::ModifiersChanged(m) => {
                self.modifiers = m;
                return Task::none();
            }
            keyboard::Event::KeyPressed { key, modifiers, .. } => (key, modifiers),
            _ => return Task::none(),
        };
        // Handled by Message::Escape.
        if key == Key::Named(Named::Escape) {
            return Task::none();
        }
        self.modifiers = modifiers;
        if !self.dialogs.is_empty() {
            return match key {
                Key::Named(Named::Enter) => self.dialog_ok(),
                _ => Task::none(),
            };
        }
        if self.menu.is_some() {
            return Task::none();
        }
        if self.search.focus == PanelFocus::Results {
            match key {
                Key::Named(Named::ArrowUp) => return self.on_search(SearchMsg::Move(-1)),
                Key::Named(Named::ArrowDown) => return self.on_search(SearchMsg::Move(1)),
                Key::Named(Named::Enter) if modifiers.control() => return self.on_search(SearchMsg::SelectNode),
                Key::Named(Named::Enter) => return self.on_search(SearchMsg::OpenFile),
                _ => {}
            }
        }
        let command = command_for_key(&key, modifiers);
        // While typing in the search box only the function keys (and Ctrl+F,
        // which focuses the box again) reach the map.
        if self.search.focus == PanelFocus::Input && !is_function_key(&key) && command != Some(Command::Search) {
            return Task::none();
        }
        match command {
            Some(cmd) => self.run_command(cmd),
            None => {
                self.navigate(&key);
                Task::none()
            }
        }
    }

    /// Escape closes what is open on top - a dialog, a menu, the search
    /// panel - or else centers the selected node.
    pub(super) fn on_escape(&mut self) -> Task<Message> {
        if self.drag_cancel() {
            Task::none()
        } else if !self.dialogs.is_empty() {
            self.dialog_cancel()
        } else if self.menu.is_some() {
            self.menu = None;
            Task::none()
        } else if self.search.open {
            self.on_search(SearchMsg::Close)
        } else {
            self.run_command(Command::CenterSelected)
        }
    }

    /// Arrow key navigation between the shown nodes (Shift extends the selection).
    fn navigate(&mut self, key: &Key) {
        let Some(doc) = &mut self.doc else { return };
        let focus = doc.focus;
        let Some(b) = doc.layout.get(focus) else { return };
        let root = doc.map.root();
        let target = match key {
            Key::Named(Named::ArrowUp) | Key::Named(Named::ArrowDown) => {
                let up = *key == Key::Named(Named::ArrowUp);
                doc.layout.neighbor_on_level(focus, up)
            }
            Key::Named(Named::ArrowRight) => {
                if doc.map.is_folded(focus) && focus != root {
                    doc.map.set_folded(focus, false);
                    None
                } else {
                    // The child nearest to the node's vertical center.
                    let cy = b.rect.center_y();
                    let distance = |n: NodeId| (doc.layout.get(n).unwrap().rect.center_y() - cy).abs();
                    doc.layout
                        .visible_children(focus)
                        .iter()
                        .copied()
                        .min_by(|&x, &y| distance(x).total_cmp(&distance(y)))
                }
            }
            Key::Named(Named::ArrowLeft) => b.parent,
            Key::Named(Named::Home) => Some(root),
            _ => None,
        };
        if let Some(t) = target {
            if self.modifiers.shift() {
                doc.extend_selection(t);
            } else {
                doc.select(vec![t]);
            }
        }
    }
}

fn is_function_key(key: &Key) -> bool {
    matches!(
        key,
        Key::Named(
            Named::F1
                | Named::F2
                | Named::F3
                | Named::F4
                | Named::F5
                | Named::F6
                | Named::F7
                | Named::F8
                | Named::F9
                | Named::F10
                | Named::F11
                | Named::F12
        )
    )
}

/// The keyboard shortcuts. F1-F12 are the PKM commands of the former add-on.
pub fn command_for_key(key: &Key, m: Modifiers) -> Option<Command> {
    use Command::*;
    let ctrl = m.control() || m.command();
    Some(match key {
        Key::Named(named) => match (named, ctrl, m.shift()) {
            (Named::F1, false, _) => PkmOpen,
            (Named::F2, false, _) => PkmEdit,
            (Named::F3, false, _) => FoldOneLevel,
            (Named::F4, false, _) => UnfoldOneLevel,
            (Named::F5, false, _) => Filter,
            (Named::F6, false, _) => Search,
            (Named::F7, false, _) => ToggleCheckmark,
            (Named::F8, false, _) => AddToToDo,
            (Named::F9, false, _) => NewPage,
            (Named::F10, false, _) => NewDirectory,
            (Named::F12, false, _) => Update,
            (Named::Insert | Named::Tab, false, false) => NewChild,
            (Named::Enter, false, false) => NewSibling,
            (Named::F11 | Named::Delete, false, _) => PkmDelete,
            (Named::F11 | Named::Delete, true, _) => PkmDeleteNow,
            (Named::Space, false, _) => ToggleFold,
            (Named::ArrowUp, true, _) => MoveUp,
            (Named::ArrowDown, true, _) => MoveDown,
            _ => return None,
        },
        Key::Character(c) if ctrl => match (c.to_lowercase().as_str(), m.shift()) {
            // Ctrl aliases of the PKM function keys.
            ("o", _) => PkmOpen,
            ("f", false) => Search,
            ("f", true) => Filter,
            ("n", false) => NewPage,
            ("n", true) => NewDirectory,
            ("s", false) => Save,
            ("s", true) => SaveAs,
            ("z", false) => Undo,
            ("z", true) | ("y", _) => Redo,
            ("x", _) => Cut,
            ("c", _) => Copy,
            ("v", _) => Paste,
            ("k", _) => SetLink,
            ("e", _) => EditText,
            ("q", _) => Exit,
            // `+` is Shift+`=` (US) or Shift+`;` (Japanese); the unshifted key works too.
            ("+" | "=" | ";", _) => ZoomIn,
            ("-", _) => ZoomOut,
            ("0", _) => ZoomReset,
            _ => return None,
        },
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The key a menu shortcut label ("Ctrl+Shift+S", "F1", "Ctrl++") names.
    fn parse_shortcut(label: &str) -> (Key, Modifiers) {
        let mut modifiers = Modifiers::empty();
        let mut rest = label;
        loop {
            if let Some(r) = rest.strip_prefix("Ctrl+").filter(|r| !r.is_empty()) {
                modifiers |= Modifiers::CTRL;
                rest = r;
            } else if let Some(r) = rest.strip_prefix("Shift+").filter(|r| !r.is_empty()) {
                modifiers |= Modifiers::SHIFT;
                rest = r;
            } else {
                break;
            }
        }
        let named = match rest {
            "Tab" => Some(Named::Tab),
            "Insert" => Some(Named::Insert),
            "Enter" => Some(Named::Enter),
            "Delete" => Some(Named::Delete),
            "Space" => Some(Named::Space),
            "Up" => Some(Named::ArrowUp),
            "Down" => Some(Named::ArrowDown),
            f if f.starts_with('F') && f.len() > 1 => {
                let n: usize = f[1..].parse().expect(label);
                Some(
                    [
                        Named::F1,
                        Named::F2,
                        Named::F3,
                        Named::F4,
                        Named::F5,
                        Named::F6,
                        Named::F7,
                        Named::F8,
                        Named::F9,
                        Named::F10,
                        Named::F11,
                        Named::F12,
                    ][n - 1],
                )
            }
            _ => None,
        };
        let key = match named {
            Some(n) => Key::Named(n),
            None => {
                assert_eq!(rest.chars().count(), 1, "unknown key in shortcut {label:?}");
                Key::Character(rest.to_lowercase().into())
            }
        };
        (key, modifiers)
    }

    /// Every shortcut a menu shows is the key binding of its command (the
    /// menu, `command_for_key` and the README must not drift apart).
    #[test]
    fn menu_shortcuts_match_the_key_bindings() {
        let mut seen = Vec::new();
        for (_, items) in super::super::view::MENUS {
            for &(label, shortcut, cmd) in items.iter() {
                assert!(!seen.contains(&cmd), "{label}: {cmd:?} is in the menus twice");
                seen.push(cmd);
                // Escape is handled before the key bindings (it closes what is open first).
                if shortcut.is_empty() || shortcut == "Esc" {
                    continue;
                }
                for alternative in shortcut.split(" / ") {
                    let (key, modifiers) = parse_shortcut(alternative);
                    assert_eq!(command_for_key(&key, modifiers), Some(cmd), "{label}: {alternative}");
                }
            }
        }
        assert_eq!(command_for_key(&Key::Named(Named::Escape), Modifiers::empty()), None);
    }

    /// The README's table of PKM commands lists the PKM menu as it is.
    #[test]
    fn readme_lists_the_pkm_menu() {
        let readme = include_str!("../../README.md");
        let rows: Vec<String> = readme
            .lines()
            .filter(|l| l.starts_with('|'))
            .map(|l| l.split('|').map(str::trim).collect::<Vec<_>>().join("|"))
            .collect();
        let (_, pkm) = super::super::view::MENUS.iter().find(|(name, _)| *name == "PKM").unwrap();
        for (label, shortcut, _) in pkm.iter() {
            let row = format!("|{shortcut}|{label}|");
            assert!(rows.contains(&row), "README has no row {row}");
        }
    }

    #[test]
    fn ctrl_aliases_of_the_pkm_function_keys() {
        let key = |c: &str, m: Modifiers| command_for_key(&Key::Character(c.into()), m);
        let (ctrl, shifted) = (Modifiers::CTRL, Modifiers::CTRL | Modifiers::SHIFT);
        assert_eq!(key("o", ctrl), Some(Command::PkmOpen));
        assert_eq!(key("f", ctrl), Some(Command::Search));
        assert_eq!(key("f", shifted), Some(Command::Filter));
        assert_eq!(key("n", ctrl), Some(Command::NewPage));
        assert_eq!(key("n", shifted), Some(Command::NewDirectory));
        assert_eq!(key("f", Modifiers::empty()), None);
    }

    #[test]
    fn zoom_keys() {
        let ctrl = Modifiers::CTRL;
        let key = |c: &str| Key::Character(c.into());
        for c in ["+", "=", ";"] {
            assert_eq!(command_for_key(&key(c), ctrl), Some(Command::ZoomIn));
            assert_eq!(command_for_key(&key(c), ctrl | Modifiers::SHIFT), Some(Command::ZoomIn));
        }
        assert_eq!(command_for_key(&key("-"), ctrl), Some(Command::ZoomOut));
        assert_eq!(command_for_key(&key("0"), ctrl), Some(Command::ZoomReset));
        assert_eq!(command_for_key(&key("+"), Modifiers::empty()), None);
    }
}
