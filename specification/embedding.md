# Embedding Specification

## Overview

Create a new `llama-embedding` crate that provides batch text embedding functionality using the same llama-cpp-2 rust wrappers. Additionally, extend the existing `llama-agent-cli` with an `embed` command and rename it to `llama-cli` to provide a unified command-line interface with both `generate` (existing functionality) and `embed` (new functionality) commands.

The new `llama-embedding` crate will be a library crate that handles the core embedding functionality, while the `embed` command in `llama-cli` will provide a command-line interface to this functionality, outputting results as Apache Parquet files containing text, MD5 hashes, and embedding vectors.

Additionally, extract HuggingFace model loading logic into a shared `llama-loader` crate to enable code reuse between `llama-agent`, `llama-embedding`, and the CLI while preserving all existing functionality including retry logic, multi-part downloads, and caching.

## Architecture

### Core Components

1. **llama-loader** (new shared crate) - HuggingFace model loading, caching, and management
2. **llama-embedding** (new library crate) - Core embedding functionality and batch processing
3. **llama-cli** (renamed from llama-agent-cli) - Unified CLI with `generate` and `embed` commands
4. **ParquetOutput** - Manages Apache Parquet file output (no console output of embeddings)

## Technical Design

### 1. Shared Model Loading (llama-loader crate)

Extract all HuggingFace model loading logic into a shared crate:

```rust
// llama-loader/src/lib.rs
pub struct ModelLoader {
    backend: Arc<LlamaBackend>,
    cache_manager: CacheManager,
}

pub struct LoadedModel {
    pub model: LlamaModel,
    pub path: PathBuf,
    pub metadata: ModelMetadata,
}

pub struct ModelMetadata {
    pub source: ModelSource,
    pub filename: String,
    pub size_bytes: u64,
    pub load_time: Duration,
    pub cache_hit: bool,
}

impl ModelLoader {
    pub async fn load_model(&self, config: &ModelConfig) -> Result<LoadedModel, ModelError>;
    pub async fn load_huggingface_model(&self, repo: &str, filename: Option<&str>) -> Result<LoadedModel, ModelError>;
    pub async fn load_local_model(&self, folder: &Path, filename: Option<&str>) -> Result<LoadedModel, ModelError>;
}
```

**Preserved Functionality:**
- All retry logic with exponential backoff
- Multi-part model downloading
- Auto-detection of BF16 preference
- File validation and error handling
- Progress indication and logging
- Memory usage tracking

**Cache Management:**
```rust
pub struct CacheManager {
    cache_dir: PathBuf,
    max_cache_size_gb: Option<u64>,
}

impl CacheManager {
    pub fn new(cache_dir: PathBuf) -> Self;
    pub async fn get_cached_model(&self, cache_key: &str) -> Option<PathBuf>;
    pub async fn cache_model(&self, model_path: &Path, cache_key: &str) -> Result<(), CacheError>;
    pub async fn cleanup_old_models(&self) -> Result<(), CacheError>;
    pub fn generate_cache_key(repo: &str, filename: &str) -> String;
}
```

### 2. Unified CLI Structure (llama-cli)

Rename and extend the existing CLI with subcommands:

```rust
// llama-cli/src/main.rs
#[derive(Parser)]
#[command(name = "llama-cli")]
#[command(about = "Unified Llama CLI for generation and embeddings")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Generate text using a language model (existing functionality)
    Generate(GenerateArgs),
    /// Generate embeddings for input texts
    Embed(EmbedArgs),
}

#[derive(Args)]
pub struct GenerateArgs {
    // Existing llama-agent-cli arguments
    #[arg(long, short)]
    model: String,
    
    #[arg(long)]
    filename: Option<String>,
    
    #[arg(long, short)]
    prompt: String,
    
    // ... other existing args
}

#[derive(Args)]
pub struct EmbedArgs {
    /// Model source (HuggingFace repo or local path)
    #[arg(long, short)]
    model: String,
    
    /// Optional model filename
    #[arg(long)]
    filename: Option<String>,
    
    /// Input text file (one text per line)
    #[arg(long, short)]
    input: PathBuf,
    
    /// Output Parquet file path
    #[arg(long, short)]
    output: PathBuf,
    
    /// Batch size for processing
    #[arg(long, default_value = "32")]
    batch_size: usize,
    
    /// Normalize embeddings
    #[arg(long)]
    normalize: bool,
    
    /// Maximum sequence length
    #[arg(long)]
    max_length: Option<usize>,
    
    /// Enable debug output
    #[arg(long)]
    debug: bool,
}
```

