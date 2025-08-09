//! Validation error types

/// Validation errors that can occur during validation
#[derive(Debug, thiserror::Error, Clone, PartialEq)]
pub enum ValidationError {
    /// Security violation detected
    #[error("Security violation: {0}")]
    SecurityViolation(String),

    /// Parameter is outside acceptable bounds
    #[error("Parameter out of bounds: {0}")]
    ParameterBounds(String),

    /// Invalid state detected
    #[error("Invalid state: {0}")]
    InvalidState(String),

    /// Content validation failed
    #[error("Content validation failed: {0}")]
    ContentValidation(String),

    /// Schema validation failed
    #[error("Schema validation failed: {0}")]
    SchemaValidation(String),

    /// Multiple validation errors occurred
    #[error("Multiple validation errors: {}", .0.iter().map(|e| e.to_string()).collect::<Vec<_>>().join(", "))]
    Multiple(Vec<ValidationError>),
}

impl ValidationError {
    /// Create a security violation error
    pub fn security_violation(msg: impl Into<String>) -> Self {
        Self::SecurityViolation(msg.into())
    }

    /// Create a parameter bounds error
    pub fn parameter_bounds(msg: impl Into<String>) -> Self {
        Self::ParameterBounds(msg.into())
    }

    /// Create an invalid state error
    pub fn invalid_state(msg: impl Into<String>) -> Self {
        Self::InvalidState(msg.into())
    }

    /// Create a content validation error
    pub fn content_validation(msg: impl Into<String>) -> Self {
        Self::ContentValidation(msg.into())
    }

    /// Create a schema validation error
    pub fn schema_validation(msg: impl Into<String>) -> Self {
        Self::SchemaValidation(msg.into())
    }

    /// Combine multiple validation errors
    pub fn multiple(errors: Vec<ValidationError>) -> Self {
        if errors.len() == 1 {
            errors.into_iter().next().unwrap()
        } else {
            Self::Multiple(errors)
        }
    }
}

/// Result type for validation operations
pub type ValidationResult<T = ()> = Result<T, ValidationError>;
