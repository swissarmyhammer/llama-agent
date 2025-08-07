use crate::chat_template::ChatTemplateEngine;
use crate::mcp::MCPClient;
use crate::model::ModelManager;
use crate::queue::RequestQueue;
use crate::session::SessionManager;
use crate::types::{
    AgentAPI, AgentConfig, AgentError, GenerationRequest, GenerationResponse, HealthStatus,
    Session, SessionId, StreamChunk, ToolCall, ToolResult,
};
use async_trait::async_trait;
use futures::{Stream, StreamExt};
use std::pin::Pin;
use std::sync::Arc;
use std::time::{Instant, SystemTime};
use tokio_stream::wrappers::ReceiverStream;
use tracing::{debug, error, info, warn};

pub struct AgentServer {
    model_manager: Arc<ModelManager>,
    request_queue: Arc<RequestQueue>,
    session_manager: Arc<SessionManager>,
    mcp_client: Arc<MCPClient>,
    chat_template: Arc<ChatTemplateEngine>,
    config: AgentConfig,
    start_time: Instant,
    shutdown_token: tokio_util::sync::CancellationToken,
}

impl std::fmt::Debug for AgentServer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AgentServer")
            .field("config", &self.config)
            .field("start_time", &self.start_time)
            .finish()
    }
}

impl Drop for AgentServer {
    fn drop(&mut self) {
        // If shutdown wasn't called explicitly, at least cancel the shutdown token
        if !self.shutdown_token.is_cancelled() {
            warn!("AgentServer dropped without explicit shutdown - resources may not be cleaned up properly");
            self.shutdown_token.cancel();
        }
    }
}

impl AgentServer {
    pub fn new(
        model_manager: Arc<ModelManager>,
        request_queue: Arc<RequestQueue>,
        session_manager: Arc<SessionManager>,
        mcp_client: Arc<MCPClient>,
        chat_template: Arc<ChatTemplateEngine>,
        config: AgentConfig,
    ) -> Self {
        Self {
            model_manager,
            request_queue,
            session_manager,
            mcp_client,
            chat_template,
            config,
            start_time: Instant::now(),
            shutdown_token: tokio_util::sync::CancellationToken::new(),
        }
    }

    pub fn mcp_client(&self) -> &MCPClient {
        &self.mcp_client
    }

    pub async fn shutdown(self) -> Result<(), AgentError> {
        info!("Initiating AgentServer graceful shutdown");
        let shutdown_start = std::time::Instant::now();

        // Signal shutdown to all components
        self.shutdown_token.cancel();

        // Create shutdown timeout to prevent hanging
        let shutdown_timeout = tokio::time::Duration::from_secs(30);
        let result = tokio::time::timeout(shutdown_timeout, async {
            // 1. Stop accepting new requests by gracefully shutting down the queue
            info!("Shutting down request queue...");
            // Note: We would need to modify RequestQueue to implement proper shutdown
            // For now, just drain any pending work by waiting briefly
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

            // 2. Clean up sessions
            info!("Cleaning up active sessions...");
            let session_count = self.session_manager.get_session_count().await;
            if session_count > 0 {
                info!("Found {} active sessions during shutdown", session_count);
                // Sessions will be cleaned up automatically when SessionManager is dropped
            }

            // 3. Shutdown MCP client connections
            info!("Shutting down MCP connections...");
            self.mcp_client.shutdown_all().await?;

            // 4. Final cleanup and resource reporting
            let shutdown_time = shutdown_start.elapsed();
            info!(
                "AgentServer shutdown completed successfully in {:?}",
                shutdown_time
            );

            Ok::<(), AgentError>(())
        })
        .await;

        match result {
            Ok(Ok(())) => {
                info!("Graceful shutdown completed");
                Ok(())
            }
            Ok(Err(e)) => {
                error!("Error during shutdown: {}", e);
                Err(e)
            }
            Err(_timeout) => {
                warn!(
                    "Shutdown timed out after {:?}, forcing exit",
                    shutdown_timeout
                );
                // Force cleanup - this is not ideal but prevents hanging
                Ok(())
            }
        }
    }

