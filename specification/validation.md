# Validation System Specification

## Overview

This specification outlines the design for a trait-based validation system to replace the current method-based validators in `agent.rs`. The system will provide modular, composable validation logic with clear separation of concerns.

## Current State Analysis

### Existing Validation Methods in `agent.rs`

The following validation methods have been identified for refactoring:

1. **`validate_generation_request_with_session`** (lines 308-420)
   - Session state validation
   - Message content validation (length, suspicious patterns, repetition)
   - Generation parameter validation (max_tokens, temperature, top_p)
   - Stop tokens validation

2. **`validate_tool_arguments`** (lines 111-137)
   - Tool parameter schema validation
   - Basic argument presence validation

3. **Helper validation functions:**
   - `contains_suspicious_content` (lines 423-448)
   - `has_excessive_repetition` (lines 451-479)

## Proposed Architecture

### Core Trait Design

```rust
pub trait Validator<Target> {
    type Error;
    
    fn validate(&self, session: &Session, target: &Target) -> Result<(), Self::Error>;
}
```

**Design Philosophy**: Session is the universal context for all validation operations. Every validation ultimately occurs within the scope of a session, which provides:

- **Message History**: Access to conversation context for validation decisions
- **Tool Availability**: Knowledge of which tools are available for validation
- **Session State**: Metadata like creation time, update time for temporal validations
- **MCP Configuration**: Server configurations that may affect validation rules
- **Consistent Interface**: Single, predictable context parameter across all validators

### Validation Error Types

```rust
#[derive(Debug, thiserror::Error)]
pub enum ValidationError {
    #[error("Security violation: {0}")]
    SecurityViolation(String),
    
    #[error("Parameter out of bounds: {0}")]
    ParameterBounds(String),
    
    #[error("Invalid state: {0}")]
    InvalidState(String),
    
    #[error("Content validation failed: {0}")]
    ContentValidation(String),
    
    #[error("Schema validation failed: {0}")]
    SchemaValidation(String),
}
```

## Module Structure

```
src/validation/
├── mod.rs                          # Module exports and re-exports
├── traits.rs                       # Core validation traits
├── errors.rs                       # Validation error types
├── generation_request/
│   ├── mod.rs                      # GenerationRequest validation module
│   ├── session_validator.rs        # Session state validation
│   ├── message_validator.rs        # Message content validation
│   ├── parameter_validator.rs      # Generation parameter validation
│   └── composite_validator.rs      # Combines all GenerationRequest validators
└── tool_call/
    ├── mod.rs                      # ToolCall validation module
    ├── argument_validator.rs       # Tool argument validation
    └── schema_validator.rs         # JSON schema validation
```

## Detailed Validator Specifications

### 1. ValidatesGenerationRequest

#### SessionStateValidator
```rust
pub struct SessionStateValidator;

impl Validator<GenerationRequest> for SessionStateValidator {
    type Error = ValidationError;
    
    fn validate(&self, session: &Session, _request: &GenerationRequest) -> Result<(), Self::Error> {
        // Validate session has messages
        if session.messages.is_empty() {
            return Err(ValidationError::InvalidState(
                "Session must have at least one message for generation".to_string()
            ));
        }
        
        Ok(())
    }
}
```

