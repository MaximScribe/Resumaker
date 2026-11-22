//! Checks generated documents against the source profile.
//!
//! The checks are deterministic, so they cannot be talked out of a finding:
//! - every entry names a known source;
//! - employers, schools, certificates and their dates exist in the source profile;
//! - every number in the text appears somewhere in the sources (catches invented metrics);
//! - every skill keyword appears somewhere in the sources.

use std::collections::HashSet;
use std::fmt;

use crate::Profile;
use crate::letter::Letter;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Issue {
    /// Where in the document, e.g. `work[1].highlights[0]`.
    pub location: String,
    pub message: String,
}

impl fmt::Display for Issue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.location, self.message)
    }
}

/// Everything the sources say, normalised for matching.
struct Facts {
    text: String,
    numbers: HashSet<String>,
}

impl Facts {
    fn new(source: &Profile, extra: &[String]) -> Self {
        let mut text = source.to_json().unwrap_or_default();
        for e in extra {
            text.push('\n');
            text.push_str(e);
        }
        let numbers = numbers(&text).into_iter().collect();
        Self {
            text: normalize(&text),
            numbers,
        }
    }

    fn mentions(&self, phrase: &str) -> bool {
        let p = normalize(phrase);
        p.is_empty() || self.text.contains(&p)
    }
}

/// Lower-case, alphanumerics only, single spaces.
fn normalize(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        if c.is_alphanumeric() {
            out.extend(c.to_lowercase());
        } else if !out.ends_with(' ') {
            out.push(' ');
        }
    }
    out.trim().to_owned()
}

/// Numbers in a text, with decimal commas read as points and thousands separators
/// removed: "2,000,000", "2.5" and "40 %" give "2000000", "2.5" and "40".
fn numbers(text: &str) -> Vec<String> {
    let chars: Vec<char> = text.chars().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        if !chars[i].is_ascii_digit() {
            i += 1;
            continue;
        }
        let mut raw = String::new();
        while i < chars.len()
            && (chars[i].is_ascii_digit()
                || ((chars[i] == '.' || chars[i] == ',')
                    && chars.get(i + 1).is_some_and(char::is_ascii_digit)))
        {
            raw.push(chars[i]);
            i += 1;
        }
        let mut value = canonical_number(&raw);
        // A scale word may follow, directly or after one space: "2M", "2 million", "5k".
        let mut j = i;
        if chars.get(j) == Some(&' ') {
            j += 1;
        }
        let word: String = chars[j.min(chars.len())..]
            .iter()
            .take_while(|c| c.is_alphabetic())
            .collect::<String>()
            .to_lowercase();
        let scale = match word.as_str() {
            "k" | "thousand" | "tausend" => 1e3,
            "m" | "mio" | "million" | "millions" | "millionen" => 1e6,
            "bn" | "billion" | "billions" | "mrd" | "milliarden" => 1e9,
            _ => 1.0,
        };
        if scale > 1.0
            && let Ok(v) = value.parse::<f64>()
        {
            let scaled = v * scale;
            value = if scaled.fract() == 0.0 {
                format!("{scaled:.0}")
            } else {
                scaled.to_string()
            };
        }
        out.push(value);
    }
    out
}

fn canonical_number(raw: &str) -> String {
    // Separators followed by exactly three digits are thousands separators.
    let groups: Vec<&str> = raw.split(['.', ',']).collect();
    let thousands = groups.len() > 1 && groups[1..].iter().all(|g| g.len() == 3);
    if thousands {
        groups.concat()
    } else {
        raw.replace(',', ".")
    }
}

/// Small numbers ("2 teams", "3 years") are too common to be meaningful evidence.
fn is_trivial(n: &str) -> bool {
    n.parse::<f64>()
        .is_ok_and(|v| v <= 10.0 && v.fract() == 0.0)
}

