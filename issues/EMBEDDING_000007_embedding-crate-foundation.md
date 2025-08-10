# EMBEDDING_000007: Create llama-embedding Crate Foundation

## Overview
Create the foundational structure for the new `llama-embedding` crate that will provide batch text embedding functionality as a reusable library.

Refer to ./specification/embedding.md

## Tasks

### 1. Create Crate Structure
- Create `llama-embedding/` directory
- Add `Cargo.toml` with appropriate dependencies
- Create basic module structure in `src/`
- Add to workspace `Cargo.toml` members

### 2. Core Dependencies
```toml
[dependencies]
llama-loader = { workspace = true }
llama-cpp-2 = { workspace = true }
tokio = { workspace = true }
md5 = { workspace = true }
thiserror = { workspace = true }
tracing = { workspace = true }
```

### 3. Define Core Types
- Create `src/types.rs` with core embedding types:
  - `EmbeddingResult` - single text embedding result
  - `EmbeddingConfig` - configuration for embedding operations
  - `EmbeddingError` - error types specific to embedding

### 4. Basic Module Structure
```
llama-embedding/
├── Cargo.toml
├── src/
│   ├── lib.rs              # Public API and re-exports
│   ├── types.rs            # Core types
│   ├── error.rs            # Error types
│   ├── model.rs            # EmbeddingModel (placeholder)
│   └── batch.rs            # BatchProcessor (placeholder)
```

### 5. Error Handling
```rust
#[derive(thiserror::Error, Debug)]
pub enum EmbeddingError {
    #[error("Model error: {0}")]
    Model(#[from] llama_loader::ModelError),
    
    #[error("Batch processing error: {0}")]
    BatchProcessing(String),
    
    #[error("Text encoding error: {0}")]
    TextEncoding(String),
    
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}
```

## Success Criteria
- [ ] llama-embedding crate compiles successfully
- [ ] Basic types and error handling defined
- [ ] Added to workspace configuration
- [ ] Clean, extensible module structure
- [ ] Proper dependency management
- [ ] Ready for EmbeddingModel implementation

## Integration Notes
- This is a library crate - no CLI dependencies
- Returns structured data, no output format dependencies
- Will integrate with llama-loader for model management
- Focus on clean, reusable API design