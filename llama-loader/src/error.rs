use thiserror::Error;

/// Errors that can occur during model loading operations
#[derive(Debug, Error)]
pub enum ModelError {
    /// Model loading failed
    #[error("Model loading failed: {0}\n🔧 Check available memory (4-8GB needed), verify GGUF file integrity, ensure compatible llama.cpp version")]
    LoadingFailed(String),

    /// Model not found at the specified location
    #[error("Model not found: {0}\n📁 Verify file path is correct, file exists and is readable. For HuggingFace: check repo name and filename")]
    NotFound(String),

    /// Invalid model configuration
    #[error("Invalid model config: {0}\n⚙️ Ensure batch_size > 0, valid model source path, and appropriate use_hf_params setting")]
    InvalidConfig(String),

    /// Model inference operation failed
    #[error("Model inference failed: {0}\n🦾 Check input format, model compatibility, and available system resources")]
    InferenceFailed(String),

    /// Network error during model download
    #[error("Network error: {0}\n🌐 Check internet connection and HuggingFace availability")]
    Network(String),

    /// I/O error during file operations
    #[error("I/O error: {0}\n💾 Check disk space, file permissions, and storage availability")]
    Io(#[from] std::io::Error),

    /// Cache operation error
    #[error("Cache error: {0}\n💽 Check cache directory permissions and disk space")]
    Cache(String),
}

impl ModelError {
    /// Create a new ModelError from a string message
    pub fn new(message: impl Into<String>) -> Self {
        Self::LoadingFailed(message.into())
    }

    /// Check if this error is retriable
    pub fn is_retriable(&self) -> bool {
        matches!(
            self,
            ModelError::Network(_) | ModelError::Io(_) | ModelError::LoadingFailed(_)
        )
    }
}

// Convert from llama-cpp-2 errors
impl From<llama_cpp_2::LLamaCppError> for ModelError {
    fn from(err: llama_cpp_2::LLamaCppError) -> Self {
        match err {
            llama_cpp_2::LLamaCppError::BackendAlreadyInitialized => {
                ModelError::LoadingFailed("Backend already initialized".to_string())
            }
            other => ModelError::LoadingFailed(format!("llama-cpp-2 error: {}", other)),
        }
    }
}

// Convert from HuggingFace hub errors
impl From<hf_hub::api::tokio::ApiError> for ModelError {
    fn from(err: hf_hub::api::tokio::ApiError) -> Self {
        let err_str = format!("{}", err);
        if err_str.contains("not found") || err_str.contains("404") {
            ModelError::NotFound(format!("HuggingFace resource not found: {}", err))
        } else {
            ModelError::Network(format!("HuggingFace API error: {}", err))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io;

    #[test]
    fn test_model_error_creation() {
        let err = ModelError::new("test error");
        assert!(matches!(err, ModelError::LoadingFailed(_)));
    }

    #[test]
    fn test_error_retriability() {
        assert!(ModelError::Network("test".to_string()).is_retriable());
        assert!(ModelError::LoadingFailed("test".to_string()).is_retriable());
        assert!(!ModelError::InvalidConfig("test".to_string()).is_retriable());
        assert!(!ModelError::InferenceFailed("test".to_string()).is_retriable());
    }

    #[test]
    fn test_io_error_conversion() {
        let io_err = io::Error::new(io::ErrorKind::NotFound, "file not found");
        let model_err = ModelError::from(io_err);
        assert!(matches!(model_err, ModelError::Io(_)));
    }

    #[test]
    fn test_llama_cpp_error_conversion() {
        let llama_err = llama_cpp_2::LLamaCppError::BackendAlreadyInitialized;
        let model_err = ModelError::from(llama_err);
        assert!(matches!(model_err, ModelError::LoadingFailed(_)));
    }

    #[test]
    fn test_error_display() {
        let err = ModelError::LoadingFailed("test error".to_string());
        let display_str = format!("{}", err);
        assert!(display_str.contains("test error"));
        assert!(display_str.contains("🔧")); // Contains helpful emoji
    }
}
