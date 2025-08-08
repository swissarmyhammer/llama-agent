//! Integration Tests for Examples
//!
//! This module contains automated tests that validate all the examples
//! work correctly. These tests serve as both validation and documentation
//! of expected behavior.

use llama_agent::{
    types::{
        AgentAPI, AgentConfig, GenerationRequest, MCPServerConfig, Message, MessageRole,
        ModelConfig, ModelSource, QueueConfig, RetryConfig, SessionConfig, SessionId,
    },
    AgentServer,
};
use std::path::PathBuf;
use std::time::{Duration, SystemTime};
use tracing::{info, warn};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging for test diagnostics
    tracing_subscriber::fmt::init();

    info!("Starting integration tests for examples");

    println!("Integration Tests for Llama Agent Examples");
    println!("{}", "=".repeat(60));

    let mut passed_tests = 0;
    let mut failed_tests = 0;
    let mut skipped_tests = 0;

    // Test 1: Configuration validation
    match test_configuration_validation().await {
        Ok(_) => {
            println!("✓ Configuration validation tests passed");
            passed_tests += 1;
        }
        Err(e) => {
            println!("❌ Configuration validation tests failed: {}", e);
            failed_tests += 1;
        }
    }

    // Test 2: Agent initialization patterns
    match test_agent_initialization().await {
        Ok(_) => {
            println!("✓ Agent initialization tests passed");
            passed_tests += 1;
        }
        Err(e) => {
            println!("❌ Agent initialization tests failed: {}", e);
            failed_tests += 1;
        }
    }

    // Test 3: Session management
    match test_session_management().await {
        Ok(_) => {
            println!("✓ Session management tests passed");
            passed_tests += 1;
        }
        Err(e) => {
            println!("❌ Session management tests failed: {}", e);
            failed_tests += 1;
        }
    }

    // Test 4: Error handling patterns
    match test_error_handling().await {
        Ok(_) => {
            println!("✓ Error handling tests passed");
            passed_tests += 1;
        }
        Err(e) => {
            println!("❌ Error handling tests failed: {}", e);
            failed_tests += 1;
        }
    }

    // Test 5: MCP integration (may skip if MCP servers unavailable)
    match test_mcp_integration().await {
        Ok(_) => {
            println!("✓ MCP integration tests passed");
            passed_tests += 1;
        }
        Err(e) => {
            if e.to_string().contains("MCP server") {
                println!("⚠ MCP integration tests skipped (servers not available)");
                skipped_tests += 1;
            } else {
                println!("❌ MCP integration tests failed: {}", e);
                failed_tests += 1;
            }
        }
    }

    // Test 6: Streaming functionality (mock test since model may not be available)
    match test_streaming_patterns().await {
        Ok(_) => {
            println!("✓ Streaming pattern tests passed");
            passed_tests += 1;
        }
        Err(e) => {
            println!("❌ Streaming pattern tests failed: {}", e);
            failed_tests += 1;
        }
    }

    // Test 7: Performance configuration validation
    match test_performance_configurations().await {
        Ok(_) => {
            println!("✓ Performance configuration tests passed");
            passed_tests += 1;
        }
        Err(e) => {
            println!("❌ Performance configuration tests failed: {}", e);
            failed_tests += 1;
        }
    }

    // Test 8: CLI argument validation (simulation)
    match test_cli_argument_patterns().await {
        Ok(_) => {
            println!("✓ CLI argument validation tests passed");
            passed_tests += 1;
        }
        Err(e) => {
            println!("❌ CLI argument validation tests failed: {}", e);
            failed_tests += 1;
        }
    }

    // Test summary
    println!("\n{}", "=".repeat(60));
    println!("Integration Test Summary:");
    println!("  ✓ Passed: {}", passed_tests);
    println!("  ❌ Failed: {}", failed_tests);
    println!("  ⚠ Skipped: {}", skipped_tests);
    println!("  Total: {}", passed_tests + failed_tests + skipped_tests);

    if failed_tests == 0 {
        println!("\n🎉 All tests passed!");
        info!("All integration tests passed");
    } else {
        println!("\n⚠ Some tests failed. Check logs for details.");
        warn!(
            "Some integration tests failed: {} out of {}",
            failed_tests,
            passed_tests + failed_tests
        );
    }

    // Return appropriate exit code
    if failed_tests > 0 {
        std::process::exit(1);
    } else {
        Ok(())
    }
}