**CLI Usage Examples:**
```bash
# Existing generation functionality (unchanged)
llama-cli generate --model Qwen/Qwen2.5-7B-Instruct-GGUF --prompt "Hello world"

# New embedding functionality
llama-cli embed --model Qwen/Qwen3-Embedding-0.6B-GGUF --input texts.txt --output embeddings.parquet

# With additional options
llama-cli embed \
  --model Qwen/Qwen3-Embedding-0.6B-GGUF \
  --input large_corpus.txt \
  --output embeddings.parquet \
  --batch-size 64 \
  --normalize \
  --max-length 512
```

### 3. Embedding Library (llama-embedding crate)

```rust
// llama-embedding/src/lib.rs
pub struct EmbeddingModel {
    loader: Arc<ModelLoader>,
    model: Option<LlamaModel>,
    config: EmbeddingConfig,
    metadata: Option<ModelMetadata>,
}

pub struct EmbeddingConfig {
    pub model_source: ModelSource,
    pub batch_size: usize,
    pub normalize_embeddings: bool,
    pub max_sequence_length: Option<usize>,
    pub debug: bool,
}

pub struct EmbeddingResult {
    pub text: String,
    pub text_hash: String,  // MD5 of text
    pub embedding: Vec<f32>,
    pub sequence_length: usize,
    pub processing_time_ms: u64,
}

pub struct BatchProcessor {
    model: Arc<EmbeddingModel>,
    batch_size: usize,
}

impl EmbeddingModel {
    pub async fn new(config: EmbeddingConfig) -> Result<Self, EmbeddingError>;
    pub async fn load_model(&mut self) -> Result<(), EmbeddingError>;
    pub async fn embed_text(&self, text: &str) -> Result<EmbeddingResult, EmbeddingError>;
    pub fn get_embedding_dimension(&self) -> Option<usize>;
}

impl BatchProcessor {
    pub fn new(model: Arc<EmbeddingModel>, batch_size: usize) -> Self;
    pub async fn process_batch(&mut self, texts: &[String]) -> Result<Vec<EmbeddingResult>, EmbeddingError>;
    pub async fn process_file(&mut self, input_path: &Path) -> Result<impl Iterator<Item = EmbeddingResult>, EmbeddingError>;
}
```

**Key Features of llama-embedding crate:**
- **Reusable library**: Can be used by CLI or other applications
- **Batch processing**: Efficient handling of multiple texts
- **Streaming support**: Process large files without loading everything into memory
- **Model management**: Loading and caching via llama-loader
- **Flexible output**: Returns structured data that can be written to any format

**Processing Flow:**
1. Read input file line by line (streaming for large files)
2. Process texts in configurable batch sizes
3. Generate embeddings using llama-cpp-2
4. Generate MD5 hash for each text
5. Write results directly to Parquet file (no console output)
6. Show progress and summary statistics only

### 4. CLI Integration (llama-cli)

The CLI acts as a thin wrapper around the llama-embedding crate:

```rust
// llama-cli/src/embed.rs - CLI command implementation
use llama_embedding::{EmbeddingModel, EmbeddingConfig, BatchProcessor};

pub async fn run_embed_command(args: EmbedArgs) -> Result<(), Box<dyn std::error::Error>> {
    // Create embedding config from CLI args
    let config = EmbeddingConfig {
        model_source: ModelSource::from_string(&args.model, args.filename),
        batch_size: args.batch_size,
        normalize_embeddings: args.normalize,
        max_sequence_length: args.max_length,
        debug: args.debug,
    };
    
    // Initialize embedding model
    let mut embedding_model = EmbeddingModel::new(config).await?;
    embedding_model.load_model().await?;
    
    let embedding_dim = embedding_model.get_embedding_dimension()
        .ok_or("Could not determine embedding dimensions")?;
    
    // Set up batch processor and Parquet writer
    let model = Arc::new(embedding_model);
    let mut processor = BatchProcessor::new(model.clone(), args.batch_size);
    let mut parquet_writer = ParquetWriter::new(&args.output, embedding_dim, args.batch_size)?;
    
    // Process file and write to Parquet
    let results = processor.process_file(&args.input).await?;
    for batch in results.chunks(args.batch_size) {
        parquet_writer.write_batch(batch.to_vec())?;
    }
    
    parquet_writer.close()?;
    println!("Embeddings written to: {}", args.output.display());
    Ok(())
}
```

### 5. Parquet Output (within llama-cli)

```rust
// llama-cli/src/parquet_writer.rs
use llama_embedding::EmbeddingResult;

pub struct ParquetWriter {
    schema: Schema,
    writer: ArrowWriter<File>,
    batch_buffer: Vec<EmbeddingResult>,
    batch_size: usize,
}

// Parquet Schema:
// - text: Utf8
// - text_hash: Utf8 (MD5)
// - embedding: FixedSizeList<Float32>
// - sequence_length: UInt32
// - processing_time_ms: UInt64

impl ParquetWriter {
    pub fn new(output_path: &Path, embedding_dim: usize, batch_size: usize) -> Result<Self, ParquetError>;
    pub fn write_batch(&mut self, results: Vec<EmbeddingResult>) -> Result<(), ParquetError>;
    pub fn flush(&mut self) -> Result<(), ParquetError>;
    pub fn close(self) -> Result<ParquetMetadata, ParquetError>;
}
```

**Output Characteristics:**
- **Single output format**: Apache Parquet only (no console output of embedding vectors)
- **Streaming writes**: Results written incrementally to avoid memory issues
- **Compressed**: Parquet compression for efficient storage
- **Metadata included**: File includes schema and statistics
- **Console feedback**: Only progress bars and summary statistics shown to user

### 6. Console Output Examples

**Generate Command (existing, unchanged):**
```bash
$ llama-cli generate --model Qwen/Qwen2.5-7B-Instruct-GGUF --prompt "Hello world"
Loading model: Qwen/Qwen2.5-7B-Instruct-GGUF
Model loaded successfully in 2.3s
Generating...

Hello world! How can I assist you today?

Generation completed in 850ms
```

**Embed Command (new):**
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

**Key Console Behavior:**
- **No embedding vectors printed**: Only metadata and progress shown
- **Progress indication**: Real-time progress bars for large files
- **Summary statistics**: Processing time, file size, counts
- **Error messages**: Clear error reporting for failures

## Integration Testing

### Test Model: Qwen/Qwen3-Embedding-0.6B-GGUF

**Test Configuration:**
- Model: `Qwen/Qwen3-Embedding-0.6B-GGUF`
- Input: Various text samples of different lengths
- Batch sizes: 1, 8, 32, 64
- Output formats: All supported formats

**Test Cases:**

1. **Basic Functionality**
   ```bash
   llama-cli embed \
     --model Qwen/Qwen3-Embedding-0.6B-GGUF \
     --input test_texts.txt \
     --output embeddings.parquet \
     --batch-size 32
   ```

2. **Local Model Loading**
   ```bash
   llama-cli embed \
     --model ./models/qwen3-embedding \
     --filename model.gguf \
     --input test_texts.txt \
     --output embeddings.parquet
   ```

