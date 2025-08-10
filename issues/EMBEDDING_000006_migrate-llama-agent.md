# EMBEDDING_000006: Migrate llama-agent to Use llama-loader

## Overview
Update `llama-agent` to use the new `llama-loader` crate instead of its internal model loading logic, ensuring no functionality regressions while significantly reducing code duplication.

Refer to ./specification/embedding.md

## Tasks

### 1. Update ModelManager
- Update `llama-agent/src/model.rs` to use `ModelLoader` instead of internal logic
- Remove extracted functions that are now in llama-loader
- Maintain same public API for backward compatibility

### 2. Simplify Model Loading Logic
```rust
// llama-agent/src/model.rs - after migration
impl ModelManager {
    pub async fn load_model(&self) -> Result<(), ModelError> {
        let loaded_model = self.loader.load_model(&self.config).await?;
        self.model = Some(loaded_model.model);
        self.metadata = Some(loaded_model.metadata);
        Ok(())
    }
}
```

### 3. Update Dependencies
- Add `llama-loader = { workspace = true }` to `llama-agent/Cargo.toml`
- Remove dependencies that are now handled by llama-loader (if any)
- Ensure all features still work

### 4. Preserve Public APIs
- Maintain all existing public methods in `ModelManager`
- Ensure same error types and messages
- Keep same logging and progress indication
- Preserve all configuration options

### 5. Code Cleanup
- Remove ~400-500 lines of extracted loading logic
- Clean up unused imports and dependencies
- Update documentation and comments
- Maintain code organization and readability

## Success Criteria
- [ ] All existing llama-agent functionality preserved
- [ ] Model loading works identically to before
- [ ] Significant code reduction (~400-500 lines removed)
- [ ] All existing tests pass
- [ ] No breaking changes to public APIs
- [ ] Error handling identical to before
- [ ] Performance characteristics unchanged
- [ ] Cache integration provides performance benefits

## Critical Requirements
- **No functionality regressions** - everything must work exactly as before
- **Backward compatibility** - all existing code using llama-agent must continue to work
- **Same error behavior** - identical error messages and handling
- **Performance preservation** - no significant performance degradation

## Testing
- Run full test suite for llama-agent
- Test with real models to ensure loading works
- Verify error handling matches previous behavior
- Test cache integration provides benefits