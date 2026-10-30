//! Where the files a page refers to are: image sources and link
//! destinations resolved against the page, the front matter settings Typora
//! defines for them, and inserting images: copying them next to the page
//! and writing the path to them.

use crate::paths;
use std::path::{Path, PathBuf};

/// The image file extensions the editor shows (and inserts as images).
pub const IMAGE_EXTENSIONS: &[&str] = &["png", "jpg", "jpeg", "gif", "bmp", "webp", "ico", "tif", "tiff", "svg"];

pub fn is_image(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| IMAGE_EXTENSIONS.contains(&e.to_ascii_lowercase().as_str()))
}

/// How inserted images are copied and referenced.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImageOptions {
    /// Copy images into the folder.
    pub copy: bool,
    /// The folder, `${filename}` standing for the page's name; relative to the page.
    pub folder: String,
    /// Write `./` before relative paths.
    pub dot_slash: bool,
    /// Escape spaces and other characters as `%XX`.
    pub escape: bool,
}

impl Default for ImageOptions {
    fn default() -> Self {
        ImageOptions { copy: true, folder: "${filename}.assets".into(), dot_slash: false, escape: false }
    }
}

/// The folder images of the page at `page` are copied into: the page's
/// `typora-copy-images-to`, else the options' folder.
pub fn image_folder(page: &Path, text: &str, opts: &ImageOptions) -> PathBuf {
    let stem = paths::file_stem(page);
    let folder = front_matter_value(text, "typora-copy-images-to").unwrap_or_else(|| opts.folder.clone());
    let folder = folder.replace("${filename}", &stem);
    let dir = page.parent().unwrap_or(Path::new("."));
    paths::link_to_path(&folder, Some(dir)).unwrap_or_else(|| dir.join(folder))
}

/// A path in `dir` for a file named `name` that is free: `name`, else
/// `stem-1.ext`, `stem-2.ext`...
pub fn free_name(dir: &Path, name: &str) -> PathBuf {
    let candidate = dir.join(name);
    if !candidate.exists() {
        return candidate;
    }
    let path = Path::new(name);
    let stem = paths::file_stem(path);
    let ext = path.extension().map(|e| format!(".{}", e.to_string_lossy())).unwrap_or_default();
    (1..).map(|n| dir.join(format!("{stem}-{n}{ext}"))).find(|p| !p.exists()).unwrap_or(candidate)
}

/// How the page at `page` refers to `file`: relative with `/` (the options
/// adding `./` and escaping), else (another drive) the absolute path.
pub fn reference(page: &Path, file: &Path, opts: &ImageOptions) -> String {
    let dir = page.parent().unwrap_or(Path::new("."));
    let link = paths::path_to_link(file, Some(dir));
    if link.starts_with("file:") {
        return file.display().to_string();
    }
    // The link is percent-encoded as a URI; decode unless escaping is wanted.
    let decoded = percent_encoding::percent_decode_str(&link).decode_utf8_lossy().into_owned();
    let mut r = if opts.escape { escape_path(&decoded) } else { decoded };
    if opts.dot_slash && !r.starts_with("../") {
        r = format!("./{r}");
    }
    r
}

/// Spaces and characters a URL cannot hold as `%XX` (slashes kept).
fn escape_path(path: &str) -> String {
    const KEEP: &percent_encoding::AsciiSet =
        &percent_encoding::NON_ALPHANUMERIC.remove(b'/').remove(b'.').remove(b'-').remove(b'_').remove(b'~');
    percent_encoding::utf8_percent_encode(path, KEEP).to_string()
}

/// Copies an image to insert into the page's image folder (unless it is
/// there already, or copying is off); the file the page should refer to.
pub fn copy_image(page: &Path, text: &str, file: &Path, opts: &ImageOptions) -> Result<PathBuf, String> {
    if !opts.copy {
        return Ok(file.to_path_buf());
    }
    let folder = image_folder(page, text, opts);
    if file.parent().is_some_and(|p| paths::same_file(p, &folder)) {
        return Ok(file.to_path_buf());
    }
    std::fs::create_dir_all(&folder).map_err(|e| format!("failed to create {}: {e}", folder.display()))?;
    let name = file.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| "image".into());
    let target = free_name(&folder, &name);
    std::fs::copy(file, &target).map_err(|e| format!("failed to copy {}: {e}", file.display()))?;
    Ok(target)
}

