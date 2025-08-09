//! Tool call argument validation

use crate::types::{Session, ToolCall};
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
        if Self::get_json_depth(arguments) > self.config.max_argument_depth {
            return Err(ValidationError::security_violation(format!(
                "Tool arguments exceed maximum depth of {} levels",
                self.config.max_argument_depth
            )));
        }

        Ok(())
    }

    /// Calculate the depth of a JSON value
    fn get_json_depth(value: &Value) -> usize {
        match value {
            Value::Object(obj) => {
                if obj.is_empty() {
                    1
                } else {
                    1 + obj.values().map(Self::get_json_depth).max().unwrap_or(0)
                }
            }
            Value::Array(arr) => {
                if arr.is_empty() {
                    1
                } else {
                    1 + arr.iter().map(Self::get_json_depth).max().unwrap_or(0)
                }
            }
            _ => 1,
        }
    }

    /// Validate tool name
    fn validate_tool_name(&self, tool_call: &ToolCall) -> ValidationResult {
        if tool_call.name.trim().is_empty() {
            return Err(ValidationError::invalid_state("Tool name cannot be empty"));
        }

        // Check for reasonable name length
        if tool_call.name.len() > 256 {
            return Err(ValidationError::security_violation(
                "Tool name exceeds maximum length of 256 characters",
            ));
        }

        // Basic name format validation (letters, numbers, underscores, hyphens)
        if !tool_call
            .name
            .chars()
            .all(|c| c.is_alphanumeric() || c == '_' || c == '-')
        {
            return Err(ValidationError::invalid_state(
                "Tool name contains invalid characters (only letters, numbers, underscores, and hyphens allowed)"
            ));
        }

        Ok(())
    }

    /// Validate that the tool is available in the session
    fn validate_tool_availability(
        &self,
        session: &Session,
        tool_call: &ToolCall,
    ) -> ValidationResult {
        // Check if the tool is available in the session
        let tool_available = session
            .available_tools
            .iter()
            .any(|tool| tool.name == tool_call.name);

        if !tool_available {
            return Err(ValidationError::invalid_state(format!(
                "Tool '{}' is not available in this session. Available tools: [{}]",
                tool_call.name,
                session
                    .available_tools
                    .iter()
                    .map(|t| t.name.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            )));
        }

        Ok(())
    }

    /// Validate basic argument requirements against tool definition
    /// Enhanced version of the existing validation in agent.rs
    fn validate_argument_requirements(
        &self,
        tool_call: &ToolCall,
        session: &Session,
    ) -> ValidationResult {
        // Find the tool definition
        let tool_def = session
            .available_tools
            .iter()
            .find(|t| t.name == tool_call.name)
            .ok_or_else(|| {
                ValidationError::invalid_state(format!(
                    "Tool '{}' definition not found",
                    tool_call.name
                ))
            })?;

        // If no parameters schema is defined, skip validation
        if tool_def.parameters.is_null() {
            return Ok(());
        }

        // Basic validation - enhanced from agent.rs logic
        if tool_call.arguments.is_null() && !tool_def.parameters.is_null() {
            return Err(ValidationError::invalid_state(
                "Tool requires arguments but none provided",
            ));
        }

        // Additional validation could be added here:
        // - JSON Schema validation against tool_def.parameters
        // - Type checking for required fields
        // - Range validation for numeric parameters
        // This will be implemented in the schema validation step

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
        // Validate tool name
        self.validate_tool_name(tool_call)?;

        // Validate tool availability in session
        self.validate_tool_availability(session, tool_call)?;

        // Validate JSON structure and limits
        self.validate_json_structure(&tool_call.arguments)?;

        // Validate argument requirements against tool definition
        self.validate_argument_requirements(tool_call, session)?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{SessionId, ToolCallId, ToolDefinition};
    use serde_json::json;
    use std::time::SystemTime;

    fn create_test_session_with_tools(tools: Vec<ToolDefinition>) -> Session {
        Session {
            id: SessionId::new(),
            messages: vec![],
            mcp_servers: vec![],
            available_tools: tools,
            available_prompts: vec![],
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

    fn create_test_tool_call(name: &str, args: Value) -> ToolCall {
        ToolCall {
            id: ToolCallId::new(),
            name: name.to_string(),
            arguments: args,
        }
    }

    #[test]
    fn test_valid_tool_call_passes() {
        let validator = ToolArgumentValidator::new();
        let session =
            create_test_session_with_tools(vec![create_test_tool_definition("test_tool")]);

        let tool_call = create_test_tool_call("test_tool", json!({"input": "hello world"}));

        assert!(validator.validate(&session, &tool_call).is_ok());
    }

    #[test]
    fn test_empty_tool_name_fails() {
        let validator = ToolArgumentValidator::new();
        let session =
            create_test_session_with_tools(vec![create_test_tool_definition("test_tool")]);

        let tool_call = create_test_tool_call("", json!({"input": "test"}));

        let result = validator.validate(&session, &tool_call);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("name cannot be empty"));
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
            let tool_call = create_test_tool_call(invalid_name, json!({"input": "test"}));

            let result = validator.validate(&session, &tool_call);
            assert!(result.is_err(), "Should fail for name: {}", invalid_name);
            assert!(result
                .unwrap_err()
                .to_string()
                .contains("invalid characters"));
        }
    }

    #[test]
    fn test_tool_not_available_fails() {
        let validator = ToolArgumentValidator::new();
        let session =
            create_test_session_with_tools(vec![create_test_tool_definition("available_tool")]);

        let tool_call = create_test_tool_call("unavailable_tool", json!({"input": "test"}));

        let result = validator.validate(&session, &tool_call);
        assert!(result.is_err());
        let error_msg = result.unwrap_err().to_string();
        assert!(error_msg.contains("is not available"));
        assert!(error_msg.contains("available_tool"));
    }

    #[test]
    fn test_excessive_argument_size_fails() {
        let config = ArgumentValidatorConfig {
            max_argument_size: 100, // Very small limit
            ..Default::default()
        };
        let validator = ToolArgumentValidator::with_config(config);
        let session =
            create_test_session_with_tools(vec![create_test_tool_definition("test_tool")]);

        // Create large argument that exceeds limit
        let large_string = "a".repeat(200);
        let tool_call = create_test_tool_call("test_tool", json!({"input": large_string}));

        let result = validator.validate(&session, &tool_call);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("exceed maximum size"));
    }

    #[test]
    fn test_excessive_argument_depth_fails() {
        let config = ArgumentValidatorConfig {
            max_argument_depth: 3,
            ..Default::default()
        };
        let validator = ToolArgumentValidator::with_config(config);
        let session =
            create_test_session_with_tools(vec![create_test_tool_definition("test_tool")]);

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

        let tool_call = create_test_tool_call("test_tool", deep_args);

        let result = validator.validate(&session, &tool_call);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("exceed maximum depth"));
    }

    #[test]
    fn test_long_name_fails() {
        let validator = ToolArgumentValidator::new();
        let session = create_test_session_with_tools(vec![]);

        // Test long name
        let long_name = "a".repeat(257);
        let tool_call = create_test_tool_call(&long_name, json!({}));
        let result = validator.validate(&session, &tool_call);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("name exceeds maximum length"));
    }

    #[test]
    fn test_json_depth_calculation() {
        // Test simple values
        assert_eq!(ToolArgumentValidator::get_json_depth(&json!("string")), 1);
        assert_eq!(ToolArgumentValidator::get_json_depth(&json!(42)), 1);
        assert_eq!(ToolArgumentValidator::get_json_depth(&json!(null)), 1);

        // Test empty containers
        assert_eq!(ToolArgumentValidator::get_json_depth(&json!({})), 1);
        assert_eq!(ToolArgumentValidator::get_json_depth(&json!([])), 1);

        // Test nested structures
        assert_eq!(ToolArgumentValidator::get_json_depth(&json!({"a": 1})), 2);
        assert_eq!(ToolArgumentValidator::get_json_depth(&json!([1, 2, 3])), 2);
        assert_eq!(
            ToolArgumentValidator::get_json_depth(&json!({"a": {"b": 1}})),
            3
        );
        assert_eq!(
            ToolArgumentValidator::get_json_depth(&json!({"a": [{"b": 1}]})),
            4
        );
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
            let tool_call = create_test_tool_call(name, json!({}));
            let result = validator.validate(&session, &tool_call);
            assert!(result.is_ok(), "Should pass for name: {}", name);
        }
    }

    #[test]
    fn test_arguments_required_but_none_provided() {
        let validator = ToolArgumentValidator::new();
        let session =
            create_test_session_with_tools(vec![create_test_tool_definition("test_tool")]);

        let tool_call = create_test_tool_call("test_tool", json!(null));

        let result = validator.validate(&session, &tool_call);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Tool requires arguments but none provided"));
    }

    #[test]
    fn test_tool_with_no_parameter_schema() {
        let validator = ToolArgumentValidator::new();
        let mut tool_def = create_test_tool_definition("test_tool");
        tool_def.parameters = json!(null);
        let session = create_test_session_with_tools(vec![tool_def]);

        let tool_call = create_test_tool_call("test_tool", json!(null));

        let result = validator.validate(&session, &tool_call);
        assert!(result.is_ok());
    }

    #[test]
    fn test_argument_validator_config_defaults() {
        let config = ArgumentValidatorConfig::default();
        assert_eq!(config.max_argument_depth, 10);
        assert_eq!(config.max_argument_size, 1_000_000);
        assert!(config.strict_type_checking);
    }

    #[test]
    fn test_validator_with_custom_config() {
        let config = ArgumentValidatorConfig {
            max_argument_depth: 5,
            max_argument_size: 500,
            strict_type_checking: false,
        };

        let validator = ToolArgumentValidator::with_config(config.clone());
        assert_eq!(validator.config().max_argument_depth, 5);
        assert_eq!(validator.config().max_argument_size, 500);
        assert!(!validator.config().strict_type_checking);
    }
}
