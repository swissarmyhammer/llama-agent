use llama_agent::{
    AgentConfig, Message, MessageRole, ModelConfig, ModelSource,
    QueueConfig, Session, SessionConfig, SessionId, ToolCall, ToolCallId, ToolDefinition,
    ToolResult,
};
use std::path::PathBuf;
use std::time::{Duration, SystemTime};
use tempfile::TempDir;

/// Test utilities and common setup functions
#[allow(dead_code)]
pub struct TestHelper;

#[allow(dead_code)]
impl TestHelper {
    /// Create a minimal test configuration for testing
    pub fn minimal_config() -> AgentConfig {
        AgentConfig {
            model: ModelConfig {
                source: ModelSource::Local {
                    folder: PathBuf::from("/tmp"),
                    filename: Some("test.gguf".to_string()),
                },
                batch_size: 128,
                use_hf_params: false,
            },
            queue_config: QueueConfig {
                max_queue_size: 10,
                request_timeout: Duration::from_secs(5),
                worker_threads: 1,
            },
            mcp_servers: vec![],
            session_config: SessionConfig {
                max_sessions: 10,
                session_timeout: Duration::from_secs(300), // 5 minutes for tests
            },
        }
    }

    /// Create a test session with some sample messages
    pub fn sample_session() -> Session {
        let session_id = SessionId::new();
        let now = SystemTime::now();

        Session {
            id: session_id,
            messages: vec![
                Message {
                    role: MessageRole::System,
                    content: "You are a helpful assistant.".to_string(),
                    tool_call_id: None,
                    tool_name: None,
                    timestamp: now,
                },
                Message {
                    role: MessageRole::User,
                    content: "Hello, how are you?".to_string(),
                    tool_call_id: None,
                    tool_name: None,
                    timestamp: now,
                },
                Message {
                    role: MessageRole::Assistant,
                    content: "I'm doing well, thank you! How can I help you today?".to_string(),
                    tool_call_id: None,
                    tool_name: None,
                    timestamp: now,
                },
            ],
            mcp_servers: vec![],
            available_tools: vec![],
            available_prompts: vec![],
            created_at: now,
            updated_at: now,
        }
    }

    /// Create a test tool definition
    pub fn sample_tool_definition() -> ToolDefinition {
        ToolDefinition {
            name: "test_tool".to_string(),
            description: "A test tool for validation".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "input": {
                        "type": "string",
                        "description": "Test input parameter"
                    }
                },
                "required": ["input"]
            }),
            server_name: "test_server".to_string(),
        }
    }

    /// Create a test tool call
    pub fn sample_tool_call() -> ToolCall {
        ToolCall {
            id: ToolCallId::new(),
            name: "test_tool".to_string(),
            arguments: serde_json::json!({
                "input": "test value"
            }),
        }
    }

    /// Create a test tool result
    pub fn sample_tool_result(call_id: ToolCallId) -> ToolResult {
        ToolResult {
            call_id,
            result: serde_json::json!({
                "status": "success",
                "output": "test result"
            }),
            error: None,
        }
    }

    /// Create a temporary directory for test files
    pub fn temp_dir() -> TempDir {
        tempfile::tempdir().expect("Failed to create temp directory")
    }

    /// Create a test model file (empty but with correct extension)
    pub fn create_test_model_file(dir: &TempDir, filename: &str) -> PathBuf {
        let model_path = dir.path().join(filename);
        std::fs::write(&model_path, b"fake model data").expect("Failed to create test model file");
        model_path
    }

    /// Create a test configuration with local model
    pub fn config_with_local_model(temp_dir: &TempDir, filename: &str) -> AgentConfig {
        let _model_path = Self::create_test_model_file(temp_dir, filename);

        AgentConfig {
            model: ModelConfig {
                source: ModelSource::Local {
                    folder: temp_dir.path().to_path_buf(),
                    filename: Some(filename.to_string()),
                },
                batch_size: 128,
                use_hf_params: false,
            },
            queue_config: QueueConfig {
                max_queue_size: 5,
                request_timeout: Duration::from_secs(2),
                worker_threads: 1,
            },
            mcp_servers: vec![],
            session_config: SessionConfig {
                max_sessions: 5,
                session_timeout: Duration::from_secs(60),
            },
        }
    }
}