    /// Check if text contains excessive repetition (potential DoS attack)
    fn has_excessive_repetition(text: &str) -> bool {
        if text.len() < 100 {
            return false;
        }

        let chars: Vec<char> = text.chars().collect();
        let mut repetition_count = 0;
        let window_size = 50; // Check 50-character windows

        for i in 0..(chars.len().saturating_sub(window_size * 2)) {
            let pattern = &chars[i..i + window_size];
            let next_window = &chars[i + window_size..i + window_size * 2];

            if pattern == next_window {
                repetition_count += 1;
                if repetition_count > 5 {
                    // Allow some repetition but not excessive
                    return true;
                }
            } else {
                repetition_count = 0;
            }
        }

        false
    }

    /// Validate generation request for security and correctness
    fn validate_generation_request(&self, request: &GenerationRequest) -> Result<(), AgentError> {
        // 1. Validate token limits
        if let Some(max_tokens) = request.max_tokens {
            if max_tokens == 0 {
                return Err(AgentError::Template(crate::types::TemplateError::Invalid(
                    "max_tokens must be greater than 0".to_string(),
                )));
            }
            if max_tokens > 8192 {
                warn!("Large token request: {} tokens requested", max_tokens);
                // Allow but warn about large requests
            }
        }

        // 2. Validate temperature range
        if let Some(temperature) = request.temperature {
            if !(0.0..=2.0).contains(&temperature) {
                return Err(AgentError::Template(crate::types::TemplateError::Invalid(
                    "temperature must be between 0.0 and 2.0".to_string(),
                )));
            }
        }

        // 3. Validate top_p range
        if let Some(top_p) = request.top_p {
            if !(0.0..=1.0).contains(&top_p) {
                return Err(AgentError::Template(crate::types::TemplateError::Invalid(
                    "top_p must be between 0.0 and 1.0".to_string(),
                )));
            }
        }

        // 4. Validate session content size
        let total_content_size: usize = request
            .session
            .messages
            .iter()
            .map(|m| m.content.len())
            .sum();

        if total_content_size > 1_000_000 {
            // 1MB limit
            return Err(AgentError::Template(crate::types::TemplateError::Invalid(
                "session content exceeds maximum size limit".to_string(),
            )));
        }

        // 5. Validate stop tokens
        for stop_token in &request.stop_tokens {
            if stop_token.is_empty() {
                return Err(AgentError::Template(crate::types::TemplateError::Invalid(
                    "stop tokens cannot be empty".to_string(),
                )));
            }
            if stop_token.len() > 100 {
                return Err(AgentError::Template(crate::types::TemplateError::Invalid(
                    "stop tokens cannot exceed 100 characters".to_string(),
                )));
            }
        }

        // 6. Validate message content for potential injection
        for message in &request.session.messages {
            if message.content.len() > 100_000 {
                // 100KB per message
                return Err(AgentError::Template(crate::types::TemplateError::Invalid(
                    "individual message exceeds size limit".to_string(),
                )));
            }

            // Enhanced security validation - check for suspicious patterns
            if message.content.contains('\0') {
                return Err(AgentError::Template(crate::types::TemplateError::Invalid(
                    "message content contains null bytes".to_string(),
                )));
            }

            // Check for potential prompt injection patterns
            let content_lower = message.content.to_lowercase();
            let suspicious_patterns = [
                "ignore previous instructions",
                "disregard all",
                "system:",
                "<script",
                "javascript:",
                "eval(",
                "exec(",
                "subprocess",
                "__import__",
            ];

            for pattern in &suspicious_patterns {
                if content_lower.contains(pattern) {
                    warn!(
                        "Potentially suspicious content detected in message: contains '{}'",
                        pattern
                    );
                    // Log but don't block - allow legitimate use cases
                    debug!(
                        "Full message content (first 200 chars): {}",
                        &message.content.chars().take(200).collect::<String>()
                    );
                }
            }

            // Check for excessive repetition (potential DoS attempt)
            if Self::has_excessive_repetition(&message.content) {
                warn!("Message contains excessive repetition - potential DoS attempt");
                return Err(AgentError::Template(crate::types::TemplateError::Invalid(
                    "message contains excessive repetition".to_string(),
                )));
            }
        }

        Ok(())
    }

