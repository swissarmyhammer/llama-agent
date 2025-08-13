# Implement Embed Args Validation Function

## Problem
The `validate_embed_args` function in `llama-cli/src/embed.rs:40` is marked as a placeholder and needs actual validation logic implemented.

## Current Implementation
```rust
// Placeholder validation function for EmbedArgs
pub fn validate_embed_args(args: &EmbedArgs) -> anyhow::Result<()> {
    // Validate model path
```

## Requirements
1. **Model Path Validation**: Verify the embedding model path exists and is accessible
2. **Input Validation**: Validate input files/text sources exist and are readable
3. **Output Validation**: Ensure output paths are writable and directories exist
4. **Parameter Validation**: Validate embedding-specific parameters (dimensions, batch size, etc.)
5. **Error Messages**: Provide clear, actionable error messages for validation failures

## Implementation Strategy
1. Add filesystem checks for model and input paths
2. Validate output directory permissions and create if needed
3. Implement parameter range and type validation
4. Add comprehensive error messages with suggestions
5. Add unit tests for validation scenarios

## Files to Modify
- `llama-cli/src/embed.rs:40` - Implement actual validation logic

## Success Criteria
- Invalid model paths are caught early with clear errors
- Missing input files are detected before processing starts
- Output path issues are resolved or reported clearly
- Parameter validation prevents runtime errors
- All validation scenarios are covered by tests