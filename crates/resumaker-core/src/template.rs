//! Resume templates: Typst files that lay out a profile for PDF export.

use std::path::Path;

use crate::Error;

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

    /// Resolves a template by built-in name or by path to a `.typ` file.
    pub fn resolve(name_or_path: &str) -> Result<Self, Error> {
        if let Some(t) = Self::builtin(name_or_path) {
            return Ok(t);
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
