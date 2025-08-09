# Migrate FinishReason Enum to Simplified Format

Refer to ./specification/stopping.md

## Objective

Migrate the existing `FinishReason` enum from multiple variants to the simplified `Stopped(String)` variant as specified, while maintaining backward compatibility.

## Background

The current `FinishReason` enum in `types.rs` (lines 193-200) has multiple variants:
- `MaxTokens`
- `StopToken`  
- `EndOfSequence`
- `ToolCall`
- `Error(String)`

The specification requires simplifying to only:
- `Stopped(String)`

## Tasks

### 1. Update FinishReason Enum
- Replace the existing enum definition with the new simplified version:
```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FinishReason {
    Stopped(String),
}
```

### 2. Update All Usage Throughout Codebase
Find and update all existing usage to use descriptive messages:
- `MaxTokens` → `Stopped("Maximum tokens reached")`
- `StopToken` → `Stopped("Stop token detected")`  
- `EndOfSequence` → `Stopped("End of sequence token detected")`
- `ToolCall` → `Stopped("Tool call detected")`
- `Error(msg)` → `Stopped(format!("Error: {}", msg))`

### 3. Search and Replace Strategy
- Use grep to find all FinishReason usage
- Update each location systematically
- Test compilation after each batch of changes

## Implementation Notes

- This is a breaking change but isolated to internal usage
- Focus on preserving the semantic meaning in the descriptive strings
- Test compilation frequently to catch any missed references
- Ensure error messages are clear and descriptive

## Acceptance Criteria

- FinishReason enum has only Stopped(String) variant
- All existing usage updated with descriptive messages
- Code compiles successfully with `cargo build`
- All tests pass with `cargo test`
- No compiler errors or warnings related to FinishReason