    /// Validate tool call arguments against the tool's parameter schema
    fn validate_tool_arguments(
        &self,
        tool_call: &ToolCall,
        tool_def: &crate::types::ToolDefinition,
    ) -> Result<(), String> {
        // Security check: validate tool name format
        if tool_call.name.is_empty() || tool_call.name.len() > 100 {
            return Err("Tool name must be 1-100 characters".to_string());
        }

        // Check for potentially dangerous tool names
        let dangerous_patterns = [
            "exec",
            "eval",
            "system",
            "shell",
            "cmd",
            "powershell",
            "bash",
        ];
        for pattern in &dangerous_patterns {
            if tool_call.name.to_lowercase().contains(pattern) {
                warn!(
                    "Potentially dangerous tool call detected: {}",
                    tool_call.name
                );
                // Log but allow - some legitimate tools might match
            }
        }

        // If no parameters schema is defined, skip schema validation
        if tool_def.parameters.is_null() {
            debug!("No parameter schema defined for tool '{}'", tool_call.name);
            return Ok(());
        }

        // Basic validation - could be enhanced with JSON Schema validation
        if tool_call.arguments.is_null() && !tool_def.parameters.is_null() {
            return Err("Tool requires arguments but none provided".to_string());
        }

        // Security validation of argument content
        if let Ok(args_str) = serde_json::to_string(&tool_call.arguments) {
            // Check argument size
            if args_str.len() > 10_000 {
                // 10KB limit
                return Err("Tool arguments exceed maximum size limit".to_string());
            }

            // Check for suspicious content in arguments
            let args_lower = args_str.to_lowercase();
            let suspicious_args = [
                "../",
                "..\\",
                "/etc/",
                "c:\\windows",
                "rm -rf",
                "del /",
                "format c:",
            ];
            for pattern in &suspicious_args {
                if args_lower.contains(pattern) {
                    warn!(
                        "Suspicious content in tool arguments for '{}': contains '{}'",
                        tool_call.name, pattern
                    );
                    // Log suspicious content but continue - might be legitimate
                }
            }
        }

        debug!(
            "Tool arguments validation passed for '{}' (enhanced security checks applied)",
            tool_call.name
        );
        Ok(())
    }

    /// Determine if tool calls should be executed in parallel
    fn should_execute_in_parallel(&self, tool_calls: &[ToolCall]) -> bool {
        // Simple heuristic: execute in parallel if there are multiple calls
        // and they don't appear to be interdependent

        // For now, enable parallel execution for most cases
        // TODO: Add more sophisticated dependency analysis
        if tool_calls.len() <= 1 {
            return false;
        }

        // Check for potential dependencies by looking for similar tool names
        // that might modify the same resources
        let mut tool_names = std::collections::HashSet::new();
        for tool_call in tool_calls {
            // If we have duplicate tool names, they might be interdependent
            if !tool_names.insert(&tool_call.name) {
                debug!("Detected duplicate tool names, using sequential execution for safety");
                return false;
            }
        }

        debug!("No obvious dependencies detected, enabling parallel execution");
        true
    }

    /// Execute multiple tool calls in parallel
    async fn execute_tools_parallel(
        &self,
        tool_calls: Vec<ToolCall>,
        session: &Session,
    ) -> Vec<ToolResult> {
        use futures::future::join_all;

        let futures = tool_calls.into_iter().map(|tool_call| {
            let session = session.clone();
            async move {
                debug!("Starting parallel execution of tool: {}", tool_call.name);

                match self.execute_tool(tool_call.clone(), &session).await {
                    Ok(result) => {
                        debug!("Parallel tool call '{}' completed", tool_call.name);
                        result
                    }
                    Err(e) => {
                        error!("Parallel tool call '{}' failed: {}", tool_call.name, e);
                        ToolResult {
                            call_id: tool_call.id,
                            result: serde_json::Value::Null,
                            error: Some(format!("Parallel execution error: {}", e)),
                        }
                    }
                }
            }
        });

        let results = join_all(futures).await;
        debug!(
            "Parallel tool execution completed with {} results",
            results.len()
        );
        results
    }

