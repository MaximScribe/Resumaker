//! Anthropic's Messages API. Structured answers are requested by forcing a tool call
//! whose input schema is the answer schema.

use serde_json::{Value, json};

use super::http::Headers;
use super::{Endpoint, Llm, Prompt};
use crate::Error;
use crate::provider::ProviderKind;

const API_VERSION: &str = "2023-06-01";

pub struct Anthropic(pub Endpoint);

impl Anthropic {
    fn headers(&self) -> Headers {
        let mut headers = vec![("anthropic-version", API_VERSION.to_owned())];
        if let Some(key) = &self.0.api_key {
            headers.push(("x-api-key", key.clone()));
        }
        headers
    }
}

impl Llm for Anthropic {
    fn complete(&self, prompt: &Prompt) -> Result<String, Error> {
        let model = self.0.require_model()?;
        let mut body = json!({
            "model": model,
            "max_tokens": prompt.max_tokens,
            "temperature": prompt.temperature,
            "system": prompt.system,
            "messages": [{ "role": "user", "content": prompt.user }],
        });
        if let Some(schema) = &prompt.schema {
            body["tools"] = json!([{
                "name": schema.name,
                "description": "Return the result in this exact structure.",
                "input_schema": schema.schema,
            }]);
            body["tool_choice"] = json!({ "type": "tool", "name": schema.name });
        }
        let url = format!("{}/messages", self.0.base_url);
        let response = self.0.transport.post(&url, &self.headers(), &body)?;
        if response.get("stop_reason").and_then(Value::as_str) == Some("max_tokens") {
            return Err(Error::Truncated);
        }
        let blocks = response
            .get("content")
            .and_then(Value::as_array)
            .ok_or_else(|| Error::Llm("response has no content".into()))?;
        if let Some(input) = blocks
            .iter()
            .find(|b| b.get("type").and_then(Value::as_str) == Some("tool_use"))
            .and_then(|b| b.get("input"))
        {
            return Ok(input.to_string());
        }
        let text: String = blocks
            .iter()
            .filter(|b| b.get("type").and_then(Value::as_str) == Some("text"))
            .filter_map(|b| b.get("text").and_then(Value::as_str))
            .collect();
        if text.is_empty() {
            return Err(Error::Llm("response has no text".into()));
        }
        Ok(text)
    }

    fn list_models(&self) -> Result<Vec<String>, Error> {
        let url = format!("{}/models?limit=1000", self.0.base_url);
        let response = self.0.transport.get(&url, &self.headers())?;
        Ok(response
            .get("data")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(|m| m.get("id").and_then(Value::as_str).map(str::to_owned))
            .collect())
    }

    fn provider(&self) -> ProviderKind {
        ProviderKind::Anthropic
    }
}

#[cfg(test)]
mod tests {
    use super::super::testing::*;
    use super::*;

    #[test]
    fn forces_tool_for_schema() {
        let t = FakeTransport::replying(vec![json!({
            "content": [{ "type": "tool_use", "name": "answer", "input": { "a": 1 } }],
            "stop_reason": "tool_use"
        })]);
        let prompt = Prompt::new("sys", "u").with_schema("answer", json!({ "type": "object" }));
        let out = Anthropic(endpoint(ProviderKind::Anthropic, t.clone()))
            .complete(&prompt)
            .unwrap();
        assert_eq!(out, "{\"a\":1}");
        let (_, url, headers, body) = t.last();
        let body = body.unwrap();
        assert_eq!(url, "https://api.anthropic.com/v1/messages");
        assert_eq!(header(&headers, "x-api-key"), Some("test-key"));
        assert_eq!(header(&headers, "anthropic-version"), Some(API_VERSION));
        assert_eq!(body["tool_choice"]["name"], "answer");
        assert_eq!(body["system"], "sys");
    }

    #[test]
    fn plain_text_answers() {
        let t = FakeTransport::replying(vec![json!({
            "content": [{ "type": "text", "text": "Hello" }], "stop_reason": "end_turn"
        })]);
        let out = Anthropic(endpoint(ProviderKind::Anthropic, t))
            .complete(&Prompt::new("s", "u"))
            .unwrap();
        assert_eq!(out, "Hello");
    }

    #[test]
    fn reports_truncation() {
        let t = FakeTransport::replying(vec![json!({
            "content": [{ "type": "text", "text": "Hel" }], "stop_reason": "max_tokens"
        })]);
        assert!(matches!(
            Anthropic(endpoint(ProviderKind::Anthropic, t)).complete(&Prompt::new("s", "u")),
            Err(Error::Truncated)
        ));
    }
}
