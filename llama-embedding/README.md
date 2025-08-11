# llama-embedding

High-performance batch text embedding library for LLaMA models.

## Features
- Efficient batch processing with configurable sizes
- Streaming support for large text files
- MD5 text hashing for deduplication
- Optional L2 normalization
- Integration with llama-loader for model management
- Apache Parquet output format
- Progress tracking for large datasets

## Usage

### Basic Embedding
```rust
use llama_embedding::{EmbeddingModel, EmbeddingConfig, BatchProcessor};

let config = EmbeddingConfig::new(model_source, batch_size);
let mut model = EmbeddingModel::new(config).await?;
model.load_model().await?;

let result = model.embed_text("Hello world").await?;
println!("Embedding: {:?}", result.embedding);
```

### Batch Processing
```rust
use llama_embedding::BatchProcessor;

let processor = BatchProcessor::new(config).await?;
let texts = vec!["Text 1".to_string(), "Text 2".to_string()];
let results = processor.process_batch(&texts).await?;

for result in results {
    println!("Text: {}, Embedding dimensions: {}", 
             result.original_text, result.embedding.len());
}
```

### File Processing
```rust
use llama_embedding::FileProcessor;

let processor = FileProcessor::new(config).await?;
let results = processor
    .process_file("input.txt")
    .await?;
    
// Results are automatically written to Parquet format
println!("Processed {} embeddings", results.len());
```

## Configuration

### EmbeddingConfig
```rust
use llama_embedding::{EmbeddingConfig, ModelSource};

let config = EmbeddingConfig {
    model_source: ModelSource::HuggingFace {
        repo: "Qwen/Qwen3-Embedding-0.6B-GGUF".to_string(),
        filename: None,
    },
    batch_size: 32,
    max_sequence_length: Some(512),
    normalize_embeddings: true,
    progress_callback: Some(Box::new(|progress| {
        println!("Progress: {:.1}%", progress * 100.0);
    })),
};
```

### Batch Size Guidelines
- **Small batch (8-16)**: Lower memory usage, better for limited resources
- **Medium batch (32-64)**: Balanced performance and memory usage (recommended)
- **Large batch (128+)**: Higher throughput, requires more memory

## Output Format

Embeddings are saved in Apache Parquet format with the following schema:
- `text`: Original input text (string)
- `text_hash`: MD5 hash for deduplication (string)
- `embedding`: Float32 array of embedding values
- `embedding_norm`: L2 norm of the embedding (if normalization enabled)

### Reading Parquet Files
```rust
use polars::prelude::*;

let df = LazyFrame::scan_parquet("embeddings.parquet", ScanArgsParquet::default())?
    .collect()?;
    
println!("Loaded {} embeddings", df.height());
```

## Performance Characteristics

- **Throughput**: 20-50 texts/second (model and hardware dependent)
- **Memory**: Scales with batch size, not total dataset size
- **Streaming**: Processes large files without loading everything into memory
- **Deduplication**: MD5 hashing prevents processing duplicate texts

## Error Handling

The crate provides comprehensive error types:
- `EmbeddingError::ModelError`: Model loading or inference errors
- `EmbeddingError::ProcessingError`: Text processing failures
- `EmbeddingError::IoError`: File I/O problems
- `EmbeddingError::FormatError`: Output format issues

## Integration with llama-loader

This crate uses `llama-loader` for model management, providing:
- Automatic model downloading from HuggingFace
- Intelligent caching with LRU eviction
- Shared cache with other crates (like `llama-agent`)
- Retry logic for network failures

## Examples

### CLI-style Processing
```rust
use llama_embedding::{EmbeddingConfig, FileProcessor, ModelSource};
use std::path::PathBuf;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = EmbeddingConfig {
        model_source: ModelSource::HuggingFace {
            repo: "Qwen/Qwen3-Embedding-0.6B-GGUF".to_string(),
            filename: None,
        },
        batch_size: 32,
        max_sequence_length: None,
        normalize_embeddings: true,
        progress_callback: Some(Box::new(|progress| {
            println!("Progress: {:.1}%", progress * 100.0);
        })),
    };

    let processor = FileProcessor::new(config).await?;
    let output_path = processor
        .process_file_to_parquet("input.txt", "output.parquet")
        .await?;
        
    println!("Embeddings saved to: {}", output_path.display());
    Ok(())
}
```