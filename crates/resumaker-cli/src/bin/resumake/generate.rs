use std::io::IsTerminal;
use std::path::PathBuf;

use anyhow::{Context, bail};
use resumaker_cli::wizard;
use resumaker_core::config::{Config, expand_home};
use resumaker_core::llm::{self, Endpoint};
use resumaker_core::pipeline::context::{self, ContextOrigin};
use resumaker_core::pipeline::{self, TailorRequest, Target};
use resumaker_core::secrets::Secrets;
use resumaker_core::sources::Collection;
use resumaker_core::{Error, Template};

use crate::Args;

pub fn run(args: Args) -> anyhow::Result<()> {
    if args.wants_cover_letter() {
        bail!("cover letters are not available yet");
    }
    if args.feedback {
        bail!("the feedback loop is not available yet");
    }

    let config = load_config()?;
    let mut inference = config.inference.clone();
    if let Some(provider) = args.provider {
        if provider != inference.provider {
            inference.model = None;
            inference.url = None;
        }
        inference.provider = provider;
    }
    if let Some(model) = &args.model {
        inference.model = Some(model.clone());
    }
    let lang = args.lang.clone().unwrap_or(config.output.lang.clone());
    let pages = args.pages.unwrap_or(config.output.pages);
    let formats = if args.format.is_empty() {
        config.output.formats.clone()
    } else {
        args.format.clone()
    };
    let style = args.style.clone().unwrap_or(config.output.style.clone());
    let template = Template::resolve(&style)?;
    let out_base = args
        .out
        .as_deref()
        .map(expand_home)
        .unwrap_or_else(|| config.output.folder());

    let collection = collect(&args, &config)?;
    let contexts = args
        .context
        .iter()
        .map(|c| context::load(c).with_context(|| format!("reading job context {c:?}")))
        .collect::<anyhow::Result<Vec<_>>>()?;
    for c in &contexts {
        let what = match &c.origin {
            ContextOrigin::Text => "text".to_owned(),
            ContextOrigin::File(f) => f.clone(),
            ContextOrigin::Url(u) => u.clone(),
        };
        println!("job context: {what} ({} chars)", c.text.chars().count());
    }
    if contexts.is_empty() {
        println!("no job context given: writing a general-purpose resume");
    }

    if args.dry_run {
        println!(
            "would use {} ({}), template `{}`, language {lang}, {pages} page(s)",
            inference.provider.display_name(),
            inference.model.as_deref().unwrap_or("no model"),
            template.name(),
        );
        let formats: Vec<String> = formats.iter().map(|f| f.to_string()).collect();
        println!(
            "would write {} to {}",
            formats.join(", "),
            out_base.display()
        );
        return Ok(());
    }

    let endpoint = Endpoint::from_config(&inference, &Secrets::default())?;
    if endpoint.model.is_empty() {
        return Err(Error::NoModel(inference.provider).into());
    }
    let model_id = endpoint.model.clone();
    let llm = llm::connect(endpoint);
    let cache = Config::dir().ok().map(|d| d.join("cache"));

    println!(
        "merging {} source(s) with {model_id}…",
        collection.documents.len()
    );
    let profile = pipeline::merge(
        llm.as_ref(),
        &model_id,
        &collection.documents,
        cache.as_deref(),
    )?;
    if args.verbose {
        println!(
            "profile: {} positions, {} education entries, {} skill groups, {} projects",
            profile.work.len(),
            profile.education.len(),
            profile.skills.len(),
            profile.projects.len()
        );
    }

    println!("tailoring…");
    let request = TailorRequest {
        job: contexts.into_iter().map(|c| c.text).collect(),
        preferences: Vec::new(),
        lang: lang.clone(),
        pages,
    };
    let tailored = pipeline::tailor(llm.as_ref(), &profile, &request)?;
    if let Some(target) = describe(&tailored.target) {
        println!("target: {target}");
    }

    let dir = pipeline::application_dir(&out_base, &tailored.target, args.force);
    let options = pipeline::export_options(&lang, template, pages);
    let files = pipeline::write_outputs(&tailored.profile, &formats, &options, &dir, "resume")?;
    for f in &files {
        println!("✔ {}", f.display());
    }
    Ok(())
}

/// Loads settings, starting the wizard on first use in an interactive terminal.
fn load_config() -> anyhow::Result<Config> {
    let path = Config::path()?;
    if let Some(config) = Config::load(&path)? {
        return Ok(config);
    }
    if !std::io::stdin().is_terminal() {
        bail!("Resumaker is not set up yet; run `resumaker` first");
    }
    println!("Resumaker is not set up yet. Let's do that first.");
    wizard::run()?;
    Config::load(&path)?.context("setup did not finish")
}

fn collect(args: &Args, config: &Config) -> anyhow::Result<Collection> {
    let mut collection = Collection::default();
    if !args.no_sources {
        if let Some(folder) = &config.sources.folder {
            collection.add_folder(&expand_home(folder))?;
        }
        if !config.sources.apis.is_empty() {
            println!("note: API sources are not supported yet and were skipped");
        }
    }
    for pattern in &args.file {
        for path in expand_glob(pattern)? {
            collection.add_file(&path);
        }
    }
    for (path, reason) in &collection.skipped {
        println!("skipped {}: {reason}", path.display());
    }
    if collection.is_empty() {
        return Err(Error::NoSources.into());
    }
    let truncated = collection.documents.iter().filter(|d| d.truncated).count();
    println!(
        "read {} source file(s){}",
        collection.documents.len(),
        if truncated > 0 {
            format!(" ({truncated} shortened)")
        } else {
            String::new()
        }
    );
    Ok(collection)
}

/// Expands `*`, `?` and `[…]` patterns, which Windows shells leave to the program.
fn expand_glob(pattern: &PathBuf) -> anyhow::Result<Vec<PathBuf>> {
    let text = pattern.to_string_lossy();
    if !text.contains(['*', '?', '[']) {
        return Ok(vec![expand_home(pattern)]);
    }
    let expanded = expand_home(pattern);
    let mut paths: Vec<PathBuf> = glob::glob(&expanded.to_string_lossy())
        .with_context(|| format!("invalid pattern {text}"))?
        .filter_map(Result::ok)
        .filter(|p| p.is_file())
        .collect();
    if paths.is_empty() {
        bail!("no files match {text}");
    }
    paths.sort();
    Ok(paths)
}

fn describe(target: &Target) -> Option<String> {
    match (&target.role, &target.company) {
        (Some(r), Some(c)) => Some(format!("{r} at {c}")),
        (Some(x), None) | (None, Some(x)) => Some(x.clone()),
        (None, None) => None,
    }
}
