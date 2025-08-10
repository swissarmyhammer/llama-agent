# EMBEDDING_000001: Create llama-loader Foundation

## Overview
Create the foundational structure for the new `llama-loader` crate that will contain shared model loading logic extracted from `llama-agent`.

Refer to ./specification/embedding.md

## Tasks

### 1. Create Crate Structure
- Create `llama-loader/` directory
- Add `Cargo.toml` with appropriate dependencies
- Create `src/lib.rs` with basic module structure
- Add to workspace `Cargo.toml` members

### 2. Define Core Types
- Create `src/types.rs` with basic types that will be extracted from llama-agent
- Define `LoadedModel` struct
- Define `ModelMetadata` struct  
- Create placeholder for `ModelLoader` struct

### 3. Error Handling
- Create `src/error.rs` with `ModelError` and related error types
- Ensure compatibility with existing llama-agent error handling

### 4. Basic Module Structure
```
llama-loader/
├── Cargo.toml
├── src/
│   ├── lib.rs              # Public API and re-exports
│   ├── types.rs            # Core types
│   ├── error.rs            # Error types
│   └── loader.rs           # Placeholder ModelLoader
```

## Dependencies
```toml
[dependencies]
llama-cpp-2 = { workspace = true }
thiserror = { workspace = true }
tokio = { workspace = true }
```

## Success Criteria
- [ ] llama-loader crate compiles successfully
- [ ] Basic types and error handling defined
- [ ] Added to workspace configuration
- [ ] No breaking changes to existing code
- [ ] Clean module structure established

## Integration Notes
- This step establishes the foundation without breaking existing functionality
- Later steps will extract actual logic from llama-agent
- Focus on establishing clean, extensible architecture