# Comprehensive Integration Testing with Real Model

Refer to ./specification/stopping.md

## Objective

Create comprehensive integration tests using the unsloth/Qwen3-0.6B-GGUF model to validate all stopper functionality works correctly with real model inference.

## Tasks

### 1. Create Integration Test Module
Create `tests/stopper_integration_tests.rs`:
- Test all stoppers with real LlamaModel and LlamaContext
- Use unsloth/Qwen3-0.6B-GGUF as specified in requirements
- Test both streaming and batch processing paths

### 2. EosStopper Integration Tests
- Test EOS detection with real model inference
- Verify EOS token ID detection works correctly
- Test edge cases with different model configurations

### 3. MaxTokensStopper Integration Tests
- Test token counting accuracy during real generation
- Test stopping at various token limits (1, 10, 100, 1000)
- Verify token counts match expected values

### 4. RepetitionStopper Integration Tests
- Generate repetitive text patterns and verify detection
- Test different pattern lengths and repetition counts
- Test sliding window behavior with real text generation
- Verify memory usage stays bounded

### 5. Combined Stopper Tests
- Test multiple stoppers working together
- Test precedence when multiple stop conditions occur
- Test configuration combinations

### 6. Performance Benchmarks
- Measure performance impact of stopper system
- Ensure < 5% throughput degradation as specified
- Test high-frequency stopper checking performance

## Implementation Notes

- Use real model for authentic testing conditions
- Test both happy path and edge cases
- Include performance regression tests
- Test concurrent request handling
- Verify thread safety with multiple concurrent generations

## Acceptance Criteria

- All integration tests pass with real model
- Performance impact < 5% of baseline throughput
- All stopper types work correctly in isolation and combination
- Thread safety verified with concurrent tests
- Edge cases handled gracefully
- Memory usage stays bounded for RepetitionStopper
- No memory leaks or resource issues