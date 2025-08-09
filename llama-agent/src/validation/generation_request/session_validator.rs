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
        Self
    }
}

impl Validator<GenerationRequest> for SessionStateValidator {
    type Error = ValidationError;

    fn validate(&self, session: &Session, _request: &GenerationRequest) -> ValidationResult {
        // Validate session has messages
        if session.messages.is_empty() {
            return Err(ValidationError::invalid_state(
                "Session must have at least one message for generation",
            ));
        }

        // Validate session timestamps are reasonable
        if session.created_at > session.updated_at {
            return Err(ValidationError::invalid_state(
                "Session created_at timestamp cannot be after updated_at",
            ));
        }

        // Note: SessionId is a ULID wrapper and cannot be empty by construction

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Message, MessageRole, SessionId};
    use std::time::{Duration, SystemTime};

    fn create_test_session() -> Session {
        Session {
            id: SessionId::new(),
            messages: vec![Message {
                role: MessageRole::User,
                content: "Hello".to_string(),
                tool_call_id: None,
                tool_name: None,
                timestamp: SystemTime::now(),
            }],
            mcp_servers: vec![],
            available_tools: vec![],
            available_prompts: vec![],
            created_at: SystemTime::now() - Duration::from_secs(10),
            updated_at: SystemTime::now(),
        }
    }

    fn create_test_request() -> GenerationRequest {
        GenerationRequest {
            session_id: SessionId::new(),
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
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("at least one message"));
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
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("created_at timestamp"));
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
    fn test_session_id_validation_always_passes() {
        let validator = SessionStateValidator::new();
        let session = create_test_session();
        let request = create_test_request();

        // SessionId is a ULID wrapper and cannot be invalid by construction
        // This test documents that session ID validation always passes for valid SessionIds
        let result = validator.validate(&session, &request);
        assert!(result.is_ok());
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
