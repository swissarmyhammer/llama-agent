use crate::types::{
    GenerationRequest, GenerationResponse, StreamChunk, FinishReason, QueueError, QueueConfig,
};
use crate::model::{ModelManager, MockModel, MockContext};
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
    model_manager: Arc<ModelManager>,
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
            model_manager,
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
        if let Err(_) = self.sender.try_send(queued_request) {
            warn!("Queue is full, rejecting request");
            return Err(QueueError::Full);
        }
        
        // Wait for response with timeout
        match tokio::time::timeout(self.config.request_timeout, response_receiver).await {
            Ok(Ok(response)) => response,
            Ok(Err(_)) => {
                error!("Response channel closed unexpectedly");
                Err(QueueError::WorkerError("Response channel closed".to_string()))
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
        
        debug!("Submitting streaming request to queue: {}", queued_request.id);
        
        // Try to send to queue
        if let Err(_) = self.sender.try_send(queued_request) {
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
                let _ = queued_request.response_sender.send(Err(QueueError::Timeout));
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
        
        // Get model and context
        let model = match model_manager.get_model().await {
            Some(model) => model,
            None => {
                let error = QueueError::WorkerError("Model not available".to_string());
                if let Some(stream_sender) = queued_request.stream_sender {
                    let _ = stream_sender.send(Err(error)).await;
                } else {
                    let _ = queued_request.response_sender.send(Err(error));
                }
                return;
            }
        };
        
        let context = match model_manager.get_context().await {
            Some(context) => context,
            None => {
                let error = QueueError::WorkerError("Context not available".to_string());
                if let Some(stream_sender) = queued_request.stream_sender {
                    let _ = stream_sender.send(Err(error)).await;
                } else {
                    let _ = queued_request.response_sender.send(Err(error));
                }
                return;
            }
        };
        
        let request_id = queued_request.id.clone();
        
        // Handle streaming vs non-streaming request
        if let Some(stream_sender) = queued_request.stream_sender {
            Self::process_streaming_request(
                worker_id,
                request_id.clone(),
                queued_request.request,
                model,
                context,
                stream_sender,
            ).await;
        } else {
            let response = Self::process_batch_request(
                worker_id,
                request_id.clone(),
                queued_request.request,
                model,
                context,
            ).await;
            
            let _ = queued_request.response_sender.send(response);
        }
        
        let processing_time = start_time.elapsed();
        debug!(
            "Worker {} completed request {} in {:?}",
            worker_id, request_id, processing_time
        );
    }
    
    async fn process_batch_request(
        worker_id: usize,
        _request_id: String,
        request: GenerationRequest,
        _model: Arc<MockModel>,
        _context: Arc<MockContext>,
    ) -> Result<GenerationResponse, QueueError> {
        let start_time = Instant::now();
        
        // Mock text generation
        let generated_text = format!(
            "Mock response for session '{}' with {} messages. Worker: {}",
            request.session.id,
            request.session.messages.len(),
            worker_id
        );
        
        // Simulate processing time
        tokio::time::sleep(Duration::from_millis(50)).await;
        
        let generation_time = start_time.elapsed();
        
        Ok(GenerationResponse {
            generated_text,
            tokens_generated: 10,
            generation_time,
            finish_reason: FinishReason::MaxTokens,
        })
    }
    
    async fn process_streaming_request(
        _worker_id: usize,
        request_id: String,
        request: GenerationRequest,
        _model: Arc<MockModel>,
        _context: Arc<MockContext>,
        stream_sender: mpsc::Sender<Result<StreamChunk, QueueError>>,
    ) {
        let session_id_str = request.session.id.to_string();
        let words = vec!["Mock", "streaming", "response", "for", "session", &session_id_str];
        let mut token_count = 0;
        
        for (i, word) in words.iter().enumerate() {
            let chunk = StreamChunk {
                text: if i == 0 { word.to_string() } else { format!(" {}", word) },
                is_complete: i == words.len() - 1,
                token_count: token_count + 1,
            };
            
            token_count += 1;
            
            if stream_sender.send(Ok(chunk)).await.is_err() {
                debug!("Stream receiver dropped for request {}", request_id);
                break;
            }
            
            // Simulate streaming delay
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    }
}

impl Drop for RequestQueue {
    fn drop(&mut self) {
        info!("RequestQueue shutting down");
        // Close the sender to signal workers to shutdown
        // The receiver channels will be closed when sender is dropped
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{
        ModelConfig, ModelSource, QueueConfig, Session, Message, MessageRole, SessionId
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
            messages: vec![
                Message {
                    role: MessageRole::User,
                    content: "Hello".to_string(),
                    tool_call_id: None,
                    tool_name: None,
                    timestamp: SystemTime::now(),
                }
            ],
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
        
        let manager = Arc::new(ModelManager::new(config));
        manager.load_model().await.unwrap();
        
        // Keep temp_dir alive
        std::mem::forget(temp_dir);
        
        manager
    }
    
    #[tokio::test]
    async fn test_request_queue_creation() {
        let model_manager = Arc::new(ModelManager::new(create_test_model_config()));
        let config = create_test_queue_config();
        
        let queue = RequestQueue::new(model_manager, config);
        assert_eq!(queue.get_queue_size(), 0);
    }
    
    #[tokio::test]
    async fn test_submit_request_model_not_loaded() {
        let model_manager = Arc::new(ModelManager::new(create_test_model_config()));
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
    async fn test_submit_request_success() {
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
        assert!(result.is_ok());
        
        let response = result.unwrap();
        assert!(!response.generated_text.is_empty());
        assert_eq!(response.tokens_generated, 10);
        assert_eq!(response.finish_reason, FinishReason::MaxTokens);
    }
    
    #[tokio::test]
    async fn test_submit_streaming_request_success() {
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
        
        let mut chunks = Vec::new();
        while let Some(chunk_result) = receiver.recv().await {
            match chunk_result {
                Ok(chunk) => {
                    chunks.push(chunk);
                    if chunks.last().unwrap().is_complete {
                        break;
                    }
                }
                Err(e) => panic!("Unexpected error: {:?}", e),
            }
        }
        
        assert!(!chunks.is_empty());
        assert!(chunks.last().unwrap().is_complete);
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
        // Should timeout because processing takes 50ms but timeout is 10ms
        assert!(matches!(result, Err(QueueError::Timeout)));
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
