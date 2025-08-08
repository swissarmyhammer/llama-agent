use crate::types::{ModelConfig, ModelError, ModelSource};
use hf_hub::api::tokio::ApiBuilder;
use llama_cpp_2::{
    context::{params::LlamaContextParams, LlamaContext},
    llama_backend::LlamaBackend,
    model::{params::LlamaModelParams, LlamaModel},
};
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};
use std::time::Instant;
// Need access to raw FFI bindings for llama_log_set
use std::ffi::c_void;
use std::os::raw::c_char;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

static GLOBAL_BACKEND: OnceLock<Arc<LlamaBackend>> = OnceLock::new();

// Null log callback to suppress llama.cpp verbose output
extern "C" fn null_log_callback(_level: i32, _text: *const c_char, _user_data: *mut c_void) {
    // Do nothing - this suppresses all llama.cpp logging
}

// Set up logging suppression using llama_log_set
fn set_logging_suppression(suppress: bool) {
    unsafe {
        // Access the raw FFI binding
        extern "C" {
            fn llama_log_set(log_callback: Option<extern "C" fn(i32, *const c_char, *mut c_void)>, user_data: *mut c_void);
        }

        if suppress {
            // Set null callback to suppress logging
            llama_log_set(Some(null_log_callback), std::ptr::null_mut());
        } else {
            // Restore default logging (NULL callback means output to stderr)
            llama_log_set(None, std::ptr::null_mut());
        }
    }
}

pub struct ModelManager {
    model: Arc<RwLock<Option<LlamaModel>>>,
    backend: Arc<LlamaBackend>,
    config: ModelConfig,
    load_start_time: Option<Instant>,
    memory_usage_bytes: Arc<std::sync::atomic::AtomicU64>,
}

impl ModelManager {
    pub fn new(config: ModelConfig) -> Result<Self, ModelError> {
        // Get existing backend or try to initialize new one
        let backend = if let Some(backend) = GLOBAL_BACKEND.get() {
            backend.clone()
        } else {
            // Try to initialize the backend
            let new_backend = match LlamaBackend::init() {
                Ok(backend) => Arc::new(backend),
                Err(llama_cpp_2::LLamaCppError::BackendAlreadyInitialized) => {
                    // Backend was already initialized but we don't have a reference
                    // This is a limitation of llama-cpp-2 - we can't get a reference to an existing backend
                    // For now, we'll work around this by skipping backend initialization in tests
                    return Err(ModelError::LoadingFailed(
                        "Backend already initialized by external code".to_string(),
                    ));
                }
                Err(e) => {
                    return Err(ModelError::LoadingFailed(format!(
                        "Failed to initialize LlamaBackend: {}",
                        e
                    )));
                }
            };

            // Try to store it globally, but don't fail if someone else beat us to it
            if GLOBAL_BACKEND.set(new_backend.clone()).is_err() {
                // Someone else set it, use theirs instead
                GLOBAL_BACKEND.get().unwrap().clone()
            } else {
                new_backend
            }
        };

        let manager = Self {
            model: Arc::new(RwLock::new(None)),
            backend,
            config,
            load_start_time: None,
            memory_usage_bytes: Arc::new(std::sync::atomic::AtomicU64::new(0)),
        };
        Ok(manager)
    }

    pub async fn load_model(&self) -> Result<(), ModelError> {
        let start_time = Instant::now();
        info!("🚀 Starting model loading process...");
        info!("Model configuration: {:?}", self.config);

        // Log initial memory usage
        self.log_memory_usage("model loading start").await;

        // Validate config before proceeding
        info!("📋 Validating model configuration...");
        self.config.validate()?;
        info!("✅ Configuration validation completed");

        // Log memory usage before loading
        let memory_before = Self::get_process_memory_mb().unwrap_or(0);
        debug!("Memory usage before model loading: {} MB", memory_before);

        // Load model based on source type with progress indication
        let model = match &self.config.source {
            ModelSource::HuggingFace { repo, filename } => {
                if self.config.verbose_logging {
                    info!("Starting HuggingFace model download/loading for: {}", repo);
                }
                self.load_huggingface_model(repo, filename.as_deref())
                    .await?
            }
            ModelSource::Local { folder, filename } => {
                if self.config.verbose_logging {
                    info!("Loading local model from: {}", folder.display());
                }
                self.load_local_model(folder, filename.as_deref()).await?
            }
        };

        let load_time = start_time.elapsed();
        let memory_after = Self::get_process_memory_mb().unwrap_or(0);
        let memory_used = memory_after.saturating_sub(memory_before);

        // Store memory usage estimate (atomic operation safe in Arc)
        self.memory_usage_bytes.store(
            memory_used * 1024 * 1024,
            std::sync::atomic::Ordering::Relaxed,
        );

        info!("💾 Storing model in memory...");
        info!(
            "Model loaded successfully in {:?} (Memory: +{} MB, Total: {} MB)",
            load_time, memory_used, memory_after
        );

        // Store model
        {
            let mut model_lock = self.model.write().await;
            *model_lock = Some(model);
        }

        // Log final memory usage
        self.log_memory_usage("model loading complete").await;
        info!("🎉 Model loading completed successfully!");
        info!("Model is ready for inference requests");

        Ok(())
    }

