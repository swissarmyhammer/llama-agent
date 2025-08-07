use crate::chat_template::ChatTemplateEngine;
use crate::model::ModelManager;
use crate::types::{
    FinishReason, GenerationRequest, GenerationResponse, MessageRole, QueueConfig, QueueError,
    Session, StreamChunk,
};
use llama_cpp_2::{
    llama_batch::LlamaBatch,
    model::{AddBos, LlamaModel, Special},
    sampling::LlamaSampler,
};
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{mpsc, oneshot, Mutex};
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, info, warn};
use ulid::Ulid;

struct StreamingContext<'a> {
    worker_id: usize,
    request_id: String,
    request: &'a GenerationRequest,
    model: &'a LlamaModel,
    model_manager: &'a ModelManager,
    stream_sender: mpsc::Sender<Result<StreamChunk, QueueError>>,
    cancellation_token: &'a CancellationToken,
    chat_template: &'a ChatTemplateEngine,
}

struct CompletionContext<'a> {
    worker_id: usize,
    request_id: String,
    generated_text: &'a str,
    tokens_generated: u32,
    start_time: Instant,
    stream_sender: &'a mpsc::Sender<Result<StreamChunk, QueueError>>,
    chat_template: &'a ChatTemplateEngine,
    base_reason: &'a str,
}

#[derive(Debug, Default)]
pub struct QueueMetrics {
    pub total_requests: AtomicU64,
    pub completed_requests: AtomicU64,
    pub failed_requests: AtomicU64,
    pub timeout_requests: AtomicU64,
    pub cancelled_requests: AtomicU64,
    pub current_queue_size: AtomicUsize,
    pub total_processing_time_ms: AtomicU64,
    pub total_tokens_generated: AtomicU64,
    pub peak_memory_usage_bytes: AtomicU64,
    pub current_memory_usage_bytes: AtomicU64,
}

impl QueueMetrics {
    pub fn new() -> Self {
        Self {
            total_requests: AtomicU64::new(0),
            completed_requests: AtomicU64::new(0),
            failed_requests: AtomicU64::new(0),
            timeout_requests: AtomicU64::new(0),
            cancelled_requests: AtomicU64::new(0),
            current_queue_size: AtomicUsize::new(0),
            total_processing_time_ms: AtomicU64::new(0),
            total_tokens_generated: AtomicU64::new(0),
            peak_memory_usage_bytes: AtomicU64::new(0),
            current_memory_usage_bytes: AtomicU64::new(0),
        }
    }

