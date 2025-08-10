# CLI Integration Tests

This directory contains comprehensive integration tests for the unified `llama-cli` that validate both `generate` and `embed` commands work correctly individually and together.

## Test Structure

### Test Data (`tests/data/`)
- `small_texts.txt`: 10 test sentences for quick functionality testing
- `medium_texts.txt`: 100 varied sentences for broader functionality testing  
- `large_texts.txt`: 1000 sentences for performance and scalability testing
- `multilingual.txt`: Unicode and multilingual text samples
- `edge_cases.txt`: Special characters, long texts, formatting edge cases
- `malformed.txt`: Invalid/malformed content for error handling tests

### Test Categories

#### 1. Generate Command Regression Tests
- `test_generate_command_compatibility`: Ensures existing generate functionality unchanged
- `test_generate_unchanged_behavior`: Validates various parameter combinations work identically

#### 2. Embed Command Basic Functionality
- `test_embed_command_basic_functionality`: Basic embed command with small dataset
- `test_embed_with_qwen_model`: Tests with Qwen embedding model specifically
- `test_embed_output_validation`: Validates Parquet output format and content

#### 3. Cross-Command Integration Tests
- `test_both_commands_same_session`: Both generate and embed commands in same CLI session
- `test_cache_sharing`: Validates model cache sharing works correctly
- `test_no_interference`: Commands don't interfere with each other

#### 4. Configuration Variation Tests
- `test_various_batch_sizes`: Tests batch sizes 1, 8, 32, 64
- `test_normalization_options`: Tests with and without embedding normalization
- `test_sequence_length_limits`: Tests various max sequence length settings
- `test_debug_mode`: Validates debug output functionality

#### 5. File Size and Scaling Tests
- `test_small_medium_large_inputs`: Tests with 10, 100, 1000 text inputs
- `test_unicode_multilingual`: Unicode and multilingual text processing
- `test_edge_cases`: Special formatting and edge case text handling

#### 6. Error Handling Tests
- `test_missing_files`: Missing input file handling
- `test_invalid_models`: Invalid model name/path handling
- `test_malformed_inputs`: Malformed input file handling
- `test_insufficient_permissions`: Permission error handling

#### 7. Performance Tests
- `test_performance_requirements`: Validates 1000 texts processed in reasonable time
- `test_memory_scaling`: Memory usage with different batch sizes
- `test_throughput_measurement`: Throughput metrics validation

## Test Models

The tests use real models for authentic validation:
- **Generation Model**: `unsloth/Qwen3-0.6B-GGUF`
- **Embedding Model**: `Qwen/Qwen3-Embedding-0.6B-GGUF`

## Running Tests

### Run All Integration Tests
```bash
cargo test --package llama-cli --test cli_integration_tests
```

### Run Specific Test Categories
```bash
# Generate command tests
cargo test --package llama-cli --test cli_integration_tests -- test_generate

# Embed command tests  
cargo test --package llama-cli --test cli_integration_tests -- test_embed

# Performance tests
cargo test --package llama-cli --test cli_integration_tests -- test_performance

# Error handling tests
cargo test --package llama-cli --test cli_integration_tests -- test_missing test_invalid test_malformed
```

### Run Individual Tests
```bash
cargo test --package llama-cli --test cli_integration_tests -- test_embed_command_basic_functionality --nocapture
```

## Test Behavior

### Model Loading Behavior
Tests are designed to handle model loading failures gracefully:
- If a model fails to load (network issues, missing files, etc.), tests will skip with informative messages
- Tests distinguish between argument parsing errors (which should fail) and model loading errors (which should skip)
- First run of tests may take longer due to model downloads

### Performance Expectations
- **First Run**: May take 3-5 minutes due to model downloads (~1-2GB)
- **Subsequent Runs**: Should be much faster with cached models
- **Performance Test**: Allows up to 300s for first run, 60s for cached runs

### Test Data Validation
Tests validate:
- CLI argument parsing correctness
- Command execution without crashes
- Output file creation and basic format validation
- Error message appropriateness
- Performance within reasonable bounds

## Test Implementation Details

### CliTestHelper
The `CliTestHelper` struct provides utilities for:
- Running CLI commands with proper argument handling
- Validating Parquet output files
- Measuring execution performance
- Handling temporary files and directories

### Test Patterns
Tests follow consistent patterns:
1. Initialize logging and test helper
2. Set up input files and output destinations
3. Execute CLI command with specific options
4. Validate results or error handling
5. Clean up temporary resources

### Error Handling Philosophy
Tests validate that:
- Argument parsing errors are reported clearly
- Model loading failures are handled gracefully
- File system errors are reported appropriately
- Invalid configurations are rejected with helpful messages

## CI/CD Integration

These tests are designed for CI/CD pipelines:
- **Timeout Handling**: Tests have reasonable timeouts
- **Resource Cleanup**: Temporary files are automatically cleaned up
- **Failure Reporting**: Clear error messages for debugging
- **Incremental Testing**: Individual test categories can be run independently

## Troubleshooting

### Common Issues

**Model Download Failures**:
- Check network connectivity
- Verify Hugging Face API access
- Check available disk space (models ~1-2GB each)

**Path Resolution Issues**:
- Tests expect to run from workspace root
- Test data files should be in `llama-cli/tests/data/`

**Performance Test Failures**:
- First run will be slow due to model downloads
- Subsequent runs should meet performance requirements
- Check system resources if tests consistently timeout

### Debug Mode
Run tests with debug output:
```bash
RUST_LOG=debug cargo test --package llama-cli --test cli_integration_tests -- --nocapture
```

## Success Criteria

Tests validate the following success criteria from the specification:

✅ **Functionality**
- Successfully loads Qwen embedding model
- Processes batches of text inputs correctly
- Generates valid Parquet output files
- Generate command maintains existing behavior

✅ **Integration**  
- Both generate and embed commands work in unified CLI
- Model caching works between commands
- No interference between command types
- Consistent error handling patterns

✅ **Performance**
- Processes 1000 texts in reasonable time (accounting for model download)
- Memory usage scales predictably with batch size
- Throughput metrics reported correctly

✅ **Error Handling**
- Missing files handled gracefully
- Invalid models reported clearly
- Malformed input processed or rejected appropriately
- Permission errors handled correctly

✅ **Output Validation**
- Parquet files created with correct structure
- File sizes appropriate for content
- Console output provides useful feedback
- Processing statistics reported accurately