use resumaker_core::sources::Collection;

use crate::settings;

/// Lists what `resumake` will read.
pub fn list() -> anyhow::Result<()> {
    let (_, config) = settings::load()?;
    if config.sources.folder.is_none() && config.sources.apis.is_empty() {
        println!("no sources configured; use `resumaker set source folder <path>`");
        return Ok(());
    }

    if let Some(folder) = &config.sources.folder {
        println!("folder {}", folder.display());
        let mut collection = Collection::default();
        collection.add_folder(folder)?;
        for doc in &collection.documents {
            let chars = doc.text.chars().count();
            let note = if doc.truncated { ", truncated" } else { "" };
            println!(
                "  {:<28} {chars:>7} chars{note}  {}",
                doc.source.id, doc.source.location
            );
        }
        for (path, reason) in &collection.skipped {
            println!("  skipped {}: {reason}", path.display());
        }
        println!(
            "  {} file(s), {} chars in total",
            collection.documents.len(),
            collection.total_chars()
        );
    }
    for api in &config.sources.apis {
        let auth = if api.token { " (token)" } else { "" };
        println!("api    {}{auth}", api.url);
    }
    Ok(())
}
