//! The resume pipeline: merge sources into a profile, tailor it to a job, export it.

pub mod context;
pub mod schema;

use std::collections::HashSet;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::export::{self, Lang, Options};
use crate::llm::{self, Llm, Prompt};
use crate::profile::Profile;
use crate::sources::Document;
use crate::{Error, Format};

const MERGE_PROMPT: &str = include_str!("../../assets/prompts/merge.md");
const TAILOR_PROMPT: &str = include_str!("../../assets/prompts/tailor.md");

/// Bump when the merge prompt or schema changes so cached profiles are rebuilt.
const MERGE_VERSION: u32 = 1;

/// Merges source documents into one profile. Results are cached in `cache_dir` by
/// the documents' contents and the model, so unchanged sources are not re-read.
pub fn merge(
    llm: &dyn Llm,
    model_id: &str,
    documents: &[Document],
    cache_dir: Option<&Path>,
) -> Result<Profile, Error> {
    if documents.is_empty() {
        return Err(Error::NoSources);
    }
    let cache_file = cache_dir.map(|dir| {
        let mut key = format!("v{MERGE_VERSION}\0{}\0{model_id}", llm.provider());
        for doc in documents {
            let _ = write!(key, "\0{}\0{}", doc.source.id, doc.text);
        }
        dir.join(format!("profile-{:016x}.json", fnv1a(key.as_bytes())))
    });
    if let Some(path) = &cache_file
        && let Ok(profile) = Profile::load(path)
    {
        return Ok(profile);
    }

    let mut user = String::new();
    for doc in documents {
        let _ = write!(
            user,
            "<source id=\"{}\">\n{}\n</source>\n\n",
            doc.source.id, doc.text
        );
    }
    let mut prompt = Prompt::new(MERGE_PROMPT, user).with_schema("profile", schema::profile());
    prompt.max_tokens = 16_000;
    prompt.temperature = 0.0;
    let mut profile: Profile = llm::complete_json(llm, &prompt)?;
    profile.sources = documents.iter().map(|d| d.source.clone()).collect();
    clear_unknown_sources(&mut profile);

    if let Some(path) = &cache_file {
        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        // A failed cache write only costs time on the next run.
        let _ = profile.save(path);
    }
    Ok(profile)
}

/// What the resume is aimed at.
#[derive(Debug, Clone, PartialEq, Eq, Default, Deserialize)]
#[serde(default)]
pub struct Target {
    pub company: Option<String>,
    pub role: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Tailored {
    pub target: Target,
    pub profile: Profile,
}

/// Inputs for tailoring.
#[derive(Debug, Clone, Default)]
pub struct TailorRequest {
    /// Job descriptions or postings; empty for a general resume.
    pub job: Vec<String>,
    /// Style guidance from earlier feedback.
    pub preferences: Vec<String>,
    /// Language code, e.g. `en`.
    pub lang: String,
    pub pages: u8,
}

/// Selects and rewrites the profile for one job.
pub fn tailor(llm: &dyn Llm, profile: &Profile, req: &TailorRequest) -> Result<Tailored, Error> {
    #[derive(Deserialize)]
    struct Answer {
        #[serde(default)]
        target: Target,
        resume: Profile,
    }

    let system = TAILOR_PROMPT
        .replace("{pages}", &req.pages.max(1).to_string())
        .replace("{language}", language_name(&req.lang));
    let mut source_profile = profile.clone();
    source_profile.sources.clear();
    let mut user = format!("<profile>\n{}\n</profile>\n\n", source_profile.to_json()?);
    if req.job.is_empty() {
        user.push_str("<job>\nNo specific job: write a strong general-purpose resume.\n</job>\n");
    } else {
        for job in &req.job {
            let _ = write!(user, "<job>\n{}\n</job>\n", job.trim());
        }
    }
    if !req.preferences.is_empty() {
        user.push_str("\n<preferences>\n");
        for p in &req.preferences {
            let _ = writeln!(user, "- {}", p.trim());
        }
        user.push_str("</preferences>\n");
    }

    let prompt = Prompt::new(system, user).with_schema("tailored_resume", schema::tailored());
    let answer: Answer = llm::complete_json(llm, &prompt)?;
    let mut tailored = answer.resume;
    tailored.sources = profile.sources.clone();
    clear_unknown_sources(&mut tailored);
    Ok(Tailored {
        target: clean_target(answer.target),
        profile: tailored,
    })
}

fn clean_target(t: Target) -> Target {
    let clean = |s: Option<String>| s.map(|s| s.trim().to_owned()).filter(|s| !s.is_empty());
    Target {
        company: clean(t.company),
        role: clean(t.role),
    }
}

/// Removes `source` ids that do not name a known source, so the fact check can
/// treat them as unsupported.
fn clear_unknown_sources(profile: &mut Profile) {
    let known: HashSet<String> = profile.sources.iter().map(|s| s.id.clone()).collect();
    let fix = |s: &mut Option<String>| {
        if s.as_ref().is_some_and(|id| !known.contains(id)) {
            *s = None;
        }
    };
    profile.work.iter_mut().for_each(|e| fix(&mut e.source));
    profile
        .volunteer
        .iter_mut()
        .for_each(|e| fix(&mut e.source));
    profile
        .education
        .iter_mut()
        .for_each(|e| fix(&mut e.source));
    profile.awards.iter_mut().for_each(|e| fix(&mut e.source));
    profile
        .certificates
        .iter_mut()
        .for_each(|e| fix(&mut e.source));
    profile
        .publications
        .iter_mut()
        .for_each(|e| fix(&mut e.source));
    profile.skills.iter_mut().for_each(|e| fix(&mut e.source));
    profile
        .languages
        .iter_mut()
        .for_each(|e| fix(&mut e.source));
    profile
        .interests
        .iter_mut()
        .for_each(|e| fix(&mut e.source));
    profile
        .references
        .iter_mut()
        .for_each(|e| fix(&mut e.source));
    profile.projects.iter_mut().for_each(|e| fix(&mut e.source));
}

/// English name of a language code, for prompts.
pub fn language_name(code: &str) -> &str {
    match code.split('-').next().unwrap_or(code) {
        "en" => "English",
        "de" => "German",
        "fr" => "French",
        "es" => "Spanish",
        "it" => "Italian",
        "pt" => "Portuguese",
        "nl" => "Dutch",
        "pl" => "Polish",
        "sv" => "Swedish",
        "da" => "Danish",
        "no" | "nb" => "Norwegian",
        "fi" => "Finnish",
        "cs" => "Czech",
        "ru" => "Russian",
        "uk" => "Ukrainian",
        "tr" => "Turkish",
        "ja" => "Japanese",
        "zh" => "Chinese",
        "ko" => "Korean",
        _ => code,
    }
}

/// A new folder for one application: `<base>/<date>_<company-or-role>`, with a
/// numeric suffix if it already exists (unless `overwrite`).
pub fn application_dir(base: &Path, target: &Target, overwrite: bool) -> PathBuf {
    let label = target
        .company
        .as_deref()
        .or(target.role.as_deref())
        .map(slug)
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "general".to_owned());
    let name = format!("{}_{label}", today());
    let dir = base.join(&name);
    if overwrite || !dir.exists() {
        return dir;
    }
    (2..)
        .map(|n| base.join(format!("{name}-{n}")))
        .find(|d| !d.exists())
        .expect("unbounded search")
}

