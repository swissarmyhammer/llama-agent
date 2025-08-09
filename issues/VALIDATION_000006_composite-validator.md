# Create CompositeGenerationRequestValidator

Refer to ./specification/validation.md

## Overview
Implement the `CompositeGenerationRequestValidator` which combines all individual validators (session, message, parameter) into a single validator that can validate complete generation requests. This provides a unified interface for complete generation request validation.

## Acceptance Criteria
- [ ] Implement `CompositeGenerationRequestValidator` struct
- [ ] Combine SessionStateValidator, MessageContentValidator, and ParameterValidator
- [ ] Implement `Validator<GenerationRequest>` trait
- [ ] Validate all messages in session using MessageContentValidator
- [ ] Provide easy configuration for all sub-validators
- [ ] Add comprehensive unit and integration tests

## Implementation Details

### Create `src/validation/generation_request/composite_validator.rs`

```rust
//! Composite validator that combines all generation request validation logic

use crate::types::{GenerationRequest, Session};
use crate::validation::{ValidationError, ValidationResult, Validator};
use super::{
    SessionStateValidator,
    MessageContentValidator, MessageContentConfig,
    ParameterValidator, ParameterConfig,
};

/// Configuration for the composite generation request validator
#[derive(Debug, Clone)]
pub struct ValidationConfig {
    /// Configuration for message content validation
    pub message_content: MessageContentConfig,
    /// Configuration for parameter validation
    pub parameters: ParameterConfig,
}

impl Default for ValidationConfig {
    fn default() -> Self {
        Self {
            message_content: MessageContentConfig::default(),
            parameters: ParameterConfig::default(),
        }
    }
}

/// Composite validator that performs comprehensive validation of generation requests
/// 
/// This validator combines:
/// - Session state validation (ensures session is valid for generation)
/// - Message content validation (validates all messages in session)
/// - Parameter validation (validates generation parameters)
/// 
/// This provides a single entry point for complete generation request validation.
#[derive(Debug, Clone)]
pub struct CompositeGenerationRequestValidator {
    session_validator: SessionStateValidator,
    message_validator: MessageContentValidator,
    parameter_validator: ParameterValidator,
}

impl CompositeGenerationRequestValidator {
    /// Create a new composite validator with default configuration
    pub fn new() -> Self {
        Self::with_config(ValidationConfig::default())
    }
    
    /// Create a composite validator with custom configuration
    pub fn with_config(config: ValidationConfig) -> Self {
        Self {
            session_validator: SessionStateValidator::new(),
            message_validator: MessageContentValidator::with_config(config.message_content),
            parameter_validator: ParameterValidator::with_config(config.parameters),
        }
    }
    
    /// Create a composite validator with individual validator configurations
    pub fn with_validators(
        session_validator: SessionStateValidator,
        message_validator: MessageContentValidator,
        parameter_validator: ParameterValidator,
    ) -> Self {
        Self {
            session_validator,
            message_validator,
            parameter_validator,
        }
    }
    
    /// Get a reference to the session validator
    pub fn session_validator(&self) -> &SessionStateValidator {
        &self.session_validator
    }
    
    /// Get a reference to the message validator
    pub fn message_validator(&self) -> &MessageContentValidator {
        &self.message_validator
    }
    
    /// Get a reference to the parameter validator
    pub fn parameter_validator(&self) -> &ParameterValidator {
        &self.parameter_validator
    }
}

impl Default for CompositeGenerationRequestValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl Validator<GenerationRequest> for CompositeGenerationRequestValidator {
    type Error = ValidationError;
    
    fn validate(&self, session: &Session, request: &GenerationRequest) -> ValidationResult {
        // Step 1: Validate session state
        self.session_validator.validate(session, request)?;
        
        // Step 2: Validate all messages in session
        for message in &session.messages {
            self.message_validator.validate(session, message)?;
        }
        
        // Step 3: Validate generation parameters
        self.parameter_validator.validate(session, request)?;
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Message, MessageRole, SessionId};
    use std::time::{SystemTime, Duration};
    
    fn create_test_session_with_messages(messages: Vec<Message>) -> Session {
        Session {
            id: SessionId("test-composite".to_string()),
            messages,
            mcp_servers: vec![],
            available_tools: vec![],
            created_at: SystemTime::now() - Duration::from_secs(10),
            updated_at: SystemTime::now(),
        }
    }
    
    fn create_valid_message(content: &str) -> Message {
        Message {
            role: MessageRole::User,
            content: content.to_string(),
            tool_call_id: None,
            tool_name: None,
            timestamp: SystemTime::now(),
        }
    }
    
    fn create_test_request() -> GenerationRequest {
        GenerationRequest {
            session_id: SessionId("test-composite".to_string()),
            max_tokens: Some(150),
            temperature: Some(0.7),
            top_p: Some(0.9),
            stop_tokens: vec!["Human:".to_string()],
        }
    }
    
    #[test]
    fn test_valid_complete_request_passes() {
        let validator = CompositeGenerationRequestValidator::new();
        let session = create_test_session_with_messages(vec![
            create_valid_message("Hello, how are you?"),
            create_valid_message("What can you help me with today?"),
        ]);
        let request = create_test_request();
        
        assert!(validator.validate(&session, &request).is_ok());
    }
    
    #[test]
    fn test_session_validation_failure() {
        let validator = CompositeGenerationRequestValidator::new();
        // Empty messages should fail session validation
        let session = create_test_session_with_messages(vec![]);
        let request = create_test_request();
        
        let result = validator.validate(&session, &request);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("at least one message"));
    }
    
    #[test]
    fn test_message_content_validation_failure() {
        let validator = CompositeGenerationRequestValidator::new();
        let session = create_test_session_with_messages(vec![
            create_valid_message("Normal message"),
            create_valid_message("<script>alert('xss')</script>"), // Suspicious content
        ]);
        let request = create_test_request();
        
        let result = validator.validate(&session, &request);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("unsafe content patterns"));
    }
    
    #[test]
    fn test_parameter_validation_failure() {
        let validator = CompositeGenerationRequestValidator::new();
        let session = create_test_session_with_messages(vec![
            create_valid_message("Valid message"),
        ]);
        let mut request = create_test_request();
        request.max_tokens = Some(0); // Invalid parameter
        
        let result = validator.validate(&session, &request);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("greater than 0"));
    }
    
    #[test]
    fn test_multiple_message_validation() {
        let validator = CompositeGenerationRequestValidator::new();
        let session = create_test_session_with_messages(vec![
            create_valid_message("First valid message"),
            create_valid_message("Second valid message"),
            create_valid_message("Third valid message"),
        ]);
        let request = create_test_request();
        
        assert!(validator.validate(&session, &request).is_ok());
    }
    
    #[test]
    fn test_custom_configuration() {
        let config = ValidationConfig {
            message_content: MessageContentConfig {
                max_length: 100, // Very short limit
                custom_suspicious_patterns: vec!["custom_bad".to_string()],
                repetition_threshold: 3,
            },
            parameters: ParameterConfig {
                max_tokens_limit: 1000,
                temperature_range: (0.1, 1.0),
                ..Default::default()
            },
        };
        
        let validator = CompositeGenerationRequestValidator::with_config(config);
        let session = create_test_session_with_messages(vec![
            create_valid_message(&"a".repeat(101)), // Over custom length limit
        ]);
        let request = create_test_request();
        
        let result = validator.validate(&session, &request);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("exceeds maximum length"));
    }
    
    #[test]
    fn test_validation_order() {
        // Test that validation fails at the first error encountered
        let validator = CompositeGenerationRequestValidator::new();
        
        // Create session with no messages (should fail session validation first)
        let session = create_test_session_with_messages(vec![]);
        let mut request = create_test_request();
        request.max_tokens = Some(0); // This would also fail, but session validation is first
        
        let result = validator.validate(&session, &request);
        assert!(result.is_err());
        // Should fail on session validation, not parameter validation
        assert!(result.unwrap_err().to_string().contains("at least one message"));
    }
    
    #[test]
    fn test_individual_validator_access() {
        let validator = CompositeGenerationRequestValidator::new();
        
        // Test that we can access individual validators
        let session = create_test_session_with_messages(vec![
            create_valid_message("test"),
        ]);
        let request = create_test_request();
        
        // Should be able to use individual validators directly
        assert!(validator.session_validator().validate(&session, &request).is_ok());
        assert!(validator.parameter_validator().validate(&session, &request).is_ok());
        
        for message in &session.messages {
            assert!(validator.message_validator().validate(&session, message).is_ok());
        }
    }
    
    #[test]
    fn test_with_validators_constructor() {
        let session_validator = SessionStateValidator::new();
        let message_validator = MessageContentValidator::new();
        let parameter_validator = ParameterValidator::new();
        
        let composite = CompositeGenerationRequestValidator::with_validators(
            session_validator,
            message_validator,
            parameter_validator,
        );
        
        let session = create_test_session_with_messages(vec![
            create_valid_message("test message"),
        ]);
        let request = create_test_request();
        
        assert!(composite.validate(&session, &request).is_ok());
    }
    
    #[test]
    fn test_realistic_generation_scenario() {
        let validator = CompositeGenerationRequestValidator::new();
        
        // Create a realistic chat session
        let session = create_test_session_with_messages(vec![
            Message {
                role: MessageRole::System,
                content: "You are a helpful AI assistant.".to_string(),
                tool_call_id: None,
                tool_name: None,
                timestamp: SystemTime::now() - Duration::from_secs(60),
            },
            Message {
                role: MessageRole::User,
                content: "Can you help me write a Python function to calculate fibonacci numbers?".to_string(),
                tool_call_id: None,
                tool_name: None,
                timestamp: SystemTime::now() - Duration::from_secs(30),
            },
            Message {
                role: MessageRole::Assistant,
                content: "I'd be happy to help you write a Fibonacci function! Here's a simple recursive implementation:".to_string(),
                tool_call_id: None,
                tool_name: None,
                timestamp: SystemTime::now() - Duration::from_secs(15),
            },
        ]);
        
        let request = GenerationRequest {
            session_id: session.id.clone(),
            max_tokens: Some(500),
            temperature: Some(0.7),
            top_p: Some(0.9),
            stop_tokens: vec!["Human:".to_string(), "\n\n".to_string()],
        };
        
        assert!(validator.validate(&session, &request).is_ok());
    }
}
```

