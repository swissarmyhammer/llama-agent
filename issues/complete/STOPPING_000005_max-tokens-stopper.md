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

## Proposed Solution

Based on analysis of the specification and existing code:

1. **Implementation Strategy:**
   - Track `tokens_generated` count in the MaxTokensStopper state 
   - In `should_stop()`, increment the count by the number of tokens being processed in the current batch
   - Use `batch.n_tokens()` to get the number of tokens in the current batch
   - Return `FinishReason::Stopped("Maximum tokens reached")` when limit is exceeded

2. **Key Implementation Details:**
   - The stopper is stateful - maintains token count across multiple calls
   - Each call to `should_stop()` receives a batch that may contain multiple tokens
   - Need to increment by batch size, not just by 1
   - Check limit after incrementing to ensure we stop at the exact limit

3. **Testing Approach:**
   - Unit tests with mock batches of varying sizes
   - Test exact limit detection
   - Test multiple small batches building up to limit
   - Test edge cases (zero max_tokens, empty batches)

4. **Thread Safety:**
   - The stopper is designed for single request usage (fresh instance per request)
   - No additional synchronization needed as each request gets its own instance
## Implementation Complete ✅

The MaxTokensStopper has been successfully implemented with the following features:

### Implementation Details
- **Token Counting**: Accurately tracks tokens across multiple batch calls using `batch.n_tokens()`
- **State Management**: Maintains `tokens_generated` count across calls (stateful stopper)
- **Limit Detection**: Returns `FinishReason::Stopped("Maximum tokens reached")` when limit is reached or exceeded
- **Type Safety**: Proper handling of i32 to usize conversion for token counts

### Key Features
- ✅ Implements the `Stopper` trait correctly
- ✅ Thread-safe (implements Send + Sync)
- ✅ Handles edge cases (zero limit, empty batches, large limits)
- ✅ Accurate token counting with batch processing
- ✅ Stops exactly at or when exceeding the configured limit

### Testing
- ✅ Comprehensive unit tests covering all scenarios
- ✅ Tests pass with cargo test
- ✅ Code formatted with rustfmt
- ✅ Passes clippy linting without new warnings
- ✅ Integration with existing stopper module exports

### Usage
```rust
let mut stopper = MaxTokensStopper::new(100);
let result = stopper.should_stop(&context, &batch);
match result {
    Some(FinishReason::Stopped(reason)) => {
        // Generation stopped due to token limit
        println!("Stopped: {}", reason);
    }
    None => {
        // Continue generation
    }
}
```

The implementation is complete and ready for production use.