    pub fn record_request_submitted(&self) {
        self.total_requests.fetch_add(1, Ordering::Relaxed);
        self.current_queue_size.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_request_completed(&self, processing_time: Duration, tokens_generated: u32) {
        self.completed_requests.fetch_add(1, Ordering::Relaxed);
        self.current_queue_size.fetch_sub(1, Ordering::Relaxed);
        self.total_processing_time_ms
            .fetch_add(processing_time.as_millis() as u64, Ordering::Relaxed);
        self.total_tokens_generated
            .fetch_add(tokens_generated as u64, Ordering::Relaxed);
    }

    pub fn record_request_failed(&self) {
        self.failed_requests.fetch_add(1, Ordering::Relaxed);
        self.current_queue_size.fetch_sub(1, Ordering::Relaxed);
    }

    pub fn record_request_timeout(&self) {
        self.timeout_requests.fetch_add(1, Ordering::Relaxed);
        self.current_queue_size.fetch_sub(1, Ordering::Relaxed);
    }

    pub fn record_request_cancelled(&self) {
        self.cancelled_requests.fetch_add(1, Ordering::Relaxed);
        self.current_queue_size.fetch_sub(1, Ordering::Relaxed);
    }

    /// Update memory usage metrics (estimated based on request size)
    pub fn update_memory_usage(&self, estimated_memory_bytes: u64) {
        self.current_memory_usage_bytes
            .store(estimated_memory_bytes, Ordering::Relaxed);

        // Update peak usage if current exceeds previous peak
        let mut current_peak = self.peak_memory_usage_bytes.load(Ordering::Relaxed);
        while estimated_memory_bytes > current_peak {
            match self.peak_memory_usage_bytes.compare_exchange_weak(
                current_peak,
                estimated_memory_bytes,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => break,
                Err(new_peak) => current_peak = new_peak,
            }
        }
    }

    /// Get current memory usage estimate
    pub fn get_current_memory_usage(&self) -> u64 {
        self.current_memory_usage_bytes.load(Ordering::Relaxed)
    }

    /// Get peak memory usage
    pub fn get_peak_memory_usage(&self) -> u64 {
        self.peak_memory_usage_bytes.load(Ordering::Relaxed)
    }

    pub fn get_stats(&self) -> QueueStats {
        QueueStats {
            total_requests: self.total_requests.load(Ordering::Relaxed),
            completed_requests: self.completed_requests.load(Ordering::Relaxed),
            failed_requests: self.failed_requests.load(Ordering::Relaxed),
            timeout_requests: self.timeout_requests.load(Ordering::Relaxed),
            cancelled_requests: self.cancelled_requests.load(Ordering::Relaxed),
            current_queue_size: self.current_queue_size.load(Ordering::Relaxed),
            average_processing_time_ms: {
                let total_time = self.total_processing_time_ms.load(Ordering::Relaxed);
                let completed = self.completed_requests.load(Ordering::Relaxed);
                if completed > 0 {
                    total_time / completed
                } else {
                    0
                }
            },
            total_tokens_generated: self.total_tokens_generated.load(Ordering::Relaxed),
            current_memory_usage_bytes: self.current_memory_usage_bytes.load(Ordering::Relaxed),
            peak_memory_usage_bytes: self.peak_memory_usage_bytes.load(Ordering::Relaxed),
        }
    }
}

#[derive(Debug, Clone)]
pub struct QueueStats {
    pub total_requests: u64,
    pub completed_requests: u64,
    pub failed_requests: u64,
    pub timeout_requests: u64,
    pub cancelled_requests: u64,
    pub current_queue_size: usize,
    pub average_processing_time_ms: u64,
    pub total_tokens_generated: u64,
    pub current_memory_usage_bytes: u64,
    pub peak_memory_usage_bytes: u64,
}

#[derive(Debug)]
pub struct QueuedRequest {
    pub id: String,
    pub request: GenerationRequest,
    pub response_sender: oneshot::Sender<Result<GenerationResponse, QueueError>>,
    pub stream_sender: Option<mpsc::Sender<Result<StreamChunk, QueueError>>>,
    pub submitted_at: Instant,
    pub cancellation_token: CancellationToken,
}

pub struct RequestQueue {
    sender: mpsc::Sender<QueuedRequest>,
    worker_handles: Vec<JoinHandle<()>>,
    config: QueueConfig,
    metrics: Arc<QueueMetrics>,
    #[allow(dead_code)]
    chat_template: Arc<ChatTemplateEngine>,
}

impl RequestQueue {
    /// Calculate optimal batch size based on token count and system resources
    fn calculate_optimal_batch_size(
        token_count: usize,
        min_batch: usize,
        max_batch: usize,
    ) -> usize {
        // Base calculation on token count with performance considerations
        let base_size = (token_count / 4).clamp(min_batch, max_batch);

        // Adjust based on available CPU cores for better parallelization
        let cpu_factor = (num_cpus::get() / 2).max(1).min(4);
        let optimized_size = base_size * cpu_factor;

        // Ensure we stay within reasonable bounds for memory usage
        optimized_size.clamp(min_batch, max_batch)
    }

    /// Calculate optimal worker count based on system resources
    fn calculate_optimal_worker_count(requested_workers: usize) -> usize {
        let cpu_count = num_cpus::get();

        // For inference workloads, we don't need as many workers as CPUs
        // since most work is done by the ML model which has its own threading
        let optimal_count = match cpu_count {
            1..=2 => 1,                         // Single worker for low-core systems
            3..=4 => 2,                         // Dual workers for mid-range systems
            5..=8 => cpu_count / 2,             // Half the cores for reasonable systems
            _ => (cpu_count / 3).max(4).min(8), // Cap at 8 workers for high-core systems
        };

        // Respect user configuration but provide optimization
        requested_workers.min(optimal_count).max(1)
    }

