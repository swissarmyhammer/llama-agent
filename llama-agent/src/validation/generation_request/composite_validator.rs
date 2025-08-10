//! Composite validator that combines all generation request validation logic

use super::{
    MessageContentConfig, MessageContentValidator, ParameterConfig, ParameterValidator,
    SessionStateValidator,
};
use crate::types::{GenerationRequest, Session};
use crate::validation::{ValidationError, ValidationResult, Validator};

/// Configuration for the composite generation request validator
#[derive(Debug, Clone, Default)]
pub struct ValidationConfig {
    /// Configuration for message content validation
    pub message_content: MessageContentConfig,
    /// Configuration for parameter validation
    pub parameters: ParameterConfig,
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
    use std::time::{Duration, SystemTime};

    fn create_test_session_with_messages(messages: Vec<Message>) -> Session {
        Session {
            id: SessionId::new(),
            messages,
            mcp_servers: vec![],
            available_tools: vec![],
            available_prompts: vec![],
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
            session_id: SessionId::new(),
            max_tokens: Some(150),
            temperature: Some(0.7),
            top_p: Some(0.9),
            stop_tokens: vec!["Human:".to_string()],
            stopping_config: None,
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
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("at least one message"));
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
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("unsafe content patterns"));
    }

    #[test]
    fn test_parameter_validation_failure() {
        let validator = CompositeGenerationRequestValidator::new();
        let session =
            create_test_session_with_messages(vec![create_valid_message("Valid message")]);
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
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("exceeds maximum length"));
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
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("at least one message"));
    }

    #[test]
    fn test_individual_validator_access() {
        let validator = CompositeGenerationRequestValidator::new();

        // Test that we can access individual validators
        let session = create_test_session_with_messages(vec![create_valid_message("test")]);
        let request = create_test_request();

        // Should be able to use individual validators directly
        assert!(validator
            .session_validator()
            .validate(&session, &request)
            .is_ok());
        assert!(validator
            .parameter_validator()
            .validate(&session, &request)
            .is_ok());

        for message in &session.messages {
            assert!(validator
                .message_validator()
                .validate(&session, message)
                .is_ok());
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

        let session = create_test_session_with_messages(vec![create_valid_message("test message")]);
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
            session_id: session.id,
            max_tokens: Some(500),
            temperature: Some(0.7),
            top_p: Some(0.9),
            stop_tokens: vec!["Human:".to_string(), "\n\n".to_string()],
            stopping_config: None,
        };

        assert!(validator.validate(&session, &request).is_ok());
    }

    #[test]
    fn test_edge_case_suspicious_content_in_middle_message() {
        let validator = CompositeGenerationRequestValidator::new();
        let session = create_test_session_with_messages(vec![
            create_valid_message("First message is safe"),
            create_valid_message("Second message is also safe"),
            create_valid_message("Third message has <script> injection"), // Bad message in middle
            create_valid_message("Fourth message is safe again"),
        ]);
        let request = create_test_request();

        let result = validator.validate(&session, &request);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("unsafe content patterns"));
    }

    #[test]
    fn test_validation_config_default() {
        let config = ValidationConfig::default();
        let validator = CompositeGenerationRequestValidator::with_config(config);

        // Should behave the same as new()
        let default_validator = CompositeGenerationRequestValidator::new();

        let session = create_test_session_with_messages(vec![create_valid_message("Test message")]);
        let request = create_test_request();

        assert_eq!(
            validator.validate(&session, &request).is_ok(),
            default_validator.validate(&session, &request).is_ok()
        );
    }

    #[test]
    fn test_clone_validator() {
        let validator1 = CompositeGenerationRequestValidator::new();
        let validator2 = validator1.clone();

        let session = create_test_session_with_messages(vec![create_valid_message("Test message")]);
        let request = create_test_request();

        // Both should behave identically
        assert_eq!(
            validator1.validate(&session, &request).is_ok(),
            validator2.validate(&session, &request).is_ok()
        );
    }

    #[test]
    fn test_empty_session_id_handling() {
        let validator = CompositeGenerationRequestValidator::new();

        // Note: SessionId is a ULID wrapper and cannot be empty by construction
        // This test documents that session ID validation is handled by the type system
        let session =
            create_test_session_with_messages(vec![create_valid_message("Valid message")]);
        let request = create_test_request();

        assert!(validator.validate(&session, &request).is_ok());
    }

    #[test]
    fn test_custom_suspicious_patterns() {
        let config = ValidationConfig {
            message_content: MessageContentConfig {
                max_length: 100_000,
                custom_suspicious_patterns: vec![
                    "FORBIDDEN_WORD".to_string(),
                    "another_bad_pattern".to_string(),
                ],
                repetition_threshold: 5,
            },
            parameters: ParameterConfig::default(),
        };

        let validator = CompositeGenerationRequestValidator::with_config(config);
        let session = create_test_session_with_messages(vec![create_valid_message(
            "This message contains FORBIDDEN_WORD which should trigger validation",
        )]);
        let request = create_test_request();

        let result = validator.validate(&session, &request);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("unsafe content patterns"));
    }
}
