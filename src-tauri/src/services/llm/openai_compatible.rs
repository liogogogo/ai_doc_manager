use super::adapter::{LlmAdapter, LlmError};
use async_trait::async_trait;
use futures_util::StreamExt;
use reqwest::Client;
use serde::{Deserialize, Serialize};

/// Models that require chain-of-thought / thinking mode and temperature = 1.0
fn is_thinking_model(model: &str) -> bool {
    let m = model.to_lowercase();
    m.contains("glm-z1")
        || m == "glm-5"
        || m.starts_with("glm-5-")
        || m.contains("deepseek-r1")
        || m.contains("deepseek-reasoner")
        || m.contains("qwq")
        || m.contains("o1-")
        || m == "o1"
        || m == "o3"
        || m.starts_with("o3-")
}

#[derive(Serialize)]
struct ThinkingConfig {
    #[serde(rename = "type")]
    thinking_type: String,
}

#[derive(Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<ChatMessage>,
    max_tokens: u32,
    temperature: f32,
    stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    thinking: Option<ThinkingConfig>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

#[derive(Deserialize)]
struct ChatResponse {
    choices: Vec<ChatChoice>,
}

#[derive(Deserialize)]
struct ChatChoice {
    message: ChatMessage,
}

#[derive(Deserialize)]
struct StreamChunkDelta {
    content: Option<String>,
    /// Thinking/reasoning content returned by models like GLM-5, DeepSeek-R1
    reasoning_content: Option<String>,
}

#[derive(Deserialize)]
struct StreamChunkChoice {
    delta: StreamChunkDelta,
}

#[derive(Deserialize)]
struct StreamChunk {
    choices: Vec<StreamChunkChoice>,
}

pub struct OpenAiCompatibleAdapter {
    client: Client,
    base_url: String,
    model: String,
    api_key: String,
}

impl OpenAiCompatibleAdapter {
    pub fn new(base_url: &str, model: &str, api_key: &str) -> Self {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(120))
            .connect_timeout(std::time::Duration::from_secs(10))
            .build()
            .unwrap_or_else(|_| Client::new());
        Self {
            client,
            base_url: base_url.trim_end_matches('/').to_string(),
            model: model.to_string(),
            api_key: api_key.to_string(),
        }
    }
}

impl OpenAiCompatibleAdapter {
    /// Streaming completion with single prompt (convenience wrapper).
    pub async fn stream_complete<F>(
        &self,
        prompt: &str,
        max_tokens: u32,
        on_chunk: F,
    ) -> Result<String, LlmError>
    where
        F: FnMut(&str) + Send,
    {
        let messages = vec![ChatMessage {
            role: "user".into(),
            content: prompt.to_string(),
        }];
        self.stream_complete_messages(&messages, max_tokens, on_chunk).await
    }

    /// Streaming completion with full message history for multi-turn conversations.
    /// Returns the full assembled assistant reply.
    pub async fn stream_complete_messages<F>(
        &self,
        messages: &[ChatMessage],
        max_tokens: u32,
        mut on_chunk: F,
    ) -> Result<String, LlmError>
    where
        F: FnMut(&str) + Send,
    {
        // Use a dedicated client without overall timeout for streaming (connection stays open)
        let stream_client = Client::builder()
            .connect_timeout(std::time::Duration::from_secs(15))
            .build()
            .unwrap_or_else(|_| Client::new());

        let url = format!("{}/chat/completions", self.base_url);
        let thinking = is_thinking_model(&self.model)
            .then(|| ThinkingConfig { thinking_type: "enabled".into() });
        let temperature = if is_thinking_model(&self.model) { 1.0 } else { 0.3 };
        let req = ChatRequest {
            model: self.model.clone(),
            messages: messages.to_vec(),
            max_tokens,
            temperature,
            stream: true,
            thinking,
        };

        let resp = stream_client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&req)
            .send()
            .await
            .map_err(|e| LlmError::ConnectionError(e.to_string()))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(LlmError::RequestFailed(format!("HTTP {} — {}", status, body)));
        }

        let mut full_text = String::new();
        let mut stream = resp.bytes_stream();
        let mut buffer = String::new();

        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(|e| LlmError::ConnectionError(e.to_string()))?;
            buffer.push_str(&String::from_utf8_lossy(&chunk));

            // SSE format: lines starting with "data: "
            while let Some(pos) = buffer.find('\n') {
                let line = buffer[..pos].trim().to_string();
                buffer = buffer[pos + 1..].to_string();

                if line.is_empty() || line.starts_with(':') {
                    continue;
                }

                if let Some(data) = line.strip_prefix("data: ") {
                    if data.trim() == "[DONE]" {
                        break;
                    }
                    if let Ok(chunk) = serde_json::from_str::<StreamChunk>(data) {
                        if let Some(choice) = chunk.choices.first() {
                            if let Some(ref content) = choice.delta.content {
                                full_text.push_str(content);
                                on_chunk(content);
                            }
                        }
                    }
                }
            }
        }

        Ok(full_text)
    }
}

