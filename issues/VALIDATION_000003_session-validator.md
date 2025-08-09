# Implement SessionStateValidator

Refer to ./specification/validation.md

## Overview
Implement the `SessionStateValidator` which validates that a session is in a valid state for generation requests. This is the simplest validator and establishes the pattern for all subsequent validators.

## Acceptance Criteria
- [ ] Implement `SessionStateValidator` struct
- [ ] Implement `Validator<GenerationRequest>` for session validation
- [ ] Extract session validation logic from existing `agent.rs`
- [ ] Add comprehensive unit tests
- [ ] Document usage patterns

## Implementation Details

### Update `src/validation/generation_request/mod.rs`

```rust
//! Generation request validation components
//! 
//! This module contains all validators related to GenerationRequest validation.

mod session_validator;

pub use session_validator::SessionStateValidator;
```

### Create `src/validation/generation_request/session_validator.rs`

```rust
//! Session state validation for generation requests

use crate::types::{GenerationRequest, Session};
use crate::validation::{ValidationError, ValidationResult, Validator};

/// Validates that a session is in a valid state for generation
/// 
/// This validator ensures that:
/// - Session has at least one message for generation context
/// - Session is not in an invalid state
/// - Session metadata is valid
#[derive(Debug, Default, Clone)]
pub struct SessionStateValidator;

impl SessionStateValidator {
    /// Create a new session state validator
    pub fn new() -> Self {
        Self::default()
    }
}

impl Validator<GenerationRequest> for SessionStateValidator {
    type Error = ValidationError;
    
    fn validate(&self, session: &Session, _request: &GenerationRequest) -> ValidationResult {
        // Validate session has messages
        if session.messages.is_empty() {
            return Err(ValidationError::invalid_state(
                "Session must have at least one message for generation"
            ));
        }
        
        // Validate session timestamps are reasonable
        if session.created_at > session.updated_at {
            return Err(ValidationError::invalid_state(
                "Session created_at timestamp cannot be after updated_at"
            ));
        }
        
        // Validate session ID is not empty
        if session.id.trim().is_empty() {
            return Err(ValidationError::invalid_state(
                "Session ID cannot be empty"
            ));
        }
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Message, MessageRole, SessionId};
    use std::time::{SystemTime, Duration};
    
    fn create_test_session() -> Session {
        Session {
            id: SessionId("test-session".to_string()),
            messages: vec![
                Message {
                    role: MessageRole::User,
                    content: "Hello".to_string(),
                    tool_call_id: None,
                    tool_name: None,
                    timestamp: SystemTime::now(),
                }
            ],
            mcp_servers: vec![],
            available_tools: vec![],
            created_at: SystemTime::now() - Duration::from_secs(10),
            updated_at: SystemTime::now(),
        }
    }
    
    fn create_test_request() -> GenerationRequest {
        GenerationRequest {
            session_id: SessionId("test-session".to_string()),
            max_tokens: Some(100),
            temperature: Some(0.7),
            top_p: Some(0.9),
            stop_tokens: vec![],
        }
    }
    
    #[test]
    fn test_valid_session_passes() {
        let validator = SessionStateValidator::new();
        let session = create_test_session();
        let request = create_test_request();
        
        assert!(validator.validate(&session, &request).is_ok());
    }
    
    #[test]
    fn test_empty_messages_fails() {
        let validator = SessionStateValidator::new();
        let mut session = create_test_session();
        session.messages.clear();
        let request = create_test_request();
        
        let result = validator.validate(&session, &request);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("at least one message"));
    }
    
    #[test]
    fn test_invalid_timestamps_fail() {
        let validator = SessionStateValidator::new();
        let mut session = create_test_session();
        session.created_at = SystemTime::now();
        session.updated_at = SystemTime::now() - Duration::from_secs(10);
        let request = create_test_request();
        
        let result = validator.validate(&session, &request);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("created_at timestamp"));
    }
    
    #[test]
    fn test_empty_session_id_fails() {
        let validator = SessionStateValidator::new();
        let mut session = create_test_session();
        session.id = SessionId("  ".to_string()); // Whitespace only
        let request = create_test_request();
        
        let result = validator.validate(&session, &request);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Session ID cannot be empty"));
    }
    
    #[test]
    fn test_validator_is_default() {
        let validator1 = SessionStateValidator::default();
        let validator2 = SessionStateValidator::new();
        
        let session = create_test_session();
        let request = create_test_request();
        
        // Both should behave identically
        assert_eq!(
            validator1.validate(&session, &request).is_ok(),
            validator2.validate(&session, &request).is_ok()
        );
    }
    
    #[test]
    fn test_validator_is_clone() {
        let validator1 = SessionStateValidator::new();
        let validator2 = validator1.clone();
        
        let session = create_test_session();
        let request = create_test_request();
        
        // Both should behave identically
        assert_eq!(
            validator1.validate(&session, &request).is_ok(),
            validator2.validate(&session, &request).is_ok()
        );
    }
}
```

