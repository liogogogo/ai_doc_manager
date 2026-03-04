use async_trait::async_trait;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum LlmError {
    #[error("LLM request failed: {0}")]
    RequestFailed(String),
    #[error("LLM connection error: {0}")]
    ConnectionError(String),
    #[error("Invalid response: {0}")]
    InvalidResponse(String),
    #[error("LLM not configured: {0}")]
    NotConfigured(String),
}

#[async_trait]
pub trait LlmAdapter: Send + Sync {
    /// Send a prompt and get a text response
    async fn complete(&self, prompt: &str, max_tokens: u32) -> Result<String, LlmError>;

    /// Check if the LLM service is reachable
    async fn health_check(&self) -> Result<bool, LlmError>;

    /// Get the provider name
    fn provider_name(&self) -> &str;
}

/// Build the appropriate LLM adapter from config
pub fn create_adapter(
    provider: &str,
    base_url: &str,
    model: &str,
    api_key: Option<&str>,
) -> Result<Box<dyn LlmAdapter>, LlmError> {
    match provider {
        "ollama" => Ok(Box::new(super::OllamaAdapter::new(base_url, model))),
        "openai" | "openai_compatible" | "deepseek" | "doubao" | "qwen" | "zhipu"
        | "ernie" | "spark" | "moonshot" | "minimax" | "yi" | "stepfun"
        | "siliconflow" | "claude" => {
            let key = api_key
                .filter(|k| !k.is_empty())
                .ok_or_else(|| LlmError::NotConfigured("API Key 未配置".into()))?;
            Ok(Box::new(super::OpenAiCompatibleAdapter::new(
                base_url, model, key,
            )))
        }
        _ => Err(LlmError::NotConfigured(format!(
            "不支持的 LLM provider: {}",
            provider
        ))),
    }
}
