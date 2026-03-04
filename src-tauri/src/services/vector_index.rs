/// Vector index service for semantic search over document chunks.
///
/// In the MVP phase, this uses a simple TF-IDF approach for similarity.
/// In Phase 2+, this will be upgraded to use sqlite-vss with real embeddings
/// from a local ONNX model (all-MiniLM-L6-v2).

use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct IndexedChunk {
    pub chunk_id: i64,
    pub document_id: String,
    pub text: String,
    pub start_line: u32,
    pub end_line: u32,
}

pub struct VectorIndex {
    chunks: Vec<IndexedChunk>,
}

impl VectorIndex {
    pub fn new() -> Self {
        Self { chunks: Vec::new() }
    }

    pub fn add_chunk(&mut self, chunk: IndexedChunk) {
        self.chunks.push(chunk);
    }

    pub fn clear(&mut self) {
        self.chunks.clear();
    }

    /// Simple keyword-based search (to be replaced with vector similarity)
    pub fn search(&self, query: &str, top_k: usize) -> Vec<&IndexedChunk> {
        let query_terms: Vec<&str> = query.split_whitespace().collect();

        let mut scored: Vec<(&IndexedChunk, f32)> = self
            .chunks
            .iter()
            .map(|chunk| {
                let text_lower = chunk.text.to_lowercase();
                let score: f32 = query_terms
                    .iter()
                    .map(|term| {
                        let term_lower = term.to_lowercase();
                        text_lower.matches(&term_lower).count() as f32
                    })
                    .sum();
                (chunk, score)
            })
            .filter(|(_, score)| *score > 0.0)
            .collect();

        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        scored.into_iter().take(top_k).map(|(c, _)| c).collect()
    }
}
