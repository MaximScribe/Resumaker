//! The profile: everything Resumaker knows about a person.
//!
//! The shape follows the [JSON Resume](https://jsonresume.org/schema) standard, so any
//! JSON Resume document can be read as a profile. Resumaker adds one thing: every entry
//! may name the [`Source`] it came from, which lets the fact check trace each claim back
//! to the user's own files. Unknown fields are ignored and missing ones are left empty.

use std::path::Path;

use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;

use crate::Error;

#[skip_serializing_none]
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Profile {
    pub basics: Basics,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub work: Vec<Work>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub volunteer: Vec<Volunteer>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub education: Vec<Education>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub awards: Vec<Award>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub certificates: Vec<Certificate>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub publications: Vec<Publication>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub skills: Vec<Skill>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub languages: Vec<Language>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub interests: Vec<Interest>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub references: Vec<Reference>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub projects: Vec<Project>,
    /// Where the facts in this profile came from (Resumaker extension).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub sources: Vec<Source>,
}

#[skip_serializing_none]
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Basics {
    pub name: Option<String>,
    /// Headline, e.g. "Robotics Engineer".
    pub label: Option<String>,
    pub image: Option<String>,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub url: Option<String>,
    pub summary: Option<String>,
    pub location: Option<Location>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub profiles: Vec<SocialProfile>,
}

#[skip_serializing_none]
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Location {
    pub address: Option<String>,
    pub postal_code: Option<String>,
    pub city: Option<String>,
    pub country_code: Option<String>,
    pub region: Option<String>,
}

#[skip_serializing_none]
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct SocialProfile {
    pub network: Option<String>,
    pub username: Option<String>,
    pub url: Option<String>,
}

#[skip_serializing_none]
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Work {
    /// Company or organisation.
    pub name: Option<String>,
    pub position: Option<String>,
    pub location: Option<String>,
    pub description: Option<String>,
    pub url: Option<String>,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
    pub summary: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub highlights: Vec<String>,
    pub source: Option<String>,
}

#[skip_serializing_none]
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Volunteer {
    pub organization: Option<String>,
    pub position: Option<String>,
    pub url: Option<String>,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
    pub summary: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub highlights: Vec<String>,
    pub source: Option<String>,
}

#[skip_serializing_none]
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Education {
    pub institution: Option<String>,
    pub url: Option<String>,
    /// Field of study, e.g. "Computer Science".
    pub area: Option<String>,
    /// Degree, e.g. "Bachelor".
    pub study_type: Option<String>,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
    pub score: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub courses: Vec<String>,
    pub source: Option<String>,
}

#[skip_serializing_none]
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Award {
    pub title: Option<String>,
    pub date: Option<String>,
    pub awarder: Option<String>,
    pub summary: Option<String>,
    pub source: Option<String>,
}

#[skip_serializing_none]
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Certificate {
    pub name: Option<String>,
    pub date: Option<String>,
    pub issuer: Option<String>,
    pub url: Option<String>,
    pub source: Option<String>,
}

#[skip_serializing_none]
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Publication {
    pub name: Option<String>,
    pub publisher: Option<String>,
    pub release_date: Option<String>,
    pub url: Option<String>,
    pub summary: Option<String>,
    pub source: Option<String>,
}

#[skip_serializing_none]
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Skill {
    /// Skill group, e.g. "Languages".
    pub name: Option<String>,
    pub level: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub keywords: Vec<String>,
    pub source: Option<String>,
}

#[skip_serializing_none]
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Language {
    pub language: Option<String>,
    pub fluency: Option<String>,
    pub source: Option<String>,
}

#[skip_serializing_none]
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Interest {
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub keywords: Vec<String>,
    pub source: Option<String>,
}

#[skip_serializing_none]
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Reference {
    pub name: Option<String>,
    pub reference: Option<String>,
    pub source: Option<String>,
}

#[skip_serializing_none]
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Project {
    pub name: Option<String>,
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub highlights: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub keywords: Vec<String>,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
    pub url: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub roles: Vec<String>,
    pub entity: Option<String>,
    #[serde(rename = "type")]
    pub kind: Option<String>,
    pub source: Option<String>,
}