/// Saves pasted image data (RGBA) as a PNG named `image-<stamp>.png` in the
/// page's image folder.
pub fn save_png(
    page: &Path,
    text: &str,
    rgba: &[u8],
    width: u32,
    height: u32,
    stamp: &str,
    opts: &ImageOptions,
) -> Result<PathBuf, String> {
    let folder =
        if opts.copy { image_folder(page, text, opts) } else { page.parent().unwrap_or(Path::new(".")).to_path_buf() };
    std::fs::create_dir_all(&folder).map_err(|e| format!("failed to create {}: {e}", folder.display()))?;
    let target = free_name(&folder, &format!("image-{stamp}.png"));
    let img = image::RgbaImage::from_raw(width, height, rgba.to_vec()).ok_or("the pasted image is broken")?;
    img.save_with_format(&target, image::ImageFormat::Png)
        .map_err(|e| format!("failed to save {}: {e}", target.display()))?;
    Ok(target)
}

/// The Markdown inserted for a file: an image, or a link for other files.
pub fn markdown_for(page: &Path, file: &Path, opts: &ImageOptions) -> String {
    let r = reference(page, file, opts);
    if is_image(file) {
        format!("![]({r})")
    } else {
        let name = paths::file_name(file);
        format!("[{name}]({r})")
    }
}

/// The Markdown inserting files into the page at `page`: images copied into
/// its image folder, other files linked where they are.
pub fn files_markdown(page: &Path, text: &str, files: &[PathBuf], opts: &ImageOptions) -> Result<Vec<String>, String> {
    files
        .iter()
        .map(|f| {
            let target = if is_image(f) { copy_image(page, text, f, opts)? } else { f.clone() };
            Ok(markdown_for(page, &target, opts))
        })
        .collect()
}

/// A value of the page's YAML front matter (`key: value` at the top level).
pub fn front_matter_value(text: &str, key: &str) -> Option<String> {
    let rest = text.strip_prefix("---\n")?;
    let end = rest.find("\n---").or_else(|| rest.find("\n..."))?;
    rest[..end].lines().find_map(|line| {
        let (k, v) = line.split_once(':')?;
        (k.trim() == key).then(|| v.trim().trim_matches(['"', '\'']).to_string()).filter(|v| !v.is_empty())
    })
}

/// Whether a destination is a URL of another scheme than `file:`.
fn is_remote(dest: &str) -> bool {
    let Some(colon) = dest.find(':') else { return false };
    let scheme = &dest[..colon];
    // A drive letter is not a scheme.
    scheme.len() > 1
        && scheme.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '-' | '.'))
        && !scheme.eq_ignore_ascii_case("file")
}

/// The local file a link or image destination of the page at `page` names:
/// relative to the page's directory (percent-escapes decoded), `/...`
/// relative to `root_url` when given, `file:` URIs and absolute paths as they
/// are. None for URLs and anchors.
pub fn local_path(page: &Path, dest: &str, root_url: Option<&str>) -> Option<PathBuf> {
    let dest = dest.trim();
    let dest = dest.strip_prefix('<').and_then(|d| d.strip_suffix('>')).unwrap_or(dest);
    if dest.is_empty() || dest.starts_with('#') || is_remote(dest) {
        return None;
    }
    let dir = page.parent();
    let file = if dest.to_ascii_lowercase().starts_with("file:") { dest } else { dest.split('#').next()? };
    if let Some(root) = root_url
        && file.starts_with('/')
        && !file.starts_with("//")
    {
        let root = paths::link_to_path(root, dir)?;
        let rel = file.trim_start_matches('/');
        return paths::link_to_path(rel, Some(&root));
    }
    paths::link_to_path(file, dir)
}

/// The text of the page at `page` with every inline link and image whose
/// destination is `old` or lies inside it pointing at `old` renamed to
/// `new_name`, and how many changed. Only the renamed segment of a
/// destination is rewritten, escaped when the destination is: `./`, `<...>`,
/// the other segments and any `#anchor` stay as they are written.
pub fn rename_destinations(page: &Path, text: &str, old: &Path, new_name: &str) -> (String, usize) {
    use pulldown_cmark::{Event, Tag};
    let doc = super::parse::Document::parse(text);
    let mut edits: Vec<(std::ops::Range<usize>, String)> = Vec::new();
    for (event, range) in &doc.events {
        let dest = match event {
            Event::Start(Tag::Link { dest_url, .. } | Tag::Image { dest_url, .. }) => dest_url.as_ref(),
            _ => continue,
        };
        let src = &text[range.clone()];
        // The destination is what follows the last `](` (with a `<` or spaces).
        let Some(at) = src
            .rmatch_indices(dest)
            .map(|(i, _)| i)
            .find(|&i| !dest.is_empty() && src[..i].trim_end_matches(['<', ' ', '\t', '\n', '\r']).ends_with("]("))
        else {
            continue;
        };
        let Some(renamed) = renamed_destination(page, dest, old, new_name) else { continue };
        let start = range.start + at;
        edits.push((start..start + dest.len(), renamed));
    }
    edits.sort_by_key(|(r, _)| r.start);
    edits.dedup_by_key(|(r, _)| r.start);
    let mut out = String::with_capacity(text.len());
    let mut pos = 0;
    for (r, with) in &edits {
        out.push_str(&text[pos..r.start]);
        out.push_str(with);
        pos = r.end;
    }
    out.push_str(&text[pos..]);
    (out, edits.len())
}

