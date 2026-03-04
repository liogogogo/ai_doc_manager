use super::adapter::{ChatMessage, LlmAdapter, LlmError, StreamEvent};
use async_trait::async_trait;
use futures_util::StreamExt;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Serialize)]
struct OllamaChatMessage {
    role: String,
    content: String,
}

#[derive(Serialize)]
struct OllamaChatRequest {
    model: String,
    messages: Vec<OllamaChatMessage>,
    stream: bool,
    options: OllamaOptions,
}

#[derive(Serialize)]
struct OllamaOptions {
    num_predict: u32,
}

#[derive(Deserialize)]
struct OllamaChatChunk {
    message: Option<OllamaChatChunkMessage>,
    done: bool,
}

#[derive(Deserialize)]
struct OllamaChatChunkMessage {
    content: Option<String>,
}

pub struct OllamaAdapter {
    client: Client,
    base_url: String,
    model: String,
}

impl OllamaAdapter {
    pub fn new(base_url: &str, model: &str) -> Self {
        let client = Client::builder()
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(300))
            .build()
            .unwrap_or_else(|_| Client::new());
        Self {
            client,
            base_url: base_url.trim_end_matches('/').to_string(),
            model: model.to_string(),
        }
    }

    fn to_ollama_messages(messages: &[ChatMessage]) -> Vec<OllamaChatMessage> {
        messages
            .iter()
            .map(|m| OllamaChatMessage {
                role: m.role.clone(),
                content: m.content.clone(),
            })
            .collect()
    }
}

#[async_trait]
impl LlmAdapter for OllamaAdapter {
    async fn complete(&self, prompt: &str, max_tokens: u32) -> Result<String, LlmError> {
        let messages = vec![ChatMessage {
            role: "user".into(),
            content: prompt.to_string(),
        }];
        let url = format!("{}/api/chat", self.base_url);
        let req = OllamaChatRequest {
            model: self.model.clone(),
            messages: Self::to_ollama_messages(&messages),
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
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(LlmError::RequestFailed(format!("HTTP {} — {}", status, body)));
        }

        let body: OllamaChatChunk = resp
            .json()
            .await
            .map_err(|e| LlmError::InvalidResponse(e.to_string()))?;

        Ok(body
            .message
            .and_then(|m| m.content)
            .unwrap_or_default())
    }

    async fn stream_complete_messages(
        &self,
        messages: &[ChatMessage],
        max_tokens: u32,
        cancel_flag: std::sync::Arc<std::sync::atomic::AtomicBool>,
        mut on_event: Box<dyn FnMut(StreamEvent) + Send>,
    ) -> Result<String, LlmError> {
        let stream_client = Client::builder()
            .connect_timeout(Duration::from_secs(10))
            .build()
            .unwrap_or_else(|_| Client::new());

        let url = format!("{}/api/chat", self.base_url);
        let req = OllamaChatRequest {
            model: self.model.clone(),
            messages: Self::to_ollama_messages(messages),
            stream: true,
            options: OllamaOptions {
                num_predict: max_tokens,
            },
        };

        let resp = stream_client
            .post(&url)
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
        let chunk_timeout = Duration::from_secs(60);

        loop {
            if cancel_flag.load(std::sync::atomic::Ordering::SeqCst) {
                on_event(StreamEvent::Done);
                return Ok(full_text);
            }
            let maybe_chunk = tokio::time::timeout(chunk_timeout, stream.next()).await;

            match maybe_chunk {
                Err(_) => {
                    return Err(LlmError::Timeout("60 秒未收到新数据".into()));
                }
                Ok(None) => break,
                Ok(Some(chunk_result)) => {
                    let chunk =
                        chunk_result.map_err(|e| LlmError::ConnectionError(e.to_string()))?;
                    buffer.push_str(&String::from_utf8_lossy(&chunk));

                    while let Some(pos) = buffer.find('\n') {
                        let line = buffer[..pos].trim().to_string();
                        buffer = buffer[pos + 1..].to_string();

                        if line.is_empty() {
                            continue;
                        }

                        if let Ok(chunk_data) = serde_json::from_str::<OllamaChatChunk>(&line) {
                            if chunk_data.done {
                                on_event(StreamEvent::Done);
                                return Ok(full_text);
                            }
                            if let Some(msg) = &chunk_data.message {
                                if let Some(ref content) = msg.content {
                                    if !content.is_empty() {
                                        full_text.push_str(content);
                                        on_event(StreamEvent::Content(content.clone()));
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
