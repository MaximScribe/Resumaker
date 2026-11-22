//! Fetching configured API sources, shared by both binaries.

use std::time::Duration;

use resumaker_core::Config;
use resumaker_core::llm::http::Http;
use resumaker_core::secrets::{SecretKey, Secrets};
use resumaker_core::sources::Collection;
use resumaker_core::sources::api::Mode;

/// Fetches every configured API into `collection` and reports each one.
pub fn fetch_all(config: &Config, collection: &mut Collection) -> anyhow::Result<()> {
    if config.sources.apis.is_empty() {
        return Ok(());
    }
    let secrets = Secrets::default();
    let transport = Http::new(Duration::from_secs(60));
    for api in &config.sources.apis {
        let token = if api.token {
            secrets.get(SecretKey::Api(&api.url))?
        } else {
            None
        };
        if let Some(mode) = collection.add_api(api, token.as_deref(), &transport) {
            let how = match mode {
                Mode::JsonResume => "JSON Resume".to_owned(),
                Mode::Adapter(name) => format!("{name} adapter"),
                Mode::Raw => "raw JSON, read by the model".to_owned(),
            };
            eprintln!("api {} ({how})", api.url);
        }
    }
    Ok(())
}
