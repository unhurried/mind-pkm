//! The system clipboard with HTML, images and file lists (iced's clipboard
//! holds text only), behind a trait so that tests use a fake one.

use std::path::PathBuf;

/// What the clipboard holds.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ClipboardContent {
    pub text: Option<String>,
    pub html: Option<String>,
    pub image: Option<PastedImage>,
    pub files: Vec<PathBuf>,
}

/// Image data, RGBA.
#[derive(Debug, Clone, PartialEq)]
pub struct PastedImage {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

pub trait ClipboardBackend: Send {
    /// Puts text on the clipboard, with HTML as its rich form when given.
    fn write(&mut self, text: &str, html: Option<&str>);
    fn read(&mut self) -> ClipboardContent;
}

/// The platform's clipboard, opened on first use (and kept open: on X11 the
/// copied data is served by it).
#[derive(Default)]
pub struct SystemClipboard(Option<arboard::Clipboard>);

impl SystemClipboard {
    fn clipboard(&mut self) -> Option<&mut arboard::Clipboard> {
        if self.0.is_none() {
            self.0 = arboard::Clipboard::new().ok();
        }
        self.0.as_mut()
    }
}

impl ClipboardBackend for SystemClipboard {
    fn write(&mut self, text: &str, html: Option<&str>) {
        let Some(c) = self.clipboard() else { return };
        let _ = match html {
            Some(html) => c.set_html(html, Some(text)),
            None => c.set_text(text),
        };
    }

    fn read(&mut self) -> ClipboardContent {
        let Some(c) = self.clipboard() else { return ClipboardContent::default() };
        let files = c.get().file_list().unwrap_or_default();
        let image = c.get_image().ok().map(|i| PastedImage {
            width: i.width as u32,
            height: i.height as u32,
            rgba: i.bytes.into_owned(),
        });
        ClipboardContent { text: c.get_text().ok(), html: c.get().html().ok(), image, files }
    }
}

/// A clipboard in memory, for tests.
#[derive(Default)]
pub struct MemoryClipboard {
    pub content: ClipboardContent,
}

impl ClipboardBackend for MemoryClipboard {
    fn write(&mut self, text: &str, html: Option<&str>) {
        self.content =
            ClipboardContent { text: Some(text.to_string()), html: html.map(str::to_string), ..Default::default() };
    }

    fn read(&mut self) -> ClipboardContent {
        self.content.clone()
    }
}

/// The clipboard the application uses: the system's, or one in memory in tests.
pub fn default_backend() -> Box<dyn ClipboardBackend> {
    if cfg!(test) { Box::new(MemoryClipboard::default()) } else { Box::new(SystemClipboard::default()) }
}
