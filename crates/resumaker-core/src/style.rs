//! Templates generated from a reference resume PDF.
//!
//! Fonts and the accent colour are read from the PDF itself. The layout (sidebar,
//! heading style, date column, density, section order) is inferred by the model from
//! the extracted text, or left at sensible defaults without one. The result is a
//! parameterised Typst template: close to the reference, not a pixel-perfect copy.

use std::collections::HashMap;
use std::path::Path;

use lopdf::content::Content;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::llm::{self, Llm, Prompt};
use crate::{Error, Template};

const BASE: &str = include_str!("../assets/templates/styled.typ");
const PROMPT: &str = include_str!("../assets/prompts/style.md");
const SECTION_KINDS: [&str; 10] = [
    "work",
    "projects",
    "education",
    "skills",
    "certificates",
    "awards",
    "publications",
    "volunteer",
    "languages",
    "interests",
];

/// What could be read from the PDF without a model.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct PdfFacts {
    /// Font families, most used first.
    pub fonts: Vec<String>,
    /// The most common non-grey fill colour, as `#rrggbb`.
    pub accent: Option<String>,
    pub pages: usize,
    pub text: String,
}

/// The parameters of a generated template.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct StyleSpec {
    pub layout: String,
    pub heading: String,
    pub dates: String,
    pub density: String,
    pub name_align: String,
    pub order: Vec<String>,
    pub fonts: Vec<String>,
    pub serif: bool,
    pub accent: String,
}

impl Default for StyleSpec {
    fn default() -> Self {
        Self {
            layout: "single".into(),
            heading: "rule".into(),
            dates: "right".into(),
            density: "normal".into(),
            name_align: "left".into(),
            order: SECTION_KINDS.iter().map(|s| (*s).to_owned()).collect(),
            fonts: Vec::new(),
            serif: false,
            accent: "#1f3a5f".into(),
        }
    }
}

/// Reads fonts, colours and text from a PDF.
pub fn analyze_pdf(path: &Path) -> Result<PdfFacts, Error> {
    let bytes = std::fs::read(path).map_err(|e| Error::io(path, e))?;
    let unreadable = |reason: String| Error::Unreadable {
        path: path.to_path_buf(),
        reason,
    };
    let doc = lopdf::Document::load_mem(&bytes).map_err(|e| unreadable(e.to_string()))?;
    let pages = doc.get_pages();

    let mut font_uses: HashMap<String, usize> = HashMap::new();
    let mut colours: HashMap<String, usize> = HashMap::new();
    for page in pages.values() {
        if let Ok(fonts) = doc.get_page_fonts(*page) {
            for font in fonts.values() {
                if let Some(family) = font
                    .get(b"BaseFont")
                    .ok()
                    .and_then(|n| n.as_name().ok())
                    .and_then(|n| font_family(&String::from_utf8_lossy(n)))
                {
                    *font_uses.entry(family).or_default() += 1;
                }
            }
        }
        let Ok(content) = doc
            .get_page_content(*page)
            .map_err(|e| e.to_string())
            .and_then(|c| Content::decode(&c).map_err(|e| e.to_string()))
        else {
            continue;
        };
        for op in &content.operations {
            let values: Vec<f32> = op
                .operands
                .iter()
                .filter_map(|o| o.as_float().ok())
                .collect();
            let rgb = match (op.operator.as_str(), values.as_slice()) {
                ("rg" | "RG" | "sc" | "SC" | "scn" | "SCN", [r, g, b]) => Some((*r, *g, *b)),
                ("k" | "K", [c, m, y, k]) => Some((
                    (1.0 - c) * (1.0 - k),
                    (1.0 - m) * (1.0 - k),
                    (1.0 - y) * (1.0 - k),
                )),
                _ => None,
            };
            if let Some(hex) = rgb.and_then(|(r, g, b)| accent_hex(r, g, b)) {
                *colours.entry(hex).or_default() += 1;
            }
        }
    }

    let text =
        crate::sources::extract::extract(path, crate::sources::FileKind::Pdf).unwrap_or_default();
    let mut fonts: Vec<(String, usize)> = font_uses.into_iter().collect();
    fonts.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    let accent = colours
        .into_iter()
        .max_by(|a, b| a.1.cmp(&b.1).then(b.0.cmp(&a.0)))
        .map(|(c, _)| c);
    Ok(PdfFacts {
        fonts: fonts.into_iter().map(|(f, _)| f).collect(),
        accent,
        pages: pages.len(),
        text,
    })
}

