# Request Queue Infrastructure

Refer to ./specifications/index.md

## Objective
Implement thread-safe request queue for serializing access to the single model instance.

## Tasks
- [ ] Create `queue.rs` module with RequestQueue struct
- [ ] Implement QueuedRequest internal type with oneshot response channels
- [ ] Create worker thread pool for processing requests
- [ ] Add support for both batch and streaming requests
- [ ] Implement request timeout handling
- [ ] Add queue capacity limits and backpressure
- [ ] Create proper shutdown handling for worker threads
- [ ] Add queue health monitoring and metrics

## Architecture
- MPSC channel for request submission
- Worker threads process requests sequentially against model
- Oneshot channels for batch request responses
- MPSC channels for streaming request chunks
- Arc<Mutex<>> protection for shared model access

## Key Methods
- `RequestQueue::new(model: Arc<LlamaModel>, config: QueueConfig)`
- `submit_request(request: GenerationRequest) -> Result<GenerationResponse>`
- `submit_streaming_request(request) -> Result<impl Stream<StreamChunk>>`
- Proper graceful shutdown with `shutdown()` method

## Error Handling
- QueueError variants for timeouts, capacity, worker failures
- Proper error propagation from workers to callers
- Request cancellation support
- Worker thread panic recovery

## Acceptance Criteria
- Queue handles concurrent requests safely
- Worker threads process requests in order
- Timeout handling works correctly
- Queue capacity limits prevent memory issues
- Streaming requests work without blocking batch requests
- Graceful shutdown cleans up all resources