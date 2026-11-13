use std::path::PathBuf;

use clap::{Parser, Subcommand};
use resumaker_cli::Format;

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
}

#[derive(Subcommand, Debug)]
enum SetCommand {
    /// Change the inference provider.
    Provider {
        /// Provider name, e.g. ollama, lmstudio, openai.
        name: String,
        /// Model to use with this provider.
        #[arg(long)]
        model: Option<String>,
    },

    /// Set where experience is read from.
    #[command(subcommand)]
    Source(SourceCommand),

    /// Set the resume style: a reference PDF or a preset name.
    Style { style: String },

    /// Set the folder where resumes are saved.
    Output { path: PathBuf },

    /// Set the default export formats, comma-separated.
    Formats {
        #[arg(value_enum, value_delimiter = ',', required = true)]
        formats: Vec<Format>,
    },
}

#[derive(Subcommand, Debug)]
enum SourceCommand {
    /// Use a folder of experience files.
    Folder { path: PathBuf },

    /// Add an API source.
    Api {
        url: String,
        /// Bearer token for the API.
        #[arg(long)]
        token: Option<String>,
    },
}

#[derive(Subcommand, Debug)]
enum AuthCommand {
    /// Add or replace a provider's API key.
    Add { provider: String },
    /// List providers with a stored key.
    List,
    /// Delete a provider's API key.
    Remove { provider: String },
    /// Send a test request. Tests the active provider if none is given.
    Test { provider: Option<String> },
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
    Preview { name: String },
}

#[derive(Subcommand, Debug)]
enum FeedbackCommand {
    /// List saved feedback.
    List,
    /// Delete all saved feedback.
    Clear,
}

fn main() {
    let cli = Cli::parse();
    match cli.command {
        None => println!("setup wizard: not implemented yet"),
        Some(command) => println!("{command:#?}"),
    }
}
