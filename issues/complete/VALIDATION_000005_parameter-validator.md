# Implement ParameterValidator

Refer to ./specification/validation.md

## Overview
Implement the `ParameterValidator` which validates generation parameters like max_tokens, temperature, top_p, and stop_tokens. This extracts and refactors the existing parameter validation logic from `agent.rs`.

## Acceptance Criteria
- [ ] Extract existing parameter validation logic from `agent.rs`
- [ ] Implement `ParameterValidator` struct with configurable limits
- [ ] Implement `Validator<GenerationRequest>` trait
- [ ] Preserve all existing parameter bounds checking
- [ ] Add comprehensive unit tests
- [ ] Make validation limits configurable

## Implementation Details

### Create `src/validation/generation_request/parameter_validator.rs`

```rust
//! Generation parameter validation for requests

use crate::types::{GenerationRequest, Session};
use crate::validation::{ValidationError, ValidationResult, Validator};

/// Configuration for generation parameter validation
#[derive(Debug, Clone)]
pub struct ParameterConfig {
    /// Maximum allowed max_tokens value
    pub max_tokens_limit: u32,
    /// Valid range for temperature parameter (min, max)
    pub temperature_range: (f32, f32),
    /// Valid range for top_p parameter (min, max)
    pub top_p_range: (f32, f32),
    /// Maximum number of stop tokens allowed
    pub max_stop_tokens: usize,
    /// Maximum length for individual stop tokens
    pub max_stop_token_length: usize,
}

impl Default for ParameterConfig {
    fn default() -> Self {
        Self {
            max_tokens_limit: 32_768,
            temperature_range: (0.0, 2.0),
            top_p_range: (0.0, 1.0),
            max_stop_tokens: 20,
            max_stop_token_length: 100,
        }
    }
}

/// Validates generation request parameters for security and bounds
/// 
/// This validator performs:
/// - max_tokens bounds checking and security limits
/// - temperature validation for finite values and ranges  
/// - top_p validation for finite values and ranges
/// - stop tokens count and length validation
#[derive(Debug, Clone)]
pub struct ParameterValidator {
    config: ParameterConfig,
}

impl ParameterValidator {
    /// Create a new parameter validator with default limits
    pub fn new() -> Self {
        Self::with_config(ParameterConfig::default())
    }
    
    /// Create a validator with custom parameter configuration
    pub fn with_config(config: ParameterConfig) -> Self {
        Self { config }
    }
    
    /// Get the current configuration
    pub fn config(&self) -> &ParameterConfig {
        &self.config
    }
    
    /// Validate max_tokens parameter
    fn validate_max_tokens(&self, max_tokens: Option<u32>) -> ValidationResult {
        if let Some(max_tokens) = max_tokens {
            if max_tokens == 0 {
                return Err(ValidationError::parameter_bounds(
                    "max_tokens must be greater than 0"
                ));
            }
            if max_tokens > self.config.max_tokens_limit {
                return Err(ValidationError::security_violation(format!(
                    "max_tokens exceeds security limit of {} (requested: {})",
                    self.config.max_tokens_limit, max_tokens
                )));
            }
        }
        Ok(())
    }
    
    /// Validate temperature parameter
    fn validate_temperature(&self, temperature: Option<f32>) -> ValidationResult {
        if let Some(temp) = temperature {
            if !temp.is_finite() {
                return Err(ValidationError::parameter_bounds(
                    "temperature must be a finite number"
                ));
            }
            if !(self.config.temperature_range.0..=self.config.temperature_range.1).contains(&temp) {
                return Err(ValidationError::parameter_bounds(format!(
                    "temperature must be between {} and {} (got: {})",
                    self.config.temperature_range.0, self.config.temperature_range.1, temp
                )));
            }
        }
        Ok(())
    }
    
    /// Validate top_p parameter
    fn validate_top_p(&self, top_p: Option<f32>) -> ValidationResult {
        if let Some(top_p) = top_p {
            if !top_p.is_finite() {
                return Err(ValidationError::parameter_bounds(
                    "top_p must be a finite number"
                ));
            }
            if !(self.config.top_p_range.0..=self.config.top_p_range.1).contains(&top_p) {
                return Err(ValidationError::parameter_bounds(format!(
                    "top_p must be between {} and {} (got: {})",
                    self.config.top_p_range.0, self.config.top_p_range.1, top_p
                )));
            }
        }
        Ok(())
    }
    
    /// Validate stop tokens
    fn validate_stop_tokens(&self, stop_tokens: &[String]) -> ValidationResult {
        // Security: Validate stop token count
        if stop_tokens.len() > self.config.max_stop_tokens {
            return Err(ValidationError::security_violation(format!(
                "Too many stop tokens: {} (max {} allowed)",
                stop_tokens.len(), self.config.max_stop_tokens
            )));
        }
        
        // Security: Validate individual stop token length
        for (i, stop_token) in stop_tokens.iter().enumerate() {
            if stop_token.len() > self.config.max_stop_token_length {
                return Err(ValidationError::security_violation(format!(
                    "Stop token {} exceeds maximum length of {} chars",
                    i, self.config.max_stop_token_length
                )));
            }
        }
        
        Ok(())
    }
}

impl Default for ParameterValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl Validator<GenerationRequest> for ParameterValidator {
    type Error = ValidationError;
    
    fn validate(&self, _session: &Session, request: &GenerationRequest) -> ValidationResult {
        // Validate max_tokens
        self.validate_max_tokens(request.max_tokens)?;
        
        // Validate temperature
        self.validate_temperature(request.temperature)?;
        
        // Validate top_p
        self.validate_top_p(request.top_p)?;
        
        // Validate stop tokens
        self.validate_stop_tokens(&request.stop_tokens)?;
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{SessionId};
    use std::time::SystemTime;
    
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
    
    fn create_test_request() -> GenerationRequest {
        GenerationRequest {
            session_id: SessionId("test".to_string()),
            max_tokens: Some(100),
            temperature: Some(0.7),
            top_p: Some(0.9),
            stop_tokens: vec!["Human:".to_string()],
        }
    }
    
    #[test]
    fn test_valid_parameters_pass() {
        let validator = ParameterValidator::new();
        let session = create_test_session();
        let request = create_test_request();
        
        assert!(validator.validate(&session, &request).is_ok());
    }
    
    #[test]
    fn test_max_tokens_validation() {
        let validator = ParameterValidator::new();
        let session = create_test_session();
        
        // Test zero max_tokens
        let mut request = create_test_request();
        request.max_tokens = Some(0);
        let result = validator.validate(&session, &request);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("greater than 0"));
        
        // Test excessive max_tokens
        request.max_tokens = Some(50_000);
        let result = validator.validate(&session, &request);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("exceeds security limit"));
        
        // Test None max_tokens (should pass)
        request.max_tokens = None;
        assert!(validator.validate(&session, &request).is_ok());
    }
    
    #[test]
    fn test_temperature_validation() {
        let validator = ParameterValidator::new();
        let session = create_test_session();
        let mut request = create_test_request();
        
        // Test infinite temperature
        request.temperature = Some(f32::INFINITY);
        let result = validator.validate(&session, &request);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("finite number"));
        
        // Test NaN temperature
        request.temperature = Some(f32::NAN);
        let result = validator.validate(&session, &request);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("finite number"));
        
        // Test out of range temperature (too low)
        request.temperature = Some(-0.1);
        let result = validator.validate(&session, &request);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("between 0.0 and 2.0"));
        
        // Test out of range temperature (too high)
        request.temperature = Some(2.1);
        let result = validator.validate(&session, &request);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("between 0.0 and 2.0"));
        
        // Test None temperature (should pass)
        request.temperature = None;
        assert!(validator.validate(&session, &request).is_ok());
    }
    
    #[test]
    fn test_top_p_validation() {
        let validator = ParameterValidator::new();
        let session = create_test_session();
        let mut request = create_test_request();
        
        // Test infinite top_p
        request.top_p = Some(f32::INFINITY);
        let result = validator.validate(&session, &request);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("finite number"));
        
        // Test out of range top_p (too low)
        request.top_p = Some(-0.1);
        let result = validator.validate(&session, &request);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("between 0.0 and 1.0"));
        
        // Test out of range top_p (too high)
        request.top_p = Some(1.1);
        let result = validator.validate(&session, &request);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("between 0.0 and 1.0"));
        
        // Test None top_p (should pass)
        request.top_p = None;
        assert!(validator.validate(&session, &request).is_ok());
    }
    
    #[test]
    fn test_stop_tokens_validation() {
        let validator = ParameterValidator::new();
        let session = create_test_session();
        let mut request = create_test_request();
        
        // Test too many stop tokens
        request.stop_tokens = (0..25).map(|i| format!("stop{}", i)).collect();
        let result = validator.validate(&session, &request);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Too many stop tokens"));
        
        // Test stop token too long
        request.stop_tokens = vec!["a".repeat(101)];
        let result = validator.validate(&session, &request);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("exceeds maximum length"));
        
        // Test empty stop tokens (should pass)
        request.stop_tokens = vec![];
        assert!(validator.validate(&session, &request).is_ok());
    }
    
    #[test]
    fn test_custom_config() {
        let config = ParameterConfig {
            max_tokens_limit: 1000,
            temperature_range: (0.1, 1.0),
            top_p_range: (0.1, 0.9),
            max_stop_tokens: 5,
            max_stop_token_length: 10,
        };
        
        let validator = ParameterValidator::with_config(config);
        let session = create_test_session();
        
        // Test custom max_tokens limit
        let mut request = create_test_request();
        request.max_tokens = Some(1001);
        let result = validator.validate(&session, &request);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("1000"));
        
        // Test custom temperature range
        request.max_tokens = Some(500);
        request.temperature = Some(0.05); // Below custom min
        let result = validator.validate(&session, &request);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("between 0.1 and 1"));
        
        // Test custom stop token limits
        request.temperature = Some(0.5);
        request.stop_tokens = vec!["a".repeat(11)]; // Over custom length limit
        let result = validator.validate(&session, &request);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("10 chars"));
    }
    
    #[test]
    fn test_edge_case_values() {
        let validator = ParameterValidator::new();
        let session = create_test_session();
        let mut request = create_test_request();
        
        // Test boundary values that should pass
        request.max_tokens = Some(1);
        request.temperature = Some(0.0);
        request.top_p = Some(0.0);
        assert!(validator.validate(&session, &request).is_ok());
        
        request.temperature = Some(2.0);
        request.top_p = Some(1.0);
        assert!(validator.validate(&session, &request).is_ok());
        
        // Test exact limit values
        request.max_tokens = Some(32_768);
        assert!(validator.validate(&session, &request).is_ok());
        
        request.stop_tokens = vec!["a".repeat(100)]; // Exactly at limit
        assert!(validator.validate(&session, &request).is_ok());
    }
    
    #[test]
    fn test_config_access() {
        let custom_config = ParameterConfig {
            max_tokens_limit: 5000,
            ..Default::default()
        };
        
        let validator = ParameterValidator::with_config(custom_config.clone());
        assert_eq!(validator.config().max_tokens_limit, 5000);
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

pub use session_validator::SessionStateValidator;
pub use message_validator::{MessageContentValidator, MessageContentConfig};
pub use parameter_validator::{ParameterValidator, ParameterConfig};
```

## Integration Points
- Extract exact logic from existing `agent.rs` parameter validation (lines 357-418)
- Maintain all existing bounds and security limits
- Provide easy configuration customization
- Map errors appropriately to existing error types

## Testing
- [ ] Test all parameter bounds from existing implementation
- [ ] Test infinite and NaN value handling
- [ ] Test stop token limits and lengths
- [ ] Test custom configuration scenarios
- [ ] Test boundary values and edge cases

## Notes
- This preserves all existing parameter validation security
- Configuration allows for different limits per deployment
- Extracted logic should have identical behavior to current implementation
- Focus on clear, actionable error messages