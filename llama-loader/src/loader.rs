use crate::cache::{CacheManager, FileMetadata};
use crate::error::ModelError;
use crate::huggingface::load_huggingface_model_with_path;
use crate::types::{LoadedModel, ModelConfig, ModelMetadata, ModelSource, RetryConfig};
use llama_cpp_2::{
    llama_backend::LlamaBackend,
    model::{params::LlamaModelParams, LlamaModel},
};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;
use tracing::{info, debug};

/// Manages loading of LLAMA models from various sources with caching support
pub struct ModelLoader {
    backend: Arc<LlamaBackend>,
    cache_manager: CacheManager,
    retry_config: RetryConfig,
}

impl ModelLoader {
    /// Create a new ModelLoader with the given backend and default cache manager
    pub fn new(backend: Arc<LlamaBackend>) -> Result<Self, ModelError> {
        let cache_manager = CacheManager::with_default_cache_dir()?;
        // Initialize the cache manager in a blocking context if needed
        Ok(Self {
            backend,
            cache_manager,
            retry_config: RetryConfig::default(),
        })
    }

    /// Create a new ModelLoader with custom cache manager and retry config
    pub fn new_with_config(
        backend: Arc<LlamaBackend>,
        cache_manager: CacheManager,
        retry_config: RetryConfig,
    ) -> Self {
        Self {
            backend,
            cache_manager,
            retry_config,
        }
    }

    /// Initialize the ModelLoader (must be called in an async context)
    pub async fn initialize(&mut self) -> Result<(), ModelError> {
        self.cache_manager.initialize().await
    }

    /// Load a model from the specified configuration with cache support
    pub async fn load_model(&mut self, config: &ModelConfig) -> Result<LoadedModel, ModelError> {
        config.validate()?;
        
        let _start_time = Instant::now();
        info!("Loading model from config: {:?}", config.source);

        match &config.source {
            ModelSource::HuggingFace { repo, filename } => {
                self.load_model_with_cache(repo, filename.as_deref(), &config.retry_config)
                    .await
            }
            ModelSource::Local { folder, filename } => {
                self.load_local_model(folder, filename.as_deref()).await
            }
        }
    }

    /// Load model with cache integration for HuggingFace models
    async fn load_model_with_cache(
        &mut self,
        repo: &str,
        filename: Option<&str>,
        retry_config: &RetryConfig,
    ) -> Result<LoadedModel, ModelError> {
        // Try to load from HuggingFace first to get the actual file path and metadata
        let start_time = Instant::now();
        debug!("Loading HuggingFace model with cache support: {}", repo);

        // Load from HuggingFace (this handles download and multi-part logic)
        let (model_path, actual_filename) = 
            self.load_hf_model_to_path(repo, filename, retry_config).await?;

        // Get file metadata for cache key generation
        let file_metadata = FileMetadata::from_path(&model_path).await?;
        let cache_key = CacheManager::generate_cache_key(repo, &actual_filename, &file_metadata);

        // Check if we already have this model in cache
        let cached_path = self.cache_manager.get_cached_model(&cache_key).await;
        
        let (final_path, cache_hit) = if let Some(cached) = cached_path {
            info!("Using cached model: {}", cached.display());
            (cached, true)
        } else {
            // Cache the newly downloaded model
            debug!("Caching model: {}", model_path.display());
            self.cache_manager.cache_model(&model_path, &cache_key).await?;
            (model_path, false)
        };

        // Load the model using llama-cpp-2
        let model_params = LlamaModelParams::default();
        let model = LlamaModel::load_from_file(&self.backend, &final_path, &model_params)
            .map_err(|e| {
                ModelError::LoadingFailed(format!(
                    "Failed to load model from {}: {}",
                    final_path.display(),
                    e
                ))
            })?;

        let load_time = start_time.elapsed();
        let metadata = ModelMetadata {
            source: ModelSource::HuggingFace {
                repo: repo.to_string(),
                filename: Some(actual_filename.clone()),
            },
            filename: actual_filename,
            size_bytes: file_metadata.size_bytes,
            load_time,
            cache_hit,
        };

        Ok(LoadedModel {
            model,
            path: final_path,
            metadata,
        })
    }

    /// Helper method to load HuggingFace model and return the downloaded path
    async fn load_hf_model_to_path(
        &self,
        repo: &str,
        filename: Option<&str>,
        retry_config: &RetryConfig,
    ) -> Result<(PathBuf, String), ModelError> {
        // Use the new function that returns both path and filename
        load_huggingface_model_with_path(repo, filename, retry_config).await
    }