/// "ABCDEF+Calibri-Bold" becomes "Calibri"; "TimesNewRomanPSMT" becomes "Times New Roman".
fn font_family(base: &str) -> Option<String> {
    let name = base.split_once('+').map(|(_, n)| n).unwrap_or(base);
    let name = name.split(['-', ',']).next()?;
    let name = name
        .trim_end_matches("PSMT")
        .trim_end_matches("MT")
        .trim_end_matches("PS");
    let mut out = String::new();
    let chars: Vec<char> = name.chars().collect();
    for (i, &c) in chars.iter().enumerate() {
        let prev = i.checked_sub(1).map(|j| chars[j]);
        let next = chars.get(i + 1);
        let boundary = c.is_uppercase()
            && prev.is_some_and(|p| {
                p.is_lowercase() || (p.is_uppercase() && next.is_some_and(|n| n.is_lowercase()))
            });
        if boundary {
            out.push(' ');
        }
        out.push(c);
    }
    let out = out.trim().to_owned();
    let standard = [
        "Symbol",
        "Zapf Dingbats",
        "Font Awesome",
        "Wingdings",
        "Noto Emoji",
    ];
    let symbol = standard.iter().any(|s| out.starts_with(s)) || out.contains("Emoji");
    (out.len() > 1 && !symbol).then_some(out)
}

fn accent_hex(r: f32, g: f32, b: f32) -> Option<String> {
    let (max, min) = (r.max(g).max(b), r.min(g).min(b));
    // Greys, black and white are body text and backgrounds, not an accent.
    if max - min < 0.15 || max < 0.15 {
        return None;
    }
    let c = |v: f32| (v.clamp(0.0, 1.0) * 255.0).round() as u8;
    Some(format!("#{:02x}{:02x}{:02x}", c(r), c(g), c(b)))
}

fn is_serif(family: &str) -> bool {
    let f = family.to_ascii_lowercase();
    !f.contains("sans")
        && [
            "times",
            "garamond",
            "georgia",
            "cambria",
            "libertinus",
            "palatino",
            "baskerville",
            "minion",
            "charter",
            "computer modern",
            "book",
            "serif",
            "caslon",
            "bodoni",
        ]
        .iter()
        .any(|s| f.contains(s))
}

/// Combines what was read from the PDF with the model's layout judgement.
pub fn infer(facts: &PdfFacts, llm: Option<&dyn Llm>) -> Result<StyleSpec, Error> {
    let mut spec = StyleSpec::default();
    if let Some(llm) = llm {
        let sample: String = facts.text.chars().take(6000).collect();
        let user = format!(
            "<facts>\npages: {}\nfonts: {}\naccent colour: {}\n</facts>\n\n<text>\n{sample}\n</text>",
            facts.pages,
            facts.fonts.join(", "),
            facts.accent.as_deref().unwrap_or("none"),
        );
        let s = json!({ "type": "string" });
        let schema = json!({
            "type": "object",
            "properties": {
                "layout": { "type": "string", "enum": ["single", "sidebar"] },
                "heading": { "type": "string", "enum": ["rule", "bar", "caps", "plain"] },
                "dates": { "type": "string", "enum": ["left", "right"] },
                "density": { "type": "string", "enum": ["compact", "normal", "airy"] },
                "name_align": { "type": "string", "enum": ["left", "center"] },
                "order": { "type": "array", "items": s },
            },
        });
        let prompt = Prompt::new(PROMPT, user).with_schema("resume_style", schema);
        spec = llm::complete_json(llm, &prompt)?;
    }
    spec.fonts = facts.fonts.iter().take(2).cloned().collect();
    spec.serif = facts.fonts.first().is_some_and(|f| is_serif(f));
    if let Some(a) = &facts.accent {
        spec.accent = a.clone();
    }
    Ok(sanitize(spec))
}

/// Keeps only values the base template understands.
fn sanitize(mut spec: StyleSpec) -> StyleSpec {
    let d = StyleSpec::default();
    let pick = |v: &str, allowed: &[&str], default: &str| {
        if allowed.contains(&v) {
            v.to_owned()
        } else {
            default.to_owned()
        }
    };
    spec.layout = pick(&spec.layout, &["single", "sidebar"], &d.layout);
    spec.heading = pick(&spec.heading, &["rule", "bar", "caps", "plain"], &d.heading);
    spec.dates = pick(&spec.dates, &["left", "right"], &d.dates);
    spec.density = pick(&spec.density, &["compact", "normal", "airy"], &d.density);
    spec.name_align = pick(&spec.name_align, &["left", "center"], &d.name_align);
    let mut order: Vec<String> = Vec::new();
    for k in spec.order.iter().chain(d.order.iter()) {
        if SECTION_KINDS.contains(&k.as_str()) && !order.contains(k) {
            order.push(k.clone());
        }
    }
    spec.order = order;
    spec.fonts.retain(|f| {
        !f.is_empty()
            && f.chars()
                .all(|c| c.is_alphanumeric() || c == ' ' || c == '-')
    });
    let hex = spec.accent.strip_prefix('#').unwrap_or("");
    if hex.len() != 6 || !hex.chars().all(|c| c.is_ascii_hexdigit()) {
        spec.accent = d.accent;
    }
    spec
}

