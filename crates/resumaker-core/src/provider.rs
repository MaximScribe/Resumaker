//! Inference providers Resumaker can talk to.

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

use crate::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "clap", derive(clap::ValueEnum))]
#[serde(rename_all = "kebab-case")]
pub enum ProviderKind {
    Ollama,
    LmStudio,
    /// Any server that speaks the OpenAI chat completions API (llama.cpp, vLLM, …).
    OpenaiCompatible,
    Gemini,
    Openrouter,
    Openai,
    Anthropic,
}

impl ProviderKind {
    pub const ALL: [ProviderKind; 7] = [
        Self::Ollama,
        Self::LmStudio,
        Self::OpenaiCompatible,
        Self::Gemini,
        Self::Openrouter,
        Self::Openai,
        Self::Anthropic,
    ];

    /// Runs on the user's machine; source files never leave it.
    pub fn is_local(self) -> bool {
        matches!(self, Self::Ollama | Self::LmStudio | Self::OpenaiCompatible)
    }

    /// Needs an API key. Local servers may still be given one.
    pub fn requires_key(self) -> bool {
        !self.is_local()
    }

    pub fn id(self) -> &'static str {
        match self {
            Self::Ollama => "ollama",
            Self::LmStudio => "lm-studio",
            Self::OpenaiCompatible => "openai-compatible",
            Self::Gemini => "gemini",
            Self::Openrouter => "openrouter",
            Self::Openai => "openai",
            Self::Anthropic => "anthropic",
        }
    }

    pub fn display_name(self) -> &'static str {
        match self {
            Self::Ollama => "Ollama",
            Self::LmStudio => "LM Studio",
            Self::OpenaiCompatible => "OpenAI-compatible server",
            Self::Gemini => "Google Gemini",
            Self::Openrouter => "OpenRouter",
            Self::Openai => "OpenAI",
            Self::Anthropic => "Anthropic",
        }
    }

    /// Base URL used when the config does not set one.
    pub fn default_url(self) -> Option<&'static str> {
        match self {
            Self::Ollama => Some("http://localhost:11434"),
            Self::LmStudio => Some("http://localhost:1234/v1"),
            Self::OpenaiCompatible => None,
            Self::Gemini => Some("https://generativelanguage.googleapis.com/v1beta"),
            Self::Openrouter => Some("https://openrouter.ai/api/v1"),
            Self::Openai => Some("https://api.openai.com/v1"),
            Self::Anthropic => Some("https://api.anthropic.com/v1"),
        }
    }

    /// Environment variable that overrides the stored API key.
    pub fn key_env_var(self) -> String {
        format!(
            "RESUMAKER_{}_KEY",
            self.id().replace('-', "_").to_ascii_uppercase()
        )
    }
}

impl fmt::Display for ProviderKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.id())
    }
}

impl FromStr for ProviderKind {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Error> {
        let s = s.trim().to_ascii_lowercase().replace('_', "-");
        Self::ALL
            .into_iter()
            .find(|p| p.id() == s)
            .ok_or(Error::UnknownProvider(s))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_round_trip() {
        for p in ProviderKind::ALL {
            assert_eq!(p.id().parse::<ProviderKind>().unwrap(), p);
        }
        assert_eq!(
            "LM_STUDIO".parse::<ProviderKind>().unwrap(),
            ProviderKind::LmStudio
        );
    }

    #[test]
    fn env_var_names() {
        assert_eq!(ProviderKind::Openai.key_env_var(), "RESUMAKER_OPENAI_KEY");
        assert_eq!(
            ProviderKind::OpenaiCompatible.key_env_var(),
            "RESUMAKER_OPENAI_COMPATIBLE_KEY"
        );
    }

    #[test]
    fn local_providers_need_no_key() {
        assert!(!ProviderKind::Ollama.requires_key());
        assert!(ProviderKind::Gemini.requires_key());
    }
}
