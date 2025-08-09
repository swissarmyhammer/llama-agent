# Update Generation Request Configuration and Callers

Refer to ./specification/stopping.md

## Objective

Update all callers of GenerationRequest to properly use the new stopping configuration system and provide sensible defaults.

## Tasks

### 1. Update GenerationRequest Creation Sites
Find all locations where GenerationRequest is created and add stopping_config:
- Look in tests, examples, and main agent code
- Add appropriate stopping configuration or None for default

### 2. Create Helper Functions
Add convenience methods to GenerationRequest or create builder pattern:
```rust
impl GenerationRequest {
    pub fn with_default_stopping(mut self) -> Self {
        if self.stopping_config.is_none() {
            self.stopping_config = Some(StoppingConfig::default());
        }
        self
    }
}
```

### 3. Update Tests and Examples
- Update existing tests to work with new configuration
- Add tests that specifically use stopping configuration
- Update examples to show stopping configuration usage

### 4. Handle Migration from max_tokens
- Ensure compatibility between existing max_tokens field and stopping config
- Create logic to merge max_tokens into stopping configuration when needed
- Maintain backward compatibility

### 5. Configuration Validation
Add validation for stopping configuration:
- Reasonable limits on repetition detection parameters
- Validation that max_tokens makes sense
- Error handling for invalid configurations

## Implementation Notes

- Maintain backward compatibility - existing code should work unchanged
- Provide clear migration path for users who want to use new features
- Consider deprecating direct max_tokens usage in favor of stopping config
- Ensure test coverage for all configuration scenarios

## Acceptance Criteria

- All GenerationRequest creation sites updated appropriately
- Backward compatibility maintained for existing code
- Helper functions make configuration easier
- Comprehensive test coverage for configuration scenarios
- All examples and tests pass
- Clear documentation of configuration options