use super::adapter::{ChatMessage, LlmAdapter, LlmError, StreamEvent};
use async_trait::async_trait;
use futures_util::StreamExt;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;

const MAX_RETRIES: u32 = 2;
const RETRY_DELAYS_MS: [u64; 2] = [1000, 3000];
const CHUNK_TIMEOUT_SECS: u64 = 60;

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

fn is_retryable(err: &LlmError) -> bool {
    match err {
        LlmError::ConnectionError(_) | LlmError::Timeout(_) => true,
        LlmError::RequestFailed(msg) => {
            msg.starts_with("HTTP 5") || msg.contains("502") || msg.contains("503") || msg.contains("504")
        }
        _ => false,
    }
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
            .timeout(Duration::from_secs(120))
            .connect_timeout(Duration::from_secs(10))
            .build()
            .unwrap_or_else(|_| Client::new());
        Self {
            client,
            base_url: base_url.trim_end_matches('/').to_string(),
            model: model.to_string(),
            api_key: api_key.to_string(),
        }
    }

    fn build_chat_request(&self, messages: &[ChatMessage], max_tokens: u32, stream: bool) -> ChatRequest {
        let thinking = is_thinking_model(&self.model)
            .then(|| ThinkingConfig { thinking_type: "enabled".into() });
        let temperature = if is_thinking_model(&self.model) { 1.0 } else { 0.3 };
        ChatRequest {
            model: self.model.clone(),
            messages: messages.to_vec(),
            max_tokens,
            temperature,
            stream,
            thinking,
        }
    }

    async fn do_stream(
        &self,
        messages: &[ChatMessage],
        max_tokens: u32,
        cancel_flag: &std::sync::Arc<std::sync::atomic::AtomicBool>,
        on_event: &mut Box<dyn FnMut(StreamEvent) + Send>,
    ) -> Result<String, LlmError> {
        let stream_client = Client::builder()
            .connect_timeout(Duration::from_secs(15))
            .build()
            .unwrap_or_else(|_| Client::new());

        let url = format!("{}/chat/completions", self.base_url);
        let req = self.build_chat_request(messages, max_tokens, true);

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
        let chunk_timeout = Duration::from_secs(CHUNK_TIMEOUT_SECS);

        loop {
            if cancel_flag.load(std::sync::atomic::Ordering::SeqCst) {
                on_event(StreamEvent::Done);
                return Ok(full_text);
            }
            let maybe_chunk = tokio::time::timeout(chunk_timeout, stream.next()).await;

            match maybe_chunk {
                Err(_) => {
                    return Err(LlmError::Timeout(format!(
                        "{} 秒未收到新数据",
                        CHUNK_TIMEOUT_SECS
                    )));
                }
                Ok(None) => break,
                Ok(Some(chunk_result)) => {
                    let chunk =
                        chunk_result.map_err(|e| LlmError::ConnectionError(e.to_string()))?;
                    buffer.push_str(&String::from_utf8_lossy(&chunk));

                    while let Some(pos) = buffer.find('\n') {
                        let line = buffer[..pos].trim().to_string();
                        buffer = buffer[pos + 1..].to_string();

                        if line.is_empty() || line.starts_with(':') {
                            continue;
                        }

                        if let Some(data) = line.strip_prefix("data: ") {
                            if data.trim() == "[DONE]" {
                                on_event(StreamEvent::Done);
                                return Ok(full_text);
                            }
                            if let Ok(parsed) = serde_json::from_str::<StreamChunk>(data) {
                                if let Some(choice) = parsed.choices.first() {
                                    if let Some(ref reasoning) = choice.delta.reasoning_content {
                                        if !reasoning.is_empty() {
                                            on_event(StreamEvent::Reasoning(reasoning.clone()));
                                        }
                                    }
                                    if let Some(ref content) = choice.delta.content {
                                        if !content.is_empty() {
                                            let prefix_len = full_text.len();
                                            let slice = if content.len() >= prefix_len
                                                && &content[..prefix_len] == full_text
                                            {
                                                &content[prefix_len..]
                                            } else {
                                                content.as_str()
                                            };
                                            if !slice.is_empty() {
                                                full_text.push_str(slice);
                                                on_event(StreamEvent::Content(slice.to_string()));
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        on_event(StreamEvent::Done);
        Ok(full_text)
    }
}

#[async_trait]
impl LlmAdapter for OpenAiCompatibleAdapter {
    async fn complete(&self, prompt: &str, max_tokens: u32) -> Result<String, LlmError> {
        let messages = vec![ChatMessage {
            role: "user".into(),
            content: prompt.to_string(),
        }];
        let url = format!("{}/chat/completions", self.base_url);
        let req = self.build_chat_request(&messages, max_tokens, false);

        let mut last_err = None;

        for attempt in 0..=MAX_RETRIES {
            if attempt > 0 {
                let delay = RETRY_DELAYS_MS[(attempt - 1) as usize];
                tracing::warn!("LLM request retry #{}, waiting {}ms", attempt, delay);
                tokio::time::sleep(Duration::from_millis(delay)).await;
            }

            let result = self
                .client
                .post(&url)
                .header("Authorization", format!("Bearer {}", self.api_key))
                .header("Content-Type", "application/json")
                .json(&req)
                .send()
                .await;

            match result {
                Err(e) => {
                    let err = LlmError::ConnectionError(e.to_string());
                    if !is_retryable(&err) || attempt == MAX_RETRIES {
                        return Err(err);
                    }
                    last_err = Some(err);
                }
                Ok(resp) => {
                    if !resp.status().is_success() {
                        let status = resp.status();
                        let body = resp.text().await.unwrap_or_default();
                        let err = LlmError::RequestFailed(format!("HTTP {} — {}", status, body));
                        if !is_retryable(&err) || attempt == MAX_RETRIES {
                            return Err(err);
                        }
                        last_err = Some(err);
                        continue;
                    }

                    let body: ChatResponse = resp
                        .json()
                        .await
                        .map_err(|e| LlmError::InvalidResponse(e.to_string()))?;

                    return body
                        .choices
                        .first()
                        .map(|c| c.message.content.clone())
                        .ok_or_else(|| LlmError::InvalidResponse("empty choices".into()));
                }
            }
        }

        Err(last_err.unwrap_or_else(|| LlmError::RequestFailed("max retries exceeded".into())))
    }

    async fn stream_complete_messages(
        &self,
        messages: &[ChatMessage],
        max_tokens: u32,
        cancel_flag: std::sync::Arc<std::sync::atomic::AtomicBool>,
        mut on_event: Box<dyn FnMut(StreamEvent) + Send>,
    ) -> Result<String, LlmError> {
        let mut last_err = None;

        for attempt in 0..=MAX_RETRIES {
            if attempt > 0 {
                let delay = RETRY_DELAYS_MS[(attempt - 1) as usize];
                tracing::warn!("LLM stream retry #{}, waiting {}ms", attempt, delay);
                tokio::time::sleep(Duration::from_millis(delay)).await;
            }

            match self
                .do_stream(messages, max_tokens, &cancel_flag, &mut on_event)
                .await
            {
                Ok(text) => return Ok(text),
                Err(e) => {
                    if !is_retryable(&e) || attempt == MAX_RETRIES {
                        return Err(e);
                    }
                    tracing::warn!("LLM stream attempt {} failed: {}", attempt + 1, e);
                    last_err = Some(e);
                }
            }
        }

        Err(last_err.unwrap_or_else(|| LlmError::RequestFailed("max retries exceeded".into())))
    }

    async fn health_check(&self) -> Result<bool, LlmError> {
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
            _ => {}
        }

        let url = format!("{}/chat/completions", self.base_url);
        let req = self.build_chat_request(
            &[ChatMessage {
                role: "user".into(),
                content: "hi".into(),
            }],
            5,
            false,
        );
        // Override thinking for probe: always off
        let probe_req = ChatRequest {
            thinking: None,
            temperature: 0.0,
            ..req
        };

        let resp = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&probe_req)
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