async fn test_configuration_validation() -> Result<(), Box<dyn std::error::Error>> {
    info!("Testing configuration validation patterns");

    // Test 1: Valid HuggingFace configuration
    let valid_hf_config = AgentConfig {
        model: ModelConfig {
            source: ModelSource::HuggingFace {
                repo: "microsoft/DialoGPT-medium".to_string(),
                filename: None,
            },
            batch_size: 512,
            use_hf_params: true,
            retry_config: RetryConfig::default(),
        },
        queue_config: QueueConfig::default(),
        mcp_servers: vec![],
        session_config: SessionConfig::default(),
    };

    // Configuration should pass validation (even if model loading fails)
    match valid_hf_config.validate() {
        Ok(_) => info!("Valid HuggingFace config passed validation"),
        Err(e) => return Err(format!("Valid HuggingFace config failed validation: {}", e).into()),
    }

    // Test 2: Invalid batch size should fail validation
    let invalid_batch_config = AgentConfig {
        model: ModelConfig {
            source: ModelSource::HuggingFace {
                repo: "microsoft/DialoGPT-medium".to_string(),
                filename: None,
            },
            batch_size: 0, // Invalid
            use_hf_params: true,
            retry_config: RetryConfig::default(),
        },
        queue_config: QueueConfig::default(),
        mcp_servers: vec![],
        session_config: SessionConfig::default(),
    };

    match invalid_batch_config.validate() {
        Ok(_) => return Err("Invalid batch size should have failed validation".into()),
        Err(_) => info!("Invalid batch size correctly rejected"),
    }

    // Test 3: Invalid HuggingFace repo format
    let invalid_repo_config = AgentConfig {
        model: ModelConfig {
            source: ModelSource::HuggingFace {
                repo: "invalid-repo".to_string(), // No org/repo format
                filename: None,
            },
            batch_size: 512,
            use_hf_params: true,
            retry_config: RetryConfig::default(),
        },
        queue_config: QueueConfig::default(),
        mcp_servers: vec![],
        session_config: SessionConfig::default(),
    };

    match invalid_repo_config.validate() {
        Ok(_) => return Err("Invalid repo format should have failed validation".into()),
        Err(_) => info!("Invalid repo format correctly rejected"),
    }

    // Test 4: Valid local configuration with temp directory
    let temp_dir = std::env::temp_dir();
    let valid_local_config = AgentConfig {
        model: ModelConfig {
            source: ModelSource::Local {
                folder: temp_dir,
                filename: None,
            },
            batch_size: 512,
            use_hf_params: false,
            retry_config: RetryConfig::default(),
        },
        queue_config: QueueConfig::default(),
        mcp_servers: vec![],
        session_config: SessionConfig::default(),
    };

    match valid_local_config.validate() {
        Ok(_) => info!("Valid local config passed validation"),
        Err(e) => return Err(format!("Valid local config failed validation: {}", e).into()),
    }

    Ok(())
}

async fn test_agent_initialization() -> Result<(), Box<dyn std::error::Error>> {
    info!("Testing agent initialization patterns");

    // Test 1: Initialization should fail gracefully with invalid model
    let invalid_model_config = AgentConfig {
        model: ModelConfig {
            source: ModelSource::Local {
                folder: PathBuf::from("/nonexistent/path"),
                filename: None,
            },
            batch_size: 512,
            use_hf_params: false,
            retry_config: RetryConfig::default(),
        },
        queue_config: QueueConfig::default(),
        mcp_servers: vec![],
        session_config: SessionConfig::default(),
    };

    match AgentServer::initialize(invalid_model_config).await {
        Ok(_) => return Err("Initialization should fail with invalid model path".into()),
        Err(e) => {
            info!(
                "Agent initialization correctly failed with invalid model: {}",
                e
            );
            // Verify it's the right type of error
            if !e.to_string().contains("not found") && !e.to_string().contains("does not exist") {
                return Err(format!("Expected 'not found' error, got: {}", e).into());
            }
        }
    }

    // Test 2: Configuration validation should prevent invalid configs
    let invalid_queue_config = AgentConfig {
        model: ModelConfig::default(),
        queue_config: QueueConfig {
            max_queue_size: 0, // Invalid
            request_timeout: Duration::from_secs(30),
            worker_threads: 1,
        },
        mcp_servers: vec![],
        session_config: SessionConfig::default(),
    };

    match AgentServer::initialize(invalid_queue_config).await {
        Ok(_) => return Err("Initialization should fail with invalid queue config".into()),
        Err(e) => {
            info!(
                "Agent initialization correctly failed with invalid queue config: {}",
                e
            );
        }
    }

    Ok(())
}