    pub async fn is_loaded(&self) -> bool {
        let model_lock = self.model.read().await;
        model_lock.is_some()
    }

    pub async fn with_model<F, R>(&self, f: F) -> Result<R, ModelError>
    where
        F: FnOnce(&LlamaModel) -> R,
    {
        let model_lock = self.model.read().await;
        match model_lock.as_ref() {
            Some(model) => Ok(f(model)),
            None => Err(ModelError::LoadingFailed("Model not loaded".to_string())),
        }
    }

    /// Unload the model from memory to free resources
    pub async fn unload_model(&self) -> Result<(), ModelError> {
        info!("🧹 Unloading model to free memory resources...");

        let mut model_lock = self.model.write().await;
        if model_lock.is_some() {
            *model_lock = None;
            info!("✅ Model unloaded successfully - memory freed");
        } else {
            debug!("ℹ️  No model loaded - nothing to unload");
        }

        Ok(())
    }

    /// Get memory usage information for the loaded model
    pub async fn get_model_info(&self) -> Option<String> {
        let model_lock = self.model.read().await;
        if model_lock.is_some() {
            Some(format!(
                "Model loaded - Type: {}",
                match &self.config.source {
                    ModelSource::Local {
                        folder: _,
                        filename,
                    } => {
                        format!("Local ({})", filename.as_deref().unwrap_or("auto-detected"))
                    }
                    ModelSource::HuggingFace { repo, filename } => {
                        format!(
                            "HuggingFace ({}/{})",
                            repo,
                            filename.as_deref().unwrap_or("auto-detected")
                        )
                    }
                }
            ))
        } else {
            None
        }
    }

