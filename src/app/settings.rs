//! Per-user settings, kept in `<config dir>/mind-pkm/` (on Windows
//! `%APPDATA%\mind-pkm\`): `settings.txt` remembers the last opened map,
//! the window's size and position and the page editor's settings (`key=value`
//! lines), `views.txt` how each map was last shown (zoom, scroll position,
//! selection), an optional `template.md` replaces the built-in New Page
//! template and an optional `export.css` the style sheet of exported pages. A
//! file missing there is read from the directory used under the application's
//! former name, `freeplane-pkm`. Tests never touch these files.

use crate::paths;
use crate::pkm::commands::DEFAULT_TEMPLATE;
use iced::{Point, Size};
use std::path::{Path, PathBuf};

const APP_DIR_NAME: &str = "mind-pkm";
const LEGACY_APP_DIR_NAME: &str = "freeplane-pkm";
const SETTINGS_FILE_NAME: &str = "settings.txt";
const TEMPLATE_FILE_NAME: &str = "template.md";
const VIEWS_FILE_NAME: &str = "views.txt";
const LAST_FILE_KEY: &str = "last_file=";
const WINDOW_KEY: &str = "window=";
const EXPORT_CSS_FILE_NAME: &str = "export.css";
// The page editor's settings.
const PAGE_EDITOR_KEY: &str = "page_editor=";
const EDITOR_WINDOW_KEY: &str = "editor_window=";
const EDITOR_ZOOM_KEY: &str = "editor_zoom=";
const EDITOR_THEME_KEY: &str = "editor_theme=";
const AUTO_PAIR_KEY: &str = "auto_pair=";
const IMAGE_COPY_KEY: &str = "image_copy=";
const IMAGE_FOLDER_KEY: &str = "image_folder=";
const IMAGE_DOT_SLASH_KEY: &str = "image_dot_slash=";
const IMAGE_ESCAPE_KEY: &str = "image_escape=";
/// A remembered window smaller than this is not restored.
const MIN_WINDOW_SIDE: f32 = 200.0;
/// How many maps `views.txt` remembers, the most recently shown first.
const MAX_VIEWS: usize = 50;

/// The user's configuration directory; none in tests, so that they never
/// read or write the real settings.
fn base_dir() -> Option<PathBuf> {
    if cfg!(test) { None } else { dirs::config_dir() }
}

pub fn config_dir() -> Option<PathBuf> {
    base_dir().map(|d| d.join(APP_DIR_NAME))
}

/// Reads a settings file, falling back to the former name's directory.
fn read(file_name: &str) -> Option<Vec<u8>> {
    let base = base_dir()?;
    [APP_DIR_NAME, LEGACY_APP_DIR_NAME].iter().find_map(|dir| std::fs::read(base.join(dir).join(file_name)).ok())
}

/// Reads a settings file as UTF-8 text.
fn read_text(file_name: &str) -> Option<String> {
    String::from_utf8(read(file_name)?).ok()
}

/// The value of `key` (`name=`) in `settings.txt`.
fn value(key: &str) -> Option<String> {
    let text = read_text(SETTINGS_FILE_NAME)?;
    text.lines().find_map(|l| l.strip_prefix(key)).filter(|v| !v.is_empty()).map(str::to_string)
}

/// Sets `key` (`name=`) in `settings.txt`, keeping the other lines.
fn set_value(key: &str, value: &str) {
    let text = read_text(SETTINGS_FILE_NAME).unwrap_or_default();
    write(SETTINGS_FILE_NAME, with_value(&text, key, value));
}

/// `text` with the line of `key` replaced by `value` (or added).
fn with_value(text: &str, key: &str, value: &str) -> String {
    let others = text.lines().filter(|l| !l.is_empty() && !l.starts_with(key)).map(str::to_string);
    others.chain([format!("{key}{value}")]).map(|l| l + "\n").collect()
}

pub fn last_file() -> Option<PathBuf> {
    value(LAST_FILE_KEY).map(PathBuf::from)
}

pub fn set_last_file(path: &Path) {
    set_value(LAST_FILE_KEY, &path.display().to_string());
}

/// The window's size and position when it is not maximized (in logical
/// pixels), and whether it is maximized.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WindowGeometry {
    pub size: Size,
    pub position: Option<Point>,
    pub maximized: bool,
}

/// The window as it was when the application last exited.
pub fn window() -> Option<WindowGeometry> {
    parse_window(&value(WINDOW_KEY)?)
}

pub fn set_window(window: &WindowGeometry) {
    set_value(WINDOW_KEY, &format_window(window));
}