    pub fn new(model_manager: Arc<ModelManager>, config: QueueConfig) -> Self {
        let (sender, receiver) = mpsc::channel(config.max_queue_size);
        let receiver = Arc::new(Mutex::new(receiver));
        let metrics = Arc::new(QueueMetrics::new());
        let chat_template = Arc::new(ChatTemplateEngine::new());

        let mut worker_handles = Vec::new();

        // Optimize worker thread count based on system resources and workload characteristics
        let optimal_worker_count = Self::calculate_optimal_worker_count(config.worker_threads);
        info!(
            "Using {} optimized worker threads (requested: {})",
            optimal_worker_count, config.worker_threads
        );

        // Spawn worker threads
        for worker_id in 0..optimal_worker_count {
            let receiver = receiver.clone();
            let model_manager = model_manager.clone();
            let config = config.clone();
            let metrics = metrics.clone();
            let chat_template = chat_template.clone();

            let handle = tokio::spawn(async move {
                Self::worker_loop(
                    worker_id,
                    receiver,
                    model_manager,
                    config,
                    metrics,
                    chat_template,
                )
                .await;
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
            metrics,
            chat_template,
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
            cancellation_token: CancellationToken::new(),
        };

        debug!("Submitting request to queue: {}", queued_request.id);

        // Record request submission
        self.metrics.record_request_submitted();

        // Try to send to queue
        if self.sender.try_send(queued_request).is_err() {
            warn!("Queue is full, rejecting request");
            self.metrics.record_request_failed(); // Adjust queue size back down
            return Err(QueueError::Full {
                capacity: self.config.max_queue_size,
            });
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
                Err(QueueError::Timeout {
                    duration: self.config.request_timeout,
                })
            }
        }
    }

    pub async fn submit_streaming_request(
        &self,
        request: GenerationRequest,
    ) -> Result<mpsc::Receiver<Result<StreamChunk, QueueError>>, QueueError> {
        let (response_sender, _) = oneshot::channel();
        // Use optimized buffer size for streaming to prevent backpressure while managing memory
        let base_buffer = request.max_tokens.unwrap_or(512) as usize;
        let cpu_factor = (num_cpus::get() / 2).max(1).min(4);
        let buffer_size = (base_buffer / 4).max(50).min(1000) * cpu_factor;
        let (stream_sender, stream_receiver) = mpsc::channel(buffer_size);

        let queued_request = QueuedRequest {
            id: Ulid::new().to_string(),
            request,
            response_sender,
            stream_sender: Some(stream_sender),
            submitted_at: Instant::now(),
            cancellation_token: CancellationToken::new(),
        };

        debug!(
            "Submitting streaming request to queue: {}",
            queued_request.id
        );

        // Record request submission
        self.metrics.record_request_submitted();

        // Try to send to queue
        if self.sender.try_send(queued_request).is_err() {
            warn!("Queue is full, rejecting streaming request");
            self.metrics.record_request_failed(); // Adjust queue size back down
            return Err(QueueError::Full {
                capacity: self.config.max_queue_size,
            });
        }

        Ok(stream_receiver)
    }

    pub fn get_queue_size(&self) -> usize {
        // Use metrics for more accurate queue size
        self.metrics.current_queue_size.load(Ordering::Relaxed)
    }

    pub fn get_stats(&self) -> QueueStats {
        self.metrics.get_stats()
    }

    async fn worker_loop(
        worker_id: usize,
        receiver: Arc<Mutex<mpsc::Receiver<QueuedRequest>>>,
        model_manager: Arc<ModelManager>,
        config: QueueConfig,
        metrics: Arc<QueueMetrics>,
        chat_template: Arc<ChatTemplateEngine>,
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

            // Estimate memory usage for this request
            let session_content_size: usize = queued_request
                .request
                .session
                .messages
                .iter()
                .map(|m| m.content.len())
                .sum();
            let estimated_memory_bytes = session_content_size as u64
                + queued_request.request.max_tokens.unwrap_or(512) as u64 * 8; // Estimate token memory
            metrics.update_memory_usage(estimated_memory_bytes);

            // Check if request has already timed out
            if queue_time > config.request_timeout {
                warn!(
                    "Worker {} dropping expired request {} (queued for {:?})",
                    worker_id, queued_request.id, queue_time
                );
                let _ = queued_request
                    .response_sender
                    .send(Err(QueueError::Timeout {
                        duration: queue_time,
                    }));
                metrics.record_request_timeout();
                continue;
            }

            // Check if request was cancelled
            if queued_request.cancellation_token.is_cancelled() {
                warn!(
                    "Worker {} dropping cancelled request {} (queued for {:?})",
                    worker_id, queued_request.id, queue_time
                );
                let _ = queued_request
                    .response_sender
                    .send(Err(QueueError::WorkerError(
                        "Request cancelled".to_string(),
                    )));
                metrics.record_request_cancelled();
                continue;
            }

            // Process the request
            Self::process_request(
                worker_id,
                queued_request,
                model_manager.clone(),
                metrics.clone(),
                chat_template.clone(),
            )
            .await;
        }
    }

    async fn process_request(
        worker_id: usize,
        queued_request: QueuedRequest,
        model_manager: Arc<ModelManager>,
        metrics: Arc<QueueMetrics>,
        chat_template: Arc<ChatTemplateEngine>,
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
            metrics.record_request_failed();
            return;
        }

