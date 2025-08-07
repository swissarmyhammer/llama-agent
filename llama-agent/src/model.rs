use crate::types::{ModelConfig, ModelSource, ModelError};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

// Simplified mock model and context for testing and initial implementation
#[derive(Debug, Clone)]
pub struct MockModel {
    pub path: PathBuf,
    pub config: ModelConfig,
}

#[derive(Debug, Clone)]
pub struct MockContext {
    pub model_path: PathBuf,
    pub batch_size: u32,
}

pub struct ModelManager {
    model: Arc<RwLock<Option<MockModel>>>,
    context: Arc<RwLock<Option<MockContext>>>,
    config: ModelConfig,
}

impl ModelManager {
    pub fn new(config: ModelConfig) -> Self {
        Self {
            model: Arc::new(RwLock::new(None)),
            context: Arc::new(RwLock::new(None)),
            config,
        }
    }

    pub async fn load_model(&self) -> Result<(), ModelError> {
        info!("Loading model with configuration: {:?}", self.config);

        let model_path = self.resolve_model_path().await?;
        debug!("Resolved model path: {:?}", model_path);

        // Create mock model
        let model = MockModel {
            path: model_path.clone(),
            config: self.config.clone(),
        };

        info!("Model loaded successfully");

        // Create mock context
        let context = MockContext {
            model_path: model_path.clone(),
            batch_size: self.config.batch_size,
        };

        info!("Model context created successfully");

        // Store model and context
        {
            let mut model_lock = self.model.write().await;
            *model_lock = Some(model);
        }
        {
            let mut context_lock = self.context.write().await;
            *context_lock = Some(context);
        }

        Ok(())
    }

    pub async fn is_loaded(&self) -> bool {
        let model_lock = self.model.read().await;
        model_lock.is_some()
    }

    pub async fn get_model(&self) -> Option<Arc<MockModel>> {
        let model_lock = self.model.read().await;
        model_lock.clone().map(Arc::new)
    }

    pub async fn get_context(&self) -> Option<Arc<MockContext>> {
        let context_lock = self.context.read().await;
        context_lock.clone().map(Arc::new)
    }

    async fn resolve_model_path(&self) -> Result<PathBuf, ModelError> {
        match &self.config.source {
            ModelSource::HuggingFace { repo, filename } => {
                self.resolve_huggingface_path(repo, filename.as_deref()).await
            }
            ModelSource::Local { folder, filename } => {
                self.resolve_local_path(folder, filename.as_deref()).await
            }
        }
    }

    async fn resolve_huggingface_path(&self, repo: &str, filename: Option<&str>) -> Result<PathBuf, ModelError> {
        // For now, we'll simulate HuggingFace model resolution
        // In a real implementation, this would use llama-cpp-2's HF integration
        warn!("HuggingFace model loading not yet implemented, treating as local path");
        
        // Treat repo as a local path for now
        let repo_path = PathBuf::from(repo);
        self.resolve_local_path(&repo_path, filename).await
    }

    async fn resolve_local_path(&self, folder: &Path, filename: Option<&str>) -> Result<PathBuf, ModelError> {
        if !folder.exists() {
            return Err(ModelError::NotFound(format!("Folder does not exist: {}", folder.display())));
        }

        match filename {
            Some(file) => {
                let model_path = folder.join(file);
                if !model_path.exists() {
                    return Err(ModelError::NotFound(format!("Model file does not exist: {}", model_path.display())));
                }
                if !model_path.extension().map_or(false, |ext| ext == "gguf") {
                    return Err(ModelError::InvalidConfig(format!("Model file must have .gguf extension: {}", model_path.display())));
                }
                Ok(model_path)
            }
            None => {
                // Auto-detect model file with BF16 preference
                self.auto_detect_model_file(folder).await
            }
        }
    }

