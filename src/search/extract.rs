//! Searchable text of a file by extension: Markdown/plain text directly, PDF,
//! and the current-format Office documents (docx/xlsx/pptx). Any other
//! extension, and any file that fails to parse, yields "" rather than an
//! error, so one bad file only costs its own content.

use quick_xml::Reader;
use quick_xml::events::Event;
use std::io::Read;
use std::path::Path;

// Keeps one pathological file (e.g. a huge PDF) from blowing up the index.
pub const MAX_CONTENT_CHARS: usize = 2_000_000;

const TEXT_EXTENSIONS: &[&str] = &["md", "markdown", "txt"];

pub fn extension_of(path: &Path) -> String {
    path.extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default()
}

pub fn extract_text(path: &Path) -> String {
    let ext = extension_of(path);
    let text = if TEXT_EXTENSIONS.contains(&ext.as_str()) {
        std::fs::read(path)
            .map(|b| {
                let s = String::from_utf8_lossy(&b).into_owned();
                s.strip_prefix('\u{feff}').map(str::to_string).unwrap_or(s)
            })
            .unwrap_or_default()
    } else if ext == "pdf" {
        read_pdf(path)
    } else if ext == "docx" {
        read_ooxml(
            path,
            |name| name == "word/document.xml" || name.starts_with("word/header") || name.starts_with("word/footer"),
            &["w:t"],
            &["w:p", "w:tab", "w:br"],
        )
    } else if ext == "xlsx" {
        read_ooxml(
            path,
            |name| {
                name == "xl/sharedStrings.xml"
                    || name == "xl/workbook.xml"
                    || (name.starts_with("xl/worksheets/") && name.ends_with(".xml"))
            },
            &["t", "v"],
            &["si", "c", "row", "sheet"],
        )
    } else if ext == "pptx" {
        read_ooxml(
            path,
            |name| {
                (name.starts_with("ppt/slides/slide") || name.starts_with("ppt/notesSlides/")) && name.ends_with(".xml")
            },
            &["a:t"],
            &["a:p"],
        )
    } else {
        String::new()
    };
    truncate_chars(text, MAX_CONTENT_CHARS)
}

pub fn truncate_chars(s: String, max: usize) -> String {
    match s.char_indices().nth(max) {
        Some((i, _)) => s[..i].to_string(),
        None => s,
    }
}

fn read_pdf(path: &Path) -> String {
    let Ok(bytes) = std::fs::read(path) else { return String::new() };
    // The PDF parser panics on some malformed files; a panic costs only this file.
    std::panic::catch_unwind(move || pdf_extract::extract_text_from_mem(&bytes).unwrap_or_default()).unwrap_or_default()
}

/// Text of the parts of an Office Open XML package: the content of the
/// `text_elements`, with a line break after each of the `break_elements`.
fn read_ooxml(path: &Path, wanted: impl Fn(&str) -> bool, text_elements: &[&str], break_elements: &[&str]) -> String {
    let Ok(file) = std::fs::File::open(path) else { return String::new() };
    let Ok(mut archive) = zip::ZipArchive::new(file) else { return String::new() };
    let mut names: Vec<String> = archive.file_names().filter(|n| wanted(n)).map(str::to_string).collect();
    // slide2 before slide10.
    names.sort_by_key(|n| {
        let digits: String = n.chars().filter(char::is_ascii_digit).collect();
        let rest: String = n.chars().filter(|c| !c.is_ascii_digit()).collect();
        (rest, digits.parse::<u64>().unwrap_or(0))
    });
    let mut out = String::new();
    for name in names {
        let mut xml = String::new();
        let Ok(mut entry) = archive.by_name(&name) else { continue };
        if entry.read_to_string(&mut xml).is_err() {
            continue;
        }
        collect_xml_text(&xml, text_elements, break_elements, &mut out);
        out.push('\n');
        if out.len() > MAX_CONTENT_CHARS * 4 {
            break;
        }
    }
    out
}

