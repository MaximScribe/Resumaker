//! Core library for Resumaker: collects experience, tailors it to a job and exports resumes.
//!
//! This crate never prints or prompts; front ends (the CLI, a future GUI) call into it.

pub mod config;
mod error;
pub mod export;
pub mod feedback;
pub mod letter;
pub mod llm;
pub mod pipeline;
pub mod profile;
pub mod provider;
pub mod secrets;
pub mod sources;
pub mod template;

pub use config::Config;
pub use error::Error;
pub use export::Format;
pub use profile::Profile;
pub use provider::ProviderKind;
pub use template::Template;
