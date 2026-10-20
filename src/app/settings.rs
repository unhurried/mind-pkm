//! Per-user settings, kept in `<config dir>/mind-pkm/` (on Windows
//! `%APPDATA%\mind-pkm\`): `settings.txt` remembers the last opened map,
//! and an optional `template.md` replaces the built-in New Page template.
//! A file missing there is read from the directory used under the
//! application's former name, `freeplane-pkm`.

use crate::pkm::commands::DEFAULT_TEMPLATE;
use std::path::PathBuf;

const APP_DIR_NAME: &str = "mind-pkm";
const LEGACY_APP_DIR_NAME: &str = "freeplane-pkm";
const SETTINGS_FILE_NAME: &str = "settings.txt";
const TEMPLATE_FILE_NAME: &str = "template.md";
const LAST_FILE_KEY: &str = "last_file=";

pub fn config_dir() -> Option<PathBuf> {
    dirs::config_dir().map(|d| d.join(APP_DIR_NAME))
}

/// Reads a settings file, falling back to the former name's directory.
fn read(file_name: &str) -> Option<Vec<u8>> {
    let base = dirs::config_dir()?;
    [APP_DIR_NAME, LEGACY_APP_DIR_NAME].iter().find_map(|dir| std::fs::read(base.join(dir).join(file_name)).ok())
}

pub fn last_file() -> Option<PathBuf> {
    let text = String::from_utf8(read(SETTINGS_FILE_NAME)?).ok()?;
    text.lines().find_map(|l| l.strip_prefix(LAST_FILE_KEY)).filter(|p| !p.is_empty()).map(PathBuf::from)
}

pub fn set_last_file(path: &std::path::Path) {
    let Some(dir) = config_dir() else { return };
    let _ = std::fs::create_dir_all(&dir);
    let _ = std::fs::write(dir.join(SETTINGS_FILE_NAME), format!("{LAST_FILE_KEY}{}\n", path.display()));
}

/// The New Page template: the user's `template.md` if there is one, else the built-in one.
pub fn page_template() -> String {
    read(TEMPLATE_FILE_NAME)
        .map(|b| String::from_utf8_lossy(&b).into_owned())
        .unwrap_or_else(|| DEFAULT_TEMPLATE.to_string())
}
