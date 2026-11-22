//! Ollama's native chat API, which supports JSON Schema output and context sizing.

use serde_json::{Value, json};

use super::http::Headers;
use super::{Endpoint, Llm, Prompt};
use crate::Error;
use crate::provider::ProviderKind;

/// Ollama's default context window is too small for a full set of source files.
const CONTEXT_TOKENS: u32 = 32_768;

pub struct Ollama(pub Endpoint);

impl Ollama {
    fn headers(&self) -> Headers {
        self.0
            .api_key
            .iter()
            .map(|k| ("Authorization", format!("Bearer {k}")))
            .collect()
    }
}

impl Llm for Ollama {
    fn complete(&self, prompt: &Prompt) -> Result<String, Error> {
        let model = self.0.require_model()?;
        let mut body = json!({
            "model": model,
            "stream": false,
            "messages": [
                { "role": "system", "content": prompt.system },
                { "role": "user", "content": prompt.user },
            ],
            "options": {
                "temperature": prompt.temperature,
                "num_predict": prompt.max_tokens,
                "num_ctx": CONTEXT_TOKENS,
            },
        });
        if let Some(schema) = &prompt.schema {
            body["format"] = schema.schema.clone();
        }
        let url = format!("{}/api/chat", self.0.base_url);
        let response = self.0.transport.post(&url, &self.headers(), &body)?;
        if response.get("done_reason").and_then(Value::as_str) == Some("length") {
            return Err(Error::Truncated);
        }
        response
            .pointer("/message/content")
            .and_then(Value::as_str)
            .map(str::to_owned)
            .ok_or_else(|| Error::Llm("response has no text".into()))
    }

    fn list_models(&self) -> Result<Vec<String>, Error> {
        let url = format!("{}/api/tags", self.0.base_url);
        let response = self.0.transport.get(&url, &self.headers())?;
        let mut models: Vec<String> = response
            .get("models")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(|m| m.get("name").and_then(Value::as_str).map(str::to_owned))
            .collect();
        models.sort();
        Ok(models)
    }

    fn provider(&self) -> ProviderKind {
        ProviderKind::Ollama
    }
}

#[cfg(test)]
mod tests {
    use super::super::testing::*;
    use super::*;

    #[test]
    fn uses_native_format_and_large_context() {
        let t = FakeTransport::replying(vec![json!({
            "message": { "role": "assistant", "content": "{}" }, "done": true
        })]);
        let prompt = Prompt::new("s", "u").with_schema("x", json!({ "type": "object" }));
        Ollama(endpoint(ProviderKind::Ollama, t.clone()))
            .complete(&prompt)
            .unwrap();
        let (_, url, headers, body) = t.last();
        let body = body.unwrap();
        assert_eq!(url, "http://localhost:11434/api/chat");
        assert_eq!(body["format"]["type"], "object");
        assert_eq!(body["stream"], false);
        assert_eq!(body["options"]["num_ctx"], CONTEXT_TOKENS);
        assert_eq!(header(&headers, "authorization"), Some("Bearer test-key"));
    }

    #[test]
    fn lists_installed_models() {
        let t = FakeTransport::replying(vec![json!({
            "models": [{ "name": "qwen3:14b" }, { "name": "llama3.2:3b" }]
        })]);
        let models = Ollama(endpoint(ProviderKind::Ollama, t))
            .list_models()
            .unwrap();
        assert_eq!(models, ["llama3.2:3b", "qwen3:14b"]);
    }
}
