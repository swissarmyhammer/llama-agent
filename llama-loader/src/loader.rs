use crate::error::ModelError;
use crate::types::{LoadedModel, ModelSource, RetryConfig};
use llama_cpp_2::llama_backend::LlamaBackend;
use std::sync::Arc;

/// Configuration for the ModelLoader
#[derive(Debug, Clone)]
pub struct ModelConfig {
    /// The source from which to load the model
    pub source: ModelSource,
    /// Batch size for model operations
    pub batch_size: u32,
    /// Whether to use HuggingFace parameters
    pub use_hf_params: bool,
    /// Configuration for retry logic
    pub retry_config: RetryConfig,
    /// Enable debug output
    pub debug: bool,
}

impl Default for ModelConfig {
    fn default() -> Self {
        Self {
            source: ModelSource::HuggingFace {
                repo: "microsoft/DialoGPT-medium".to_string(),
                filename: None,
            },
            batch_size: 512,
            use_hf_params: true,
            retry_config: RetryConfig::default(),
            debug: false,
        }
    }
}

impl ModelConfig {
    /// Validate the model configuration
    pub fn validate(&self) -> Result<(), ModelError> {
        self.source.validate()?;

        if self.batch_size == 0 {
            return Err(ModelError::InvalidConfig(
                "Batch size must be greater than 0".to_string(),
            ));
        }

        if self.batch_size > 8192 {
            return Err(ModelError::InvalidConfig(
                "Batch size should not exceed 8192 for most models".to_string(),
            ));
        }

        Ok(())
    }
}

/// Manages loading of LLAMA models from various sources
pub struct ModelLoader {
    backend: Arc<LlamaBackend>,
}

impl ModelLoader {
    /// Create a new ModelLoader with the given backend
    pub fn new(backend: Arc<LlamaBackend>) -> Self {
        Self { backend }
    }

    /// Load a model according to the provided configuration
    ///
    /// This is a placeholder implementation that will be expanded in future issues
    /// to include the actual HuggingFace loading logic extracted from llama-agent.
    pub async fn load_model(&self, _config: &ModelConfig) -> Result<LoadedModel, ModelError> {
        // Placeholder implementation - will be implemented in future issues
        Err(ModelError::LoadingFailed(
            "ModelLoader::load_model not yet implemented - this is a placeholder".to_string(),
        ))
    }

    /// Load a model from HuggingFace
    ///
    /// This is a placeholder that will contain the extracted HuggingFace loading logic.
    pub async fn load_huggingface_model(
        &self,
        _repo: &str,
        _filename: Option<&str>,
    ) -> Result<LoadedModel, ModelError> {
        // Placeholder implementation - will be implemented in future issues
        Err(ModelError::LoadingFailed(
            "ModelLoader::load_huggingface_model not yet implemented - this is a placeholder"
                .to_string(),
        ))
    }

    /// Load a model from local filesystem
    ///
    /// This is a placeholder that will contain local model loading logic.
    pub async fn load_local_model(
        &self,
        _folder: &std::path::Path,
        _filename: Option<&str>,
    ) -> Result<LoadedModel, ModelError> {
        // Placeholder implementation - will be implemented in future issues
        Err(ModelError::LoadingFailed(
            "ModelLoader::load_local_model not yet implemented - this is a placeholder".to_string(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_model_config_default() {
        let config = ModelConfig::default();
        assert_eq!(config.batch_size, 512);
        assert!(config.use_hf_params);
        assert!(!config.debug);

        match config.source {
            ModelSource::HuggingFace { ref repo, .. } => {
                assert_eq!(repo, "microsoft/DialoGPT-medium")
            }
            _ => panic!("Wrong default model source"),
        }
    }

    #[test]
    fn test_model_config_validation() {
        // Valid config should pass
        let config = ModelConfig {
            source: ModelSource::HuggingFace {
                repo: "microsoft/DialoGPT-medium".to_string(),
                filename: Some("model.gguf".to_string()),
            },
            batch_size: 512,
            use_hf_params: true,
            retry_config: RetryConfig::default(),
            debug: false,
        };
        assert!(config.validate().is_ok());

        // Zero batch size should fail
        let mut config = ModelConfig::default();
        config.batch_size = 0;
        assert!(config.validate().is_err());

        // Very large batch size should fail
        let mut config = ModelConfig::default();
        config.batch_size = 10000;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_model_config_with_local_source() {
        let temp_dir = std::env::temp_dir();
        let config = ModelConfig {
            source: ModelSource::Local {
                folder: temp_dir,
                filename: None,
            },
            batch_size: 256,
            use_hf_params: false,
            retry_config: RetryConfig::default(),
            debug: true,
        };

        assert!(config.validate().is_ok());
        assert_eq!(config.batch_size, 256);
        assert!(!config.use_hf_params);
        assert!(config.debug);
    }

    // Note: We can't test ModelLoader methods without a real LlamaBackend
    // These will be tested in integration tests once the implementation is complete
}