### Update `src/validation/generation_request/mod.rs`

```rust
//! Generation request validation components
//! 
//! This module contains all validators related to GenerationRequest validation.

mod session_validator;

pub use session_validator::SessionStateValidator;

#[cfg(test)]
mod integration_tests {
    use super::*;
    use crate::types::{GenerationRequest, Session, Message, MessageRole, SessionId};
    use crate::validation::Validator;
    use std::time::SystemTime;
    
    #[test]
    fn test_session_validator_integration() {
        let validator = SessionStateValidator::new();
        
        // Create a realistic session
        let session = Session {
            id: SessionId("integration-test".to_string()),
            messages: vec![
                Message {
                    role: MessageRole::User,
                    content: "What is the weather like?".to_string(),
                    tool_call_id: None,
                    tool_name: None,
                    timestamp: SystemTime::now(),
                }
            ],
            mcp_servers: vec![],
            available_tools: vec![],
            created_at: SystemTime::now(),
            updated_at: SystemTime::now(),
        };
        
        // Create a realistic generation request
        let request = GenerationRequest {
            session_id: session.id.clone(),
            max_tokens: Some(150),
            temperature: Some(0.8),
            top_p: None,
            stop_tokens: vec!["Human:".to_string()],
        };
        
        // Validation should pass
        assert!(validator.validate(&session, &request).is_ok());
    }
}
```

## Integration Points
- Uses existing `Session` and `GenerationRequest` types
- Follows established validation error patterns
- Prepares for integration with composite validators

## Testing
- [ ] Unit tests cover all validation scenarios
- [ ] Integration tests demonstrate real-world usage
- [ ] Error messages are clear and actionable
- [ ] Validator traits work correctly

## Notes
- This validator establishes the pattern for all other validators
- Focus on clear, descriptive error messages
- Should be stateless and thread-safe
- Extracted logic should match or improve upon current session validation in agent.rs

## Proposed Solution

I will implement the `SessionStateValidator` following the specification exactly as detailed in the validation.md. The implementation will:

1. **Create `src/validation/generation_request/session_validator.rs`** containing:
   - `SessionStateValidator` struct implementing `Validator<GenerationRequest>`
   - Validation logic for session state including:
     - Session has at least one message
     - Timestamps are logically consistent (created_at <= updated_at)
     - Session ID is not empty or whitespace
   - Comprehensive unit tests covering all validation scenarios
   - Integration test demonstrating real-world usage

2. **Update `src/validation/generation_request/mod.rs`** to:
   - Export the `SessionStateValidator` 
   - Add integration tests for the validator

This establishes the pattern for all subsequent validators and extracts session validation logic from the existing `agent.rs` while following the established trait-based architecture already implemented in the validation module.

The implementation follows TDD principles and includes all error cases specified in the issue requirements.