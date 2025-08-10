# EMBEDDING_000010: llama-embedding Integration Testing

## Overview
Create comprehensive integration tests for the `llama-embedding` library using a real embedding model, focusing on end-to-end functionality and performance validation.

Refer to ./specification/embedding.md

## Tasks

### 1. Test Model: Qwen/Qwen3-Embedding-0.6B-GGUF
- Set up integration tests with `Qwen/Qwen3-Embedding-0.6B-GGUF` model
- Test both HuggingFace downloading and local model loading
- Verify embedding dimensions (should be 384 for this model)
- Test model caching integration with llama-loader

### 2. Single Text Embedding Tests
```rust
#[tokio::test]
async fn test_single_text_embedding() {
    let config = EmbeddingConfig::new(/* Qwen model config */);
    let mut model = EmbeddingModel::new(config).await.unwrap();
    model.load_model().await.unwrap();
    
    let result = model.embed_text("Hello world").await.unwrap();
    assert_eq!(result.embedding.len(), 384);
    assert!(!result.text_hash.is_empty());
    assert_eq!(result.text, "Hello world");
}
```

### 3. Batch Processing Tests
- Test various batch sizes: 1, 8, 32, 64
- Test with different text lengths
- Test with empty strings and edge cases
- Verify batch processing produces same results as individual processing

### 4. File Processing Tests
- Create test files with various sizes: 10, 100, 1000 texts
- Test streaming file processing
- Test memory usage doesn't grow with file size
- Test error handling for malformed input files

### 5. Performance Validation Tests
```rust
#[tokio::test]
async fn test_performance_requirements() {
    // Test: Process 1000 texts in under 60 seconds
    let texts: Vec<String> = generate_test_texts(1000);
    let start = Instant::now();
    let results = processor.process_texts(texts).await.unwrap();
    assert!(start.elapsed() < Duration::from_secs(60));
    assert_eq!(results.len(), 1000);
}
```

### 6. MD5 Hash Consistency Tests
- Test that same text produces same MD5 hash
- Test that different texts produce different hashes
- Test hash consistency across processing methods

### 7. Error Handling Tests
- Test model loading failures
- Test invalid text input handling
- Test file I/O error scenarios
- Test graceful degradation on processing errors

### 8. Integration with llama-loader Tests
- Test cache hit/miss scenarios
- Test model sharing between multiple EmbeddingModel instances
- Test cache persistence and retrieval
- Verify no memory leaks in model loading/unloading

## Test Data Setup
```rust
// Test texts covering various scenarios
const TEST_TEXTS: &[&str] = &[
    "Hello world, this is a test sentence.",
    "The quick brown fox jumps over the lazy dog.",
    "Artificial intelligence is transforming our world.",
    "短い日本語のテスト文です。", // Unicode/multilingual
    "", // Empty string edge case
    "This is a much longer text that will test how the embedding model handles sequences of varying lengths and complexity, including punctuation, numbers like 123, and mixed content.",
];
```

## Success Criteria
- [ ] All integration tests pass consistently
- [ ] Qwen embedding model loads and works correctly
- [ ] Embedding dimensions match expected (384)
- [ ] Performance meets requirements (1000 texts < 60s)
- [ ] Memory usage scales predictably
- [ ] MD5 hashing works correctly
- [ ] Error handling robust and informative
- [ ] Cache integration works properly
- [ ] No memory leaks or resource issues

## Critical Requirements
- Tests must use real embedding model, not mocks
- Performance requirements must be validated
- Memory usage must be monitored and validated
- All edge cases and error conditions tested
- Integration with llama-loader fully validated

## Integration Notes
- These tests validate the complete library functionality
- Will be used to validate CLI integration in later steps
- Performance benchmarks guide optimization efforts
- Establishes baseline for production usage

## Proposed Solution

After analyzing the current codebase and specification, I will implement comprehensive integration tests for the llama-embedding library focusing on real-world usage with the Qwen/Qwen3-Embedding-0.6B-GGUF model. 

### Implementation Plan

1. **Test Infrastructure Setup**
   - Create `real_model_integration_test.rs` for comprehensive integration tests with actual models
   - Set up test data generator with various text scenarios (multilingual, edge cases, different lengths)
   - Configure proper test timeouts and performance monitoring

2. **Model Integration Tests**
   - Single text embedding validation with dimension verification (384 for Qwen model)
   - HuggingFace model downloading and caching integration
   - Local model loading fallback scenarios
   - Model metadata and configuration validation

3. **Batch Processing Validation**
   - Test batch sizes: 1, 8, 32, 64 with performance monitoring
   - Memory usage scaling verification
   - Batch consistency (same results as individual processing)
   - Edge case handling (empty strings, very long texts)

4. **File Processing Tests**
   - Streaming file processing with various sizes (10, 100, 1000 texts)
   - Memory efficiency validation (constant memory usage regardless of file size)
   - UTF-8 and multilingual text handling
   - Line-by-line processing accuracy

5. **Performance Requirements**
   - Benchmark test: 1000 texts processed in under 60 seconds
   - Memory usage profiling and limits validation
   - Processing time consistency across batches
   - GPU/CPU utilization monitoring

6. **Hash Consistency & Error Handling**
   - MD5 hash determinism across multiple runs
   - Model loading failure scenarios
   - Invalid input handling and graceful degradation
   - Integration with llama-loader error propagation

7. **Cache Integration Testing**
   - Test shared cache between multiple EmbeddingModel instances
   - Cache hit/miss performance validation
   - Model persistence and retrieval accuracy
   - Cache cleanup and memory leak prevention

### Key Changes

- Create comprehensive test suite in `tests/real_model_integration_test.rs`
- Add performance monitoring utilities and test helpers
- Implement test data generation with comprehensive edge cases
- Add memory usage tracking and validation
- Create cache integration test scenarios
- Set up proper test configuration for CI/CD environments

### Success Validation

- All integration tests pass with real Qwen model
- Performance benchmarks meet specified requirements (< 60s for 1000 texts)
- Memory usage scales predictably with batch size, not file size
- Cache integration works correctly with llama-loader
- Error handling is robust and informative
- No memory leaks or resource issues detected

This approach ensures the llama-embedding library is production-ready with comprehensive real-world validation.