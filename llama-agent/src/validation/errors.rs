//! Validation error types

/// Validation errors that can occur during validation
#[derive(Debug, thiserror::Error)]
pub enum ValidationError {
    #[error("Security violation: {0}")]
    SecurityViolation(String),

    #[error("Parameter out of bounds: {0}")]
    ParameterBounds(String),

    #[error("Invalid state: {0}")]
    InvalidState(String),

    #[error("Content validation failed: {0}")]
    ContentValidation(String),

    #[error("Schema validation failed: {0}")]
    SchemaValidation(String),
}