#[async_trait]
impl LlmAdapter for OpenAiCompatibleAdapter {
    async fn complete(&self, prompt: &str, max_tokens: u32) -> Result<String, LlmError> {
        let url = format!("{}/chat/completions", self.base_url);
        let thinking = is_thinking_model(&self.model)
            .then(|| ThinkingConfig { thinking_type: "enabled".into() });
        let temperature = if is_thinking_model(&self.model) { 1.0 } else { 0.3 };
        let req = ChatRequest {
            model: self.model.clone(),
            messages: vec![ChatMessage {
                role: "user".into(),
                content: prompt.to_string(),
            }],
            max_tokens,
            temperature,
            stream: false,
            thinking,
        };

        let resp = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&req)
            .send()
            .await
            .map_err(|e| LlmError::ConnectionError(e.to_string()))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(LlmError::RequestFailed(format!(
                "HTTP {} — {}",
                status, body
            )));
        }

        let body: ChatResponse = resp
            .json()
            .await
            .map_err(|e| LlmError::InvalidResponse(e.to_string()))?;

        body.choices
            .first()
            .map(|c| c.message.content.clone())
            .ok_or_else(|| LlmError::InvalidResponse("empty choices".into()))
    }

    async fn health_check(&self) -> Result<bool, LlmError> {
        // Step 1: GET /models — fast check for API key validity and connectivity.
        // A 401/403 here means the key is definitely wrong; we stop early.
        // A 200 only means the key is valid — it does NOT verify the specific model
        // has an available resource package, so we always continue to the chat probe.
        let models_url = format!("{}/models", self.base_url);
        let models_resp = self
            .client
            .get(&models_url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .send()
            .await;

        match models_resp {
            Ok(r) if r.status() == 401 || r.status() == 403 => {
                let status = r.status().as_u16();
                let body = r.text().await.unwrap_or_default();
                return Err(LlmError::RequestFailed(format!(
                    "HTTP {} — {}",
                    status, body
                )));
            }
            // Any other outcome (200, 404, network error): fall through to chat probe
            _ => {}
        }

        // Step 2: Minimal chat probe — verifies this specific model is accessible.
        // For thinking models we deliberately disable thinking to keep costs near-zero
        // (thinking mode requires a much larger token budget).
        let url = format!("{}/chat/completions", self.base_url);
        let req = ChatRequest {
            model: self.model.clone(),
            messages: vec![ChatMessage {
                role: "user".into(),
                content: "hi".into(),
            }],
            max_tokens: 5,
            temperature: 0.0,
            stream: false,
            thinking: None, // always off for the probe
        };

        let resp = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&req)
            .send()
            .await
            .map_err(|e| LlmError::ConnectionError(e.to_string()))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(LlmError::RequestFailed(format!(
                "HTTP {} — {}",
                status, body
            )));
        }

        Ok(true)
    }

    fn provider_name(&self) -> &str {
        "openai_compatible"
    }
}
