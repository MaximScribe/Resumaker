//! Plain-text extraction from the file types Resumaker reads.

use std::io::Read;
use std::path::Path;

use calamine::{Reader, open_workbook_auto};
use quick_xml::Reader as XmlReader;
use quick_xml::events::Event;

use crate::Error;

/// File types that can be read as experience sources.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileKind {
    Text,
    Markdown,
    Json,
    Csv,
    Pdf,
    Docx,
    Spreadsheet,
}

impl FileKind {
    pub fn from_path(path: &Path) -> Option<Self> {
        let ext = path.extension()?.to_str()?.to_ascii_lowercase();
        Some(match ext.as_str() {
            "txt" | "text" => Self::Text,
            "md" | "markdown" => Self::Markdown,
            "json" => Self::Json,
            "csv" | "tsv" => Self::Csv,
            "pdf" => Self::Pdf,
            "docx" => Self::Docx,
            "xlsx" | "xlsm" | "xls" | "ods" => Self::Spreadsheet,
            _ => return None,
        })
    }
}

/// Extensions accepted by [`FileKind::from_path`].
pub const SUPPORTED_EXTENSIONS: &[&str] = &[
    "txt", "md", "json", "csv", "tsv", "pdf", "docx", "xlsx", "xlsm", "xls", "ods",
];

pub fn extract(path: &Path, kind: FileKind) -> Result<String, Error> {
    let text = match kind {
        FileKind::Text | FileKind::Markdown | FileKind::Json | FileKind::Csv => {
            let bytes = std::fs::read(path).map_err(|e| Error::io(path, e))?;
            String::from_utf8_lossy(&bytes).into_owned()
        }
        FileKind::Pdf => pdf(path)?,
        FileKind::Docx => docx(path)?,
        FileKind::Spreadsheet => spreadsheet(path)?,
    };
    Ok(normalize(&text))
}

fn pdf(path: &Path) -> Result<String, Error> {
    let bytes = std::fs::read(path).map_err(|e| Error::io(path, e))?;
    // pdf-extract can panic on malformed files; treat that as an unreadable file.
    std::panic::catch_unwind(|| pdf_extract::extract_text_from_mem(&bytes))
        .map_err(|_| unreadable(path, "the PDF could not be parsed"))?
        .map_err(|e| unreadable(path, &e.to_string()))
}

fn docx(path: &Path) -> Result<String, Error> {
    let file = std::fs::File::open(path).map_err(|e| Error::io(path, e))?;
    let mut archive = zip::ZipArchive::new(file).map_err(|e| unreadable(path, &e.to_string()))?;
    let mut xml = String::new();
    archive
        .by_name("word/document.xml")
        .map_err(|e| unreadable(path, &e.to_string()))?
        .read_to_string(&mut xml)
        .map_err(|e| Error::io(path, e))?;
    docx_text(&xml).map_err(|e| unreadable(path, &e))
}

/// Text of a WordprocessingML body: one line per paragraph, tabs and breaks kept.
fn docx_text(xml: &str) -> Result<String, String> {
    let mut reader = XmlReader::from_str(xml);
    let mut out = String::new();
    let mut in_text = false;
    loop {
        match reader.read_event().map_err(|e| e.to_string())? {
            Event::Start(e) if e.local_name().as_ref() == b"t" => in_text = true,
            Event::End(e) => match e.local_name().as_ref() {
                b"t" => in_text = false,
                b"p" => out.push('\n'),
                _ => {}
            },
            Event::Empty(e) => match e.local_name().as_ref() {
                b"tab" => out.push('\t'),
                b"br" | b"cr" => out.push('\n'),
                _ => {}
            },
            Event::Text(t) if in_text => {
                out.push_str(&t.decode().map_err(|e| e.to_string())?);
            }
            Event::GeneralRef(r) if in_text => {
                let name = r.decode().map_err(|e| e.to_string())?;
                match name.as_ref() {
                    "amp" => out.push('&'),
                    "lt" => out.push('<'),
                    "gt" => out.push('>'),
                    "quot" => out.push('"'),
                    "apos" => out.push('\''),
                    other => {
                        if let Some(c) = r.resolve_char_ref().ok().flatten() {
                            out.push(c);
                        } else {
                            out.push_str(&format!("&{other};"));
                        }
                    }
                }
            }
            Event::Eof => break,
            _ => {}
        }
    }
    Ok(out)
}

fn spreadsheet(path: &Path) -> Result<String, Error> {
    let mut workbook = open_workbook_auto(path).map_err(|e| unreadable(path, &e.to_string()))?;
    let mut out = String::new();
    for name in workbook.sheet_names() {
        let range = workbook
            .worksheet_range(&name)
            .map_err(|e| unreadable(path, &e.to_string()))?;
        if range.is_empty() {
            continue;
        }
        out.push_str(&format!("## Sheet: {name}\n"));
        for row in range.rows() {
            let cells: Vec<String> = row.iter().map(|c| c.to_string()).collect();
            if cells.iter().any(|c| !c.trim().is_empty()) {
                out.push_str(&cells.join(" | "));
                out.push('\n');
            }
        }
        out.push('\n');
    }
    Ok(out)
}

fn unreadable(path: &Path, reason: &str) -> Error {
    Error::Unreadable {
        path: path.to_path_buf(),
        reason: reason.to_owned(),
    }
}

/// Trims lines, collapses runs of blank lines and removes control characters.
fn normalize(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut blank = 0;
    for line in text.lines() {
        let line: String = line
            .chars()
            .filter(|c| !c.is_control() || *c == '\t')
            .collect();
        let line = line.trim_end();
        if line.trim().is_empty() {
            blank += 1;
            if blank > 1 {
                continue;
            }
        } else {
            blank = 0;
        }
        out.push_str(line);
        out.push('\n');
    }
    out.trim().to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_kinds() {
        assert_eq!(
            FileKind::from_path(Path::new("a/CV.PDF")),
            Some(FileKind::Pdf)
        );
        assert_eq!(
            FileKind::from_path(Path::new("x.xlsx")),
            Some(FileKind::Spreadsheet)
        );
        assert_eq!(FileKind::from_path(Path::new("photo.jpg")), None);
        assert_eq!(FileKind::from_path(Path::new("README")), None);
    }

    #[test]
    fn extracts_docx_paragraphs() {
        let xml = r#"<w:document xmlns:w="x"><w:body>
            <w:p><w:r><w:t>Senior &amp; Lead</w:t></w:r><w:r><w:tab/><w:t xml:space="preserve"> 2020</w:t></w:r></w:p>
            <w:p><w:r><w:t>Second</w:t><w:br/><w:t>line</w:t></w:r></w:p>
            <w:p><w:r><w:instrText>IGNORED</w:instrText></w:r></w:p>
        </w:body></w:document>"#;
        assert_eq!(
            docx_text(xml).unwrap(),
            "Senior & Lead\t 2020\nSecond\nline\n\n"
        );
    }

    #[test]
    fn normalizes_whitespace() {
        assert_eq!(normalize("  a  \n\n\n\nb\u{7}\n\n"), "a\n\nb");
    }
}
