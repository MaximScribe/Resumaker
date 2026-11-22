//! Feedback on generated documents, saved as plain text and reused as style guidance.
//!
//! Each feedback session is one `.txt` file with one point per line (`- …`). Lines
//! starting with `#` are comments. Feedback never counts as experience: it is passed
//! to the model as preferences only.

use std::path::{Path, PathBuf};

use crate::sources::FEEDBACK_DIR;
use crate::{Config, Error};

/// Most recent feedback points passed to the model; older ones are dropped first.
const MAX_POINTS: usize = 40;

/// Where feedback is stored: inside the sources folder if there is one, otherwise in
/// the config directory.
pub fn dir(sources_folder: Option<&Path>) -> Result<PathBuf, Error> {
    match sources_folder {
        Some(folder) => Ok(folder.join(FEEDBACK_DIR)),
        None => Ok(Config::dir()?.join("feedback")),
    }
}

/// A saved feedback file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub path: PathBuf,
    pub points: Vec<String>,
}

/// All feedback files, oldest first (file names start with the date).
pub fn list(dir: &Path) -> Result<Vec<Entry>, Error> {
    let read = match std::fs::read_dir(dir) {
        Ok(r) => r,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(Error::io(dir, e)),
    };
    let mut paths: Vec<PathBuf> = read
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|e| e == "txt") && p.is_file())
        .collect();
    paths.sort_by_cached_key(|p| order_key(p));
    paths
        .into_iter()
        .map(|path| {
            let text = std::fs::read_to_string(&path).map_err(|e| Error::io(&path, e))?;
            Ok(Entry {
                points: parse(&text),
                path,
            })
        })
        .collect()
}

/// The feedback points to pass to the model, most recent last, without duplicates.
pub fn preferences(dir: &Path) -> Result<Vec<String>, Error> {
    let mut points: Vec<String> = Vec::new();
    for entry in list(dir)? {
        for p in entry.points {
            points.retain(|q| !q.eq_ignore_ascii_case(&p));
            points.push(p);
        }
    }
    let skip = points.len().saturating_sub(MAX_POINTS);
    Ok(points.split_off(skip))
}

/// Saves one session's feedback as `<date>_<label>.txt` and returns the path.
pub fn save(
    dir: &Path,
    date: &str,
    label: &str,
    about: &str,
    points: &[String],
) -> Result<PathBuf, Error> {
    std::fs::create_dir_all(dir).map_err(|e| Error::io(dir, e))?;
    let stem = format!("{date}_{label}");
    let mut path = dir.join(format!("{stem}.txt"));
    let mut n = 2;
    while path.exists() {
        path = dir.join(format!("{stem}-{n}.txt"));
        n += 1;
    }
    let mut text = format!("# Feedback on {about}, {date}\n");
    for p in points {
        let p = p.trim();
        if !p.is_empty() {
            text.push_str(&format!("- {}\n", p.replace('\n', " ")));
        }
    }
    std::fs::write(&path, text).map_err(|e| Error::io(&path, e))?;
    Ok(path)
}

/// Deletes all feedback files; returns how many were removed.
pub fn clear(dir: &Path) -> Result<usize, Error> {
    let entries = list(dir)?;
    for e in &entries {
        std::fs::remove_file(&e.path).map_err(|err| Error::io(&e.path, err))?;
    }
    Ok(entries.len())
}

/// Sorts `x.txt` before `x-2.txt` before `x-10.txt`.
fn order_key(path: &Path) -> (String, u32) {
    let stem = path
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();
    match stem.rsplit_once('-') {
        Some((base, n)) if !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()) => {
            (base.to_owned(), n.parse().unwrap_or(u32::MAX))
        }
        _ => (stem, 1),
    }
}

fn parse(text: &str) -> Vec<String> {
    text.lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .map(|l| l.trim_start_matches(['-', '*']).trim().to_owned())
        .filter(|l| !l.is_empty())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("resumaker-fb-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        d
    }

    #[test]
    fn save_list_and_preferences() {
        let dir = temp("save");
        let p1 = save(
            &dir,
            "2026-01-01",
            "acme",
            "the Acme resume",
            &["Shorter summary".into(), " ".into()],
        )
        .unwrap();
        let p2 = save(
            &dir,
            "2026-01-01",
            "acme",
            "the Acme resume",
            &["shorter summary".into(), "Projects first".into()],
        )
        .unwrap();
        assert!(p1.ends_with("2026-01-01_acme.txt"));
        assert!(p2.ends_with("2026-01-01_acme-2.txt"));
        let text = std::fs::read_to_string(&p1).unwrap();
        assert_eq!(
            text,
            "# Feedback on the Acme resume, 2026-01-01\n- Shorter summary\n"
        );
        assert_eq!(list(&dir).unwrap().len(), 2);
        assert_eq!(
            preferences(&dir).unwrap(),
            ["shorter summary", "Projects first"],
            "duplicates collapse to the latest wording"
        );
        assert_eq!(clear(&dir).unwrap(), 2);
        assert!(preferences(&dir).unwrap().is_empty());
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn hand_written_files_are_read() {
        assert_eq!(
            parse("# note\n* Use British English\n\n  plain line  \n-\n"),
            ["Use British English", "plain line"]
        );
    }

    #[test]
    fn missing_dir_is_empty() {
        assert!(list(Path::new("no/such/feedback")).unwrap().is_empty());
    }

    #[test]
    fn dir_prefers_sources_folder() {
        assert_eq!(
            dir(Some(Path::new("career"))).unwrap(),
            Path::new("career").join(FEEDBACK_DIR)
        );
    }
}
