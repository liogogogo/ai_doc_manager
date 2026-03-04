use super::adapter::{LlmAdapter, LlmError};
use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};

#[derive(Serialize)]
struct OllamaRequest {
    model: String,
    prompt: String,
    stream: bool,
    options: OllamaOptions,
}

#[derive(Serialize)]
struct OllamaOptions {
    num_predict: u32,
}

#[derive(Deserialize)]
struct OllamaResponse {
    response: String,
}

pub struct OllamaAdapter {
    client: Client,
    base_url: String,
    model: String,
}

impl OllamaAdapter {
    pub fn new(base_url: &str, model: &str) -> Self {
        Self {
            client: Client::new(),
            base_url: base_url.to_string(),
            model: model.to_string(),
        }
    }
}

#[async_trait]
impl LlmAdapter for OllamaAdapter {
    async fn complete(&self, prompt: &str, max_tokens: u32) -> Result<String, LlmError> {
        let url = format!("{}/api/generate", self.base_url);
        let req = OllamaRequest {
            model: self.model.clone(),
            prompt: prompt.to_string(),
            stream: false,
            options: OllamaOptions {
                num_predict: max_tokens,
            },
        };

        let resp = self
            .client
            .post(&url)
            .json(&req)
            .send()
            .await
            .map_err(|e| LlmError::ConnectionError(e.to_string()))?;

        if !resp.status().is_success() {
            return Err(LlmError::RequestFailed(format!(
                "HTTP {}",
                resp.status()
            )));
        }

        let body: OllamaResponse = resp
            .json()
            .await
            .map_err(|e| LlmError::InvalidResponse(e.to_string()))?;

        Ok(body.response)
    }

    async fn health_check(&self) -> Result<bool, LlmError> {
        let url = format!("{}/api/tags", self.base_url);
        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| LlmError::ConnectionError(e.to_string()))?;

        Ok(resp.status().is_success())
    }

    fn provider_name(&self) -> &str {
        "ollama"
    }
}
