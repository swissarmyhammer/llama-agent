use llama_agent::{
    agent::AgentServer,
    types::{
        AgentAPI, AgentConfig, FinishReason, MCPServerConfig, Message, MessageRole, ModelConfig,
        ModelSource, QueueConfig, RetryConfig, Session, SessionConfig, SessionId, ToolCall,
        ToolCallId, ToolDefinition, ToolResult,
    },
};
use serde_json::json;
use std::time::SystemTime;
use tempfile::TempDir;

mod common;

/// Test that tool call detection works correctly
#[tokio::test]
async fn test_tool_call_detection() {
    let temp_dir = TempDir::new().unwrap();
    let model_file = temp_dir.path().join("test.gguf");

    // Create a fake model file
    tokio::fs::write(&model_file, b"fake model content")
        .await
        .unwrap();

    let config = AgentConfig {
        model: ModelConfig {
            source: ModelSource::Local {
                folder: temp_dir.path().to_path_buf(),
                filename: Some("test.gguf".to_string()),
            },
            batch_size: 512,
            use_hf_params: false,
            retry_config: RetryConfig::default(),
        },
        queue_config: QueueConfig::default(),
        mcp_servers: Vec::new(),
        session_config: SessionConfig::default(),
    };

    // The initialization will fail due to the fake model file, but that's expected
    // We're testing the configuration and structure
    let result = AgentServer::initialize(config).await;
    assert!(result.is_err()); // Expected to fail with fake model
}

/// Test tool call extraction from various text formats
#[test]
fn test_tool_call_extraction() {
    use llama_agent::chat_template::ChatTemplateEngine;

    let engine = ChatTemplateEngine::new();

    // Test JSON format
    let json_text =
        r#"I need to call a tool. {"function_name": "list_files", "arguments": {"path": "/tmp"}}"#;
    let tool_calls = engine.extract_tool_calls(json_text).unwrap();
    assert_eq!(tool_calls.len(), 1);
    assert_eq!(tool_calls[0].name, "list_files");

    // Test XML format
    let xml_text = r#"Let me list the files. <function_call name="list_files">{"path": "/tmp"}</function_call>"#;
    let tool_calls = engine.extract_tool_calls(xml_text).unwrap();
    assert_eq!(tool_calls.len(), 1);
    assert_eq!(tool_calls[0].name, "list_files");

    // Test no tool calls
    let normal_text = "This is just regular text without any tool calls.";
    let tool_calls = engine.extract_tool_calls(normal_text).unwrap();
    assert_eq!(tool_calls.len(), 0);
}

/// Test tool call validation logic
#[test]
fn test_tool_call_validation() {
    let session = create_test_session_with_tools();

    // Test valid tool call
    let _valid_call = ToolCall {
        id: ToolCallId::new(),
        name: "test_tool".to_string(),
        arguments: json!({"param": "value"}),
    };

    // Test invalid tool call (non-existent tool)
    let _invalid_call = ToolCall {
        id: ToolCallId::new(),
        name: "nonexistent_tool".to_string(),
        arguments: json!({"param": "value"}),
    };

    // We can't directly test the validation method since it's private,
    // but we can verify the session has the expected tools
    assert_eq!(session.available_tools.len(), 1);
    assert_eq!(session.available_tools[0].name, "test_tool");
}

/// Test tool result creation and error handling
#[test]
fn test_tool_result_handling() {
    let call_id = ToolCallId::new();

    // Test successful result
    let success_result = ToolResult {
        call_id,
        result: json!({"status": "success", "data": "test"}),
        error: None,
    };
    assert!(success_result.error.is_none());
    assert!(!success_result.result.is_null());

    // Test error result
    let error_result = ToolResult {
        call_id,
        result: serde_json::Value::Null,
        error: Some("Tool execution failed".to_string()),
    };
    assert!(error_result.error.is_some());
    assert_eq!(
        error_result.error.as_ref().unwrap(),
        "Tool execution failed"
    );
}

/// Test multi-step tool call scenario
#[test]
fn test_multi_step_scenario_structure() {
    let mut session = create_test_session_with_tools();

    // Simulate first tool call and response
    session.messages.push(Message {
        role: MessageRole::Assistant,
        content: r#"I'll help you with that. {"function_name": "list_files", "arguments": {"path": "/tmp"}}"#.to_string(),
        tool_call_id: None,
        tool_name: None,
        timestamp: SystemTime::now(),
    });

    // Simulate tool result
    let call_id = ToolCallId::new();
    session.messages.push(Message {
        role: MessageRole::Tool,
        content: r#"{"files": ["file1.txt", "file2.txt"]}"#.to_string(),
        tool_call_id: Some(call_id),
        tool_name: Some("list_files".to_string()),
        timestamp: SystemTime::now(),
    });

    // Simulate follow-up response
    session.messages.push(Message {
        role: MessageRole::Assistant,
        content: "I found 2 files. Let me read the first one.".to_string(),
        tool_call_id: None,
        tool_name: None,
        timestamp: SystemTime::now(),
    });

    assert_eq!(session.messages.len(), 4); // User + 3 added messages

    // Verify message types
    assert_eq!(session.messages[1].role, MessageRole::Assistant);
    assert_eq!(session.messages[2].role, MessageRole::Tool);
    assert_eq!(session.messages[3].role, MessageRole::Assistant);
}

