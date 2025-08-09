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

## Proposed Solution

Based on the specification, I will implement the foundational stopper module structure with:

1. **Module Structure**: Create `src/stopper/mod.rs` with the core Stopper trait and stub files for individual implementations
2. **Core Trait**: Define the `Stopper` trait exactly as specified with `should_stop` method
3. **Stub Files**: Create empty implementation files for `eos.rs`, `max_tokens.rs`, and `repetition.rs`
4. **Integration**: Add module export to `src/lib.rs`

The implementation will:
- Import required types from `llama-cpp-2` crate (`LlamaContext`, `LlamaBatch`)  
- Use the existing `FinishReason` enum from `types.rs` (will be migrated in later issue)
- Ensure all files compile without errors
- Follow Rust module best practices and project conventions

## Implementation Complete ✅

Successfully implemented the foundational stopper module structure with the following deliverables:

### 1. Created Module Structure ✅
- Created `src/stopper/mod.rs` with the main Stopper trait definition
- Created stub files for individual stopper implementations:
  - `src/stopper/eos.rs` - EosStopper with eos_token_id field
  - `src/stopper/max_tokens.rs` - MaxTokensStopper with max_tokens and current_tokens fields
  - `src/stopper/repetition.rs` - RepetitionStopper with RepetitionConfig and recent_text field

### 2. Defined Core Stopper Trait ✅
```rust
pub trait Stopper {
    fn should_stop(&mut self, context: &LlamaContext, batch: &LlamaBatch) -> Option<FinishReason>;
}
```
- Correctly imports required types from `llama-cpp-2` crate
- Uses existing `FinishReason` enum from types.rs
- Follows exact specification requirements

### 3. Module Organization ✅
- Added stopper module to `src/lib.rs`
- Properly exports all stopper components:
  - `Stopper` trait
  - `EosStopper`, `MaxTokensStopper`, `RepetitionStopper` implementations
  - `RepetitionConfig` configuration type
- All exports working correctly

### 4. Verification ✅
- ✅ All unit tests pass (153 tests)
- ✅ Debug build compiles successfully 
- ✅ Release build compiles successfully
- ✅ No breaking changes to existing functionality
- ✅ Expected dead code warnings for stub implementations

The module foundation is now ready for the actual stopper implementations in subsequent issues.
