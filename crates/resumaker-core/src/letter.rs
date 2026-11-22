//! Cover letters: written from the same facts as the resume.

use std::fmt::Write as _;
use std::io::Cursor;

use docx_rs::{Docx, LineSpacing, PageMargin, Paragraph, Run, RunFonts};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::export::{Lang, pdf};
use crate::llm::{self, Llm, Prompt};
use crate::pipeline::{Target, language_name};
use crate::template::Template;
use crate::{Error, Format, Profile};

const PROMPT: &str = include_str!("../assets/prompts/cover-letter.md");
const TEMPLATE: &str = include_str!("../assets/templates/letter.typ");

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Letter {
    pub recipient: Recipient,
    pub subject: String,
    pub salutation: String,
    pub paragraphs: Vec<String>,
    pub closing: String,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Recipient {
    pub company: Option<String>,
    pub contact: Option<String>,
    pub address: Option<String>,
}

/// Inputs for writing a cover letter.
#[derive(Debug, Clone, Default)]
pub struct LetterRequest {
    /// Job descriptions or postings; empty for a generic letter.
    pub job: Vec<String>,
    /// Things only the letter should mention (motivation, availability, …).
    pub notes: Vec<String>,
    pub preferences: Vec<String>,
    pub lang: String,
}

#[derive(Deserialize)]
struct Answer {
    #[serde(default)]
    target: Target,
    letter: Letter,
}

fn schema() -> Value {
    let s = json!({ "type": "string" });
    json!({
        "type": "object",
        "properties": {
            "target": { "type": "object", "properties": { "company": s, "role": s } },
            "letter": {
                "type": "object",
                "properties": {
                    "recipient": { "type": "object", "properties": { "company": s, "contact": s, "address": s } },
                    "subject": s,
                    "salutation": s,
                    "paragraphs": { "type": "array", "items": s },
                    "closing": s,
                },
                "required": ["subject", "salutation", "paragraphs", "closing"],
            },
        },
        "required": ["letter"],
    })
}

/// Writes a cover letter from the candidate's profile.
pub fn write(
    llm: &dyn Llm,
    profile: &Profile,
    req: &LetterRequest,
) -> Result<(Target, Letter), Error> {
    let system = PROMPT.replace("{language}", language_name(&req.lang));
    let mut facts = profile.clone();
    facts.sources.clear();
    let mut user = format!("<profile>\n{}\n</profile>\n\n", facts.to_json()?);
    if req.job.is_empty() {
        user.push_str(
            "<job>\nNo specific job: write a generic letter that can be adapted.\n</job>\n",
        );
    }
    for job in &req.job {
        let _ = write!(user, "<job>\n{}\n</job>\n", job.trim());
    }
    for note in &req.notes {
        let _ = write!(user, "<notes>\n{}\n</notes>\n", note.trim());
    }
    if !req.preferences.is_empty() {
        user.push_str("<preferences>\n");
        for p in &req.preferences {
            let _ = writeln!(user, "- {}", p.trim());
        }
        user.push_str("</preferences>\n");
    }
    let prompt = Prompt::new(system, user).with_schema("cover_letter", schema());
    let answer: Answer = llm::complete_json(llm, &prompt)?;
    let mut letter = answer.letter;
    letter.paragraphs.retain(|p| !p.trim().is_empty());
    if letter.paragraphs.is_empty() {
        return Err(Error::Llm("the cover letter came back empty".into()));
    }
    Ok((answer.target, letter))
}

/// The sender block, taken from the profile.
#[derive(Debug, Clone, PartialEq, Default, Serialize)]
struct Sender {
    name: String,
    lines: Vec<String>,
}

impl Sender {
    fn from(profile: &Profile) -> Self {
        let b = &profile.basics;
        let location = b.location.as_ref().and_then(|l| {
            let parts: Vec<&str> = [&l.address, &l.postal_code, &l.city]
                .into_iter()
                .filter_map(|x| x.as_deref())
                .filter(|x| !x.trim().is_empty())
                .collect();
            (!parts.is_empty()).then(|| parts.join(" "))
        });
        let lines = [location, b.email.clone(), b.phone.clone()]
            .into_iter()
            .flatten()
            .filter(|l| !l.trim().is_empty())
            .collect();
        Self {
            name: b.name.clone().unwrap_or_default(),
            lines,
        }
    }
}

/// Renders the letter in one format. `date` is pre-formatted, e.g. "22 November 2026".
pub fn render(
    letter: &Letter,
    profile: &Profile,
    format: Format,
    lang: Lang,
    date: &str,
) -> Result<Vec<u8>, Error> {
    let sender = Sender::from(profile);
    Ok(match format {
        Format::Pdf => {
            let data =
                json!({ "letter": letter, "sender": sender, "date": date, "lang": lang.code() });
            pdf::compile(
                &Template::new("letter", TEMPLATE),
                [("letter.json", serde_json::to_vec(&data)?)],
            )?
        }
        Format::Json => serde_json::to_vec_pretty(letter)?,
        Format::Md => markdown(letter, &sender, date).into_bytes(),
        Format::Txt => text(letter, &sender, date).into_bytes(),
        Format::Html => html(letter, &sender, date, lang).into_bytes(),
        Format::Docx => docx(letter, &sender, date)?,
    })
}

fn recipient_lines(r: &Recipient) -> Vec<String> {
    [&r.company, &r.contact, &r.address]
        .into_iter()
        .flatten()
        .flat_map(|s| s.lines().map(str::to_owned).collect::<Vec<_>>())
        .filter(|l| !l.trim().is_empty())
        .collect()
}

fn text(letter: &Letter, sender: &Sender, date: &str) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "{}", sender.name);
    for l in &sender.lines {
        let _ = writeln!(out, "{l}");
    }
    out.push('\n');
    for l in recipient_lines(&letter.recipient) {
        let _ = writeln!(out, "{l}");
    }
    let _ = writeln!(
        out,
        "\n{date}\n\n{}\n\n{}\n",
        letter.subject, letter.salutation
    );
    for p in &letter.paragraphs {
        let _ = writeln!(out, "{}\n", p.trim());
    }
    let _ = writeln!(out, "{}\n{}", letter.closing, sender.name);
    out
}

