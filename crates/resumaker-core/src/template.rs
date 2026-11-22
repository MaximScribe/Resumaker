//! Resume templates: Typst files that lay out a profile for PDF export.

use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::{Config, Error};

const BUILTIN: &[(&str, &str)] = &[("classic", include_str!("../assets/templates/classic.typ"))];

/// The template used when none is configured.
pub const DEFAULT: &str = "classic";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Template {
    name: String,
    source: String,
}

impl Template {
    /// A template bundled with Resumaker.
    pub fn builtin(name: &str) -> Option<Self> {
        BUILTIN
            .iter()
            .find(|(n, _)| *n == name)
            .map(|(n, src)| Self::new(*n, *src))
    }

    /// Names of the bundled templates.
    pub fn builtin_names() -> impl Iterator<Item = &'static str> {
        BUILTIN.iter().map(|(n, _)| *n)
    }

    /// Loads a template from a `.typ` file.
    pub fn from_file(path: &Path) -> Result<Self, Error> {
        let source = std::fs::read_to_string(path).map_err(|e| Error::io(path, e))?;
        let name = path
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| "custom".to_owned());
        Ok(Self { name, source })
    }

    /// Resolves a template by built-in name, installed name or path to a `.typ` file.
    pub fn resolve(name_or_path: &str) -> Result<Self, Error> {
        if let Some(t) = Self::builtin(name_or_path) {
            return Ok(t);
        }
        if is_plain_name(name_or_path)
            && let Ok(dir) = user_dir()
        {
            let installed = dir.join(format!("{name_or_path}.typ"));
            if installed.is_file() {
                return Self::from_file(&installed);
            }
        }
        let path = Path::new(name_or_path);
        if path.extension().is_some_and(|e| e == "typ") {
            return Self::from_file(path);
        }
        Err(Error::UnknownTemplate(name_or_path.to_owned()))
    }

    pub fn new(name: impl Into<String>, source: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            source: source.into(),
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn source(&self) -> &str {
        &self.source
    }
}

impl Default for Template {
    fn default() -> Self {
        Self::builtin(DEFAULT).expect("default template is bundled")
    }
}

/// Where downloaded templates are installed.
pub fn user_dir() -> Result<PathBuf, Error> {
    Ok(Config::dir()?.join("templates"))
}

/// Where the template pack is downloaded from; override with `RESUMAKER_TEMPLATES_URL`.
pub fn pack_url() -> String {
    std::env::var("RESUMAKER_TEMPLATES_URL").unwrap_or_else(|_| {
        "https://raw.githubusercontent.com/MaximScribe/Resumaker/main/templates".to_owned()
    })
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct PackEntry {
    pub name: String,
    pub file: String,
    #[serde(default)]
    pub description: String,
}

#[derive(Deserialize)]
struct PackIndex {
    templates: Vec<PackEntry>,
}

/// Templates installed in `dir`, with descriptions from the saved pack index.
pub fn installed(dir: &Path) -> Vec<PackEntry> {
    let index: Vec<PackEntry> = std::fs::read_to_string(dir.join("index.json"))
        .ok()
        .and_then(|t| serde_json::from_str::<PackIndex>(&t).ok())
        .map(|i| i.templates)
        .unwrap_or_default();
    let Ok(read) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut out: Vec<PackEntry> = read
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|e| e == "typ"))
        .filter_map(|p| {
            let name = p.file_stem()?.to_string_lossy().into_owned();
            let description = index
                .iter()
                .find(|e| e.name == name)
                .map(|e| e.description.clone())
                .unwrap_or_default();
            Some(PackEntry {
                file: format!("{name}.typ"),
                name,
                description,
            })
        })
        .collect();
    out.sort_by(|a, b| a.name.cmp(&b.name));
    out
}

/// Downloads the template pack into `dir` using `fetch` (URL to text). Each template
/// must compile before it is installed. Returns the installed entries.
pub fn download(
    base_url: &str,
    dir: &Path,
    fetch: impl Fn(&str) -> Result<String, Error>,
) -> Result<Vec<PackEntry>, Error> {
    let base = base_url.trim_end_matches('/');
    let index_text = fetch(&format!("{base}/index.json"))?;
    let index: PackIndex = serde_json::from_str(&index_text)?;
    let mut checked = Vec::new();
    for entry in index.templates {
        let stem = entry.file.strip_suffix(".typ").unwrap_or("");
        if !is_plain_name(&entry.name) || !is_plain_name(stem) {
            return Err(Error::UnknownTemplate(entry.name));
        }
        let template = Template::new(
            entry.name.clone(),
            fetch(&format!("{base}/{}", entry.file))?,
        );
        crate::export::render(
            &crate::Profile::example(),
            crate::Format::Pdf,
            &crate::export::Options {
                template: template.clone(),
                ..Default::default()
            },
        )?;
        checked.push((entry, template));
    }
    std::fs::create_dir_all(dir).map_err(|e| Error::io(dir, e))?;
    for (entry, template) in &checked {
        let path = dir.join(format!("{}.typ", entry.name));
        std::fs::write(&path, template.source()).map_err(|e| Error::io(&path, e))?;
    }
    let path = dir.join("index.json");
    std::fs::write(&path, index_text).map_err(|e| Error::io(&path, e))?;
    Ok(checked.into_iter().map(|(e, _)| e).collect())
}

/// A name that is safe to use as a file name: letters, digits, `-` and `_`.
fn is_plain_name(name: &str) -> bool {
    !name.is_empty()
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

#[cfg(test)]
mod tests {
    use super::*;

    const INDEX: &str =
        r#"{ "templates": [{ "name": "plain", "file": "plain.typ", "description": "Bare" }] }"#;
    const PLAIN: &str = "#let v = json(\"/view.json\")\n= #v.header.name";

    #[test]
    fn downloads_and_lists_pack() {
        let dir = std::env::temp_dir().join(format!("resumaker-pack-{}", std::process::id()));
        let fetch = |url: &str| -> Result<String, Error> {
            Ok(if url.ends_with("index.json") {
                INDEX
            } else {
                PLAIN
            }
            .to_owned())
        };
        let got = download("https://example/templates/", &dir, fetch).unwrap();
        assert_eq!(got.len(), 1);
        let listed = installed(&dir);
        assert_eq!(listed[0].name, "plain");
        assert_eq!(listed[0].description, "Bare");
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn rejects_unsafe_names_and_broken_templates() {
        let dir = std::env::temp_dir().join(format!("resumaker-pack-bad-{}", std::process::id()));
        let evil = |_: &str| -> Result<String, Error> {
            Ok(r#"{ "templates": [{ "name": "../x", "file": "x.typ" }] }"#.to_owned())
        };
        assert!(download("u", &dir, evil).is_err());
        let broken = |url: &str| -> Result<String, Error> {
            Ok(if url.ends_with("index.json") {
                r#"{ "templates": [{ "name": "b", "file": "b.typ" }] }"#.to_owned()
            } else {
                "#nope(".to_owned()
            })
        };
        assert!(download("u", &dir, broken).is_err());
        assert!(!dir.join("b.typ").exists());
        std::fs::remove_dir_all(&dir).ok();
    }
}