    async fn auto_detect_model_file(&self, folder: &Path) -> Result<PathBuf, ModelError> {
        let mut gguf_files = Vec::new();
        let mut bf16_files = Vec::new();

        // Read directory
        let mut entries = match tokio::fs::read_dir(folder).await {
            Ok(entries) => entries,
            Err(e) => return Err(ModelError::LoadingFailed(format!("Cannot read directory {}: {}", folder.display(), e))),
        };

        while let Some(entry) = entries.next_entry().await.map_err(|e| ModelError::LoadingFailed(e.to_string()))? {
            let path = entry.path();
            if let Some(extension) = path.extension() {
                if extension == "gguf" {
                    let filename = path.file_name().unwrap().to_string_lossy().to_lowercase();
                    if filename.contains("bf16") || filename.contains("BF16") {
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

        Err(ModelError::NotFound(format!("No .gguf model files found in {}", folder.display())))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{ModelSource, ModelConfig};
    use std::path::PathBuf;
    use tempfile::TempDir;
    use tokio::fs;

    fn create_test_config_local(folder: PathBuf, filename: Option<String>) -> ModelConfig {
        ModelConfig {
            source: ModelSource::Local { folder, filename },
            batch_size: 512,
            use_hf_params: false,
        }
    }

    fn create_test_config_hf(repo: String, filename: Option<String>) -> ModelConfig {
        ModelConfig {
            source: ModelSource::HuggingFace { repo, filename },
            batch_size: 512,
            use_hf_params: true,
        }
    }

    #[tokio::test]
    async fn test_model_manager_creation() {
        let config = create_test_config_local(PathBuf::from("/tmp"), None);
        let manager = ModelManager::new(config);
        
        assert!(!manager.is_loaded().await);
        assert!(manager.get_model().await.is_none());
        assert!(manager.get_context().await.is_none());
    }

    #[tokio::test]
    async fn test_model_loading_with_valid_file() {
        let temp_dir = TempDir::new().unwrap();
        let model_file = temp_dir.path().join("test-model.gguf");
        
        // Create a dummy .gguf file
        fs::write(&model_file, b"dummy model content").await.unwrap();

        let config = create_test_config_local(temp_dir.path().to_path_buf(), Some("test-model.gguf".to_string()));
        let manager = ModelManager::new(config);

        // This should successfully load the mock model
        let result = manager.load_model().await;
        assert!(result.is_ok());
        
        assert!(manager.is_loaded().await);
        assert!(manager.get_model().await.is_some());
        assert!(manager.get_context().await.is_some());
        
        let model = manager.get_model().await.unwrap();
        assert_eq!(model.path, model_file);
        assert_eq!(model.config.batch_size, 512);
    }

    #[tokio::test]
    async fn test_resolve_local_path_with_filename() {
        let temp_dir = TempDir::new().unwrap();
        let model_file = temp_dir.path().join("test-model.gguf");
        
        // Create a dummy .gguf file
        fs::write(&model_file, b"dummy model content").await.unwrap();

        let config = create_test_config_local(temp_dir.path().to_path_buf(), Some("test-model.gguf".to_string()));
        let manager = ModelManager::new(config);

        let resolved_path = manager.resolve_model_path().await.unwrap();
        assert_eq!(resolved_path, model_file);
    }

    #[tokio::test]
    async fn test_resolve_local_path_file_not_found() {
        let temp_dir = TempDir::new().unwrap();
        let config = create_test_config_local(temp_dir.path().to_path_buf(), Some("nonexistent.gguf".to_string()));
        let manager = ModelManager::new(config);

        let result = manager.resolve_model_path().await;
        assert!(matches!(result, Err(ModelError::NotFound(_))));
    }

    #[tokio::test]
    async fn test_resolve_local_path_folder_not_found() {
        let config = create_test_config_local(PathBuf::from("/nonexistent/folder"), Some("model.gguf".to_string()));
        let manager = ModelManager::new(config);

        let result = manager.resolve_model_path().await;
        assert!(matches!(result, Err(ModelError::NotFound(_))));
    }

    #[tokio::test]
    async fn test_resolve_local_path_invalid_extension() {
        let temp_dir = TempDir::new().unwrap();
        let model_file = temp_dir.path().join("test-model.txt");
        
        // Create a dummy .txt file
        fs::write(&model_file, b"not a model").await.unwrap();

        let config = create_test_config_local(temp_dir.path().to_path_buf(), Some("test-model.txt".to_string()));
        let manager = ModelManager::new(config);

        let result = manager.resolve_model_path().await;
        assert!(matches!(result, Err(ModelError::InvalidConfig(_))));
    }

    #[tokio::test]
    async fn test_auto_detect_bf16_preference() {
        let temp_dir = TempDir::new().unwrap();
        
        // Create multiple GGUF files, including BF16
        let regular_model = temp_dir.path().join("model-q4.gguf");
        let bf16_model = temp_dir.path().join("model-bf16.gguf");
        let another_model = temp_dir.path().join("model-q8.gguf");
        
        fs::write(&regular_model, b"regular model").await.unwrap();
        fs::write(&bf16_model, b"bf16 model").await.unwrap();
        fs::write(&another_model, b"another model").await.unwrap();

        let config = create_test_config_local(temp_dir.path().to_path_buf(), None);
        let manager = ModelManager::new(config);

        let resolved_path = manager.resolve_model_path().await.unwrap();
        assert_eq!(resolved_path, bf16_model);
    }

    #[tokio::test]
    async fn test_auto_detect_fallback_to_first_gguf() {
        let temp_dir = TempDir::new().unwrap();
        
        // Create GGUF files without BF16
        let model1 = temp_dir.path().join("model-q4.gguf");
        let model2 = temp_dir.path().join("model-q8.gguf");
        
        fs::write(&model1, b"model 1").await.unwrap();
        fs::write(&model2, b"model 2").await.unwrap();

        let config = create_test_config_local(temp_dir.path().to_path_buf(), None);
        let manager = ModelManager::new(config);

        let resolved_path = manager.resolve_model_path().await.unwrap();
        // Should return one of the models (order may vary by filesystem)
        assert!(resolved_path == model1 || resolved_path == model2);
        assert!(resolved_path.extension().unwrap() == "gguf");
    }

    #[tokio::test]
    async fn test_auto_detect_no_gguf_files() {
        let temp_dir = TempDir::new().unwrap();
        
        // Create non-GGUF files
        let txt_file = temp_dir.path().join("readme.txt");
        fs::write(&txt_file, b"readme content").await.unwrap();

        let config = create_test_config_local(temp_dir.path().to_path_buf(), None);
        let manager = ModelManager::new(config);

        let result = manager.resolve_model_path().await;
        assert!(matches!(result, Err(ModelError::NotFound(_))));
    }

    #[tokio::test]
    async fn test_huggingface_config_creation() {
        let config = create_test_config_hf("microsoft/DialoGPT-medium".to_string(), None);
        let manager = ModelManager::new(config);
        
        // Test that we can create the manager (HF resolution will fail in test environment)
        assert!(!manager.is_loaded().await);
    }

    #[test]
    fn test_model_config_debug() {
        let config = create_test_config_local(PathBuf::from("/tmp"), Some("test.gguf".to_string()));
        let debug_str = format!("{:?}", config);
        assert!(debug_str.contains("Local"));
        assert!(debug_str.contains("test.gguf"));
        assert!(debug_str.contains("512"));
    }
}
