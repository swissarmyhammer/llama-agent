# EMBEDDING_000014: CLI Integration Testing

## Overview
Create comprehensive integration tests for the unified `llama-cli` that validate both `generate` and `embed` commands work correctly individually and together.

Refer to ./specification/embedding.md

## Tasks

### 1. Generate Command Regression Testing
```rust
#[tokio::test]
async fn test_generate_command_compatibility() {
    // Test that existing generate functionality works identically
    // Compare with previous llama-agent-cli behavior
    // Ensure all command-line options work
    // Verify output format unchanged
}
```

### 2. Embed Command Integration Tests
```rust
#[tokio::test]  
async fn test_embed_command_basic_functionality() {
    // Test basic embed command with Qwen model
    let temp_input = create_test_input_file();
    let temp_output = temp_dir().join("embeddings.parquet");
    
    // Run: llama-cli embed --model Qwen/Qwen3-Embedding-0.6B-GGUF --input input.txt --output output.parquet
    let result = run_cli_command(&["embed", "--model", "Qwen/Qwen3-Embedding-0.6B-GGUF", 
                                   "--input", temp_input.to_str().unwrap(),
                                   "--output", temp_output.to_str().unwrap()]).await;
    
    assert!(result.is_ok());
    assert!(temp_output.exists());
    
    // Validate Parquet file contents
    let parquet_data = read_parquet_file(&temp_output).unwrap();
    validate_embedding_data(&parquet_data);
}
```

### 3. Cross-Command Integration Tests
```rust
#[tokio::test]
async fn test_both_commands_in_same_session() {
    // Test that generate and embed can be used in same session
    // Verify no interference between commands
    // Test shared model caching works correctly
    
    // First generate text
    let generate_result = run_cli_command(&["generate", "--model", "Qwen/Qwen2.5-7B-Instruct-GGUF", "--prompt", "Test"]).await;
    assert!(generate_result.is_ok());
    
    // Then embed text  
    let embed_result = run_cli_command(&["embed", "--model", "Qwen/Qwen3-Embedding-0.6B-GGUF", "--input", "test.txt", "--output", "embeddings.parquet"]).await;
    assert!(embed_result.is_ok());
}
```

### 4. Various Configuration Tests
- Test different batch sizes: 1, 8, 32, 64
- Test with and without normalization
- Test with different max sequence lengths
- Test debug mode functionality
- Test local model loading vs HuggingFace

### 5. File Format and Size Tests
```rust
#[tokio::test]
async fn test_various_input_sizes() {
    // Test with different input file sizes
    for text_count in [10, 100, 1000] {
        let input_file = create_test_file_with_texts(text_count);
        let output_file = temp_dir().join(format!("embeddings_{}.parquet", text_count));
        
        let result = run_embed_command(&input_file, &output_file).await;
        assert!(result.is_ok());
        
        let parquet_data = read_parquet_file(&output_file).unwrap();
        assert_eq!(parquet_data.len(), text_count);
    }
}
```

### 6. Error Handling Integration Tests
```rust
#[tokio::test]
async fn test_error_handling_scenarios() {
    // Test missing input file
    // Test invalid model name
    // Test insufficient permissions
    // Test disk space issues (if feasible)
    // Test invalid batch sizes
    // Test malformed input files
}
```

### 7. Performance Integration Tests
```rust
#[tokio::test]
async fn test_performance_requirements() {
    let large_input = create_test_file_with_texts(1000);
    let output = temp_dir().join("large_embeddings.parquet");
    
    let start = Instant::now();
    let result = run_embed_command_with_batch_size(&large_input, &output, 32).await;
    let duration = start.elapsed();
    
    assert!(result.is_ok());
    assert!(duration < Duration::from_secs(60)); // Must process 1000 texts in under 60s
    
    // Validate output correctness
    let parquet_data = read_parquet_file(&output).unwrap();
    assert_eq!(parquet_data.len(), 1000);
}
```

### 8. Output Validation Tests
```rust
fn validate_embedding_data(parquet_data: &[EmbeddingRecord]) {
    for record in parquet_data {
        // Validate text field is non-empty
        assert!(!record.text.is_empty());
        
        // Validate MD5 hash format
        assert_eq!(record.text_hash.len(), 32);
        assert!(record.text_hash.chars().all(|c| c.is_ascii_hexdigit()));
        
        // Validate embedding dimensions (should be 384 for Qwen)
        assert_eq!(record.embedding.len(), 384);
        
        // Validate metadata fields
        assert!(record.sequence_length > 0);
        assert!(record.processing_time_ms > 0);
        
        // Validate normalization if requested
        if normalized {
            let norm: f32 = record.embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
            assert!((norm - 1.0).abs() < 1e-6);
        }
    }
}
```

### 9. Test Utilities
```rust
// Helper functions for integration testing
async fn run_cli_command(args: &[&str]) -> Result<String, Box<dyn std::error::Error>>;
fn create_test_input_file() -> PathBuf;
fn create_test_file_with_texts(count: usize) -> PathBuf;
fn read_parquet_file(path: &Path) -> Result<Vec<EmbeddingRecord>, ParquetError>;
async fn run_embed_command(input: &Path, output: &Path) -> Result<(), Box<dyn std::error::Error>>;
```

## Success Criteria
- [ ] All integration tests pass consistently
- [ ] Generate command works identically to old CLI
- [ ] Embed command produces valid Parquet output
- [ ] Performance requirements met (1000 texts < 60s)
- [ ] Error handling robust for various failure modes
- [ ] Memory usage scales predictably
- [ ] Both commands can be used in same session
- [ ] Cache sharing works between commands
- [ ] Output validation confirms correctness

## Test Data Requirements
```
tests/data/
├── small_texts.txt      # 10 texts for quick tests
├── medium_texts.txt     # 100 texts for functionality tests  
├── large_texts.txt      # 1000 texts for performance tests
├── multilingual.txt     # Unicode/multilingual test cases
├── edge_cases.txt       # Empty lines, long texts, special chars
└── malformed.txt        # Invalid input for error testing
```

## Integration Notes
- These tests validate the complete CLI functionality
- Must use real models, not mocks
- Should run in CI/CD pipeline
- Performance benchmarks guide optimization
- Establishes production readiness baseline