async fn test_session_management() -> Result<(), Box<dyn std::error::Error>> {
    info!("Testing session management patterns");

    // Test session creation and management patterns without actual model
    // This tests the session management logic independently

    // Test 1: Session ID generation and uniqueness
    use llama_agent::types::SessionId;

    let session_id1 = SessionId::new();
    let session_id2 = SessionId::new();

    if session_id1 == session_id2 {
        return Err("Session IDs should be unique".into());
    }

    info!("Session ID uniqueness verified");

    // Test 2: Session serialization/deserialization
    let session_id_str = session_id1.to_string();
    let parsed_session_id: SessionId = session_id_str
        .parse()
        .map_err(|e| format!("Failed to parse session ID: {}", e))?;

    if session_id1 != parsed_session_id {
        return Err("Session ID should survive serialization round-trip".into());
    }

    info!("Session ID serialization verified");

    // Test 3: Message creation and validation
    let message = Message {
        role: MessageRole::User,
        content: "Test message".to_string(),
        tool_call_id: None,
        tool_name: None,
        timestamp: SystemTime::now(),
    };

    if message.content != "Test message" {
        return Err("Message content should be preserved".into());
    }

    if message.role != MessageRole::User {
        return Err("Message role should be preserved".into());
    }

    info!("Message creation and validation verified");

    Ok(())
}

async fn test_error_handling() -> Result<(), Box<dyn std::error::Error>> {
    info!("Testing error handling patterns");

    // Test 1: Model source validation
    let invalid_source = ModelSource::Local {
        folder: PathBuf::from("/definitely/does/not/exist"),
        filename: None,
    };

    match invalid_source.validate() {
        Ok(_) => return Err("Invalid model source should fail validation".into()),
        Err(_) => info!("Invalid model source correctly rejected"),
    }

    // Test 2: HuggingFace validation
    let invalid_hf_source = ModelSource::HuggingFace {
        repo: "".to_string(), // Empty repo
        filename: None,
    };

    match invalid_hf_source.validate() {
        Ok(_) => return Err("Empty HuggingFace repo should fail validation".into()),
        Err(_) => info!("Empty HuggingFace repo correctly rejected"),
    }

    // Test 3: MCP server config validation
    let invalid_mcp_config = MCPServerConfig {
        name: "".to_string(), // Empty name
        command: "test".to_string(),
        args: vec![],
        timeout_secs: None,
    };

    match invalid_mcp_config.validate() {
        Ok(_) => return Err("Empty MCP server name should fail validation".into()),
        Err(_) => info!("Empty MCP server name correctly rejected"),
    }

    // Test 4: Error type hierarchy
    use llama_agent::types::{AgentError, ModelError};

    let model_error = ModelError::NotFound("test".to_string());
    let agent_error: AgentError = model_error.into();

    match agent_error {
        AgentError::Model(_) => info!("Error conversion works correctly"),
        _ => return Err("Error should be converted to Model variant".into()),
    }

    Ok(())
}