/// `dest` with the segment naming `old` replaced by `new_name`, when it
/// points at `old` or inside it.
fn renamed_destination(page: &Path, dest: &str, old: &Path, new_name: &str) -> Option<String> {
    let resolved = local_path(page, dest, None)?;
    let (resolved, old_key) = (paths::key(&resolved), paths::key(old));
    let below = if resolved == old_key {
        0
    } else {
        let rest = resolved.strip_prefix(&old_key)?.strip_prefix(std::path::MAIN_SEPARATOR)?;
        rest.split(std::path::MAIN_SEPARATOR).count()
    };
    let (path, anchor) = match dest.find('#') {
        Some(i) if !dest.to_ascii_lowercase().starts_with("file:") => dest.split_at(i),
        _ => (dest, ""),
    };
    let mut segments: Vec<&str> = path.split('/').collect();
    // The segment `below` real ones before the end (trailing `/` and `.` skipped).
    let mut seen = 0;
    let mut index = None;
    for (i, s) in segments.iter().enumerate().rev() {
        match *s {
            "" | "." => continue,
            ".." => return None,
            _ if seen == below => {
                index = Some(i);
                break;
            }
            _ => seen += 1,
        }
    }
    let index = index?;
    let decoded = |s: &str| percent_encoding::percent_decode_str(s).decode_utf8_lossy().into_owned();
    let old_name = old.file_name()?.to_string_lossy();
    if !paths::same_file(Path::new(&decoded(segments[index])), Path::new(old_name.as_ref())) {
        return None;
    }
    let escaped = decoded(path) != path;
    let new_segment = if escaped { escape_path(new_name) } else { new_name.to_string() };
    segments[index] = &new_segment;
    Some(format!("{}{anchor}", segments.join("/")))
}

