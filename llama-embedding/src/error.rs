use thiserror::Error;

/// Errors that can occur during embedding operations
#[derive(Error, Debug)]
pub enum EmbeddingError {
    /// Error from the model loader
    #[error("Model loading error: {0}")]
    ModelLoader(#[from] llama_loader::ModelError),

    /// Error initializing or using the llama-cpp-2 model
    #[error("Model error: {0}")]
    Model(String),

    /// Error during text processing or tokenization
    #[error("Text processing error: {0}")]
    TextProcessing(String),

    /// Error during batch processing
    #[error("Batch processing error: {0}")]
    BatchProcessing(String),

    /// Error with text encoding
    #[error("Text encoding error: {0}")]
    TextEncoding(String),

    /// Configuration error
    #[error("Configuration error: {0}")]
    Configuration(String),

    /// IO error
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Error when model is not loaded
    #[error("Model not loaded - call load_model() first")]
    ModelNotLoaded,

    /// Error when embedding dimensions don't match expectations
    #[error("Embedding dimension mismatch: expected {expected}, got {actual}")]
    DimensionMismatch { expected: usize, actual: usize },
}

impl EmbeddingError {
    /// Create a new model error
    pub fn model<S: Into<String>>(message: S) -> Self {
        Self::Model(message.into())
    }

    /// Create a new text processing error
    pub fn text_processing<S: Into<String>>(message: S) -> Self {
        Self::TextProcessing(message.into())
    }

    /// Create a new batch processing error
    pub fn batch_processing<S: Into<String>>(message: S) -> Self {
        Self::BatchProcessing(message.into())
    }

    /// Create a new text encoding error
    pub fn text_encoding<S: Into<String>>(message: S) -> Self {
        Self::TextEncoding(message.into())
    }

    /// Create a new configuration error
    pub fn configuration<S: Into<String>>(message: S) -> Self {
        Self::Configuration(message.into())
    }
}

/// Result type alias for embedding operations
pub type EmbeddingResult<T> = Result<T, EmbeddingError>;

#[cfg(test)]
mod tests {
    use super::*;
    use std::io;

    #[test]
    fn test_error_creation() {
        let error = EmbeddingError::model("test model error");
        assert!(matches!(error, EmbeddingError::Model(_)));
        assert_eq!(error.to_string(), "Model error: test model error");

        let error = EmbeddingError::text_processing("test text error");
        assert!(matches!(error, EmbeddingError::TextProcessing(_)));
        assert_eq!(error.to_string(), "Text processing error: test text error");

        let error = EmbeddingError::ModelNotLoaded;
        assert!(matches!(error, EmbeddingError::ModelNotLoaded));
        assert_eq!(error.to_string(), "Model not loaded - call load_model() first");
    }

    #[test]
    fn test_error_conversion() {
        let io_error = io::Error::new(io::ErrorKind::NotFound, "file not found");
        let embedding_error: EmbeddingError = io_error.into();
        assert!(matches!(embedding_error, EmbeddingError::Io(_)));
    }

    #[test]
    fn test_dimension_mismatch_error() {
        let error = EmbeddingError::DimensionMismatch {
            expected: 384,
            actual: 768,
        };
        assert_eq!(
            error.to_string(),
            "Embedding dimension mismatch: expected 384, got 768"
        );
    }
}
