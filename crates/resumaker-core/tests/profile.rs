use std::path::PathBuf;

use resumaker_core::Profile;
use resumaker_core::profile::SourceKind;

fn example_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets/example-profile.json")
}

#[test]
fn example_profile_loads() {
    let p = Profile::load(&example_path()).unwrap();
    assert_eq!(p.basics.name.as_deref(), Some("Alex Morgan"));
    assert_eq!(p.work.len(), 2);
    assert_eq!(p.source("cv-2024").unwrap().kind, SourceKind::File);
}

#[test]
fn example_profile_has_no_dangling_sources() {
    let p = Profile::load(&example_path()).unwrap();
    assert!(
        p.dangling_sources().is_empty(),
        "{:?}",
        p.dangling_sources()
    );
}

#[test]
fn round_trip_preserves_profile() {
    let p = Profile::load(&example_path()).unwrap();
    assert_eq!(Profile::from_json(&p.to_json().unwrap()).unwrap(), p);
}

#[test]
fn save_then_load() {
    let p = Profile::load(&example_path()).unwrap();
    let path = std::env::temp_dir().join(format!("resumaker-test-{}.json", std::process::id()));
    p.save(&path).unwrap();
    let loaded = Profile::load(&path);
    std::fs::remove_file(&path).ok();
    assert_eq!(loaded.unwrap(), p);
}

#[test]
fn reads_standard_json_resume() {
    let p = Profile::from_json(include_str!("fixtures/json-resume.json")).unwrap();
    assert_eq!(p.basics.name.as_deref(), Some("Sam Rivera"));
    assert_eq!(p.volunteer[0].organization.as_deref(), Some("Code Club"));
    assert_eq!(p.publications[0].release_date.as_deref(), Some("2021-11"));
    assert_eq!(p.projects[0].kind.as_deref(), Some("library"));
    assert!(p.sources.is_empty());
    assert_eq!(p.source_refs().count(), 0);
}

#[test]
fn missing_file_reports_path() {
    let err = Profile::load(&PathBuf::from("does/not/exist.json")).unwrap_err();
    assert!(err.to_string().contains("exist.json"), "{err}");
}
