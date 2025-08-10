use llama_loader::ModelSource;

/// Result of a single text embedding operation
#[derive(Debug, Clone)]
pub struct EmbeddingResult {
    /// The original text that was embedded
    pub text: String,
    /// MD5 hash of the text for deduplication
    pub text_hash: String,
    /// The embedding vector
    pub embedding: Vec<f32>,
    /// Length of the tokenized sequence
    pub sequence_length: usize,
    /// Processing time in milliseconds
    pub processing_time_ms: u64,
}

/// Configuration for embedding operations
#[derive(Debug, Clone)]
pub struct EmbeddingConfig {
    /// Source of the model (HuggingFace repo or local path)
    pub model_source: ModelSource,
    /// Batch size for processing multiple texts
    pub batch_size: usize,
    /// Whether to normalize embedding vectors
    pub normalize_embeddings: bool,
    /// Maximum sequence length (None for model default)
    pub max_sequence_length: Option<usize>,
    /// Enable debug logging
    pub debug: bool,
}

impl Default for EmbeddingConfig {
    fn default() -> Self {
        Self {
            model_source: ModelSource::HuggingFace {
                repo: "sentence-transformers/all-MiniLM-L6-v2".to_string(),
                filename: None,
            },
            batch_size: 32,
            normalize_embeddings: false,
            max_sequence_length: None,
            debug: false,
        }
    }
}

impl EmbeddingResult {
    /// Create a new embedding result
    pub fn new(
        text: String,
        embedding: Vec<f32>,
        sequence_length: usize,
        processing_time_ms: u64,
    ) -> Self {
        let text_hash = format!("{:x}", md5::compute(&text));
        Self {
            text,
            text_hash,
            embedding,
            sequence_length,
            processing_time_ms,
        }
    }
}
