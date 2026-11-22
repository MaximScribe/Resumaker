mod feedback;
mod settings;
mod sources;
mod templates;

use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use resumaker_core::{Format, ProviderKind};

/// Set up Resumaker and manage its settings. Run without a command to start the setup wizard.
#[derive(Parser, Debug)]
#[command(name = "resumaker", version)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Change a setting without the wizard.
    #[command(subcommand)]
    Set(SetCommand),

    /// Manage cloud API keys.
    #[command(subcommand)]
    Auth(AuthCommand),

    /// List configured sources.
    Sources {
        #[command(subcommand)]
        action: Option<SourcesCommand>,
    },

    /// Manage resume templates.
    #[command(subcommand)]
    Templates(TemplatesCommand),

    /// View or delete saved feedback.
    #[command(subcommand)]
    Feedback(FeedbackCommand),

    /// Show the config file location and contents.
    Config,
}

#[derive(Subcommand, Debug)]
enum SetCommand {
    /// Change the inference provider.
    Provider {
        #[arg(value_enum)]
        provider: ProviderKind,
        /// Model to use with this provider.
        #[arg(long)]
        model: Option<String>,
        /// Server address, if not the provider's default.
        #[arg(long)]
        url: Option<String>,
    },

    /// Set where experience is read from.
    #[command(subcommand)]
    Source(SourceCommand),

    /// Set the resume style: a built-in template or a .typ file.
    Style { style: String },

    /// Set the folder where resumes are saved.
    Output { path: PathBuf },

    /// Set the default export formats, comma-separated.
    Formats {
        #[arg(value_enum, value_delimiter = ',', required = true)]
        formats: Vec<Format>,
    },

    /// Set the output language, e.g. en, de.
    Lang { code: String },

    /// Set the maximum resume length in pages.
    Pages {
        #[arg(value_parser = clap::value_parser!(u8).range(1..=2))]
        pages: u8,
    },
}

#[derive(Subcommand, Debug)]
enum SourceCommand {
    /// Use a folder of experience files.
    Folder { path: PathBuf },

    /// Add an API source.
    Api {
        url: String,
        /// Adapter to read the API with; detected automatically if omitted.
        #[arg(long)]
        adapter: Option<String>,
        /// Ask for a bearer token and store it in the keychain.
        #[arg(long)]
        token: bool,
    },

    /// Remove an API source.
    RemoveApi { url: String },
}

#[derive(Subcommand, Debug)]
enum AuthCommand {
    /// Add or replace a provider's API key.
    Add {
        #[arg(value_enum)]
        provider: ProviderKind,
    },
    /// Show which providers have a key.
    List,
    /// Delete a provider's API key.
    Remove {
        #[arg(value_enum)]
        provider: ProviderKind,
    },
    /// Send a test request. Tests the configured provider if none is given.
    Test {
        #[arg(value_enum)]
        provider: Option<ProviderKind>,
    },
}

#[derive(Subcommand, Debug)]
enum SourcesCommand {
    /// Fetch and merge all sources, then print the profile as JSON.
    Pull,
}

#[derive(Subcommand, Debug)]
enum TemplatesCommand {
    /// List installed templates.
    List,
    /// Download the preset template pack.
    Download,
    /// Render a template with example data.
    Preview {
        /// Built-in template name or path to a .typ file.
        name: String,
        /// Output folder.
        #[arg(short, long, default_value = ".")]
        out: PathBuf,
        /// Formats to write, comma-separated.
        #[arg(long, value_enum, value_delimiter = ',', default_value = "pdf")]
        format: Vec<Format>,
    },
}

#[derive(Subcommand, Debug)]
enum FeedbackCommand {
    /// List saved feedback.
    List,
    /// Delete all saved feedback.
    Clear {
        /// Do not ask for confirmation.
        #[arg(short, long)]
        yes: bool,
    },
}

fn main() -> ExitCode {
    match run(Cli::parse()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e:#}");
            ExitCode::FAILURE
        }
    }
}

fn run(cli: Cli) -> anyhow::Result<()> {
    match cli.command {
        None => resumaker_cli::wizard::run(),
        Some(Command::Set(cmd)) => settings::set(cmd),
        Some(Command::Auth(cmd)) => settings::auth(cmd),
        Some(Command::Config) => settings::show(),
        Some(Command::Sources { action: None }) => sources::list(),
        Some(Command::Sources {
            action: Some(SourcesCommand::Pull),
        }) => sources::pull(),
        Some(Command::Feedback(FeedbackCommand::List)) => feedback::list(),
        Some(Command::Feedback(FeedbackCommand::Clear { yes })) => feedback::clear(yes),
        Some(Command::Templates(TemplatesCommand::List)) => templates::list(),
        Some(Command::Templates(TemplatesCommand::Preview { name, out, format })) => {
            templates::preview(&name, &out, &format)
        }
        Some(command) => anyhow::bail!("not implemented yet: {command:?}"),
    }
}
