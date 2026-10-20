//! Path arithmetic and the conversion between file paths and the link URIs
//! Freeplane stores in a node's `LINK` attribute.

use std::path::{Component, Path, PathBuf};

/// Absolute path with `.` and `..` folded away - pure path arithmetic, no file
/// system access, symlinks are not resolved.
pub fn normalize(path: &Path) -> PathBuf {
    let abs = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir().map(|d| d.join(path)).unwrap_or_else(|_| path.to_path_buf())
    };
    let mut out = PathBuf::new();
    for c in abs.components() {
        match c {
            Component::CurDir => {}
            Component::ParentDir => {
                out.pop();
            }
            other => out.push(other.as_os_str()),
        }
    }
    out
}

/// A key identifying a file the way the platform does: normalized, and on
/// Windows with separators unified and case folded (like `java.io.File#equals`).
pub fn key(path: &Path) -> String {
    let s = normalize(path).to_string_lossy().into_owned();
    if cfg!(windows) { s.replace('/', "\\").to_lowercase() } else { s }
}

pub fn same_file(a: &Path, b: &Path) -> bool {
    key(a) == key(b)
}

/// Whether `path` lies inside `dir` and is not `dir` itself, compared like
/// [`key`]. Every segment below `dir` has to name something once its trailing
/// dots and spaces are stripped, as Windows does - otherwise (`...`, `. `) it
/// would resolve to `dir` or above it there. Checked the same on every platform.
pub fn is_strictly_inside(path: &Path, dir: &Path) -> bool {
    let (path, dir) = (key(path), key(dir));
    let sep = std::path::MAIN_SEPARATOR;
    let Some(rest) = path.strip_prefix(&dir) else { return false };
    let rest = if dir.ends_with(sep) { Some(rest) } else { rest.strip_prefix(sep) };
    rest.is_some_and(|r| !r.is_empty() && r.split(sep).all(|s| !s.trim_end_matches(['.', ' ']).is_empty()))
}

// Characters encoded in a link URI path, as java.net.URI does for a path
// component; non-ASCII characters are left as they are.
const URI_PATH_ENCODE: &percent_encoding::AsciiSet = &percent_encoding::CONTROLS
    .add(b' ')
    .add(b'"')
    .add(b'#')
    .add(b'%')
    .add(b'<')
    .add(b'>')
    .add(b'?')
    .add(b'[')
    .add(b'\\')
    .add(b']')
    .add(b'^')
    .add(b'`')
    .add(b'{')
    .add(b'|')
    .add(b'}');

fn encode_path(path: &str) -> String {
    percent_encoding::utf8_percent_encode(path, URI_PATH_ENCODE).to_string()
}

fn decode(s: &str) -> String {
    percent_encoding::percent_decode_str(s).decode_utf8_lossy().into_owned()
}

/// Whether the link has a URI scheme (`file:`, `http:`, ...). A single letter
/// followed by a colon is a Windows drive, not a scheme.
fn scheme_of(link: &str) -> Option<&str> {
    let colon = link.find(':')?;
    let scheme = &link[..colon];
    let valid = scheme.len() > 1
        && scheme.chars().next().is_some_and(|c| c.is_ascii_alphabetic())
        && scheme.chars().all(|c| c.is_ascii_alphanumeric() || "+-.".contains(c));
    valid.then_some(scheme)
}

/// The file a link points at, or None for node links (`#ID_...`) and non-file
/// URIs (http, mailto, ...). Relative links are resolved against `base_dir`
/// (the map file's directory).
pub fn link_to_path(link: &str, base_dir: Option<&Path>) -> Option<PathBuf> {
    let link = link.trim();
    if link.is_empty() || link.starts_with('#') {
        return None;
    }
    match scheme_of(link) {
        Some(s) if s.eq_ignore_ascii_case("file") => {
            let rest = &link[5..];
            let rest = rest.split(['?', '#']).next().unwrap_or(rest);
            let path = if let Some(authority_path) = rest.strip_prefix("//") {
                // file://host/path or file:///path
                let slash = authority_path.find('/').unwrap_or(authority_path.len());
                let (host, path) = authority_path.split_at(slash);
                if host.is_empty() || host.eq_ignore_ascii_case("localhost") {
                    decode(path)
                } else {
                    format!("//{}{}", host, decode(path))
                }
            } else {
                decode(rest)
            };
            Some(normalize(&platform_path(&path)))
        }
        Some(_) => None,
        None => {
            let decoded = if link.contains('%') { decode(link) } else { link.to_string() };
            let p = platform_path(&decoded);
            if p.is_absolute() || looks_like_windows_absolute(&decoded) {
                return Some(normalize(&p));
            }
            let base = base_dir.map(Path::to_path_buf).unwrap_or_default();
            Some(normalize(&base.join(p)))
        }
    }
}

fn looks_like_windows_absolute(s: &str) -> bool {
    let b = s.as_bytes();
    b.len() >= 3 && b[0].is_ascii_alphabetic() && b[1] == b':' && (b[2] == b'/' || b[2] == b'\\')
}

/// Turns a URI-style path ("/C:/x/y", "//server/share/x") into a platform path.
fn platform_path(s: &str) -> PathBuf {
    if cfg!(windows) {
        let s = if s.len() >= 3 && s.starts_with('/') && looks_like_windows_absolute(&s[1..]) { &s[1..] } else { s };
        PathBuf::from(s.replace('/', "\\"))
    } else {
        PathBuf::from(s)
    }
}

