# Implement MessageContentValidator

Refer to ./specification/validation.md

## Overview
Implement the `MessageContentValidator` which validates message content for security issues, length limits, and suspicious patterns. This extracts and refactors the existing suspicious content detection and repetition checking logic from `agent.rs`.

## Acceptance Criteria
- [ ] Extract existing content validation logic from `agent.rs`
- [ ] Implement `MessageContentValidator` struct with configurable limits
- [ ] Implement `Validator<Message>` trait
- [ ] Preserve all existing security checks
- [ ] Add comprehensive unit tests
- [ ] Make validation rules configurable

## Implementation Details

### Create `src/validation/generation_request/message_validator.rs`

```rust
//! Message content validation for generation requests

use crate::types::{Message, Session};
use crate::validation::{ValidationError, ValidationResult, Validator};

/// Configuration for message content validation
#[derive(Debug, Clone)]
pub struct MessageContentConfig {
    /// Maximum allowed message length in characters
    pub max_length: usize,
    /// Custom suspicious patterns to check for
    pub custom_suspicious_patterns: Vec<String>,
    /// Repetition threshold for detecting spam
    pub repetition_threshold: usize,
}

impl Default for MessageContentConfig {
    fn default() -> Self {
        Self {
            max_length: 100_000,
            custom_suspicious_patterns: Vec::new(),
            repetition_threshold: 5,
        }
    }
}

/// Validates message content for security and quality issues
/// 
/// This validator performs:
/// - Length validation to prevent DoS attacks
/// - Suspicious pattern detection for security
/// - Excessive repetition detection for spam prevention
#[derive(Debug, Clone)]
pub struct MessageContentValidator {
    config: MessageContentConfig,
    suspicious_patterns: Vec<String>,
}

impl MessageContentValidator {
    /// Create a new message content validator with default settings
    pub fn new() -> Self {
        Self::with_config(MessageContentConfig::default())
    }
    
    /// Create a validator with custom configuration
    pub fn with_config(config: MessageContentConfig) -> Self {
        let mut validator = Self {
            suspicious_patterns: Self::default_suspicious_patterns(),
            config,
        };
        
        // Add custom patterns
        validator.suspicious_patterns.extend(validator.config.custom_suspicious_patterns.clone());
        validator
    }
    
    /// Get the default list of suspicious patterns
    fn default_suspicious_patterns() -> Vec<String> {
        vec![
            // Script injection patterns
            "<script".to_string(),
            "</script>".to_string(),
            "javascript:".to_string(),
            "eval(".to_string(),
            "function(".to_string(),
            
            // Template injection patterns
            "${{".to_string(),
            "}}".to_string(),
            "<%".to_string(),
            "%>".to_string(),
            "<?php".to_string(),
            "?>".to_string(),
            
            // Command injection patterns
            "rm -rf".to_string(),
            
            // SQL injection patterns
            "DELETE FROM".to_string(),
            "DROP TABLE".to_string(),
            "INSERT INTO".to_string(),
            
            // Path traversal patterns
            "../../../".to_string(),
            "..\\..\\..\\".to_string(),
        ]
    }
    
    /// Check if content contains suspicious patterns
    /// 
    /// This method is extracted from the existing `contains_suspicious_content`
    /// function in agent.rs
    fn contains_suspicious_content(&self, content: &str) -> bool {
        let content_lower = content.to_lowercase();
        self.suspicious_patterns
            .iter()
            .any(|pattern| content_lower.contains(&pattern.to_lowercase()))
    }
    
    /// Check for excessive repetition that might indicate spam/DoS
    /// 
    /// This method is extracted from the existing `has_excessive_repetition`
    /// function in agent.rs  
    fn has_excessive_repetition(&self, content: &str) -> bool {
        if content.len() < 100 {
            return false; // Short content is fine
        }
        
        // Check for repeated substrings
        let chars: Vec<char> = content.chars().collect();
        let len = chars.len();
        
        // Check for repeated 4-char patterns
        if len >= 20 {
            for i in 0..=(len - 20) {
                let pattern = &chars[i..i + 4];
                let mut count = 1;
                
                for j in ((i + 4)..=(len - 4)).step_by(4) {
                    if &chars[j..j + 4] == pattern {
                        count += 1;
                        if count >= self.config.repetition_threshold {
                            return true;
                        }
                    }
                }
            }
        }
        
        false
    }
}

impl Default for MessageContentValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl Validator<Message> for MessageContentValidator {
    type Error = ValidationError;
    
    fn validate(&self, _session: &Session, message: &Message) -> ValidationResult {
        // DoS protection: limit message size
        if message.content.len() > self.config.max_length {
            return Err(ValidationError::security_violation(format!(
                "Message exceeds maximum length of {}KB (current: {}KB)",
                self.config.max_length / 1000,
                message.content.len() / 1000
            )));
        }
        
        // Security: Check for potentially malicious content patterns
        if self.contains_suspicious_content(&message.content) {
            return Err(ValidationError::security_violation(
                "Message contains potentially unsafe content patterns"
            ));
        }
        
        // Security: Check for excessive repetition
        if self.has_excessive_repetition(&message.content) {
            return Err(ValidationError::security_violation(
                "Message contains excessive repetition patterns"
            ));
        }
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{MessageRole, SessionId};
    use std::time::SystemTime;
    
    fn create_test_message(content: &str) -> Message {
        Message {
            role: MessageRole::User,
            content: content.to_string(),
            tool_call_id: None,
            tool_name: None,
            timestamp: SystemTime::now(),
        }
    }
    
    fn create_test_session() -> Session {
        Session {
            id: SessionId("test".to_string()),
            messages: vec![],
            mcp_servers: vec![],
            available_tools: vec![],
            created_at: SystemTime::now(),
            updated_at: SystemTime::now(),
        }
    }
    
    #[test]
    fn test_valid_message_passes() {
        let validator = MessageContentValidator::new();
        let session = create_test_session();
        let message = create_test_message("Hello, how are you today?");
        
        assert!(validator.validate(&session, &message).is_ok());
    }
    
    #[test]
    fn test_message_too_long_fails() {
        let validator = MessageContentValidator::new();
        let session = create_test_session();
        let long_content = "a".repeat(100_001); // Just over the limit
        let message = create_test_message(&long_content);
        
        let result = validator.validate(&session, &message);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("exceeds maximum length"));
    }
    
    #[test]
    fn test_suspicious_patterns_fail() {
        let validator = MessageContentValidator::new();
        let session = create_test_session();
        
        let suspicious_contents = vec![
            "<script>alert('xss')</script>",
            "javascript:alert('xss')",
            "rm -rf /",
            "DELETE FROM users",
            "../../../etc/passwd",
            "eval(user_input)",
        ];
        
        for suspicious_content in suspicious_contents {
            let message = create_test_message(suspicious_content);
            let result = validator.validate(&session, &message);
            assert!(result.is_err(), "Should fail for: {}", suspicious_content);
            assert!(result.unwrap_err().to_string().contains("unsafe content patterns"));
        }
    }
    
    #[test]
    fn test_excessive_repetition_fails() {
        let validator = MessageContentValidator::new();
        let session = create_test_session();
        
        // Create content with excessive repetition
        let repetitive_content = "abcd".repeat(10); // 5+ repetitions of "abcd"
        let message = create_test_message(&repetitive_content);
        
        let result = validator.validate(&session, &message);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("excessive repetition"));
    }
    
    #[test]
    fn test_short_repetitive_content_passes() {
        let validator = MessageContentValidator::new();
        let session = create_test_session();
        
        // Short repetitive content should be allowed
        let short_content = "abcd".repeat(5); // Less than 100 chars
        let message = create_test_message(&short_content);
        
        assert!(validator.validate(&session, &message).is_ok());
    }
    
    #[test]
    fn test_custom_config() {
        let config = MessageContentConfig {
            max_length: 1000,
            custom_suspicious_patterns: vec!["custom_bad".to_string()],
            repetition_threshold: 3,
        };
        
        let validator = MessageContentValidator::with_config(config);
        let session = create_test_session();
        
        // Test custom pattern detection
        let message = create_test_message("This contains custom_bad content");
        let result = validator.validate(&session, &message);
        assert!(result.is_err());
        
        // Test custom length limit
        let long_message = create_test_message(&"a".repeat(1001));
        let result = validator.validate(&session, &long_message);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("1KB"));
    }
    
    #[test]
    fn test_case_insensitive_pattern_matching() {
        let validator = MessageContentValidator::new();
        let session = create_test_session();
        
        // Test uppercase patterns
        let message = create_test_message("This has <SCRIPT> tag");
        let result = validator.validate(&session, &message);
        assert!(result.is_err());
        
        // Test mixed case
        let message = create_test_message("This has <ScRiPt> tag");
        let result = validator.validate(&session, &message);
        assert!(result.is_err());
    }
    
    #[test]
    fn test_validator_is_cloneable() {
        let validator1 = MessageContentValidator::new();
        let validator2 = validator1.clone();
        
        let session = create_test_session();
        let message = create_test_message("test message");
        
        // Both should work identically
        assert_eq!(
            validator1.validate(&session, &message).is_ok(),
            validator2.validate(&session, &message).is_ok()
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
mod message_validator;

pub use session_validator::SessionStateValidator;
pub use message_validator::{MessageContentValidator, MessageContentConfig};
```

## Integration Points
- Extract exact logic from existing `agent.rs` methods:
  - `contains_suspicious_content` (lines 423-448)
  - `has_excessive_repetition` (lines 451-479)
- Maintain all existing security patterns
- Make configuration easily customizable

## Testing
- [ ] Test all existing suspicious patterns still work
- [ ] Test length validation with various sizes
- [ ] Test repetition detection with edge cases
- [ ] Test custom configuration options
- [ ] Test case-insensitive pattern matching

## Notes
- This preserves all existing security functionality while making it more modular
- Configuration allows for different security policies per deployment
- Extracted logic should have identical behavior to current implementation