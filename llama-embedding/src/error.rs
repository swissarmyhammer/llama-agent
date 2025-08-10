use thiserror::Error;

/// Errors that can occur during embedding operations
#[derive(Error, Debug)]
pub enum EmbeddingError {
    /// Error loading or using the model
    #[error("Model error: {0}")]
    Model(#[from] llama_loader::ModelError),

    /// Error during batch processing
    #[error("Batch processing error: {0}")]
    BatchProcessing(String),

    /// Error encoding text for the model
    #[error("Text encoding error: {0}")]
    TextEncoding(String),

    /// IO error reading/writing files
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Model is not loaded
    #[error("Model is not loaded")]
    ModelNotLoaded,

    /// Invalid configuration
    #[error("Invalid configuration: {0}")]
    InvalidConfiguration(String),
}

/// Result type for embedding operations
pub type EmbeddingResult<T> = Result<T, EmbeddingError>;
