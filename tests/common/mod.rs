use llama_agent::{
    AgentConfig, MCPServerConfig, Message, MessageRole, ModelConfig, ModelSource, QueueConfig,
    Session, SessionConfig, SessionId, ToolCall, ToolCallId, ToolDefinition, ToolResult,
};
use std::path::PathBuf;
use std::time::{Duration, SystemTime};
use tempfile::TempDir;

/// Test utilities and common setup functions

pub struct TestHelper;

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

    /// Create a test MCP server configuration
    pub fn sample_mcp_config() -> MCPServerConfig {
        MCPServerConfig {
            name: "test_server".to_string(),
            command: "echo".to_string(),
            args: vec!["test".to_string()],
            timeout_secs: Some(30),
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

    /// Wait for a short duration in tests
    pub async fn short_delay() {
        tokio::time::sleep(Duration::from_millis(10)).await;
    }

    /// Wait for a medium duration in tests
    pub async fn medium_delay() {
        tokio::time::sleep(Duration::from_millis(100)).await;
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

    /// Assert that two sessions are equivalent (ignoring timestamps)
    pub fn assert_sessions_equivalent(a: &Session, b: &Session) {
        assert_eq!(a.id, b.id);
        assert_eq!(a.messages.len(), b.messages.len());
        
        for (msg_a, msg_b) in a.messages.iter().zip(b.messages.iter()) {
            assert_eq!(msg_a.role.as_str(), msg_b.role.as_str());
            assert_eq!(msg_a.content, msg_b.content);
            assert_eq!(msg_a.tool_call_id, msg_b.tool_call_id);
            assert_eq!(msg_a.tool_name, msg_b.tool_name);
        }
        
        assert_eq!(a.mcp_servers.len(), b.mcp_servers.len());
        assert_eq!(a.available_tools.len(), b.available_tools.len());
    }
}

/// Mock implementations for testing

#[derive(Clone)]
pub struct MockModel {
    pub should_fail: bool,
    pub response_text: String,
    pub response_delay: Duration,
}

impl MockModel {
    pub fn new() -> Self {
        Self {
            should_fail: false,
            response_text: "Mock model response".to_string(),
            response_delay: Duration::from_millis(10),
        }
    }

    pub fn with_failure(mut self) -> Self {
        self.should_fail = true;
        self
    }

    pub fn with_response(mut self, response: impl Into<String>) -> Self {
        self.response_text = response.into();
        self
    }

    pub fn with_delay(mut self, delay: Duration) -> Self {
        self.response_delay = delay;
        self
    }
}

impl Default for MockModel {
    fn default() -> Self {
        Self::new()
    }
}

/// Test constants
pub mod constants {
    use std::time::Duration;

    pub const TEST_TIMEOUT: Duration = Duration::from_secs(5);
    pub const SHORT_TIMEOUT: Duration = Duration::from_millis(100);
    pub const MEDIUM_TIMEOUT: Duration = Duration::from_secs(1);

    pub const SAMPLE_HUGGINGFACE_REPO: &str = "microsoft/DialoGPT-medium";
    pub const SAMPLE_MODEL_FILE: &str = "model.gguf";
    pub const SAMPLE_BF16_MODEL_FILE: &str = "model-bf16.gguf";

    pub const TEST_USER_MESSAGE: &str = "Hello, this is a test message";
    pub const TEST_SYSTEM_MESSAGE: &str = "You are a helpful assistant";
    pub const TEST_ASSISTANT_RESPONSE: &str = "Hello! I'm here to help you.";
}

/// Test assertions
pub mod assertions {
    use llama_agent::{FinishReason, GenerationResponse};
    use std::time::Duration;

    /// Assert that a generation response is valid
    pub fn assert_valid_generation_response(response: &GenerationResponse) {
        assert!(!response.generated_text.is_empty(), "Generated text should not be empty");
        assert!(response.tokens_generated > 0, "Should generate at least one token");
        assert!(response.generation_time > Duration::ZERO, "Generation time should be positive");
        
        match &response.finish_reason {
            FinishReason::Error(msg) => panic!("Generation failed with error: {}", msg),
            _ => {} // Other finish reasons are acceptable
        }
    }

    /// Assert that an error message contains expected text
    pub fn assert_error_contains(error: &dyn std::error::Error, expected: &str) {
        let error_string = format!("{}", error);
        assert!(
            error_string.contains(expected),
            "Error '{}' does not contain expected text '{}'",
            error_string,
            expected
        );
    }
}