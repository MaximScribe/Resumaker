//! Reading experience from HTTP APIs.

use serde_json::Value;

use super::adapters;
use crate::config::ApiSource;
use crate::llm::http::{Headers, Transport};
use crate::profile::{Source, SourceKind};
use crate::{Error, Profile};

/// What an API returned.
#[derive(Debug, Clone, PartialEq)]
pub enum Fetched {
    /// Structured data: a JSON Resume document or an adapter's output.
    Profile(Box<Profile>),
    /// Any other JSON, to be read by the model.
    Raw(String),
}

/// How the response was interpreted, for display.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    JsonResume,
    Adapter(&'static str),
    Raw,
}

/// Fetches an API source and interprets the response: a configured or matching
/// adapter first, then JSON Resume, then raw JSON.
pub fn fetch(
    api: &ApiSource,
    token: Option<&str>,
    transport: &dyn Transport,
) -> Result<(Mode, Fetched), Error> {
    let mut headers: Headers = vec![("Accept", "application/json".to_owned())];
    if let Some(t) = token {
        headers.push(("Authorization", format!("Bearer {t}")));
    }
    let response = transport.get(&api.url, &headers)?;
    interpret(api, &response)
}

pub fn interpret(api: &ApiSource, response: &Value) -> Result<(Mode, Fetched), Error> {
    match api.adapter.as_deref() {
        Some("json-resume") => {
            return Ok((
                Mode::JsonResume,
                Fetched::Profile(Box::new(as_json_resume(response)?)),
            ));
        }
        Some("raw") => return Ok((Mode::Raw, Fetched::Raw(pretty(response)))),
        _ => {}
    }
    if let Some(adapter) = adapters::find(api.adapter.as_deref(), &api.url) {
        return Ok((
            Mode::Adapter(adapter.name()),
            Fetched::Profile(Box::new(adapter.convert(response)?)),
        ));
    }
    if let Some(name) = api.adapter.as_deref() {
        return Err(Error::UnknownAdapter(name.to_owned()));
    }
    if looks_like_json_resume(response)
        && let Ok(profile) = as_json_resume(response)
    {
        return Ok((Mode::JsonResume, Fetched::Profile(Box::new(profile))));
    }
    Ok((Mode::Raw, Fetched::Raw(pretty(response))))
}

fn looks_like_json_resume(v: &Value) -> bool {
    let Some(obj) = v.as_object() else {
        return false;
    };
    obj.get("basics").is_some_and(Value::is_object)
        || ["work", "education", "skills"]
            .iter()
            .any(|k| obj.get(*k).is_some_and(Value::is_array))
}

fn as_json_resume(v: &Value) -> Result<Profile, Error> {
    Ok(serde_json::from_value(v.clone())?)
}

fn pretty(v: &Value) -> String {
    serde_json::to_string_pretty(v).unwrap_or_else(|_| v.to_string())
}

/// A source id for an API: `https://api.example.com/v1/resume` becomes
/// `api-example-com-v1-resume`.
pub fn source_id(url: &str) -> String {
    let rest = url
        .split_once("://")
        .map(|(_, r)| r)
        .unwrap_or(url)
        .split('?')
        .next()
        .unwrap_or_default();
    let mut id = String::new();
    for c in rest.chars() {
        if c.is_ascii_alphanumeric() {
            id.push(c.to_ascii_lowercase());
        } else if !id.is_empty() && !id.ends_with('-') {
            id.push('-');
        }
    }
    let id = id.trim_end_matches('-');
    if id.is_empty() {
        "api".to_owned()
    } else {
        id.to_owned()
    }
}

/// Tags every entry of a structured profile with one source.
pub fn tag(mut profile: Profile, source: &Source) -> Profile {
    let id = Some(source.id.clone());
    profile.work.iter_mut().for_each(|e| e.source = id.clone());
    profile
        .volunteer
        .iter_mut()
        .for_each(|e| e.source = id.clone());
    profile
        .education
        .iter_mut()
        .for_each(|e| e.source = id.clone());
    profile
        .awards
        .iter_mut()
        .for_each(|e| e.source = id.clone());
    profile
        .certificates
        .iter_mut()
        .for_each(|e| e.source = id.clone());
    profile
        .publications
        .iter_mut()
        .for_each(|e| e.source = id.clone());
    profile
        .skills
        .iter_mut()
        .for_each(|e| e.source = id.clone());
    profile
        .languages
        .iter_mut()
        .for_each(|e| e.source = id.clone());
    profile
        .interests
        .iter_mut()
        .for_each(|e| e.source = id.clone());
    profile
        .references
        .iter_mut()
        .for_each(|e| e.source = id.clone());
    profile
        .projects
        .iter_mut()
        .for_each(|e| e.source = id.clone());
    profile.sources = vec![source.clone()];
    profile
}

pub fn source_for(url: &str, id: String) -> Source {
    Source {
        id,
        kind: SourceKind::Api,
        location: url.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn api(url: &str, adapter: Option<&str>) -> ApiSource {
        ApiSource {
            url: url.into(),
            adapter: adapter.map(str::to_owned),
            token: false,
        }
    }

    #[test]
    fn detects_json_resume() {
        let (mode, fetched) = interpret(
            &api("https://x/resume", None),
            &json!({ "basics": { "name": "Sam" }, "work": [{ "name": "Acme" }] }),
        )
        .unwrap();
        assert_eq!(mode, Mode::JsonResume);
        let Fetched::Profile(p) = fetched else {
            panic!()
        };
        assert_eq!(p.work[0].name.as_deref(), Some("Acme"));
    }

    #[test]
    fn other_json_is_raw() {
        let (mode, fetched) = interpret(
            &api("https://x/me", None),
            &json!({ "jobs": [{ "title": "Dev" }] }),
        )
        .unwrap();
        assert_eq!(mode, Mode::Raw);
        assert!(matches!(fetched, Fetched::Raw(t) if t.contains("\"title\": \"Dev\"")));
    }

    #[test]
    fn configured_adapter_wins() {
        let (mode, _) =
            interpret(&api("https://x/me", Some("raw")), &json!({ "basics": {} })).unwrap();
        assert_eq!(mode, Mode::Raw);
        let (mode, _) = interpret(
            &api("https://api.github.com/users/x/repos", None),
            &json!([]),
        )
        .unwrap();
        assert_eq!(mode, Mode::Adapter("github"));
        assert!(matches!(
            interpret(&api("https://x", Some("nope")), &json!({})),
            Err(Error::UnknownAdapter(_))
        ));
    }

    #[test]
    fn ids_and_tags() {
        assert_eq!(
            source_id("https://api.example.com/v1/resume?x=1"),
            "api-example-com-v1-resume"
        );
        let source = source_for("https://x", "x".into());
        let p = tag(
            Profile::from_json(r#"{ "work": [{}], "skills": [{}] }"#).unwrap(),
            &source,
        );
        assert_eq!(p.source_refs().collect::<Vec<_>>(), ["x", "x"]);
        assert!(p.dangling_sources().is_empty());
    }
}