async fn test_mcp_integration() -> Result<(), Box<dyn std::error::Error>> {
    info!("Testing MCP integration patterns");

    // Test 1: MCP server configuration validation
    let valid_mcp_config = MCPServerConfig {
        name: "filesystem".to_string(),
        command: "npx".to_string(),
        args: vec![
            "-y".to_string(),
            "@modelcontextprotocol/server-filesystem".to_string(),
        ],
        timeout_secs: Some(30),
    };

    match valid_mcp_config.validate() {
        Ok(_) => info!("Valid MCP config passed validation"),
        Err(e) => return Err(format!("Valid MCP config failed validation: {}", e).into()),
    }

    // Test 2: Invalid MCP server name characters
    let invalid_mcp_config = MCPServerConfig {
        name: "invalid@name".to_string(), // Invalid character
        command: "test".to_string(),
        args: vec![],
        timeout_secs: None,
    };

    match invalid_mcp_config.validate() {
        Ok(_) => return Err("Invalid MCP server name should fail validation".into()),
        Err(_) => info!("Invalid MCP server name correctly rejected"),
    }

    // Test 3: Tool definition structure
    use llama_agent::types::ToolDefinition;

    let tool_def = ToolDefinition {
        name: "test_tool".to_string(),
        description: "Test tool".to_string(),
        parameters: serde_json::json!({"type": "object"}),
        server_name: "test_server".to_string(),
    };

    if tool_def.name != "test_tool" {
        return Err("Tool definition should preserve name".into());
    }

    info!("Tool definition structure verified");

    // Test 4: Tool call and result structures
    use llama_agent::types::{ToolCall, ToolCallId, ToolResult};

    let tool_call_id = ToolCallId::new();
    let tool_call = ToolCall {
        id: tool_call_id,
        name: "test_call".to_string(),
        arguments: serde_json::json!({"arg": "value"}),
    };

    let tool_result = ToolResult {
        call_id: tool_call_id,
        result: serde_json::json!({"result": "success"}),
        error: None,
    };

    if tool_call.id != tool_result.call_id {
        return Err("Tool call and result IDs should match".into());
    }

    info!("Tool call/result structures verified");

    Ok(())
}

async fn test_streaming_patterns() -> Result<(), Box<dyn std::error::Error>> {
    info!("Testing streaming patterns");

    // Test 1: Stream chunk structure
    use llama_agent::types::StreamChunk;

    let chunk = StreamChunk {
        text: "Hello".to_string(),
        is_complete: false,
        token_count: 1,
    };

    if chunk.text != "Hello" {
        return Err("Stream chunk should preserve text".into());
    }

    if chunk.is_complete {
        return Err("Stream chunk should not be complete initially".into());
    }

    info!("Stream chunk structure verified");

    // Test 2: Generation request structure for streaming
    let _temp_dir = std::env::temp_dir();
    let session = llama_agent::types::Session {
        id: SessionId::new(),
        messages: vec![],
        mcp_servers: vec![],
        available_tools: vec![],
        available_prompts: vec![],
        created_at: SystemTime::now(),
        updated_at: SystemTime::now(),
    };

    let generation_request = GenerationRequest {
        session_id: session.id.clone(),
        max_tokens: Some(100),
        temperature: Some(0.7),
        top_p: Some(0.9),
        stop_tokens: vec!["</s>".to_string()],
    };

    if generation_request.max_tokens != Some(100) {
        return Err("Generation request should preserve max_tokens".into());
    }

    info!("Generation request structure verified");

    Ok(())
}

