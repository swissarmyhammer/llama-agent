use async_trait::async_trait;
use futures::Stream;
use serde::{Deserialize, Serialize};
use std::pin::Pin;
use std::time::{Duration, SystemTime};
use thiserror::Error;
use ulid::Ulid;

#[cfg(test)]
use std::path::PathBuf;

// Re-export model types from llama-loader
pub use llama_loader::{ModelConfig, ModelError, ModelSource, RetryConfig};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SessionId(Ulid);

impl SessionId {
    pub fn new() -> Self {
        Self(Ulid::new())
    }

    pub fn from_ulid(ulid: Ulid) -> Self {
        Self(ulid)
    }

    pub fn as_ulid(&self) -> Ulid {
        self.0
    }
}

impl std::fmt::Display for SessionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::str::FromStr for SessionId {
    type Err = ulid::DecodeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(Ulid::from_string(s)?))
    }
}

impl Default for SessionId {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ToolCallId(Ulid);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PromptId(Ulid);

impl PromptId {
    pub fn new() -> Self {
        Self(Ulid::new())
    }

    pub fn from_ulid(ulid: Ulid) -> Self {
        Self(ulid)
    }

    pub fn as_ulid(&self) -> Ulid {
        self.0
    }
}

impl std::fmt::Display for PromptId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::str::FromStr for PromptId {
    type Err = ulid::DecodeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(Ulid::from_string(s)?))
    }
}

impl Default for PromptId {
    fn default() -> Self {
        Self::new()
    }
}

impl ToolCallId {
    pub fn new() -> Self {
        Self(Ulid::new())
    }

    pub fn from_ulid(ulid: Ulid) -> Self {
        Self(ulid)
    }

    pub fn as_ulid(&self) -> Ulid {
        self.0
    }
}

impl std::fmt::Display for ToolCallId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::str::FromStr for ToolCallId {
    type Err = ulid::DecodeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(Ulid::from_string(s)?))
    }
}

