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

## Proposed Solution

I will implement the complete ModelLoader integration by:

### 1. Update ModelLoader struct to include CacheManager
- Add cache_manager field to ModelLoader 
- Add retry_config field to ModelLoader
- Update constructor to accept these dependencies

### 2. Implement the main load_model method
- Check cache first before downloading/loading
- Use extracted HuggingFace loading logic from huggingface.rs
- Apply retry logic for failed downloads
- Handle multi-part models automatically
- Cache successfully loaded models
- Return rich metadata about loading process

### 3. Update existing methods to use cache
- Modify load_huggingface_model to use cache integration
- Update load_local_model to respect cache when appropriate
- Add proper error handling with cache-specific errors

### 4. Add file size and path tracking
- Update return types to include actual file paths
- Add file size calculation for metadata
- Track cache hits/misses properly

### 5. Integration with existing code
- Ensure ModelLoader works with the existing HuggingFace loading functions
- Maintain backward compatibility with existing error handling
- Preserve all retry logic and multi-part handling

This approach will complete the llama-loader extraction by providing a unified API that integrates caching, retry logic, and model loading from both HuggingFace and local sources.

## Implementation Complete

✅ **Successfully implemented the ModelLoader integration** with the following features:

### 1. Enhanced ModelLoader Struct
- Added `CacheManager` for model caching with LRU eviction
- Added `RetryConfig` for configurable retry logic
- Updated constructors to support both default and custom configurations

### 2. Unified load_model Method
- Implemented the main `load_model` method that accepts `ModelConfig`
- Integrated cache checking before downloading/loading
- Automatic cache storage for successfully loaded models
- Rich metadata returned with cache hit/miss tracking and file sizes

### 3. HuggingFace Integration Enhancement
- Created `load_huggingface_model_with_path` function for cache integration
- Preserved existing `load_huggingface_model` function for backward compatibility
- Maintained all retry logic, multi-part handling, and error handling

### 4. Local Model Loading Improvements
- Enhanced `load_local_model` with proper file size tracking
- Auto-detection of GGUF files with BF16 preference
- Complete metadata generation including accurate file sizes

### 5. Comprehensive Testing
- Added integration tests covering cache operations, validation, and API usage
- All existing tests continue to pass
- Created example demonstrating the public API

### 6. Thread Safety & Performance
- ModelLoader is thread-safe for concurrent access
- Cache operations are async and non-blocking
- LRU eviction ensures memory usage stays within limits

## Key Features Delivered

✅ ModelLoader successfully loads HuggingFace models  
✅ Cache integration works correctly (hit/miss tracking)  
✅ All retry logic preserved and functional  
✅ Multi-part model loading works  
✅ Local model loading works  
✅ Error handling matches existing behavior  
✅ Rich metadata returned for loaded models  
✅ Thread-safe for concurrent access  

## Integration Status

The ModelLoader implementation is now complete and ready to be integrated into:
- llama-agent (replacing existing model loading)
- llama-embedding (new embedding crate)
- llama-cli (unified CLI)

All loading functionality is backward compatible while adding cache capabilities.