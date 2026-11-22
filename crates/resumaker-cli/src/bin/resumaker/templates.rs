use std::path::Path;

use anyhow::Context;
use resumaker_core::export::{self, Options};
use resumaker_core::{Format, Profile, Template};

pub fn list() -> anyhow::Result<()> {
    for name in Template::builtin_names() {
        let marker = if name == resumaker_core::template::DEFAULT {
            " (default)"
        } else {
            ""
        };
        println!("{name}{marker}");
    }
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
