use std::io::IsTerminal;
use std::path::{Path, PathBuf};

use anyhow::{Context, bail};
use resumaker_cli::wizard;
use resumaker_core::config::{Config, expand_home};
use resumaker_core::export::Options;
use resumaker_core::feedback;
use resumaker_core::letter::{self, LetterRequest};
use resumaker_core::llm::{self, Endpoint, Llm};
use resumaker_core::pipeline::context::{self, ContextOrigin};
use resumaker_core::pipeline::{self, TailorRequest, Target};
use resumaker_core::secrets::Secrets;
use resumaker_core::sources::Collection;
use resumaker_core::{Error, Format, Profile, Template};

use crate::Args;

pub fn run(args: Args) -> anyhow::Result<()> {
    if args.feedback && !(std::io::stdin().is_terminal() && std::io::stdout().is_terminal()) {
        bail!("the feedback loop (-F) needs an interactive terminal");
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
    let job = load_contexts(&args.context, "job context")?;
    let notes = load_contexts(&args.cover_context, "cover letter notes")?;
    if job.is_empty() {
        println!("no job context given: writing general-purpose documents");
    }
    let wants_letter = args.wants_cover_letter();

    if args.dry_run {
        println!(
            "would use {} ({}), template `{}`, language {lang}, {pages} page(s)",
            inference.provider.display_name(),
            inference.model.as_deref().unwrap_or("no model"),
            template.name(),
        );
        let mut docs = Vec::new();
        if !args.cover_only {
            docs.push("resume");
        }
        if wants_letter {
            docs.push("cover letter");
        }
        let formats: Vec<String> = formats.iter().map(|f| f.to_string()).collect();
        println!(
            "would write {} as {} to {}",
            docs.join(" and "),
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

    let feedback_dir = feedback::dir(config.sources.folder.as_deref().map(expand_home).as_deref())?;
    let saved = feedback::preferences(&feedback_dir)?;
    if !saved.is_empty() {
        println!("using {} point(s) of saved feedback", saved.len());
    }

    let session = Session {
        llm: llm.as_ref(),
        profile,
        job,
        notes,
        lang: lang.clone(),
        pages,
        formats,
        options: pipeline::export_options(&lang, template, pages),
        resume: !args.cover_only,
        letter: wants_letter,
        out_base,
        force: args.force,
    };
    let output = session.produce(&saved, None)?;
    output.print();
    if args.feedback {
        feedback_loop(&session, saved, output, &feedback_dir)?;
    }
    Ok(())
}

/// Asks for feedback, regenerates into the same folder, and saves what was said.
fn feedback_loop(
    session: &Session,
    saved: Vec<String>,
    mut output: Output,
    feedback_dir: &Path,
) -> anyhow::Result<()> {
    let mut points: Vec<String> = Vec::new();
    loop {
        let answer = match inquire::Text::new("How did it turn out?")
            .with_help_message("Describe what to change, or press Enter to finish")
            .prompt()
        {
            Ok(a) => a,
            Err(
                inquire::InquireError::OperationCanceled
                | inquire::InquireError::OperationInterrupted,
            ) => String::new(),
            Err(e) => return Err(e.into()),
        };
        let answer = answer.trim();
        if answer.is_empty() {
            break;
        }
        points.push(answer.to_owned());
        let mut preferences = saved.clone();
        preferences.extend(points.iter().cloned());
        output = session.produce(&preferences, Some(&output.dir))?;
        output.print();
    }
    if points.is_empty() {
        return Ok(());
    }
    let label = output
        .dir
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .and_then(|n| n.split_once('_').map(|(_, rest)| rest.to_owned()))
        .unwrap_or_else(|| "general".to_owned());
    let about = format!("the {label} application");
    let path = feedback::save(feedback_dir, &pipeline::today(), &label, &about, &points)?;
    println!(
        "feedback saved to {}; future runs will take it into account",
        path.display()
    );
    Ok(())
}

/// Everything needed to (re)produce one application's documents.
struct Session<'a> {
    llm: &'a dyn Llm,
    profile: Profile,
    job: Vec<String>,
    notes: Vec<String>,
    lang: String,
    pages: u8,
    formats: Vec<Format>,
    options: Options,
    resume: bool,
    letter: bool,
    out_base: PathBuf,
    force: bool,
}

struct Output {
    dir: PathBuf,
    files: Vec<PathBuf>,
}

impl Output {
    fn print(&self) {
        println!("saved in {}", self.dir.display());
        for f in &self.files {
            let name = f
                .file_name()
                .map(|n| n.to_string_lossy())
                .unwrap_or_default();
            println!("  ✔ {name}");
        }
    }
}

impl Session<'_> {
    /// Writes the documents. `dir` reuses an existing application folder.
    fn produce(&self, preferences: &[String], dir: Option<&Path>) -> anyhow::Result<Output> {
        let mut target = Target::default();
        let mut tailored = None;
        if self.resume {
            println!("tailoring the resume…");
            let request = TailorRequest {
                job: self.job.clone(),
                preferences: preferences.to_vec(),
                lang: self.lang.clone(),
                pages: self.pages,
            };
            let t = pipeline::tailor(self.llm, &self.profile, &request)?;
            target = t.target.clone();
            tailored = Some(t.profile);
        }
        let mut letter = None;
        if self.letter {
            println!("writing the cover letter…");
            let request = LetterRequest {
                job: self.job.clone(),
                notes: self.notes.clone(),
                preferences: preferences.to_vec(),
                lang: self.lang.clone(),
            };
            // The letter draws on the tailored resume when there is one, so both agree.
            let facts = tailored.as_ref().unwrap_or(&self.profile);
            let (t, l) = letter::write(self.llm, facts, &request)?;
            if target == Target::default() {
                target = t;
            }
            letter = Some(l);
        }
        if let Some(t) = describe(&target) {
            println!("target: {t}");
        }

        let dir = match dir {
            Some(d) => d.to_path_buf(),
            None => pipeline::application_dir(&self.out_base, &target, self.force),
        };
        let mut files = Vec::new();
        if let Some(profile) = &tailored {
            files.extend(pipeline::write_outputs(
                profile,
                &self.formats,
                &self.options,
                &dir,
                "resume",
            )?);
        }
        if let Some(l) = &letter {
            let date = letter::long_date(&pipeline::today(), self.options.lang);
            let sender = tailored.as_ref().unwrap_or(&self.profile);
            std::fs::create_dir_all(&dir).with_context(|| format!("creating {}", dir.display()))?;
            for &format in &self.formats {
                let bytes = letter::render(l, sender, format, self.options.lang, &date)?;
                let path = dir.join(format!("cover-letter.{}", format.extension()));
                std::fs::write(&path, bytes)
                    .with_context(|| format!("writing {}", path.display()))?;
                files.push(path);
            }
        }
        Ok(Output { dir, files })
    }
}

fn load_contexts(items: &[String], what: &str) -> anyhow::Result<Vec<String>> {
    let mut texts = Vec::new();
    for item in items {
        let c = context::load(item).with_context(|| format!("reading {what} {item:?}"))?;
        let origin = match &c.origin {
            ContextOrigin::Text => "text".to_owned(),
            ContextOrigin::File(f) => f.clone(),
            ContextOrigin::Url(u) => u.clone(),
        };
        println!("{what}: {origin} ({} chars)", c.text.chars().count());
        texts.push(c.text);
    }
    Ok(texts)
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
fn expand_glob(pattern: &Path) -> anyhow::Result<Vec<PathBuf>> {
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
