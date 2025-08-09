# Implement Core Validation Traits

Refer to ./specification/validation.md

## Overview
Implement the core validation traits and error handling system that will be used by all validators. This creates the foundational interface that ensures consistent validation patterns across the system.

## Acceptance Criteria
- [ ] Complete the `Validator<Target>` trait definition
- [ ] Implement comprehensive `ValidationError` enum
- [ ] Add trait bounds and generic constraints
- [ ] Create trait implementations for common validation patterns
- [ ] Add documentation and usage examples

## Implementation Details

### Update `src/validation/traits.rs`

```rust
//! Core validation traits and interfaces

use super::errors::ValidationError;
use crate::types::Session;

/// Core validation trait that all validators implement
/// 
/// The session parameter provides universal context for all validation operations.
/// Every validation occurs within the scope of a session, providing access to:
/// - Message history for context-aware validation
/// - Tool availability for validation decisions  
/// - Session state and metadata for temporal validations
/// - MCP configuration that may affect validation rules
pub trait Validator<Target> {
    type Error;
    
    /// Validate a target within the context of a session
    /// 
    /// # Arguments
    /// * `session` - The session context providing validation metadata
    /// * `target` - The object to validate
    /// 
    /// # Returns
    /// Ok(()) if validation passes, Error if validation fails
    fn validate(&self, session: &Session, target: &Target) -> Result<(), Self::Error>;
}

/// Validation trait specifically for generation requests
/// 
/// This is a convenience trait that pre-specifies the error type for generation
/// request validation to ensure consistency across all generation validators.
pub trait ValidatesGenerationRequest<Target>: Validator<Target, Error = ValidationError> {}

/// Blanket implementation for any validator that validates with ValidationError
impl<T, Target> ValidatesGenerationRequest<Target> for T 
where 
    T: Validator<Target, Error = ValidationError>
{}

/// Validation trait for tool calls
/// 
/// Similar convenience trait for tool call validation
pub trait ValidatesToolCall<Target>: Validator<Target, Error = ValidationError> {}

/// Blanket implementation for tool call validators
impl<T, Target> ValidatesToolCall<Target> for T
where
    T: Validator<Target, Error = ValidationError>
{}

/// Trait for composite validators that combine multiple validators
pub trait CompositeValidator<Target> {
    type Error;
    
    /// Add a validator to this composite
    fn add_validator<V>(&mut self, validator: V) 
    where 
        V: Validator<Target, Error = Self::Error> + Send + Sync + 'static;
        
    /// Validate using all contained validators
    fn validate_all(&self, session: &Session, target: &Target) -> Result<(), Self::Error>;
}
```

### Update `src/validation/errors.rs`

```rust
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
```

### Update `src/validation/mod.rs`

```rust
//! Validation system for agent requests and data
//! 
//! This module provides a trait-based validation system with modular,
//! composable validation logic and clear separation of concerns.

pub mod errors;
pub mod traits;
pub mod generation_request;
pub mod tool_call;

// Re-export main validation types
pub use errors::{ValidationError, ValidationResult};
pub use traits::{Validator, ValidatesGenerationRequest, ValidatesToolCall, CompositeValidator};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Session, Message, MessageRole};
    
    /// Simple test validator for testing the trait system
    struct TestValidator {
        should_fail: bool,
    }
    
    impl Validator<String> for TestValidator {
        type Error = ValidationError;
        
        fn validate(&self, _session: &Session, target: &String) -> Result<(), Self::Error> {
            if self.should_fail {
                Err(ValidationError::invalid_state(format!("Test failure for: {}", target)))
            } else {
                Ok(())
            }
        }
    }
    
    #[test]
    fn test_validator_trait_success() {
        let validator = TestValidator { should_fail: false };
        let session = Session::default();
        let target = "test input".to_string();
        
        assert!(validator.validate(&session, &target).is_ok());
    }
    
    #[test]
    fn test_validator_trait_failure() {
        let validator = TestValidator { should_fail: true };
        let session = Session::default();
        let target = "test input".to_string();
        
        let result = validator.validate(&session, &target);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Test failure"));
    }
    
    #[test]
    fn test_validation_error_constructors() {
        let error = ValidationError::security_violation("test security issue");
        assert!(matches!(error, ValidationError::SecurityViolation(_)));
        
        let error = ValidationError::parameter_bounds("test bounds issue");
        assert!(matches!(error, ValidationError::ParameterBounds(_)));
    }
    
    #[test]
    fn test_multiple_errors() {
        let errors = vec![
            ValidationError::security_violation("security issue"),
            ValidationError::parameter_bounds("bounds issue"),
        ];
        
        let combined = ValidationError::multiple(errors.clone());
        assert!(matches!(combined, ValidationError::Multiple(_)));
        
        // Single error should not be wrapped
        let single = ValidationError::multiple(vec![ValidationError::invalid_state("single")]);
        assert!(matches!(single, ValidationError::InvalidState(_)));
    }
}
```

## Integration Points
- Traits must work with existing `Session` and other types
- Error types must be convertible to existing `AgentError`
- All traits must be Send + Sync for async usage

## Testing
- [ ] All trait methods compile correctly
- [ ] Error type conversions work as expected
- [ ] Test validator implementations work correctly
- [ ] Documentation examples compile

## Notes
- This creates the foundation that all subsequent validators will build upon
- Focus on clean, composable trait design
- Error handling must be comprehensive but not overly complex