### Update `src/validation/generation_request/mod.rs`

```rust
//! Generation request validation components
//! 
//! This module contains all validators related to GenerationRequest validation.

mod session_validator;
mod message_validator;
mod parameter_validator;
mod composite_validator;

pub use session_validator::SessionStateValidator;
pub use message_validator::{MessageContentValidator, MessageContentConfig};
pub use parameter_validator::{ParameterValidator, ParameterConfig};
pub use composite_validator::{CompositeGenerationRequestValidator, ValidationConfig};

#[cfg(test)]
mod integration_tests {
    use super::*;
    use crate::types::{GenerationRequest, Session, Message, MessageRole, SessionId};
    use crate::validation::Validator;
    use std::time::{SystemTime, Duration};
    
    #[test]
    fn test_full_generation_request_validation_pipeline() {
        // Test the complete validation pipeline with realistic data
        let validator = CompositeGenerationRequestValidator::new();
        
        let session = Session {
            id: SessionId("integration-test-session".to_string()),
            messages: vec![
                Message {
                    role: MessageRole::System,
                    content: "You are a helpful assistant specializing in programming help.".to_string(),
                    tool_call_id: None,
                    tool_name: None,
                    timestamp: SystemTime::now() - Duration::from_secs(120),
                },
                Message {
                    role: MessageRole::User,
                    content: "I need help with error handling in Rust. Can you show me best practices?".to_string(),
                    tool_call_id: None,
                    tool_name: None,
                    timestamp: SystemTime::now() - Duration::from_secs(60),
                },
                Message {
                    role: MessageRole::Assistant,
                    content: "I'd be happy to help with Rust error handling! Here are the key best practices:".to_string(),
                    tool_call_id: None,
                    tool_name: None,
                    timestamp: SystemTime::now() - Duration::from_secs(30),
                },
            ],
            mcp_servers: vec![],
            available_tools: vec![],
            created_at: SystemTime::now() - Duration::from_secs(180),
            updated_at: SystemTime::now() - Duration::from_secs(30),
        };
        
        let request = GenerationRequest {
            session_id: session.id.clone(),
            max_tokens: Some(800),
            temperature: Some(0.8),
            top_p: Some(0.95),
            stop_tokens: vec!["User:".to_string(), "Human:".to_string()],
        };
        
        // This should pass all validation stages
        let result = validator.validate(&session, &request);
        assert!(result.is_ok(), "Validation failed: {:?}", result);
    }
}
```

## Integration Points
- Combines all existing generation request validation logic
- Provides single entry point equivalent to current `validate_generation_request_with_session`
- Maintains all existing security and bounds checking
- Allows fine-grained configuration of all validation aspects

## Testing
- [ ] Test complete validation pipeline with realistic data
- [ ] Test validation failure at each stage (session, messages, parameters)
- [ ] Test custom configuration scenarios
- [ ] Test validation order and error reporting
- [ ] Test individual validator access

## Notes
- This creates the main validation entry point that will replace existing validation
- Validates all messages in session, not just the current request
- Maintains identical security behavior to current implementation
- Provides foundation for easy migration from current validation methods