/// Checks a generated resume against the profile it was made from.
pub fn check_resume(generated: &Profile, source: &Profile) -> Vec<Issue> {
    let facts = Facts::new(source, &[]);
    let known: HashSet<&str> = source.sources.iter().map(|s| s.id.as_str()).collect();
    let mut issues = Vec::new();
    let mut flag = |location: String, message: String| issues.push(Issue { location, message });

    let mut source_check = |section: &str, i: usize, id: &Option<String>| match id {
        None => flag(format!("{section}[{i}]"), "has no source".into()),
        Some(id) if !known.is_empty() && !known.contains(id.as_str()) => flag(
            format!("{section}[{i}]"),
            format!("cites unknown source `{id}`"),
        ),
        _ => {}
    };
    for (i, e) in generated.work.iter().enumerate() {
        source_check("work", i, &e.source);
    }
    for (i, e) in generated.education.iter().enumerate() {
        source_check("education", i, &e.source);
    }
    for (i, e) in generated.projects.iter().enumerate() {
        source_check("projects", i, &e.source);
    }
    for (i, e) in generated.certificates.iter().enumerate() {
        source_check("certificates", i, &e.source);
    }
    for (i, e) in generated.skills.iter().enumerate() {
        source_check("skills", i, &e.source);
    }

    for (i, w) in generated.work.iter().enumerate() {
        if let Some(name) = w.name.as_deref().filter(|n| !facts.mentions(n)) {
            flag(
                format!("work[{i}].name"),
                format!("employer `{name}` is not in the sources"),
            );
        }
        for (field, date) in [("startDate", &w.start_date), ("endDate", &w.end_date)] {
            if let Some(d) = date.as_deref().filter(|d| !facts.mentions(d)) {
                flag(
                    format!("work[{i}].{field}"),
                    format!("date `{d}` is not in the sources"),
                );
            }
        }
    }
    for (i, e) in generated.education.iter().enumerate() {
        if let Some(name) = e.institution.as_deref().filter(|n| !facts.mentions(n)) {
            flag(
                format!("education[{i}].institution"),
                format!("`{name}` is not in the sources"),
            );
        }
    }
    for (i, c) in generated.certificates.iter().enumerate() {
        if let Some(name) = c.name.as_deref().filter(|n| !facts.mentions(n)) {
            flag(
                format!("certificates[{i}].name"),
                format!("`{name}` is not in the sources"),
            );
        }
    }
    for (i, s) in generated.skills.iter().enumerate() {
        for k in s.keywords.iter().filter(|k| !facts.mentions(k)) {
            flag(
                format!("skills[{i}].keywords"),
                format!("`{k}` is not in the sources"),
            );
        }
    }

    let mut texts: Vec<(String, &str)> = Vec::new();
    if let Some(s) = &generated.basics.summary {
        texts.push(("basics.summary".into(), s));
    }
    for (i, w) in generated.work.iter().enumerate() {
        if let Some(s) = &w.summary {
            texts.push((format!("work[{i}].summary"), s));
        }
        for (j, h) in w.highlights.iter().enumerate() {
            texts.push((format!("work[{i}].highlights[{j}]"), h));
        }
    }
    for (i, p) in generated.projects.iter().enumerate() {
        if let Some(s) = &p.description {
            texts.push((format!("projects[{i}].description"), s));
        }
        for (j, h) in p.highlights.iter().enumerate() {
            texts.push((format!("projects[{i}].highlights[{j}]"), h));
        }
    }
    for (location, text) in texts {
        for issue in unsupported_numbers(&location, text, &facts) {
            issues.push(issue);
        }
    }
    issues
}

/// Checks a cover letter: every number must come from the profile or the notes.
pub fn check_letter(letter: &Letter, source: &Profile, notes: &[String]) -> Vec<Issue> {
    let facts = Facts::new(source, notes);
    letter
        .paragraphs
        .iter()
        .enumerate()
        .flat_map(|(i, p)| unsupported_numbers(&format!("paragraph {}", i + 1), p, &facts))
        .collect()
}

fn unsupported_numbers(location: &str, text: &str, facts: &Facts) -> Vec<Issue> {
    let mut seen = HashSet::new();
    numbers(text)
        .into_iter()
        .filter(|n| !is_trivial(n) && !facts.numbers.contains(n) && seen.insert(n.clone()))
        .map(|n| Issue {
            location: location.to_owned(),
            message: format!("the number {n} is not in the sources"),
        })
        .collect()
}

/// Turns issues into instructions for a corrected draft.
pub fn correction_note(issues: &[Issue]) -> String {
    let mut note = String::from(
        "A fact check of your previous draft found claims that are not supported by the profile. \
         Remove or correct each of them, and change nothing else:",
    );
    for i in issues {
        note.push_str(&format!("\n- {i}"));
    }
    note
}

#[cfg(test)]
mod tests {
    use super::*;

    fn source() -> Profile {
        Profile::example()
    }

    #[test]
    fn example_tailored_to_itself_is_clean() {
        let p = source();
        assert_eq!(check_resume(&p, &p), Vec::<Issue>::new());
    }

    #[test]
    fn catches_invented_facts() {
        let src = source();
        let mut fake = src.clone();
        fake.work[0].highlights[0] = "Cut planning time by 87% for 12000 robots".into();
        fake.work[1].name = Some("Initech".into());
        fake.work[1].start_date = Some("2018-01".into());
        fake.skills[0].keywords.push("Haskell".into());
        fake.projects[0].source = None;
        let issues = check_resume(&fake, &src);
        let text: Vec<String> = issues.iter().map(ToString::to_string).collect();
        let has = |s: &str| text.iter().any(|t| t.contains(s));
        assert!(has("work[0].highlights[0]: the number 87"), "{text:?}");
        assert!(has("the number 12000"));
        assert!(has("employer `Initech`"));
        assert!(has("date `2018-01`"));
        assert!(has("`Haskell`"));
        assert!(has("projects[0]: has no source"));
    }

    #[test]
    fn allows_rephrasing_and_number_formats() {
        let src = source();
        let mut ok = src.clone();
        ok.work[0].highlights =
            vec!["Made the planner 8x faster: 120 ms down to 15 ms, in Rust".into()];
        ok.work[1].highlights = vec!["Served 2,000,000 requests per day; p99 latency −40 %".into()];
        assert_eq!(check_resume(&ok, &src), Vec::<Issue>::new());
    }

    #[test]
    fn numbers_are_canonical() {
        assert_eq!(
            numbers("2,000,000 and 2.5 and 3,5 and 40%"),
            ["2000000", "2.5", "3.5", "40"]
        );
        assert_eq!(
            numbers("2 million, 2M, 1.5k users, 3 months"),
            ["2000000", "2000000", "1500", "3"]
        );
    }

    #[test]
    fn letters_may_use_notes() {
        let letter = Letter {
            paragraphs: vec![
                "I cut latency by 40% and can start on 15 March.".into(),
                "I led 250 people.".into(),
            ],
            ..Letter::default()
        };
        let issues = check_letter(&letter, &source(), &["Available from 15 March".into()]);
        assert_eq!(issues.len(), 1);
        assert!(issues[0].to_string().contains("250"));
    }
}
