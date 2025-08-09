# Stopping Specification

## Overview

The `Stopper` trait and `stopper` module provide a flexible mechanism for determining when to stop generation in streaming and batch request processing. This system allows multiple stopping conditions to be evaluated and provides detailed reasons for why generation was terminated.

## Module Structure

```
src/
├── stopper/
│   ├── mod.rs           # Main module with Stopper trait
│   ├── max_tokens.rs    # MaxTokensStopper implementation
│   ├── repetition.rs    # RepetitionStopper implementation
│   └── eos.rs           # EosStopper implementation
```

## Core Trait

```rust
pub trait Stopper {
    fn should_stop(&mut self, context: &LlamaContext, batch: &LlamaBatch) -> Option<FinishReason>;
}
```

## Finish Reasons

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FinishReason {
    Stopped(String),
}
```

**Migration Note:** The existing `FinishReason` enum should be simplified to only have the `Stopped(String)` variant. Remove any existing variants such as:
- `MaxTokens` - Replace with `Stopped("Maximum tokens reached")`  
- `Eos` - Replace with `Stopped("End of sequence token detected")`
- Any other specific stopping reason variants

All stopping conditions now use the unified `Stopped(String)` variant with descriptive messages.

## Stopper Implementations

### MaxTokensStopper

Tracks total tokens generated and stops when a configured maximum is reached.

**Configuration:**
- `max_tokens: usize` - Maximum number of tokens to generate

**Logic:**
- Increment token count for each new token in the batch
- Return `FinishReason::Stopped("Maximum tokens reached")` when limit exceeded

### RepetitionStopper

Detects repetitive patterns in generated text to prevent infinite loops, inspired by the Gemini CLI loop detection service.

**Configuration:**
- `min_pattern_length: usize` - Minimum length of patterns to detect (default: 10)
- `max_pattern_length: usize` - Maximum length of patterns to detect (default: 100)
- `min_repetitions: usize` - Minimum repetitions to trigger stop (default: 3)
- `window_size: usize` - Size of recent text to analyze (default: 1000)

**Detection Algorithm:**
1. Maintain a sliding window of recent generated text
2. For each pattern length from min to max:
   - Extract the most recent pattern of that length
   - Count consecutive occurrences of this pattern in the window
   - If occurrences >= min_repetitions, return stop reason
3. Use efficient string matching (Boyer-Moore or similar)

**Logic:**
- Analyze the most recent tokens in the batch
- Look for repeating subsequences of various lengths
- Return `FinishReason::Stopped("Repetition detected: {pattern} repeated {count} times")` with descriptive message

### EosStopper

Detects End-of-Sequence (EOS) tokens in the generated output.

**Configuration:**
- `eos_token_id: u32` - The token ID that represents end-of-sequence

**Logic:**
- Check each token in the batch against the configured EOS token ID
- Return `FinishReason::Stopped("End of sequence token detected")` when EOS token is found

## Integration Points

### In queue.rs

Both `process_streaming_request_sync` and `process_batch_request_sync` will be modified to:

1. **Per-Request Initialization:**
   ```rust
   // Create fresh stoppers for each request
   let mut stoppers: Vec<Box<dyn Stopper>> = vec![
       Box::new(EosStopper::new(model.eos_token_id())),
       Box::new(MaxTokensStopper::new(request.max_tokens)),
       Box::new(RepetitionStopper::new(request.repetition_config)),
   ];
   ```

2. **Check During Processing:**
   ```rust
   for stopper in &mut stoppers {
       if let Some(reason) = stopper.should_stop(&context, &batch) {
           return ProcessResult::Finished { reason };
       }
   }
   ```

Note: Stoppers are created fresh for each request and automatically dropped when the request completes. No manual reset is needed since each request gets its own stopper instances.

## Configuration

Stopping behavior should be configurable via:

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

## Error Handling

- Stoppers should be resilient and never panic
- Invalid configurations should be caught at initialization
- Errors during checking should log warnings but not crash processing

## Performance Considerations

- Stoppers will be called frequently during generation
- RepetitionStopper should use efficient algorithms (sliding window, string matching)
- Consider caching and incremental updates where possible
- Memory usage should be bounded (fixed-size windows)

## Testing Requirements

- Unit tests for each stopper implementation
- Integration tests with real LlamaContext and LlamaBatch using unsloth/Qwen3-0.6B-GGUF model
- Performance benchmarks for high-frequency checking
- Edge case testing (empty batches, single tokens, etc.)
