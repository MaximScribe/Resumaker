use resumaker_cli::apis;
use resumaker_core::config::expand_home;
use resumaker_core::llm::{self, Endpoint};
use resumaker_core::secrets::Secrets;
use resumaker_core::sources::Collection;
use resumaker_core::{Config, Error, pipeline};

use crate::settings;

fn collect(config: &Config) -> anyhow::Result<Collection> {
    let mut collection = Collection::default();
    if let Some(folder) = &config.sources.folder {
        collection.add_folder(&expand_home(folder))?;
    }
    apis::fetch_all(config, &mut collection)?;
    Ok(collection)
}

/// Lists what `resumake` will read.
pub fn list() -> anyhow::Result<()> {
    let (_, config) = settings::load()?;
    if config.sources.folder.is_none() && config.sources.apis.is_empty() {
        println!("no sources configured; use `resumaker set source folder <path>`");
        return Ok(());
    }
    if let Some(folder) = &config.sources.folder {
        println!("folder {}", folder.display());
    }
    let collection = collect(&config)?;
    for doc in &collection.documents {
        let chars = doc.text.chars().count();
        let note = if doc.truncated { ", truncated" } else { "" };
        println!("  {:<28} {chars:>7} chars{note}", doc.source.id);
    }
    for p in &collection.profiles {
        if let Some(s) = p.sources.first() {
            println!(
                "  {:<28} {} entries (structured)",
                s.id,
                p.source_refs().count()
            );
        }
    }
    for (path, reason) in &collection.skipped {
        println!("  skipped {}: {reason}", path.display());
    }
    println!(
        "{} source(s), {} chars of text",
        collection.documents.len() + collection.profiles.len(),
        collection.total_chars()
    );
    Ok(())
}

/// Builds the merged profile and prints it as JSON.
pub fn pull() -> anyhow::Result<()> {
    let (_, config) = settings::load()?;
    let collection = collect(&config)?;
    for (path, reason) in &collection.skipped {
        eprintln!("skipped {}: {reason}", path.display());
    }
    let endpoint = Endpoint::from_config(&config.inference, &Secrets::default())?;
    if endpoint.model.is_empty() && !collection.documents.is_empty() {
        return Err(Error::NoModel(config.inference.provider).into());
    }
    let model = endpoint.model.clone();
    let llm = llm::connect(endpoint);
    let cache = Config::dir().ok().map(|d| d.join("cache"));
    let profile = pipeline::build_profile(llm.as_ref(), &model, &collection, cache.as_deref())?;
    println!("{}", profile.to_json()?);
    Ok(())
}