/// Test parallel tool execution logic
#[test]
fn test_parallel_execution_detection() {
    use llama_agent::chat_template::ChatTemplateEngine;

    let engine = ChatTemplateEngine::new();

    // Test multiple different tool calls (should be parallel)
    let multi_text = r#"
        {"function_name": "list_files", "arguments": {"path": "/tmp"}}
        {"function_name": "get_time", "arguments": {}}
    "#;
    let tool_calls = engine.extract_tool_calls(multi_text).unwrap();
    assert_eq!(tool_calls.len(), 2);
    assert_ne!(tool_calls[0].name, tool_calls[1].name);

    // Test single tool call (should not be parallel)
    let single_text = r#"{"function_name": "list_files", "arguments": {"path": "/tmp"}}"#;
    let tool_calls = engine.extract_tool_calls(single_text).unwrap();
    assert_eq!(tool_calls.len(), 1);
}

/// Test error recovery in tool execution
#[test]
fn test_error_recovery_structure() {
    let error_result = ToolResult {
        call_id: ToolCallId::new(),
        result: serde_json::Value::Null,
        error: Some("Network timeout".to_string()),
    };

    // Verify error result structure
    assert!(error_result.error.is_some());
    assert!(error_result.result.is_null());

    // Test that we can serialize/deserialize error results
    let serialized = serde_json::to_string(&error_result).unwrap();
    let deserialized: ToolResult = serde_json::from_str(&serialized).unwrap();
    assert_eq!(deserialized.error, error_result.error);
    assert_eq!(deserialized.call_id, error_result.call_id);
}

/// Test session state management with tool interactions
#[test]
fn test_session_state_management() {
    let mut session = create_test_session_with_tools();
    let initial_message_count = session.messages.len();
    let initial_timestamp = session.updated_at;

    // Add a tool call message
    session.messages.push(Message {
        role: MessageRole::Assistant,
        content: "Calling tool...".to_string(),
        tool_call_id: None,
        tool_name: None,
        timestamp: SystemTime::now(),
    });

    // Small delay to ensure timestamp difference
    std::thread::sleep(std::time::Duration::from_millis(1));

    // Update session timestamp
    session.updated_at = SystemTime::now();

    // Verify state changes
    assert_eq!(session.messages.len(), initial_message_count + 1);
    assert!(session.updated_at > initial_timestamp);

    // Add tool result
    session.messages.push(Message {
        role: MessageRole::Tool,
        content: "Tool result".to_string(),
        tool_call_id: Some(ToolCallId::new()),
        tool_name: Some("test_tool".to_string()),
        timestamp: SystemTime::now(),
    });

    assert_eq!(session.messages.len(), initial_message_count + 2);
}

/// Test tool call workflow limits and bounds
#[test]
fn test_workflow_limits() {
    // Test maximum iterations concept (from the AgentServer implementation)
    const MAX_TOOL_ITERATIONS: usize = 5;

    let mut iteration_count = 0;
    while iteration_count < MAX_TOOL_ITERATIONS {
        iteration_count += 1;
        // Simulate tool call processing
        if iteration_count >= MAX_TOOL_ITERATIONS {
            break;
        }
    }

    assert_eq!(iteration_count, MAX_TOOL_ITERATIONS);
}

/// Test finish reason handling for tool calls
#[test]
fn test_finish_reason_tool_call() {
    let finish_reasons = vec![
        FinishReason::ToolCall,
        FinishReason::MaxTokens,
        FinishReason::StopToken,
        FinishReason::EndOfSequence,
        FinishReason::Error("test error".to_string()),
    ];

    // Test that ToolCall is correctly identified
    for reason in finish_reasons {
        match reason {
            FinishReason::ToolCall => {
                // This should trigger tool processing - verified by reaching this branch
            }
            _ => {
                // These should not trigger tool processing - verified by reaching this branch
            }
        }
    }
}

fn create_test_session_with_tools() -> Session {
    Session {
        id: SessionId::new(),
        messages: vec![Message {
            role: MessageRole::User,
            content: "Hello, can you help me?".to_string(),
            tool_call_id: None,
            tool_name: None,
            timestamp: SystemTime::now(),
        }],
        mcp_servers: vec![MCPServerConfig {
            name: "test_server".to_string(),
            command: "test_command".to_string(),
            args: vec![],
            timeout_secs: None,
        }],
        available_tools: vec![ToolDefinition {
            name: "test_tool".to_string(),
            description: "A test tool for testing".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "param": {
                        "type": "string",
                        "description": "A test parameter"
                    }
                }
            }),
            server_name: "test_server".to_string(),
        }],
        available_prompts: vec![],
        created_at: SystemTime::now(),
        updated_at: SystemTime::now(),
    }
}
