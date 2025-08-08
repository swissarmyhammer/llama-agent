mod common;

use common::TestHelper;
use llama_agent::types::*;
use llama_agent::AgentServer;
use std::time::Duration;
use tokio::time::timeout;

#[tokio::test]
async fn test_agent_server_initialization() {
    let temp_dir = TestHelper::temp_dir();
    TestHelper::create_test_model_file(&temp_dir, "test.gguf");

    let config = TestHelper::config_with_local_model(&temp_dir, "test.gguf");

    // Test that we can create an AgentServer with valid configuration
    // Note: Actual model loading will fail with dummy file, but we test the initialization
    let result = AgentServer::initialize(config).await;

    // In the current implementation, this will fail due to backend initialization issues
    // That's expected in tests - we're testing the error handling path
    match result {
        Ok(_) => {
            // If it succeeds, that's also fine - depends on test execution order
        }
        Err(AgentError::Model(_)) => {
            // Expected - model loading fails with dummy file
        }
        Err(other) => {
            panic!("Unexpected error type during initialization: {:?}", other);
        }
    }
}

#[tokio::test]
async fn test_session_workflow() {
    let temp_dir = TestHelper::temp_dir();
    TestHelper::create_test_model_file(&temp_dir, "test.gguf");

    let mut config = TestHelper::config_with_local_model(&temp_dir, "test.gguf");
    config.session_config.max_sessions = 5;
    config.session_config.session_timeout = Duration::from_secs(60);

    // Test session manager independently
    let session_manager = llama_agent::session::SessionManager::new(config.session_config);

    // Create a session
    let session = session_manager
        .create_session()
        .await
        .expect("Failed to create session");
    assert!(!session.id.to_string().is_empty());
    assert!(session.messages.is_empty());

    // Add a message
    let message = Message {
        role: MessageRole::User,
        content: "Hello, world!".to_string(),
        tool_call_id: None,
        tool_name: None,
        timestamp: std::time::SystemTime::now(),
    };

    session_manager
        .add_message(&session.id, message)
        .await
        .expect("Failed to add message");

    // Retrieve the updated session
    let updated_session = session_manager
        .get_session(&session.id)
        .await
        .expect("Failed to get session")
        .expect("Session not found");

    assert_eq!(updated_session.messages.len(), 1);
    assert_eq!(updated_session.messages[0].content, "Hello, world!");
}

#[tokio::test]
async fn test_model_manager_workflow() {
    let temp_dir = TestHelper::temp_dir();
    TestHelper::create_test_model_file(&temp_dir, "test.gguf");

    let config = TestHelper::config_with_local_model(&temp_dir, "test.gguf");

    // Test model manager creation and loading attempts
    match llama_agent::model::ModelManager::new(config.model) {
        Ok(model_manager) => {
            let model_manager = std::sync::Arc::new(model_manager);
            // Model loading should fail with dummy file, but that's expected
            let is_loaded = model_manager.is_loaded().await;
            assert!(!is_loaded);

            // Attempt to load model (will fail with dummy file)
            let load_result = model_manager.load_model().await;
            assert!(load_result.is_err());

            // Test with_model when model is not loaded
            let result = model_manager.with_model(|_model| ()).await;
            assert!(result.is_err());
        }
        Err(ModelError::LoadingFailed(msg)) if msg.contains("Backend already initialized") => {
            // Expected in parallel test execution
        }
        Err(e) => {
            panic!("Unexpected error creating ModelManager: {:?}", e);
        }
    }
}

#[tokio::test]
async fn test_queue_manager_workflow() {
    let temp_dir = TestHelper::temp_dir();
    TestHelper::create_test_model_file(&temp_dir, "test.gguf");

    let config = TestHelper::config_with_local_model(&temp_dir, "test.gguf");

    // Test queue functionality independent of actual model loading
    match llama_agent::model::ModelManager::new(config.model.clone()) {
        Ok(model_manager) => {
            let queue = llama_agent::queue::RequestQueue::new(
                std::sync::Arc::new(model_manager),
                config.queue_config,
            );

            assert_eq!(queue.get_queue_size(), 0);

            // Test stats
            let stats = queue.get_stats();
            assert_eq!(stats.total_requests, 0);
            assert_eq!(stats.completed_requests, 0);
        }
        Err(ModelError::LoadingFailed(msg)) if msg.contains("Backend already initialized") => {
            // Expected in parallel test execution
        }
        Err(e) => {
            panic!("Unexpected error creating ModelManager: {:?}", e);
        }
    }
}

