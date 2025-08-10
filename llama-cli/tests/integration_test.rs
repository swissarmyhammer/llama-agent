use anyhow::Result;
use llama_cli::{run_generate, GenerateArgs};
use tokio::test;
use tracing_subscriber;

/// Integration test that exercises the CLI with the unsloth/Qwen3-0.6B-GGUF model.
/// This test verifies that the CLI can successfully load a HuggingFace model and generate text.
#[test]
async fn test_cli_integration_with_qwen_model() -> Result<()> {
    // Initialize logging for the test
    let _ = tracing_subscriber::fmt()
        .with_max_level(tracing::Level::WARN)
        .try_init();

    // Create Args struct with the same parameters as the manual test
    // cargo run --package llama-agent-cli -- --model unsloth/Qwen3-0.6B-GGUF --prompt "What is an apple?" --limit 64
    let args = GenerateArgs {
        model: "unsloth/Qwen3-0.6B-GGUF".to_string(),
        filename: None,
        prompt: "What is an apple?".to_string(),
        limit: 64,
        temperature: 0.7,
        top_p: 0.9,
        debug: false, // Keep debug off to avoid verbose output in tests
        batch_size: 512,
        max_queue_size: 10,
        request_timeout: 120,
        worker_threads: 1,
        max_sessions: 10,
        session_timeout: 3600,
    };

    // Run the agent and verify it completes successfully
    let result = run_generate(args).await;

    // Verify the result is Ok and contains some generated text
    match result {
        Ok(response) => {
            // Verify we got a non-empty response
            assert!(!response.trim().is_empty(), "Response should not be empty");

            // Since we're asking about an apple, the response should be somewhat relevant
            // We'll do a basic check that it's at least a few characters long
            assert!(response.len() > 10, "Response should be substantial");

            // Log the response for manual inspection during test runs if needed
            eprintln!("Generated response: {}", response);

            Ok(())
        }
        Err(e) => {
            // If the test fails, provide detailed error information
            panic!("CLI integration test failed: {}", e);
        }
    }
}

/// Test CLI argument validation
#[test]
async fn test_cli_argument_validation() -> Result<()> {
    let _ = tracing_subscriber::fmt()
        .with_max_level(tracing::Level::WARN)
        .try_init();

    // Test with empty model - should fail validation
    let args_empty_model = GenerateArgs {
        model: "".to_string(),
        filename: None,
        prompt: "Test prompt".to_string(),
        limit: 64,
        temperature: 0.7,
        top_p: 0.9,
        debug: false,
        batch_size: 512,
        max_queue_size: 10,
        request_timeout: 120,
        worker_threads: 1,
        max_sessions: 10,
        session_timeout: 3600,
    };

    let result = run_generate(args_empty_model).await;
    assert!(result.is_err(), "Should fail validation with empty model");

    // Test with empty prompt - should fail validation
    let args_empty_prompt = GenerateArgs {
        model: "unsloth/Qwen3-0.6B-GGUF".to_string(),
        filename: None,
        prompt: "".to_string(),
        limit: 64,
        temperature: 0.7,
        top_p: 0.9,
        debug: false,
        batch_size: 512,
        max_queue_size: 10,
        request_timeout: 120,
        worker_threads: 1,
        max_sessions: 10,
        session_timeout: 3600,
    };

    let result = run_generate(args_empty_prompt).await;
    assert!(result.is_err(), "Should fail validation with empty prompt");

    // Test with invalid temperature - should fail validation
    let args_invalid_temp = GenerateArgs {
        model: "unsloth/Qwen3-0.6B-GGUF".to_string(),
        filename: None,
        prompt: "Test prompt".to_string(),
        limit: 64,
        temperature: 3.0, // Invalid - should be <= 2.0
        top_p: 0.9,
        debug: false,
        batch_size: 512,
        max_queue_size: 10,
        request_timeout: 120,
        worker_threads: 1,
        max_sessions: 10,
        session_timeout: 3600,
    };

    let result = run_generate(args_invalid_temp).await;
    assert!(
        result.is_err(),
        "Should fail validation with invalid temperature"
    );

    Ok(())
}

/// Test that the CLI can handle different token limits correctly
#[test]
async fn test_cli_with_different_token_limits() -> Result<()> {
    let _ = tracing_subscriber::fmt()
        .with_max_level(tracing::Level::WARN)
        .try_init();

    // Test with a very small token limit
    let args_small_limit = GenerateArgs {
        model: "unsloth/Qwen3-0.6B-GGUF".to_string(),
        filename: None,
        prompt: "What is an apple?".to_string(),
        limit: 10, // Very small limit
        temperature: 0.7,
        top_p: 0.9,
        debug: false,
        batch_size: 512,
        max_queue_size: 10,
        request_timeout: 120,
        worker_threads: 1,
        max_sessions: 10,
        session_timeout: 3600,
    };

    // This should still work, just with a shorter response
    let result = run_generate(args_small_limit).await;
    assert!(result.is_ok(), "Should work with small token limit");

    if let Ok(response) = result {
        // Response should be present but possibly truncated
        assert!(
            !response.trim().is_empty(),
            "Response should not be empty even with small limit"
        );
    }

    Ok(())
}
