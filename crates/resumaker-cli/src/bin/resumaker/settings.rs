use std::path::{Path, PathBuf};

use anyhow::{Context, bail};
use resumaker_core::config::{ApiSource, expand_home};
use resumaker_core::secrets::{SecretKey, SecretOrigin, Secrets, mask};
use resumaker_core::{Config, ProviderKind, Template};

use crate::{AuthCommand, SetCommand, SourceCommand};

/// The config file path and its current contents (defaults if it does not exist yet).
pub fn load() -> anyhow::Result<(PathBuf, Config)> {
    let path = Config::path()?;
    let config = Config::load(&path)?.unwrap_or_default();
    Ok((path, config))
}

fn save(path: &Path, config: &Config) -> anyhow::Result<()> {
    config.save(path)?;
    println!("saved {}", path.display());
    Ok(())
}

pub fn set(cmd: SetCommand) -> anyhow::Result<()> {
    let (path, mut config) = load()?;
    match cmd {
        SetCommand::Provider {
            provider,
            model,
            url,
        } => {
            if provider == ProviderKind::OpenaiCompatible && url.is_none() {
                bail!("an OpenAI-compatible server needs --url, e.g. http://localhost:8080/v1");
            }
            if config.inference.provider != provider {
                config.inference.model = None;
                config.inference.url = None;
            }
            config.inference.provider = provider;
            config.inference.model = model.or(config.inference.model);
            config.inference.url = url.or(config.inference.url);
            if !provider.is_local() {
                println!(
                    "note: {} is a cloud provider; your source files will be sent to it.",
                    provider.display_name()
                );
                if Secrets::default()
                    .get(SecretKey::Provider(provider))?
                    .is_none()
                {
                    println!("      add a key with `resumaker auth add {provider}`.");
                }
            }
        }
        SetCommand::Source(SourceCommand::Folder { path: folder }) => {
            let resolved = expand_home(&folder);
            if !resolved.is_dir() {
                bail!("{} is not a folder", resolved.display());
            }
            let absolute = std::path::absolute(&resolved)
                .with_context(|| format!("resolving {}", resolved.display()))?;
            config.sources.folder = Some(absolute);
        }
        SetCommand::Source(SourceCommand::Api {
            url,
            adapter,
            token,
        }) => {
            if !(url.starts_with("http://") || url.starts_with("https://")) {
                bail!("the API URL must start with http:// or https://");
            }
            if token {
                let secret = inquire::Password::new("Bearer token:")
                    .without_confirmation()
                    .prompt()?;
                Secrets::default().set(SecretKey::Api(&url), &secret)?;
            }
            config.sources.apis.retain(|a| a.url != url);
            config.sources.apis.push(ApiSource {
                url,
                adapter,
                token,
            });
        }
        SetCommand::Source(SourceCommand::RemoveApi { url }) => {
            let before = config.sources.apis.len();
            config.sources.apis.retain(|a| a.url != url);
            if config.sources.apis.len() == before {
                bail!("no API source with URL {url}");
            }
            Secrets::default().delete(SecretKey::Api(&url))?;
        }
        SetCommand::Style { style } => {
            if style.to_ascii_lowercase().ends_with(".pdf") {
                bail!("reference PDFs are not supported yet; use a template name or a .typ file");
            }
            Template::resolve(&style)?;
            config.output.style = style;
        }
        SetCommand::Output { path: folder } => {
            config.output.folder = Some(folder);
        }
        SetCommand::Formats { formats } => {
            let mut unique = Vec::new();
            for f in formats {
                if !unique.contains(&f) {
                    unique.push(f);
                }
            }
            config.output.formats = unique;
        }
        SetCommand::Lang { code } => {
            let code = code.trim().to_ascii_lowercase();
            if code.len() < 2 || !code.chars().all(|c| c.is_ascii_alphabetic() || c == '-') {
                bail!("expected a language code such as en or de");
            }
            config.output.lang = code;
        }
        SetCommand::Pages { pages } => config.output.pages = pages,
    }
    save(&path, &config)
}

pub fn auth(cmd: AuthCommand) -> anyhow::Result<()> {
    let secrets = Secrets::default();
    match cmd {
        AuthCommand::Add { provider } => {
            if provider.is_local() {
                println!(
                    "{} runs locally and usually needs no key.",
                    provider.display_name()
                );
            }
            let key = inquire::Password::new(&format!("{} API key:", provider.display_name()))
                .without_confirmation()
                .with_display_mode(inquire::PasswordDisplayMode::Masked)
                .prompt()?;
            secrets.set(SecretKey::Provider(provider), &key)?;
            println!("stored {} key in the OS keychain", provider.display_name());
        }
        AuthCommand::List => {
            for provider in ProviderKind::ALL.into_iter().filter(|p| !p.is_local()) {
                let key = SecretKey::Provider(provider);
                let status = match secrets.origin(key)? {
                    Some(SecretOrigin::Env(var)) => format!("from ${var}"),
                    Some(SecretOrigin::Keychain) => {
                        let value = secrets.get(key)?.unwrap_or_default();
                        format!("keychain ({})", mask(&value))
                    }
                    None => "not set".to_owned(),
                };
                println!("{:<12} {status}", provider.id());
            }
        }
        AuthCommand::Remove { provider } => {
            if secrets.delete(SecretKey::Provider(provider))? {
                println!("removed {} key", provider.display_name());
            } else {
                println!("no {} key was stored", provider.display_name());
            }
        }
        AuthCommand::Test { .. } => bail!("provider tests are not implemented yet"),
    }
    Ok(())
}

pub fn show() -> anyhow::Result<()> {
    let path = Config::path()?;
    println!("# {}", path.display());
    match Config::load(&path)? {
        Some(config) => print!("{}", config.to_toml()?),
        None => println!("# not created yet; run `resumaker` to set up"),
    }
    Ok(())
}
