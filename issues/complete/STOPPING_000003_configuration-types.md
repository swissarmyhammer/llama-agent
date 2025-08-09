# Create Configuration Types for Stopping System

Refer to ./specification/stopping.md

## Objective

Create the configuration types for the stopping system as specified, and integrate them into the GenerationRequest structure.

## Tasks

### 1. Add Configuration Structs to types.rs
Add the configuration types as specified:

```rust
#[derive(Debug, Clone)]
pub struct StoppingConfig {
    pub max_tokens: Option<usize>,
    pub repetition_detection: Option<RepetitionConfig>,
    pub eos_detection: bool, // Default: true
}

#[derive(Debug, Clone)]
pub struct RepetitionConfig {
    pub min_pattern_length: usize,
    pub max_pattern_length: usize,
    pub min_repetitions: usize,
    pub window_size: usize,
}
```

### 2. Add Default Implementation
Implement sensible defaults:
- `eos_detection: true`
- RepetitionConfig with defaults from specification:
  - `min_pattern_length: 10`
  - `max_pattern_length: 100`
  - `min_repetitions: 3`
  - `window_size: 1000`

### 3. Update GenerationRequest
Add stopping configuration to GenerationRequest:
```rust
pub struct GenerationRequest {
    // existing fields...
    pub stopping_config: Option<StoppingConfig>,
}
```

### 4. Update Related Code
- Update GenerationRequest creation in tests and examples
- Ensure backward compatibility with None as default

## Implementation Notes

- Follow the exact configuration structure from specification
- Provide reasonable defaults for all values
- Make the configuration optional in GenerationRequest to maintain compatibility
- Consider adding validation methods for configuration values

## Acceptance Criteria

- StoppingConfig and RepetitionConfig structs exist in types.rs
- Default implementations provide sensible values
- GenerationRequest includes optional stopping_config field
- All existing code compiles without changes
- `cargo build` and `cargo test` pass successfully