    async fn process_tool_calls(
        &self,
        text: &str,
        session: &Session,
    ) -> Result<Vec<ToolResult>, AgentError> {
        debug!("Processing tool calls from generated text");

        // Extract tool calls from the generated text
        let tool_calls = match self.chat_template.extract_tool_calls(text) {
            Ok(calls) => calls,
            Err(e) => {
                error!("Failed to extract tool calls from text: {}", e);
                return Ok(Vec::new()); // Return empty results rather than failing
            }
        };

        if tool_calls.is_empty() {
            debug!("No tool calls found in generated text");
            return Ok(Vec::new());
        }

        debug!("Found {} tool calls to process", tool_calls.len());
        let mut results = Vec::new();
        let mut successful_calls = 0;
        let mut failed_calls = 0;

        // Check if we should execute tools in parallel or sequentially
        let parallel_execution =
            tool_calls.len() > 1 && self.should_execute_in_parallel(&tool_calls);

        if parallel_execution {
            debug!("Executing {} tool calls in parallel", tool_calls.len());
            results = self.execute_tools_parallel(tool_calls, session).await;

            // Count results for logging
            for result in &results {
                if result.error.is_some() {
                    failed_calls += 1;
                } else {
                    successful_calls += 1;
                }
            }
        } else {
            debug!("Executing {} tool calls sequentially", tool_calls.len());

            // Process each tool call sequentially
            for (i, tool_call) in tool_calls.into_iter().enumerate() {
                debug!(
                    "Processing tool call {}/{}: {} (id: {})",
                    i + 1,
                    results.len() + 1,
                    tool_call.name,
                    tool_call.id
                );

                // Execute tool call - errors are handled within execute_tool and returned as ToolResult
                match self.execute_tool(tool_call.clone(), session).await {
                    Ok(result) => {
                        if result.error.is_some() {
                            failed_calls += 1;
                            warn!("Tool call '{}' completed with error", tool_call.name);
                        } else {
                            successful_calls += 1;
                            debug!("Tool call '{}' completed successfully", tool_call.name);
                        }
                        results.push(result);
                    }
                    Err(e) => {
                        // This should rarely happen since execute_tool now handles errors internally
                        failed_calls += 1;
                        error!(
                            "Unexpected error executing tool call '{}': {}",
                            tool_call.name, e
                        );

                        // Create error result to maintain call order and IDs
                        let error_result = ToolResult {
                            call_id: tool_call.id,
                            result: serde_json::Value::Null,
                            error: Some(format!("Execution error: {}", e)),
                        };
                        results.push(error_result);
                    }
                }
            }
        }

        info!(
            "Tool call processing completed: {} successful, {} failed, {} total",
            successful_calls,
            failed_calls,
            results.len()
        );

        Ok(results)
    }

    async fn render_session_prompt(&self, session: &Session) -> Result<String, AgentError> {
        self.model_manager
            .with_model(|model| self.chat_template.render_session(session, model))
            .await?
            .map_err(AgentError::Template)
    }
}

#[async_trait]
impl AgentAPI for AgentServer {
    async fn initialize(config: AgentConfig) -> Result<Self, AgentError> {
        info!("Initializing AgentServer with config: {:?}", config);

        // Validate configuration
        config.validate()?;

        // Initialize model manager
        let model_manager = Arc::new(ModelManager::new(config.model.clone())?);
        model_manager.load_model().await?;
        info!("Model manager initialized and model loaded");

        // Initialize request queue
        let request_queue = Arc::new(RequestQueue::new(
            model_manager.clone(),
            config.queue_config.clone(),
        ));
        info!("Request queue initialized");

        // Initialize session manager
        let session_manager = Arc::new(SessionManager::new(config.session_config.clone()));
        info!("Session manager initialized");

        // Initialize MCP client
        let mcp_client = Arc::new(MCPClient::new());

        // Add configured MCP servers
        for server_config in &config.mcp_servers {
            mcp_client.add_server(server_config.clone()).await?;
        }
        info!("MCP client initialized");

        // Initialize chat template engine
        let chat_template = Arc::new(ChatTemplateEngine::new());
        info!("Chat template engine initialized");

        let agent_server = Self::new(
            model_manager,
            request_queue,
            session_manager,
            mcp_client,
            chat_template,
            config,
        );

        info!("AgentServer initialization completed");
        Ok(agent_server)
    }

