//! Generation request validation components
//!
//! This module contains all validators related to GenerationRequest validation.

mod parameter_validator;
mod session_validator;

pub use parameter_validator::{ParameterConfig, ParameterValidator};
pub use session_validator::SessionStateValidator;

#[cfg(test)]
mod integration_tests {
    use super::*;
    use crate::types::{GenerationRequest, Message, MessageRole, Session, SessionId};
    use crate::validation::Validator;
    use std::time::SystemTime;

    #[test]
    fn test_session_validator_integration() {
        let validator = SessionStateValidator::new();

        // Create a realistic session
        let session = Session {
            id: SessionId::new(),
            messages: vec![Message {
                role: MessageRole::User,
                content: "What is the weather like?".to_string(),
                tool_call_id: None,
                tool_name: None,
                timestamp: SystemTime::now(),
            }],
            mcp_servers: vec![],
            available_tools: vec![],
            available_prompts: vec![],
            created_at: SystemTime::now(),
            updated_at: SystemTime::now(),
        };

        // Create a realistic generation request
        let request = GenerationRequest {
            session_id: session.id,
            max_tokens: Some(150),
            temperature: Some(0.8),
            top_p: None,
            stop_tokens: vec!["Human:".to_string()],
        };

        // Validation should pass
        assert!(validator.validate(&session, &request).is_ok());
    }
}
