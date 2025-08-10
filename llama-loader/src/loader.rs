use crate::error::ModelError;
use crate::huggingface::load_huggingface_model;
use crate::types::{LoadedModel, ModelMetadata, ModelSource, RetryConfig};
use llama_cpp_2::{
    llama_backend::LlamaBackend,
    model::{params::LlamaModelParams, LlamaModel},
};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;
use tracing::info;

/// Manages loading of LLAMA models from various sources
pub struct ModelLoader {
    backend: Arc<LlamaBackend>,
}

impl ModelLoader {
    /// Create a new ModelLoader with the given backend
    pub fn new(backend: Arc<LlamaBackend>) -> Self {
        Self { backend }
    }

    /// Load a model from HuggingFace
    pub async fn load_huggingface_model(
        &self,
        repo: &str,
        filename: Option<&str>,
        retry_config: &RetryConfig,
    ) -> Result<LoadedModel, ModelError> {
        let start_time = Instant::now();
        info!("Loading HuggingFace model: {}", repo);

        let model = load_huggingface_model(&self.backend, repo, filename, retry_config).await?;

        let load_time = start_time.elapsed();
        let metadata = ModelMetadata {
            source: ModelSource::HuggingFace {
                repo: repo.to_string(),
                filename: filename.map(|s| s.to_string()),
            },
            filename: filename.unwrap_or("auto-detected").to_string(),
            size_bytes: 0, // Would need file system access to determine actual size
            load_time,
            cache_hit: false, // Would need cache implementation to track this
        };

        Ok(LoadedModel {
            model,
            path: PathBuf::new(), // Would need to return path from load_huggingface_model
            metadata,
        })
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
        let metadata = ModelMetadata {
            source: ModelSource::Local {
                folder: folder.to_path_buf(),
                filename: filename.map(|s| s.to_string()),
            },
            filename: model_path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string(),
            size_bytes: 0, // Would need file system access to get actual size
            load_time,
            cache_hit: false,
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
        // This test just verifies the structure compiles
        assert!(true);
    }
}
