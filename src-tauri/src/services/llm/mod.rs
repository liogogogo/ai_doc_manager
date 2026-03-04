pub mod adapter;
pub mod ollama;
pub mod openai_compatible;

pub use adapter::{create_adapter, LlmAdapter, LlmError};
pub use ollama::OllamaAdapter;
pub use openai_compatible::{ChatMessage, OpenAiCompatibleAdapter};
