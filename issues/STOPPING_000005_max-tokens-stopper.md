# Implement MaxTokensStopper for Token Limit Detection

Refer to ./specification/stopping.md

## Objective

Implement the MaxTokensStopper for tracking total tokens generated and stopping when a configured maximum is reached.

## Tasks

### 1. Implement MaxTokensStopper in src/stopper/max_tokens.rs
```rust
use crate::stopper::Stopper;
use crate::types::FinishReason;
use llama_cpp_2::{llama_batch::LlamaBatch, context::LlamaContext};

pub struct MaxTokensStopper {
    max_tokens: usize,
    token_count: usize,
}

impl MaxTokensStopper {
    pub fn new(max_tokens: usize) -> Self {
        Self {
            max_tokens,
            token_count: 0,
        }
    }
}

impl Stopper for MaxTokensStopper {
    fn should_stop(&mut self, _context: &LlamaContext, batch: &LlamaBatch) -> Option<FinishReason> {
        // Implementation details from specification
    }
}
```

### 2. Implementation Logic
- Increment token count for each new token in the batch
- Return `FinishReason::Stopped("Maximum tokens reached")` when limit exceeded
- Track tokens across multiple calls (stateful)
- Handle batch processing correctly

### 3. Add to Module Exports
- Re-export MaxTokensStopper in `src/stopper/mod.rs`

### 4. Unit Tests
Create comprehensive tests:
- Test token counting accuracy
- Test stopping at exact limit
- Test multiple batches building up to limit
- Test edge cases (zero max_tokens, empty batches)

## Implementation Notes

- This stopper is stateful (tracks token count across calls)
- Be careful with token counting - ensure accuracy
- Consider how to handle tokens in batch (may be multiple tokens per call)
- Ensure thread safety since this has mutable state

## Acceptance Criteria

- MaxTokensStopper struct and implementation complete
- Token counting works accurately across multiple calls
- Stops exactly when max_tokens reached
- Unit tests cover all important cases
- Code compiles with no warnings
- Integration with stopper trait works correctly