/// `width,height,x,y,maximized`; x and y are empty when the position is unknown.
fn parse_window(text: &str) -> Option<WindowGeometry> {
    let fields: Vec<&str> = text.split(',').collect();
    let [w, h, x, y, maximized] = fields[..] else { return None };
    let number = |s: &str| s.parse::<f32>().ok().filter(|n| n.is_finite());
    let size = Size::new(number(w)?, number(h)?);
    if size.width < MIN_WINDOW_SIDE || size.height < MIN_WINDOW_SIDE {
        return None;
    }
    let position = match (x, y) {
        ("", "") => None,
        _ => Some(Point::new(number(x)?, number(y)?)),
    };
    Some(WindowGeometry { size, position, maximized: maximized == "1" })
}

fn format_window(w: &WindowGeometry) -> String {
    let (x, y) = w.position.map_or((String::new(), String::new()), |p| (p.x.to_string(), p.y.to_string()));
    format!("{},{},{x},{y},{}", w.size.width, w.size.height, u8::from(w.maximized))
}

/// Writes a settings file; a failure only means the setting is not kept.
fn write(file_name: &str, text: String) {
    let Some(dir) = config_dir() else { return };
    let _ = std::fs::create_dir_all(&dir);
    let _ = std::fs::write(dir.join(file_name), text);
}

/// How a map was last shown.
#[derive(Debug, Clone, PartialEq)]
pub struct MapView {
    pub zoom: f32,
    /// The scroll position of the map, in screen pixels at `zoom`.
    pub scroll: (f32, f32),
    /// The Freeplane IDs of the selected nodes, in the order they were
    /// selected (the focused one last).
    pub selection: Vec<String>,
}

/// The remembered view of the map at `path`.
pub fn map_view(path: &Path) -> Option<MapView> {
    let text = read_text(VIEWS_FILE_NAME)?;
    let key = paths::key(path);
    parse_views(&text).into_iter().find(|(p, _)| paths::key(p) == key).map(|(_, view)| view)
}

/// Remembers the view of the map at `path`, as the most recent one.
pub fn set_map_view(path: &Path, view: MapView) {
    let text = read_text(VIEWS_FILE_NAME).unwrap_or_default();
    write(VIEWS_FILE_NAME, format_views(&with_view(parse_views(&text), path, view)));
}

/// `views` with `view` for `path` first, replacing an older one, at most `MAX_VIEWS` long.
fn with_view(views: Vec<(PathBuf, MapView)>, path: &Path, view: MapView) -> Vec<(PathBuf, MapView)> {
    let key = paths::key(path);
    let others = views.into_iter().filter(|(p, _)| paths::key(p) != key);
    std::iter::once((path.to_path_buf(), view)).chain(others).take(MAX_VIEWS).collect()
}

/// One map per line: `zoom TAB x TAB y TAB id,id,... TAB path`; lines that
/// don't read are skipped.
fn parse_views(text: &str) -> Vec<(PathBuf, MapView)> {
    let parse = |line: &str| {
        let mut fields = line.splitn(5, '\t');
        let mut number = || fields.next()?.parse::<f32>().ok().filter(|n| n.is_finite());
        let (zoom, x, y) = (number()?, number()?, number()?);
        let selection = fields.next()?.split(',').filter(|id| !id.is_empty()).map(str::to_string).collect();
        let path = fields.next().filter(|p| !p.is_empty())?;
        Some((PathBuf::from(path), MapView { zoom, scroll: (x, y), selection }))
    };
    text.lines().filter_map(parse).collect()
}

fn format_views(views: &[(PathBuf, MapView)]) -> String {
    views
        .iter()
        .map(|(path, v)| {
            format!("{}\t{}\t{}\t{}\t{}\n", v.zoom, v.scroll.0, v.scroll.1, v.selection.join(","), path.display())
        })
        .collect()
}

/// Whether pages open in the built-in editor (`page_editor=external` turns it off).
pub fn editor_builtin() -> bool {
    value(PAGE_EDITOR_KEY).is_none_or(|v| v != "external")
}

pub fn set_editor_builtin(builtin: bool) {
    set_value(PAGE_EDITOR_KEY, if builtin { "builtin" } else { "external" });
}

/// The size of the last editor window.
pub fn editor_window() -> Option<Size> {
    let v = value(EDITOR_WINDOW_KEY)?;
    let (w, h) = v.split_once(',')?;
    let size = Size::new(w.parse().ok()?, h.parse().ok()?);
    (size.width >= MIN_WINDOW_SIDE && size.height >= MIN_WINDOW_SIDE).then_some(size)
}

pub fn set_editor_window(size: Size) {
    set_value(EDITOR_WINDOW_KEY, &format!("{},{}", size.width, size.height));
}

pub fn editor_zoom() -> Option<f32> {
    value(EDITOR_ZOOM_KEY)?.parse().ok().filter(|z: &f32| z.is_finite() && *z > 0.1)
}

pub fn set_editor_zoom(zoom: f32) {
    set_value(EDITOR_ZOOM_KEY, &zoom.to_string());
}