/// The link URI for a file: relative to `base_dir` when there is one and the
/// file can be reached from it (same drive), else an absolute `file:` URI -
/// what Freeplane writes with its default "relative links" preference.
pub fn path_to_link(path: &Path, base_dir: Option<&Path>) -> String {
    let target = normalize(path);
    if let Some(base) = base_dir
        && let Some(rel) = relative_path(&target, &normalize(base))
    {
        return encode_path(&rel);
    }
    let s = target.to_string_lossy().replace('\\', "/");
    if let Some(unc) = s.strip_prefix("//") {
        format!("file://{}", encode_path(unc))
    } else if s.starts_with('/') {
        format!("file:{}", encode_path(&s))
    } else {
        format!("file:/{}", encode_path(&s))
    }
}

/// `target` relative to `base` with "/" separators, or None when they share no root.
fn relative_path(target: &Path, base: &Path) -> Option<String> {
    let t: Vec<Component> = target.components().collect();
    let b: Vec<Component> = base.components().collect();
    let eq = |x: &Component, y: &Component| {
        if cfg!(windows) {
            x.as_os_str().to_string_lossy().to_lowercase() == y.as_os_str().to_string_lossy().to_lowercase()
        } else {
            x == y
        }
    };
    // The root (drive / prefix) has to match.
    let mut common = 0;
    while common < t.len() && common < b.len() && eq(&t[common], &b[common]) {
        common += 1;
    }
    let root_len = b.iter().take_while(|c| matches!(c, Component::Prefix(_) | Component::RootDir)).count();
    if common < root_len.max(1) {
        return None;
    }
    let mut parts: Vec<String> = Vec::new();
    for _ in common..b.len() {
        parts.push("..".into());
    }
    for c in &t[common..] {
        parts.push(c.as_os_str().to_string_lossy().into_owned());
    }
    if parts.is_empty() {
        return Some(".".into());
    }
    Some(parts.join("/"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(not(windows))]
    #[test]
    fn resolves_relative_and_absolute_links() {
        let base = Path::new("/maps");
        assert_eq!(link_to_path("docs/My%20Page.md", Some(base)), Some(PathBuf::from("/maps/docs/My Page.md")));
        assert_eq!(link_to_path("../docs/日本.md", Some(base)), Some(PathBuf::from("/docs/日本.md")));
        assert_eq!(link_to_path("file:/home/u/a%20b.md", Some(base)), Some(PathBuf::from("/home/u/a b.md")));
        assert_eq!(link_to_path("file:///home/u/x", None), Some(PathBuf::from("/home/u/x")));
        assert_eq!(link_to_path("#ID_1", Some(base)), None);
        assert_eq!(link_to_path("https://example.com", Some(base)), None);
    }

    #[cfg(not(windows))]
    #[test]
    fn writes_relative_links_when_possible() {
        let base = Path::new("/maps");
        assert_eq!(path_to_link(Path::new("/maps/docs/My Page.md"), Some(base)), "docs/My%20Page.md");
        assert_eq!(path_to_link(Path::new("/docs/C++ #1.md"), Some(base)), "../docs/C++%20%231.md");
        assert_eq!(path_to_link(Path::new("/docs/x.md"), None), "file:/docs/x.md");
        let link = path_to_link(Path::new("/docs/日本 語.md"), Some(base));
        assert_eq!(link_to_path(&link, Some(base)), Some(PathBuf::from("/docs/日本 語.md")));
    }

    #[cfg(not(windows))]
    #[test]
    fn normalizes_dot_segments() {
        let p = normalize(Path::new("/a/./b/../c"));
        assert_eq!(p, PathBuf::from("/a/c"));
    }

    #[test]
    fn strictly_inside() {
        let d = std::env::temp_dir().join("docs");
        for (p, inside) in [
            (d.join("a.md"), true),
            (d.join("sub").join("a.md"), true),
            (d.join(".hidden"), true),
            (d.join("a/../b"), true),
            (d.clone(), false),
            (d.join(""), false),
            (d.join("."), false),
            (d.join(".."), false),
            (d.join("../docs"), false),
            (d.join("../docs2/x"), false),
            (d.with_file_name("docs2").join("x"), false),
            // Windows strips trailing dots and spaces: these name `d` (or its parent).
            (d.join("..."), false),
            (d.join(". "), false),
            (d.join("a").join(".. "), false),
        ] {
            assert_eq!(is_strictly_inside(&p, &d), inside, "{}", p.display());
        }
    }

    #[cfg(windows)]
    #[test]
    fn windows_links() {
        let base = Path::new(r"C:\Users\me\maps");
        assert_eq!(normalize(Path::new(r"C:\a\.\b\..\c")), PathBuf::from(r"C:\a\c"));
        assert_eq!(link_to_path("file:/C:/docs/a%20b.md", Some(base)), Some(PathBuf::from(r"C:\docs\a b.md")));
        assert_eq!(link_to_path("file:///C:/docs/x.md", Some(base)), Some(PathBuf::from(r"C:\docs\x.md")));
        assert_eq!(link_to_path("../docs/x.md", Some(base)), Some(PathBuf::from(r"C:\Users\me\docs\x.md")));
        assert_eq!(path_to_link(Path::new(r"C:\Users\me\docs\My Page.md"), Some(base)), "../docs/My%20Page.md");
        assert_eq!(path_to_link(Path::new(r"D:\docs\x.md"), Some(base)), "file:/D:/docs/x.md");
        assert!(same_file(Path::new(r"C:\Docs\X.md"), Path::new("c:/docs/x.md")));
    }
}