#### MessageContentValidator
```rust
pub struct MessageContentValidator {
    max_message_length: usize,
    suspicious_patterns: Vec<String>,
}

impl MessageContentValidator {
    pub fn new() -> Self {
        Self {
            max_message_length: 100_000,
            suspicious_patterns: vec![
                "<script".to_string(),
                "javascript:".to_string(),
                "rm -rf".to_string(),
                // ... other patterns
            ],
        }
    }
    
    fn contains_suspicious_content(&self, content: &str) -> bool {
        // Implementation extracted from current agent.rs
    }
    
    fn has_excessive_repetition(&self, content: &str) -> bool {
        // Implementation extracted from current agent.rs
    }
}

impl Validator<Message> for MessageContentValidator {
    type Error = ValidationError;
    
    fn validate(&self, _session: &Session, message: &Message) -> Result<(), Self::Error> {
        // DoS protection: limit message size
        if message.content.len() > self.max_message_length {
            return Err(ValidationError::SecurityViolation(format!(
                "Message exceeds maximum length of {}KB (current: {}KB)",
                self.max_message_length / 1000,
                message.content.len() / 1000
            )));
        }
        
        // Security: Check for potentially malicious content patterns
        if self.contains_suspicious_content(&message.content) {
            return Err(ValidationError::SecurityViolation(
                "Message contains potentially unsafe content patterns".to_string()
            ));
        }
        
        // Security: Check for excessive repetition
        if self.has_excessive_repetition(&message.content) {
            return Err(ValidationError::SecurityViolation(
                "Message contains excessive repetition patterns".to_string()
            ));
        }
        
        Ok(())
    }
}
```

#### ParameterValidator
```rust
pub struct ParameterValidator {
    max_tokens_limit: u32,
    temperature_range: (f32, f32),
    top_p_range: (f32, f32),
    max_stop_tokens: usize,
    max_stop_token_length: usize,
}

impl Default for ParameterValidator {
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

impl Validator<GenerationRequest> for ParameterValidator {
    type Error = ValidationError;
    
    fn validate(&self, _session: &Session, request: &GenerationRequest) -> Result<(), Self::Error> {
        // Validate max_tokens
        if let Some(max_tokens) = request.max_tokens {
            if max_tokens == 0 {
                return Err(ValidationError::ParameterBounds(
                    "max_tokens must be greater than 0".to_string()
                ));
            }
            if max_tokens > self.max_tokens_limit {
                return Err(ValidationError::SecurityViolation(format!(
                    "max_tokens exceeds security limit of {} (requested: {})",
                    self.max_tokens_limit, max_tokens
                )));
            }
        }
        
        // Validate temperature
        if let Some(temp) = request.temperature {
            if !temp.is_finite() {
                return Err(ValidationError::ParameterBounds(
                    "temperature must be a finite number".to_string()
                ));
            }
            if !(self.temperature_range.0..=self.temperature_range.1).contains(&temp) {
                return Err(ValidationError::ParameterBounds(format!(
                    "temperature must be between {} and {} (got: {})",
                    self.temperature_range.0, self.temperature_range.1, temp
                )));
            }
        }
        
        // Validate top_p
        if let Some(top_p) = request.top_p {
            if !top_p.is_finite() {
                return Err(ValidationError::ParameterBounds(
                    "top_p must be a finite number".to_string()
                ));
            }
            if !(self.top_p_range.0..=self.top_p_range.1).contains(&top_p) {
                return Err(ValidationError::ParameterBounds(format!(
                    "top_p must be between {} and {} (got: {})",
                    self.top_p_range.0, self.top_p_range.1, top_p
                )));
            }
        }
        
        // Validate stop tokens
        if request.stop_tokens.len() > self.max_stop_tokens {
            return Err(ValidationError::SecurityViolation(format!(
                "Too many stop tokens: {} (max {} allowed)",
                request.stop_tokens.len(), self.max_stop_tokens
            )));
        }
        
        for (i, stop_token) in request.stop_tokens.iter().enumerate() {
            if stop_token.len() > self.max_stop_token_length {
                return Err(ValidationError::SecurityViolation(format!(
                    "Stop token {} exceeds maximum length of {} chars",
                    i, self.max_stop_token_length
                )));
            }
        }
        
        Ok(())
    }
}
```

#### CompositeGenerationRequestValidator
```rust
pub struct CompositeGenerationRequestValidator {
    session_validator: SessionStateValidator,
    message_validator: MessageContentValidator,
    parameter_validator: ParameterValidator,
}

impl Default for CompositeGenerationRequestValidator {
    fn default() -> Self {
        Self {
            session_validator: SessionStateValidator,
            message_validator: MessageContentValidator::new(),
            parameter_validator: ParameterValidator::default(),
        }
    }
}

impl Validator<GenerationRequest> for CompositeGenerationRequestValidator {
    type Error = ValidationError;
    
    fn validate(&self, session: &Session, request: &GenerationRequest) -> Result<(), Self::Error> {
        // Validate session state
        self.session_validator.validate(session, request)?;
        
        // Validate all messages in session
        for message in &session.messages {
            self.message_validator.validate(session, message)?;
        }
        
        // Validate generation parameters
        self.parameter_validator.validate(session, request)?;
        
        Ok(())
    }
}
```