3. **Large File Processing**
   ```bash
   llama-cli embed \
     --model Qwen/Qwen3-Embedding-0.6B-GGUF \
     --input large_corpus.txt \
     --output embeddings.parquet \
     --batch-size 64 \
     --normalize \
     --debug
   ```

4. **Integration with Generation**
   ```bash
   # Verify both commands work in same CLI
   llama-cli generate --model Qwen/Qwen2.5-7B-Instruct-GGUF --prompt "Test"
   llama-cli embed --model Qwen/Qwen3-Embedding-0.6B-GGUF --input texts.txt --output embeddings.parquet
   ```

### Test Data Structure

**Input (`test_texts.txt`):**
```
Hello world, this is a test sentence.
The quick brown fox jumps over the lazy dog.
Artificial intelligence is transforming our world.
短い日本語のテスト文です。
This is a much longer text that will test how the embedding model handles sequences of varying lengths and complexity, including punctuation, numbers like 123, and mixed content.
```

**Expected Output Schema:**
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

## Workspace Structure Changes

### Updated Workspace Members
```toml
[workspace]
members = ["llama-agent", "llama-cli", "llama-loader", "llama-embedding"]
resolver = "2"
```

### New Dependencies for workspace:
```toml
[workspace.dependencies]
# Existing dependencies remain...

# Apache Arrow (new)
arrow = "53.0"
arrow-array = "53.0"
arrow-schema = "53.0"
parquet = "53.0"

# Hashing (new)
md5 = "0.7"

# Internal workspace crates
llama-loader = { path = "llama-loader" }
```

## Crate Structure

### llama-loader/ (new shared crate)
```
llama-loader/
├── Cargo.toml
├── src/
│   ├── lib.rs              # Public API and re-exports
│   ├── loader.rs           # ModelLoader implementation
│   ├── cache.rs            # CacheManager implementation
│   ├── huggingface.rs      # HF-specific loading logic (extracted from model.rs)
│   ├── local.rs            # Local model loading
│   ├── retry.rs            # Retry and backoff logic
│   ├── multipart.rs        # Multi-part download handling
│   ├── detection.rs        # Auto-detection logic (BF16, GGUF)
│   └── error.rs            # Shared error types
├── tests/
│   ├── integration_test.rs # Loading tests
│   └── cache_test.rs       # Cache functionality tests
└── examples/
    └── load_model.rs       # Usage examples

# Key extracted functionality from llama-agent/src/model.rs:
# - load_huggingface_model()
# - download_model_file_with_retry()  
# - download_multi_part_model()
# - is_retriable_error()
# - format_download_error() 
# - auto_detect_hf_model_file()
# - detect_multi_part_base()
# - get_all_parts()
# - All retry and backoff logic
```

### llama-embedding/ (new library crate)
```
llama-embedding/
├── Cargo.toml
├── src/
│   ├── lib.rs              # Public API and main types
│   ├── model.rs            # EmbeddingModel implementation
│   ├── batch.rs            # BatchProcessor implementation
│   ├── config.rs           # Configuration types
│   └── error.rs            # Error types
├── tests/
│   ├── integration_test.rs # Integration tests with Qwen model
│   └── batch_test.rs       # Batch processing tests
└── examples/
    └── basic_embedding.rs  # Usage examples

# Key features:
# - Core embedding functionality as reusable library
# - Batch processing with configurable sizes
# - Integration with llama-loader for model management
# - Streaming support for large datasets
# - No output format dependencies (returns structured data)
```

