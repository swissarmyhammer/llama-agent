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

## Proposed Solution

Based on analysis of the current validation function and the available ModelSource validation in `llama-loader`, here's the implementation plan:

### Current State Analysis
The current validation function has basic checks but lacks comprehensive validation:
- Basic model path empty check (not leveraging ModelSource validation)
- Input file existence check (but no readability or content validation)  
- Output directory creation (but no write permission validation)
- Basic batch size and max_length bounds checking

### Enhancement Plan
1. **Enhanced Model Path Validation**: Use `ModelSource` validation from `llama-loader` to properly validate both HuggingFace repos and local paths with GGUF file format checking
2. **Improved Input Validation**: Check file readability, non-empty content, and proper text format
3. **Better Output Validation**: Check write permissions and validate output file extension (.parquet)
4. **Comprehensive Parameter Validation**: Add more robust bounds checking with configurable limits
5. **Better Error Messages**: Use consistent error formatting similar to `ModelError` with actionable suggestions
6. **Unit Tests**: Add comprehensive tests covering all validation scenarios

### Implementation Steps
- Enhance `validate_embed_args` function with proper `ModelSource` validation
- Add filesystem permission checks for input/output files
- Improve parameter bounds validation with reasonable defaults
- Add comprehensive unit tests covering edge cases and error conditions

This will provide early validation to catch configuration issues before expensive model loading operations begin.
## Implementation Complete

### Summary 
Successfully implemented comprehensive validation logic for the `validate_embed_args` function in `llama-cli/src/embed.rs`. The validation now provides early error detection with actionable error messages before expensive model loading operations.

### Changes Made

1. **Enhanced Model Path Validation**: 
   - Uses `ModelSource` validation from `llama-loader`
   - Properly validates both HuggingFace repos and local paths
   - Validates GGUF file format requirements

2. **Improved Input File Validation**:
   - Checks file existence and readability  
   - Validates file is not a directory
   - Ensures file contains content (not empty)
   - Tests actual file permissions

3. **Better Output Path Validation**:
   - Enforces `.parquet` file extension
   - Creates output directories as needed
   - Tests write permissions before processing starts

4. **Comprehensive Parameter Validation**:
   - Batch size bounds (1-1024) with helpful suggestions
   - Max length bounds (32-8192) with semantic guidance
   - Clear error messages with actionable recommendations

5. **Enhanced Error Messages**:
   - All errors include 💡 suggestions for resolution
   - Consistent formatting similar to `ModelError` style
   - Specific guidance for common configuration mistakes

6. **Comprehensive Unit Tests**: 
   - 21 test cases covering all validation scenarios
   - Edge case testing for boundary conditions
   - Integration tests with various valid configurations
   - Error message quality validation

### Test Results
- ✅ All 21 validation tests passing
- ✅ No clippy warnings  
- ✅ Code formatted with `cargo fmt`
- ✅ Full integration with existing embed command workflow

The validation function now provides robust early error detection that will save users time by catching configuration issues before attempting model loading operations.