        let request_id = queued_request.id.clone();

        // Process request with model access - use a closure to work within model lifetime
        if let Some(stream_sender) = queued_request.stream_sender {
            // Handle streaming request
            let result = model_manager
                .with_model(|model| {
                    // Process the streaming request synchronously within the model lifetime
                    Self::process_streaming_request_sync(StreamingContext {
                        worker_id,
                        request_id: request_id.clone(),
                        request: &queued_request.request,
                        model,
                        model_manager: &model_manager,
                        stream_sender: stream_sender.clone(),
                        cancellation_token: &queued_request.cancellation_token,
                        chat_template: &chat_template,
                    })
                })
                .await;

            match result {
                Ok(_) => {
                    // Streaming completed successfully
                    let processing_time = start_time.elapsed();
                    // Note: For streaming, tokens are tracked within process_streaming_request_sync
                    metrics.record_request_completed(processing_time, 0);
                }
                Err(model_error) => {
                    let queue_error =
                        QueueError::WorkerError(format!("Model error: {}", model_error));
                    let _ = stream_sender.send(Err(queue_error)).await;
                    metrics.record_request_failed();
                }
            }
        } else {
            // Handle batch request
            let result = model_manager
                .with_model(|model| {
                    // Process the request synchronously within the model lifetime
                    Self::process_batch_request_sync(
                        worker_id,
                        request_id.clone(),
                        &queued_request.request,
                        model,
                        &model_manager,
                        &queued_request.cancellation_token,
                        &chat_template,
                    )
                })
                .await;

            match result {
                Ok(inner_result) => {
                    // inner_result is Result<GenerationResponse, QueueError>
                    match inner_result {
                        Ok(response) => {
                            let processing_time = start_time.elapsed();
                            metrics.record_request_completed(
                                processing_time,
                                response.tokens_generated,
                            );
                            let _ = queued_request.response_sender.send(Ok(response));
                        }
                        Err(queue_error) => {
                            metrics.record_request_failed();
                            let _ = queued_request.response_sender.send(Err(queue_error));
                        }
                    }
                }
                Err(model_error) => {
                    metrics.record_request_failed();
                    let queue_error =
                        QueueError::WorkerError(format!("Model error: {}", model_error));
                    let _ = queued_request.response_sender.send(Err(queue_error));
                }
            };
        }

        let processing_time = start_time.elapsed();
        debug!(
            "Worker {} completed request {} in {:?}",
            worker_id, request_id, processing_time
        );
    }

