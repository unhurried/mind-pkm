//! The settings every editor window shares, as the user's settings
//! remember them; the application keeps them there when an editor changes
//! them (`Effect::SetPrefs`).

use crate::app::settings;
use crate::markdown::images::ImageOptions;
use iced::Size;

/// The size of the first editor window, when none was remembered.
pub const DEFAULT_WINDOW: Size = Size::new(900.0, 860.0);

/// The editor settings every editor window shares.
#[derive(Debug, Clone, PartialEq)]
pub struct EditorPrefs {
    /// Pages open in the built-in editor (else with the default application).
    pub builtin: bool,
    pub dark: bool,
    pub zoom: f32,
    pub auto_pair: bool,
    pub window: Size,
    pub images: ImageOptions,
    /// A new window shows the outline (as it was last shown or hidden).
    pub outline: bool,
    /// A new window shows the assets pane (as it was last shown or hidden).
    pub assets: bool,
}

impl EditorPrefs {
    pub fn load() -> EditorPrefs {
        EditorPrefs {
            builtin: settings::editor_builtin(),
            dark: settings::editor_dark(),
            zoom: settings::editor_zoom().unwrap_or(1.0),
            auto_pair: settings::editor_auto_pair(),
            window: settings::editor_window().unwrap_or(DEFAULT_WINDOW),
            images: settings::image_options(),
            outline: settings::editor_outline(),
            assets: settings::editor_assets(),
        }
    }
}

impl Default for EditorPrefs {
    fn default() -> Self {
        EditorPrefs {
            builtin: true,
            dark: false,
            zoom: 1.0,
            auto_pair: true,
            window: DEFAULT_WINDOW,
            images: ImageOptions::default(),
            outline: false,
            assets: false,
        }
    }
}