    /// Load a model from HuggingFace (deprecated - use load_model with ModelConfig instead)
    pub async fn load_huggingface_model(
        &mut self,
        repo: &str,
        filename: Option<&str>,
        retry_config: &RetryConfig,
    ) -> Result<LoadedModel, ModelError> {
        // Use the provided retry_config, falling back to the struct's default
        self.load_model_with_cache(repo, filename, retry_config).await
    }

    /// Load a model from HuggingFace using the loader's default retry config
    pub async fn load_huggingface_model_with_defaults(
        &mut self,
        repo: &str,
        filename: Option<&str>,
    ) -> Result<LoadedModel, ModelError> {
        // Clone the retry config to avoid borrow conflicts
        let retry_config = self.retry_config.clone();
        self.load_model_with_cache(repo, filename, &retry_config).await
    }

    /// Load a model from local filesystem
    pub async fn load_local_model(
        &self,
        folder: &Path,
        filename: Option<&str>,
    ) -> Result<LoadedModel, ModelError> {
        let start_time = Instant::now();
        info!("Loading local model from folder: {:?}", folder);

        let model_path = if let Some(filename) = filename {
            let path = folder.join(filename);
            if !path.exists() {
                return Err(ModelError::NotFound(format!(
                    "Model file does not exist: {}",
                    path.display()
                )));
            }
            path
        } else {
            // Auto-detect with BF16 preference
            self.auto_detect_model_file(folder).await?
        };

        info!("Loading model from path: {:?}", model_path);

        // Get file metadata for proper size tracking
        let file_metadata = tokio::fs::metadata(&model_path).await?;
        let size_bytes = file_metadata.len();

        let model_params = LlamaModelParams::default();
        let model =
            LlamaModel::load_from_file(&self.backend, &model_path, &model_params).map_err(|e| {
                ModelError::LoadingFailed(format!(
                    "Failed to load model from {}: {}",
                    model_path.display(),
                    e
                ))
            })?;

        let load_time = start_time.elapsed();
        let filename_str = model_path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();

        let metadata = ModelMetadata {
            source: ModelSource::Local {
                folder: folder.to_path_buf(),
                filename: Some(filename_str.clone()),
            },
            filename: filename_str,
            size_bytes,
            load_time,
            cache_hit: false, // Local models are not cached
        };

        Ok(LoadedModel {
            model,
            path: model_path,
            metadata,
        })
    }

    /// Auto-detect model file in local directory with BF16 preference
    async fn auto_detect_model_file(&self, folder: &Path) -> Result<PathBuf, ModelError> {
        let mut gguf_files = Vec::new();
        let mut bf16_files = Vec::new();

        // Read directory
        let mut entries = match tokio::fs::read_dir(folder).await {
            Ok(entries) => entries,
            Err(e) => {
                return Err(ModelError::LoadingFailed(format!(
                    "Cannot read directory {}: {}",
                    folder.display(),
                    e
                )))
            }
        };

        while let Some(entry) = entries
            .next_entry()
            .await
            .map_err(|e| ModelError::LoadingFailed(e.to_string()))?
        {
            let path = entry.path();
            if let Some(extension) = path.extension() {
                if extension == "gguf" {
                    let filename = path.file_name().unwrap().to_string_lossy().to_lowercase();
                    if filename.contains("bf16") {
                        bf16_files.push(path);
                    } else {
                        gguf_files.push(path);
                    }
                }
            }
        }

        // Prioritize BF16 files
        if !bf16_files.is_empty() {
            info!("Found BF16 model file: {:?}", bf16_files[0]);
            return Ok(bf16_files[0].clone());
        }

        // Fallback to first GGUF file
        if !gguf_files.is_empty() {
            info!("Found GGUF model file: {:?}", gguf_files[0]);
            return Ok(gguf_files[0].clone());
        }

        Err(ModelError::NotFound(format!(
            "No .gguf model files found in {}",
            folder.display()
        )))
    }
}

#[cfg(test)]
mod tests {

    // Note: Integration tests would go here for ModelLoader methods
    // These require a real LlamaBackend and are better suited for integration tests

    #[test]
    fn test_model_loader_creation() {
        // We can't create a real LlamaBackend in unit tests
        // This test just verifies the structure compiles correctly
        // If this test runs, the struct definition is valid
    }
}
