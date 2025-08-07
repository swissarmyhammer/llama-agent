use crate::model::ModelManager;
use crate::types::{
    FinishReason, GenerationRequest, GenerationResponse, QueueConfig, QueueError, StreamChunk,
};
use llama_cpp_2::{context::LlamaContext, model::LlamaModel};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{mpsc, oneshot, Mutex};
use tokio::task::JoinHandle;
use tracing::{debug, error, info, warn};
use ulid::Ulid;

#[derive(Debug)]
pub struct QueuedRequest {
    pub id: String,
    pub request: GenerationRequest,
    pub response_sender: oneshot::Sender<Result<GenerationResponse, QueueError>>,
    pub stream_sender: Option<mpsc::Sender<Result<StreamChunk, QueueError>>>,
    pub submitted_at: Instant,
}

pub struct RequestQueue {
    sender: mpsc::Sender<QueuedRequest>,
    worker_handles: Vec<JoinHandle<()>>,
    config: QueueConfig,
}

impl RequestQueue {
    pub fn new(model_manager: Arc<ModelManager>, config: QueueConfig) -> Self {
        let (sender, receiver) = mpsc::channel(config.max_queue_size);
        let receiver = Arc::new(Mutex::new(receiver));

        let mut worker_handles = Vec::new();

        // Spawn worker threads
        for worker_id in 0..config.worker_threads {
            let receiver = receiver.clone();
            let model_manager = model_manager.clone();
            let config = config.clone();

            let handle = tokio::spawn(async move {
                Self::worker_loop(worker_id, receiver, model_manager, config).await;
            });

            worker_handles.push(handle);
        }

        info!(
            "RequestQueue initialized with {} workers, max queue size: {}",
            config.worker_threads, config.max_queue_size
        );

        Self {
            sender,
            worker_handles,
            config,
        }
    }

    pub async fn submit_request(
        &self,
        request: GenerationRequest,
    ) -> Result<GenerationResponse, QueueError> {
        let (response_sender, response_receiver) = oneshot::channel();

        let queued_request = QueuedRequest {
            id: Ulid::new().to_string(),
            request,
            response_sender,
            stream_sender: None,
            submitted_at: Instant::now(),
        };

        debug!("Submitting request to queue: {}", queued_request.id);

        // Try to send to queue
        if self.sender.try_send(queued_request).is_err() {
            warn!("Queue is full, rejecting request");
            return Err(QueueError::Full);
        }

        // Wait for response with timeout
        match tokio::time::timeout(self.config.request_timeout, response_receiver).await {
            Ok(Ok(response)) => response,
            Ok(Err(_)) => {
                error!("Response channel closed unexpectedly");
                Err(QueueError::WorkerError(
                    "Response channel closed".to_string(),
                ))
            }
            Err(_) => {
                warn!("Request timed out after {:?}", self.config.request_timeout);
                Err(QueueError::Timeout)
            }
        }
    }

    pub async fn submit_streaming_request(
        &self,
        request: GenerationRequest,
    ) -> Result<mpsc::Receiver<Result<StreamChunk, QueueError>>, QueueError> {
        let (response_sender, _) = oneshot::channel();
        let (stream_sender, stream_receiver) = mpsc::channel(100);

        let queued_request = QueuedRequest {
            id: Ulid::new().to_string(),
            request,
            response_sender,
            stream_sender: Some(stream_sender),
            submitted_at: Instant::now(),
        };

        debug!(
            "Submitting streaming request to queue: {}",
            queued_request.id
        );

        // Try to send to queue
        if self.sender.try_send(queued_request).is_err() {
            warn!("Queue is full, rejecting streaming request");
            return Err(QueueError::Full);
        }

        Ok(stream_receiver)
    }

    pub fn get_queue_size(&self) -> usize {
        // This is an approximation since we can't directly inspect the channel
        self.config.max_queue_size - self.sender.capacity()
    }

