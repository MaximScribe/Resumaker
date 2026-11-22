use std::path::PathBuf;

use resumaker_core::export::{self, Options};
use resumaker_core::sources::{Collection, FEEDBACK_DIR};
use resumaker_core::{Format, Profile};

struct TempDir(PathBuf);

impl TempDir {
    fn new(name: &str) -> Self {
        let dir = std::env::temp_dir().join(format!("resumaker-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        Self(dir)
    }

    fn write(&self, rel: &str, contents: impl AsRef<[u8]>) -> PathBuf {
        let path = self.0.join(rel);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, contents).unwrap();
        path
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn exported(format: Format) -> Vec<u8> {
    export::render(&Profile::example(), format, &Options::default()).unwrap()
}

#[test]
fn reads_a_folder_of_mixed_files() {
    let dir = TempDir::new("folder");
    dir.write("notes.md", "# Projects\n\n- Built tinyslam\n");
    dir.write("jobs/2020 Contoso.txt", "Backend developer at Contoso");
    dir.write("data.json", r#"{"certificate": "AWS"}"#);
    dir.write("skills.csv", "skill,level\nRust,expert\n");
    dir.write("cv.pdf", exported(Format::Pdf));
    dir.write("cv.docx", exported(Format::Docx));
    dir.write("photo.jpg", [0xff, 0xd8, 0xff]);
    dir.write(".hidden/secret.txt", "should be ignored");
    dir.write(&format!("{FEEDBACK_DIR}/2026-01-01.txt"), "make it shorter");
    dir.write("empty.txt", "   \n");

    let mut c = Collection::default();
    c.add_folder(&dir.0).unwrap();

    let ids: Vec<&str> = c.documents.iter().map(|d| d.source.id.as_str()).collect();
    assert_eq!(
        ids,
        ["cv", "cv-2", "data", "jobs-2020-contoso", "notes", "skills"]
    );
    let text = |id: &str| {
        c.documents
            .iter()
            .find(|d| d.source.id == id)
            .unwrap()
            .text
            .clone()
    };
    assert!(text("cv").contains("Northwind Robotics"), "pdf text");
    assert!(
        text("cv-2").contains("Rewrote the path planner"),
        "docx text"
    );
    assert!(text("skills").contains("Rust,expert"));
    assert!(c.documents.iter().all(|d| !d.text.contains("ignored")));
    assert!(c.documents.iter().all(|d| !d.text.contains("shorter")));
    assert_eq!(c.skipped.len(), 1, "{:?}", c.skipped);
    assert!(c.skipped[0].0.ends_with("empty.txt"));
}

#[test]
fn add_file_reports_unsupported_and_corrupt_files() {
    let dir = TempDir::new("files");
    let jpg = dir.write("photo.jpg", [0xff, 0xd8]);
    let pdf = dir.write("broken.pdf", "not really a pdf");
    let missing = dir.0.join("missing.txt");
    let mut c = Collection::default();
    c.add_file(&jpg);
    c.add_file(&pdf);
    c.add_file(&missing);
    assert!(c.is_empty());
    assert_eq!(c.skipped.len(), 3);
    assert!(c.skipped[0].1.contains("unsupported"));
}

#[test]
fn missing_folder_is_an_error() {
    let mut c = Collection::default();
    assert!(c.add_folder(&PathBuf::from("definitely/not/here")).is_err());
}
