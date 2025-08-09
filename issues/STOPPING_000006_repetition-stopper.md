# Implement RepetitionStopper for Loop Detection

Refer to ./specification/stopping.md

## Objective

Implement the RepetitionStopper for detecting repetitive patterns in generated text to prevent infinite loops, inspired by Gemini CLI loop detection.

## Tasks

### 1. Implement RepetitionStopper in src/stopper/repetition.rs
```rust
use crate::stopper::Stopper;
use crate::types::{FinishReason, RepetitionConfig};
use llama_cpp_2::{llama_batch::LlamaBatch, context::LlamaContext};
use std::collections::VecDeque;

pub struct RepetitionStopper {
    config: RepetitionConfig,
    text_window: VecDeque<String>,
    current_window_size: usize,
}

impl RepetitionStopper {
    pub fn new(config: RepetitionConfig) -> Self {
        // Implementation
    }
}
```

### 2. Implementation Algorithm
Following the specification's detection algorithm:
1. Maintain a sliding window of recent generated text
2. For each pattern length from min to max:
   - Extract the most recent pattern of that length
   - Count consecutive occurrences in the window
   - If occurrences >= min_repetitions, return stop reason
3. Use efficient string matching

### 3. Key Implementation Details
- Convert tokens to text for pattern analysis
- Maintain bounded sliding window (memory management)
- Implement efficient pattern matching (consider Boyer-Moore or similar)
- Return descriptive messages: `"Repetition detected: {pattern} repeated {count} times"`

### 4. Add to Module Exports
- Re-export RepetitionStopper in `src/stopper/mod.rs`

### 5. Comprehensive Unit Tests
- Test pattern detection at various lengths
- Test minimum repetition thresholds
- Test sliding window behavior
- Test memory bounds (window size limits)
- Test edge cases (empty input, single tokens)

## Implementation Notes

- This is the most complex stopper - focus on correctness first, optimize later
- Memory usage must be bounded by window_size
- Consider performance impact - this will be called frequently
- Pattern matching should be efficient but correct
- Handle text conversion from tokens carefully

## Acceptance Criteria

- RepetitionStopper correctly detects patterns at configured lengths
- Memory usage bounded by window_size configuration
- Accurate repetition counting and threshold detection
- Comprehensive test coverage for all scenarios
- No performance regression in basic generation cases
- Clear, descriptive stop messages for debugging