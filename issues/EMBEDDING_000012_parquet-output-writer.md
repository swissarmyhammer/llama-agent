# EMBEDDING_000012: Implement Parquet Output Writer

## Overview
Create a `ParquetWriter` that efficiently writes embedding results to Apache Parquet files with the specified schema, supporting streaming writes for large datasets.

Refer to ./specification/embedding.md

## Tasks

### 1. Add Parquet Dependencies
```toml
# llama-cli/Cargo.toml
[dependencies]
arrow = { workspace = true }
arrow-array = { workspace = true }  
arrow-schema = { workspace = true }
parquet = { workspace = true }
```

### 2. Define Parquet Schema
```rust
// llama-cli/src/parquet_writer.rs
// Schema:
// - text: Utf8
// - text_hash: Utf8 (MD5)
// - embedding: FixedSizeList<Float32>
// - sequence_length: UInt32  
// - processing_time_ms: UInt64
```

### 3. Implement ParquetWriter
```rust
pub struct ParquetWriter {
    schema: Schema,
    writer: ArrowWriter<File>,
    batch_buffer: Vec<EmbeddingResult>,
    batch_size: usize,
    embedding_dim: usize,
}

impl ParquetWriter {
    pub fn new(output_path: &Path, embedding_dim: usize, batch_size: usize) -> Result<Self, ParquetError>;
    pub fn write_batch(&mut self, results: Vec<EmbeddingResult>) -> Result<(), ParquetError>;
    pub fn flush(&mut self) -> Result<(), ParquetError>;
    pub fn close(self) -> Result<ParquetMetadata, ParquetError>;
}
```

### 4. Efficient Batch Writing
- Buffer embedding results to write in batches
- Convert `EmbeddingResult` to Arrow arrays efficiently
- Handle variable-length text and fixed-size embeddings
- Optimize memory usage for large datasets

### 5. Schema Conversion Logic
```rust
fn convert_to_arrow_batch(results: &[EmbeddingResult], embedding_dim: usize) -> RecordBatch {
    // Convert Vec<EmbeddingResult> to Arrow RecordBatch
    // Handle text (Utf8Array)
    // Handle text_hash (Utf8Array)
    // Handle embedding (FixedSizeListArray of Float32)
    // Handle sequence_length (UInt32Array)
    // Handle processing_time_ms (UInt64Array)
}
```

### 6. Error Handling
```rust
#[derive(thiserror::Error, Debug)]
pub enum ParquetError {
    #[error("Arrow error: {0}")]
    Arrow(#[from] arrow::error::ArrowError),
    
    #[error("Parquet error: {0}")]
    Parquet(#[from] parquet::errors::ParquetError),
    
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("Schema mismatch: expected {expected} dimensions, got {actual}")]
    SchemaMismatch { expected: usize, actual: usize },
}
```

### 7. Streaming Support
- Write results incrementally to avoid memory accumulation
- Handle backpressure for very large datasets
- Provide progress callback integration
- Efficient file I/O with proper buffering

## Success Criteria
- [ ] ParquetWriter compiles and basic tests pass
- [ ] Can create valid Parquet files with correct schema
- [ ] Handles various embedding dimensions correctly
- [ ] Streaming writes work for large datasets
- [ ] Memory usage scales with batch size, not dataset size
- [ ] Generated files are readable by other Parquet tools
- [ ] Error handling robust and informative
- [ ] Performance suitable for production workloads

## Testing Requirements
- Unit tests for schema conversion
- Test with various embedding dimensions (128, 384, 768, etc.)
- Test with different batch sizes
- Test file output correctness with external Parquet readers
- Test memory usage with large datasets
- Test error handling for I/O failures

## Output Example
```
┌─────────────────────────┬──────────────────────────────────┬─────────────────┬─────────────────┬────────────────────┐
│ text                    │ text_hash                        │ embedding       │ sequence_length │ processing_time_ms │
│ ---                     │ ---                              │ ---             │ ---             │ ---                │
│ str                     │ str                              │ list<f32>[384]  │ u32             │ u64                │
├─────────────────────────┼──────────────────────────────────┼─────────────────┼─────────────────┼────────────────────┤
│ Hello world, this is... │ a1b2c3d4e5f6...                  │ [0.1, 0.2, ...] │ 8               │ 45                 │
│ The quick brown fox...  │ f6e5d4c3b2a1...                  │ [0.3, 0.4, ...] │ 10              │ 52                 │
└─────────────────────────┴──────────────────────────────────┴─────────────────┴─────────────────┴────────────────────┘
```

## Integration Notes
- Will be used by embed command implementation
- Must handle production-scale datasets efficiently
- Focus on performance and memory efficiency
- Should provide detailed error messages for debugging