### Updated llama-cli/ (renamed from llama-agent-cli)
```
llama-cli/
├── Cargo.toml              # Updated name, depends on llama-loader, llama-agent, and llama-embedding
├── src/
│   ├── main.rs             # Main CLI with subcommands (generate, embed)
│   ├── lib.rs              # Shared CLI utilities
│   ├── generate.rs         # Generate command implementation (existing logic)
│   ├── embed.rs            # New embed command implementation (thin wrapper)
│   ├── parquet_writer.rs   # Parquet output handling
│   └── progress.rs         # Progress bar utilities
├── tests/
│   ├── integration_test.rs # Integration tests for both commands
│   ├── embed_test.rs       # Embedding-specific tests
│   └── common/
│       └── mod.rs          # Test utilities
└── examples/
    └── usage_examples.rs   # CLI usage examples

# Key changes:
# - Renamed from llama-agent-cli to llama-cli
# - Added embed.rs as thin wrapper around llama-embedding crate
# - Added parquet_writer.rs for Parquet output
# - Updated Cargo.toml to depend on llama-loader, llama-agent, and llama-embedding
```

### Updated llama-agent/
```
llama-agent/
├── Cargo.toml              # Updated to depend on llama-loader
├── src/
│   ├── lib.rs
│   ├── model.rs            # Simplified - delegates to llama-loader
│   ├── agent.rs
│   ├── chat_template.rs
│   ├── queue.rs
│   ├── session.rs
│   ├── types.rs            # ModelSource moves to llama-loader
│   └── ...

# Key changes:
# - ModelManager updated to use llama-loader
# - Reduced model loading code (~500 lines moved to llama-loader)
# - Maintains same public API for backward compatibility
```

## Migration Plan

### Phase 1: Extract llama-loader
1. **Create llama-loader crate structure**
2. **Move ModelSource and related types** from `llama-agent/src/types.rs` to `llama-loader/src/lib.rs`
3. **Extract HuggingFace loading logic** from `llama-agent/src/model.rs`:
   - `load_huggingface_model()` → `llama-loader/src/huggingface.rs`
   - `download_model_file_with_retry()` → `llama-loader/src/retry.rs`
   - `download_multi_part_model()` → `llama-loader/src/multipart.rs`
   - Auto-detection functions → `llama-loader/src/detection.rs`
   - Error handling logic → `llama-loader/src/error.rs`

4. **Add caching layer** in `llama-loader/src/cache.rs`
5. **Update llama-agent** to use llama-loader:
   ```rust
   // llama-agent/Cargo.toml
   [dependencies]
   llama-loader = { workspace = true }
   
   // llama-agent/src/model.rs - simplified
   impl ModelManager {
       pub async fn load_model(&self) -> Result<(), ModelError> {
           let loaded_model = self.loader.load_model(&self.config).await?;
           // Store loaded_model.model in self.model
           Ok(())
       }
   }
   ```

6. **Run full test suite** to ensure no regressions

### Phase 2: Create llama-embedding crate
1. **Create llama-embedding crate structure**
   - Set up Cargo.toml with dependencies on llama-loader
   - Implement EmbeddingModel and BatchProcessor
   - Add configuration and error types
   - Create comprehensive test suite

2. **Integration with llama-loader**
   - Use ModelLoader for all model loading operations
   - Implement caching and retry logic integration
   - Test with Qwen/Qwen3-Embedding-0.6B-GGUF model

### Phase 3: Rename and extend CLI
1. **Rename llama-agent-cli to llama-cli**
   - Update Cargo.toml name and binary name
   - Update all references and documentation

2. **Restructure CLI with subcommands**
   - Move existing main.rs logic to generate.rs
   - Create new main.rs with clap subcommands
   - Add Commands enum with Generate and Embed variants

3. **Implement embed command**
   - Create embed.rs as thin wrapper around llama-embedding crate
   - Add parquet_writer.rs for output handling
   - Integrate progress reporting and user feedback

4. **Add integration tests**
   - Test both generate and embed commands
   - Test with Qwen embedding model
   - Test Parquet output verification
   - Test library usage separate from CLI

### Cache Configuration

