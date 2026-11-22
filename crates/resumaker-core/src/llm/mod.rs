//! Language model access: one interface over local and cloud providers.

mod anthropic;
mod gemini;
pub mod http;
mod ollama;
mod openai;

use std::sync::Arc;

use serde::de::DeserializeOwned;
use serde_json::Value;

use crate::Error;
use crate::config::Inference;
use crate::provider::ProviderKind;
use crate::secrets::{SecretKey, Secrets};
use http::{Http, Transport};

/// One request to a model: instructions, input and, optionally, the JSON shape of the
/// answer.
#[derive(Debug, Clone, PartialEq)]
pub struct Prompt {
    pub system: String,
    pub user: String,
    pub schema: Option<Schema>,
    pub temperature: f32,
    pub max_tokens: u32,
}

impl Prompt {
    pub fn new(system: impl Into<String>, user: impl Into<String>) -> Self {
        Self {
            system: system.into(),
            user: user.into(),
            schema: None,
            temperature: 0.2,
            max_tokens: 8192,
        }
    }

    /// Asks for a JSON answer matching `schema`.
    pub fn with_schema(mut self, name: &str, schema: Value) -> Self {
        self.schema = Some(Schema {
            name: name.to_owned(),
            schema,
        });
        self
    }
}

/// A JSON Schema the answer must follow.
#[derive(Debug, Clone, PartialEq)]
pub struct Schema {
    pub name: String,
    pub schema: Value,
}

pub trait Llm: Send + Sync {
    /// Returns the model's text answer (JSON text when the prompt has a schema).
    fn complete(&self, prompt: &Prompt) -> Result<String, Error>;

    /// Models available to this account or server.
    fn list_models(&self) -> Result<Vec<String>, Error>;

    fn provider(&self) -> ProviderKind;
}

/// Everything needed to reach a provider.
#[derive(Clone)]
pub struct Endpoint {
    pub provider: ProviderKind,
    pub base_url: String,
    pub model: String,
    pub api_key: Option<String>,
    pub transport: Arc<dyn Transport>,
}

impl Endpoint {
    /// Builds an endpoint from settings, reading the API key from `secrets`.
    /// `model` may be empty when only listing models.
    pub fn from_config(inference: &Inference, secrets: &Secrets) -> Result<Self, Error> {
        let provider = inference.provider;
        let base_url = inference
            .base_url()
            .ok_or(Error::NoServerUrl(provider))?
            .trim_end_matches('/')
            .to_owned();
        let api_key = secrets.get(SecretKey::Provider(provider))?;
        if provider.requires_key() && api_key.is_none() {
            return Err(Error::MissingKey(provider));
        }
        Ok(Self {
            provider,
            base_url,
            model: inference.model.clone().unwrap_or_default(),
            api_key,
            transport: Arc::new(Http::default()),
        })
    }

    pub fn with_transport(mut self, transport: Arc<dyn Transport>) -> Self {
        self.transport = transport;
        self
    }

    fn require_model(&self) -> Result<&str, Error> {
        if self.model.trim().is_empty() {
            Err(Error::NoModel(self.provider))
        } else {
            Ok(&self.model)
        }
    }
}

/// Creates a client for the endpoint's provider.
pub fn connect(endpoint: Endpoint) -> Box<dyn Llm> {
    match endpoint.provider {
        ProviderKind::Ollama => Box::new(ollama::Ollama(endpoint)),
        ProviderKind::Gemini => Box::new(gemini::Gemini(endpoint)),
        ProviderKind::Anthropic => Box::new(anthropic::Anthropic(endpoint)),
        ProviderKind::LmStudio
        | ProviderKind::OpenaiCompatible
        | ProviderKind::Openrouter
        | ProviderKind::Openai => Box::new(openai::OpenAi(endpoint)),
    }
}

/// Runs a prompt with a schema and parses the answer. If the answer is not valid
/// JSON for `T`, the model is asked once more with the parse error.
pub fn complete_json<T: DeserializeOwned>(llm: &dyn Llm, prompt: &Prompt) -> Result<T, Error> {
    let first = llm.complete(prompt)?;
    let error = match parse_json(&first) {
        Ok(value) => return Ok(value),
        Err(e) => e,
    };
    let mut retry = prompt.clone();
    retry.user = format!(
        "{}\n\nYour previous answer could not be parsed ({error}). \
         Reply with only the JSON object, no commentary.",
        prompt.user
    );
    let second = llm.complete(&retry)?;
    parse_json(&second).map_err(|e| Error::Llm(format!("model returned invalid JSON: {e}")))
}

/// Parses JSON from a model answer, tolerating code fences and surrounding prose.
pub fn parse_json<T: DeserializeOwned>(text: &str) -> Result<T, serde_json::Error> {
    let trimmed = text.trim();
    if let Ok(v) = serde_json::from_str(trimmed) {
        return Ok(v);
    }
    let unfenced = trimmed
        .strip_prefix("```json")
        .or_else(|| trimmed.strip_prefix("```"))
        .and_then(|s| s.trim_end().strip_suffix("```"))
        .map(str::trim);
    if let Some(inner) = unfenced
        && let Ok(v) = serde_json::from_str(inner)
    {
        return Ok(v);
    }
    match (trimmed.find('{'), trimmed.rfind('}')) {
        (Some(start), Some(end)) if start < end => serde_json::from_str(&trimmed[start..=end]),
        _ => serde_json::from_str(trimmed),
    }
}

