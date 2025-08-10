# EMBEDDING_000013: Implement Embed Command

## Overview
Implement the `embed` command functionality that integrates `llama-embedding` library with `ParquetWriter` to provide a complete CLI embedding solution.

Refer to ./specification/embedding.md

## Tasks

### 1. Implement Embed Command Logic
```rust
// llama-cli/src/embed.rs
use llama_embedding::{EmbeddingModel, EmbeddingConfig, BatchProcessor};
use crate::parquet_writer::ParquetWriter;

pub async fn run_embed_command(args: EmbedArgs) -> Result<(), Box<dyn std::error::Error>> {
    // Implementation here
}
```

### 2. CLI Args to Config Conversion
```rust
impl EmbedArgs {
    fn to_embedding_config(&self) -> EmbeddingConfig {
        EmbeddingConfig {
            model_source: ModelSource::from_string(&self.model, self.filename.clone()),
            normalize_embeddings: self.normalize,
            max_sequence_length: self.max_length,
            debug: self.debug,
        }
    }
}
```

### 3. Complete Embed Pipeline
```rust
pub async fn run_embed_command(args: EmbedArgs) -> Result<(), Box<dyn std::error::Error>> {
    // 1. Create embedding config from CLI args
    let config = args.to_embedding_config();
    
    // 2. Initialize embedding model
    let mut embedding_model = EmbeddingModel::new(config).await?;
    embedding_model.load_model().await?;
    
    // 3. Get embedding dimensions for Parquet schema
    let embedding_dim = embedding_model.get_embedding_dimension()
        .ok_or("Could not determine embedding dimensions")?;
    
    // 4. Set up batch processor and Parquet writer
    let model = Arc::new(embedding_model);
    let mut processor = BatchProcessor::new(model.clone(), args.batch_size);
    let mut parquet_writer = ParquetWriter::new(&args.output, embedding_dim, args.batch_size)?;
    
    // 5. Process file and write to Parquet with progress tracking
    let mut total_processed = 0;
    processor.process_file_streaming(&args.input, |batch| {
        total_processed += batch.len();
        parquet_writer.write_batch(batch)?;
        // Update progress here
        Ok(())
    }).await?;
    
    // 6. Close writer and show summary
    parquet_writer.close()?;
    println!("Embeddings written to: {}", args.output.display());
    println!("Total embeddings: {}", total_processed);
    Ok(())
}
```

### 4. Progress Tracking and User Feedback
- Real-time progress bars during processing
- Processing statistics (texts/sec, time elapsed)
- Memory usage monitoring
- File size and embedding count reporting
- Clear error messages and troubleshooting hints

### 5. Console Output Design
```bash
$ llama-cli embed --model Qwen/Qwen3-Embedding-0.6B-GGUF --input texts.txt --output embeddings.parquet
Loading model: Qwen/Qwen3-Embedding-0.6B-GGUF
Model loaded successfully in 1.8s (384 dimensions)
Processing 1,000 texts with batch size 32...

Progress: [████████████████████] 1000/1000 (100%) - 45.2s elapsed
Average processing time: 45.2ms per text
Total embeddings: 1,000
Output written to: embeddings.parquet (2.1 MB)
```

### 6. Error Handling and Recovery
- Graceful handling of model loading failures
- Clear error messages for file I/O issues
- Recovery strategies for processing failures
- Validation of input file format
- Helpful error messages for common issues

### 7. Integration with Main CLI
```rust
// llama-cli/src/main.rs
match cli.command {
    Commands::Generate(args) => {
        // Existing generate logic
    }
    Commands::Embed(args) => {
        crate::embed::run_embed_command(args).await?;
    }
}
```

## Success Criteria
- [ ] Embed command works end-to-end with real models
- [ ] Integrates llama-embedding and ParquetWriter correctly
- [ ] Progress tracking provides good user feedback
- [ ] Error handling is robust and helpful
- [ ] Performance meets requirements (1000 texts < 60s)
- [ ] Output Parquet files are valid and correct
- [ ] Memory usage scales with batch size, not file size
- [ ] Console output is informative and user-friendly