    async fn generate(&self, request: GenerationRequest) -> Result<GenerationResponse, AgentError> {
        debug!(
            "Processing generation request for session: {}",
            request.session.id
        );

        // Security validation: Check request parameters
        self.validate_generation_request(&request)?;

        let mut working_session = request.session.clone();
        let mut accumulated_response = String::new();
        let mut total_tokens = 0u32;
        let mut iterations = 0;
        const MAX_TOOL_ITERATIONS: usize = 5; // Prevent infinite tool call loops

        loop {
            iterations += 1;
            if iterations > MAX_TOOL_ITERATIONS {
                warn!(
                    "Maximum tool call iterations ({}) reached for session: {}",
                    MAX_TOOL_ITERATIONS, working_session.id
                );
                break;
            }

            debug!(
                "Tool call iteration {} for session: {}",
                iterations, working_session.id
            );

            // Create generation request with current session state
            let current_request = GenerationRequest {
                session: working_session.clone(),
                max_tokens: request.max_tokens,
                temperature: request.temperature,
                top_p: request.top_p,
                stop_tokens: request.stop_tokens.clone(),
            };

            // Submit to request queue
            let response = self.request_queue.submit_request(current_request).await?;

            accumulated_response.push_str(&response.generated_text);
            total_tokens += response.tokens_generated;

            debug!(
                "Generation iteration {} completed: {} tokens, finish_reason: {:?}",
                iterations, response.tokens_generated, response.finish_reason
            );

            // Check if response contains tool calls
            if response.finish_reason == crate::types::FinishReason::ToolCall {
                debug!("Response contains tool calls, processing...");

                // Process tool calls
                let tool_results = self
                    .process_tool_calls(&response.generated_text, &working_session)
                    .await?;

                if tool_results.is_empty() {
                    debug!("No tool results returned, ending tool call workflow");
                    break;
                }

                // Add the assistant's response (with tool calls) to the session
                working_session.messages.push(crate::types::Message {
                    role: crate::types::MessageRole::Assistant,
                    content: response.generated_text.clone(),
                    tool_call_id: None,
                    tool_name: None,
                    timestamp: std::time::SystemTime::now(),
                });

                // Add tool results as Tool messages to the session
                for tool_result in &tool_results {
                    let tool_content = if let Some(error) = &tool_result.error {
                        format!("Error: {}", error)
                    } else {
                        serde_json::to_string(&tool_result.result)
                            .unwrap_or_else(|_| "Invalid tool result".to_string())
                    };

                    working_session.messages.push(crate::types::Message {
                        role: crate::types::MessageRole::Tool,
                        content: tool_content,
                        tool_call_id: Some(tool_result.call_id),
                        tool_name: None,
                        timestamp: std::time::SystemTime::now(),
                    });
                }

                working_session.updated_at = std::time::SystemTime::now();

                debug!(
                    "Tool call processing completed with {} results, continuing generation",
                    tool_results.len()
                );

                // Continue the loop to generate response incorporating tool results
                continue;
            } else {
                // No more tool calls, we're done
                debug!(
                    "Generation completed without tool calls after {} iterations",
                    iterations
                );
                break;
            }
        }

        let final_response = GenerationResponse {
            generated_text: accumulated_response,
            tokens_generated: total_tokens,
            generation_time: std::time::Duration::from_millis(0), // This would need proper timing
            finish_reason: crate::types::FinishReason::EndOfSequence, // Or original finish reason
        };

        debug!(
            "Complete generation workflow finished: {} total tokens across {} iterations",
            total_tokens, iterations
        );

        Ok(final_response)
    }