**Default Cache Behavior:**
- Cache directory: `~/.cache/llama-loader/models/` (Linux/macOS) or `%LOCALAPPDATA%\llama-loader\models\` (Windows)
- Cache key format: `{repo_slug}_{filename}_{file_size}_{modified_time}`
- Max cache size: 50GB (configurable)
- Cache cleanup: LRU eviction when size exceeded

**Cache Integration:**
```rust
// All three crates (llama-agent, llama-embedding, llama-cli) benefit from shared cache
let cache_dir = dirs::cache_dir().unwrap_or_else(|| PathBuf::from(".cache"))
    .join("llama-loader").join("models");

let cache_manager = CacheManager::new(cache_dir)
    .with_max_size_gb(50)
    .with_cleanup_on_start(true);

let loader = ModelLoader::new(backend, cache_manager);
```

### Detailed API Migration

**Before (llama-agent/src/model.rs):**
```rust
impl ModelManager {
    async fn load_huggingface_model(&self, repo: &str, filename: Option<&str>) -> Result<LlamaModel, ModelError>
    async fn download_model_file_with_retry(&self, repo_api: &ApiRepo, filename: &str, repo: &str) -> Result<PathBuf, ModelError>
    // ... 500+ lines of loading logic
}
```

**After (llama-loader/src/lib.rs):**
```rust
pub struct ModelLoader {
    backend: Arc<LlamaBackend>,
    cache_manager: CacheManager,
    retry_config: RetryConfig,
}

impl ModelLoader {
    pub async fn load_model(&self, config: &ModelConfig) -> Result<LoadedModel, ModelError> {
        let cached_path = self.cache_manager.get_cached_model(&cache_key).await;
        
        let model_path = if let Some(path) = cached_path {
            path
        } else {
            match &config.source {
                ModelSource::HuggingFace { repo, filename } => {
                    let path = self.load_huggingface_model(repo, filename.as_deref()).await?;
                    self.cache_manager.cache_model(&path, &cache_key).await?;
                    path
                }
                ModelSource::Local { folder, filename } => {
                    self.load_local_model(folder, filename.as_deref()).await?
                }
            }
        };
        
        let model = LlamaModel::load_from_file(&self.backend, &model_path, &params)?;
        Ok(LoadedModel { model, path: model_path, metadata })
    }
}
```

**Updated llama-cli dependency structure:**
```rust
// llama-cli/Cargo.toml
[dependencies]
llama-loader = { workspace = true }
llama-agent = { workspace = true }     # For generate command
llama-embedding = { workspace = true } # For embed command
clap = { workspace = true, features = ["derive"] }
parquet = { workspace = true }
arrow = { workspace = true }
md5 = { workspace = true }
indicatif = "0.17"  # For progress bars

// llama-cli/src/main.rs
use llama_agent::Agent;  // For generate command
use llama_embedding::{EmbeddingModel, EmbeddingConfig, BatchProcessor}; // For embed command

async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    
    match cli.command {
        Commands::Generate(args) => {
            // Use existing llama-agent functionality
            let agent = Agent::new(/* config from args */)?;
            // ... existing generate logic
        }
        Commands::Embed(args) => {
            // Use new llama-embedding library
            let config = EmbeddingConfig::from_cli_args(args);
            let mut model = EmbeddingModel::new(config).await?;
            model.load_model().await?;
            
            let processor = BatchProcessor::new(Arc::new(model), args.batch_size);
            // Process and write to Parquet via parquet_writer
        }
    }
    
    Ok(())
}
```

**llama-embedding crate API:**
```rust
// llama-embedding/Cargo.toml
[dependencies]
llama-loader = { workspace = true }
llama-cpp-2 = { workspace = true }
tokio = { workspace = true }
md5 = { workspace = true }
# No Apache Arrow dependencies - returns structured data only

// llama-embedding/src/lib.rs
pub use crate::model::{EmbeddingModel, EmbeddingConfig};
pub use crate::batch::{BatchProcessor, EmbeddingResult};
pub use crate::error::EmbeddingError;

// Example usage by other crates:
use llama_embedding::{EmbeddingModel, EmbeddingConfig};

