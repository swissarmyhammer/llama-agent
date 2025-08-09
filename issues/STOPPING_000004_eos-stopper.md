# Implement EosStopper for End-of-Sequence Detection

Refer to ./specification/stopping.md

## Objective

Implement the EosStopper for detecting end-of-sequence tokens as the first concrete stopper implementation.

## Tasks

### 1. Implement EosStopper in src/stopper/eos.rs
```rust
use crate::stopper::Stopper;
use crate::types::FinishReason;
use llama_cpp_2::{llama_batch::LlamaBatch, context::LlamaContext};

pub struct EosStopper {
    eos_token_id: u32,
}

impl EosStopper {
    pub fn new(eos_token_id: u32) -> Self {
        Self { eos_token_id }
    }
}

impl Stopper for EosStopper {
    fn should_stop(&mut self, _context: &LlamaContext, batch: &LlamaBatch) -> Option<FinishReason> {
        // Implementation details from specification
    }
}
```

### 2. Implementation Logic
- Check each token in the batch against the configured EOS token ID
- Return `FinishReason::Stopped("End of sequence token detected")` when EOS token found
- Handle edge cases (empty batch, multiple tokens)

### 3. Add to Module Exports
- Re-export EosStopper in `src/stopper/mod.rs`
- Ensure proper module organization

### 4. Basic Unit Tests
Create tests in `src/stopper/eos.rs`:
- Test EOS token detection
- Test non-EOS tokens (should not stop)
- Test empty batch handling

## Implementation Notes

- This is the simplest stopper implementation - use it to validate the pattern
- Focus on correctness over performance optimization
- Handle all edge cases gracefully (no panics)
- Ensure thread safety for concurrent usage

## Acceptance Criteria

- EosStopper struct and implementation complete
- should_stop method works correctly for EOS detection
- Basic unit tests pass
- Code compiles with no warnings
- Integration with stopper trait works correctly

## Proposed Solution

Based on the specification and existing code structure, I'll implement the EosStopper as follows:

1. **Implementation Strategy**: Replace the TODO in `should_stop` method to check each token in the batch against the configured EOS token ID.

2. **Key Implementation Details**:
   - Use LlamaBatch API to iterate through tokens
   - Check each token against the stored `eos_token_id`  
   - Return `FinishReason::Stopped("End of sequence token detected")` when EOS found
   - Handle edge cases: empty batch (return None to continue)
   - Ensure thread safety (stopper is stateless except for eos_token_id)

3. **Testing Approach**:
   - Unit tests for EOS detection with mock/test tokens
   - Unit tests for non-EOS tokens (should not stop)
   - Unit tests for empty batch handling
   - Tests should use actual LlamaBatch structures

4. **Integration Requirements**:
   - Verify module exports are correct
   - Ensure compatibility with existing Stopper trait
   - No changes needed to types.rs (FinishReason already has correct structure)

This implementation will serve as the foundational pattern for other stoppers and validate the overall stopping system architecture.