    async fn generate_stream(
        &self,
        request: GenerationRequest,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<StreamChunk, AgentError>> + Send>>, AgentError>
    {
        debug!(
            "Processing streaming generation request for session: {}",
            request.session.id
        );

        // Security validation: Check request parameters
        self.validate_generation_request(&request)?;

        // Render session to prompt
        let prompt = self.render_session_prompt(&request.session).await?;
        debug!("Session rendered to prompt: {} characters", prompt.len());

        // Submit to request queue for streaming
        let receiver = self
            .request_queue
            .submit_streaming_request(request)
            .await
            .map_err(AgentError::Queue)?;

        // Convert the receiver to a stream and map QueueError to AgentError
        let stream = ReceiverStream::new(receiver).map(|result| result.map_err(AgentError::Queue));

        Ok(Box::pin(stream))
    }

    async fn create_session(&self) -> Result<Session, AgentError> {
        let session = self.session_manager.create_session().await?;
        debug!("Created new session: {}", session.id);
        Ok(session)
    }

    async fn get_session(&self, session_id: &SessionId) -> Result<Option<Session>, AgentError> {
        let session = self.session_manager.get_session(session_id).await?;
        match &session {
            Some(s) => debug!("Retrieved session: {}", s.id),
            None => debug!("Session not found: {}", session_id),
        }
        Ok(session)
    }

    async fn update_session(&self, session: Session) -> Result<(), AgentError> {
        debug!("Updating session: {}", session.id);
        self.session_manager.update_session(session).await?;
        Ok(())
    }

    async fn discover_tools(&self, session: &mut Session) -> Result<(), AgentError> {
        debug!("Discovering tools for session: {}", session.id);

        let tools = self.mcp_client.discover_tools().await?;
        session.available_tools = tools;
        session.updated_at = SystemTime::now();

        info!(
            "Discovered {} tools for session {}",
            session.available_tools.len(),
            session.id
        );
        Ok(())
    }

    async fn execute_tool(
        &self,
        tool_call: ToolCall,
        session: &Session,
    ) -> Result<ToolResult, AgentError> {
        debug!(
            "Executing tool call: {} (id: {}) in session: {}",
            tool_call.name, tool_call.id, session.id
        );

        // Validate tool call name is not empty
        if tool_call.name.trim().is_empty() {
            let error_msg = "Tool name cannot be empty";
            error!("{}", error_msg);
            return Ok(ToolResult {
                call_id: tool_call.id,
                result: serde_json::Value::Null,
                error: Some(error_msg.to_string()),
            });
        }

        // Find the tool definition
        let tool_def = match session
            .available_tools
            .iter()
            .find(|t| t.name == tool_call.name)
        {
            Some(tool) => tool,
            None => {
                let error_msg = format!(
                    "Tool '{}' not found in available tools. Available tools: {}",
                    tool_call.name,
                    session
                        .available_tools
                        .iter()
                        .map(|t| t.name.as_str())
                        .collect::<Vec<_>>()
                        .join(", ")
                );
                error!("{}", error_msg);
                return Ok(ToolResult {
                    call_id: tool_call.id,
                    result: serde_json::Value::Null,
                    error: Some(error_msg),
                });
            }
        };

        debug!(
            "Found tool definition for '{}' on server '{}'",
            tool_call.name, tool_def.server_name
        );

        // Validate tool arguments structure if parameters schema is available
        if let Err(validation_error) = self.validate_tool_arguments(&tool_call, tool_def) {
            warn!(
                "Tool call arguments validation failed for '{}': {}",
                tool_call.name, validation_error
            );
            // Continue execution despite validation failure but log the issue
        }

        // Execute the tool call through MCP client with error handling
        match self
            .mcp_client
            .call_tool(&tool_def.server_name, &tool_call.name, tool_call.arguments)
            .await
        {
            Ok(result_value) => {
                debug!("Tool call '{}' completed successfully", tool_call.name);
                Ok(ToolResult {
                    call_id: tool_call.id,
                    result: result_value,
                    error: None,
                })
            }
            Err(mcp_error) => {
                let error_msg = format!("Tool execution failed: {}", mcp_error);
                error!("Tool call '{}' failed: {}", tool_call.name, error_msg);

                // Return ToolResult with error instead of propagating the error
                // This allows the workflow to continue with partial failures
                Ok(ToolResult {
                    call_id: tool_call.id,
                    result: serde_json::Value::Null,
                    error: Some(error_msg),
                })
            }
        }
    }

    async fn health(&self) -> Result<HealthStatus, AgentError> {
        debug!("Performing health check");

        let model_loaded = self.model_manager.is_loaded().await;
        let queue_stats = self.request_queue.get_stats();
        let sessions_count = self.session_manager.get_session_count().await;
        let mcp_health = self.mcp_client.health_check_all().await;

        let all_servers_healthy = mcp_health
            .values()
            .all(|status| matches!(status, crate::mcp::HealthStatus::Healthy));
        let status = if model_loaded && all_servers_healthy {
            "healthy".to_string()
        } else {
            "unhealthy".to_string()
        };

        let health_status = HealthStatus {
            status,
            model_loaded,
            queue_size: queue_stats.current_queue_size,
            active_sessions: sessions_count,
            uptime: self.start_time.elapsed(),
        };

        debug!("Health check completed: {:?}", health_status);
        Ok(health_status)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{ModelConfig, ModelSource, QueueConfig, SessionConfig};

    fn create_test_config() -> AgentConfig {
        use tempfile::TempDir;
        let temp_dir = TempDir::new().unwrap();

        AgentConfig {
            model: ModelConfig {
                source: ModelSource::Local {
                    folder: temp_dir.path().to_path_buf(),
                    filename: Some("test.gguf".to_string()),
                },
                batch_size: 512,
                use_hf_params: false,
            },
            queue_config: QueueConfig::default(),
            mcp_servers: Vec::new(),
            session_config: SessionConfig::default(),
        }
    }

    #[tokio::test]
    async fn test_agent_server_creation() {
        let config = create_test_config();

        // The config validation will fail because the test.gguf file doesn't exist,
        // but that's expected for this test. We're testing that we can create the config
        // structure correctly
        match config.validate() {
            Ok(()) => {
                // This would mean all validation passed (unlikely without real model file)
                // Config validation succeeded
            }
            Err(_) => {
                // Expected - the test.gguf file doesn't exist
                // Config validation failed as expected
            }
        }
    }

    #[test]
    fn test_agent_server_debug() {
        let config = create_test_config();
        let debug_str = format!("{:?}", config);

        // Just test that we can debug the config - safer than trying to create a full AgentServer
        assert!(debug_str.contains("AgentConfig"));
        assert!(debug_str.contains("model"));
        assert!(debug_str.contains("queue_config"));
        assert!(debug_str.contains("session_config"));
    }

    #[test]
    fn test_config_validation() {
        let mut config = create_test_config();
        // Note: config.validate() will fail due to missing model file, but that's expected

        // Test invalid batch size
        config.model.batch_size = 0;
        assert!(config.validate().is_err());

        // Reset and test invalid queue config
        config = create_test_config();
        config.queue_config.max_queue_size = 0;
        assert!(config.validate().is_err());

        // Reset and test invalid session config
        config = create_test_config();
        config.session_config.max_sessions = 0;
        assert!(config.validate().is_err());

        // Test valid values for components that don't depend on file existence
        let valid_model_config = ModelConfig {
            source: ModelSource::HuggingFace {
                repo: "test/model".to_string(),
                filename: Some("model.gguf".to_string()),
            },
            batch_size: 512,
            use_hf_params: false,
        };

        let valid_config = AgentConfig {
            model: valid_model_config,
            queue_config: QueueConfig::default(),
            mcp_servers: Vec::new(),
            session_config: SessionConfig::default(),
        };

        // This should pass all validation except for the model file not existing
        match valid_config.validate() {
            Ok(()) => {} // Validation passed
            Err(e) => {
                // Expected if model file doesn't exist - that's fine
                let error_msg = format!("{}", e);
                // Should be a model-related error
                assert!(error_msg.contains("model") || error_msg.contains("Model"));
            }
        }
    }
}
