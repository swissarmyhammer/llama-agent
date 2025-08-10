# EMBEDDING_000008: Implement EmbeddingModel

## Overview
Implement the core `EmbeddingModel` struct that handles individual text embedding using llama-cpp-2 and llama-loader for model management.

Refer to ./specification/embedding.md

## Tasks

### 1. Implement EmbeddingModel Struct
```rust
// llama-embedding/src/model.rs
pub struct EmbeddingModel {
    loader: Arc<ModelLoader>,
    model: Option<LlamaModel>,
    config: EmbeddingConfig,
    metadata: Option<ModelMetadata>,
}
```

### 2. Core Embedding Configuration
```rust
pub struct EmbeddingConfig {
    pub model_source: ModelSource,
    pub normalize_embeddings: bool,
    pub max_sequence_length: Option<usize>,
    pub debug: bool,
}
```

### 3. Single Text Embedding
```rust
impl EmbeddingModel {
    pub async fn new(config: EmbeddingConfig) -> Result<Self, EmbeddingError>;
    pub async fn load_model(&mut self) -> Result<(), EmbeddingError>;
    pub async fn embed_text(&self, text: &str) -> Result<EmbeddingResult, EmbeddingError>;
    pub fn get_embedding_dimension(&self) -> Option<usize>;
}
```

### 4. EmbeddingResult Structure
```rust
pub struct EmbeddingResult {
    pub text: String,
    pub text_hash: String,  // MD5 hash
    pub embedding: Vec<f32>,
    pub sequence_length: usize,
    pub processing_time_ms: u64,
}
```

### 5. Integration with llama-loader
- Use `ModelLoader` for all model loading operations
- Leverage caching and retry logic from llama-loader
- Handle model loading errors gracefully
- Support both HuggingFace and local models

### 6. Text Processing Features
- MD5 hash generation for text deduplication
- Optional embedding normalization (L2 norm)
- Sequence length handling and truncation
- Processing time measurement
- Debug logging and tracing

## Success Criteria
- [ ] EmbeddingModel compiles and basic tests pass
- [ ] Can successfully load embedding models via llama-loader
- [ ] Single text embedding works correctly
- [ ] MD5 hash generation works
- [ ] Embedding normalization optional feature works
- [ ] Error handling robust and informative
- [ ] Integration with llama-loader seamless
- [ ] Performance characteristics reasonable

## Testing Requirements
- Unit tests for EmbeddingModel functionality
- Test with a real embedding model (can be small test model)
- Test MD5 hash consistency
- Test normalization correctness
- Test error handling for various failure modes

## Integration Notes
- This provides the core embedding functionality for the library
- Will be used by BatchProcessor in next step
- Must be thread-safe for concurrent use
- Focus on correctness and reliability