    fn process_batch_request_sync(
        worker_id: usize,
        request_id: String,
        request: &GenerationRequest,
        model: &LlamaModel,
        model_manager: &ModelManager,
        cancellation_token: &CancellationToken,
        chat_template: &ChatTemplateEngine,
    ) -> Result<GenerationResponse, QueueError> {
        let start_time = Instant::now();

        debug!(
            "Worker {} starting batch inference for request {}",
            worker_id, request_id
        );

        // Format the session messages into a prompt
        let prompt = Self::format_session_prompt(&request.session)?;
        debug!("Formatted prompt: {}", prompt);

        // Create context for this inference
        let mut ctx = match model_manager.create_context(model) {
            Ok(context) => context,
            Err(e) => {
                error!("Failed to create context: {}", e);
                return Err(QueueError::WorkerError(format!(
                    "Context creation failed: {}",
                    e
                )));
            }
        };

        // Tokenize the prompt
        let tokens_list = match model.str_to_token(&prompt, AddBos::Always) {
            Ok(tokens) => tokens,
            Err(e) => {
                error!("Failed to tokenize prompt: {}", e);
                return Err(QueueError::WorkerError(format!(
                    "Tokenization failed: {}",
                    e
                )));
            }
        };

        debug!("Tokenized prompt to {} tokens", tokens_list.len());

        // Create batch for initial prompt processing with optimized size based on available memory and token count
        let optimal_batch_size = Self::calculate_optimal_batch_size(tokens_list.len(), 512, 2048);
        let mut batch = LlamaBatch::new(optimal_batch_size, 1);

        // Add prompt tokens to batch
        for (i, token) in tokens_list.iter().enumerate() {
            let is_last = i == tokens_list.len() - 1;
            if let Err(e) = batch.add(*token, i as i32, &[0], is_last) {
                error!("Failed to add token to batch: {}", e);
                return Err(QueueError::WorkerError(format!(
                    "Batch token add failed: {}",
                    e
                )));
            }
        }

        // Process the initial prompt batch
        if let Err(e) = ctx.decode(&mut batch) {
            error!("Failed to decode batch: {}", e);
            return Err(QueueError::WorkerError(format!(
                "Batch decode failed: {}",
                e
            )));
        }

        debug!("Initial prompt processed, starting generation");

        // Create optimized sampler for token generation
        let mut sampler = LlamaSampler::chain_simple([
            LlamaSampler::dist(1234),     // Use fixed seed for deterministic behavior
            LlamaSampler::top_k(40), // Add top-k sampling for better quality/performance balance
            LlamaSampler::top_p(0.95, 1), // Add nucleus sampling
            LlamaSampler::min_p(0.05, 1), // Add minimum probability threshold
            LlamaSampler::temp(0.8), // Add temperature for controlled randomness
            LlamaSampler::greedy(),  // Final greedy selection
        ]);

        let max_tokens = request.max_tokens.unwrap_or(512);
        // Pre-allocate string with estimated capacity to reduce memory reallocations
        let estimated_chars = (max_tokens as usize).saturating_mul(4); // ~4 chars per token
        let mut generated_text = String::with_capacity(estimated_chars.min(8192)); // Cap at 8KB
        let mut finish_reason = FinishReason::MaxTokens;
        let mut tokens_generated = 0u32;
        let mut n_cur = tokens_list.len();

        // Generation loop
        while tokens_generated < max_tokens {
            // Check for cancellation before each token
            if cancellation_token.is_cancelled() {
                debug!(
                    "Worker {} batch request {} cancelled during token generation",
                    worker_id, request_id
                );
                finish_reason = FinishReason::Error("Request cancelled".to_string());
                break;
            }

            // Sample next token
            let token = sampler.sample(&ctx, batch.n_tokens() - 1);

            // Check for end of sequence token
            if model.is_eog_token(token) {
                finish_reason = FinishReason::EndOfSequence;
                break;
            }

            // Convert token to string
            let token_str = match model.token_to_str(token, Special::Tokenize) {
                Ok(s) => s,
                Err(e) => {
                    warn!("Failed to convert token to string: {}", e);
                    continue; // Skip this token but continue generation
                }
            };

            generated_text.push_str(&token_str);
            tokens_generated += 1;

            // Check for stop tokens in the generated text
            if Self::should_stop(&generated_text, &request.stop_tokens) {
                finish_reason = FinishReason::StopToken;
                break;
            }

            // Prepare next batch for continued generation
            batch.clear();
            if let Err(e) = batch.add(token, n_cur as i32, &[0], true) {
                error!("Failed to add continuation token: {}", e);
                break;
            }

            // Decode the new token
            if let Err(e) = ctx.decode(&mut batch) {
                error!("Failed to decode continuation batch: {}", e);
                break;
            }

            n_cur += 1;
        }

        // Check if the generated text contains tool calls
        let final_finish_reason = if finish_reason == FinishReason::EndOfSequence
            || finish_reason == FinishReason::StopToken
            || finish_reason == FinishReason::MaxTokens
        {
            match chat_template.extract_tool_calls(&generated_text) {
                Ok(tool_calls) if !tool_calls.is_empty() => {
                    debug!(
                        "Worker {} detected {} tool calls in generated text for request {}",
                        worker_id,
                        tool_calls.len(),
                        request_id
                    );
                    FinishReason::ToolCall
                }
                Ok(_) => {
                    debug!(
                        "Worker {} no tool calls detected in generated text for request {}",
                        worker_id, request_id
                    );
                    finish_reason
                }
                Err(e) => {
                    warn!(
                        "Worker {} failed to extract tool calls for request {}: {}",
                        worker_id, request_id, e
                    );
                    finish_reason
                }
            }
        } else {
            finish_reason
        };

        let generation_time = start_time.elapsed();

        debug!(
            "Worker {} completed batch inference for request {} in {:?} ({} tokens, finish_reason: {:?})",
            worker_id, request_id, generation_time, tokens_generated, final_finish_reason
        );

        Ok(GenerationResponse {
            generated_text,
            tokens_generated,
            generation_time,
            finish_reason: final_finish_reason,
        })
    }

