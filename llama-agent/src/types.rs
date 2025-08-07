use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::{Duration, SystemTime};
use thiserror::Error;
use async_trait::async_trait;
use futures::Stream;
use std::pin::Pin;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: MessageRole,
    pub content: String,
    pub tool_call_id: Option<String>,
    pub tool_name: Option<String>,
    pub timestamp: SystemTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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
    pub id: String,
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
    pub id: String,
    pub name: String,
    pub arguments: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    pub call_id: String,
    pub result: serde_json::Value,
    pub error: Option<String>,
}

#[derive(Debug)]
pub struct StreamChunk {
    pub text: String,
    pub is_complete: bool,
    pub token_count: u32,
}

#[derive(Debug, Clone)]
pub struct AgentConfig {
    pub model: ModelConfig,
    pub queue_config: QueueConfig,
    pub mcp_servers: Vec<MCPServerConfig>,
    pub session_config: SessionConfig,
}

#[derive(Debug, Clone)]
pub struct ModelConfig {
    pub source: ModelSource,
    pub batch_size: u32,
    pub use_hf_params: bool,
}

#[derive(Debug, Clone)]
pub enum ModelSource {
    HuggingFace { repo: String, filename: Option<String> },
    Local { folder: PathBuf, filename: Option<String> },
}

#[derive(Debug, Clone)]
pub struct QueueConfig {
    pub max_queue_size: usize,
    pub request_timeout: Duration,
    pub worker_threads: usize,
}

#[derive(Debug, Clone)]
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
    
    #[error("Timeout: request took longer than {timeout:?}")]
    Timeout { timeout: Duration },
    
    #[error("Queue full: maximum capacity {capacity} exceeded")]
    QueueFull { capacity: usize },
}

#[derive(Debug, Error)]
pub enum ModelError {
    #[error("Model loading failed: {0}")]
    LoadingFailed(String),
    
    #[error("Model not found at source: {0}")]
    NotFound(String),
    
    #[error("Invalid model configuration: {0}")]
    InvalidConfig(String),
    
    #[error("Model inference failed: {0}")]
    InferenceFailed(String),
}

#[derive(Debug, Error)]
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
        request: GenerationRequest
    ) -> Result<Pin<Box<dyn Stream<Item = Result<StreamChunk, AgentError>> + Send>>, AgentError>;
    
    async fn create_session(&self) -> Result<Session, AgentError>;
    
    async fn get_session(&self, session_id: &str) -> Result<Option<Session>, AgentError>;
    
    async fn update_session(&self, session: Session) -> Result<(), AgentError>;
    
    async fn discover_tools(&self, session: &mut Session) -> Result<(), AgentError>;
    
    async fn execute_tool(&self, tool_call: ToolCall, session: &Session) -> Result<ToolResult, AgentError>;
    
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
            id: "test-session".to_string(),
            messages: Vec::new(),
            mcp_servers: Vec::new(),
            available_tools: Vec::new(),
            created_at: SystemTime::now(),
            updated_at: SystemTime::now(),
        };
        
        assert_eq!(session.id, "test-session");
        assert!(session.messages.is_empty());
        assert!(session.mcp_servers.is_empty());
        assert!(session.available_tools.is_empty());
    }
    
    #[test]
    fn test_mcp_server_config() {
        let config = MCPServerConfig {
            name: "filesystem".to_string(),
            command: "npx".to_string(),
            args: vec!["-y".to_string(), "@modelcontextprotocol/server-filesystem".to_string()],
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
            id: "test".to_string(),
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
            ModelSource::Local { filename, .. } => assert_eq!(filename, Some("model.gguf".to_string())),
            _ => panic!("Wrong variant"),
        }
    }
    
    #[test]
    fn test_finish_reason() {
        let reasons = vec![
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
            id: "call_123".to_string(),
            name: "list_files".to_string(),
            arguments: serde_json::json!({"path": "/tmp"}),
        };
        
        let serialized = serde_json::to_string(&tool_call).unwrap();
        let deserialized: ToolCall = serde_json::from_str(&serialized).unwrap();
        
        assert_eq!(deserialized.id, "call_123");
        assert_eq!(deserialized.name, "list_files");
    }
}