/// The local file an image source names, using the page's `typora-root-url`.
pub fn image_path(page: &Path, text: &str, src: &str) -> Option<PathBuf> {
    let root = front_matter_value(text, "typora-root-url");
    local_path(page, src, root.as_deref())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inserted_images_are_copied_next_to_the_page() {
        let dir = tempfile::tempdir().unwrap();
        let docs = dir.path().join("docs");
        let elsewhere = dir.path().join("pictures");
        std::fs::create_dir_all(&docs).unwrap();
        std::fs::create_dir_all(&elsewhere).unwrap();
        let page = docs.join("My Page.md");
        let a = elsewhere.join("photo 1.png");
        std::fs::write(&a, b"one").unwrap();
        let opts = ImageOptions::default();
        let copied = copy_image(&page, "", &a, &opts).unwrap();
        assert_eq!(copied, docs.join("My Page.assets").join("photo 1.png"));
        assert_eq!(markdown_for(&page, &copied, &opts), "![](My Page.assets/photo 1.png)");
        // A name taken gets a number.
        let again = copy_image(&page, "", &a, &opts).unwrap();
        assert_eq!(again, docs.join("My Page.assets").join("photo 1-1.png"));
        // A file already in the folder stays.
        assert_eq!(copy_image(&page, "", &copied, &opts).unwrap(), copied);
        // Escaping and ./ when asked for.
        let escaped = ImageOptions { escape: true, dot_slash: true, ..ImageOptions::default() };
        assert_eq!(markdown_for(&page, &copied, &escaped), "![](./My%20Page.assets/photo%201.png)");
        // Copying off: the original is referenced.
        let off = ImageOptions { copy: false, ..ImageOptions::default() };
        assert_eq!(
            markdown_for(&page, &copy_image(&page, "", &a, &off).unwrap(), &off),
            "![](../pictures/photo 1.png)"
        );
    }

    #[test]
    fn the_folder_from_settings_or_front_matter() {
        let page = Path::new("/docs/Trip.md");
        let opts = ImageOptions { folder: "img/${filename}".into(), ..ImageOptions::default() };
        assert_eq!(image_folder(page, "", &opts), paths::normalize(Path::new("/docs/img/Trip")));
        let text = "---\ntypora-copy-images-to: pics\n---\n";
        assert_eq!(image_folder(page, text, &opts), paths::normalize(Path::new("/docs/pics")));
    }

    #[test]
    fn pasted_images_are_saved_as_png() {
        let dir = tempfile::tempdir().unwrap();
        let page = dir.path().join("P.md");
        let path = save_png(&page, "", &[255, 0, 0, 255], 1, 1, "20241024120000", &ImageOptions::default()).unwrap();
        assert_eq!(path, dir.path().join("P.assets").join("image-20241024120000.png"));
        assert!(std::fs::read(&path).unwrap().starts_with(b"\x89PNG"));
    }

    #[test]
    fn other_files_become_links() {
        let page = Path::new("/docs/P.md");
        assert_eq!(
            markdown_for(page, Path::new("/docs/P.assets/a.pdf"), &ImageOptions::default()),
            "[a.pdf](P.assets/a.pdf)"
        );
        assert!(is_image(Path::new("x.JPG")));
        assert!(!is_image(Path::new("x.md")));
    }

    fn page() -> PathBuf {
        std::env::temp_dir().join("docs").join("Trip.md")
    }

    #[test]
    fn relative_and_escaped_sources() {
        let dir = page().parent().unwrap().to_path_buf();
        assert_eq!(
            local_path(&page(), "Trip.assets/map.png", None),
            Some(paths::normalize(&dir.join("Trip.assets/map.png")))
        );
        assert_eq!(
            local_path(&page(), "My%20Page.assets/a.png", None),
            Some(paths::normalize(&dir.join("My Page.assets/a.png")))
        );
        assert_eq!(local_path(&page(), "<a b.png>", None), Some(paths::normalize(&dir.join("a b.png"))));
        assert_eq!(local_path(&page(), "Other.md#part", None), Some(paths::normalize(&dir.join("Other.md"))));
    }

    #[test]
    fn urls_and_anchors_are_not_files() {
        assert_eq!(local_path(&page(), "https://example.com/logo.png", None), None);
        assert_eq!(local_path(&page(), "mailto:a@b.c", None), None);
        assert_eq!(local_path(&page(), "#heading", None), None);
    }

    #[test]
    fn root_relative_sources() {
        let dir = page().parent().unwrap().to_path_buf();
        let expected = paths::normalize(&dir.join("../site/img/a.png"));
        assert_eq!(local_path(&page(), "/img/a.png", Some("../site")), Some(expected));
        let text = "---\ntypora-root-url: ../site\n---\n\n![](/img/a.png)";
        assert_eq!(image_path(&page(), text, "/img/a.png"), Some(paths::normalize(&dir.join("../site/img/a.png"))));
    }

    #[test]
    fn front_matter_values() {
        let text = "---\ntitle: \"x\"\ntypora-copy-images-to: img\n---\nbody";
        assert_eq!(front_matter_value(text, "typora-copy-images-to").as_deref(), Some("img"));
        assert_eq!(front_matter_value(text, "title").as_deref(), Some("x"));
        assert_eq!(front_matter_value("no front matter", "title"), None);
    }

    #[test]
    fn renaming_rewrites_the_destinations_naming_the_item() {
        let dir = paths::normalize(Path::new("/docs"));
        let page = dir.join("Trip.md");
        let map = dir.join("Trip.assets").join("map.png");
        let rename = |text: &str, old: &Path, new: &str| rename_destinations(&page, text, old, new);
        assert_eq!(
            rename("![](Trip.assets/map.png) and [map](./Trip.assets/map.png#top)\n", &map, "route.png"),
            ("![](Trip.assets/route.png) and [map](./Trip.assets/route.png#top)\n".into(), 2)
        );
        // Escaped and angle-bracketed destinations keep their form.
        let raw = dir.join("Trip.assets").join("raw");
        assert_eq!(
            rename("[raw](./Trip.assets/raw/a%20b.txt)\n", &raw, "source files"),
            ("[raw](./Trip.assets/source%20files/a%20b.txt)\n".into(), 1)
        );
        assert_eq!(
            rename("[r](<Trip.assets/raw/a b.txt> \"raw\")\n", &raw, "source"),
            ("[r](<Trip.assets/source/a b.txt> \"raw\")\n".into(), 1)
        );
        assert_eq!(rename("[r](Trip.assets/raw/)\n", &raw, "src"), ("[r](Trip.assets/src/)\n".into(), 1));
        // A link around an image: both destinations.
        assert_eq!(
            rename("[![m](Trip.assets/map.png)](Trip.assets/map.png)\n", &map, "r.png"),
            ("[![m](Trip.assets/r.png)](Trip.assets/r.png)\n".into(), 2)
        );
        // Code, other files, a name that only starts the same, URLs: untouched.
        let untouched = "`![](Trip.assets/map.png)`\n\n![](Trip.assets/map.png.bak) [x](Trip.assets/other.png)\n\n\
                         [u](https://x.org/Trip.assets/map.png) [r][ref]\n\n[ref]: Trip.assets/map.png\n";
        assert_eq!(rename(untouched, &map, "route.png"), (untouched.to_string(), 0));
        // Renaming the assets folder itself.
        let assets = dir.join("Trip.assets");
        assert_eq!(
            rename("![](Trip.assets/map.png)\n", &assets, "Old.assets"),
            ("![](Old.assets/map.png)\n".into(), 1)
        );
    }
}