## Testing Requirements
- Integration tests with Qwen embedding model
- Test with various input file sizes (10, 100, 1000 texts)
- Test with different batch sizes and configurations
- Test error handling for various failure modes
- Validate Parquet output correctness
- Performance testing for throughput requirements

## CLI Usage Examples
```bash
# Basic usage
llama-cli embed --model Qwen/Qwen3-Embedding-0.6B-GGUF --input texts.txt --output embeddings.parquet

# With options
llama-cli embed \
  --model Qwen/Qwen3-Embedding-0.6B-GGUF \
  --input large_corpus.txt \
  --output embeddings.parquet \
  --batch-size 64 \
  --normalize \
  --max-length 512 \
  --debug

# Local model
llama-cli embed \
  --model ./models/qwen3-embedding \
  --filename model.gguf \
  --input texts.txt \
  --output embeddings.parquet
```

## Integration Notes
- This completes the core embedding functionality
- Provides production-ready CLI embedding tool
- Should handle real-world workloads efficiently
- Focus on user experience and reliability

## Proposed Solution

After analyzing the current codebase, I'll implement the embed command with the following approach:

### 1. Implementation Strategy

The embed command will be implemented in `llama-cli/src/embed.rs` by:

1. **Converting CLI args to EmbeddingConfig**: Map the EmbedArgs to the internal config structure
2. **Model Loading**: Initialize and load the embedding model using the llama-embedding crate
3. **Batch Processing**: Use BatchProcessor for efficient text processing with streaming
4. **Parquet Output**: Write results using the existing ParquetWriter
5. **Progress Tracking**: Implement progress bars and user feedback
6. **Error Handling**: Provide clear error messages and recovery

### 2. Key Implementation Details

```rust
pub async fn run_embed_command(args: EmbedArgs) -> anyhow::Result<()> {
    // 1. Validate input arguments
    validate_embed_args(&args)?;
    
    // 2. Convert args to embedding config
    let config = EmbeddingConfig {
        model_source: ModelSource::from_string(&args.model, args.filename),
        normalize_embeddings: args.normalize,
        max_sequence_length: args.max_length,
        debug: args.debug,
    };
    
    // 3. Initialize and load model
    let mut embedding_model = EmbeddingModel::new(config).await?;
    embedding_model.load_model().await?;
    
    // 4. Get embedding dimensions and setup writer
    let embedding_dim = embedding_model.get_embedding_dimension()
        .ok_or_else(|| anyhow::anyhow!("Could not determine embedding dimensions"))?;
    
    // 5. Setup batch processor with progress tracking
    let model = Arc::new(embedding_model);
    let mut processor = BatchProcessor::new(model.clone(), args.batch_size);
    let mut parquet_writer = ParquetWriter::new(&args.output, embedding_dim, args.batch_size)?;
    
    // 6. Process file with streaming and progress
    let mut total_processed = 0;
    processor.process_file_streaming(&args.input, |batch| {
        total_processed += batch.len();
        parquet_writer.write_batch(batch)?;
        // Update progress display
        Ok(())
    }).await?;
    
    // 7. Finalize and show results
    parquet_writer.close()?;
    println!("Embeddings written to: {}", args.output.display());
    println!("Total embeddings: {}", total_processed);
    
    Ok(())
}
```

### 3. Progress Tracking Implementation

Will add console output that matches the specification:
- Model loading progress with timing
- Batch processing with progress bars 
- Throughput statistics
- Clear error messages

### 4. Integration Points

- Update main.rs to call `crate::embed::run_embed_command(args).await?`
- Use existing ParquetWriter for output
- Leverage BatchProcessor streaming capabilities
- Follow existing error handling patterns

This solution provides a complete end-to-end embed command that integrates all the existing components while providing good user experience with progress tracking and clear feedback.