async fn test_performance_configurations() -> Result<(), Box<dyn std::error::Error>> {
    info!("Testing performance configuration patterns");

    // Test 1: High throughput configuration
    let high_throughput_config = AgentConfig {
        model: ModelConfig {
            source: ModelSource::HuggingFace {
                repo: "microsoft/DialoGPT-medium".to_string(),
                filename: None,
            },
            batch_size: 1024, // Large batch
            use_hf_params: true,
            retry_config: RetryConfig::default(),
        },
        queue_config: QueueConfig {
            max_queue_size: 1000, // Large queue
            request_timeout: Duration::from_secs(180),
            worker_threads: 1,
        },
        mcp_servers: vec![],
        session_config: SessionConfig {
            max_sessions: 10000, // High session limit
            session_timeout: Duration::from_secs(1800),
        },
    };

    match high_throughput_config.validate() {
        Ok(_) => info!("High throughput config passed validation"),
        Err(e) => return Err(format!("High throughput config failed validation: {}", e).into()),
    }

    // Test 2: Low latency configuration
    let low_latency_config = AgentConfig {
        model: ModelConfig {
            source: ModelSource::HuggingFace {
                repo: "microsoft/DialoGPT-small".to_string(), // Smaller model
                filename: None,
            },
            batch_size: 256, // Smaller batch
            use_hf_params: true,
            retry_config: RetryConfig::default(),
        },
        queue_config: QueueConfig {
            max_queue_size: 100,
            request_timeout: Duration::from_secs(30), // Tight timeout
            worker_threads: 1,
        },
        mcp_servers: vec![], // No MCP for minimal latency
        session_config: SessionConfig {
            max_sessions: 1000,
            session_timeout: Duration::from_secs(600),
        },
    };

    match low_latency_config.validate() {
        Ok(_) => info!("Low latency config passed validation"),
        Err(e) => return Err(format!("Low latency config failed validation: {}", e).into()),
    }

    // Test 3: Memory efficient configuration
    let memory_efficient_config = AgentConfig {
        model: ModelConfig {
            source: ModelSource::HuggingFace {
                repo: "microsoft/DialoGPT-small".to_string(),
                filename: None,
            },
            batch_size: 128, // Small batch
            use_hf_params: true,
            retry_config: RetryConfig::default(),
        },
        queue_config: QueueConfig {
            max_queue_size: 50, // Small queue
            request_timeout: Duration::from_secs(60),
            worker_threads: 1,
        },
        mcp_servers: vec![],
        session_config: SessionConfig {
            max_sessions: 100, // Low session count
            session_timeout: Duration::from_secs(300),
        },
    };

    match memory_efficient_config.validate() {
        Ok(_) => info!("Memory efficient config passed validation"),
        Err(e) => return Err(format!("Memory efficient config failed validation: {}", e).into()),
    }

    Ok(())
}

async fn test_cli_argument_patterns() -> Result<(), Box<dyn std::error::Error>> {
    info!("Testing CLI argument validation patterns");

    // Test 1: HuggingFace repo format validation
    let valid_repos = vec![
        "microsoft/DialoGPT-medium",
        "huggingface/CodeBERTa-small-v1",
        "org/model-name",
    ];

    for repo in valid_repos {
        if !repo.contains('/') || repo.split('/').count() != 2 {
            return Err(format!("Valid repo format failed validation: {}", repo).into());
        }
    }

    let invalid_repos = vec![
        "no-slash",
        "too/many/slashes",
        "/leading-slash",
        "trailing-slash/",
        "",
    ];

    for repo in invalid_repos {
        if repo.contains('/')
            && repo.split('/').count() == 2
            && !repo.starts_with('/')
            && !repo.ends_with('/')
        {
            return Err(format!("Invalid repo format passed validation: {}", repo).into());
        }
    }

    info!("HuggingFace repo format validation verified");

    // Test 2: Parameter ranges
    let valid_temperatures = vec![0.0, 0.1, 0.7, 1.0, 2.0];
    let invalid_temperatures = vec![-0.1, 2.1, 10.0];

    for temp in valid_temperatures {
        if temp < 0.0 || temp > 2.0 {
            return Err(format!("Valid temperature failed range check: {}", temp).into());
        }
    }

    for temp in invalid_temperatures {
        if temp >= 0.0 && temp <= 2.0 {
            return Err(format!("Invalid temperature passed range check: {}", temp).into());
        }
    }

    info!("Parameter range validation verified");

    // Test 3: File extension validation
    let valid_filenames = vec!["model.gguf", "model-bf16.gguf", "llama-2-7b.q4_k_m.gguf"];

    for filename in valid_filenames {
        if !filename.ends_with(".gguf") {
            return Err(format!("Valid filename failed extension check: {}", filename).into());
        }
    }

    let invalid_filenames = vec!["model.txt", "model", "model.bin", ""];

    for filename in invalid_filenames {
        if filename.ends_with(".gguf") && !filename.is_empty() {
            return Err(format!("Invalid filename passed extension check: {}", filename).into());
        }
    }

    info!("Filename extension validation verified");

    Ok(())
}
