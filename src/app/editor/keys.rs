//! The page editor's commands, its menus and their keyboard shortcuts.

use super::EditorMsg;
use crate::app::ui::Menu;
use iced::keyboard::key::{Code, Named, Physical};
use iced::keyboard::{Key, Modifiers};
use iced::widget::text_editor::{Binding, KeyPress, Status};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditorCommand {
    // File
    Open,
    Save,
    ExportHtml,
    ExportPdf,
    Close,
    // Edit
    Undo,
    Redo,
    Cut,
    Copy,
    CopyMarkdown,
    Paste,
    PastePlain,
    SelectAll,
    SelectWord,
    SelectLine,
    SelectSpan,
    Find,
    Replace,
    FindNext,
    FindPrevious,
    ClearFormatting,
    ToggleAutoPair,
    // Paragraph
    Heading(u8),
    Paragraph,
    HeadingUp,
    HeadingDown,
    InsertTable,
    CodeBlock,
    MathBlock,
    Quote,
    OrderedList,
    BulletList,
    HorizontalRule,
    // Format
    Bold,
    Italic,
    Underline,
    Code,
    Strikethrough,
    Highlight,
    InlineMath,
    Link,
    Image,
    // Table
    RowAbove,
    RowBelow,
    DeleteRow,
    ColumnLeft,
    ColumnRight,
    DeleteColumn,
    MoveRowUp,
    MoveRowDown,
    MoveColumnLeft,
    MoveColumnRight,
    AlignLeft,
    AlignCenter,
    AlignRight,
    DeleteTable,
    // View
    SourceMode,
    Outline,
    Assets,
    FullScreen,
    ZoomIn,
    ZoomOut,
    ZoomReset,
    DarkTheme,
}

use EditorCommand as C;