## Integration Pattern

### Usage in AgentServer

```rust
impl AgentServer {
    fn new(...) -> Self {
        Self {
            // ... other fields
            generation_validator: Arc::new(CompositeGenerationRequestValidator::default()),
        }
    }
    
    fn validate_generation_request_with_session(
        &self,
        request: &GenerationRequest,
        session: &Session,
    ) -> Result<(), AgentError> {
        self.generation_validator
            .validate(session, request)
            .map_err(|e| match e {
                ValidationError::SecurityViolation(msg) => {
                    AgentError::Session(SessionError::InvalidState(msg))
                }
                ValidationError::ParameterBounds(msg) => {
                    AgentError::Queue(QueueError::WorkerError(msg))
                }
                ValidationError::InvalidState(msg) => {
                    AgentError::Session(SessionError::InvalidState(msg))
                }
                // ... other error mappings
            })
    }
}
```

## Configuration and Extensibility

### Configurable Validators

Each validator should support configuration to allow customization of validation rules:

```rust
pub struct ValidationConfig {
    pub message_content: MessageContentConfig,
    pub parameters: ParameterConfig,
    pub security: SecurityConfig,
}

pub struct MessageContentConfig {
    pub max_length: usize,
    pub custom_suspicious_patterns: Vec<String>,
    pub repetition_threshold: usize,
}

pub struct ParameterConfig {
    pub max_tokens_limit: u32,
    pub temperature_range: (f32, f32),
    pub top_p_range: (f32, f32),
}

pub struct SecurityConfig {
    pub enable_content_scanning: bool,
    pub strict_mode: bool,
}
```

### Plugin System

Allow for custom validators to be plugged in:

```rust
pub trait ValidatorPlugin<T> {
    fn name(&self) -> &str;
    fn validate(&self, input: &T) -> Result<(), ValidationError>;
}

pub struct PluginRegistry<T> {
    plugins: Vec<Box<dyn ValidatorPlugin<T>>>,
}

impl<T> PluginRegistry<T> {
    pub fn register(&mut self, plugin: Box<dyn ValidatorPlugin<T>>) {
        self.plugins.push(plugin);
    }
    
    pub fn validate_all(&self, input: &T) -> Result<(), ValidationError> {
        for plugin in &self.plugins {
            plugin.validate(input)?;
        }
        Ok(())
    }
}
```

## Testing Strategy

### Unit Tests
- Each validator component should have comprehensive unit tests
- Test both valid and invalid inputs
- Test edge cases and boundary conditions
- Test error message quality and clarity

### Integration Tests
- Test the composite validator with realistic request/session combinations
- Test error propagation through the validation chain
- Test performance with large message sets

### Property-Based Tests
- Use `proptest` to generate random valid/invalid inputs
- Verify validation consistency across different input combinations
- Test that valid inputs never fail validation

## Migration Strategy

1. **Phase 1**: Implement new validation module structure with comprehensive tests
2. **Phase 2**: Replace existing validation calls with new trait-based system
3. **Phase 3**: Remove old validation methods
4. **Phase 4**: Add configuration support and plugin system

## Benefits

1. **Modularity**: Each validation concern is isolated in its own validator
2. **Testability**: Individual validators can be tested in isolation
3. **Composability**: Validators can be combined in different ways
4. **Configurability**: Validation rules can be customized per deployment
5. **Extensibility**: New validators can be added without modifying existing code
6. **Performance**: Validation can be optimized per validator type
7. **Maintainability**: Clear separation of concerns makes code easier to maintain

