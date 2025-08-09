# Create Stopper Module Structure

Refer to ./specification/stopping.md

## Objective

Create the foundational stopper module structure with the core Stopper trait and module organization as specified in the stopping specification.

## Tasks

### 1. Create Module Structure
- Create `src/stopper/mod.rs` with the main Stopper trait definition
- Create stub files for individual stopper implementations:
  - `src/stopper/eos.rs`
  - `src/stopper/max_tokens.rs`
  - `src/stopper/repetition.rs`
- Add module export to `src/lib.rs`

### 2. Define Core Stopper Trait
Define the Stopper trait exactly as specified:
```rust
pub trait Stopper {
    fn should_stop(&mut self, context: &LlamaContext, batch: &LlamaBatch) -> Option<FinishReason>;
}
```

### 3. Module Organization
Organize the stopper module to expose:
- The Stopper trait
- All stopper implementations (as re-exports)
- Configuration types

## Implementation Notes

- Follow the exact module structure from the specification
- Keep individual stopper files as empty stubs for now (just module structure)
- Focus only on the foundation - actual implementations come in later steps
- Ensure the module compiles without errors

## Acceptance Criteria

- `src/stopper/mod.rs` exists with Stopper trait
- All stub files exist and compile
- Module is properly exported in `src/lib.rs`
- No existing functionality is broken
- Code compiles successfully with `cargo build`