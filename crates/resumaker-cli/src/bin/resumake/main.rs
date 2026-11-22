mod generate;

use std::path::PathBuf;
use std::process::ExitCode;

use clap::Parser;
use resumaker_core::{Format, ProviderKind};

/// Make a resume tailored to a job, with an optional cover letter.
#[derive(Parser, Debug)]
#[command(name = "resumake", version)]
struct Args {
    /// The job to tailor to: text, a file or a URL. Repeatable.
    #[arg(short = 'c', long, help_heading = "Job")]
    context: Vec<String>,

    /// Also write a cover letter.
    #[arg(short = 'm', long, help_heading = "Cover letter")]
    cover_letter: bool,

    /// Notes or files for the cover letter only. Repeatable. Implies -m.
    #[arg(short = 'C', long, help_heading = "Cover letter")]
    cover_context: Vec<String>,

    /// Write only the cover letter, no resume.
    #[arg(long, help_heading = "Cover letter")]
    cover_only: bool,

    /// Extra experience files for this run. Repeatable, globs allowed.
    #[arg(short = 'f', long, help_heading = "Sources")]
    file: Vec<PathBuf>,

    /// Ignore the configured sources and use only --file.
    #[arg(long, help_heading = "Sources")]
    no_sources: bool,

    /// Output folder. Default: from setup.
    #[arg(short = 'o', long, help_heading = "Output")]
    out: Option<PathBuf>,

    /// Export formats for this run, comma-separated. Default: from setup.
    #[arg(long, value_enum, value_delimiter = ',', help_heading = "Output")]
    format: Vec<Format>,

    /// Template name or .typ file for this run. Default: from setup.
    #[arg(short = 's', long, help_heading = "Output")]
    style: Option<String>,

    /// Output language, e.g. en, de. Default: from setup.
    #[arg(short = 'l', long, help_heading = "Output")]
    lang: Option<String>,

    /// Maximum resume length in pages. Default: from setup.
    #[arg(
        long,
        value_parser = clap::value_parser!(u8).range(1..=2),
        help_heading = "Output"
    )]
    pages: Option<u8>,

    /// Reuse today's application folder instead of creating a new one.
    #[arg(short = 'y', long, help_heading = "Output")]
    force: bool,

    /// Inference provider for this run. Default: from setup.
    #[arg(long, value_enum, help_heading = "Model")]
    provider: Option<ProviderKind>,

    /// Model for this run. Default: from setup.
    #[arg(long, help_heading = "Model")]
    model: Option<String>,

    /// Review and revise interactively, then save your feedback.
    #[arg(short = 'F', long, help_heading = "Run")]
    feedback: bool,

    /// Fail instead of warning when the fact check still finds unsupported claims.
    #[arg(long, help_heading = "Run")]
    strict: bool,

    /// Show the sources and plan without calling the model.
    #[arg(long, help_heading = "Run")]
    dry_run: bool,

    /// Show each pipeline step.
    #[arg(short = 'v', long, help_heading = "Run")]
    verbose: bool,
}

impl Args {
    /// A cover letter is written if it was asked for directly,
    /// or implied by --cover-context or --cover-only.
    fn wants_cover_letter(&self) -> bool {
        self.cover_letter || self.cover_only || !self.cover_context.is_empty()
    }
}

fn main() -> ExitCode {
    match generate::run(Args::parse()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e:#}");
            ExitCode::FAILURE
        }
    }
}
