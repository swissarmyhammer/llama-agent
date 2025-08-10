# EMBEDDING_000015: End-to-End Integration Testing and Validation

## Overview
Create comprehensive end-to-end tests that validate the complete embedding system works correctly across all three crates (`llama-loader`, `llama-embedding`, `llama-cli`) with real models and production scenarios.

Refer to ./specification/embedding.md

## Tasks

### 1. Complete System Integration Tests
```rust
#[tokio::test]
async fn test_complete_embedding_pipeline() {
    // Test the complete flow from CLI command to Parquet output
    // Using real Qwen/Qwen3-Embedding-0.6B-GGUF model
    // Validate every step of the pipeline
    
    let input_texts = vec![
        "Hello world, this is a test sentence.",
        "The quick brown fox jumps over the lazy dog.",
        "Artificial intelligence is transforming our world.",
        "短い日本語のテスト文です。",
        "This is a much longer text that will test how the embedding model handles sequences of varying lengths and complexity, including punctuation, numbers like 123, and mixed content.",
    ];
    
    // Write test input file
    let input_file = write_test_input(&input_texts);
    let output_file = temp_dir().join("complete_test.parquet");
    
    // Run complete CLI command
    let start = Instant::now();
    let result = run_cli(&[
        "embed",
        "--model", "Qwen/Qwen3-Embedding-0.6B-GGUF",
        "--input", input_file.to_str().unwrap(),
        "--output", output_file.to_str().unwrap(),
        "--batch-size", "2",
        "--normalize",
    ]).await;
    let duration = start.elapsed();
    
    // Validate success
    assert!(result.is_ok(), "CLI command failed: {:?}", result.err());
    assert!(output_file.exists(), "Output file not created");
    
    // Validate performance (should be fast for small test)
    assert!(duration < Duration::from_secs(30), "Too slow: {:?}", duration);
    
    // Validate output file
    validate_complete_parquet_output(&output_file, &input_texts, true).await;
}
```

### 2. Cache Integration Validation
```rust
#[tokio::test]
async fn test_cache_sharing_across_crates() {
    // Test that model cache is shared between llama-agent and llama-embedding
    let cache_dir = temp_dir().join("test_cache");
    std::env::set_var("LLAMA_CACHE_DIR", cache_dir.to_str().unwrap());
    
    // First, load model via llama-agent (simulate generate command)
    let agent_start = Instant::now();
    let agent_result = test_agent_model_loading("Qwen/Qwen3-Embedding-0.6B-GGUF").await;
    let agent_duration = agent_start.elapsed();
    assert!(agent_result.is_ok());
    
    // Then, load same model via llama-embedding (should hit cache)  
    let embed_start = Instant::now();
    let embed_result = test_embedding_model_loading("Qwen/Qwen3-Embedding-0.6B-GGUF").await;
    let embed_duration = embed_start.elapsed();
    assert!(embed_result.is_ok());
    
    // Second load should be much faster (cache hit)
    assert!(embed_duration < agent_duration / 2, 
            "Cache not working: first={:?}, second={:?}", agent_duration, embed_duration);
}
```

### 3. Multi-Model Scenario Tests
```rust
#[tokio::test]
async fn test_multiple_models_workflow() {
    // Test realistic workflow with both generation and embedding models
    
    // 1. Generate some text with generation model
    let generated_texts = run_cli(&[
        "generate",
        "--model", "Qwen/Qwen2.5-7B-Instruct-GGUF",
        "--prompt", "Write 3 short sentences about AI.",
    ]).await?;
    
    // 2. Parse generated text into separate sentences
    let sentences = parse_generated_sentences(&generated_texts);
    let input_file = write_test_input(&sentences);
    
    // 3. Embed the generated sentences
    let output_file = temp_dir().join("generated_embeddings.parquet");
    let embed_result = run_cli(&[
        "embed",
        "--model", "Qwen/Qwen3-Embedding-0.6B-GGUF", 
        "--input", input_file.to_str().unwrap(),
        "--output", output_file.to_str().unwrap(),
    ]).await;
    
    assert!(embed_result.is_ok());
    validate_parquet_output(&output_file, sentences.len()).await;
}
```

### 4. Performance Benchmarking
```rust
#[tokio::test]
async fn test_production_performance_benchmark() {
    // Create large test dataset
    let test_texts: Vec<String> = (0..1000)
        .map(|i| format!("This is test sentence number {} with some additional content to make it more realistic for benchmarking purposes.", i))
        .collect();
    
    let input_file = write_test_input(&test_texts);
    let output_file = temp_dir().join("benchmark.parquet");
    
    // Benchmark different batch sizes
    for batch_size in [16, 32, 64, 128] {
        let start = Instant::now();
        let result = run_cli(&[
            "embed",
            "--model", "Qwen/Qwen3-Embedding-0.6B-GGUF",
            "--input", input_file.to_str().unwrap(),
            "--output", output_file.with_extension(&format!("batch_{}.parquet", batch_size)).to_str().unwrap(),
            "--batch-size", &batch_size.to_string(),
        ]).await;
        let duration = start.elapsed();
        
        assert!(result.is_ok(), "Batch size {} failed: {:?}", batch_size, result.err());
        assert!(duration < Duration::from_secs(60), "Batch size {} too slow: {:?}", batch_size, duration);
        
        println!("Batch size {}: {:.2}s ({:.1} texts/sec)", 
                batch_size, duration.as_secs_f64(), 1000.0 / duration.as_secs_f64());
    }
}
```