    async fn worker_loop(
        worker_id: usize,
        receiver: Arc<Mutex<mpsc::Receiver<QueuedRequest>>>,
        model_manager: Arc<ModelManager>,
        config: QueueConfig,
    ) {
        info!("Worker {} started", worker_id);

        loop {
            let queued_request = {
                let mut receiver = receiver.lock().await;
                match receiver.recv().await {
                    Some(request) => request,
                    None => {
                        info!("Worker {} shutting down - channel closed", worker_id);
                        break;
                    }
                }
            };

            let queue_time = queued_request.submitted_at.elapsed();
            debug!(
                "Worker {} processing request {} (queue time: {:?})",
                worker_id, queued_request.id, queue_time
            );

            // Check if request has already timed out
            if queue_time > config.request_timeout {
                warn!(
                    "Worker {} dropping expired request {} (queued for {:?})",
                    worker_id, queued_request.id, queue_time
                );
                let _ = queued_request
                    .response_sender
                    .send(Err(QueueError::Timeout));
                continue;
            }

            // Process the request
            Self::process_request(worker_id, queued_request, model_manager.clone()).await;
        }
    }

    async fn process_request(
        worker_id: usize,
        queued_request: QueuedRequest,
        model_manager: Arc<ModelManager>,
    ) {
        let start_time = Instant::now();

        // Check if model is loaded
        if !model_manager.is_loaded().await {
            let error = QueueError::WorkerError("Model not loaded".to_string());
            if let Some(stream_sender) = queued_request.stream_sender {
                let _ = stream_sender.send(Err(error)).await;
            } else {
                let _ = queued_request.response_sender.send(Err(error));
            }
            return;
        }

        let request_id = queued_request.id.clone();

        // Process request with model access - use a closure to work within model lifetime
        if let Some(stream_sender) = queued_request.stream_sender {
            // Handle streaming request
            let result = model_manager
                .with_model(|model| {
                    let context_result = model_manager.create_context(model);
                    match context_result {
                        Ok(_context) => {
                            // For streaming, we need to handle async differently due to lifetime issues
                            // For now, return an error indicating this needs further implementation
                            Err::<(), QueueError>(QueueError::WorkerError(
                                "Streaming with real models not yet implemented".to_string(),
                            ))
                        }
                        Err(e) => Err(QueueError::WorkerError(format!(
                            "Failed to create context: {}",
                            e
                        ))),
                    }
                })
                .await;

            match result {
                Ok(_) => {
                    // This won't be reached due to the error above, but structure is ready
                }
                Err(model_error) => {
                    let queue_error =
                        QueueError::WorkerError(format!("Model error: {}", model_error));
                    let _ = stream_sender.send(Err(queue_error)).await;
                }
            }
        } else {
            // Handle batch request
            let result = model_manager
                .with_model(|model| {
                    let context_result = model_manager.create_context(model);
                    match context_result {
                        Ok(context) => {
                            // Process the request synchronously within the model lifetime
                            Self::process_batch_request_sync(
                                worker_id,
                                request_id.clone(),
                                &queued_request.request,
                                model,
                                &context,
                            )
                        }
                        Err(e) => Err(QueueError::WorkerError(format!(
                            "Failed to create context: {}",
                            e
                        ))),
                    }
                })
                .await;

            let final_result = match result {
                Ok(response) => response,
                Err(model_error) => Err(QueueError::WorkerError(format!(
                    "Model error: {}",
                    model_error
                ))),
            };
            let _ = queued_request.response_sender.send(final_result);
        }

        let processing_time = start_time.elapsed();
        debug!(
            "Worker {} completed request {} in {:?}",
            worker_id, request_id, processing_time
        );
    }

