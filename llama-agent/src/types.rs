use async_trait::async_trait;
use futures::Stream;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::pin::Pin;
use std::time::{Duration, SystemTime};
use thiserror::Error;
use ulid::Ulid;

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

#[derive(Debug)]
pub struct GenerationRequest {
    pub session: Session,
    pub max_tokens: Option<u32>,
    pub temperature: Option<f32>,
    pub top_p: Option<f32>,
    pub stop_tokens: Vec<String>,
}

#[derive(Debug)]
pub struct GenerationResponse {
    pub generated_text: String,
    pub tokens_generated: u32,
    pub generation_time: Duration,
    pub finish_reason: FinishReason,
}

#[derive(Debug, PartialEq)]
pub enum FinishReason {
    MaxTokens,
    StopToken,
    EndOfSequence,
    ToolCall,
    Error(String),
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
pub struct ModelConfig {
    pub source: ModelSource,
    pub batch_size: u32,
    pub use_hf_params: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ModelSource {
    HuggingFace {
        repo: String,
        filename: Option<String>,
    },
    Local {
        folder: PathBuf,
        filename: Option<String>,
    },
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

impl Default for ModelConfig {
    fn default() -> Self {
        Self {
            source: ModelSource::HuggingFace {
                repo: "microsoft/DialoGPT-medium".to_string(),
                filename: None,
            },
            batch_size: 512,
            use_hf_params: true,
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

impl ModelConfig {
    pub fn validate(&self) -> Result<(), ModelError> {
        self.source.validate()?;

        if self.batch_size == 0 {
            return Err(ModelError::InvalidConfig(
                "Batch size must be greater than 0".to_string(),
            ));
        }

        if self.batch_size > 8192 {
            return Err(ModelError::InvalidConfig(
                "Batch size should not exceed 8192 for most models".to_string(),
            ));
        }

        Ok(())
    }
}

impl ModelSource {
    pub fn validate(&self) -> Result<(), ModelError> {
        match self {
            ModelSource::HuggingFace { repo, filename } => {
                if repo.is_empty() {
                    return Err(ModelError::InvalidConfig(
                        "HuggingFace repo name cannot be empty".to_string(),
                    ));
                }

                // Validate repo format (should contain at least one '/')
                if !repo.contains('/') {
                    return Err(ModelError::InvalidConfig(
                        "HuggingFace repo must be in format 'org/repo'".to_string(),
                    ));
                }

                // Check for invalid characters
                if repo
                    .chars()
                    .any(|c| !c.is_alphanumeric() && !"-_./".contains(c))
                {
                    return Err(ModelError::InvalidConfig(
                        "Invalid characters in HuggingFace repo name".to_string(),
                    ));
                }

                if let Some(f) = filename {
                    if f.is_empty() {
                        return Err(ModelError::InvalidConfig(
                            "Filename cannot be empty".to_string(),
                        ));
                    }
                    if !f.ends_with(".gguf") {
                        return Err(ModelError::InvalidConfig(
                            "Model file must have .gguf extension".to_string(),
                        ));
                    }
                }

                Ok(())
            }
            ModelSource::Local { folder, filename } => {
                if !folder.exists() {
                    return Err(ModelError::NotFound(format!(
                        "Local folder does not exist: {}",
                        folder.display()
                    )));
                }

                if !folder.is_dir() {
                    return Err(ModelError::InvalidConfig(format!(
                        "Path is not a directory: {}",
                        folder.display()
                    )));
                }

                if let Some(f) = filename {
                    if f.is_empty() {
                        return Err(ModelError::InvalidConfig(
                            "Filename cannot be empty".to_string(),
                        ));
                    }
                    if !f.ends_with(".gguf") {
                        return Err(ModelError::InvalidConfig(
                            "Model file must have .gguf extension".to_string(),
                        ));
                    }

                    let full_path = folder.join(f);
                    if !full_path.exists() {
                        return Err(ModelError::NotFound(format!(
                            "Model file does not exist: {}",
                            full_path.display()
                        )));
                    }

                    if !full_path.is_file() {
                        return Err(ModelError::InvalidConfig(format!(
                            "Path is not a file: {}",
                            full_path.display()
                        )));
                    }
                }

                Ok(())
            }
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
    #[error("Model error: {0}")]
    Model(#[from] ModelError),

    #[error("Queue error: {0}")]
    Queue(#[from] QueueError),

    #[error("Session error: {0}")]
    Session(#[from] SessionError),

    #[error("MCP error: {0}")]
    MCP(#[from] MCPError),

    #[error("Template error: {0}")]
    Template(#[from] TemplateError),

    #[error("Request timeout: Operation took longer than {timeout:?}. Try reducing the request complexity or increasing the timeout value.")]
    Timeout { timeout: Duration },

    #[error("Request queue full: Maximum capacity of {capacity} requests exceeded. Please wait for pending requests to complete or increase queue capacity.")]
    QueueFull { capacity: usize },
}

#[derive(Debug, Error)]
pub enum ModelError {
    #[error("Failed to load model: {0}. \n\nTroubleshooting steps:\n• Verify model format is .gguf (GGML Unified Format)\n• Check available system memory (models require 4-16GB typically)\n• Ensure model file is not corrupted (re-download if needed)\n• Try reducing batch size or context length in configuration")]
    LoadingFailed(String),

    #[error("Model not found: {0}. \n\nPlease check:\n• Model file path exists and is readable\n• Filename matches exactly (case-sensitive)\n• File permissions allow read access\n• For HuggingFace repos: verify repo name and model file exists")]
    NotFound(String),

    #[error("Invalid model configuration: {0}. \n\nConfiguration requirements:\n• batch_size must be > 0 (recommended: 512-2048)\n• Model path must be absolute or relative to current directory\n• File extension must be .gguf\n• HuggingFace repo format: 'username/repo-name'")]
    InvalidConfig(String),

    #[error("Model inference failed: {0}. \n\nPossible causes:\n• Insufficient system memory or GPU memory\n• Model format incompatible with current version\n• Context length exceeds model's maximum\n• Hardware acceleration (Metal/CUDA) unavailable")]
    InferenceFailed(String),
}

#[derive(Debug, Clone, Error)]
pub enum QueueError {
    #[error("Request queue is full (all {capacity} slots occupied). \n\nOptions:\n• Wait a few seconds and retry\n• Increase max_queue_size in configuration\n• Reduce concurrent request load\n• Check if requests are processing normally (use health check)")]
    Full { capacity: usize },

    #[error("Request timeout after {duration:?}. \n\nSuggestions:\n• Reduce max_tokens in the request\n• Simplify the prompt or conversation context\n• Increase request_timeout in queue configuration\n• Check system resources (CPU/memory usage)")]
    Timeout { duration: Duration },

    #[error("Processing error: {0}. \n\nDebugging steps:\n• Check detailed logs for stack trace\n• Verify model is properly loaded and accessible\n• Ensure sufficient system resources\n• Try with a simpler request to isolate the issue")]
    WorkerError(String),
}

#[derive(Debug, Error)]
pub enum SessionError {
    #[error("Session not found: {0}. The session may have expired or been removed. Create a new session to continue.")]
    NotFound(String),

    #[error("Session limit exceeded. Close unused sessions or increase the maximum session limit in configuration.")]
    LimitExceeded,

    #[error("Session timed out due to inactivity. Create a new session to continue.")]
    Timeout,

    #[error("Invalid session state: {0}. This may indicate corrupted session data.")]
    InvalidState(String),
}

#[derive(Debug, Error)]
pub enum MCPError {
    #[error("MCP server '{0}' not found. Check server configuration and ensure it's properly initialized.")]
    ServerNotFound(String),

    #[error("Tool execution failed: {0}. Verify tool arguments and ensure the MCP server is running properly.")]
    ToolCallFailed(String),

    #[error("MCP server connection error: {0}. Check server status and network connectivity.")]
    Connection(String),

    #[error("MCP protocol error: {0}. This may indicate incompatible server version or malformed request.")]
    Protocol(String),
}

#[derive(Debug, Error)]
pub enum TemplateError {
    #[error("Template rendering failed: {0}. Check template syntax and provided variables.")]
    RenderingFailed(String),

    #[error("Failed to parse tool calls: {0}. Check the format of tool call requests in the generated text.")]
    ToolCallParsing(String),

    #[error("Invalid template format: {0}. Verify template syntax is correct.")]
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

    async fn update_session(&self, session: Session) -> Result<(), AgentError>;

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
            created_at: SystemTime::now(),
            updated_at: SystemTime::now(),
        };

        assert!(!session.id.to_string().is_empty());
        assert!(session.messages.is_empty());
        assert!(session.mcp_servers.is_empty());
        assert!(session.available_tools.is_empty());
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
            created_at: SystemTime::now(),
            updated_at: SystemTime::now(),
        };

        let request = GenerationRequest {
            session,
            max_tokens: Some(100),
            temperature: Some(0.7),
            top_p: Some(0.9),
            stop_tokens: vec!["</s>".to_string()],
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
            FinishReason::MaxTokens,
            FinishReason::StopToken,
            FinishReason::EndOfSequence,
            FinishReason::ToolCall,
            FinishReason::Error("test error".to_string()),
        ];

        assert_eq!(reasons.len(), 5);

        match &reasons[4] {
            FinishReason::Error(msg) => assert_eq!(msg, "test error"),
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
        };

        assert!(config.validate().is_err());

        let config = ModelConfig {
            source: ModelSource::HuggingFace {
                repo: "microsoft/DialoGPT-medium".to_string(),
                filename: None,
            },
            batch_size: 10000,
            use_hf_params: true,
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
}