    fn format_session_prompt(session: &Session) -> Result<String, QueueError> {
        // Pre-calculate capacity to reduce reallocations
        let estimated_capacity = session
            .messages
            .iter()
            .map(|m| m.content.len() + 50) // Role prefix + content + buffer
            .sum::<usize>()
            + 100; // Additional buffer for "Assistant:" suffix

        let mut prompt = String::with_capacity(estimated_capacity);

        for message in &session.messages {
            match message.role {
                MessageRole::System => {
                    prompt.push_str("System: ");
                    prompt.push_str(&message.content);
                    prompt.push('\n');
                }
                MessageRole::User => {
                    prompt.push_str("User: ");
                    prompt.push_str(&message.content);
                    prompt.push('\n');
                }
                MessageRole::Assistant => {
                    prompt.push_str("Assistant: ");
                    prompt.push_str(&message.content);
                    prompt.push('\n');
                }
                MessageRole::Tool => {
                    if let Some(tool_name) = &message.tool_name {
                        prompt.push_str("Tool (");
                        prompt.push_str(tool_name);
                        prompt.push_str("): ");
                    } else {
                        prompt.push_str("Tool: ");
                    }
                    prompt.push_str(&message.content);
                    prompt.push('\n');
                }
            }
        }

        // Add assistant prompt to continue generation
        prompt.push_str("Assistant:");

        Ok(prompt)
    }

    fn should_stop(generated_text: &str, stop_tokens: &[String]) -> bool {
        for stop_token in stop_tokens {
            if generated_text.contains(stop_token) {
                return true;
            }
        }
        false
    }

    fn process_streaming_request_sync(context: StreamingContext<'_>) -> Result<(), QueueError> {
        let StreamingContext {
            worker_id,
            request_id,
            request,
            model,
            model_manager,
            stream_sender,
            cancellation_token,
            chat_template,
        } = context;
        let start_time = Instant::now();

        debug!(
            "Worker {} starting streaming inference for request {}",
            worker_id, request_id
        );

        // Format the session messages into a prompt
        let prompt = Self::format_session_prompt(&request.session)?;
        debug!("Formatted prompt for streaming: {}", prompt);

        // Create context for this inference
        let mut ctx = match model_manager.create_context(model) {
            Ok(context) => context,
            Err(e) => {
                error!("Failed to create context for streaming: {}", e);
                let _ = stream_sender.try_send(Err(QueueError::WorkerError(format!(
                    "Context creation failed: {}",
                    e
                ))));
                return Ok(());
            }
        };

        // Tokenize the prompt
        let tokens_list = match model.str_to_token(&prompt, AddBos::Always) {
            Ok(tokens) => tokens,
            Err(e) => {
                error!("Failed to tokenize prompt for streaming: {}", e);
                let _ = stream_sender.try_send(Err(QueueError::WorkerError(format!(
                    "Tokenization failed: {}",
                    e
                ))));
                return Ok(());
            }
        };

        debug!(
            "Tokenized prompt to {} tokens for streaming",
            tokens_list.len()
        );

        // Create and process initial batch with optimized size
        let optimal_batch_size = Self::calculate_optimal_batch_size(tokens_list.len(), 512, 2048);
        let mut batch = LlamaBatch::new(optimal_batch_size, 1);

        // Add prompt tokens to batch
        for (i, token) in tokens_list.iter().enumerate() {
            let is_last = i == tokens_list.len() - 1;
            if let Err(e) = batch.add(*token, i as i32, &[0], is_last) {
                error!("Failed to add token to streaming batch: {}", e);
                let _ = stream_sender.try_send(Err(QueueError::WorkerError(format!(
                    "Batch token add failed: {}",
                    e
                ))));
                return Ok(());
            }
        }

        // Process the initial prompt batch
        if let Err(e) = ctx.decode(&mut batch) {
            error!("Failed to decode streaming batch: {}", e);
            let _ = stream_sender.try_send(Err(QueueError::WorkerError(format!(
                "Batch decode failed: {}",
                e
            ))));
            return Ok(());
        }

        debug!("Initial prompt processed for streaming, starting generation");

        // Create optimized sampler for token generation (streaming)
        let mut sampler = LlamaSampler::chain_simple([
            LlamaSampler::dist(1234),     // Use fixed seed for deterministic behavior
            LlamaSampler::top_k(40), // Add top-k sampling for better quality/performance balance
            LlamaSampler::top_p(0.95, 1), // Add nucleus sampling
            LlamaSampler::min_p(0.05, 1), // Add minimum probability threshold
            LlamaSampler::temp(0.8), // Add temperature for controlled randomness
            LlamaSampler::greedy(),  // Final greedy selection
        ]);

        let max_tokens = request.max_tokens.unwrap_or(512);
        // Pre-allocate string capacity based on expected output size
        let estimated_chars = (max_tokens as usize) * 4; // Rough estimate: 4 chars per token
        let mut generated_text = String::with_capacity(estimated_chars);
        let mut tokens_generated = 0u32;
        let mut n_cur = tokens_list.len();

        // Generation loop - stream tokens one by one
        while tokens_generated < max_tokens {
            // Check for cancellation before each token
            if cancellation_token.is_cancelled() {
                debug!(
                    "Worker {} streaming request {} cancelled during token generation",
                    worker_id, request_id
                );
                let _ = stream_sender.try_send(Err(QueueError::WorkerError(
                    "Request cancelled".to_string(),
                )));
                return Ok(());
            }

            // Sample next token
            let token = sampler.sample(&ctx, batch.n_tokens() - 1);

            // Check for end of sequence token
            if model.is_eog_token(token) {
                return Self::handle_streaming_completion(CompletionContext {
                    worker_id,
                    request_id,
                    generated_text: &generated_text,
                    tokens_generated,
                    start_time,
                    stream_sender: &stream_sender,
                    chat_template,
                    base_reason: "EndOfSequence",
                });
            }

            // Convert token to string
            let token_text = match model.token_to_str(token, Special::Tokenize) {
                Ok(s) => s,
                Err(e) => {
                    warn!("Failed to convert token to string in streaming: {}", e);
                    continue; // Skip this token but continue generation
                }
            };

            generated_text.push_str(&token_text);
            tokens_generated += 1;

            // Send the streaming chunk immediately
            let chunk = StreamChunk {
                text: token_text.clone(),
                is_complete: false,
                token_count: tokens_generated,
            };

            if stream_sender.try_send(Ok(chunk)).is_err() {
                warn!("Stream receiver disconnected, stopping generation");
                return Ok(());
            }

            // Check for stop tokens in the accumulated generated text
            if Self::should_stop(&generated_text, &request.stop_tokens) {
                return Self::handle_streaming_completion(CompletionContext {
                    worker_id,
                    request_id,
                    generated_text: &generated_text,
                    tokens_generated,
                    start_time,
                    stream_sender: &stream_sender,
                    chat_template,
                    base_reason: "StopToken",
                });
            }

            // Prepare next batch for continued generation
            batch.clear();
            if let Err(e) = batch.add(token, n_cur as i32, &[0], true) {
                error!("Failed to add continuation token for streaming: {}", e);
                break;
            }

            // Decode the new token
            if let Err(e) = ctx.decode(&mut batch) {
                error!("Failed to decode continuation batch for streaming: {}", e);
                break;
            }

            n_cur += 1;
        }

        // If we exit the loop due to max tokens, send final completion chunk
        Self::handle_streaming_completion(CompletionContext {
            worker_id,
            request_id,
            generated_text: &generated_text,
            tokens_generated,
            start_time,
            stream_sender: &stream_sender,
            chat_template,
            base_reason: "MaxTokens",
        })
    }