fn collect_xml_text(xml: &str, text_elements: &[&str], break_elements: &[&str], out: &mut String) {
    let mut reader = Reader::from_str(xml);
    let mut depth_in_text = 0usize;
    loop {
        match reader.read_event() {
            Ok(Event::Start(e)) => {
                if text_elements.contains(&String::from_utf8_lossy(e.name().as_ref()).as_ref()) {
                    depth_in_text += 1;
                }
            }
            Ok(Event::End(e)) => {
                let name = String::from_utf8_lossy(e.name().as_ref()).into_owned();
                if text_elements.contains(&name.as_str()) {
                    depth_in_text = depth_in_text.saturating_sub(1);
                    if !out.ends_with([' ', '\n']) {
                        out.push(' ');
                    }
                }
                if break_elements.contains(&name.as_str()) && !out.ends_with('\n') {
                    out.push('\n');
                }
            }
            Ok(Event::Empty(e)) => {
                let name = String::from_utf8_lossy(e.name().as_ref()).into_owned();
                if break_elements.contains(&name.as_str()) && !out.ends_with('\n') {
                    out.push('\n');
                }
            }
            Ok(Event::Text(t)) if depth_in_text > 0 => {
                out.push_str(&crate::xml::unescape(&String::from_utf8_lossy(&t)));
            }
            Ok(Event::GeneralRef(r)) if depth_in_text > 0 => {
                out.push_str(&crate::xml::unescape(&format!("&{};", String::from_utf8_lossy(&r))));
            }
            Ok(Event::Eof) | Err(_) => break,
            _ => {}
        }
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use std::io::Write;

    /// A minimal Office Open XML package holding the given parts.
    pub fn write_zip(path: &Path, parts: &[(&str, &str)]) {
        let file = std::fs::File::create(path).unwrap();
        let mut zip = zip::ZipWriter::new(file);
        for (name, content) in parts {
            zip.start_file(*name, zip::write::SimpleFileOptions::default()).unwrap();
            zip.write_all(content.as_bytes()).unwrap();
        }
        zip.finish().unwrap();
    }

    #[test]
    fn extracts_office_documents() {
        let dir = tempfile::tempdir().unwrap();
        let docx = dir.path().join("a.docx");
        write_zip(
            &docx,
            &[(
                "word/document.xml",
                "<w:document><w:body><w:p><w:r><w:t>Hello</w:t></w:r><w:r><w:t>日本語&amp;</w:t></w:r></w:p><w:p><w:r><w:t>Next</w:t></w:r></w:p></w:body></w:document>",
            )],
        );
        let text = extract_text(&docx);
        assert!(text.contains("Hello") && text.contains("日本語&") && text.contains("Next"), "{text}");

        let xlsx = dir.path().join("b.xlsx");
        write_zip(
            &xlsx,
            &[
                ("xl/sharedStrings.xml", "<sst><si><t>Budget</t></si></sst>"),
                (
                    "xl/worksheets/sheet1.xml",
                    "<worksheet><sheetData><row><c><v>42</v></c></row></sheetData></worksheet>",
                ),
            ],
        );
        let text = extract_text(&xlsx);
        assert!(text.contains("Budget") && text.contains("42"), "{text}");

        let pptx = dir.path().join("c.pptx");
        write_zip(&pptx, &[("ppt/slides/slide1.xml", "<p:sld><a:p><a:r><a:t>Slide text</a:t></a:r></a:p></p:sld>")]);
        assert!(extract_text(&pptx).contains("Slide text"));
    }

    #[test]
    fn broken_files_yield_nothing() {
        let dir = tempfile::tempdir().unwrap();
        let bad = dir.path().join("bad.pdf");
        std::fs::write(&bad, b"not a pdf").unwrap();
        assert_eq!(extract_text(&bad), "");
        let bad = dir.path().join("bad.docx");
        std::fs::write(&bad, b"not a zip").unwrap();
        assert_eq!(extract_text(&bad), "");
        let other = dir.path().join("x.bin");
        std::fs::write(&other, b"abc").unwrap();
        assert_eq!(extract_text(&other), "");
    }

    #[test]
    fn markdown_without_bom() {
        let dir = tempfile::tempdir().unwrap();
        let md = dir.path().join("a.md");
        std::fs::write(&md, "\u{feff}# Title").unwrap();
        assert_eq!(extract_text(&md), "# Title");
    }
}