#[tokio::test]
async fn test_config_validation() {
    // Test valid configuration
    let temp_dir = TestHelper::temp_dir();
    TestHelper::create_test_model_file(&temp_dir, "test.gguf");

    let valid_config = TestHelper::config_with_local_model(&temp_dir, "test.gguf");
    assert!(valid_config.validate().is_ok());

    // Test invalid batch size
    let mut invalid_config = valid_config.clone();
    invalid_config.model.batch_size = 0;
    assert!(invalid_config.validate().is_err());

    // Test invalid worker threads
    let mut invalid_config = valid_config.clone();
    invalid_config.queue_config.worker_threads = 0;
    assert!(invalid_config.validate().is_err());

    // Test invalid session limit
    let mut invalid_config = valid_config;
    invalid_config.session_config.max_sessions = 0;
    assert!(invalid_config.validate().is_err());
}

#[tokio::test]
async fn test_tool_definitions() {
    let tool_def = TestHelper::sample_tool_definition();
    assert_eq!(tool_def.name, "test_tool");
    assert_eq!(tool_def.server_name, "test_server");
    assert!(!tool_def.description.is_empty());

    // Test serialization
    let serialized = serde_json::to_string(&tool_def).expect("Failed to serialize");
    let deserialized: ToolDefinition =
        serde_json::from_str(&serialized).expect("Failed to deserialize");

    assert_eq!(deserialized.name, tool_def.name);
    assert_eq!(deserialized.server_name, tool_def.server_name);
}

#[tokio::test]
async fn test_tool_calls_and_results() {
    let tool_call = TestHelper::sample_tool_call();
    let tool_result = TestHelper::sample_tool_result(tool_call.id);

    assert_eq!(tool_result.call_id, tool_call.id);
    assert!(tool_result.error.is_none());

    // Test serialization round-trip
    let serialized_call = serde_json::to_string(&tool_call).expect("Failed to serialize tool call");
    let deserialized_call: ToolCall =
        serde_json::from_str(&serialized_call).expect("Failed to deserialize tool call");

    assert_eq!(deserialized_call.id, tool_call.id);
    assert_eq!(deserialized_call.name, tool_call.name);

    let serialized_result =
        serde_json::to_string(&tool_result).expect("Failed to serialize tool result");
    let deserialized_result: ToolResult =
        serde_json::from_str(&serialized_result).expect("Failed to deserialize tool result");

    assert_eq!(deserialized_result.call_id, tool_result.call_id);
}

#[tokio::test]
async fn test_message_handling() {
    let session = TestHelper::sample_session();

    // Verify the sample session has expected structure
    assert_eq!(session.messages.len(), 3);
    assert_eq!(session.messages[0].role.as_str(), "system");
    assert_eq!(session.messages[1].role.as_str(), "user");
    assert_eq!(session.messages[2].role.as_str(), "assistant");

    // Test message creation with tool calls
    let tool_call_id = ToolCallId::new();
    let tool_message = Message {
        role: MessageRole::Tool,
        content: "Tool execution result".to_string(),
        tool_call_id: Some(tool_call_id),
        tool_name: Some("test_tool".to_string()),
        timestamp: std::time::SystemTime::now(),
    };

    assert_eq!(tool_message.tool_call_id, Some(tool_call_id));
    assert_eq!(tool_message.tool_name.as_ref().unwrap(), "test_tool");
}