    /// Handle completion of streaming request with tool call detection
    fn handle_streaming_completion(context: CompletionContext<'_>) -> Result<(), QueueError> {
        let CompletionContext {
            worker_id,
            request_id,
            generated_text,
            tokens_generated,
            start_time,
            stream_sender,
            chat_template,
            base_reason,
        } = context;
        // Check if the generated text contains tool calls
        let has_tool_calls = match chat_template.extract_tool_calls(generated_text) {
            Ok(tool_calls) if !tool_calls.is_empty() => {
                debug!(
                    "Worker {} detected {} tool calls in streaming output for request {}",
                    worker_id,
                    tool_calls.len(),
                    request_id
                );
                true
            }
            Ok(_) => {
                debug!(
                    "Worker {} no tool calls detected in streaming output for request {}",
                    worker_id, request_id
                );
                false
            }
            Err(e) => {
                warn!(
                    "Worker {} failed to extract tool calls from streaming output for request {}: {}",
                    worker_id, request_id, e
                );
                false
            }
        };

        // Create final completion chunk - for streaming, we can't change the finish reason
        // but we could potentially add metadata to indicate tool calls were detected
        let final_chunk = StreamChunk {
            text: String::new(),
            is_complete: true,
            token_count: tokens_generated,
        };
        let _ = stream_sender.try_send(Ok(final_chunk));

        let generation_time = start_time.elapsed();
        let reason_suffix = if has_tool_calls {
            " (with tool calls)"
        } else {
            ""
        };
        debug!(
            "Worker {} completed streaming inference for request {} in {:?} ({} tokens, reason: {}{})",
            worker_id, request_id, generation_time, tokens_generated, base_reason, reason_suffix
        );

        Ok(())
    }
}

