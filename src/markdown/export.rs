//! Exporting a page: a self-contained HTML document, and a PDF of it printed
//! by Microsoft Edge or Google Chrome in the background.

use super::Document;
use super::html::{self, HtmlOptions};
use super::{images, render};
use crate::paths;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

pub const NO_BROWSER: &str = "PDF export needs Microsoft Edge or Google Chrome.";

/// The color math is drawn in (the style sheet's text color).
const MATH_COLOR: (u8, u8, u8) = (0x1f, 0x23, 0x28);

/// How long printing a PDF may take.
const PDF_TIMEOUT: Duration = Duration::from_secs(90);

/// The HTML document of the page at `page` (source `text`): math and diagrams
/// as SVG, local images relative to `base_dir` (where the document will be),
/// as absolute `file:` URIs without one.
pub fn html_document(page: &Path, text: &str, base_dir: Option<&Path>, css: &str) -> String {
    let doc = Document::parse(text);
    let math = |tex: &str, display: bool| render::math_svg(tex, display, MATH_COLOR).ok();
    let mermaid = |code: &str| render::mermaid_svg(code).ok();
    let image_src = |src: &str| match images::image_path(page, text, src) {
        Some(file) => paths::path_to_link(&file, base_dir),
        None => src.to_string(),
    };
    let body = html::body(&doc, &HtmlOptions { math: &math, mermaid: &mermaid, image_src: &image_src });
    let title = page.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
    html::page(&title, &body, css)
}

/// Writes the page as HTML to `out`.
pub fn export_html(page: &Path, text: &str, out: &Path, css: &str) -> Result<(), String> {
    let html = html_document(page, text, out.parent(), css);
    std::fs::write(out, html).map_err(|e| format!("failed to write {}: {e}", out.display()))
}

/// Edge, else Chrome, where they are installed.
pub fn find_browser() -> Option<PathBuf> {
    browser_candidates().into_iter().find(|p| p.is_file())
}

fn browser_candidates() -> Vec<PathBuf> {
    if cfg!(windows) {
        let dirs = ["ProgramFiles(x86)", "ProgramFiles", "LOCALAPPDATA"];
        let apps = [r"Microsoft\Edge\Application\msedge.exe", r"Google\Chrome\Application\chrome.exe"];
        apps.iter()
            .flat_map(|app| dirs.iter().filter_map(move |d| std::env::var_os(d).map(|d| PathBuf::from(d).join(app))))
            .collect()
    } else {
        let names = ["microsoft-edge", "microsoft-edge-stable", "google-chrome", "google-chrome-stable", "chromium"];
        let path = std::env::var_os("PATH").unwrap_or_default();
        names.iter().flat_map(|n| std::env::split_paths(&path).map(move |d| d.join(n)).collect::<Vec<_>>()).collect()
    }
}

/// The browser's arguments printing `html` to `out` without a header and
/// footer (headless, it uses a temporary profile of its own).
fn pdf_args(html: &Path, out: &Path, no_sandbox: bool) -> Vec<String> {
    let mut args = vec![
        "--headless".to_string(),
        "--disable-gpu".into(),
        "--no-first-run".into(),
        "--no-default-browser-check".into(),
        "--no-pdf-header-footer".into(),
        format!("--print-to-pdf={}", out.display()),
    ];
    if no_sandbox {
        args.push("--no-sandbox".into());
    }
    args.push(html.display().to_string());
    args
}

/// Chromium refuses to run as root with its sandbox (Linux, development only).
fn running_as_root() -> bool {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        std::fs::metadata("/proc/self").is_ok_and(|m| m.uid() == 0)
    }
    #[cfg(not(unix))]
    false
}

/// Prints the page as a PDF to `out` with `browser`, without a window.
pub fn export_pdf(browser: &Path, page: &Path, text: &str, out: &Path, css: &str) -> Result<(), String> {
    let fail = |e: &dyn std::fmt::Display| format!("failed to write {}: {e}", out.display());
    let work = tempfile::tempdir().map_err(|e| fail(&e))?;
    let html_file = work.path().join("page.html");
    // Images as absolute URIs: the document is printed from a temporary folder.
    std::fs::write(&html_file, html_document(page, text, None, css)).map_err(|e| fail(&e))?;
    let started = std::time::SystemTime::now();
    let mut command = Command::new(browser);
    command
        .args(pdf_args(&html_file, out, running_as_root()))
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        command.creation_flags(CREATE_NO_WINDOW);
    }
    let mut child = command.spawn().map_err(|e| fail(&e))?;
    let deadline = Instant::now() + PDF_TIMEOUT;
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) if Instant::now() < deadline => std::thread::sleep(Duration::from_millis(100)),
            Ok(None) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(fail(&"the browser did not finish"));
            }
            Err(e) => return Err(fail(&e)),
        }
    }
    let written =
        std::fs::metadata(out).and_then(|m| m.modified()).is_ok_and(|t| t >= started - Duration::from_secs(2));
    if written { Ok(()) } else { Err(fail(&"the browser wrote no PDF")) }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PAGE: &str =
        "# Trip\n\n$$\nE = mc^2\n$$\n\n![map](Trip.assets/map one.png)\n\n![remote](https://x.org/a.png)\n";

    #[test]
    fn exported_html_is_self_contained() {
        let dir = tempfile::tempdir().unwrap();
        let page = dir.path().join("Trip.md");
        let out = dir.path().join("Trip.html");
        export_html(&page, PAGE, &out, "body { color: red }").unwrap();
        let html = std::fs::read_to_string(&out).unwrap();
        assert!(html.contains("<title>Trip</title>"));
        assert!(html.contains("body { color: red }"));
        assert!(html.contains("<h1"));
        assert!(html.contains("<svg"), "math as SVG");
        assert!(html.contains("src=\"Trip.assets/map%20one.png\""), "{html}");
        assert!(html.contains("src=\"https://x.org/a.png\""));
    }

    #[test]
    fn images_are_relative_to_where_the_html_goes() {
        let dir = tempfile::tempdir().unwrap();
        let page = dir.path().join("docs").join("Trip.md");
        let elsewhere = dir.path().join("out");
        let html = html_document(&page, PAGE, Some(&elsewhere), "");
        assert!(html.contains("src=\"../docs/Trip.assets/map%20one.png\""), "{html}");
        // Without a place (printing): absolute URIs.
        let html = html_document(&page, PAGE, None, "");
        assert!(html.contains("src=\"file:/"), "{html}");
    }

    #[test]
    fn the_browser_prints_headless_without_header_and_footer() {
        let args = pdf_args(Path::new("/t/page.html"), Path::new("/o/Trip.pdf"), false);
        assert!(args.contains(&"--headless".to_string()));
        assert!(args.contains(&"--no-pdf-header-footer".to_string()));
        assert!(args.contains(&"--print-to-pdf=/o/Trip.pdf".to_string()));
        assert_eq!(args.last().map(String::as_str), Some("/t/page.html"));
        assert!(!args.contains(&"--no-sandbox".to_string()));
    }
}
