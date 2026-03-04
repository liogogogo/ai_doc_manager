pub mod adapter;
pub mod ollama;
pub mod openai_compatible;

pub use adapter::{create_adapter, ChatMessage, LlmAdapter, LlmError, StreamEvent};
pub use ollama::OllamaAdapter;
pub use openai_compatible::OpenAiCompatibleAdapter;