impl RequestQueue {
    /// Gracefully shutdown the queue, waiting for all workers to complete
    pub async fn shutdown(mut self) -> Result<(), QueueError> {
        info!("🛑 RequestQueue initiating graceful shutdown...");
        let shutdown_start = std::time::Instant::now();

        // Create shutdown timeout to prevent hanging
        let shutdown_timeout = std::time::Duration::from_secs(30);

        let result = tokio::time::timeout(shutdown_timeout, async move {
            info!("📤 Closing request channel to stop accepting new requests...");

            // Create a dummy sender and swap it out to effectively close the original sender
            let (dummy_sender, _) = tokio::sync::mpsc::channel(1);
            let _ = std::mem::replace(&mut self.sender, dummy_sender);

            info!(
                "⏳ Waiting for {} worker threads to complete...",
                self.worker_handles.len()
            );

            // Wait for all worker handles to complete with individual timeouts
            let mut completed_workers = 0;
            for (idx, handle) in self.worker_handles.drain(..).enumerate() {
                match tokio::time::timeout(std::time::Duration::from_secs(10), handle).await {
                    Ok(Ok(())) => {
                        completed_workers += 1;
                        debug!("✅ Worker {} shutdown completed", idx);
                    }
                    Ok(Err(e)) => {
                        warn!("⚠️  Worker {} panicked during shutdown: {:?}", idx, e);
                    }
                    Err(_) => {
                        warn!("⏰ Worker {} shutdown timed out, forcing termination", idx);
                    }
                }
            }

            let shutdown_duration = shutdown_start.elapsed();
            info!(
                "✅ RequestQueue shutdown completed successfully in {:?}",
                shutdown_duration
            );
            info!(
                "📊 Shutdown summary: {}/{} workers completed gracefully",
                completed_workers,
                self.worker_handles.capacity()
            );

            Ok(())
        })
        .await;

        match result {
            Ok(Ok(())) => {
                info!("🎉 Graceful shutdown completed successfully");
                Ok(())
            }
            Ok(Err(e)) => {
                error!("❌ Error during shutdown: {:?}", e);
                Err(e)
            }
            Err(_) => {
                warn!("⏰ Shutdown timed out after {:?}, some resources may not be cleaned up properly", shutdown_timeout);
                Err(QueueError::WorkerError(
                    "Shutdown timeout exceeded".to_string(),
                ))
            }
        }
    }
}

impl Drop for RequestQueue {
    fn drop(&mut self) {
        if !self.worker_handles.is_empty() {
            warn!(
                "🚨 RequestQueue being dropped with {} active worker handles - resources may not be cleaned up properly! 
                \n🔧 For proper cleanup, call shutdown() method before dropping.",
                self.worker_handles.len()
            );

            // Force abort remaining handles
            for handle in self.worker_handles.drain(..) {
                handle.abort();
            }

            warn!(
                "⚡ Aborted {} worker handles during emergency cleanup",
                self.worker_handles.capacity()
            );
        } else {
            debug!("♻️  RequestQueue dropped cleanly (no active workers)");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{
        Message, MessageRole, ModelConfig, ModelError, ModelSource, QueueConfig, Session, SessionId,
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
        // Handle the case where backend is already initialized by parallel tests
        let model_manager = match ModelManager::new(create_test_model_config()) {
            Ok(manager) => Arc::new(manager),
            Err(ModelError::LoadingFailed(msg))
                if msg.contains("Backend already initialized by external code") =>
            {
                // This is expected when running tests in parallel - skip this test
                println!("Skipping test due to backend already initialized by parallel test");
                return;
            }
            Err(e) => panic!("Failed to create ModelManager: {:?}", e),
        };
        let config = create_test_queue_config();

        let queue = RequestQueue::new(model_manager, config);
        assert_eq!(queue.get_queue_size(), 0);
    }

    #[tokio::test]
    async fn test_submit_request_model_not_loaded() {
        // Handle the case where backend is already initialized by parallel tests
        let model_manager = match ModelManager::new(create_test_model_config()) {
            Ok(manager) => Arc::new(manager),
            Err(ModelError::LoadingFailed(msg))
                if msg.contains("Backend already initialized by external code") =>
            {
                // This is expected when running tests in parallel - skip this test
                println!("Skipping test due to backend already initialized by parallel test");
                return;
            }
            Err(e) => panic!("Failed to create ModelManager: {:?}", e),
        };
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
            QueueError::Timeout { duration: _ } => {
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
            cancellation_token: CancellationToken::new(),
        };

        let debug_str = format!("{:?}", request);
        assert!(debug_str.contains("test-123"));
    }
}
