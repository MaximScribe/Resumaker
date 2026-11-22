//! The interactive setup wizard, started by `resumaker` (and by `resumake` on first run).

use std::fmt;
use std::io::IsTerminal;
use std::path::{Path, PathBuf};

use anyhow::{Context, bail};
use inquire::validator::Validation;
use inquire::{Confirm, InquireError, MultiSelect, Password, PasswordDisplayMode, Select, Text};
use resumaker_core::config::{ApiSource, Inference, expand_home};
use resumaker_core::llm::{self, Endpoint, Prompt};
use resumaker_core::secrets::{SecretKey, Secrets};
use resumaker_core::sources::Collection;
use resumaker_core::{Config, Format, ProviderKind, Template};

pub const BANNER: &str = r"
  ____                                 _
 |  _ \ ___  ___ _   _ _ __ ___   __ _| | _____ _ __
 | |_) / _ \/ __| | | | '_ ` _ \ / _` | |/ / _ \ '__|
 |  _ <  __/\__ \ |_| | | | | | | (_| |   <  __/ |
 |_| \_\___||___/\__,_|_| |_| |_|\__,_|_|\_\___|_|
";

/// Runs the full setup, or the settings menu when a config already exists.
pub fn run() -> anyhow::Result<()> {
    if !std::io::stdin().is_terminal() || !std::io::stdout().is_terminal() {
        bail!("the setup wizard needs an interactive terminal; use `resumaker set …` in scripts");
    }
    let path = Config::path()?;
    let existing = Config::load(&path)?;
    println!("{BANNER}");
    let result = match existing {
        None => first_run(&path),
        Some(config) => menu(&path, config),
    };
    match result {
        Err(e) if is_cancel(&e) => {
            println!("\nSetup cancelled; nothing was changed.");
            Ok(())
        }
        Err(e) if is_not_tty(&e) => bail!(
            "the setup wizard needs an interactive terminal; use `resumaker set …` in scripts"
        ),
        other => other,
    }
}

fn first_run(path: &Path) -> anyhow::Result<()> {
    println!(" Welcome! Five quick steps and you're ready to make resumes.\n");
    let mut config = Config::default();
    let secrets = Secrets::default();
    step(1, "Inference provider");
    inference(&mut config, &secrets)?;
    step(2, "Experience sources");
    sources(&mut config, &secrets)?;
    step(3, "Resume style");
    style(&mut config)?;
    step(4, "Output folder");
    output_folder(&mut config)?;
    step(5, "Export formats");
    formats(&mut config)?;
    language(&mut config)?;
    config.save(path)?;
    println!("\n Saved {}", path.display());
    println!(" Make your first resume with:  resumake -c \"<the job you're applying for>\"");
    Ok(())
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum MenuItem {
    Provider,
    Sources,
    Style,
    Output,
    Formats,
    Language,
    Restart,
    Done,
}

impl fmt::Display for MenuItem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Provider => "Inference provider and model",
            Self::Sources => "Experience sources",
            Self::Style => "Resume style",
            Self::Output => "Output folder",
            Self::Formats => "Export formats",
            Self::Language => "Language",
            Self::Restart => "Run the full setup again",
            Self::Done => "Done",
        })
    }
}

fn menu(path: &Path, mut config: Config) -> anyhow::Result<()> {
    let secrets = Secrets::default();
    let items = vec![
        MenuItem::Provider,
        MenuItem::Sources,
        MenuItem::Style,
        MenuItem::Output,
        MenuItem::Formats,
        MenuItem::Language,
        MenuItem::Restart,
        MenuItem::Done,
    ];
    loop {
        summary(&config);
        let choice = Select::new("What would you like to change?", items.clone())
            .with_page_size(items.len())
            .prompt()?;
        match choice {
            MenuItem::Provider => inference(&mut config, &secrets)?,
            MenuItem::Sources => sources(&mut config, &secrets)?,
            MenuItem::Style => style(&mut config)?,
            MenuItem::Output => output_folder(&mut config)?,
            MenuItem::Formats => formats(&mut config)?,
            MenuItem::Language => language(&mut config)?,
            MenuItem::Restart => return first_run(path),
            MenuItem::Done => return Ok(()),
        }
        config.save(path)?;
        println!(" Saved.\n");
    }
}

fn summary(config: &Config) {
    let model = config.inference.model.as_deref().unwrap_or("no model");
    println!(
        " Provider  {} ({model})",
        config.inference.provider.display_name()
    );
    match &config.sources.folder {
        Some(f) => println!(" Sources   {}", f.display()),
        None => println!(" Sources   no folder"),
    }
    for api in &config.sources.apis {
        println!("           {}", api.url);
    }
    println!(" Style     {}", config.output.style);
    println!(" Output    {}", config.output.folder().display());
    let formats: Vec<String> = config
        .output
        .formats
        .iter()
        .map(|f| f.to_string())
        .collect();
    println!(" Formats   {}", formats.join(", "));
    println!(" Language  {}\n", config.output.lang);
}

fn step(n: u8, title: &str) {
    println!("\n [{n}/5] {title}");
}

struct ProviderChoice(ProviderKind);