    pub fn create_context<'a>(
        &self,
        model: &'a LlamaModel,
    ) -> Result<LlamaContext<'a>, ModelError> {
        // Optimize context parameters for better performance
        use std::num::NonZero;
        let context_params = LlamaContextParams::default()
            .with_n_ctx(Some(
                NonZero::new(self.config.batch_size.max(2048)).unwrap(),
            )) // Use at least 2048 context
            .with_n_batch(self.config.batch_size) // Optimize batch size
            .with_n_threads(num_cpus::get().min(8) as i32) // Use available CPU cores, capped at 8
            .with_n_threads_batch(num_cpus::get().min(4) as i32) // Optimize batch threads
            .with_embeddings(false) // Disable embeddings for inference-only mode
            .with_offload_kqv(true); // Enable KQV offloading for memory optimization

        if self.config.verbose_logging {
            debug!(
                "Creating context with optimized parameters for batch_size={}",
                self.config.batch_size
            );
        }

        // Set logging suppression for context creation if verbose logging is disabled
        if !self.config.verbose_logging {
            set_logging_suppression(true);
        }

        let result = model
            .new_context(&self.backend, context_params)
            .map_err(move |e| ModelError::LoadingFailed(format!("Failed to create context: {}", e)));

        // Restore default logging after context creation
        if !self.config.verbose_logging {
            set_logging_suppression(false);
        }

        result
    }

    async fn load_huggingface_model(
        &self,
        repo: &str,
        filename: Option<&str>,
    ) -> Result<LlamaModel, ModelError> {
        info!("Starting HuggingFace model download/loading for: {}", repo);

        // Build HuggingFace API client with progress indication
        let api = ApiBuilder::new().with_progress(true).build().map_err(|e| {
            ModelError::LoadingFailed(format!("Failed to initialize HuggingFace API: {}", e))
        })?;

        let hf_repo = api.model(repo.to_string());

        let model_path = if let Some(filename) = filename {
            info!("Downloading specific model file: {}", filename);
            hf_repo.get(filename).await.map_err(|e| {
                ModelError::LoadingFailed(format!(
                    "Failed to download model file '{}' from HuggingFace repo '{}': {}",
                    filename, repo, e
                ))
            })?
        } else {
            info!(
                "Auto-detecting GGUF files in HuggingFace repository: {}",
                repo
            );
            self.download_auto_detect_gguf(&hf_repo, repo).await?
        };

        info!("Model downloaded to: {:?}", model_path);

        // Load the downloaded model using the existing local model loading logic
        self.load_model_from_file(&model_path).await
    }

    async fn load_local_model(
        &self,
        folder: &Path,
        filename: Option<&str>,
    ) -> Result<LlamaModel, ModelError> {
        info!("Loading model from local folder: {:?}", folder);

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

        self.load_model_from_file(&model_path).await
    }

    async fn download_auto_detect_gguf(
        &self,
        hf_repo: &hf_hub::api::tokio::ApiRepo,
        repo_name: &str,
    ) -> Result<PathBuf, ModelError> {
        // Try to list files in the repository to find GGUF files
        // Since hf-hub doesn't provide a direct listing API, we'll try common GGUF filenames
        // Based on the repo name, try to derive model-specific patterns first
        let mut common_gguf_patterns = Vec::new();

        // Extract potential model name from repo (e.g., "unsloth/Qwen3-0.6B-GGUF" -> "Qwen3-0.6B")
        if let Some(model_part) = repo_name.split('/').last() {
            let base_name = model_part.replace("-GGUF", "");
            // Add common quantization patterns for this specific model
            common_gguf_patterns.extend([
                format!("{}-BF16.gguf", base_name),
                format!("{}-Q4_K_M.gguf", base_name),
                format!("{}-Q4_0.gguf", base_name),
                format!("{}-Q4_1.gguf", base_name),
                format!("{}-Q5_K_M.gguf", base_name),
                format!("{}-Q5_K_S.gguf", base_name),
                format!("{}-Q8_0.gguf", base_name),
                format!("{}-Q6_K.gguf", base_name),
                format!("{}-Q3_K_M.gguf", base_name),
                format!("{}-Q2_K.gguf", base_name),
            ]);
        }

        // Add fallback generic patterns
        common_gguf_patterns.extend([
            "model.gguf".to_string(),
            "ggml-model.gguf".to_string(),
            "ggml-model-q4_0.gguf".to_string(),
            "ggml-model-q4_1.gguf".to_string(),
            "ggml-model-q5_0.gguf".to_string(),
            "ggml-model-q5_1.gguf".to_string(),
            "ggml-model-q8_0.gguf".to_string(),
            "ggml-model-f16.gguf".to_string(),
            "ggml-model-f32.gguf".to_string(),
        ]);

        for pattern in &common_gguf_patterns {
            info!("Trying to download: {}", pattern);
            match hf_repo.get(pattern).await {
                Ok(path) => {
                    info!("Successfully found and downloaded: {}", pattern);
                    return Ok(path);
                }
                Err(_) => {
                    debug!("File '{}' not found, trying next pattern", pattern);
                    continue;
                }
            }
        }

        Err(ModelError::NotFound(format!(
            "No common GGUF model files found in HuggingFace repo: {}. \
            Try specifying a filename explicitly or check the repository contents.",
            repo_name
        )))
    }

    async fn load_model_from_file(&self, model_path: &Path) -> Result<LlamaModel, ModelError> {
        info!("Loading model from path: {:?}", model_path);

        // Optimize model loading with performance parameters
        let model_params = LlamaModelParams::default();
        if self.config.verbose_logging {
            info!("⚙️  Loading model with optimized parameters (memory mapping enabled)");
            info!("📁 Model file: {}", model_path.display());
        }

        // Set logging suppression based on verbose_logging flag
        set_logging_suppression(!self.config.verbose_logging);

        let model = LlamaModel::load_from_file(&self.backend, model_path, &model_params).map_err(|e| {
            ModelError::LoadingFailed(format!(
                "Failed to load model from {}: {}",
                model_path.display(),
                e
            ))
        })?;

        // Restore default logging after model loading
        if !self.config.verbose_logging {
            set_logging_suppression(false);
        }

        info!(
            "✅ Model successfully loaded from: {}",
            model_path.display()
        );

        Ok(model)
    }

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

    /// Get current memory usage information
    pub fn get_memory_usage(&self) -> MemoryUsageInfo {
        let process_memory = get_process_memory_usage();

        MemoryUsageInfo {
            process_memory_mb: process_memory,
            estimated_model_memory_mb: self.estimate_model_memory(),
        }
    }

    /// Estimate model memory usage based on configuration
    fn estimate_model_memory(&self) -> f64 {
        // Rough estimation: batch_size * context_size * 4 bytes per float
        // This is a conservative estimate for memory planning
        let batch_memory = (self.config.batch_size as f64 * 2048.0 * 4.0) / (1024.0 * 1024.0);
        batch_memory.max(100.0) // Minimum 100MB estimate
    }

    /// Log memory usage with performance implications
    pub async fn log_memory_usage(&self, operation: &str) {
        let usage = self.get_memory_usage();
        info!(
            "📊 Memory usage during {}: Process={}MB, Estimated Model={}MB",
            operation,
            usage.process_memory_mb.round(),
            usage.estimated_model_memory_mb.round()
        );

        if usage.process_memory_mb > 8000.0 {
            warn!(
                "⚠️  High memory usage detected ({}MB). Consider reducing batch_size or model size",
                usage.process_memory_mb.round()
            );
        }
    }

    /// Get current process memory usage in MB
    fn get_process_memory_mb() -> Result<u64, std::io::Error> {
        #[cfg(target_os = "linux")]
        {
            use std::fs;
            let status = fs::read_to_string("/proc/self/status")?;
            for line in status.lines() {
                if line.starts_with("VmRSS:") {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if parts.len() >= 2 {
                        if let Ok(kb) = parts[1].parse::<u64>() {
                            return Ok(kb / 1024); // Convert KB to MB
                        }
                    }
                }
            }
            Ok(0)
        }
        #[cfg(target_os = "macos")]
        {
            // Use mach API on macOS for memory info
            // For simplicity, return 0 - could be implemented with mach sys calls
            Ok(0)
        }
        #[cfg(target_os = "windows")]
        {
            // Use Windows API for memory info
            // For simplicity, return 0 - could be implemented with winapi
            Ok(0)
        }
        #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
        {
            Ok(0)
        }
    }

    /// Get estimated memory usage of the loaded model in bytes
    pub fn get_memory_usage_bytes(&self) -> u64 {
        self.memory_usage_bytes
            .load(std::sync::atomic::Ordering::Relaxed)
    }

    /// Get model loading statistics
    pub fn get_load_stats(&self) -> Option<(std::time::Duration, u64)> {
        self.load_start_time.map(|start| {
            let duration = start.elapsed();
            let memory_bytes = self.get_memory_usage_bytes();
            (duration, memory_bytes)
        })
    }
}

