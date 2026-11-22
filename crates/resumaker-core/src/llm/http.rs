//! A minimal JSON-over-HTTP transport, swappable for tests.

use std::time::Duration;

use serde_json::Value;

use crate::Error;

pub type Headers = Vec<(&'static str, String)>;

pub trait Transport: Send + Sync {
    fn get(&self, url: &str, headers: &Headers) -> Result<Value, Error>;
    fn post(&self, url: &str, headers: &Headers, body: &Value) -> Result<Value, Error>;
}

/// The real network transport.
pub struct Http {
    agent: ureq::Agent,
}

impl Http {
    /// `timeout` bounds a whole request; local models on modest hardware can be slow.
    pub fn new(timeout: Duration) -> Self {
        let agent = ureq::Agent::config_builder()
            .timeout_connect(Some(Duration::from_secs(10)))
            .timeout_global(Some(timeout))
            .http_status_as_error(false)
            .user_agent(concat!("resumaker/", env!("CARGO_PKG_VERSION")))
            .build()
            .into();
        Self { agent }
    }

    fn finish(
        url: &str,
        result: Result<ureq::http::Response<ureq::Body>, ureq::Error>,
    ) -> Result<Value, Error> {
        let mut response = result.map_err(|e| Error::Http {
            url: redact(url),
            status: None,
            message: e.to_string(),
        })?;
        let status = response.status();
        let text = response
            .body_mut()
            .with_config()
            .limit(64 * 1024 * 1024)
            .read_to_string()
            .map_err(|e| Error::Http {
                url: redact(url),
                status: Some(status.as_u16()),
                message: e.to_string(),
            })?;
        if !status.is_success() {
            return Err(Error::Http {
                url: redact(url),
                status: Some(status.as_u16()),
                message: error_message(&text),
            });
        }
        serde_json::from_str(&text).map_err(|e| Error::Http {
            url: redact(url),
            status: Some(status.as_u16()),
            message: format!("response is not JSON: {e}"),
        })
    }
}

impl Default for Http {
    fn default() -> Self {
        Self::new(Duration::from_secs(600))
    }
}

impl Transport for Http {
    fn get(&self, url: &str, headers: &Headers) -> Result<Value, Error> {
        let mut req = self.agent.get(url);
        for (k, v) in headers {
            req = req.header(*k, v);
        }
        Self::finish(url, req.call())
    }

    fn post(&self, url: &str, headers: &Headers, body: &Value) -> Result<Value, Error> {
        let mut req = self.agent.post(url);
        for (k, v) in headers {
            req = req.header(*k, v);
        }
        Self::finish(url, req.send_json(body))
    }
}

/// Pulls the human-readable message out of a provider's error body.
fn error_message(body: &str) -> String {
    let parsed: Option<Value> = serde_json::from_str(body).ok();
    parsed
        .as_ref()
        .and_then(|v| {
            v.pointer("/error/message")
                .or_else(|| v.get("error").filter(|e| e.is_string()))
                .or_else(|| v.get("message"))
                .and_then(Value::as_str)
        })
        .map(str::to_owned)
        .unwrap_or_else(|| body.chars().take(300).collect())
}

/// Removes query strings, which may carry API keys, from URLs shown to the user.
fn redact(url: &str) -> String {
    url.split('?').next().unwrap_or(url).to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_error_messages() {
        assert_eq!(
            error_message(r#"{"error":{"message":"bad key"}}"#),
            "bad key"
        );
        assert_eq!(
            error_message(r#"{"error":"model not found"}"#),
            "model not found"
        );
        assert_eq!(error_message("plain failure"), "plain failure");
    }

    #[test]
    fn redacts_query() {
        assert_eq!(redact("https://x/models?key=secret"), "https://x/models");
    }
}
