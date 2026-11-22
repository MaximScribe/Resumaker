//! Turning a reference resume PDF into an installed template.

use std::path::{Path, PathBuf};

use anyhow::Context;
use resumaker_core::export::{self, Options};
use resumaker_core::llm::{self, Endpoint};
use resumaker_core::secrets::Secrets;
use resumaker_core::{Config, Format, Profile, Template, style, template};

/// A template generated from a reference PDF.
pub struct Generated {
    pub name: String,
    pub file: PathBuf,
    pub preview: PathBuf,
}

/// Analyses `pdf`, writes `<name>.typ` to the templates folder and renders a preview.
pub fn create(pdf: &Path, config: &Config) -> anyhow::Result<Generated> {
    println!("reading {}…", pdf.display());
    let facts = style::analyze_pdf(pdf)?;
    let fonts = if facts.fonts.is_empty() {
        "none found".to_owned()
    } else {
        facts.fonts.join(", ")
    };
    println!(
        "  fonts: {fonts}; accent: {}",
        facts.accent.as_deref().unwrap_or("none")
    );

    let llm = Endpoint::from_config(&config.inference, &Secrets::default())
        .ok()
        .filter(|e| !e.model.is_empty())
        .map(llm::connect);
    match &llm {
        Some(_) => println!("  asking the model about the layout…"),
        None => println!(
            "  no model configured: using a default layout with the PDF's fonts and colour"
        ),
    }
    let spec = style::infer(&facts, llm.as_deref())?;
    println!(
        "  layout: {}, headings: {}, dates: {}, density: {}",
        spec.layout, spec.heading, spec.dates, spec.density
    );

    let stem = pdf
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();
    let mut name: String = stem
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-");
    if name.is_empty() {
        name = "my-style".into();
    }
    if Template::builtin(&name).is_some() {
        name.push_str("-style");
    }

    let origin = pdf
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let template = style::to_template(&name, &origin, &spec);
    let dir = template::user_dir()?;
    std::fs::create_dir_all(&dir).with_context(|| format!("creating {}", dir.display()))?;
    let file = dir.join(format!("{name}.typ"));
    std::fs::write(&file, template.source())
        .with_context(|| format!("writing {}", file.display()))?;

    let preview = dir.join(format!("{name}-preview.pdf"));
    let options = Options {
        template,
        lang: config.output.lang(),
        pages: Some(config.output.pages),
    };
    let bytes = export::render(&Profile::example(), Format::Pdf, &options)?;
    std::fs::write(&preview, bytes).with_context(|| format!("writing {}", preview.display()))?;
    Ok(Generated {
        name,
        file,
        preview,
    })
}
