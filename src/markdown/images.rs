//! Where the files a page refers to are: image sources and link
//! destinations resolved against the page, and the front matter settings
//! Typora defines for them.

use crate::paths;
use std::path::{Path, PathBuf};

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

/// The local file an image source names, using the page's `typora-root-url`.
pub fn image_path(page: &Path, text: &str, src: &str) -> Option<PathBuf> {
    let root = front_matter_value(text, "typora-root-url");
    local_path(page, src, root.as_deref())
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