fn markdown(letter: &Letter, sender: &Sender, date: &str) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "**{}**  ", sender.name);
    for l in &sender.lines {
        let _ = writeln!(out, "{l}  ");
    }
    out.push('\n');
    for l in recipient_lines(&letter.recipient) {
        let _ = writeln!(out, "{l}  ");
    }
    let _ = writeln!(
        out,
        "\n{date}\n\n**{}**\n\n{}\n",
        letter.subject, letter.salutation
    );
    for p in &letter.paragraphs {
        let _ = writeln!(out, "{}\n", p.trim());
    }
    let _ = writeln!(out, "{}  \n{}", letter.closing, sender.name);
    out
}

fn html(letter: &Letter, sender: &Sender, date: &str, lang: Lang) -> String {
    let e = html_escape;
    let mut out = format!(
        "<!doctype html>\n<html lang=\"{}\">\n<head>\n<meta charset=\"utf-8\">\n<title>{}</title>\n\
         <style>body{{font:16px/1.6 Georgia,serif;max-width:700px;margin:48px auto;padding:0 16px;color:#1f2328}}\
         .from{{text-align:right;color:#57606a}}.to{{margin:2em 0 1em}}.date{{text-align:right}}h1{{font-size:1em}}</style>\n\
         </head>\n<body>\n<div class=\"from\"><strong>{}</strong>",
        lang.code(),
        e(&letter.subject),
        e(&sender.name)
    );
    for l in &sender.lines {
        let _ = write!(out, "<br>{}", e(l));
    }
    out.push_str("</div>\n<div class=\"to\">");
    out.push_str(
        &recipient_lines(&letter.recipient)
            .iter()
            .map(|l| e(l))
            .collect::<Vec<_>>()
            .join("<br>"),
    );
    let _ = write!(
        out,
        "</div>\n<p class=\"date\">{}</p>\n<h1>{}</h1>\n<p>{}</p>\n",
        e(date),
        e(&letter.subject),
        e(&letter.salutation)
    );
    for p in &letter.paragraphs {
        let _ = writeln!(out, "<p>{}</p>", e(p.trim()));
    }
    let _ = write!(
        out,
        "<p>{}<br>{}</p>\n</body>\n</html>\n",
        e(&letter.closing),
        e(&sender.name)
    );
    out
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn docx(letter: &Letter, sender: &Sender, date: &str) -> Result<Vec<u8>, Error> {
    let line = |text: &str| Paragraph::new().add_run(Run::new().add_text(text));
    let gap = |p: Paragraph, before: u32| p.line_spacing(LineSpacing::new().before(before));
    let mut doc = Docx::new()
        .default_fonts(RunFonts::new().ascii("Calibri").hi_ansi("Calibri"))
        .default_size(22)
        .page_margin(
            PageMargin::new()
                .top(1300)
                .bottom(1300)
                .left(1400)
                .right(1400),
        )
        .add_paragraph(Paragraph::new().add_run(Run::new().add_text(&sender.name).bold()));
    for l in &sender.lines {
        doc = doc.add_paragraph(line(l));
    }
    for (i, l) in recipient_lines(&letter.recipient).iter().enumerate() {
        let p = line(l);
        doc = doc.add_paragraph(if i == 0 { gap(p, 480) } else { p });
    }
    doc = doc
        .add_paragraph(gap(line(date), 360))
        .add_paragraph(gap(
            Paragraph::new().add_run(Run::new().add_text(&letter.subject).bold()),
            360,
        ))
        .add_paragraph(gap(line(&letter.salutation), 240));
    for p in &letter.paragraphs {
        doc = doc.add_paragraph(gap(line(p.trim()), 160));
    }
    doc = doc
        .add_paragraph(gap(line(&letter.closing), 240))
        .add_paragraph(gap(line(&sender.name), 480));
    let mut buf = Cursor::new(Vec::new());
    doc.build()
        .pack(&mut buf)
        .map_err(|e| Error::Render(format!("docx: {e}")))?;
    Ok(buf.into_inner())
}

/// A long-form date for letters: "22 November 2026" or "22. November 2026".
pub fn long_date(iso: &str, lang: Lang) -> String {
    const EN: [&str; 12] = [
        "January",
        "February",
        "March",
        "April",
        "May",
        "June",
        "July",
        "August",
        "September",
        "October",
        "November",
        "December",
    ];
    const DE: [&str; 12] = [
        "Januar",
        "Februar",
        "März",
        "April",
        "Mai",
        "Juni",
        "Juli",
        "August",
        "September",
        "Oktober",
        "November",
        "Dezember",
    ];
    let parts: Vec<&str> = iso.split('-').collect();
    let (Some(y), Some(m), Some(d)) = (parts.first(), parts.get(1), parts.get(2)) else {
        return iso.to_owned();
    };
    let (Ok(m), Ok(d)) = (m.parse::<usize>(), d.parse::<u32>()) else {
        return iso.to_owned();
    };
    if !(1..=12).contains(&m) {
        return iso.to_owned();
    }
    match lang {
        Lang::En => format!("{d} {} {y}", EN[m - 1]),
        Lang::De => format!("{d}. {} {y}", DE[m - 1]),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Letter {
        Letter {
            recipient: Recipient {
                company: Some("Acme Robotics".into()),
                contact: Some("Hiring Team".into()),
                address: Some("1 Main St\nMunich".into()),
            },
            subject: "Application: Robotics Engineer".into(),
            salutation: "Dear Hiring Team,".into(),
            paragraphs: vec!["First <para>.".into(), "Second.".into()],
            closing: "Kind regards,".into(),
        }
    }

    #[test]
    fn dates() {
        assert_eq!(long_date("2026-11-22", Lang::En), "22 November 2026");
        assert_eq!(long_date("2026-03-01", Lang::De), "1. März 2026");
        assert_eq!(long_date("garbage", Lang::En), "garbage");
    }

    #[test]
    fn renders_every_format() {
        let profile = Profile::example();
        for format in Format::ALL {
            let bytes = render(&sample(), &profile, format, Lang::En, "22 November 2026").unwrap();
            assert!(!bytes.is_empty(), "{format}");
        }
        let pdf = render(&sample(), &profile, Format::Pdf, Lang::De, "1. März 2026").unwrap();
        assert!(pdf.starts_with(b"%PDF-"));
    }

    #[test]
    fn text_layout() {
        let txt = String::from_utf8(
            render(&sample(), &Profile::example(), Format::Txt, Lang::En, "D").unwrap(),
        )
        .unwrap();
        assert!(txt.starts_with("Alex Morgan\n"));
        assert!(txt.contains("Acme Robotics\nHiring Team\n1 Main St\nMunich\n"));
        assert!(txt.trim_end().ends_with("Kind regards,\nAlex Morgan"));
        let html = String::from_utf8(
            render(&sample(), &Profile::example(), Format::Html, Lang::En, "D").unwrap(),
        )
        .unwrap();
        assert!(html.contains("First &lt;para&gt;."));
    }
}
