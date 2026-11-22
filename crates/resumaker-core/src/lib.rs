//! Core library for Resumaker: collects experience, tailors it to a job and exports resumes.
//!
//! This crate never prints or prompts; front ends (the CLI, a future GUI) call into it.

mod error;
pub mod profile;

pub use error::Error;
pub use profile::Profile;