impl fmt::Display for ProviderChoice {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let p = self.0;
        match p {
            ProviderKind::Ollama => write!(f, "{} (local, recommended)", p.display_name()),
            _ if p.is_local() => write!(f, "{} (local)", p.display_name()),
            _ => write!(f, "{} (cloud)", p.display_name()),
        }
    }
}

fn inference(config: &mut Config, secrets: &Secrets) -> anyhow::Result<()> {
    let choices: Vec<ProviderChoice> = ProviderKind::ALL.into_iter().map(ProviderChoice).collect();
    let start = ProviderKind::ALL
        .iter()
        .position(|p| *p == config.inference.provider)
        .unwrap_or(0);
    let provider = Select::new("Which model provider should write your resumes?", choices)
        .with_starting_cursor(start)
        .with_page_size(ProviderKind::ALL.len())
        .with_help_message("Local providers keep your data on this machine")
        .prompt()?
        .0;

    let mut inference = Inference {
        provider,
        model: None,
        url: None,
    };
    if provider == config.inference.provider {
        inference.url = config.inference.url.clone();
    }

    if provider.is_local() {
        let default = inference
            .url
            .clone()
            .or_else(|| provider.default_url().map(str::to_owned));
        let mut prompt = Text::new("Server address:").with_validator(|s: &str| {
            Ok(if s.starts_with("http://") || s.starts_with("https://") {
                Validation::Valid
            } else {
                Validation::Invalid("start with http:// or https://".into())
            })
        });
        if let Some(d) = default.as_deref() {
            prompt = prompt.with_default(d);
        }
        let url = prompt.prompt()?;
        inference.url = (Some(url.as_str()) != provider.default_url()).then_some(url);
    } else {
        println!(
            " Note: with {} your source files are sent to a cloud service.",
            provider.display_name()
        );
        let key = SecretKey::Provider(provider);
        let replace = match secrets.get(key)? {
            Some(_) => Confirm::new("A key is already stored. Replace it?")
                .with_default(false)
                .prompt()?,
            None => true,
        };
        if replace {
            let value = Password::new(&format!("{} API key:", provider.display_name()))
                .without_confirmation()
                .with_display_mode(PasswordDisplayMode::Masked)
                .prompt()?;
            secrets.set(key, &value)?;
        }
    }

    inference.model = choose_model(&inference, secrets, config.inference.model.as_deref())?;
    config.inference = inference;
    Ok(())
}

fn choose_model(
    inference: &Inference,
    secrets: &Secrets,
    current: Option<&str>,
) -> anyhow::Result<Option<String>> {
    let endpoint = Endpoint::from_config(inference, secrets)?;
    let llm = llm::connect(endpoint.clone());
    println!(" Connecting to {}…", endpoint.base_url);
    let models = match llm.list_models() {
        Ok(models) if !models.is_empty() => models,
        Ok(_) => {
            println!(" The server reports no models.");
            return manual_model(current);
        }
        Err(e) => {
            println!(" Could not list models: {e}");
            if inference.provider.is_local() {
                println!(" Is {} running?", inference.provider.display_name());
            }
            return manual_model(current);
        }
    };
    let start = current
        .and_then(|c| models.iter().position(|m| m == c))
        .unwrap_or(0);
    let model = Select::new("Model:", models)
        .with_starting_cursor(start)
        .with_help_message("Type to filter")
        .prompt()?;

    let test = Confirm::new("Send a short test prompt?")
        .with_default(true)
        .prompt()?;
    if test {
        let mut endpoint = endpoint;
        endpoint.model = model.clone();
        let mut prompt = Prompt::new("You are a connection test.", "Reply with the word OK.");
        prompt.max_tokens = 256;
        match llm::connect(endpoint).complete(&prompt) {
            Ok(_) => println!(" The model answered. All good."),
            Err(e) => {
                println!(" The test failed: {e}\n You can change this later with `resumaker`.")
            }
        }
    }
    Ok(Some(model))
}

fn manual_model(current: Option<&str>) -> anyhow::Result<Option<String>> {
    let mut prompt = Text::new("Model id (leave empty to choose later):");
    if let Some(c) = current {
        prompt = prompt.with_default(c);
    }
    let model = prompt.prompt()?;
    Ok((!model.trim().is_empty()).then(|| model.trim().to_owned()))
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum SourceChoice {
    Folder,
    Api,
    Both,
    Skip,
}

impl fmt::Display for SourceChoice {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Folder => "A folder of files (CVs, notes, certificates, spreadsheets)",
            Self::Api => "An API (your own backend)",
            Self::Both => "Both",
            Self::Skip => "Skip for now (pass files with resumake -f)",
        })
    }
}

