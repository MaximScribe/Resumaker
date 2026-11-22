//! Reading experience from files.

pub mod extract;

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use walkdir::WalkDir;

pub use extract::{FileKind, SUPPORTED_EXTENSIONS};

use crate::Error;
use crate::profile::{Source, SourceKind};

/// Folder (inside the sources folder) where feedback is saved. It is read separately
/// as style guidance, never as experience.
pub const FEEDBACK_DIR: &str = "resumaker-feedback";

/// Files larger than this are skipped; they are almost never hand-written CVs.
const MAX_FILE_BYTES: u64 = 25 * 1024 * 1024;

/// Text longer than this is cut so a single file cannot crowd out the rest.
const MAX_CHARS_PER_FILE: usize = 60_000;

/// One source's extracted text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Document {
    pub source: Source,
    pub text: String,
    /// The text was cut at [`MAX_CHARS_PER_FILE`].
    pub truncated: bool,
}

/// The result of reading a folder or a list of files.
#[derive(Debug, Default)]
pub struct Collection {
    pub documents: Vec<Document>,
    /// Files that were found but not read, with the reason.
    pub skipped: Vec<(PathBuf, String)>,
    ids: HashSet<String>,
}

impl Collection {
    /// Reads every supported file under `folder`, recursively, in a stable order.
    /// Hidden files and folders and the feedback folder are ignored.
    pub fn add_folder(&mut self, folder: &Path) -> Result<(), Error> {
        if !folder.is_dir() {
            return Err(Error::io(
                folder,
                std::io::Error::new(std::io::ErrorKind::NotFound, "not a folder"),
            ));
        }
        let walker = WalkDir::new(folder)
            .follow_links(true)
            .sort_by_file_name()
            .into_iter()
            .filter_entry(|e| {
                e.depth() == 0 || !(is_hidden(e.file_name()) || e.file_name() == FEEDBACK_DIR)
            });
        for entry in walker {
            let entry = match entry {
                Ok(e) => e,
                Err(e) => {
                    let path = e.path().map(Path::to_path_buf).unwrap_or_default();
                    self.skipped.push((path, e.to_string()));
                    continue;
                }
            };
            if !entry.file_type().is_file() {
                continue;
            }
            let path = entry.path();
            if FileKind::from_path(path).is_none() {
                continue;
            }
            let label = path.strip_prefix(folder).unwrap_or(path);
            self.add_file_as(path, label);
        }
        Ok(())
    }

    /// Reads one file. Unsupported or unreadable files are recorded in `skipped`.
    pub fn add_file(&mut self, path: &Path) {
        let label = path.file_name().map(Path::new).unwrap_or(path);
        self.add_file_as(path, label);
    }

    fn add_file_as(&mut self, path: &Path, label: &Path) {
        match self.read(path, label) {
            Ok(Some(doc)) => {
                self.ids.insert(doc.source.id.clone());
                self.documents.push(doc);
            }
            Ok(None) => {}
            Err(e) => self.skipped.push((path.to_path_buf(), e.to_string())),
        }
    }

    fn read(&self, path: &Path, label: &Path) -> Result<Option<Document>, Error> {
        let kind = FileKind::from_path(path).ok_or_else(|| Error::Unreadable {
            path: path.to_path_buf(),
            reason: format!(
                "unsupported file type (supported: {})",
                SUPPORTED_EXTENSIONS.join(", ")
            ),
        })?;
        let size = std::fs::metadata(path)
            .map_err(|e| Error::io(path, e))?
            .len();
        if size > MAX_FILE_BYTES {
            return Err(Error::Unreadable {
                path: path.to_path_buf(),
                reason: format!("larger than {} MB", MAX_FILE_BYTES / 1024 / 1024),
            });
        }
        let text = extract::extract(path, kind)?;
        if text.is_empty() {
            return Err(Error::Unreadable {
                path: path.to_path_buf(),
                reason: "no text found (scanned PDFs need OCR first)".into(),
            });
        }
        let (text, truncated) = truncate(text, MAX_CHARS_PER_FILE);
        Ok(Some(Document {
            source: Source {
                id: self.unique_id(label),
                kind: SourceKind::File,
                location: path.to_string_lossy().into_owned(),
            },
            text,
            truncated,
        }))
    }

    /// A short, readable id from the file's path relative to its folder:
    /// `jobs/CV 2024.pdf` becomes `jobs-cv-2024`.
    fn unique_id(&self, label: &Path) -> String {
        let stem = label.with_extension("");
        let base = slug(&stem.to_string_lossy());
        let base = if base.is_empty() {
            "source".into()
        } else {
            base
        };
        let mut id = base.clone();
        let mut n = 2;
        while self.ids.contains(&id) {
            id = format!("{base}-{n}");
            n += 1;
        }
        id
    }

    pub fn is_empty(&self) -> bool {
        self.documents.is_empty()
    }

    /// Total characters of extracted text.
    pub fn total_chars(&self) -> usize {
        self.documents.iter().map(|d| d.text.chars().count()).sum()
    }
}

fn is_hidden(name: &std::ffi::OsStr) -> bool {
    name.to_string_lossy().starts_with('.')
}

fn slug(s: &str) -> String {
    let mut out = String::new();
    for c in s.chars() {
        if c.is_alphanumeric() {
            out.extend(c.to_lowercase());
        } else if !out.ends_with('-') && !out.is_empty() {
            out.push('-');
        }
    }
    out.trim_end_matches('-').to_owned()
}

fn truncate(text: String, max: usize) -> (String, bool) {
    match text.char_indices().nth(max) {
        Some((byte, _)) => (text[..byte].to_owned(), true),
        None => (text, false),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slugs() {
        assert_eq!(slug("jobs/CV 2024 (final)"), "jobs-cv-2024-final");
        assert_eq!(slug("Lebenslauf_Müller"), "lebenslauf-müller");
        assert_eq!(slug("***"), "");
    }

    #[test]
    fn truncates_on_char_boundary() {
        assert_eq!(truncate("äöü".into(), 2), ("äö".into(), true));
        assert_eq!(truncate("ab".into(), 5), ("ab".into(), false));
    }
}