/// Writes the Typst template for a spec.
pub fn to_template(name: &str, origin: &str, spec: &StyleSpec) -> Template {
    let spec = sanitize(spec.clone());
    let quoted = |items: &[String]| {
        let inner: Vec<String> = items.iter().map(|s| format!("\"{s}\"")).collect();
        if inner.len() == 1 {
            format!("({},)", inner[0])
        } else {
            format!("({})", inner.join(", "))
        }
    };
    let origin: String = origin.chars().filter(|c| !c.is_control()).collect();
    let header = format!(
        "// Generated by Resumaker from {origin}. Edit the values below to adjust it.\n\
         #let spec = (\n  layout: \"{}\",\n  heading: \"{}\",\n  dates: \"{}\",\n  density: \"{}\",\n  \
         name-align: \"{}\",\n  order: {},\n  fonts: {},\n  serif: {},\n  accent: rgb(\"{}\"),\n)\n\n",
        spec.layout,
        spec.heading,
        spec.dates,
        spec.density,
        spec.name_align,
        quoted(&spec.order),
        if spec.fonts.is_empty() {
            "()".to_owned()
        } else {
            quoted(&spec.fonts)
        },
        spec.serif,
        spec.accent,
    );
    Template::new(name, format!("{header}{BASE}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::export::{self, Lang, Options};
    use crate::{Format, Profile};

    #[test]
    fn font_families() {
        assert_eq!(
            font_family("ABCDEF+Calibri-Bold").as_deref(),
            Some("Calibri")
        );
        assert_eq!(
            font_family("TimesNewRomanPSMT").as_deref(),
            Some("Times New Roman")
        );
        assert_eq!(
            font_family("SourceSans3-Regular").as_deref(),
            Some("Source Sans3")
        );
        assert_eq!(
            font_family("LibertinusSerif").as_deref(),
            Some("Libertinus Serif")
        );
        assert_eq!(font_family("Symbol"), None);
    }

    #[test]
    fn accents_ignore_greys() {
        assert_eq!(accent_hex(0.12, 0.23, 0.37).as_deref(), Some("#1f3b5e"));
        assert_eq!(accent_hex(0.4, 0.4, 0.42), None);
        assert_eq!(accent_hex(0.0, 0.0, 0.0), None);
    }

    #[test]
    fn sanitizes_untrusted_values() {
        let spec = sanitize(StyleSpec {
            layout: "weird".into(),
            order: vec!["skills".into(), "bogus".into(), "skills".into()],
            fonts: vec!["Evil\") #read(\"x".into(), "Inter".into()],
            accent: "red".into(),
            ..StyleSpec::default()
        });
        assert_eq!(spec.layout, "single");
        assert_eq!(spec.order[0], "skills");
        assert_eq!(spec.order.len(), SECTION_KINDS.len());
        assert_eq!(spec.fonts, ["Inter"]);
        assert_eq!(spec.accent, "#1f3a5f");
    }

    #[test]
    fn every_variant_renders() {
        let variants = [
            ("single", "rule", "right", "normal", "left"),
            ("sidebar", "bar", "left", "compact", "left"),
            ("single", "caps", "left", "airy", "center"),
            ("sidebar", "plain", "right", "normal", "center"),
        ];
        for (layout, heading, dates, density, align) in variants {
            let spec = StyleSpec {
                layout: layout.into(),
                heading: heading.into(),
                dates: dates.into(),
                density: density.into(),
                name_align: align.into(),
                fonts: vec!["Calibri".into()],
                ..StyleSpec::default()
            };
            let template = to_template("test", "test.pdf", &spec);
            for profile in [Profile::example(), Profile::default()] {
                let options = Options {
                    lang: Lang::De,
                    template: template.clone(),
                    pages: Some(1),
                };
                let pdf = export::render(&profile, Format::Pdf, &options)
                    .unwrap_or_else(|e| panic!("{layout}/{heading}: {e}"));
                assert!(pdf.starts_with(b"%PDF-"));
            }
        }
    }

    #[test]
    fn analyzes_a_generated_pdf() {
        let pdf = export::render(&Profile::example(), Format::Pdf, &Options::default()).unwrap();
        let path = std::env::temp_dir().join(format!("resumaker-style-{}.pdf", std::process::id()));
        std::fs::write(&path, pdf).unwrap();
        let facts = analyze_pdf(&path).unwrap();
        std::fs::remove_file(&path).ok();
        assert_eq!(facts.pages, 1);
        assert_eq!(
            facts.fonts.first().map(String::as_str),
            Some("Libertinus Serif")
        );
        assert!(
            facts.accent.is_some(),
            "the classic template uses a blue accent"
        );
        assert!(facts.text.contains("Alex Morgan"));
        let spec = infer(&facts, None).unwrap();
        assert!(spec.serif);
    }
}
