use std::path::Path;

use anyhow::Context;
use resumaker_core::export::{self, Options};
use resumaker_core::llm::http;
use resumaker_core::template::{self, DEFAULT};
use resumaker_core::{Format, Profile, Template};

pub fn list() -> anyhow::Result<()> {
    for name in Template::builtin_names() {
        let marker = if name == DEFAULT { " (default)" } else { "" };
        println!("{name:<14} built-in{marker}");
    }
    let installed = template::installed(&template::user_dir()?);
    for entry in &installed {
        println!("{:<14} {}", entry.name, entry.description);
    }
    if installed.is_empty() {
        println!("\nget more with `resumaker templates download`");
    }
    Ok(())
}

pub fn download() -> anyhow::Result<()> {
    let url = template::pack_url();
    let dir = template::user_dir()?;
    println!("downloading templates from {url}");
    let entries = template::download(&url, &dir, http::get_text)?;
    for e in &entries {
        println!("  ✔ {:<12} {}", e.name, e.description);
    }
    println!("installed in {}", dir.display());
    println!("preview one with `resumaker templates preview <name>`");
    Ok(())
}

pub fn preview(name: &str, out: &Path, formats: &[Format]) -> anyhow::Result<()> {
    let template = Template::resolve(name)?;
    let profile = Profile::example();
    let options = Options {
        template,
        ..Options::default()
    };
    std::fs::create_dir_all(out).with_context(|| format!("creating {}", out.display()))?;
    for &format in formats {
        let bytes = export::render(&profile, format, &options)?;
        let path = out.join(format!(
            "{}-preview.{}",
            options.template.name(),
            format.extension()
        ));
        std::fs::write(&path, bytes).with_context(|| format!("writing {}", path.display()))?;
        println!("wrote {}", path.display());
    }
    Ok(())
}
