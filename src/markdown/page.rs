//! Page files: UTF-8 with a byte-order mark, as the former add-on wrote them
//! (what Markdown editors on Windows expect). `read_page` / `write_page` take
//! the text as it is; `PageText` is a page as the editor holds it, with LF line
//! endings, remembering the file's line endings to write it back with.

use std::path::Path;

/// The byte-order mark pages start with.
const BOM: &str = "\u{feff}";

/// `text` without the byte-order mark it may start with.
pub fn strip_bom(text: &str) -> &str {
    text.strip_prefix(BOM).unwrap_or(text)
}

/// Reads a page as UTF-8, without the byte-order mark `write_page` puts in front.
pub fn read_page(path: &Path) -> std::io::Result<String> {
    let bytes = std::fs::read(path)?;
    let text = String::from_utf8(bytes).map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    Ok(text.strip_prefix(BOM).map(str::to_string).unwrap_or(text))
}

/// Writes a page as UTF-8 with a byte-order mark.
pub fn write_page(path: &Path, text: &str) -> std::io::Result<()> {
    std::fs::write(path, format!("{BOM}{text}"))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PageText {
    /// With LF line endings.
    pub text: String,
    /// Whether the file used CRLF line endings (as its first line break does).
    pub crlf: bool,
}

impl PageText {
    /// The text of a file's bytes; an error when it is not UTF-8.
    pub fn decode(bytes: &[u8]) -> Result<PageText, String> {
        let text = std::str::from_utf8(bytes).map_err(|_| "not a UTF-8 text file".to_string())?;
        let text = strip_bom(text);
        let crlf = text.find('\n').is_some_and(|i| i > 0 && text.as_bytes()[i - 1] == b'\r');
        let text = if crlf { text.replace("\r\n", "\n") } else { text.to_string() };
        Ok(PageText { text, crlf })
    }

    /// The text with the file's line endings (without the byte-order mark).
    pub fn encoded_text(&self) -> String {
        if self.crlf { self.text.replace('\n', "\r\n") } else { self.text.clone() }
    }

    /// Reads a file; the error is the message shown to the user.
    pub fn read(path: &Path) -> Result<PageText, String> {
        let error = |e: String| format!("failed to open {}: {e}", path.display());
        let bytes = std::fs::read(path).map_err(|e| error(e.to_string()))?;
        PageText::decode(&bytes).map_err(error)
    }

    /// Writes the file as UTF-8 with a byte-order mark, as pages are written.
    pub fn write(&self, path: &Path) -> Result<(), String> {
        write_page(path, &self.encoded_text()).map_err(|e| format!("failed to save {}: {e}", path.display()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crlf_and_bom_round_trip_with_one_word_changed() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("p.md");
        let original = "\u{feff}# Title\r\n\r\nsome words here\r\n- item\r\n";
        std::fs::write(&path, original).unwrap();
        let mut page = PageText::read(&path).unwrap();
        assert!(page.crlf);
        assert_eq!(page.text, "# Title\n\nsome words here\n- item\n");
        page.text = page.text.replace("words", "WORDS");
        page.write(&path).unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), original.replace("words", "WORDS"));
    }

    #[test]
    fn lf_without_bom_is_read_and_written_with_a_bom() {
        let page = PageText::decode(b"a\nb\n").unwrap();
        assert!(!page.crlf);
        assert_eq!(page.encoded_text(), "a\nb\n");
    }

    #[test]
    fn not_utf8() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("sjis.md");
        // "日本" in Shift_JIS.
        std::fs::write(&path, [0x93, 0xfa, 0x96, 0x7b]).unwrap();
        let e = PageText::read(&path).unwrap_err();
        assert!(e.starts_with("failed to open ") && e.contains("sjis.md"), "{e}");
    }
}