impl Default for ToolCallId {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: MessageRole,
    pub content: String,
    pub tool_call_id: Option<ToolCallId>,
    pub tool_name: Option<String>,
    pub timestamp: SystemTime,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MessageRole {
    System,
    User,
    Assistant,
    Tool,
}

impl MessageRole {
    pub fn as_str(&self) -> &'static str {
        match self {
            MessageRole::System => "system",
            MessageRole::User => "user",
            MessageRole::Assistant => "assistant",
            MessageRole::Tool => "tool",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: SessionId,
    pub messages: Vec<Message>,
    pub mcp_servers: Vec<MCPServerConfig>,
    pub available_tools: Vec<ToolDefinition>,
    pub available_prompts: Vec<PromptDefinition>,
    pub created_at: SystemTime,
    pub updated_at: SystemTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MCPServerConfig {
    pub name: String,
    pub command: String,
    pub args: Vec<String>,
    pub timeout_secs: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
    pub server_name: String,
}

#[derive(Debug, Clone)]
pub struct StoppingConfig {
    pub max_tokens: Option<usize>,
    pub repetition_detection: Option<RepetitionConfig>,
    pub eos_detection: bool,
}

impl Default for StoppingConfig {
    fn default() -> Self {
        Self {
            max_tokens: None,
            repetition_detection: None,
            eos_detection: true,
        }
    }
}

impl StoppingConfig {
    /// Validate the stopping configuration for reasonable limits
    pub fn validate(&self) -> Result<(), String> {
        // Validate max_tokens
        if let Some(max_tokens) = self.max_tokens {
            if max_tokens == 0 {
                return Err("max_tokens must be greater than 0".to_string());
            }
            if max_tokens > 100_000 {
                return Err("max_tokens cannot exceed 100,000 for safety".to_string());
            }
        }

        // Validate repetition_detection config
        if let Some(ref repetition_config) = self.repetition_detection {
            if repetition_config.min_pattern_length == 0 {
                return Err("min_pattern_length must be greater than 0".to_string());
            }
            if repetition_config.max_pattern_length < repetition_config.min_pattern_length {
                return Err("max_pattern_length must be >= min_pattern_length".to_string());
            }
            if repetition_config.min_repetitions < 2 {
                return Err("min_repetitions must be at least 2".to_string());
            }
            if repetition_config.window_size == 0 {
                return Err("window_size must be greater than 0".to_string());
            }
            if repetition_config.window_size > 100_000 {
                return Err("window_size cannot exceed 100,000 for memory safety".to_string());
            }
        }

        Ok(())
    }

    /// Create a validated StoppingConfig
    pub fn new_validated(
        max_tokens: Option<usize>,
        repetition_detection: Option<RepetitionConfig>,
        eos_detection: bool,
    ) -> Result<Self, String> {
        let config = Self {
            max_tokens,
            repetition_detection,
            eos_detection,
        };
        config.validate()?;
        Ok(config)
    }
}

// Re-export RepetitionConfig from stopper module to avoid duplication
pub use crate::stopper::repetition::RepetitionConfig;

#[derive(Debug)]
pub struct GenerationRequest {
    pub session_id: SessionId,
    pub max_tokens: Option<u32>,
    pub temperature: Option<f32>,
    pub top_p: Option<f32>,
    pub stop_tokens: Vec<String>,
    pub stopping_config: Option<StoppingConfig>,
}

impl GenerationRequest {
    /// Create a new GenerationRequest with default stopping configuration
    pub fn new(session_id: SessionId) -> Self {
        Self {
            session_id,
            max_tokens: None,
            temperature: None,
            top_p: None,
            stop_tokens: Vec::new(),
            stopping_config: None,
        }
    }

    /// Create a GenerationRequest with default stopping config if none is provided
    pub fn with_default_stopping(mut self) -> Self {
        if self.stopping_config.is_none() {
            self.stopping_config = Some(StoppingConfig::default());
        }
        self
    }

    /// Create a GenerationRequest with custom stopping configuration
    pub fn with_stopping_config(mut self, config: StoppingConfig) -> Self {
        self.stopping_config = Some(config);
        self
    }

    /// Create a GenerationRequest with validated stopping configuration
    pub fn with_validated_stopping_config(
        mut self,
        config: StoppingConfig,
    ) -> Result<Self, String> {
        config.validate()?;
        self.stopping_config = Some(config);
        Ok(self)
    }

    /// Set max_tokens using builder pattern
    pub fn with_max_tokens(mut self, max_tokens: u32) -> Self {
        self.max_tokens = Some(max_tokens);
        self
    }

    /// Set temperature using builder pattern
    pub fn with_temperature(mut self, temperature: f32) -> Self {
        self.temperature = Some(temperature);
        self
    }

    /// Set top_p using builder pattern
    pub fn with_top_p(mut self, top_p: f32) -> Self {
        self.top_p = Some(top_p);
        self
    }

    /// Set stop_tokens using builder pattern
    pub fn with_stop_tokens(mut self, stop_tokens: Vec<String>) -> Self {
        self.stop_tokens = stop_tokens;
        self
    }

    /// Get the effective max_tokens considering both the direct field and stopping_config
    pub fn effective_max_tokens(&self) -> Option<u32> {
        // Priority: direct max_tokens field, then stopping_config max_tokens, then None
        self.max_tokens.or_else(|| {
            self.stopping_config
                .as_ref()
                .and_then(|config| config.max_tokens.map(|val| val as u32))
        })
    }

    /// Migrate max_tokens to stopping_config for consistency
    pub fn migrate_max_tokens_to_stopping_config(mut self) -> Self {
        if let Some(max_tokens) = self.max_tokens {
            let max_tokens_usize = max_tokens as usize;

            match &mut self.stopping_config {
                Some(config) => {
                    // If stopping_config exists but no max_tokens is set, use the direct field
                    if config.max_tokens.is_none() {
                        config.max_tokens = Some(max_tokens_usize);
                    }
                    // Clear the direct field since we've moved it to stopping_config
                    self.max_tokens = None;
                }
                None => {
                    // Create new stopping config with the max_tokens
                    self.stopping_config = Some(StoppingConfig {
                        max_tokens: Some(max_tokens_usize),
                        ..StoppingConfig::default()
                    });
                    self.max_tokens = None;
                }
            }
        }
        self
    }
}

#[derive(Debug)]
pub struct GenerationResponse {
    pub generated_text: String,
    pub tokens_generated: u32,
    pub generation_time: Duration,
    pub finish_reason: FinishReason,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FinishReason {
    Stopped(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: ToolCallId,
    pub name: String,
    pub arguments: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    pub call_id: ToolCallId,
    pub result: serde_json::Value,
    pub error: Option<String>,
}

// MCP Prompt types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptDefinition {
    pub name: String,
    pub title: Option<String>,
    pub description: Option<String>,
    pub arguments: Option<Vec<PromptArgument>>,
    pub server_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptArgument {
    pub name: String,
    pub description: Option<String>,
    pub required: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptMessage {
    pub role: PromptRole,
    pub content: PromptContent,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PromptRole {
    #[serde(rename = "user")]
    User,
    #[serde(rename = "assistant")]
    Assistant,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum PromptContent {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "image")]
    Image { data: String, mime_type: String },
    #[serde(rename = "resource")]
    Resource { resource: PromptResource },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptResource {
    pub uri: String,
    pub name: String,
    pub title: Option<String>,
    pub mime_type: String,
    pub text: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetPromptResult {
    pub description: Option<String>,
    pub messages: Vec<PromptMessage>,
}

// Dependency Analysis types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParallelExecutionConfig {
    pub max_parallel_tools: usize,
    pub conflict_detection: bool,
    pub resource_analysis: bool,
    pub timeout_ms: u64,
    pub never_parallel: Vec<(String, String)>,
    pub tool_conflicts: Vec<ToolConflict>,
    pub resource_access_patterns: std::collections::HashMap<String, Vec<ResourceAccess>>,
}

impl Default for ParallelExecutionConfig {
    fn default() -> Self {
        Self {
            max_parallel_tools: 4,
            conflict_detection: true,
            resource_analysis: true,
            timeout_ms: 30000,
            never_parallel: Vec::new(),
            tool_conflicts: Vec::new(),
            resource_access_patterns: std::collections::HashMap::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AccessType {
    Read,
    Write,
    ReadWrite,
    Delete,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceAccess {
    pub resource: ResourceType,
    pub access_type: AccessType,
    pub exclusive: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ResourceType {
    File(String),
    FileSystem(String),
    Network(String),
    Database(String),
    Memory,
    System,
    Other(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ConflictType {
    ResourceConflict,
    DependencyConflict,
    OrderDependency,
    MutualExclusion,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolConflict {
    pub tool1: String,
    pub tool2: String,
    pub conflict_type: ConflictType,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ReferenceType {
    Input,
    Output,
    Context,
    DirectOutput,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParameterReference {
    pub parameter_name: String,
    pub parameter_path: String,
    pub reference_type: ReferenceType,
    pub target_tool: Option<String>,
    pub referenced_tool: String,
}

#[derive(Debug)]
pub struct StreamChunk {
    pub text: String,
    pub is_complete: bool,
    pub token_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AgentConfig {
    pub model: ModelConfig,
    pub queue_config: QueueConfig,
    pub mcp_servers: Vec<MCPServerConfig>,
    pub session_config: SessionConfig,
}




#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueConfig {
    pub max_queue_size: usize,
    pub request_timeout: Duration,
    pub worker_threads: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionConfig {
    pub max_sessions: usize,
    pub session_timeout: Duration,
}

impl Default for SessionConfig {
    fn default() -> Self {
        Self {
            max_sessions: 1000,
            session_timeout: Duration::from_secs(3600), // 1 hour
        }
    }
}


impl Default for QueueConfig {
    fn default() -> Self {
        Self {
            max_queue_size: 100,
            request_timeout: Duration::from_secs(30),
            worker_threads: 1,
        }
    }
}



impl QueueConfig {
    pub fn validate(&self) -> Result<(), QueueError> {
        if self.max_queue_size == 0 {
            return Err(QueueError::WorkerError(
                "Queue size must be greater than 0".to_string(),
            ));
        }

        if self.worker_threads == 0 {
            return Err(QueueError::WorkerError(
                "Worker threads must be greater than 0".to_string(),
            ));
        }

        if self.worker_threads > 16 {
            return Err(QueueError::WorkerError(
                "Worker threads should not exceed 16 for most systems".to_string(),
            ));
        }

        if self.request_timeout.as_secs() == 0 {
            return Err(QueueError::WorkerError(
                "Request timeout must be greater than 0 seconds".to_string(),
            ));
        }

        Ok(())
    }
}

impl SessionConfig {
    pub fn validate(&self) -> Result<(), SessionError> {
        if self.max_sessions == 0 {
            return Err(SessionError::InvalidState(
                "Max sessions must be greater than 0".to_string(),
            ));
        }

        if self.session_timeout.as_secs() == 0 {
            return Err(SessionError::InvalidState(
                "Session timeout must be greater than 0 seconds".to_string(),
            ));
        }

        Ok(())
    }
}

impl MCPServerConfig {
    pub fn validate(&self) -> Result<(), MCPError> {
        if self.name.is_empty() {
            return Err(MCPError::Protocol(
                "MCP server name cannot be empty".to_string(),
            ));
        }

        if self.command.is_empty() {
            return Err(MCPError::Protocol(
                "MCP server command cannot be empty".to_string(),
            ));
        }

        // Check for invalid characters in name
        if self
            .name
            .chars()
            .any(|c| !c.is_alphanumeric() && !"-_".contains(c))
        {
            return Err(MCPError::Protocol(
                "MCP server name contains invalid characters".to_string(),
            ));
        }

        Ok(())
    }
}

impl AgentConfig {
    pub fn validate(&self) -> Result<(), AgentError> {
        self.model.validate()?;
        self.queue_config.validate()?;
        self.session_config.validate()?;

        for server_config in &self.mcp_servers {
            server_config.validate()?;
        }

        // Check for duplicate MCP server names
        let mut server_names = std::collections::HashSet::new();
        for server_config in &self.mcp_servers {
            if !server_names.insert(&server_config.name) {
                return Err(AgentError::MCP(MCPError::Protocol(format!(
                    "Duplicate MCP server name: {}",
                    server_config.name
                ))));
            }
        }

        Ok(())
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct HealthStatus {
    pub status: String,
    pub model_loaded: bool,
    pub queue_size: usize,
    pub active_sessions: usize,
    pub uptime: Duration,
}

// Error types
#[derive(Debug, Error)]
pub enum AgentError {
    #[error("Model error: {0}\n💡 Check model file exists, is valid GGUF format, and sufficient memory is available")]
    Model(#[from] ModelError),

    #[error("Request processing error: {0}\n💡 Try reducing concurrent requests, increasing queue size, or adding more system resources")]
    Queue(#[from] QueueError),

    #[error(
        "Session error: {0}\n💡 Verify session ID is valid and session limits are not exceeded"
    )]
    Session(#[from] SessionError),

    #[error("MCP server error: {0}\n💡 Ensure MCP server is running, accessible, and check network connectivity")]
    MCP(#[from] MCPError),

    #[error("Template processing error: {0}\n💡 Check message format and tool definitions are properly structured")]
    Template(#[from] TemplateError),

    #[error("Request timeout: processing took longer than {timeout:?}\n💡 Increase timeout settings, reduce max_tokens, or check system performance")]
    Timeout { timeout: Duration },

    #[error("Queue overloaded: {capacity} requests queued (max capacity)\n💡 Wait and retry, or increase max_queue_size configuration")]
    QueueFull { capacity: usize },
}


#[derive(Debug, Clone, Error)]
pub enum QueueError {
    #[error("Queue is full")]
    Full,

    #[error("Request timeout")]
    Timeout,

    #[error("Worker thread error: {0}")]
    WorkerError(String),
}

#[derive(Debug, Error)]
pub enum SessionError {
    #[error("Session not found: {0}")]
    NotFound(String),

    #[error("Session limit exceeded")]
    LimitExceeded,

    #[error("Session timeout")]
    Timeout,

    #[error("Invalid session state: {0}")]
    InvalidState(String),
}

#[derive(Debug, Error)]
pub enum MCPError {
    #[error("MCP server not found: {0}")]
    ServerNotFound(String),

    #[error("Tool call failed: {0}")]
    ToolCallFailed(String),

    #[error("Connection error: {0}")]
    Connection(String),

    #[error("Protocol error: {0}")]
    Protocol(String),
}

#[derive(Debug, Error)]
pub enum TemplateError {
    #[error("Template rendering failed: {0}")]
    RenderingFailed(String),

    #[error("Tool call parsing failed: {0}")]
    ToolCallParsing(String),

    #[error("Invalid template: {0}")]
    Invalid(String),
}

// Main API trait
#[async_trait]
pub trait AgentAPI {
    async fn initialize(config: AgentConfig) -> Result<Self, AgentError>
    where
        Self: Sized;

    async fn generate(&self, request: GenerationRequest) -> Result<GenerationResponse, AgentError>;

    async fn generate_stream(
        &self,
        request: GenerationRequest,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<StreamChunk, AgentError>> + Send>>, AgentError>;

    async fn create_session(&self) -> Result<Session, AgentError>;

    async fn get_session(&self, session_id: &SessionId) -> Result<Option<Session>, AgentError>;

    async fn add_message(&self, session_id: &SessionId, message: Message)
        -> Result<(), AgentError>;

    async fn discover_tools(&self, session: &mut Session) -> Result<(), AgentError>;

    async fn execute_tool(
        &self,
        tool_call: ToolCall,
        session: &Session,
    ) -> Result<ToolResult, AgentError>;

    async fn health(&self) -> Result<HealthStatus, AgentError>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::SystemTime;

    #[test]
    fn test_message_role_as_str() {
        assert_eq!(MessageRole::System.as_str(), "system");
        assert_eq!(MessageRole::User.as_str(), "user");
        assert_eq!(MessageRole::Assistant.as_str(), "assistant");
        assert_eq!(MessageRole::Tool.as_str(), "tool");
    }

    #[test]
    fn test_message_creation() {
        let message = Message {
            role: MessageRole::User,
            content: "Hello, world!".to_string(),
            tool_call_id: None,
            tool_name: None,
            timestamp: SystemTime::now(),
        };

        assert_eq!(message.role.as_str(), "user");
        assert_eq!(message.content, "Hello, world!");
        assert!(message.tool_call_id.is_none());
        assert!(message.tool_name.is_none());
    }

    #[test]
    fn test_session_creation() {
        let session = Session {
            id: SessionId::new(),
            messages: Vec::new(),
            mcp_servers: Vec::new(),
            available_tools: Vec::new(),
            available_prompts: Vec::new(),
            created_at: SystemTime::now(),
            updated_at: SystemTime::now(),
        };

        assert!(!session.id.to_string().is_empty());
        assert!(session.messages.is_empty());
        assert!(session.mcp_servers.is_empty());
        assert!(session.available_tools.is_empty());
        assert!(session.available_prompts.is_empty());
    }

    #[test]
    fn test_mcp_server_config() {
        let config = MCPServerConfig {
            name: "filesystem".to_string(),
            command: "npx".to_string(),
            args: vec![
                "-y".to_string(),
                "@modelcontextprotocol/server-filesystem".to_string(),
            ],
            timeout_secs: None,
        };

        assert_eq!(config.name, "filesystem");
        assert_eq!(config.command, "npx");
        assert_eq!(config.args.len(), 2);
    }

    #[test]
    fn test_tool_definition() {
        let tool = ToolDefinition {
            name: "list_files".to_string(),
            description: "List files in a directory".to_string(),
            parameters: serde_json::json!({"type": "object"}),
            server_name: "filesystem".to_string(),
        };

        assert_eq!(tool.name, "list_files");
        assert_eq!(tool.server_name, "filesystem");
    }

    #[test]
    fn test_generation_request() {
        let session = Session {
            id: SessionId::new(),
            messages: Vec::new(),
            mcp_servers: Vec::new(),
            available_tools: Vec::new(),
            available_prompts: Vec::new(),
            created_at: SystemTime::now(),
            updated_at: SystemTime::now(),
        };

        let request = GenerationRequest {
            session_id: session.id.clone(),
            max_tokens: Some(100),
            temperature: Some(0.7),
            top_p: Some(0.9),
            stop_tokens: vec!["</s>".to_string()],
            stopping_config: None,
        };

        assert_eq!(request.max_tokens, Some(100));
        assert_eq!(request.temperature, Some(0.7));
        assert_eq!(request.stop_tokens.len(), 1);
    }

    #[test]
    fn test_model_source_variants() {
        let hf_source = ModelSource::HuggingFace {
            repo: "microsoft/DialoGPT-medium".to_string(),
            filename: None,
        };

        let local_source = ModelSource::Local {
            folder: PathBuf::from("/models/llama2"),
            filename: Some("model.gguf".to_string()),
        };

        match hf_source {
            ModelSource::HuggingFace { repo, .. } => assert_eq!(repo, "microsoft/DialoGPT-medium"),
            _ => panic!("Wrong variant"),
        }

        match local_source {
            ModelSource::Local { filename, .. } => {
                assert_eq!(filename, Some("model.gguf".to_string()))
            }
            _ => panic!("Wrong variant"),
        }
    }

    #[test]
    fn test_finish_reason() {
        let reasons = [
            FinishReason::Stopped("Maximum tokens reached".to_string()),
            FinishReason::Stopped("Stop token detected".to_string()),
            FinishReason::Stopped("End of sequence token detected".to_string()),
            FinishReason::Stopped("Tool call detected".to_string()),
            FinishReason::Stopped("Error: test error".to_string()),
        ];

        assert_eq!(reasons.len(), 5);

        match &reasons[4] {
            FinishReason::Stopped(msg) => assert_eq!(msg, "Error: test error"),
            _ => panic!("Wrong variant"),
        }
    }

    #[test]
    fn test_session_config_default() {
        let config = SessionConfig::default();
        assert_eq!(config.max_sessions, 1000);
        assert_eq!(config.session_timeout, Duration::from_secs(3600));
    }

    #[test]
    fn test_stream_chunk() {
        let chunk = StreamChunk {
            text: "Hello".to_string(),
            is_complete: false,
            token_count: 1,
        };

        assert_eq!(chunk.text, "Hello");
        assert!(!chunk.is_complete);
        assert_eq!(chunk.token_count, 1);
    }

    #[test]
    fn test_tool_call_serialization() {
        let tool_call = ToolCall {
            id: ToolCallId::new(),
            name: "list_files".to_string(),
            arguments: serde_json::json!({"path": "/tmp"}),
        };

        let serialized = serde_json::to_string(&tool_call).unwrap();
        let deserialized: ToolCall = serde_json::from_str(&serialized).unwrap();

        assert_eq!(deserialized.id.to_string(), tool_call.id.to_string());
        assert_eq!(deserialized.name, "list_files");
    }

    #[test]
    fn test_session_id() {
        let session_id = SessionId::new();
        let session_id_str = session_id.to_string();

        // Test that we can parse back the string representation
        let parsed_session_id: SessionId = session_id_str.parse().unwrap();
        assert_eq!(session_id, parsed_session_id);

        // Test serialization
        let serialized = serde_json::to_string(&session_id).unwrap();
        let deserialized: SessionId = serde_json::from_str(&serialized).unwrap();
        assert_eq!(session_id, deserialized);

        // Test Display trait
        assert!(!format!("{}", session_id).is_empty());
    }

    #[test]
    fn test_tool_call_id() {
        let tool_call_id = ToolCallId::new();
        let tool_call_id_str = tool_call_id.to_string();

        // Test that we can parse back the string representation
        let parsed_tool_call_id: ToolCallId = tool_call_id_str.parse().unwrap();
        assert_eq!(tool_call_id, parsed_tool_call_id);

        // Test serialization
        let serialized = serde_json::to_string(&tool_call_id).unwrap();
        let deserialized: ToolCallId = serde_json::from_str(&serialized).unwrap();
        assert_eq!(tool_call_id, deserialized);

        // Test Display trait
        assert!(!format!("{}", tool_call_id).is_empty());
    }

    #[test]
    fn test_message_with_tool_call() {
        let tool_call_id = ToolCallId::new();
        let message = Message {
            role: MessageRole::Tool,
            content: "Tool response content".to_string(),
            tool_call_id: Some(tool_call_id),
            tool_name: Some("test_tool".to_string()),
            timestamp: SystemTime::now(),
        };

        assert_eq!(message.role.as_str(), "tool");
        assert_eq!(message.tool_call_id, Some(tool_call_id));
        assert_eq!(message.tool_name.as_ref().unwrap(), "test_tool");
    }

    #[test]
    fn test_tool_result() {
        let call_id = ToolCallId::new();
        let result = ToolResult {
            call_id,
            result: serde_json::json!({"status": "success"}),
            error: None,
        };

        assert_eq!(result.call_id, call_id);
        assert!(result.error.is_none());

        // Test serialization
        let serialized = serde_json::to_string(&result).unwrap();
        let deserialized: ToolResult = serde_json::from_str(&serialized).unwrap();
        assert_eq!(deserialized.call_id, call_id);
    }

    #[test]
    fn test_config_defaults() {
        let model_config = ModelConfig::default();
        match model_config.source {
            ModelSource::HuggingFace { ref repo, .. } => {
                assert_eq!(repo, "microsoft/DialoGPT-medium")
            }
            _ => panic!("Wrong default model source"),
        }
        assert_eq!(model_config.batch_size, 512);
        assert!(model_config.use_hf_params);

        let queue_config = QueueConfig::default();
        assert_eq!(queue_config.max_queue_size, 100);
        assert_eq!(queue_config.request_timeout, Duration::from_secs(30));
        assert_eq!(queue_config.worker_threads, 1);

        let session_config = SessionConfig::default();
        assert_eq!(session_config.max_sessions, 1000);
        assert_eq!(session_config.session_timeout, Duration::from_secs(3600));

        let agent_config = AgentConfig::default();
        assert!(agent_config.mcp_servers.is_empty());
    }

    #[test]
    fn test_config_serialization() {
        let config = AgentConfig::default();

        let serialized = serde_json::to_string(&config).unwrap();
        let deserialized: AgentConfig = serde_json::from_str(&serialized).unwrap();

        // Compare key fields
        assert_eq!(
            deserialized.queue_config.max_queue_size,
            config.queue_config.max_queue_size
        );
        assert_eq!(
            deserialized.session_config.max_sessions,
            config.session_config.max_sessions
        );
        assert_eq!(deserialized.model.batch_size, config.model.batch_size);
    }

    #[test]
    fn test_model_config_validation_valid() {
        let config = ModelConfig {
            source: ModelSource::HuggingFace {
                repo: "microsoft/DialoGPT-medium".to_string(),
                filename: Some("model.gguf".to_string()),
            },
            batch_size: 512,
            use_hf_params: true,
            retry_config: RetryConfig::default(),
            debug: false,
        };

        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_model_config_validation_invalid_batch_size() {
        let config = ModelConfig {
            source: ModelSource::HuggingFace {
                repo: "microsoft/DialoGPT-medium".to_string(),
                filename: None,
            },
            batch_size: 0,
            use_hf_params: true,
            retry_config: RetryConfig::default(),
            debug: false,
        };

        assert!(config.validate().is_err());

        let config = ModelConfig {
            source: ModelSource::HuggingFace {
                repo: "microsoft/DialoGPT-medium".to_string(),
                filename: None,
            },
            batch_size: 10000,
            use_hf_params: true,
            retry_config: RetryConfig::default(),
            debug: false,
        };

        assert!(config.validate().is_err());
    }

    #[test]
    fn test_model_source_validation_huggingface() {
        // Valid HuggingFace repo
        let source = ModelSource::HuggingFace {
            repo: "microsoft/DialoGPT-medium".to_string(),
            filename: Some("model.gguf".to_string()),
        };
        assert!(source.validate().is_ok());

        // Empty repo
        let source = ModelSource::HuggingFace {
            repo: "".to_string(),
            filename: None,
        };
        assert!(source.validate().is_err());

        // Invalid repo format (no slash)
        let source = ModelSource::HuggingFace {
            repo: "invalid-repo".to_string(),
            filename: None,
        };
        assert!(source.validate().is_err());

        // Invalid filename extension
        let source = ModelSource::HuggingFace {
            repo: "microsoft/DialoGPT-medium".to_string(),
            filename: Some("model.txt".to_string()),
        };
        assert!(source.validate().is_err());

        // Empty filename
        let source = ModelSource::HuggingFace {
            repo: "microsoft/DialoGPT-medium".to_string(),
            filename: Some("".to_string()),
        };
        assert!(source.validate().is_err());
    }

    #[test]
    fn test_model_source_validation_local() {
        // Test with actual temp directory
        let temp_dir = std::env::temp_dir();

        // Valid local source with existing directory
        let source = ModelSource::Local {
            folder: temp_dir.clone(),
            filename: None,
        };
        assert!(source.validate().is_ok());

        // Non-existent directory
        let source = ModelSource::Local {
            folder: PathBuf::from("/non/existent/path"),
            filename: None,
        };
        assert!(source.validate().is_err());

        // Empty filename
        let source = ModelSource::Local {
            folder: temp_dir,
            filename: Some("".to_string()),
        };
        assert!(source.validate().is_err());
    }

    #[test]
    fn test_queue_config_validation() {
        // Valid config
        let config = QueueConfig {
            max_queue_size: 100,
            request_timeout: Duration::from_secs(30),
            worker_threads: 2,
        };
        assert!(config.validate().is_ok());

        // Invalid queue size
        let config = QueueConfig {
            max_queue_size: 0,
            request_timeout: Duration::from_secs(30),
            worker_threads: 1,
        };
        assert!(config.validate().is_err());

        // Invalid worker threads
        let config = QueueConfig {
            max_queue_size: 100,
            request_timeout: Duration::from_secs(30),
            worker_threads: 0,
        };
        assert!(config.validate().is_err());

        // Too many worker threads
        let config = QueueConfig {
            max_queue_size: 100,
            request_timeout: Duration::from_secs(30),
            worker_threads: 20,
        };
        assert!(config.validate().is_err());

        // Invalid timeout
        let config = QueueConfig {
            max_queue_size: 100,
            request_timeout: Duration::from_secs(0),
            worker_threads: 1,
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_session_config_validation() {
        // Valid config
        let config = SessionConfig {
            max_sessions: 1000,
            session_timeout: Duration::from_secs(3600),
        };
        assert!(config.validate().is_ok());

        // Invalid max sessions
        let config = SessionConfig {
            max_sessions: 0,
            session_timeout: Duration::from_secs(3600),
        };
        assert!(config.validate().is_err());

        // Invalid timeout
        let config = SessionConfig {
            max_sessions: 1000,
            session_timeout: Duration::from_secs(0),
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_mcp_server_config_validation() {
        // Valid config
        let config = MCPServerConfig {
            name: "filesystem".to_string(),
            command: "npx".to_string(),
            args: vec!["-y".to_string()],
            timeout_secs: None,
        };
        assert!(config.validate().is_ok());

        // Empty name
        let config = MCPServerConfig {
            name: "".to_string(),
            command: "npx".to_string(),
            args: vec![],
            timeout_secs: None,
        };
        assert!(config.validate().is_err());

        // Empty command
        let config = MCPServerConfig {
            name: "filesystem".to_string(),
            command: "".to_string(),
            args: vec![],
            timeout_secs: None,
        };
        assert!(config.validate().is_err());

        // Invalid characters in name
        let config = MCPServerConfig {
            name: "file@system".to_string(),
            command: "npx".to_string(),
            args: vec![],
            timeout_secs: None,
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_agent_config_validation() {
        // Valid config
        let config = AgentConfig::default();
        assert!(config.validate().is_ok());

        // Config with duplicate MCP server names
        let config = AgentConfig {
            mcp_servers: vec![
                MCPServerConfig {
                    name: "filesystem".to_string(),
                    command: "npx".to_string(),
                    args: vec![],
                    timeout_secs: None,
                },
                MCPServerConfig {
                    name: "filesystem".to_string(),
                    command: "another".to_string(),
                    args: vec![],
                    timeout_secs: None,
                },
            ],
            ..Default::default()
        };
        assert!(config.validate().is_err());

        // Config with invalid model
        let mut config = AgentConfig::default();
        config.model.batch_size = 0;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_model_source_serialization() {
        let hf_source = ModelSource::HuggingFace {
            repo: "microsoft/DialoGPT-medium".to_string(),
            filename: Some("model.gguf".to_string()),
        };

        let serialized = serde_json::to_string(&hf_source).unwrap();
        let deserialized: ModelSource = serde_json::from_str(&serialized).unwrap();

        match deserialized {
            ModelSource::HuggingFace { repo, filename } => {
                assert_eq!(repo, "microsoft/DialoGPT-medium");
                assert_eq!(filename, Some("model.gguf".to_string()));
            }
            _ => panic!("Wrong variant after deserialization"),
        }

        let local_source = ModelSource::Local {
            folder: PathBuf::from("/tmp/models"),
            filename: None,
        };

        let serialized = serde_json::to_string(&local_source).unwrap();
        let deserialized: ModelSource = serde_json::from_str(&serialized).unwrap();

        match deserialized {
            ModelSource::Local { folder, filename } => {
                assert_eq!(folder, PathBuf::from("/tmp/models"));
                assert_eq!(filename, None);
            }
            _ => panic!("Wrong variant after deserialization"),
        }
    }

    #[test]
    fn test_stopping_config_validation() {
        // Valid config should pass
        let config = StoppingConfig {
            max_tokens: Some(100),
            repetition_detection: Some(RepetitionConfig::default()),
            eos_detection: true,
        };
        assert!(config.validate().is_ok());

        // Zero max_tokens should fail
        let config = StoppingConfig {
            max_tokens: Some(0),
            repetition_detection: None,
            eos_detection: true,
        };
        assert!(config.validate().is_err());

        // Extremely high max_tokens should fail
        let config = StoppingConfig {
            max_tokens: Some(200_000),
            repetition_detection: None,
            eos_detection: true,
        };
        assert!(config.validate().is_err());

        // Invalid repetition config should fail
        let config = StoppingConfig {
            max_tokens: Some(100),
            repetition_detection: Some(RepetitionConfig {
                min_pattern_length: 0,
                max_pattern_length: 10,
                min_repetitions: 3,
                window_size: 1000,
            }),
            eos_detection: true,
        };
        assert!(config.validate().is_err());

        // max_pattern_length < min_pattern_length should fail
        let config = StoppingConfig {
            max_tokens: Some(100),
            repetition_detection: Some(RepetitionConfig {
                min_pattern_length: 20,
                max_pattern_length: 10,
                min_repetitions: 3,
                window_size: 1000,
            }),
            eos_detection: true,
        };
        assert!(config.validate().is_err());

        // min_repetitions < 2 should fail
        let config = StoppingConfig {
            max_tokens: Some(100),
            repetition_detection: Some(RepetitionConfig {
                min_pattern_length: 10,
                max_pattern_length: 100,
                min_repetitions: 1,
                window_size: 1000,
            }),
            eos_detection: true,
        };
        assert!(config.validate().is_err());

        // Zero window_size should fail
        let config = StoppingConfig {
            max_tokens: Some(100),
            repetition_detection: Some(RepetitionConfig {
                min_pattern_length: 10,
                max_pattern_length: 100,
                min_repetitions: 3,
                window_size: 0,
            }),
            eos_detection: true,
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_stopping_config_new_validated() {
        // Valid config should create successfully
        let config =
            StoppingConfig::new_validated(Some(100), Some(RepetitionConfig::default()), true);
        assert!(config.is_ok());

        // Invalid config should fail creation
        let config = StoppingConfig::new_validated(
            Some(0), // Invalid max_tokens
            None,
            true,
        );
        assert!(config.is_err());
    }

    #[test]
    fn test_generation_request_builder_methods() {
        let session_id = SessionId::new();

        // Test basic builder pattern
        let request = GenerationRequest::new(session_id.clone())
            .with_max_tokens(100)
            .with_temperature(0.7)
            .with_top_p(0.9)
            .with_stop_tokens(vec!["</s>".to_string()])
            .with_default_stopping();

        assert_eq!(request.max_tokens, Some(100));
        assert_eq!(request.temperature, Some(0.7));
        assert_eq!(request.top_p, Some(0.9));
        assert_eq!(request.stop_tokens, vec!["</s>".to_string()]);
        assert!(request.stopping_config.is_some());

        // Test validated stopping config (should succeed with valid config)
        let stopping_config = StoppingConfig::default();
        let request = GenerationRequest::new(session_id.clone())
            .with_validated_stopping_config(stopping_config);
        assert!(request.is_ok());

        // Test validated stopping config (should fail with invalid config)
        let invalid_config = StoppingConfig {
            max_tokens: Some(0), // Invalid
            repetition_detection: None,
            eos_detection: true,
        };
        let request =
            GenerationRequest::new(session_id).with_validated_stopping_config(invalid_config);
        assert!(request.is_err());
    }

    #[test]
    fn test_generation_request_effective_max_tokens() {
        let session_id = SessionId::new();

        // Direct max_tokens should take priority
        let request = GenerationRequest::new(session_id.clone())
            .with_max_tokens(200)
            .with_stopping_config(StoppingConfig {
                max_tokens: Some(100),
                repetition_detection: None,
                eos_detection: true,
            });
        assert_eq!(request.effective_max_tokens(), Some(200));

        // Stopping config max_tokens should be used if no direct field
        let request =
            GenerationRequest::new(session_id.clone()).with_stopping_config(StoppingConfig {
                max_tokens: Some(150),
                repetition_detection: None,
                eos_detection: true,
            });
        assert_eq!(request.effective_max_tokens(), Some(150));

        // No max_tokens anywhere should return None
        let request = GenerationRequest::new(session_id);
        assert_eq!(request.effective_max_tokens(), None);
    }

    #[test]
    fn test_generation_request_migrate_max_tokens() {
        let session_id = SessionId::new();

        // Test migration from direct max_tokens to stopping_config
        let request = GenerationRequest::new(session_id.clone())
            .with_max_tokens(300)
            .migrate_max_tokens_to_stopping_config();

        assert_eq!(request.max_tokens, None);
        assert!(request.stopping_config.is_some());
        let stopping_config = request.stopping_config.unwrap();
        assert_eq!(stopping_config.max_tokens, Some(300));
        assert!(stopping_config.eos_detection);

        // Test migration when stopping_config already exists
        let existing_config = StoppingConfig {
            max_tokens: None,
            repetition_detection: Some(RepetitionConfig::default()),
            eos_detection: false,
        };
        let request = GenerationRequest::new(session_id)
            .with_max_tokens(400)
            .with_stopping_config(existing_config)
            .migrate_max_tokens_to_stopping_config();

        assert_eq!(request.max_tokens, None);
        let stopping_config = request.stopping_config.unwrap();
        assert_eq!(stopping_config.max_tokens, Some(400));
        assert!(stopping_config.repetition_detection.is_some());
        assert!(!stopping_config.eos_detection);
    }
}