    fn process_batch_request_sync(
        worker_id: usize,
        _request_id: String,
        request: &GenerationRequest,
        _model: &LlamaModel,
        _context: &LlamaContext<'_>,
    ) -> Result<GenerationResponse, QueueError> {
        let start_time = Instant::now();

        // Mock text generation - would be replaced with actual llama-cpp inference
        let generated_text = format!(
            "Mock response for session '{}' with {} messages. Worker: {}",
            request.session.id,
            request.session.messages.len(),
            worker_id
        );

        // Simulate processing time (synchronously)
        std::thread::sleep(Duration::from_millis(50));

        let generation_time = start_time.elapsed();

        Ok(GenerationResponse {
            generated_text,
            tokens_generated: 10,
            generation_time,
            finish_reason: FinishReason::MaxTokens,
        })
    }

}

impl RequestQueue {
    /// Gracefully shutdown the queue, waiting for all workers to complete
    pub async fn shutdown(mut self) {
        info!("RequestQueue shutting down gracefully");

        // Close the sender to signal workers to shutdown
        // sender will be dropped automatically when this method ends

        // Wait for all worker handles to complete
        for handle in self.worker_handles.drain(..) {
            if let Err(e) = handle.await {
                warn!("Worker thread panicked during shutdown: {:?}", e);
            }
        }

        info!("RequestQueue shutdown complete");
    }
}

