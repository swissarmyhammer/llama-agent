# EMBEDDING_000002: Extract ModelSource and Related Types

## Overview
Extract `ModelSource` and related types from `llama-agent/src/types.rs` into `llama-loader`, preparing for the model loading logic extraction.

Refer to ./specification/embedding.md

## Tasks

### 1. Move ModelSource to llama-loader
- Copy `ModelSource` enum from `llama-agent/src/types.rs` to `llama-loader/src/types.rs`
- Update imports and re-exports in `llama-loader/src/lib.rs`
- Add workspace dependency: `llama-loader = { path = "llama-loader" }`

### 2. Update llama-agent to Use llama-loader Types
- Add `llama-loader` dependency to `llama-agent/Cargo.toml`
- Replace `ModelSource` definition with `pub use llama_loader::ModelSource;`
- Ensure all existing code still compiles

### 3. Add ModelConfig Type
- Create `ModelConfig` struct in llama-loader that combines model source with loading parameters
- Define clean API for model configuration

### 4. Preserve Backward Compatibility
- Ensure all existing public APIs in llama-agent continue to work
- Maintain same imports and usage patterns for consumers

## Code Changes
```rust
// llama-loader/src/types.rs
#[derive(Debug, Clone)]
pub enum ModelSource {
    HuggingFace { repo: String, filename: Option<String> },
    Local { folder: PathBuf, filename: Option<String> },
}

#[derive(Debug, Clone)]
pub struct ModelConfig {
    pub source: ModelSource,
    // Additional configuration will be added in later steps
}
```

## Success Criteria
- [ ] ModelSource successfully moved to llama-loader
- [ ] All existing llama-agent functionality works unchanged
- [ ] llama-agent can import and use types from llama-loader
- [ ] Clean separation of concerns established
- [ ] No breaking changes to public APIs

## Integration Notes
- This step establishes the type foundation for shared model loading
- Keeps changes minimal to avoid breaking existing functionality
- Sets up clean dependency relationship between crates