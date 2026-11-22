//! Adapters map a known API's response onto a [`Profile`].
//!
//! To add one: implement [`Adapter`] in a new module and list it in [`ADAPTERS`].

mod github;

use serde_json::Value;

use crate::{Error, Profile};

pub trait Adapter: Sync {
    /// Name used in the config's `adapter` field.
    fn name(&self) -> &'static str;

    /// Whether this adapter recognises the URL when no adapter is configured.
    fn matches(&self, url: &str) -> bool;

    fn convert(&self, response: &Value) -> Result<Profile, Error>;
}

pub static ADAPTERS: &[&dyn Adapter] = &[&github::GitHubRepos];

/// The adapter named `name`, or one that recognises `url`.
pub fn find(name: Option<&str>, url: &str) -> Option<&'static dyn Adapter> {
    match name {
        Some(n) => ADAPTERS.iter().copied().find(|a| a.name() == n),
        None => ADAPTERS.iter().copied().find(|a| a.matches(url)),
    }
}
