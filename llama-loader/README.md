# llama-loader

Shared model loading library for LLaMA models with caching support.

## Features
- HuggingFace model downloading with retry logic
- Multi-part model support
- Intelligent caching with LRU eviction
- Platform-appropriate cache directories
- Thread-safe concurrent access
- MD5-based integrity verification
- Automatic retry on network failures

## Usage

```rust
use llama_loader::{ModelLoader, ModelConfig, ModelSource};

let loader = ModelLoader::new(backend, cache_manager);
let config = ModelConfig {
    source: ModelSource::HuggingFace { 
        repo: "Qwen/Qwen2.5-7B-Instruct-GGUF".to_string(),
        filename: None 
    },
};

let loaded_model = loader.load_model(&config).await?;
```

### Local Model Loading
```rust
let config = ModelConfig {
    source: ModelSource::Local {
        path: PathBuf::from("/path/to/model.gguf"),
    },
};
```

### Cache Management
```rust
use llama_loader::CacheManager;

let cache_manager = CacheManager::new(cache_dir, max_size_bytes);
let cache_stats = cache_manager.get_stats().await?;
println!("Cache usage: {} / {}", cache_stats.current_size, cache_stats.max_size);
```

## Configuration

### Cache Directory
The cache directory is automatically determined based on the platform:
- Linux/macOS: `~/.cache/llama-loader/`
- Windows: `%LOCALAPPDATA%/llama-loader/cache/`

Override with the `LLAMA_CACHE_DIR` environment variable:
```bash
export LLAMA_CACHE_DIR=/custom/cache/path
```

### Cache Size
Default cache size is 50GB. Configure via environment variable:
```bash
export LLAMA_CACHE_MAX_SIZE=107374182400  # 100GB in bytes
```

## Architecture

- **ModelLoader**: Main interface for loading models
- **CacheManager**: Handles model caching with LRU eviction
- **HuggingFaceLoader**: Downloads models from HuggingFace Hub
- **ModelSource**: Enum for different model sources (local/remote)

## Error Handling

The crate provides detailed error types:
- `LoaderError::NetworkError`: Network connectivity issues
- `LoaderError::CacheError`: Cache management problems  
- `LoaderError::ValidationError`: Model integrity failures
- `LoaderError::ConfigError`: Configuration problems

## Performance Tips

- Use a fast SSD for the cache directory
- Ensure adequate disk space (models can be 4-20GB+)
- Configure cache size based on available disk space
- Models are automatically cached after first download