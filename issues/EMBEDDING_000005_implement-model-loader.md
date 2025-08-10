# EMBEDDING_000005: Implement ModelLoader Integration

## Overview
Create the main `ModelLoader` struct that integrates all extracted loading logic with the new caching system, providing a unified API for model loading across all crates.

Refer to ./specification/embedding.md

## Tasks

### 1. Implement ModelLoader Struct
```rust
// llama-loader/src/loader.rs
pub struct ModelLoader {
    backend: Arc<LlamaBackend>,
    cache_manager: CacheManager,
    retry_config: RetryConfig,
}

pub struct LoadedModel {
    pub model: LlamaModel,
    pub path: PathBuf,
    pub metadata: ModelMetadata,
}
```

### 2. Core Loading Methods
```rust
impl ModelLoader {
    pub async fn load_model(&self, config: &ModelConfig) -> Result<LoadedModel, ModelError>;
    pub async fn load_huggingface_model(&self, repo: &str, filename: Option<&str>) -> Result<LoadedModel, ModelError>;
    pub async fn load_local_model(&self, folder: &Path, filename: Option<&str>) -> Result<LoadedModel, ModelError>;
}
```

### 3. Integration Logic
- Check cache first before downloading
- Use extracted HuggingFace loading logic
- Apply retry logic for failed downloads
- Handle multi-part models automatically
- Cache successfully loaded models
- Return rich metadata about loading process

### 4. Configuration Support
- Configurable retry settings
- Cache size and cleanup settings
- Backend configuration options
- Debug and logging configuration

### 5. Error Handling Integration
- Map all loading errors to consistent `ModelError` types
- Preserve existing error messages and context
- Add cache-specific error information
- Maintain backward compatibility with existing error handling

## Success Criteria
- [ ] ModelLoader successfully loads HuggingFace models
- [ ] Cache integration works correctly (hit/miss)
- [ ] All retry logic preserved and functional
- [ ] Multi-part model loading works
- [ ] Local model loading works
- [ ] Error handling matches existing behavior
- [ ] Rich metadata returned for loaded models
- [ ] Thread-safe for concurrent access

## Integration Notes
- This completes the llama-loader extraction
- Will replace existing model loading in llama-agent
- Must maintain exact same behavior as existing code
- Focus on reliability and backward compatibility