/// A place facts were read from.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Source {
    /// Short identifier that entries refer to, e.g. `"cv-2024"`.
    pub id: String,
    pub kind: SourceKind,
    /// File path or URL.
    pub location: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SourceKind {
    File,
    Api,
}

impl Profile {
    /// A fictional sample profile, used for template previews and tests.
    pub fn example() -> Self {
        Self::from_json(include_str!("../assets/example-profile.json"))
            .expect("bundled example profile is valid")
    }

    /// Parses a profile from JSON.
    pub fn from_json(json: &str) -> Result<Self, Error> {
        Ok(serde_json::from_str(json)?)
    }

    /// Serialises the profile as pretty-printed JSON.
    pub fn to_json(&self) -> Result<String, Error> {
        Ok(serde_json::to_string_pretty(self)?)
    }

    /// Reads a profile from a JSON file.
    pub fn load(path: &Path) -> Result<Self, Error> {
        let json = std::fs::read_to_string(path).map_err(|e| Error::io(path, e))?;
        Self::from_json(&json)
    }

    /// Writes the profile to a JSON file.
    pub fn save(&self, path: &Path) -> Result<(), Error> {
        std::fs::write(path, self.to_json()?).map_err(|e| Error::io(path, e))
    }

    pub fn source(&self, id: &str) -> Option<&Source> {
        self.sources.iter().find(|s| s.id == id)
    }

    /// Source ids referenced by entries but missing from [`Profile::sources`].
    pub fn dangling_sources(&self) -> Vec<&str> {
        let mut missing: Vec<&str> = self
            .source_refs()
            .filter(|id| self.source(id).is_none())
            .collect();
        missing.sort_unstable();
        missing.dedup();
        missing
    }

    /// Every source id referenced by an entry, in document order.
    pub fn source_refs(&self) -> impl Iterator<Item = &str> {
        let refs = [
            self.work.iter().map(|e| &e.source).collect::<Vec<_>>(),
            self.volunteer.iter().map(|e| &e.source).collect(),
            self.education.iter().map(|e| &e.source).collect(),
            self.awards.iter().map(|e| &e.source).collect(),
            self.certificates.iter().map(|e| &e.source).collect(),
            self.publications.iter().map(|e| &e.source).collect(),
            self.skills.iter().map(|e| &e.source).collect(),
            self.languages.iter().map(|e| &e.source).collect(),
            self.interests.iter().map(|e| &e.source).collect(),
            self.references.iter().map(|e| &e.source).collect(),
            self.projects.iter().map(|e| &e.source).collect(),
        ];
        refs.into_iter().flatten().filter_map(|s| s.as_deref())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_object_is_empty_profile() {
        assert_eq!(Profile::from_json("{}").unwrap(), Profile::default());
    }

    #[test]
    fn empty_profile_serialises_compactly() {
        assert_eq!(
            Profile::default().to_json().unwrap(),
            "{\n  \"basics\": {}\n}"
        );
    }

    #[test]
    fn camel_case_keys_and_unknown_fields() {
        let p = Profile::from_json(
            r#"{ "work": [{ "name": "Acme", "startDate": "2021-03", "x-custom": 1 }] }"#,
        )
        .unwrap();
        assert_eq!(p.work[0].start_date.as_deref(), Some("2021-03"));
        assert!(p.to_json().unwrap().contains("\"startDate\": \"2021-03\""));
    }

    #[test]
    fn project_type_maps_to_kind() {
        let p = Profile::from_json(r#"{ "projects": [{ "type": "application" }] }"#).unwrap();
        assert_eq!(p.projects[0].kind.as_deref(), Some("application"));
    }

    #[test]
    fn finds_dangling_source_refs() {
        let p = Profile::from_json(
            r#"{
                "work": [{ "source": "cv" }, { "source": "gone" }],
                "skills": [{ "source": "gone" }, {}],
                "sources": [{ "id": "cv", "kind": "file", "location": "cv.pdf" }]
            }"#,
        )
        .unwrap();
        assert_eq!(p.source_refs().collect::<Vec<_>>(), ["cv", "gone", "gone"]);
        assert_eq!(p.dangling_sources(), ["gone"]);
    }

    #[test]
    fn rejects_malformed_json() {
        assert!(matches!(Profile::from_json("{"), Err(Error::Json(_))));
    }
}
