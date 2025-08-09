# Create Validation Module Structure

Refer to ./specification/validation.md

## Overview
Create the foundational module structure for the new trait-based validation system. This establishes the organizational framework that will house all validation components.

## Acceptance Criteria
- [ ] Create `src/validation/` directory structure
- [ ] Implement `mod.rs` with proper exports
- [ ] Create placeholder modules for all validation components
- [ ] Add validation module to main library exports
- [ ] Ensure module compiles without errors

## Implementation Details

### Directory Structure to Create
```
src/validation/
├── mod.rs                          # Module exports and re-exports
├── traits.rs                       # Core validation traits (placeholder)
├── errors.rs                       # Validation error types (placeholder)
├── generation_request/
│   └── mod.rs                      # GenerationRequest validation module (placeholder)
└── tool_call/
    └── mod.rs                      # ToolCall validation module (placeholder)
```

### Files to Create

#### `src/validation/mod.rs`
```rust
//! Validation system for agent requests and data
//! 
//! This module provides a trait-based validation system with modular,
//! composable validation logic and clear separation of concerns.

pub mod errors;
pub mod traits;
pub mod generation_request;
pub mod tool_call;

// Re-export main validation types
pub use errors::ValidationError;
pub use traits::Validator;
```

#### `src/validation/traits.rs` 
```rust
//! Core validation traits and interfaces

use super::errors::ValidationError;
use crate::types::Session;

/// Core validation trait that all validators implement
pub trait Validator<Target> {
    type Error;
    
    fn validate(&self, session: &Session, target: &Target) -> Result<(), Self::Error>;
}
```

#### `src/validation/errors.rs`
```rust
//! Validation error types

/// Validation errors that can occur during validation
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

#### `src/validation/generation_request/mod.rs`
```rust
//! Generation request validation components
//! 
//! This module contains all validators related to GenerationRequest validation.

// Placeholder - will be implemented in subsequent steps
```

#### `src/validation/tool_call/mod.rs`
```rust
//! Tool call validation components
//! 
//! This module contains all validators related to ToolCall validation.

// Placeholder - will be implemented in subsequent steps
```

### Integration Points
- Add `pub mod validation;` to `src/lib.rs`
- Ensure all placeholder modules compile
- Maintain compatibility with existing code

## Testing
- [ ] Module structure compiles without errors
- [ ] All exports are accessible from root crate
- [ ] No breaking changes to existing functionality

## Notes
This step focuses purely on establishing the module structure. No actual validation logic is implemented yet - that comes in subsequent steps.