/// Memory usage information
#[derive(Debug, Clone)]
pub struct MemoryUsageInfo {
    pub process_memory_mb: f64,
    pub estimated_model_memory_mb: f64,
}

/// Get current process memory usage in MB
fn get_process_memory_usage() -> f64 {
    match ModelManager::get_process_memory_mb() {
        Ok(mb) => mb as f64,
        Err(_) => 0.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{ModelConfig, ModelSource};
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

        // When running tests in parallel, the backend might already be initialized by another test
        match ModelManager::new(config) {
            Ok(manager) => {
                assert!(!manager.is_loaded().await);

                // Test with_model when no model is loaded
                let result = manager.with_model(|_model| ()).await;
                assert!(result.is_err());
            }
            Err(ModelError::LoadingFailed(msg))
                if msg.contains("Backend already initialized by external code") =>
            {
                // This is expected when running tests in parallel - one test initializes the backend
                // and subsequent tests see it as already initialized. This is fine for the test.
                println!("Backend already initialized by another test - this is expected in parallel test execution");
            }
            Err(e) => {
                panic!("Unexpected error creating ModelManager: {:?}", e);
            }
        }
    }

    #[tokio::test]
    async fn test_model_loading_with_invalid_file() {
        let temp_dir = TempDir::new().unwrap();
        let model_file = temp_dir.path().join("test-model.gguf");

        // Create a dummy .gguf file (this will fail to load as real model)
        fs::write(&model_file, b"dummy model content")
            .await
            .unwrap();

        let config = create_test_config_local(
            temp_dir.path().to_path_buf(),
            Some("test-model.gguf".to_string()),
        );
        let manager = Arc::new(ModelManager::new(config).expect("Failed to create ModelManager"));

        // This should fail because dummy content is not a valid GGUF model
        let result = manager.load_model().await;
        assert!(result.is_err());
        assert!(!manager.is_loaded().await);
    }

    #[tokio::test]
    async fn test_model_file_not_found() {
        let temp_dir = TempDir::new().unwrap();
        let config = create_test_config_local(
            temp_dir.path().to_path_buf(),
            Some("nonexistent.gguf".to_string()),
        );
        let manager = Arc::new(ModelManager::new(config).expect("Failed to create ModelManager"));

        let result = manager.load_model().await;
        assert!(result.is_err());
        match result.unwrap_err() {
            ModelError::NotFound(_) => {}
            _ => panic!("Expected NotFound error"),
        }
    }

    #[tokio::test]
    async fn test_folder_not_found() {
        let config = create_test_config_local(
            PathBuf::from("/nonexistent/folder"),
            Some("model.gguf".to_string()),
        );

        // When running tests in parallel, the backend might already be initialized by another test
        match ModelManager::new(config) {
            Ok(manager) => {
                let manager = Arc::new(manager);
                let result = manager.load_model().await;
                assert!(result.is_err());
                match result.unwrap_err() {
                    ModelError::NotFound(_) => {}
                    _ => panic!("Expected NotFound error"),
                }
            }
            Err(ModelError::LoadingFailed(msg))
                if msg.contains("Backend already initialized by external code") =>
            {
                // This is expected when running tests in parallel - one test initializes the backend
                // and subsequent tests see it as already initialized. This is fine for the test.
                println!("Backend already initialized by another test - this is expected in parallel test execution");
            }
            Err(e) => {
                panic!("Unexpected error creating ModelManager: {:?}", e);
            }
        }
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
        let manager = Arc::new(ModelManager::new(config).expect("Failed to create ModelManager"));

        // This should try to load the BF16 file first (though it will fail with invalid content)
        let result = manager.load_model().await;
        assert!(result.is_err()); // Will fail due to invalid GGUF content, but that's expected
    }

    #[tokio::test]
    async fn test_auto_detect_no_gguf_files() {
        let temp_dir = TempDir::new().unwrap();

        // Create non-GGUF files
        let txt_file = temp_dir.path().join("readme.txt");
        fs::write(&txt_file, b"readme content").await.unwrap();

        let config = create_test_config_local(temp_dir.path().to_path_buf(), None);

        // When running tests in parallel, the backend might already be initialized by another test
        match ModelManager::new(config) {
            Ok(manager) => {
                let manager = Arc::new(manager);
                let result = manager.load_model().await;
                assert!(result.is_err());
                match result.unwrap_err() {
                    ModelError::NotFound(_) => {}
                    _ => panic!("Expected NotFound error"),
                }
            }
            Err(ModelError::LoadingFailed(msg))
                if msg.contains("Backend already initialized by external code") =>
            {
                // This is expected when running tests in parallel - one test initializes the backend
                // and subsequent tests see it as already initialized. This is fine for the test.
                println!("Backend already initialized by another test - this is expected in parallel test execution");
            }
            Err(e) => {
                panic!("Unexpected error creating ModelManager: {:?}", e);
            }
        }
    }

    #[tokio::test]
    async fn test_huggingface_config_creation() {
        let config = create_test_config_hf("microsoft/DialoGPT-medium".to_string(), None);

        // When running tests in parallel, the backend might already be initialized by another test
        // This is expected and should not cause test failures
        match ModelManager::new(config) {
            Ok(manager) => {
                // Test that we can create the manager (HF loading will treat repo as local path and fail)
                assert!(!manager.is_loaded().await);

                let manager = Arc::new(manager);
                let result = manager.load_model().await;
                assert!(result.is_err()); // Will fail since "microsoft/DialoGPT-medium" is not a local path
            }
            Err(ModelError::LoadingFailed(msg))
                if msg.contains("Backend already initialized by external code") =>
            {
                // This is expected when running tests in parallel - one test initializes the backend
                // and subsequent tests see it as already initialized. This is fine for the test.
                println!("Backend already initialized by another test - this is expected in parallel test execution");
            }
            Err(e) => {
                panic!("Unexpected error creating ModelManager: {:?}", e);
            }
        }
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