let config = EmbeddingConfig::new(model_source, batch_size);
let mut model = EmbeddingModel::new(config).await?;
model.load_model().await?;

let result = model.embed_text("Hello world").await?;
// result.embedding contains the vector, result.text_hash contains MD5, etc.
```

## Performance Considerations

1. **Memory Management**
   - Streaming processing for large text files
   - Configurable batch sizes to manage GPU/CPU memory
   - Efficient Arrow memory layout

2. **Model Loading**
   - Reuse existing model loading infrastructure
   - Support for GGUF quantized models
   - Auto-detection of embedding dimensions

3. **Parallel Processing**
   - Batch processing for efficiency
   - Configurable thread pools for CPU inference
   - GPU acceleration where available (Metal on macOS)

## Error Handling

Following existing patterns:

```rust
#[derive(thiserror::Error, Debug)]
pub enum EmbeddingError {
    #[error("Model error: {0}")]
    Model(#[from] ModelError),
    
    #[error("Batch processing error: {0}")]
    BatchProcessing(String),
    
    #[error("Arrow output error: {0}")]
    ArrowOutput(#[from] arrow::error::ArrowError),
    
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("Text encoding error: {0}")]
    TextEncoding(String),
}
```

## Success Criteria

1. **Functionality**
   - Successfully loads Qwen/Qwen3-Embedding-0.6B-GGUF model
   - Processes batches of text inputs
   - Generates correct MD5 hashes
   - Outputs valid Apache Arrow datasets

2. **Performance**
   - Processes 1000 texts in under 60 seconds
   - Memory usage scales predictably with batch size
   - Supports texts up to 2048 tokens

3. **Integration**
   - Works with existing model loading infrastructure
   - Compatible with HuggingFace model loading
   - Consistent error handling and logging patterns

4. **Testing**
   - Complete integration test suite for llama-embedding library
   - CLI integration tests for both generate and embed commands
   - Performance benchmarks for batch processing
   - Cross-platform compatibility (macOS, Linux)
   - Shared cache testing between llama-agent and llama-embedding
   - Library API testing separate from CLI functionality

**Integration Tests for llama-loader:**
```rust
#[tokio::test]
async fn test_cache_sharing_between_crates() {
    // Test that models downloaded by llama-agent are available to llama-embedding
    let cache_dir = tempfile::tempdir().unwrap();
    let loader = ModelLoader::new(backend, CacheManager::new(cache_dir.path()));
    
    // Load model via agent-style config
    let agent_config = ModelConfig { /* ... */ };
    let loaded_model = loader.load_model(&agent_config).await.unwrap();
    
    // Same model should be cached for embedding use
    let embedding_config = ModelConfig { /* same source */ };
    let start_time = Instant::now();
    let cached_model = loader.load_model(&embedding_config).await.unwrap();
    
    // Should load much faster from cache
    assert!(start_time.elapsed() < Duration::from_secs(5));
    assert!(cached_model.metadata.cache_hit);
}

#[tokio::test]  
async fn test_multipart_download_preservation() {
    // Ensure multi-part downloads work exactly as before
    let loader = ModelLoader::new(backend, cache_manager);
    
    // Test with a known multi-part model
    let config = ModelConfig {
        source: ModelSource::HuggingFace {
            repo: "microsoft/DialoGPT-large".to_string(),
            filename: None,
        },
        // ... other config
    };
    
    let result = loader.load_model(&config).await;
    // Should successfully handle multi-part detection and download
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_retry_logic_preservation() {
    // Test that all retry logic is preserved
    let mut retry_config = RetryConfig::default();
    retry_config.max_retries = 2;
    retry_config.initial_delay_ms = 100;
    
    let loader = ModelLoader::with_retry_config(backend, cache_manager, retry_config);
    
    // Test with intentionally failing scenario to verify retry behavior
    // (Implementation would use mock HTTP responses)
}
```

