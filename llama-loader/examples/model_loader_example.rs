/// Example demonstrating the ModelLoader API
/// This example shows how to use ModelLoader with both HuggingFace and local models
use llama_loader::{CacheManager, ModelConfig, ModelSource, RetryConfig};
use std::path::PathBuf;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Note: This is a compilation example only, not a functional example
    // since we would need a real llama-cpp-2 backend initialized

    println!("ModelLoader API Example");

    // Create model configurations
    let hf_config = ModelConfig {
        source: ModelSource::HuggingFace {
            repo: "microsoft/DialoGPT-medium".to_string(),
            filename: Some("model.gguf".to_string()),
        },
        batch_size: 512,
        use_hf_params: true,
        retry_config: RetryConfig::default(),
        debug: false,
    };

    let local_config = ModelConfig {
        source: ModelSource::Local {
            folder: PathBuf::from("./models"),
            filename: Some("local-model.gguf".to_string()),
        },
        batch_size: 512,
        use_hf_params: false,
        retry_config: RetryConfig::default(),
        debug: false,
    };

    println!("HuggingFace config: {:?}", hf_config);
    println!("Local config: {:?}", local_config);

    // Validate configurations
    hf_config.validate()?;
    println!("HuggingFace config is valid!");

    // Local config validation will fail because the directory doesn't exist
    match local_config.validate() {
        Ok(_) => println!("Local config is valid!"),
        Err(e) => println!("Local config validation failed as expected: {}", e),
    }

    // Example of creating cache manager with custom settings
    let cache_manager = CacheManager::with_default_cache_dir()?
        .with_max_size_gb(25) // 25GB cache limit
        .with_unlimited_size(); // Actually, no limit

    println!("Cache manager created successfully");

    // Example of cache key generation
    let file_metadata = llama_loader::FileMetadata {
        size_bytes: 1024 * 1024 * 1024, // 1GB
        modified_time: 1234567890,
    };

    let cache_key = CacheManager::generate_cache_key(
        "microsoft/DialoGPT-medium",
        "model.gguf",
        &file_metadata,
    );

    println!("Generated cache key: {}", cache_key);

    // Example of retry configuration
    let retry_config = RetryConfig {
        max_retries: 5,
        initial_delay_ms: 500,
        backoff_multiplier: 1.5,
        max_delay_ms: 15000,
    };

    println!("Custom retry config: {:?}", retry_config);

    println!("ModelLoader API example completed successfully!");
    Ok(())
}