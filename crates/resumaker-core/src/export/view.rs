//! A presentation-ready view of a profile: ordered sections with formatted dates.
//!
//! Every exporter renders from this view, so they all agree on section order, titles
//! and date formatting. Typst templates receive it as `/view.json`.

use serde::Serialize;

use crate::profile::{Location, Profile};

/// Output language for labels and dates.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Lang {
    #[default]
    En,
    De,
}

impl Lang {
    /// Parses an ISO 639-1 code; unsupported languages fall back to English.
    pub fn from_code(code: &str) -> Self {
        match code.trim().to_ascii_lowercase().as_str() {
            "de" | "de-de" | "de-at" | "de-ch" => Self::De,
            _ => Self::En,
        }
    }

    pub fn code(self) -> &'static str {
        match self {
            Self::En => "en",
            Self::De => "de",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum SectionKind {
    Work,
    Projects,
    Education,
    Skills,
    Certificates,
    Awards,
    Publications,
    Volunteer,
    Languages,
    Interests,
}

impl SectionKind {
    fn title(self, lang: Lang) -> &'static str {
        use SectionKind::*;
        match (lang, self) {
            (Lang::En, Work) => "Experience",
            (Lang::En, Projects) => "Projects",
            (Lang::En, Education) => "Education",
            (Lang::En, Skills) => "Skills",
            (Lang::En, Certificates) => "Certifications",
            (Lang::En, Awards) => "Awards",
            (Lang::En, Publications) => "Publications",
            (Lang::En, Volunteer) => "Volunteering",
            (Lang::En, Languages) => "Languages",
            (Lang::En, Interests) => "Interests",
            (Lang::De, Work) => "Berufserfahrung",
            (Lang::De, Projects) => "Projekte",
            (Lang::De, Education) => "Ausbildung",
            (Lang::De, Skills) => "Kenntnisse",
            (Lang::De, Certificates) => "Zertifikate",
            (Lang::De, Awards) => "Auszeichnungen",
            (Lang::De, Publications) => "Publikationen",
            (Lang::De, Volunteer) => "Ehrenamt",
            (Lang::De, Languages) => "Sprachen",
            (Lang::De, Interests) => "Interessen",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct View {
    pub lang: Lang,
    pub header: Header,
    pub sections: Vec<Section>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize)]
pub struct Header {
    pub name: String,
    pub label: Option<String>,
    pub summary: Option<String>,
    pub location: Option<String>,
    pub contacts: Vec<Contact>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Contact {
    pub text: String,
    pub url: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Section {
    pub kind: SectionKind,
    pub title: String,
    pub entries: Vec<Entry>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize)]
pub struct Entry {
    pub title: String,
    pub subtitle: Option<String>,
    pub dates: Option<String>,
    pub location: Option<String>,
    pub summary: Option<String>,
    pub bullets: Vec<String>,
    pub url: Option<String>,
}

impl View {
    pub fn new(profile: &Profile, lang: Lang) -> Self {
        let mut sections = Vec::new();
        let mut push = |kind: SectionKind, entries: Vec<Entry>| {
            let entries: Vec<Entry> = entries.into_iter().filter(|e| !e.is_empty()).collect();
            if !entries.is_empty() {
                sections.push(Section {
                    kind,
                    title: kind.title(lang).to_owned(),
                    entries,
                });
            }
        };

        push(
            SectionKind::Work,
            profile
                .work
                .iter()
                .map(|w| Entry {
                    title: text(&w.position)
                        .or_else(|| text(&w.name))
                        .unwrap_or_default(),
                    subtitle: text(&w.position).and(text(&w.name)),
                    dates: date_range(&w.start_date, &w.end_date, lang),
                    location: text(&w.location),
                    summary: text(&w.summary).or_else(|| text(&w.description)),
                    bullets: clean(&w.highlights),
                    url: text(&w.url),
                })
                .collect(),
        );
        push(
            SectionKind::Projects,
            profile
                .projects
                .iter()
                .map(|p| Entry {
                    title: text(&p.name).unwrap_or_default(),
                    subtitle: (!p.keywords.is_empty()).then(|| clean(&p.keywords).join(", ")),
                    dates: date_range(&p.start_date, &p.end_date, lang),
                    summary: text(&p.description),
                    bullets: clean(&p.highlights),
                    url: text(&p.url),
                    ..Entry::default()
                })
                .collect(),
        );
        push(
            SectionKind::Education,
            profile
                .education
                .iter()
                .map(|e| {
                    let degree = match (text(&e.study_type), text(&e.area)) {
                        (Some(t), Some(a)) => Some(format!("{t}, {a}")),
                        (t, a) => t.or(a),
                    };
                    Entry {
                        title: degree
                            .clone()
                            .or_else(|| text(&e.institution))
                            .unwrap_or_default(),
                        subtitle: degree.and(text(&e.institution)),
                        dates: date_range(&e.start_date, &e.end_date, lang),
                        summary: education_details(&e.score, &e.courses, lang),
                        url: text(&e.url),
                        ..Entry::default()
                    }
                })
                .collect(),
        );
        push(
            SectionKind::Skills,
            profile
                .skills
                .iter()
                .map(|s| Entry {
                    title: text(&s.name).unwrap_or_default(),
                    subtitle: text(&s.level),
                    summary: (!s.keywords.is_empty()).then(|| clean(&s.keywords).join(", ")),
                    ..Entry::default()
                })
                .collect(),
        );
        push(
            SectionKind::Certificates,
            profile
                .certificates
                .iter()
                .map(|c| Entry {
                    title: text(&c.name).unwrap_or_default(),
                    subtitle: text(&c.issuer),
                    dates: c.date.as_deref().map(|d| format_date(d, lang)),
                    url: text(&c.url),
                    ..Entry::default()
                })
                .collect(),
        );
        push(
            SectionKind::Awards,
            profile
                .awards
                .iter()
                .map(|a| Entry {
                    title: text(&a.title).unwrap_or_default(),
                    subtitle: text(&a.awarder),
                    dates: a.date.as_deref().map(|d| format_date(d, lang)),
                    summary: text(&a.summary),
                    ..Entry::default()
                })
                .collect(),
        );
        push(
            SectionKind::Publications,
            profile
                .publications
                .iter()
                .map(|p| Entry {
                    title: text(&p.name).unwrap_or_default(),
                    subtitle: text(&p.publisher),
                    dates: p.release_date.as_deref().map(|d| format_date(d, lang)),
                    summary: text(&p.summary),
                    url: text(&p.url),
                    ..Entry::default()
                })
                .collect(),
        );
        push(
            SectionKind::Volunteer,
            profile
                .volunteer
                .iter()
                .map(|v| Entry {
                    title: text(&v.position)
                        .or_else(|| text(&v.organization))
                        .unwrap_or_default(),
                    subtitle: text(&v.position).and(text(&v.organization)),
                    dates: date_range(&v.start_date, &v.end_date, lang),
                    summary: text(&v.summary),
                    bullets: clean(&v.highlights),
                    url: text(&v.url),
                    ..Entry::default()
                })
                .collect(),
        );
        push(
            SectionKind::Languages,
            profile
                .languages
                .iter()
                .map(|l| Entry {
                    title: text(&l.language).unwrap_or_default(),
                    subtitle: text(&l.fluency),
                    ..Entry::default()
                })
                .collect(),
        );
        push(
            SectionKind::Interests,
            profile
                .interests
                .iter()
                .map(|i| Entry {
                    title: text(&i.name).unwrap_or_default(),
                    summary: (!i.keywords.is_empty()).then(|| clean(&i.keywords).join(", ")),
                    ..Entry::default()
                })
                .collect(),
        );

        Self {
            lang,
            header: Header::new(profile),
            sections,
        }
    }
}

impl Header {
    fn new(profile: &Profile) -> Self {
        let b = &profile.basics;
        let mut contacts = Vec::new();
        if let Some(email) = text(&b.email) {
            contacts.push(Contact {
                url: Some(format!("mailto:{email}")),
                text: email,
            });
        }
        if let Some(phone) = text(&b.phone) {
            contacts.push(Contact {
                url: Some(format!("tel:{}", phone.replace(' ', ""))),
                text: phone,
            });
        }
        if let Some(url) = text(&b.url) {
            contacts.push(Contact {
                text: display_url(&url),
                url: Some(url),
            });
        }
        for p in &b.profiles {
            if let Some(url) = text(&p.url) {
                contacts.push(Contact {
                    text: display_url(&url),
                    url: Some(url),
                });
            } else if let (Some(network), Some(user)) = (text(&p.network), text(&p.username)) {
                contacts.push(Contact {
                    text: format!("{network}: {user}"),
                    url: None,
                });
            }
        }

        Self {
            name: text(&b.name).unwrap_or_default(),
            label: text(&b.label),
            summary: text(&b.summary),
            location: b.location.as_ref().and_then(format_location),
            contacts,
        }
    }
}

impl Entry {
    fn is_empty(&self) -> bool {
        self.title.is_empty() && self.summary.is_none() && self.bullets.is_empty()
    }
}

fn text(value: &Option<String>) -> Option<String> {
    value
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_owned)
}

fn clean(items: &[String]) -> Vec<String> {
    items
        .iter()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(str::to_owned)
        .collect()
}

fn education_details(score: &Option<String>, courses: &[String], lang: Lang) -> Option<String> {
    let (grade, courses_label) = match lang {
        Lang::En => ("Grade", "Courses"),
        Lang::De => ("Note", "Schwerpunkte"),
    };
    let courses = clean(courses);
    let parts: Vec<String> = [
        text(score).map(|s| format!("{grade}: {s}")),
        (!courses.is_empty()).then(|| format!("{courses_label}: {}", courses.join(", "))),
    ]
    .into_iter()
    .flatten()
    .collect();
    (!parts.is_empty()).then(|| parts.join(" · "))
}

fn format_location(loc: &Location) -> Option<String> {
    let parts: Vec<String> = [&loc.city, &loc.region, &loc.country_code]
        .into_iter()
        .filter_map(text)
        .collect();
    (!parts.is_empty()).then(|| parts.join(", "))
}

fn display_url(url: &str) -> String {
    let url = url
        .strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"))
        .unwrap_or(url);
    let url = url.strip_prefix("www.").unwrap_or(url);
    url.trim_end_matches('/').to_owned()
}

/// Formats a JSON Resume date (`YYYY`, `YYYY-MM` or `YYYY-MM-DD`). Anything else is
/// returned unchanged, so free-form dates from sources survive.
pub fn format_date(date: &str, lang: Lang) -> String {
    const EN: [&str; 12] = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ];
    let date = date.trim();
    let mut parts = date.split('-');
    let (Some(year), month) = (parts.next(), parts.next()) else {
        return date.to_owned();
    };
    if year.len() != 4 || !year.bytes().all(|b| b.is_ascii_digit()) {
        return date.to_owned();
    }
    let Some(month) = month else {
        return year.to_owned();
    };
    match month.parse::<usize>() {
        Ok(m @ 1..=12) => match lang {
            Lang::En => format!("{} {year}", EN[m - 1]),
            Lang::De => format!("{m:02}/{year}"),
        },
        _ => date.to_owned(),
    }
}

fn date_range(start: &Option<String>, end: &Option<String>, lang: Lang) -> Option<String> {
    let present = match lang {
        Lang::En => "present",
        Lang::De => "heute",
    };
    match (text(start), text(end)) {
        (Some(s), Some(e)) if s == e => Some(format_date(&s, lang)),
        (Some(s), Some(e)) => Some(format!(
            "{} – {}",
            format_date(&s, lang),
            format_date(&e, lang)
        )),
        (Some(s), None) => Some(format!("{} – {present}", format_date(&s, lang))),
        (None, Some(e)) => Some(format_date(&e, lang)),
        (None, None) => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_dates() {
        assert_eq!(format_date("2022-04", Lang::En), "Apr 2022");
        assert_eq!(format_date("2019-05-01", Lang::En), "May 2019");
        assert_eq!(format_date("2022-04", Lang::De), "04/2022");
        assert_eq!(format_date("2021", Lang::En), "2021");
        assert_eq!(format_date("Summer 2020", Lang::En), "Summer 2020");
        assert_eq!(format_date("2020-13", Lang::En), "2020-13");
    }

    #[test]
    fn formats_ranges() {
        let s = |v: &str| Some(v.to_owned());
        assert_eq!(
            date_range(&s("2020-01"), &s("2022-03"), Lang::En).unwrap(),
            "Jan 2020 – Mar 2022"
        );
        assert_eq!(
            date_range(&s("2022-04"), &None, Lang::De).unwrap(),
            "04/2022 – heute"
        );
        assert_eq!(date_range(&None, &None, Lang::En), None);
    }

    #[test]
    fn skips_empty_sections_and_entries() {
        let profile =
            Profile::from_json(r#"{ "work": [{}], "skills": [{ "name": "Rust" }] }"#).unwrap();
        let view = View::new(&profile, Lang::En);
        assert_eq!(view.sections.len(), 1);
        assert_eq!(view.sections[0].kind, SectionKind::Skills);
    }

    #[test]
    fn work_title_is_position_with_company_subtitle() {
        let profile =
            Profile::from_json(r#"{ "work": [{ "name": "Acme", "position": "Engineer" }] }"#)
                .unwrap();
        let entry = &View::new(&profile, Lang::En).sections[0].entries[0];
        assert_eq!(entry.title, "Engineer");
        assert_eq!(entry.subtitle.as_deref(), Some("Acme"));
    }

    #[test]
    fn shortens_contact_urls() {
        assert_eq!(display_url("https://www.example.com/"), "example.com");
    }
}
