use std::sync::Mutex;

use resumaker_core::export::Options;
use resumaker_core::llm::{Llm, Prompt};
use resumaker_core::pipeline::{self, TailorRequest};
use resumaker_core::profile::{Source, SourceKind};
use resumaker_core::sources::Document;
use resumaker_core::{Error, Format, Profile, ProviderKind};

/// Answers with canned replies and records the prompts it was given.
struct FakeLlm {
    replies: Mutex<Vec<String>>,
    prompts: Mutex<Vec<Prompt>>,
}

impl FakeLlm {
    fn new(replies: &[&str]) -> Self {
        Self {
            replies: Mutex::new(replies.iter().rev().map(|s| s.to_string()).collect()),
            prompts: Mutex::default(),
        }
    }

    fn calls(&self) -> usize {
        self.prompts.lock().unwrap().len()
    }
}

impl Llm for FakeLlm {
    fn complete(&self, prompt: &Prompt) -> Result<String, Error> {
        self.prompts.lock().unwrap().push(prompt.clone());
        self.replies
            .lock()
            .unwrap()
            .pop()
            .ok_or_else(|| Error::Llm("no reply".into()))
    }

    fn list_models(&self) -> Result<Vec<String>, Error> {
        Ok(vec![])
    }

    fn provider(&self) -> ProviderKind {
        ProviderKind::Ollama
    }
}

fn doc(id: &str, text: &str) -> Document {
    Document {
        source: Source {
            id: id.into(),
            kind: SourceKind::File,
            location: format!("{id}.txt"),
        },
        text: text.into(),
        truncated: false,
    }
}

const MERGED: &str = r#"{
  "basics": { "name": "Alex Morgan" },
  "work": [
    { "name": "Northwind", "position": "Engineer", "startDate": "2022-04",
      "highlights": ["Rewrote the planner"], "source": "cv" },
    { "name": "Ghost Corp", "position": "CEO", "source": "made-up" }
  ],
  "skills": [{ "name": "Languages", "keywords": ["Rust"], "source": "notes" }]
}"#;

#[test]
fn merge_tags_sources_and_drops_unknown_ids() {
    let llm = FakeLlm::new(&[MERGED]);
    let docs = [
        doc("cv", "Engineer at Northwind since 2022"),
        doc("notes", "Rust"),
    ];
    let profile = pipeline::merge(&llm, "m", &docs, None).unwrap();
    assert_eq!(profile.sources.len(), 2);
    assert_eq!(profile.work[0].source.as_deref(), Some("cv"));
    assert_eq!(profile.work[1].source, None, "unknown source id is cleared");
    let prompt = &llm.prompts.lock().unwrap()[0];
    assert!(prompt.user.contains("<source id=\"cv\">"));
    assert!(prompt.schema.is_some());
}

#[test]
fn merge_uses_the_cache() {
    let dir = std::env::temp_dir().join(format!("resumaker-cache-{}", std::process::id()));
    let docs = [doc("cv", "text")];
    let llm = FakeLlm::new(&[MERGED]);
    let first = pipeline::merge(&llm, "m", &docs, Some(&dir)).unwrap();
    let second = pipeline::merge(&llm, "m", &docs, Some(&dir)).unwrap();
    assert_eq!(first, second);
    assert_eq!(llm.calls(), 1, "second merge is served from the cache");

    let changed = [doc("cv", "different text")];
    let llm2 = FakeLlm::new(&[MERGED]);
    pipeline::merge(&llm2, "m", &changed, Some(&dir)).unwrap();
    assert_eq!(llm2.calls(), 1, "changed sources are merged again");
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn merge_without_sources_fails() {
    let llm = FakeLlm::new(&[]);
    assert!(matches!(
        pipeline::merge(&llm, "m", &[], None),
        Err(Error::NoSources)
    ));
}

#[test]
fn tailor_builds_prompt_and_reads_target() {
    let llm = FakeLlm::new(&[MERGED]);
    let profile = pipeline::merge(&llm, "m", &[doc("cv", "x"), doc("notes", "y")], None).unwrap();

    let answer = r#"{
      "target": { "company": "Acme Robotics", "role": "Robotics Engineer" },
      "resume": {
        "basics": { "name": "Alex Morgan", "label": "Robotics Engineer" },
        "work": [{ "name": "Northwind", "position": "Engineer", "source": "cv" }]
      }
    }"#;
    let llm = FakeLlm::new(&[answer]);
    let req = TailorRequest {
        job: vec!["Robotics Engineer at Acme, ROS 2".into()],
        preferences: vec!["Keep it short".into()],
        lang: "de".into(),
        pages: 1,
    };
    let tailored = pipeline::tailor(&llm, &profile, &req).unwrap();
    assert_eq!(tailored.target.company.as_deref(), Some("Acme Robotics"));
    assert_eq!(tailored.profile.sources, profile.sources);
    assert_eq!(tailored.profile.dangling_sources(), Vec::<&str>::new());

    let prompt = &llm.prompts.lock().unwrap()[0];
    assert!(prompt.system.contains("German"));
    assert!(prompt.system.contains("1 page"));
    assert!(prompt.user.contains("<job>\nRobotics Engineer at Acme"));
    assert!(prompt.user.contains("- Keep it short"));
    assert!(
        !prompt.user.contains("\"sources\""),
        "source paths stay private"
    );
}

#[test]
fn writes_every_format_into_an_application_folder() {
    let base = std::env::temp_dir().join(format!("resumaker-out-{}", std::process::id()));
    let target = pipeline::Target {
        company: Some("Acme".into()),
        role: None,
    };
    let dir = pipeline::application_dir(&base, &target, false);
    let files = pipeline::write_outputs(
        &Profile::example(),
        &[Format::Pdf, Format::Md],
        &Options::default(),
        &dir,
        "resume",
    )
    .unwrap();
    assert_eq!(files.len(), 2);
    assert!(files.iter().all(|f| f.is_file()));
    let next = pipeline::application_dir(&base, &target, false);
    assert!(next.to_string_lossy().ends_with("_acme-2"));
    std::fs::remove_dir_all(&base).ok();
}
