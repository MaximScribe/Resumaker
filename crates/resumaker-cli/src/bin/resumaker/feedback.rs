use std::io::IsTerminal;
use std::path::PathBuf;

use anyhow::bail;
use resumaker_core::config::expand_home;
use resumaker_core::feedback;

use crate::settings;

fn dir() -> anyhow::Result<PathBuf> {
    let (_, config) = settings::load()?;
    Ok(feedback::dir(
        config.sources.folder.as_deref().map(expand_home).as_deref(),
    )?)
}

pub fn list() -> anyhow::Result<()> {
    let dir = dir()?;
    let entries = feedback::list(&dir)?;
    if entries.is_empty() {
        println!("no saved feedback in {}", dir.display());
        return Ok(());
    }
    for e in &entries {
        println!("{}", e.path.display());
        for p in &e.points {
            println!("  - {p}");
        }
    }
    Ok(())
}

pub fn clear(yes: bool) -> anyhow::Result<()> {
    let dir = dir()?;
    let count = feedback::list(&dir)?.len();
    if count == 0 {
        println!("no saved feedback");
        return Ok(());
    }
    if !yes {
        if !std::io::stdin().is_terminal() {
            bail!("pass --yes to delete feedback without a prompt");
        }
        let confirmed = inquire::Confirm::new(&format!("Delete {count} feedback file(s)?"))
            .with_default(false)
            .prompt()?;
        if !confirmed {
            return Ok(());
        }
    }
    println!("deleted {} feedback file(s)", feedback::clear(&dir)?);
    Ok(())
}
