use std::io;
use std::path::{Path, PathBuf};

/// Errors returned by the Resumaker core.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    #[error("{path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    #[error("invalid JSON: {0}")]
    Json(#[from] serde_json::Error),

    #[error("rendering failed: {0}")]
    Render(String),

    #[error("unknown template `{0}` (use a built-in name or a path to a .typ file)")]
    UnknownTemplate(String),

    #[error("unknown format `{0}` (expected pdf, docx, md, html, txt or json)")]
    UnknownFormat(String),

    #[error("unknown provider `{0}`")]
    UnknownProvider(String),

    #[error("{}invalid config: {message}", path.as_ref().map(|p| format!("{}: ", p.display())).unwrap_or_default())]
    Config {
        path: Option<PathBuf>,
        message: String,
    },

    #[error("could not determine the config directory; set RESUMAKER_CONFIG")]
    NoConfigDir,

    #[error("keychain: {0}")]
    Keychain(String),
}

impl Error {
    /// Attaches a file path to a config error.
    pub(crate) fn in_file(self, file: &Path) -> Self {
        match self {
            Self::Config { message, .. } => Self::Config {
                path: Some(file.to_path_buf()),
                message,
            },
            other => other,
        }
    }

    pub(crate) fn io(path: &Path, source: io::Error) -> Self {
        Self::Io {
            path: path.to_path_buf(),
            source,
        }
    }
}
