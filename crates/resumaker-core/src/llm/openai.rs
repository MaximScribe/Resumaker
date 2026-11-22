//! OpenAI chat completions: OpenAI, OpenRouter, LM Studio and compatible servers.

use serde_json::{Value, json};

use super::http::Headers;
use super::{Endpoint, Llm, Prompt, content_text};
use crate::Error;
use crate::provider::ProviderKind;

pub struct OpenAi(pub Endpoint);

impl OpenAi {
    fn headers(&self) -> Headers {
        let mut headers = Vec::new();
        if let Some(key) = &self.0.api_key {
            headers.push(("Authorization", format!("Bearer {key}")));
        }
        if self.0.provider == ProviderKind::Openrouter {
            headers.push(("X-Title", "Resumaker".to_owned()));
        }
        headers
    }

    fn body(&self, prompt: &Prompt, model: &str) -> Value {
        let mut body = json!({
            "model": model,
            "messages": [
                { "role": "system", "content": prompt.system },
                { "role": "user", "content": prompt.user },
            ],
        });
        // OpenAI's current models reject `max_tokens` and custom temperatures.
        if self.0.provider == ProviderKind::Openai {
            body["max_completion_tokens"] = json!(prompt.max_tokens);
        } else {
            body["max_tokens"] = json!(prompt.max_tokens);
            body["temperature"] = json!(prompt.temperature);
        }
        if let Some(schema) = &prompt.schema {
            body["response_format"] = json!({
                "type": "json_schema",
                "json_schema": { "name": schema.name, "schema": schema.schema, "strict": false },
            });
        }
        body
    }
}

impl Llm for OpenAi {
    fn complete(&self, prompt: &Prompt) -> Result<String, Error> {
        let model = self.0.require_model()?;
        let url = format!("{}/chat/completions", self.0.base_url);
        let response = self
            .0
            .transport
            .post(&url, &self.headers(), &self.body(prompt, model))?;
        let choice = response
            .pointer("/choices/0")
            .ok_or_else(|| Error::Llm("response has no choices".into()))?;
        if choice.get("finish_reason").and_then(Value::as_str) == Some("length") {
            return Err(Error::Truncated);
        }
        choice
            .pointer("/message/content")
            .and_then(content_text)
            .ok_or_else(|| Error::Llm("response has no text".into()))
    }

    fn list_models(&self) -> Result<Vec<String>, Error> {
        let url = format!("{}/models", self.0.base_url);
        let response = self.0.transport.get(&url, &self.headers())?;
        let mut models: Vec<String> = response
            .get("data")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(|m| m.get("id").and_then(Value::as_str).map(str::to_owned))
            .collect();
        models.sort();
        Ok(models)
    }

    fn provider(&self) -> ProviderKind {
        self.0.provider
    }
}

#[cfg(test)]
mod tests {
    use super::super::testing::*;
    use super::*;

    #[test]
    fn sends_schema_and_auth() {
        let t = FakeTransport::replying(vec![json!({
            "choices": [{ "message": { "content": "{\"a\":1}" }, "finish_reason": "stop" }]
        })]);
        let llm = OpenAi(endpoint(ProviderKind::Openrouter, t.clone()));
        let prompt = Prompt::new("sys", "hi").with_schema("thing", json!({ "type": "object" }));
        assert_eq!(llm.complete(&prompt).unwrap(), "{\"a\":1}");
        let (method, url, headers, body) = t.last();
        let body = body.unwrap();
        assert_eq!(method, "POST");
        assert_eq!(url, "https://openrouter.ai/api/v1/chat/completions");
        assert_eq!(header(&headers, "authorization"), Some("Bearer test-key"));
        assert_eq!(body["response_format"]["json_schema"]["name"], "thing");
        assert_eq!(body["messages"][0]["role"], "system");
        assert_eq!(body["max_tokens"], 8192);
    }

    #[test]
    fn openai_uses_completion_token_limit() {
        let t = FakeTransport::replying(vec![json!({
            "choices": [{ "message": { "content": "ok" } }]
        })]);
        OpenAi(endpoint(ProviderKind::Openai, t.clone()))
            .complete(&Prompt::new("s", "u"))
            .unwrap();
        let body = t.last().3.unwrap();
        assert_eq!(body["max_completion_tokens"], 8192);
        assert!(body.get("temperature").is_none());
        assert!(body.get("max_tokens").is_none());
    }

    #[test]
    fn reports_truncation() {
        let t = FakeTransport::replying(vec![json!({
            "choices": [{ "message": { "content": "{\"a\":" }, "finish_reason": "length" }]
        })]);
        let err = OpenAi(endpoint(ProviderKind::LmStudio, t))
            .complete(&Prompt::new("s", "u"))
            .unwrap_err();
        assert!(matches!(err, Error::Truncated));
    }

    #[test]
    fn lists_models_sorted() {
        let t = FakeTransport::replying(vec![json!({ "data": [{ "id": "b" }, { "id": "a" }] })]);
        let models = OpenAi(endpoint(ProviderKind::LmStudio, t.clone()))
            .list_models()
            .unwrap();
        assert_eq!(models, ["a", "b"]);
        assert_eq!(t.last().1, "http://localhost:1234/v1/models");
    }
}
