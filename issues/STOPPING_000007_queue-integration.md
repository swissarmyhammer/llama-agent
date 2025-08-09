# Integrate Stoppers into Queue Processing

Refer to ./specification/stopping.md

## Objective

Integrate the stopper system into the existing queue processing functions `process_streaming_request_sync` and `process_batch_request_sync` as specified.

## Tasks

### 1. Modify process_streaming_request_sync in queue.rs
Add stopper integration following the specification pattern:
```rust
// Create fresh stoppers for each request
let mut stoppers: Vec<Box<dyn Stopper>> = vec![
    Box::new(EosStopper::new(model.eos_token_id())),
    Box::new(MaxTokensStopper::new(request.max_tokens.unwrap_or(4096))),
    Box::new(RepetitionStopper::new(request.stopping_config
        .as_ref()
        .and_then(|c| c.repetition_detection.clone())
        .unwrap_or_default())),
];

// Check during processing
for stopper in &mut stoppers {
    if let Some(reason) = stopper.should_stop(&context, &batch) {
        return ProcessResult::Finished { reason };
    }
}
```

### 2. Modify process_batch_request_sync in queue.rs
Apply the same integration pattern to batch processing

### 3. Handle Configuration
- Extract stopping configuration from GenerationRequest
- Use reasonable defaults when configuration is None
- Integrate with existing max_tokens handling

### 4. Update Generation Loop
- Add stopper checking to the main generation loops
- Ensure stoppers are called at appropriate points
- Handle early termination gracefully

### 5. Import Required Types
- Add necessary imports for stopper types
- Ensure LlamaContext is available for stopper calls

## Implementation Notes

- Stoppers are created fresh for each request (no shared state)
- Check stoppers after each batch processing step
- Preserve existing functionality while adding stopping
- Handle the case where max_tokens comes from different sources
- Ensure performance impact is minimal

## Acceptance Criteria

- Both streaming and batch processing integrate stoppers
- Stoppers are created fresh per request with proper configuration
- Generation stops correctly when any stopper triggers
- Existing functionality preserved (backward compatibility)
- No performance regression in normal operation
- Integration tests pass with all stopper types