/// Text content of an OpenAI-style message, which may be a string or a list of parts.
fn content_text(content: &Value) -> Option<String> {
    match content {
        Value::String(s) => Some(s.clone()),
        Value::Array(parts) => {
            let text: String = parts
                .iter()
                .filter_map(|p| p.get("text").and_then(Value::as_str))
                .collect();
            (!text.is_empty()).then_some(text)
        }
        _ => None,
    }
}

#[cfg(test)]
pub(crate) mod testing {
    use std::sync::Mutex;

    use super::*;
    use http::Headers;

    /// Method, URL, headers and body of a recorded request.
    pub type Recorded = (String, String, Headers, Option<Value>);

    /// Records requests and replies with canned responses.
    #[derive(Default)]
    pub struct FakeTransport {
        pub requests: Mutex<Vec<Recorded>>,
        pub responses: Mutex<Vec<Value>>,
    }

    impl FakeTransport {
        pub fn replying(responses: Vec<Value>) -> Arc<Self> {
            Arc::new(Self {
                responses: Mutex::new(responses.into_iter().rev().collect()),
                ..Self::default()
            })
        }

        pub fn last(&self) -> Recorded {
            self.requests.lock().unwrap().last().cloned().unwrap()
        }

        fn reply(&self) -> Result<Value, Error> {
            self.responses
                .lock()
                .unwrap()
                .pop()
                .ok_or_else(|| Error::Llm("no canned response".into()))
        }
    }

    impl Transport for FakeTransport {
        fn get(&self, url: &str, headers: &Headers) -> Result<Value, Error> {
            self.requests
                .lock()
                .unwrap()
                .push(("GET".into(), url.into(), headers.clone(), None));
            self.reply()
        }

        fn post(&self, url: &str, headers: &Headers, body: &Value) -> Result<Value, Error> {
            self.requests.lock().unwrap().push((
                "POST".into(),
                url.into(),
                headers.clone(),
                Some(body.clone()),
            ));
            self.reply()
        }
    }

    pub fn endpoint(provider: ProviderKind, transport: Arc<FakeTransport>) -> Endpoint {
        Endpoint {
            provider,
            base_url: provider
                .default_url()
                .unwrap_or("http://localhost:8080/v1")
                .to_owned(),
            model: "test-model".into(),
            api_key: Some("test-key".into()),
            transport,
        }
    }

    pub fn header<'a>(headers: &'a Headers, name: &str) -> Option<&'a str> {
        headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(name))
            .map(|(_, v)| v.as_str())
    }
}

#[cfg(test)]
mod tests {
    use serde::Deserialize;
    use serde_json::json;

    use super::testing::*;
    use super::*;

    #[derive(Debug, Deserialize, PartialEq)]
    struct Answer {
        ok: bool,
    }

    #[test]
    fn parses_fenced_and_wrapped_json() {
        let a: Answer = parse_json("```json\n{\"ok\": true}\n```").unwrap();
        assert!(a.ok);
        let a: Answer = parse_json("Sure! Here it is: {\"ok\": true} Hope that helps.").unwrap();
        assert!(a.ok);
        assert!(parse_json::<Answer>("no json here").is_err());
    }

    #[test]
    fn retries_once_on_invalid_json() {
        let transport = FakeTransport::replying(vec![
            json!({ "choices": [{ "message": { "content": "not json" } }] }),
            json!({ "choices": [{ "message": { "content": "{\"ok\": true}" } }] }),
        ]);
        let llm = connect(endpoint(ProviderKind::Openai, transport.clone()));
        let prompt = Prompt::new("sys", "user").with_schema("answer", json!({}));
        let answer: Answer = complete_json(llm.as_ref(), &prompt).unwrap();
        assert!(answer.ok);
        let (_, _, _, body) = transport.last();
        let retry_text = body.unwrap()["messages"][1]["content"]
            .as_str()
            .unwrap()
            .to_owned();
        assert!(retry_text.contains("could not be parsed"));
    }

    #[test]
    fn cloud_provider_without_key_fails() {
        let inference = Inference {
            provider: ProviderKind::Anthropic,
            model: Some("m".into()),
            url: None,
        };
        let secrets = Secrets::new(crate::secrets::MemoryStore::default());
        // SAFETY-free: only reads the environment; the variable is unlikely to be set in CI.
        if std::env::var(ProviderKind::Anthropic.key_env_var()).is_err() {
            assert!(matches!(
                Endpoint::from_config(&inference, &secrets),
                Err(Error::MissingKey(ProviderKind::Anthropic))
            ));
        }
    }

    #[test]
    fn local_provider_needs_no_key() {
        let secrets = Secrets::new(crate::secrets::MemoryStore::default());
        let endpoint = Endpoint::from_config(&Inference::default(), &secrets).unwrap();
        assert_eq!(endpoint.base_url, "http://localhost:11434");
        assert!(matches!(endpoint.require_model(), Err(Error::NoModel(_))));
    }
}