pub fn editor_dark() -> bool {
    value(EDITOR_THEME_KEY).is_some_and(|v| v == "dark")
}

pub fn set_editor_dark(dark: bool) {
    set_value(EDITOR_THEME_KEY, if dark { "dark" } else { "light" });
}

/// Whether the editor pairs brackets (`auto_pair=0` turns it off).
pub fn editor_auto_pair() -> bool {
    value(AUTO_PAIR_KEY).is_none_or(|v| v != "0")
}

pub fn set_editor_auto_pair(on: bool) {
    set_value(AUTO_PAIR_KEY, if on { "1" } else { "0" });
}

pub use crate::markdown::images::ImageOptions;

pub fn image_options() -> ImageOptions {
    let d = ImageOptions::default();
    ImageOptions {
        copy: value(IMAGE_COPY_KEY).is_none_or(|v| v != "0"),
        folder: value(IMAGE_FOLDER_KEY).unwrap_or(d.folder),
        dot_slash: value(IMAGE_DOT_SLASH_KEY).is_some_and(|v| v == "1"),
        escape: value(IMAGE_ESCAPE_KEY).is_some_and(|v| v == "1"),
    }
}

/// The user's style sheet for exported pages, if any.
pub fn export_css() -> Option<String> {
    read_text(EXPORT_CSS_FILE_NAME)
}

/// The New Page template: the user's `template.md` if there is one, else the built-in one.
pub fn page_template() -> String {
    read(TEMPLATE_FILE_NAME)
        .map(|b| String::from_utf8_lossy(&b).into_owned())
        .unwrap_or_else(|| DEFAULT_TEMPLATE.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn view(zoom: f32, ids: &[&str]) -> MapView {
        MapView { zoom, scroll: (12.5, 300.0), selection: ids.iter().map(|s| s.to_string()).collect() }
    }

    #[test]
    fn a_value_replaces_its_line_and_keeps_the_others() {
        let text = with_value("last_file=/a.mm\n", WINDOW_KEY, "1,2,3,4,0");
        assert_eq!(text, "last_file=/a.mm\nwindow=1,2,3,4,0\n");
        assert_eq!(with_value(&text, LAST_FILE_KEY, "/b.mm"), "window=1,2,3,4,0\nlast_file=/b.mm\n");
    }

    #[test]
    fn the_window_reads_back_as_written() {
        let placed = WindowGeometry {
            size: Size::new(1000.0, 700.5),
            position: Some(Point::new(-1200.0, 30.0)),
            maximized: true,
        };
        let unplaced = WindowGeometry { position: None, maximized: false, ..placed };
        for w in [placed, unplaced] {
            assert_eq!(parse_window(&format_window(&w)), Some(w));
        }
        for bad in ["", "1000,700", "x,700,0,0,0", "100,700,0,0,0", "1000,700,5,,0", "1000,700,0,0,0,0"] {
            assert_eq!(parse_window(bad), None, "{bad}");
        }
    }

    #[test]
    fn views_read_back_as_written() {
        let views = vec![
            (PathBuf::from("/maps/a map.mm"), view(1.5, &["ID_1", "ID_2"])),
            (PathBuf::from("/maps/b.mm"), view(1.0, &[])),
        ];
        assert_eq!(parse_views(&format_views(&views)), views);
    }

    #[test]
    fn lines_that_dont_read_are_skipped() {
        let text = "x\t0\t0\tID_1\t/a.mm\n1\t0\t0\tID_1\n\n1.1\t5\t6\tID_9\t/b.mm\n1\tNaN\t0\t\t/c.mm\n";
        assert_eq!(
            parse_views(text),
            vec![(PathBuf::from("/b.mm"), MapView { zoom: 1.1, scroll: (5.0, 6.0), selection: vec!["ID_9".into()] })]
        );
    }

    #[test]
    fn the_latest_view_comes_first_and_replaces_the_older_one() {
        let views = vec![(PathBuf::from("/a.mm"), view(1.0, &[])), (PathBuf::from("/b.mm"), view(1.0, &[]))];
        let views = with_view(views, Path::new("/b.mm"), view(2.0, &["ID_3"]));
        assert_eq!(
            views,
            vec![(PathBuf::from("/b.mm"), view(2.0, &["ID_3"])), (PathBuf::from("/a.mm"), view(1.0, &[]))]
        );
    }

    #[test]
    fn at_most_max_views_are_kept() {
        let mut views = Vec::new();
        for i in 0..MAX_VIEWS + 5 {
            views = with_view(views, &PathBuf::from(format!("/m{i}.mm")), view(1.0, &[]));
        }
        assert_eq!(views.len(), MAX_VIEWS);
        assert_eq!(views[0].0, PathBuf::from(format!("/m{}.mm", MAX_VIEWS + 4)));
    }
}