pub const MENUS: &[Menu<EditorCommand>] = &[
    (
        "File",
        &[
            ("Open...", "Ctrl+O", C::Open),
            ("Save", "Ctrl+S", C::Save),
            ("Export HTML...", "", C::ExportHtml),
            ("Export PDF...", "", C::ExportPdf),
            ("Close", "Ctrl+W", C::Close),
        ],
    ),
    (
        "Edit",
        &[
            ("Undo", "Ctrl+Z", C::Undo),
            ("Redo", "Ctrl+Y", C::Redo),
            ("Cut", "Ctrl+X", C::Cut),
            ("Copy", "Ctrl+C", C::Copy),
            ("Copy as Markdown", "Ctrl+Shift+C", C::CopyMarkdown),
            ("Paste", "Ctrl+V", C::Paste),
            ("Paste as Plain Text", "Ctrl+Shift+V", C::PastePlain),
            ("Select All", "Ctrl+A", C::SelectAll),
            ("Select Word", "Ctrl+D", C::SelectWord),
            ("Select Line / Table Row", "Ctrl+L", C::SelectLine),
            ("Select Styled Text / Table Cell", "Ctrl+E", C::SelectSpan),
            ("Find", "Ctrl+F", C::Find),
            ("Replace", "Ctrl+H", C::Replace),
            ("Find Next", "F3", C::FindNext),
            ("Find Previous", "Shift+F3", C::FindPrevious),
            ("Clear Formatting", "Ctrl+\\", C::ClearFormatting),
            ("Auto-Pair Brackets", "", C::ToggleAutoPair),
        ],
    ),
    (
        "Paragraph",
        &[
            ("Heading 1", "Ctrl+1", C::Heading(1)),
            ("Heading 2", "Ctrl+2", C::Heading(2)),
            ("Heading 3", "Ctrl+3", C::Heading(3)),
            ("Heading 4", "Ctrl+4", C::Heading(4)),
            ("Heading 5", "Ctrl+5", C::Heading(5)),
            ("Heading 6", "Ctrl+6", C::Heading(6)),
            ("Paragraph", "Ctrl+0", C::Paragraph),
            ("Increase Heading Level", "Ctrl+=", C::HeadingUp),
            ("Decrease Heading Level", "Ctrl+-", C::HeadingDown),
            ("Table", "Ctrl+T", C::InsertTable),
            ("Code Block", "Ctrl+Shift+K", C::CodeBlock),
            ("Math Block", "Ctrl+Shift+M", C::MathBlock),
            ("Quote", "Ctrl+Shift+Q", C::Quote),
            ("Ordered List", "Ctrl+Shift+[", C::OrderedList),
            ("Bullet List", "Ctrl+Shift+]", C::BulletList),
            ("Horizontal Rule", "", C::HorizontalRule),
        ],
    ),
    (
        "Format",
        &[
            ("Bold", "Ctrl+B", C::Bold),
            ("Italic", "Ctrl+I", C::Italic),
            ("Underline", "Ctrl+U", C::Underline),
            ("Code", "Ctrl+Shift+`", C::Code),
            ("Strikethrough", "Alt+Shift+5", C::Strikethrough),
            ("Highlight", "", C::Highlight),
            ("Inline Math", "", C::InlineMath),
            ("Link", "Ctrl+K", C::Link),
            ("Image...", "Ctrl+Shift+I", C::Image),
        ],
    ),
    (
        "Table",
        &[
            ("Insert Row Above", "", C::RowAbove),
            ("Insert Row Below", "Ctrl+Enter", C::RowBelow),
            ("Delete Row", "Ctrl+Shift+Backspace", C::DeleteRow),
            ("Insert Column Left", "", C::ColumnLeft),
            ("Insert Column Right", "", C::ColumnRight),
            ("Delete Column", "", C::DeleteColumn),
            ("Move Row Up", "Alt+Up", C::MoveRowUp),
            ("Move Row Down", "Alt+Down", C::MoveRowDown),
            ("Move Column Left", "Alt+Left", C::MoveColumnLeft),
            ("Move Column Right", "Alt+Right", C::MoveColumnRight),
            ("Align Left", "", C::AlignLeft),
            ("Align Center", "", C::AlignCenter),
            ("Align Right", "", C::AlignRight),
            ("Delete Table", "", C::DeleteTable),
        ],
    ),
    (
        "View",
        &[
            ("Source Mode", "Ctrl+/", C::SourceMode),
            ("Outline", "Ctrl+Shift+1", C::Outline),
            ("Assets", "Ctrl+Shift+2", C::Assets),
            ("Full Screen", "F11", C::FullScreen),
            ("Zoom In", "Ctrl+Shift+=", C::ZoomIn),
            ("Zoom Out", "Ctrl+Shift+-", C::ZoomOut),
            ("Reset Zoom", "Ctrl+Shift+0", C::ZoomReset),
            ("Dark Theme", "", C::DarkTheme),
        ],
    ),
];

/// The page's right-click menu: commands of the menu bar, None for a separator.
pub const PAGE_MENU: &[Option<EditorCommand>] = &[
    Some(C::Cut),
    Some(C::Copy),
    Some(C::CopyMarkdown),
    Some(C::Paste),
    Some(C::PastePlain),
    Some(C::SelectAll),
    None,
    Some(C::Bold),
    Some(C::Italic),
    Some(C::Underline),
    Some(C::Code),
    Some(C::Strikethrough),
    Some(C::Link),
    None,
    Some(C::ClearFormatting),
];

/// A cell's right-click menu: the clipboard, then the table commands and
/// copying a row or column; None for a separator.
pub const TABLE_MENU: &[Option<TableMenuItem>] = &[
    Some(TableMenuItem::Command(C::Cut)),
    Some(TableMenuItem::Command(C::Copy)),
    Some(TableMenuItem::Command(C::Paste)),
    Some(TableMenuItem::Command(C::SelectAll)),
    None,
    Some(TableMenuItem::Command(C::RowAbove)),
    Some(TableMenuItem::Command(C::RowBelow)),
    Some(TableMenuItem::Command(C::DeleteRow)),
    Some(TableMenuItem::Command(C::ColumnLeft)),
    Some(TableMenuItem::Command(C::ColumnRight)),
    Some(TableMenuItem::Command(C::DeleteColumn)),
    None,
    Some(TableMenuItem::Command(C::MoveRowUp)),
    Some(TableMenuItem::Command(C::MoveRowDown)),
    Some(TableMenuItem::Command(C::MoveColumnLeft)),
    Some(TableMenuItem::Command(C::MoveColumnRight)),
    None,
    Some(TableMenuItem::Command(C::AlignLeft)),
    Some(TableMenuItem::Command(C::AlignCenter)),
    Some(TableMenuItem::Command(C::AlignRight)),
    None,
    Some(TableMenuItem::CopyRow),
    Some(TableMenuItem::CopyColumn),
];