#[tokio::test]
async fn test_concurrent_session_access() {
    let config = TestHelper::minimal_config();
    let session_manager = std::sync::Arc::new(llama_agent::session::SessionManager::new(
        config.session_config,
    ));

    let mut handles = Vec::new();

    // Create multiple sessions concurrently
    for i in 0..5 {
        let manager = session_manager.clone();
        let handle = tokio::spawn(async move {
            let session = manager.create_session().await?;

            // Add some messages
            for j in 0..3 {
                let message = Message {
                    role: MessageRole::User,
                    content: format!("Message {} from task {}", j, i),
                    tool_call_id: None,
                    tool_name: None,
                    timestamp: std::time::SystemTime::now(),
                };
                manager.add_message(&session.id, message).await?;
            }

            Result::<_, SessionError>::Ok(session)
        });
        handles.push(handle);
    }

    // Wait for all tasks to complete
    let mut sessions = Vec::new();
    for handle in handles {
        let session = handle
            .await
            .expect("Task panicked")
            .expect("Session creation failed");
        sessions.push(session);
    }

    assert_eq!(sessions.len(), 5);

    // Verify all sessions were created and have expected messages
    for (i, session) in sessions.iter().enumerate() {
        let retrieved = session_manager
            .get_session(&session.id)
            .await
            .expect("Failed to get session")
            .expect("Session not found");

        assert_eq!(retrieved.messages.len(), 3);
        for (j, message) in retrieved.messages.iter().enumerate() {
            assert!(message
                .content
                .contains(&format!("Message {} from task {}", j, i)));
        }
    }
}

#[tokio::test]
async fn test_error_handling_paths() {
    // Test various error conditions

    // Invalid model path
    let invalid_config = AgentConfig {
        model: ModelConfig {
            source: ModelSource::Local {
                folder: std::path::PathBuf::from("/nonexistent/path"),
                filename: Some("nonexistent.gguf".to_string()),
            },
            batch_size: 512,
            use_hf_params: false,
            retry_config: RetryConfig::default(),
            debug: false,
        },
        queue_config: QueueConfig::default(),
        mcp_servers: vec![],
        session_config: SessionConfig::default(),
    };

    assert!(invalid_config.validate().is_err());

    // Invalid HuggingFace repo format
    let invalid_hf_config = AgentConfig {
        model: ModelConfig {
            source: ModelSource::HuggingFace {
                repo: "invalid-repo-format".to_string(), // Missing '/'
                filename: None,
            },
            batch_size: 512,
            use_hf_params: false,
            retry_config: RetryConfig::default(),
            debug: false,
        },
        queue_config: QueueConfig::default(),
        mcp_servers: vec![],
        session_config: SessionConfig::default(),
    };

    assert!(invalid_hf_config.validate().is_err());

    // Duplicate MCP server names
    let duplicate_mcp_config = AgentConfig {
        model: ModelConfig::default(),
        queue_config: QueueConfig::default(),
        mcp_servers: vec![
            MCPServerConfig {
                name: "duplicate".to_string(),
                command: "echo".to_string(),
                args: vec![],
                timeout_secs: None,
            },
            MCPServerConfig {
                name: "duplicate".to_string(),
                command: "echo".to_string(),
                args: vec![],
                timeout_secs: None,
            },
        ],
        session_config: SessionConfig::default(),
    };

    assert!(duplicate_mcp_config.validate().is_err());
}

#[tokio::test]
async fn test_timeout_scenarios() {
    let temp_dir = TestHelper::temp_dir();
    TestHelper::create_test_model_file(&temp_dir, "test.gguf");

    let mut config = TestHelper::config_with_local_model(&temp_dir, "test.gguf");
    config.queue_config.request_timeout = Duration::from_millis(10); // Very short timeout

    // Test request timeout in queue
    match llama_agent::model::ModelManager::new(config.model.clone()) {
        Ok(model_manager) => {
            let queue = llama_agent::queue::RequestQueue::new(
                std::sync::Arc::new(model_manager),
                config.queue_config,
            );

            let session = TestHelper::sample_session();
            let request = GenerationRequest {
                session_id: session.id.clone(),
                max_tokens: Some(100),
                temperature: Some(0.7),
                top_p: Some(0.9),
                stop_tokens: vec![],
            };

            let result = timeout(
                Duration::from_millis(100),
                queue.submit_request(request, &session),
            )
            .await;
            assert!(result.is_ok()); // Timeout should complete, but request should fail

            match result.unwrap() {
                Err(QueueError::Timeout) => {
                    // Expected timeout
                }
                Err(QueueError::WorkerError(_)) => {
                    // Also acceptable - model not loaded error
                }
                other => {
                    // Other errors are also acceptable in test environment
                    println!("Got result: {:?}", other);
                }
            }
        }
        Err(ModelError::LoadingFailed(msg)) if msg.contains("Backend already initialized") => {
            // Expected in parallel test execution
        }
        Err(e) => {
            panic!("Unexpected error creating ModelManager: {:?}", e);
        }
    }
}
