# EMBEDDING_000009: Implement BatchProcessor for Efficient Processing

## Overview
Implement `BatchProcessor` that efficiently processes multiple texts in batches, with streaming support for large files and configurable batch sizes.

Refer to ./specification/embedding.md

## Tasks

### 1. Implement BatchProcessor Struct
```rust
// llama-embedding/src/batch.rs
pub struct BatchProcessor {
    model: Arc<EmbeddingModel>,
    batch_size: usize,
}
```

### 2. Batch Processing Methods
```rust
impl BatchProcessor {
    pub fn new(model: Arc<EmbeddingModel>, batch_size: usize) -> Self;
    pub async fn process_batch(&mut self, texts: &[String]) -> Result<Vec<EmbeddingResult>, EmbeddingError>;
    pub async fn process_texts(&mut self, texts: Vec<String>) -> Result<Vec<EmbeddingResult>, EmbeddingError>;
}
```

### 3. File Processing with Streaming
```rust
impl BatchProcessor {
    pub async fn process_file(&mut self, input_path: &Path) -> Result<impl Iterator<Item = EmbeddingResult>, EmbeddingError>;
    pub async fn process_file_streaming<F>(&mut self, input_path: &Path, callback: F) -> Result<(), EmbeddingError>
    where
        F: Fn(Vec<EmbeddingResult>) -> Result<(), EmbeddingError>;
}
```

### 4. Efficient Batch Processing
- Process texts in configurable batch sizes (default: 32)
- Minimize memory usage for large files
- Stream results to avoid memory accumulation
- Handle empty lines and invalid text gracefully
- Progress tracking and statistics

### 5. Memory Management
- Use streaming file reading to handle large inputs
- Process and yield results in batches to control memory
- Configurable batch sizes for different memory constraints
- Efficient text parsing and handling

### 6. Error Handling and Recovery
- Handle individual text processing failures within batches
- Continue processing on non-fatal errors
- Collect and report batch-level statistics
- Graceful handling of file reading errors

### 7. Performance Optimizations
- Minimize string copying and allocations
- Efficient batch preparation for model inference
- Parallel processing within batches where possible
- Memory reuse and pooling strategies

## Success Criteria
- [ ] BatchProcessor compiles and basic tests pass
- [ ] Can process batches of texts efficiently
- [ ] File streaming works for large inputs
- [ ] Memory usage scales predictably with batch size
- [ ] Error handling allows graceful continuation
- [ ] Performance suitable for 1000+ text processing
- [ ] Statistics and progress tracking work

## Testing Requirements
- Unit tests for batch processing logic
- Test with various batch sizes (1, 8, 32, 64)
- Test with large text files (1000+ lines)
- Test memory usage doesn't grow unbounded
- Test error handling and recovery
- Performance testing for reasonable throughput

## Integration Notes
- This will be the primary interface used by the CLI
- Must handle production-scale workloads efficiently  
- Focus on memory efficiency and throughput
- Should provide progress feedback capabilities