impl TableMenuItem {
    /// Its label and shortcut: a command's as in the menu bar.
    pub fn label(self) -> (&'static str, &'static str) {
        match self {
            TableMenuItem::Command(cmd) => crate::app::ui::entry_of(MENUS, cmd).unwrap_or(("?", "")),
            TableMenuItem::CopyRow => ("Copy Row", ""),
            TableMenuItem::CopyColumn => ("Copy Column", ""),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TableMenuItem {
    Command(EditorCommand),
    CopyRow,
    CopyColumn,
}

/// The character of a key for shortcuts: the unshifted key, Latin letters
/// whatever the layout.
fn shortcut_char(key: &Key, physical: Physical) -> Option<String> {
    if let Some(c) = key.to_latin(physical) {
        return Some(c.to_ascii_lowercase().to_string());
    }
    match key {
        Key::Character(c) => Some(c.to_lowercase()),
        _ => None,
    }
}

/// The editor's keyboard shortcuts. Characters that depend on the keyboard
/// layout are accepted in their US and Japanese places (`;` is where `+` is
/// printed on Japanese keyboards, `@` where `` ` `` is).
pub fn command_for_key(key: &Key, physical: Physical, m: Modifiers) -> Option<EditorCommand> {
    let ctrl = m.control() || m.command();
    let (shift, alt) = (m.shift(), m.alt());
    if let Key::Named(named) = key {
        return Some(match (named, ctrl, shift, alt) {
            (Named::F3, false, false, false) => C::FindNext,
            (Named::F3, false, true, false) => C::FindPrevious,
            (Named::F11, false, false, false) => C::FullScreen,
            (Named::Enter, true, false, false) => C::RowBelow,
            (Named::Backspace, true, true, false) => C::DeleteRow,
            (Named::ArrowUp, false, false, true) => C::MoveRowUp,
            (Named::ArrowDown, false, false, true) => C::MoveRowDown,
            (Named::ArrowLeft, false, false, true) => C::MoveColumnLeft,
            (Named::ArrowRight, false, false, true) => C::MoveColumnRight,
            _ => return None,
        });
    }
    // Ctrl+Shift+2 by its key: the character there is `@` on US keyboards,
    // which (where `` ` `` is on Japanese ones) means Code.
    if ctrl && shift && !alt && physical == Physical::Code(Code::Digit2) {
        return Some(C::Assets);
    }
    let c = shortcut_char(key, physical)?;
    if alt && shift && !ctrl {
        return (c == "5" || c == "%").then_some(C::Strikethrough);
    }
    if !ctrl || alt {
        return None;
    }
    Some(match (c.as_str(), shift) {
        ("o", false) => C::Open,
        ("s", false) => C::Save,
        ("w", false) => C::Close,
        ("z", false) => C::Undo,
        ("y", _) | ("z", true) => C::Redo,
        ("x", false) => C::Cut,
        ("c", false) => C::Copy,
        ("c", true) => C::CopyMarkdown,
        ("v", false) => C::Paste,
        ("v", true) => C::PastePlain,
        ("a", false) => C::SelectAll,
        ("d", false) => C::SelectWord,
        ("l", false) => C::SelectLine,
        ("e", false) => C::SelectSpan,
        ("f", false) => C::Find,
        ("h", false) => C::Replace,
        ("\\" | "¥", false) => C::ClearFormatting,
        (d @ ("1" | "2" | "3" | "4" | "5" | "6"), false) => C::Heading(d.parse().unwrap_or(1)),
        ("0", false) => C::Paragraph,
        ("=" | ";" | "+", false) => C::HeadingUp,
        ("-", false) => C::HeadingDown,
        ("t", false) => C::InsertTable,
        ("k", true) => C::CodeBlock,
        ("m", true) => C::MathBlock,
        ("q", true) => C::Quote,
        ("[" | "{", true) => C::OrderedList,
        ("]" | "}", true) => C::BulletList,
        ("b", false) => C::Bold,
        ("i", false) => C::Italic,
        ("u", false) => C::Underline,
        ("`" | "@" | "~", true) => C::Code,
        ("k", false) => C::Link,
        ("i", true) => C::Image,
        ("/", false) => C::SourceMode,
        ("1" | "!", true) => C::Outline,
        ("2" | "\"", true) => C::Assets,
        ("=" | ";" | "+", true) => C::ZoomIn,
        ("-" | "_", true) => C::ZoomOut,
        ("0" | ")", true) => C::ZoomReset,
        _ => return None,
    })
}

/// What the keys bound by the editor (rather than the text editor) do.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Bind {
    Enter,
    ShiftEnter,
    Tab,
    ShiftTab,
    Backspace,
    Delete,
    Up,
    Down,
    Left,
    Right,
    ShiftUp,
    ShiftDown,
    /// Shift+Left / Shift+Right in a table (over cells at the edge of a cell's text).
    ShiftLeft,
    ShiftRight,
    /// A character auto-pairing may act on.
    Type(char),
}

/// What the text editor needs to know to bind keys.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BindContext {
    pub auto_pair: bool,
    /// The region is code (fewer characters are paired).
    pub code: bool,
    pub source_mode: bool,
    /// The cursor is in a table's cell.
    pub table: bool,
}

/// The characters auto-pairing acts on.
const PAIR_CHARS: &[char] =
    &['(', ')', '[', ']', '{', '}', '"', '\'', '`', '（', '）', '「', '」', '『', '』', '【', '】', '*', '_', '~', '$'];

/// The text editor's key bindings: the editor's commands and rules, else its own.
pub fn binding(press: KeyPress, ctx: BindContext) -> Option<Binding<EditorMsg>> {
    if !matches!(press.status, Status::Focused { .. }) {
        return None;
    }
    let m = press.modifiers;
    if let Some(cmd) = command_for_key(&press.key, press.physical_key, m) {
        return Some(Binding::Custom(EditorMsg::Command(cmd)));
    }
    let ctrl = m.control() || m.command();
    let custom = |b: Bind| Some(Binding::Custom(EditorMsg::Bind(b)));
    if let Key::Named(named) = press.key.as_ref() {
        let plain = !ctrl && !m.alt();
        match (named, m.shift()) {
            (Named::Escape, _) => return None,
            (Named::Enter, false) if plain && !ctx.source_mode => return custom(Bind::Enter),
            (Named::Enter, true) if plain => return custom(Bind::ShiftEnter),
            (Named::Tab, false) if plain => return custom(Bind::Tab),
            (Named::Tab, true) if plain => return custom(Bind::ShiftTab),
            (Named::Backspace, false) if plain => return custom(Bind::Backspace),
            (Named::Delete, false) if plain => return custom(Bind::Delete),
            (Named::ArrowUp, false) if plain && !ctx.source_mode => return custom(Bind::Up),
            (Named::ArrowDown, false) if plain && !ctx.source_mode => return custom(Bind::Down),
            (Named::ArrowLeft, false) if plain && !ctx.source_mode => return custom(Bind::Left),
            (Named::ArrowRight, false) if plain && !ctx.source_mode => return custom(Bind::Right),
            (Named::ArrowUp, true) if plain && !ctx.source_mode => return custom(Bind::ShiftUp),
            (Named::ArrowDown, true) if plain && !ctx.source_mode => return custom(Bind::ShiftDown),
            (Named::ArrowLeft, true) if plain && ctx.table => return custom(Bind::ShiftLeft),
            (Named::ArrowRight, true) if plain && ctx.table => return custom(Bind::ShiftRight),
            _ => {}
        }
    }
    if ctx.auto_pair
        && !ctrl
        && let Some(c) = press.text.as_deref().and_then(|t| t.chars().next())
        && PAIR_CHARS.contains(&c)
    {
        return custom(Bind::Type(c));
    }
    Binding::from_key_press(press)
}

#[cfg(test)]
mod tests {
    use super::*;
    use iced::keyboard::key::NativeCode;

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
            } else if let Some(r) = rest.strip_prefix("Alt+").filter(|r| !r.is_empty()) {
                modifiers |= Modifiers::ALT;
                rest = r;
            } else {
                break;
            }
        }
        let key = match rest {
            "Enter" => Key::Named(Named::Enter),
            "Backspace" => Key::Named(Named::Backspace),
            "Up" => Key::Named(Named::ArrowUp),
            "Down" => Key::Named(Named::ArrowDown),
            "Left" => Key::Named(Named::ArrowLeft),
            "Right" => Key::Named(Named::ArrowRight),
            "F3" => Key::Named(Named::F3),
            "F11" => Key::Named(Named::F11),
            c => {
                assert_eq!(c.chars().count(), 1, "unknown key in shortcut {label:?}");
                Key::Character(c.to_lowercase().into())
            }
        };
        (key, modifiers)
    }

    fn unidentified() -> Physical {
        Physical::Unidentified(NativeCode::Unidentified)
    }

    /// The right-click menus list commands of the menu bar.
    #[test]
    fn context_menu_items_are_commands_of_the_menu_bar() {
        for cmd in PAGE_MENU.iter().flatten() {
            assert!(crate::app::ui::entry_of(MENUS, *cmd).is_some(), "{cmd:?} is not in the menu bar");
        }
        for item in TABLE_MENU.iter().flatten() {
            assert_ne!(item.label().0, "?", "{item:?} is not in the menu bar");
        }
    }

    /// Every shortcut the editor's menus show is the key binding of its command.
    #[test]
    fn menu_shortcuts_match_the_key_bindings() {
        let mut seen = Vec::new();
        for (_, items) in MENUS {
            for &(label, shortcut, cmd) in items.iter() {
                assert!(!seen.contains(&cmd), "{label}: {cmd:?} is in the menus twice");
                seen.push(cmd);
                if shortcut.is_empty() {
                    continue;
                }
                let (key, modifiers) = parse_shortcut(shortcut);
                assert_eq!(command_for_key(&key, unidentified(), modifiers), Some(cmd), "{label}: {shortcut}");
            }
        }
    }

    #[test]
    fn ctrl_o_opens_a_file() {
        let o = Key::Character("o".into());
        assert_eq!(command_for_key(&o, unidentified(), Modifiers::CTRL), Some(C::Open));
        assert!(
            MENUS[0].1.iter().any(|&(label, shortcut, cmd)| (label, shortcut, cmd) == ("Open...", "Ctrl+O", C::Open))
        );
    }

    #[test]
    fn japanese_layout_keys() {
        let ch = |c: &str, m: Modifiers| command_for_key(&Key::Character(c.into()), unidentified(), m);
        let (ctrl, ctrl_shift) = (Modifiers::CTRL, Modifiers::CTRL | Modifiers::SHIFT);
        assert_eq!(ch(";", ctrl), Some(C::HeadingUp));
        assert_eq!(ch(";", ctrl_shift), Some(C::ZoomIn));
        assert_eq!(ch("-", ctrl_shift), Some(C::ZoomOut));
        assert_eq!(ch("@", ctrl_shift), Some(C::Code));
        assert_eq!(ch("¥", ctrl), Some(C::ClearFormatting));
        assert_eq!(ch("b", Modifiers::empty()), None);
        assert_eq!(ch("\"", ctrl_shift), Some(C::Assets));
    }

    #[test]
    fn assets_by_the_key_of_2() {
        let digit2 = Physical::Code(Code::Digit2);
        let ctrl_shift = Modifiers::CTRL | Modifiers::SHIFT;
        // US: Shift+2 is `@`; Japanese: `"`.
        for c in ["@", "\"", "2"] {
            assert_eq!(command_for_key(&Key::Character(c.into()), digit2, ctrl_shift), Some(C::Assets), "{c}");
        }
        // `@` on its own key (Japanese) stays Code.
        let at = Physical::Code(Code::BracketLeft);
        assert_eq!(command_for_key(&Key::Character("@".into()), at, ctrl_shift), Some(C::Code));
        assert_eq!(command_for_key(&Key::Character("2".into()), digit2, Modifiers::CTRL), Some(C::Heading(2)));
    }
}