/// Writes the profile in each format to `dir/<stem>.<ext>`.
pub fn write_outputs(
    profile: &Profile,
    formats: &[Format],
    options: &Options,
    dir: &Path,
    stem: &str,
) -> Result<Vec<PathBuf>, Error> {
    std::fs::create_dir_all(dir).map_err(|e| Error::io(dir, e))?;
    formats
        .iter()
        .map(|&format| {
            let bytes = export::render(profile, format, options)?;
            let path = dir.join(format!("{stem}.{}", format.extension()));
            std::fs::write(&path, bytes).map_err(|e| Error::io(&path, e))?;
            Ok(path)
        })
        .collect()
}

/// Export options from settings.
pub fn export_options(lang: &str, template: crate::Template, pages: u8) -> Options {
    Options {
        lang: Lang::from_code(lang),
        template,
        pages: Some(pages),
    }
}

fn slug(s: &str) -> String {
    let mut out = String::new();
    for c in s.chars() {
        if c.is_alphanumeric() {
            out.extend(c.to_lowercase());
        } else if !out.is_empty() && !out.ends_with('-') {
            out.push('-');
        }
        if out.len() >= 40 {
            break;
        }
    }
    out.trim_end_matches('-').to_owned()
}

/// Today's date (UTC) as `YYYY-MM-DD`.
pub fn today() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let (y, m, d) = civil_from_days((secs / 86_400) as i64);
    format!("{y:04}-{m:02}-{d:02}")
}

/// Converts days since 1970-01-01 to a calendar date (Howard Hinnant's algorithm).
fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let y = yoe + era * 400 + i64::from(m <= 2);
    (y, m, d)
}

/// 64-bit FNV-1a: a stable hash for cache keys.
fn fnv1a(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325, |h, b| {
        (h ^ u64::from(*b)).wrapping_mul(0x0100_0000_01b3)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn civil_dates() {
        assert_eq!(civil_from_days(0), (1970, 1, 1));
        assert_eq!(civil_from_days(19_723), (2024, 1, 1));
        assert_eq!(civil_from_days(20_513), (2026, 3, 1));
        assert_eq!(civil_from_days(20_512), (2026, 2, 28));
        assert_eq!(today().len(), 10);
    }

    #[test]
    fn application_dirs() {
        let base = Path::new("out");
        let t = Target {
            company: Some("Acme GmbH & Co.".into()),
            role: Some("Engineer".into()),
        };
        assert!(
            application_dir(base, &t, false)
                .to_string_lossy()
                .ends_with("_acme-gmbh-co")
        );
        let general = application_dir(base, &Target::default(), false);
        assert!(general.to_string_lossy().ends_with("_general"));
    }

    #[test]
    fn language_names() {
        assert_eq!(language_name("de"), "German");
        assert_eq!(language_name("pt-BR"), "Portuguese");
        assert_eq!(language_name("xx"), "xx");
    }

    #[test]
    fn hash_is_stable() {
        assert_eq!(fnv1a(b""), 0xcbf2_9ce4_8422_2325);
        assert_eq!(fnv1a(b"a"), 0xaf63_dc4c_8601_ec8c);
    }
}
