# EMBEDDING_000004: Implement Cache Manager

## Overview
Create a caching system for downloaded models to enable sharing between `llama-agent`, `llama-embedding`, and `llama-cli` crates.

Refer to ./specification/embedding.md

## Tasks

### 1. Create Cache Manager Implementation
- Create `llama-loader/src/cache.rs`
- Implement `CacheManager` struct with LRU eviction
- Add cache key generation based on repo, filename, and file metadata
- Implement cache size limits and cleanup

### 2. Cache Directory Management
- Use platform-appropriate cache directories:
  - Linux/macOS: `~/.cache/llama-loader/models/`
  - Windows: `%LOCALAPPDATA%\llama-loader\models\`
- Create directories as needed
- Handle permission issues gracefully

### 3. Cache Operations
```rust
impl CacheManager {
    pub fn new(cache_dir: PathBuf) -> Self;
    pub async fn get_cached_model(&self, cache_key: &str) -> Option<PathBuf>;
    pub async fn cache_model(&self, model_path: &Path, cache_key: &str) -> Result<(), CacheError>;
    pub async fn cleanup_old_models(&self) -> Result<(), CacheError>;
    pub fn generate_cache_key(repo: &str, filename: &str, metadata: &FileMetadata) -> String;
}
```

### 4. Cache Configuration
- Default max cache size: 50GB
- Configurable cache settings
- LRU eviction when size limit exceeded
- Cache statistics and monitoring

### 5. Add Required Dependencies
```toml
# Add to llama-loader/Cargo.toml
[dependencies]
dirs = "5.0"  # For platform cache directories
sha2 = "0.10"  # For cache key generation
tokio = { workspace = true, features = ["fs"] }
```

## Success Criteria
- [ ] CacheManager compiles and basic tests pass
- [ ] Cache keys generated consistently
- [ ] Cache directories created appropriately per platform
- [ ] LRU eviction works correctly
- [ ] Size limits enforced
- [ ] Cache hit/miss tracking works
- [ ] Error handling for permission/disk issues

## Integration Notes
- This will be used by ModelLoader in the next step
- Must be thread-safe for concurrent access
- Should handle cache corruption gracefully
- Cache keys must be deterministic and unique