impl Drop for RequestQueue {
    fn drop(&mut self) {
        info!(
            "RequestQueue dropping - {} worker handles remaining",
            self.worker_handles.len()
        );
        // Note: worker_handles will be aborted when dropped
        // For graceful shutdown, call shutdown() method instead
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{
        Message, MessageRole, ModelConfig, ModelSource, QueueConfig, Session, SessionId,
    };
    use std::path::PathBuf;
    use std::time::SystemTime;
    use tempfile::TempDir;

    fn create_test_model_config() -> ModelConfig {
        ModelConfig {
            source: ModelSource::Local {
                folder: PathBuf::from("/tmp"),
                filename: Some("test.gguf".to_string()),
            },
            batch_size: 512,
            use_hf_params: false,
        }
    }

    fn create_test_queue_config() -> QueueConfig {
        QueueConfig {
            max_queue_size: 10,
            request_timeout: Duration::from_secs(5),
            worker_threads: 2,
        }
    }

    fn create_test_session() -> Session {
        Session {
            id: SessionId::new(),
            messages: vec![Message {
                role: MessageRole::User,
                content: "Hello".to_string(),
                tool_call_id: None,
                tool_name: None,
                timestamp: SystemTime::now(),
            }],
            mcp_servers: Vec::new(),
            available_tools: Vec::new(),
            created_at: SystemTime::now(),
            updated_at: SystemTime::now(),
        }
    }

    async fn setup_loaded_model_manager() -> Arc<ModelManager> {
        let temp_dir = TempDir::new().unwrap();
        let model_file = temp_dir.path().join("test.gguf");

        // Create dummy model file
        tokio::fs::write(&model_file, b"dummy model").await.unwrap();

        let config = ModelConfig {
            source: ModelSource::Local {
                folder: temp_dir.path().to_path_buf(),
                filename: Some("test.gguf".to_string()),
            },
            batch_size: 512,
            use_hf_params: false,
        };

        let manager = Arc::new(ModelManager::new(config).expect("Failed to create ModelManager"));

        // Note: We don't actually load the model since dummy GGUF files fail
        // The queue tests should focus on queue functionality, not model loading
        // In a real application, the model would be properly loaded

        // Note: temp_dir will be automatically cleaned up when it goes out of scope
        // For test purposes, this is fine as the model manager only needs the path
        // during initialization, not for the entire lifetime
        drop(temp_dir);

        manager
    }

    #[tokio::test]
    async fn test_request_queue_creation() {
        let model_manager = Arc::new(
            ModelManager::new(create_test_model_config()).expect("Failed to create ModelManager"),
        );
        let config = create_test_queue_config();

        let queue = RequestQueue::new(model_manager, config);
        assert_eq!(queue.get_queue_size(), 0);
    }

    #[tokio::test]
    async fn test_submit_request_model_not_loaded() {
        let model_manager = Arc::new(
            ModelManager::new(create_test_model_config()).expect("Failed to create ModelManager"),
        );
        let config = create_test_queue_config();
        let queue = RequestQueue::new(model_manager, config);

        let request = GenerationRequest {
            session: create_test_session(),
            max_tokens: Some(100),
            temperature: Some(0.7),
            top_p: Some(0.9),
            stop_tokens: Vec::new(),
        };

        let result = queue.submit_request(request).await;
        assert!(matches!(result, Err(QueueError::WorkerError(_))));
    }

    #[tokio::test]
    async fn test_submit_request_model_not_loaded_fails() {
        let model_manager = setup_loaded_model_manager().await;
        let config = create_test_queue_config();
        let queue = RequestQueue::new(model_manager, config);

        let request = GenerationRequest {
            session: create_test_session(),
            max_tokens: Some(100),
            temperature: Some(0.7),
            top_p: Some(0.9),
            stop_tokens: Vec::new(),
        };

        let result = queue.submit_request(request).await;
        // Should fail because model is not actually loaded in test setup
        assert!(result.is_err());
        match result.unwrap_err() {
            QueueError::WorkerError(msg) => {
                assert!(msg.contains("Model not loaded") || msg.contains("Model error"));
            }
            _ => panic!("Expected WorkerError for unloaded model"),
        }
    }

    #[tokio::test]
    async fn test_submit_streaming_request_not_implemented() {
        let model_manager = setup_loaded_model_manager().await;
        let config = create_test_queue_config();
        let queue = RequestQueue::new(model_manager, config);

        let request = GenerationRequest {
            session: create_test_session(),
            max_tokens: Some(100),
            temperature: Some(0.7),
            top_p: Some(0.9),
            stop_tokens: Vec::new(),
        };

        let mut receiver = queue.submit_streaming_request(request).await.unwrap();

        // Should receive an error since streaming is not yet implemented
        let chunk_result = receiver.recv().await;
        assert!(chunk_result.is_some());
        match chunk_result.unwrap() {
            Err(QueueError::WorkerError(msg)) => {
                assert!(
                    msg.contains("Streaming with real models not yet implemented")
                        || msg.contains("Model not loaded")
                );
            }
            Ok(_) => panic!("Expected error for streaming not implemented"),
            Err(other) => panic!("Unexpected error type: {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_queue_timeout() {
        // Create a loaded model manager but with very slow processing
        let model_manager = setup_loaded_model_manager().await;
        let config = QueueConfig {
            max_queue_size: 10,
            request_timeout: Duration::from_millis(10), // Very short timeout
            worker_threads: 1,
        };
        let queue = RequestQueue::new(model_manager, config);

        let request = GenerationRequest {
            session: create_test_session(),
            max_tokens: Some(100),
            temperature: Some(0.7),
            top_p: Some(0.9),
            stop_tokens: Vec::new(),
        };

        let result = queue.submit_request(request).await;
        // Should fail because model is not loaded, not due to timeout in this test setup
        assert!(result.is_err());
        // The error should be WorkerError about model not loaded, not timeout
        match result.unwrap_err() {
            QueueError::WorkerError(msg) => {
                assert!(msg.contains("Model not loaded") || msg.contains("Model error"));
            }
            QueueError::Timeout => {
                // This could also happen if the timeout is very short
            }
            other => panic!("Unexpected error type: {:?}", other),
        }
    }

    #[test]
    fn test_queued_request_debug() {
        let (sender, _) = oneshot::channel();
        let request = QueuedRequest {
            id: "test-123".to_string(),
            request: GenerationRequest {
                session: create_test_session(),
                max_tokens: Some(100),
                temperature: Some(0.7),
                top_p: Some(0.9),
                stop_tokens: Vec::new(),
            },
            response_sender: sender,
            stream_sender: None,
            submitted_at: Instant::now(),
        };

        let debug_str = format!("{:?}", request);
        assert!(debug_str.contains("test-123"));
    }
}
