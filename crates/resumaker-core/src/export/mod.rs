//! Turning a profile into files: PDF, DOCX, Markdown, HTML, plain text and JSON.

mod docx;
mod html;
mod markdown;
pub(crate) mod pdf;
mod text;
pub mod view;

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

pub use view::{Lang, View};

use crate::template::Template;
use crate::{Error, Profile};

/// File formats a resume can be exported as.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "clap", derive(clap::ValueEnum))]
#[serde(rename_all = "lowercase")]
pub enum Format {
    Pdf,
    Docx,
    Md,
    Html,
    Txt,
    Json,
}

impl Format {
    pub const ALL: [Format; 6] = [
        Format::Pdf,
        Format::Docx,
        Format::Md,
        Format::Html,
        Format::Txt,
        Format::Json,
    ];

    pub fn extension(self) -> &'static str {
        match self {
            Self::Pdf => "pdf",
            Self::Docx => "docx",
            Self::Md => "md",
            Self::Html => "html",
            Self::Txt => "txt",
            Self::Json => "json",
        }
    }
}

impl fmt::Display for Format {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.extension())
    }
}

impl FromStr for Format {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Error> {
        Self::ALL
            .into_iter()
            .find(|f| f.extension().eq_ignore_ascii_case(s.trim()))
            .ok_or_else(|| Error::UnknownFormat(s.to_owned()))
    }
}

/// Settings that apply to every format.
#[derive(Debug, Clone, Default)]
pub struct Options {
    pub lang: Lang,
    pub template: Template,
    /// Maximum resume length in pages; templates may use it to tighten the layout.
    pub pages: Option<u8>,
}

/// Renders a profile in the given format.
pub fn render(profile: &Profile, format: Format, options: &Options) -> Result<Vec<u8>, Error> {
    let view = View::new(profile, options.lang);
    Ok(match format {
        Format::Pdf => {
            let opts = serde_json::json!({
                "lang": options.lang.code(),
                "pages": options.pages,
            });
            pdf::render(&options.template, profile, &view, &opts)?
        }
        Format::Docx => docx::render(&view)?,
        Format::Md => markdown::render(&view).into_bytes(),
        Format::Html => html::render(&view).into_bytes(),
        Format::Txt => text::render(&view).into_bytes(),
        Format::Json => profile.to_json()?.into_bytes(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_formats() {
        assert_eq!("PDF".parse::<Format>().unwrap(), Format::Pdf);
        assert_eq!(" md ".parse::<Format>().unwrap(), Format::Md);
        assert!("png".parse::<Format>().is_err());
    }
}
