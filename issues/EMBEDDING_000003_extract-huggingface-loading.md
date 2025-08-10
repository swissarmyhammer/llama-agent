# EMBEDDING_000003: Extract HuggingFace Loading Logic

## Overview
Extract the core HuggingFace model loading functionality from `llama-agent/src/model.rs` into `llama-loader/src/huggingface.rs`, preserving all existing behavior including retry logic and multi-part downloads.

Refer to ./specification/embedding.md

## Tasks

### 1. Create HuggingFace Module
- Create `llama-loader/src/huggingface.rs`
- Extract `load_huggingface_model()` function from `ModelManager`
- Extract all HuggingFace-specific helper functions

### 2. Extract Retry Logic
- Create `llama-loader/src/retry.rs`
- Move `download_model_file_with_retry()` function
- Move `is_retriable_error()` and related retry utilities
- Preserve all existing retry behavior and exponential backoff

### 3. Extract Multi-part Download Logic
- Create `llama-loader/src/multipart.rs`
- Move `download_multi_part_model()` function
- Move `detect_multi_part_base()` and `get_all_parts()` functions
- Preserve all multi-part model detection and assembly

### 4. Extract Auto-detection Logic
- Create `llama-loader/src/detection.rs`
- Move `auto_detect_hf_model_file()` function
- Move BF16 preference detection logic
- Preserve all existing auto-detection behavior

### 5. Update Dependencies
- Add required dependencies to `llama-loader/Cargo.toml`:
  - `hf-hub = { workspace = true }`
  - `tokio = { workspace = true }`
  - `regex = { workspace = true }`

## Code Structure
```
llama-loader/src/
├── huggingface.rs    # Main HF loading logic
├── retry.rs          # Retry and backoff logic  
├── multipart.rs      # Multi-part download handling
├── detection.rs      # Auto-detection logic
└── error.rs          # Shared error types
```

## Success Criteria
- [ ] All HuggingFace loading logic extracted successfully
- [ ] All retry logic preserved with same behavior
- [ ] Multi-part download functionality works identically
- [ ] Auto-detection logic preserved
- [ ] Extracted code compiles and passes basic tests
- [ ] No functionality regressions from original code

## Critical Requirements
- **Preserve ALL existing functionality** - no behavior changes
- **Maintain error handling patterns** - same error types and messages
- **Keep same logging and progress indication** 
- **Preserve memory usage patterns**
- **Maintain same file validation logic**

## Integration Notes
- This step extracts ~300-400 lines of complex loading logic
- Focus on exact preservation of existing behavior
- Will be integrated into ModelLoader in next step