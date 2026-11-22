//! User settings, stored as TOML in the platform config directory.
//!
//! Secrets are never written here; see [`crate::secrets`].

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::export::Lang;
use crate::provider::ProviderKind;
use crate::{Error, Format};

/// Overrides the config file location.
pub const CONFIG_ENV: &str = "RESUMAKER_CONFIG";

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    pub inference: Inference,
    pub sources: Sources,
    pub output: Output,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Inference {
    pub provider: ProviderKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    /// Base URL; defaults to the provider's standard address.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
}

impl Default for Inference {
    fn default() -> Self {
        Self {
            provider: ProviderKind::Ollama,
            model: None,
            url: None,
        }
    }
}

impl Inference {
    pub fn base_url(&self) -> Option<&str> {
        self.url.as_deref().or(self.provider.default_url())
    }
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Sources {
    /// Folder of experience files, read recursively.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub folder: Option<PathBuf>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub apis: Vec<ApiSource>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ApiSource {
    pub url: String,
    /// Adapter name; detected automatically when unset.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub adapter: Option<String>,
    /// A bearer token for this API is stored in the keychain.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub token: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Output {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub folder: Option<PathBuf>,
    pub formats: Vec<Format>,
    /// Built-in template name or path to a template.
    pub style: String,
    pub lang: String,
    pub pages: u8,
}

impl Default for Output {
    fn default() -> Self {
        Self {
            folder: None,
            formats: vec![Format::Pdf],
            style: crate::template::DEFAULT.to_owned(),
            lang: "en".to_owned(),
            pages: 1,
        }
    }
}

impl Output {
    /// The configured folder, or `Documents/Resumaker`.
    pub fn folder(&self) -> PathBuf {
        self.folder.as_deref().map(expand_home).unwrap_or_else(|| {
            dirs::document_dir()
                .or_else(dirs::home_dir)
                .unwrap_or_default()
                .join("Resumaker")
        })
    }

    pub fn lang(&self) -> Lang {
        Lang::from_code(&self.lang)
    }
}

impl Config {
    /// The directory holding the config file and other Resumaker data.
    pub fn dir() -> Result<PathBuf, Error> {
        if let Some(path) = std::env::var_os(CONFIG_ENV) {
            let path = PathBuf::from(path);
            return Ok(path.parent().map(Path::to_path_buf).unwrap_or_default());
        }
        dirs::config_dir()
            .map(|d| d.join("resumaker"))
            .ok_or(Error::NoConfigDir)
    }

    /// Location of `config.toml`, honouring `RESUMAKER_CONFIG`.
    pub fn path() -> Result<PathBuf, Error> {
        match std::env::var_os(CONFIG_ENV) {
            Some(path) => Ok(PathBuf::from(path)),
            None => Ok(Self::dir()?.join("config.toml")),
        }
    }

    /// Loads the config, or returns `None` if setup has not been run yet.
    pub fn load(path: &Path) -> Result<Option<Self>, Error> {
        match std::fs::read_to_string(path) {
            Ok(text) => Self::from_toml(&text)
                .map(Some)
                .map_err(|e| e.in_file(path)),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(Error::io(path, e)),
        }
    }

    pub fn save(&self, path: &Path) -> Result<(), Error> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).map_err(|e| Error::io(dir, e))?;
        }
        std::fs::write(path, self.to_toml()?).map_err(|e| Error::io(path, e))
    }

    pub fn from_toml(text: &str) -> Result<Self, Error> {
        let config: Self = toml::from_str(text).map_err(|e| Error::Config {
            path: None,
            message: e.to_string(),
        })?;
        config.validate()?;
        Ok(config)
    }

    pub fn to_toml(&self) -> Result<String, Error> {
        let body = toml::to_string_pretty(self).map_err(|e| Error::Config {
            path: None,
            message: e.to_string(),
        })?;
        Ok(format!(
            "# Resumaker settings. Edit by hand or with `resumaker set …`.\n\
             # API keys are stored in the OS keychain, not here.\n\n{body}"
        ))
    }

    fn validate(&self) -> Result<(), Error> {
        let invalid = |message: String| Error::Config {
            path: None,
            message,
        };
        if self.output.formats.is_empty() {
            return Err(invalid(
                "output.formats must list at least one format".into(),
            ));
        }
        if !(1..=2).contains(&self.output.pages) {
            return Err(invalid("output.pages must be 1 or 2".into()));
        }
        if self.inference.provider == ProviderKind::OpenaiCompatible && self.inference.url.is_none()
        {
            return Err(invalid(
                "inference.url is required for an OpenAI-compatible server".into(),
            ));
        }
        Ok(())
    }
}

/// Expands a leading `~` to the home directory.
pub fn expand_home(path: &Path) -> PathBuf {
    let Ok(rest) = path.strip_prefix("~") else {
        return path.to_path_buf();
    };
    match dirs::home_dir() {
        Some(home) => home.join(rest),
        None => path.to_path_buf(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_file_gives_defaults() {
        assert_eq!(Config::from_toml("").unwrap(), Config::default());
    }

    #[test]
    fn round_trips_through_toml() {
        let mut config = Config::default();
        config.inference.provider = ProviderKind::Openai;
        config.inference.model = Some("gpt-5".into());
        config.sources.folder = Some("~/Career".into());
        config.sources.apis.push(ApiSource {
            url: "https://api.example.com/resume".into(),
            adapter: None,
            token: true,
        });
        config.output.formats = vec![Format::Pdf, Format::Md];
        let text = config.to_toml().unwrap();
        assert!(text.contains("provider = \"openai\""));
        assert_eq!(Config::from_toml(&text).unwrap(), config);
    }

    #[test]
    fn rejects_unknown_keys_and_bad_values() {
        assert!(Config::from_toml("[output]\ncolour = \"red\"").is_err());
        assert!(Config::from_toml("[output]\nformats = []").is_err());
        assert!(Config::from_toml("[output]\npages = 3").is_err());
        assert!(Config::from_toml("[inference]\nprovider = \"openai-compatible\"").is_err());
    }

    #[test]
    fn missing_file_is_none() {
        assert_eq!(
            Config::load(Path::new("no/such/config.toml")).unwrap(),
            None
        );
    }

    #[test]
    fn save_and_load() {
        let path = std::env::temp_dir()
            .join(format!("resumaker-cfg-{}", std::process::id()))
            .join("config.toml");
        let config = Config::default();
        config.save(&path).unwrap();
        let loaded = Config::load(&path).unwrap();
        std::fs::remove_dir_all(path.parent().unwrap()).ok();
        assert_eq!(loaded, Some(config));
    }

    #[test]
    fn expands_home() {
        let home = dirs::home_dir().unwrap();
        assert_eq!(expand_home(Path::new("~/x")), home.join("x"));
        assert_eq!(expand_home(Path::new("/abs")), PathBuf::from("/abs"));
    }
}