### 5. Error Recovery and Resilience Tests
```rust
#[tokio::test]  
async fn test_error_recovery_scenarios() {
    // Test various failure modes and recovery
    
    // Invalid model name
    let result1 = run_cli(&[
        "embed", "--model", "nonexistent/model", "--input", "test.txt", "--output", "out.parquet"
    ]).await;
    assert!(result1.is_err());
    assert!(result1.unwrap_err().to_string().contains("model"));
    
    // Insufficient disk space (if testable)
    // Network connectivity issues (if testable)
    // Corrupted input files
    let malformed_input = temp_dir().join("malformed.txt");
    std::fs::write(&malformed_input, b"\xFF\xFE invalid utf8 \xFF").unwrap();
    
    let result2 = run_cli(&[
        "embed",
        "--model", "Qwen/Qwen3-Embedding-0.6B-GGUF",
        "--input", malformed_input.to_str().unwrap(),
        "--output", "out.parquet",
    ]).await;
    
    // Should handle gracefully, not crash
    assert!(result2.is_err());
    assert!(result2.unwrap_err().to_string().contains("encoding") || 
            result2.unwrap_err().to_string().contains("utf8"));
}
```

### 6. Memory Usage Validation
```rust
#[tokio::test]
async fn test_memory_usage_scalability() {
    // Test that memory usage scales with batch size, not dataset size
    let large_dataset: Vec<String> = (0..10000)
        .map(|i| format!("Large dataset test sentence number {} with substantial content to test memory usage patterns in batch processing scenarios.", i))
        .collect();
    
    let input_file = write_test_input(&large_dataset);
    
    // Test with small batch size (should use minimal memory)
    let small_batch_result = run_cli_with_memory_monitoring(&[
        "embed",
        "--model", "Qwen/Qwen3-Embedding-0.6B-GGUF",
        "--input", input_file.to_str().unwrap(),
        "--output", "small_batch.parquet",
        "--batch-size", "8",
    ]).await;
    
    assert!(small_batch_result.is_ok());
    let small_batch_memory = small_batch_result.unwrap().max_memory_mb;
    
    // Test with large batch size (should use more memory, but not proportional to dataset size)
    let large_batch_result = run_cli_with_memory_monitoring(&[
        "embed", 
        "--model", "Qwen/Qwen3-Embedding-0.6B-GGUF",
        "--input", input_file.to_str().unwrap(),
        "--output", "large_batch.parquet",
        "--batch-size", "128",
    ]).await;
    
    assert!(large_batch_result.is_ok());
    let large_batch_memory = large_batch_result.unwrap().max_memory_mb;
    
    // Memory should scale with batch size, not linearly with dataset size
    assert!(large_batch_memory > small_batch_memory);
    assert!(large_batch_memory < small_batch_memory * 50); // Not 50x more memory for 10k texts
}
```

### 7. Cross-Platform Validation
```rust
#[tokio::test]
async fn test_cross_platform_compatibility() {
    // Test that cache directories work correctly on different platforms
    // Test that file paths and encoding work correctly
    // Test that model loading works across platforms
    
    let platform_cache_dir = llama_loader::get_default_cache_dir();
    assert!(platform_cache_dir.exists() || platform_cache_dir.parent().unwrap().exists());
    
    // Test with paths containing spaces and unicode
    let unicode_dir = temp_dir().join("测试 directory with spaces");
    std::fs::create_dir_all(&unicode_dir).unwrap();
    
    let unicode_input = unicode_dir.join("输入.txt");
    let unicode_output = unicode_dir.join("输出.parquet");
    
    write_test_input_to_file(&unicode_input, &["Test with unicode paths"]);
    
    let result = run_cli(&[
        "embed",
        "--model", "Qwen/Qwen3-Embedding-0.6B-GGUF",
        "--input", unicode_input.to_str().unwrap(),
        "--output", unicode_output.to_str().unwrap(),
    ]).await;
    
    assert!(result.is_ok(), "Unicode path handling failed: {:?}", result.err());
    assert!(unicode_output.exists(), "Unicode output file not created");
}
```

## Success Criteria
- [ ] Complete system works end-to-end with real models
- [ ] Cache sharing works between all crates
- [ ] Performance meets requirements across different scenarios
- [ ] Error handling robust for production scenarios
- [ ] Memory usage scales appropriately
- [ ] Cross-platform compatibility validated
- [ ] Multi-model workflows work correctly
- [ ] All edge cases handled gracefully

## Test Infrastructure
```rust
// Test utilities for end-to-end testing
struct MemoryUsage {
    max_memory_mb: u64,
    avg_memory_mb: u64,
}

async fn run_cli(args: &[&str]) -> Result<String, Box<dyn std::error::Error>>;
async fn run_cli_with_memory_monitoring(args: &[&str]) -> Result<MemoryUsage, Box<dyn std::error::Error>>;
fn write_test_input(texts: &[String]) -> PathBuf;
fn write_test_input_to_file(path: &Path, texts: &[&str]);
async fn validate_complete_parquet_output(path: &Path, expected_texts: &[String], normalized: bool);
async fn validate_parquet_output(path: &Path, expected_count: usize);
```

## Integration Notes
- These tests validate production readiness
- Must run with real models in CI/CD
- Benchmarks guide performance optimization
- Establishes confidence for production deployment
- Tests the complete specification implementation