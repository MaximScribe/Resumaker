//! Google Gemini's generateContent API.

use serde_json::{Value, json};

use super::http::Headers;
use super::{Endpoint, Llm, Prompt};
use crate::Error;
use crate::provider::ProviderKind;

pub struct Gemini(pub Endpoint);

impl Gemini {
    fn headers(&self) -> Headers {
        self.0
            .api_key
            .iter()
            .map(|k| ("x-goog-api-key", k.clone()))
            .collect()
    }
}

impl Llm for Gemini {
    fn complete(&self, prompt: &Prompt) -> Result<String, Error> {
        let model = self.0.require_model()?;
        let model = model.strip_prefix("models/").unwrap_or(model);
        let mut generation = json!({
            "temperature": prompt.temperature,
            "maxOutputTokens": prompt.max_tokens,
        });
        if let Some(schema) = &prompt.schema {
            generation["responseMimeType"] = json!("application/json");
            generation["responseJsonSchema"] = schema.schema.clone();
        }
        let body = json!({
            "systemInstruction": { "parts": [{ "text": prompt.system }] },
            "contents": [{ "role": "user", "parts": [{ "text": prompt.user }] }],
            "generationConfig": generation,
        });
        let url = format!("{}/models/{model}:generateContent", self.0.base_url);
        let response = self.0.transport.post(&url, &self.headers(), &body)?;

        let candidate = response.pointer("/candidates/0").ok_or_else(|| {
            let reason = response
                .pointer("/promptFeedback/blockReason")
                .and_then(Value::as_str)
                .unwrap_or("no candidates returned");
            Error::Llm(format!("Gemini returned no answer: {reason}"))
        })?;
        if candidate.get("finishReason").and_then(Value::as_str) == Some("MAX_TOKENS") {
            return Err(Error::Truncated);
        }
        let text: String = candidate
            .pointer("/content/parts")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter(|p| p.get("thought").and_then(Value::as_bool) != Some(true))
            .filter_map(|p| p.get("text").and_then(Value::as_str))
            .collect();
        if text.is_empty() {
            return Err(Error::Llm("Gemini returned an empty answer".into()));
        }
        Ok(text)
    }

    fn list_models(&self) -> Result<Vec<String>, Error> {
        let url = format!("{}/models?pageSize=1000", self.0.base_url);
        let response = self.0.transport.get(&url, &self.headers())?;
        let mut models: Vec<String> = response
            .get("models")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter(|m| {
                m.get("supportedGenerationMethods")
                    .and_then(Value::as_array)
                    .is_some_and(|ms| ms.iter().any(|x| x == "generateContent"))
            })
            .filter_map(|m| m.get("name").and_then(Value::as_str))
            .map(|n| n.strip_prefix("models/").unwrap_or(n).to_owned())
            .collect();
        models.sort();
        Ok(models)
    }

    fn provider(&self) -> ProviderKind {
        ProviderKind::Gemini
    }
}

#[cfg(test)]
mod tests {
    use super::super::testing::*;
    use super::*;

    #[test]
    fn builds_request_and_joins_parts() {
        let t = FakeTransport::replying(vec![json!({
            "candidates": [{
                "content": { "parts": [
                    { "text": "thinking…", "thought": true },
                    { "text": "{\"a\":" }, { "text": "1}" }
                ] },
                "finishReason": "STOP"
            }]
        })]);
        let prompt = Prompt::new("sys", "u").with_schema("x", json!({ "type": "object" }));
        let mut ep = endpoint(ProviderKind::Gemini, t.clone());
        ep.model = "models/gemini-test".into();
        assert_eq!(Gemini(ep).complete(&prompt).unwrap(), "{\"a\":1}");
        let (_, url, headers, body) = t.last();
        let body = body.unwrap();
        assert!(url.ends_with("/models/gemini-test:generateContent"));
        assert_eq!(header(&headers, "x-goog-api-key"), Some("test-key"));
        assert_eq!(body["systemInstruction"]["parts"][0]["text"], "sys");
        assert_eq!(
            body["generationConfig"]["responseMimeType"],
            "application/json"
        );
    }

    #[test]
    fn explains_blocked_prompts() {
        let t = FakeTransport::replying(vec![
            json!({ "promptFeedback": { "blockReason": "SAFETY" } }),
        ]);
        let err = Gemini(endpoint(ProviderKind::Gemini, t))
            .complete(&Prompt::new("s", "u"))
            .unwrap_err();
        assert!(err.to_string().contains("SAFETY"));
    }

    #[test]
    fn lists_generation_models_only() {
        let t = FakeTransport::replying(vec![json!({ "models": [
            { "name": "models/gemini-a", "supportedGenerationMethods": ["generateContent"] },
            { "name": "models/embed", "supportedGenerationMethods": ["embedContent"] }
        ] })]);
        let models = Gemini(endpoint(ProviderKind::Gemini, t))
            .list_models()
            .unwrap();
        assert_eq!(models, ["gemini-a"]);
    }
}
