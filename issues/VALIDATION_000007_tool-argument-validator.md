# Implement Tool Argument Validation

Refer to ./specification/validation.md

## Overview
Implement tool call argument validation for MCP tool calls. This includes basic argument presence validation and prepares for schema validation. This extracts and enhances the existing `validate_tool_arguments` logic from `agent.rs`.

## Acceptance Criteria
- [ ] Extract existing tool argument validation from `agent.rs` (lines 111-137)
- [ ] Implement `ToolArgumentValidator` struct
- [ ] Implement `Validator<ToolCall>` trait  
- [ ] Add argument presence and basic type validation
- [ ] Prepare structure for schema validation (next step)
- [ ] Add comprehensive unit tests

## Implementation Details

### Create `src/validation/tool_call/argument_validator.rs`

```rust
//! Tool call argument validation

use crate::types::{ToolCall, Session};
use crate::validation::{ValidationError, ValidationResult, Validator};
use serde_json::Value;

/// Configuration for tool argument validation
#[derive(Debug, Clone)]
pub struct ArgumentValidatorConfig {
    /// Maximum depth for nested JSON arguments
    pub max_argument_depth: usize,
    /// Maximum size for argument JSON in bytes
    pub max_argument_size: usize,
    /// Whether to perform strict type checking
    pub strict_type_checking: bool,
}

impl Default for ArgumentValidatorConfig {
    fn default() -> Self {
        Self {
            max_argument_depth: 10,
            max_argument_size: 1_000_000, // 1MB
            strict_type_checking: true,
        }
    }
}

/// Validates tool call arguments for basic structure and safety
/// 
/// This validator performs:
/// - Argument presence validation
/// - JSON structure validation
/// - Size and depth limits for security
/// - Basic type checking
#[derive(Debug, Clone)]
pub struct ToolArgumentValidator {
    config: ArgumentValidatorConfig,
}

impl ToolArgumentValidator {
    /// Create a new tool argument validator with default configuration
    pub fn new() -> Self {
        Self::with_config(ArgumentValidatorConfig::default())
    }
    
    /// Create a validator with custom configuration
    pub fn with_config(config: ArgumentValidatorConfig) -> Self {
        Self { config }
    }
    
    /// Get the current configuration
    pub fn config(&self) -> &ArgumentValidatorConfig {
        &self.config
    }
    
    /// Validate JSON structure and size limits
    fn validate_json_structure(&self, arguments: &Value) -> ValidationResult {
        // Check argument size (serialize to estimate size)
        let json_string = serde_json::to_string(arguments).map_err(|e| {
            ValidationError::schema_validation(format!("Failed to serialize arguments: {}", e))
        })?;
        
        if json_string.len() > self.config.max_argument_size {
            return Err(ValidationError::security_violation(format!(
                "Tool arguments exceed maximum size of {} bytes (current: {} bytes)",
                self.config.max_argument_size,
                json_string.len()
            )));
        }
        
        // Check argument depth
        if self.get_json_depth(arguments) > self.config.max_argument_depth {
            return Err(ValidationError::security_violation(format!(
                "Tool arguments exceed maximum depth of {} levels",
                self.config.max_argument_depth
            )));
        }
        
        Ok(())
    }
    
    /// Calculate the depth of a JSON value
    fn get_json_depth(&self, value: &Value) -> usize {
        match value {
            Value::Object(obj) => {
                if obj.is_empty() {
                    1
                } else {
                    1 + obj.values().map(|v| self.get_json_depth(v)).max().unwrap_or(0)
                }
            },
            Value::Array(arr) => {
                if arr.is_empty() {
                    1
                } else {
                    1 + arr.iter().map(|v| self.get_json_depth(v)).max().unwrap_or(0)
                }
            },
            _ => 1,
        }
    }
    
    /// Validate that tool call ID is valid
    fn validate_tool_call_id(&self, tool_call: &ToolCall) -> ValidationResult {
        if tool_call.id.trim().is_empty() {
            return Err(ValidationError::invalid_state(
                "Tool call ID cannot be empty"
            ));
        }
        
        // Check for reasonable ID length (prevent DoS)
        if tool_call.id.len() > 256 {
            return Err(ValidationError::security_violation(
                "Tool call ID exceeds maximum length of 256 characters"
            ));
        }
        
        Ok(())
    }
    
    /// Validate tool name
    fn validate_tool_name(&self, tool_call: &ToolCall) -> ValidationResult {
        if tool_call.name.trim().is_empty() {
            return Err(ValidationError::invalid_state(
                "Tool name cannot be empty"
            ));
        }
        
        // Check for reasonable name length
        if tool_call.name.len() > 256 {
            return Err(ValidationError::security_violation(
                "Tool name exceeds maximum length of 256 characters"
            ));
        }
        
        // Basic name format validation (letters, numbers, underscores, hyphens)
        if !tool_call.name.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-') {
            return Err(ValidationError::invalid_state(
                "Tool name contains invalid characters (only letters, numbers, underscores, and hyphens allowed)"
            ));
        }
        
        Ok(())
    }
    
    /// Validate that the tool is available in the session
    fn validate_tool_availability(&self, session: &Session, tool_call: &ToolCall) -> ValidationResult {
        // Check if the tool is available in the session
        let tool_available = session.available_tools.iter()
            .any(|tool| tool.name == tool_call.name);
            
        if !tool_available {
            return Err(ValidationError::invalid_state(format!(
                "Tool '{}' is not available in this session. Available tools: [{}]",
                tool_call.name,
                session.available_tools.iter().map(|t| &t.name).collect::<Vec<_>>().join(", ")
            )));
        }
        
        Ok(())
    }
}

impl Default for ToolArgumentValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl Validator<ToolCall> for ToolArgumentValidator {
    type Error = ValidationError;
    
    fn validate(&self, session: &Session, tool_call: &ToolCall) -> ValidationResult {
        // Validate tool call ID
        self.validate_tool_call_id(tool_call)?;
        
        // Validate tool name
        self.validate_tool_name(tool_call)?;
        
        // Validate tool availability in session
        self.validate_tool_availability(session, tool_call)?;
        
        // Validate JSON structure and limits
        self.validate_json_structure(&tool_call.arguments)?;
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{SessionId, ToolDefinition};
    use serde_json::json;
    use std::time::SystemTime;
    
    fn create_test_session_with_tools(tools: Vec<ToolDefinition>) -> Session {
        Session {
            id: SessionId("test".to_string()),
            messages: vec![],
            mcp_servers: vec![],
            available_tools: tools,
            created_at: SystemTime::now(),
            updated_at: SystemTime::now(),
        }
    }
    
    fn create_test_tool_definition(name: &str) -> ToolDefinition {
        ToolDefinition {
            name: name.to_string(),
            description: format!("Test tool {}", name),
            parameters: json!({
                "type": "object",
                "properties": {
                    "input": {"type": "string"}
                }
            }),
            server_name: "test_server".to_string(),
        }
    }
    
    fn create_test_tool_call(id: &str, name: &str, args: Value) -> ToolCall {
        ToolCall {
            id: id.to_string(),
            name: name.to_string(),
            arguments: args,
        }
    }
    
    #[test]
    fn test_valid_tool_call_passes() {
        let validator = ToolArgumentValidator::new();
        let session = create_test_session_with_tools(vec![
            create_test_tool_definition("test_tool")
        ]);
        
        let tool_call = create_test_tool_call(
            "call_123",
            "test_tool",
            json!({"input": "hello world"})
        );
        
        assert!(validator.validate(&session, &tool_call).is_ok());
    }
    
    #[test]
    fn test_empty_tool_call_id_fails() {
        let validator = ToolArgumentValidator::new();
        let session = create_test_session_with_tools(vec![
            create_test_tool_definition("test_tool")
        ]);
        
        let tool_call = create_test_tool_call(
            "",
            "test_tool",
            json!({"input": "test"})
        );
        
        let result = validator.validate(&session, &tool_call);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("ID cannot be empty"));
    }
    
    #[test]
    fn test_empty_tool_name_fails() {
        let validator = ToolArgumentValidator::new();
        let session = create_test_session_with_tools(vec![]);
        
        let tool_call = create_test_tool_call(
            "call_123",
            "",
            json!({"input": "test"})
        );
        
        let result = validator.validate(&session, &tool_call);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("name cannot be empty"));
    }
    
    #[test]
    fn test_invalid_tool_name_characters_fail() {
        let validator = ToolArgumentValidator::new();
        let session = create_test_session_with_tools(vec![]);
        
        let invalid_names = vec![
            "tool@name",
            "tool name", // space
            "tool.name",
            "tool/name",
            "tool\\name",
        ];
        
        for invalid_name in invalid_names {
            let tool_call = create_test_tool_call(
                "call_123",
                invalid_name,
                json!({"input": "test"})
            );
            
            let result = validator.validate(&session, &tool_call);
            assert!(result.is_err(), "Should fail for name: {}", invalid_name);
            assert!(result.unwrap_err().to_string().contains("invalid characters"));
        }
    }
    
    #[test]
    fn test_tool_not_available_fails() {
        let validator = ToolArgumentValidator::new();
        let session = create_test_session_with_tools(vec![
            create_test_tool_definition("available_tool")
        ]);
        
        let tool_call = create_test_tool_call(
            "call_123",
            "unavailable_tool",
            json!({"input": "test"})
        );
        
        let result = validator.validate(&session, &tool_call);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("is not available"));
        assert!(result.unwrap_err().to_string().contains("available_tool"));
    }
    
    #[test]
    fn test_excessive_argument_size_fails() {
        let config = ArgumentValidatorConfig {
            max_argument_size: 100, // Very small limit
            ..Default::default()
        };
        let validator = ToolArgumentValidator::with_config(config);
        let session = create_test_session_with_tools(vec![
            create_test_tool_definition("test_tool")
        ]);
        
        // Create large argument that exceeds limit
        let large_string = "a".repeat(200);
        let tool_call = create_test_tool_call(
            "call_123",
            "test_tool",
            json!({"input": large_string})
        );
        
        let result = validator.validate(&session, &tool_call);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("exceed maximum size"));
    }
    
    #[test]
    fn test_excessive_argument_depth_fails() {
        let config = ArgumentValidatorConfig {
            max_argument_depth: 3,
            ..Default::default()
        };
        let validator = ToolArgumentValidator::with_config(config);
        let session = create_test_session_with_tools(vec![
            create_test_tool_definition("test_tool")
        ]);
        
        // Create deeply nested argument
        let deep_args = json!({
            "level1": {
                "level2": {
                    "level3": {
                        "level4": "too deep"
                    }
                }
            }
        });
        
        let tool_call = create_test_tool_call("call_123", "test_tool", deep_args);
        
        let result = validator.validate(&session, &tool_call);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("exceed maximum depth"));
    }
    
    #[test]
    fn test_long_id_and_name_fail() {
        let validator = ToolArgumentValidator::new();
        let session = create_test_session_with_tools(vec![]);
        
        // Test long ID
        let long_id = "a".repeat(257);
        let tool_call = create_test_tool_call(&long_id, "test_tool", json!({}));
        let result = validator.validate(&session, &tool_call);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("ID exceeds maximum length"));
        
        // Test long name
        let long_name = "a".repeat(257);
        let session = create_test_session_with_tools(vec![
            create_test_tool_definition(&long_name)
        ]);
        let tool_call = create_test_tool_call("call_123", &long_name, json!({}));
        let result = validator.validate(&session, &tool_call);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("name exceeds maximum length"));
    }
    
    #[test]
    fn test_json_depth_calculation() {
        let validator = ToolArgumentValidator::new();
        
        // Test simple values
        assert_eq!(validator.get_json_depth(&json!("string")), 1);
        assert_eq!(validator.get_json_depth(&json!(42)), 1);
        assert_eq!(validator.get_json_depth(&json!(null)), 1);
        
        // Test empty containers
        assert_eq!(validator.get_json_depth(&json!({})), 1);
        assert_eq!(validator.get_json_depth(&json!([])), 1);
        
        // Test nested structures
        assert_eq!(validator.get_json_depth(&json!({"a": 1})), 2);
        assert_eq!(validator.get_json_depth(&json!([1, 2, 3])), 2);
        assert_eq!(validator.get_json_depth(&json!({"a": {"b": 1}})), 3);
        assert_eq!(validator.get_json_depth(&json!({"a": [{"b": 1}]})), 4);
    }
    
    #[test]
    fn test_valid_tool_name_formats() {
        let validator = ToolArgumentValidator::new();
        let session = create_test_session_with_tools(vec![
            create_test_tool_definition("valid_tool_123"),
            create_test_tool_definition("tool-with-hyphens"),
            create_test_tool_definition("UPPERCASE"),
            create_test_tool_definition("123numbers"),
        ]);
        
        let valid_names = vec![
            "valid_tool_123",
            "tool-with-hyphens", 
            "UPPERCASE",
            "123numbers",
        ];
        
        for name in valid_names {
            let tool_call = create_test_tool_call("call_123", name, json!({}));
            let result = validator.validate(&session, &tool_call);
            assert!(result.is_ok(), "Should pass for name: {}", name);
        }
    }
}
```

### Update `src/validation/tool_call/mod.rs`

```rust
//! Tool call validation components
//! 
//! This module contains all validators related to ToolCall validation.

mod argument_validator;

pub use argument_validator::{ToolArgumentValidator, ArgumentValidatorConfig};
```

## Integration Points
- Extract logic from existing `validate_tool_arguments` in agent.rs (lines 111-137)
- Work with existing `ToolCall` and `ToolDefinition` types
- Prepare for schema validation integration in next step
- Ensure tool availability checking works with session context

## Testing
- [ ] Test basic argument structure validation
- [ ] Test tool availability checking
- [ ] Test size and depth limits for security
- [ ] Test ID and name validation
- [ ] Test JSON structure validation
- [ ] Test custom configuration options

## Notes
- This provides the foundation for more sophisticated schema validation
- Focuses on basic security and structure validation
- Prepares the interface for schema-based validation in the next step
- Maintains compatibility with existing tool call workflows