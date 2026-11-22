//! Every template in the downloadable pack must render the example profile.

use std::path::PathBuf;

use resumaker_core::export::{self, Lang, Options};
use resumaker_core::{Format, Profile, Template};

fn pack_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../templates")
}

#[test]
fn index_lists_every_template_file() {
    let index: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(pack_dir().join("index.json")).unwrap())
            .unwrap();
    let listed: Vec<&str> = index["templates"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["file"].as_str().unwrap())
        .collect();
    let mut on_disk: Vec<String> = std::fs::read_dir(pack_dir())
        .unwrap()
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|n| n.ends_with(".typ"))
        .collect();
    on_disk.sort();
    let mut listed_sorted: Vec<String> = listed.iter().map(|s| s.to_string()).collect();
    listed_sorted.sort();
    assert_eq!(listed_sorted, on_disk);
}

#[test]
fn pack_templates_render() {
    let full = Profile::example();
    let empty = Profile::default();
    for entry in std::fs::read_dir(pack_dir()).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().is_none_or(|e| e != "typ") {
            continue;
        }
        let template = Template::from_file(&path).unwrap();
        for lang in [Lang::En, Lang::De] {
            for profile in [&full, &empty] {
                let options = Options {
                    lang,
                    template: template.clone(),
                    pages: Some(1),
                };
                let pdf = export::render(profile, Format::Pdf, &options)
                    .unwrap_or_else(|e| panic!("{}: {e}", path.display()));
                assert!(pdf.starts_with(b"%PDF-"));
            }
        }
    }
}