fn sources(config: &mut Config, secrets: &Secrets) -> anyhow::Result<()> {
    let choice = Select::new(
        "Where should Resumaker read your experience from?",
        vec![
            SourceChoice::Folder,
            SourceChoice::Api,
            SourceChoice::Both,
            SourceChoice::Skip,
        ],
    )
    .prompt()?;
    if matches!(choice, SourceChoice::Folder | SourceChoice::Both) {
        config.sources.folder = Some(folder_prompt(config.sources.folder.as_deref())?);
    }
    if matches!(choice, SourceChoice::Api | SourceChoice::Both) {
        let url = Text::new("API URL:")
            .with_validator(|s: &str| {
                Ok(if s.starts_with("http://") || s.starts_with("https://") {
                    Validation::Valid
                } else {
                    Validation::Invalid("start with http:// or https://".into())
                })
            })
            .prompt()?;
        let token = Confirm::new("Does it need a bearer token?")
            .with_default(false)
            .prompt()?;
        if token {
            let value = Password::new("Token:")
                .without_confirmation()
                .with_display_mode(PasswordDisplayMode::Masked)
                .prompt()?;
            secrets.set(SecretKey::Api(&url), &value)?;
        }
        config.sources.apis.retain(|a| a.url != url);
        config.sources.apis.push(ApiSource {
            url,
            adapter: None,
            token,
        });
    }
    Ok(())
}

fn folder_prompt(current: Option<&Path>) -> anyhow::Result<PathBuf> {
    let mut prompt = Text::new("Folder:").with_validator(|s: &str| {
        Ok(if expand_home(Path::new(s.trim())).is_dir() {
            Validation::Valid
        } else {
            Validation::Invalid("not a folder".into())
        })
    });
    let current = current.map(|p| p.display().to_string());
    if let Some(c) = current.as_deref() {
        prompt = prompt.with_default(c);
    }
    let input = prompt.prompt()?;
    let folder = std::path::absolute(expand_home(Path::new(input.trim())))
        .context("resolving the folder")?;
    let mut collection = Collection::default();
    collection.add_folder(&folder)?;
    println!(
        " Found {} readable file(s){}.",
        collection.documents.len(),
        if collection.skipped.is_empty() {
            String::new()
        } else {
            format!(", {} skipped", collection.skipped.len())
        }
    );
    Ok(folder)
}

fn style(config: &mut Config) -> anyhow::Result<()> {
    const CUSTOM: &str = "Use my own Typst template (.typ file)";
    let mut options: Vec<String> = Template::builtin_names().map(str::to_owned).collect();
    if let Ok(dir) = resumaker_core::template::user_dir() {
        options.extend(
            resumaker_core::template::installed(&dir)
                .into_iter()
                .map(|e| e.name),
        );
    }
    options.push(CUSTOM.to_owned());
    let start = options
        .iter()
        .position(|o| *o == config.output.style)
        .unwrap_or(0);
    let choice = Select::new("Resume template:", options)
        .with_starting_cursor(start)
        .with_help_message("Preview with `resumaker templates preview <name>`")
        .prompt()?;
    if choice == CUSTOM {
        let path = Text::new("Path to the .typ file:")
            .with_validator(|s: &str| {
                Ok(match Template::resolve(s.trim()) {
                    Ok(_) => Validation::Valid,
                    Err(e) => Validation::Invalid(e.to_string().into()),
                })
            })
            .prompt()?;
        config.output.style = path.trim().to_owned();
    } else {
        config.output.style = choice;
    }
    Ok(())
}

fn output_folder(config: &mut Config) -> anyhow::Result<()> {
    let default = config.output.folder().display().to_string();
    let input = Text::new("Save resumes in:")
        .with_default(&default)
        .prompt()?;
    config.output.folder = Some(PathBuf::from(input.trim()));
    Ok(())
}

fn formats(config: &mut Config) -> anyhow::Result<()> {
    let defaults: Vec<usize> = Format::ALL
        .iter()
        .enumerate()
        .filter(|(_, f)| config.output.formats.contains(f))
        .map(|(i, _)| i)
        .collect();
    let chosen = MultiSelect::new("Export resumes as:", Format::ALL.to_vec())
        .with_default(&defaults)
        .with_help_message("Space to select, Enter to confirm")
        .with_validator(|list: &[inquire::list_option::ListOption<&Format>]| {
            Ok(if list.is_empty() {
                Validation::Invalid("choose at least one format".into())
            } else {
                Validation::Valid
            })
        })
        .prompt()?;
    config.output.formats = chosen;
    Ok(())
}

fn language(config: &mut Config) -> anyhow::Result<()> {
    let options = vec!["English (en)", "Deutsch (de)", "Other"];
    let start = match config.output.lang.as_str() {
        "en" => 0,
        "de" => 1,
        _ => 2,
    };
    let choice = Select::new("Resume language:", options)
        .with_starting_cursor(start)
        .prompt()?;
    config.output.lang = match choice {
        "English (en)" => "en".into(),
        "Deutsch (de)" => "de".into(),
        _ => Text::new("Language code (e.g. fr, es):")
            .prompt()?
            .trim()
            .to_ascii_lowercase(),
    };
    Ok(())
}

fn is_cancel(e: &anyhow::Error) -> bool {
    matches!(
        e.downcast_ref::<InquireError>(),
        Some(InquireError::OperationCanceled | InquireError::OperationInterrupted)
    )
}

fn is_not_tty(e: &anyhow::Error) -> bool {
    matches!(e.downcast_ref::<InquireError>(), Some(InquireError::NotTTY))
}
