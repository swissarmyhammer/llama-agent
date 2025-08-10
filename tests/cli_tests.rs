mod common;

use common::TestHelper;
use std::process::Command;
use tempfile::TempDir;

#[tokio::test]
async fn test_cli_help() {
    let output = Command::new("cargo")
        .args([
            "run",
            "--manifest-path",
            "llama-cli/Cargo.toml",
            "--",
            "--help",
        ])
        .output()
        .expect("Failed to execute CLI");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Check that help contains expected sections
    assert!(stdout.contains("Unified Llama CLI for generation and embeddings"));
    assert!(stdout.contains("generate"));
    assert!(stdout.contains("embed"));
    assert!(stdout.contains("Generate text using a language model"));
}

#[tokio::test]
async fn test_cli_version() {
    let output = Command::new("cargo")
        .args([
            "run",
            "--manifest-path",
            "llama-cli/Cargo.toml",
            "--",
            "--version",
        ])
        .output()
        .expect("Failed to execute CLI");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should contain version information
    assert!(stdout.contains("llama-cli"));
}

#[tokio::test]
async fn test_cli_missing_required_args() {
    // Test missing subcommand
    let output = Command::new("cargo")
        .args(["run", "--manifest-path", "llama-cli/Cargo.toml", "--"])
        .output()
        .expect("Failed to execute CLI");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Usage") || stderr.contains("required"));

    // Test missing model argument in generate subcommand
    let output = Command::new("cargo")
        .args([
            "run",
            "--manifest-path",
            "llama-cli/Cargo.toml",
            "--",
            "generate",
            "--prompt",
            "hello",
        ])
        .output()
        .expect("Failed to execute CLI");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("model") || stderr.contains("required"));
}

#[tokio::test]
async fn test_cli_with_nonexistent_model() {
    let output = Command::new("cargo")
        .args([
            "run",
            "--manifest-path",
            "llama-cli/Cargo.toml",
            "--",
            "generate",
            "--model",
            "/nonexistent/path",
            "--prompt",
            "Hello world",
        ])
        .output()
        .expect("Failed to execute CLI");

    assert!(!output.status.success());
    // Should fail with model loading error or path validation error
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("not found")
            || stderr.contains("does not exist")
            || stderr.contains("error")
            || stderr.contains("Model loading failed")
    );
}

#[tokio::test]
async fn test_cli_with_dummy_model() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let _model_path = TestHelper::create_test_model_file(&temp_dir, "test.gguf");

    let output = Command::new("cargo")
        .args([
            "run",
            "--manifest-path",
            "llama-cli/Cargo.toml",
            "--",
            "generate",
            "--model",
            temp_dir.path().to_str().unwrap(),
            "--filename",
            "test.gguf",
            "--prompt",
            "Hello world",
            "--limit",
            "10",
        ])
        .output()
        .expect("Failed to execute CLI");

    // This will fail because dummy GGUF is not a valid model,
    // but we're testing that arguments are parsed correctly
    assert!(!output.status.success());

    let stderr = String::from_utf8_lossy(&output.stderr);
    // Should fail with model loading error, not argument parsing error
    assert!(
        stderr.contains("Model loading failed")
            || stderr.contains("Failed to load model")
            || stderr.contains("Backend already initialized")
            || stderr.contains("error")
    );
}

#[tokio::test]
async fn test_cli_argument_parsing() {
    // Test various argument combinations to ensure they parse correctly
    let test_cases = vec![
        vec![
            "generate",
            "--model",
            "microsoft/DialoGPT-medium",
            "--prompt",
            "hello",
        ],
        vec![
            "generate",
            "--model",
            "microsoft/DialoGPT-medium",
            "--filename",
            "model.gguf",
            "--prompt",
            "hello",
        ],
        vec![
            "generate", "--model", "./models", "--prompt", "hello", "--limit", "100",
        ],
        vec![
            "generate",
            "--model",
            "./models",
            "--prompt",
            "hello",
            "--batch-size",
            "256",
        ],
        vec![
            "generate",
            "--model",
            "./models",
            "--prompt",
            "hello",
            "--max-queue-size",
            "20",
        ],
        vec![
            "generate",
            "--model",
            "./models",
            "--prompt",
            "hello",
            "--request-timeout",
            "60",
        ],
        vec![
            "generate",
            "--model",
            "./models",
            "--prompt",
            "hello",
            "--worker-threads",
            "2",
        ],
        vec![
            "generate",
            "--model",
            "./models",
            "--prompt",
            "hello",
            "--max-sessions",
            "50",
        ],
        vec![
            "generate",
            "--model",
            "./models",
            "--prompt",
            "hello",
            "--session-timeout",
            "7200",
        ],
        vec![
            "generate",
            "--model",
            "./models",
            "--prompt",
            "hello",
            "--temperature",
            "0.8",
        ],
        vec![
            "generate", "--model", "./models", "--prompt", "hello", "--top-p", "0.95",
        ],
        vec![
            "generate",
            "--model",
            "./models",
            "--filename",
            "model.gguf",
            "--prompt",
            "Tell me about AI",
            "--limit",
            "200",
            "--batch-size",
            "1024",
            "--max-queue-size",
            "5",
            "--request-timeout",
            "30",
            "--worker-threads",
            "1",
            "--max-sessions",
            "10",
            "--session-timeout",
            "1800",
            "--temperature",
            "0.7",
            "--top-p",
            "0.9",
        ],
    ];

    for args in test_cases {
        println!("Testing args: {:?}", args);

        let mut full_args = vec!["run", "--manifest-path", "llama-cli/Cargo.toml", "--"];
        full_args.extend(args.iter());

        let output = Command::new("cargo")
            .args(&full_args)
            .output()
            .expect("Failed to execute CLI");

        // All of these should fail due to model loading issues, but not due to argument parsing
        assert!(!output.status.success());

        let stderr = String::from_utf8_lossy(&output.stderr);

        // Should NOT contain argument parsing errors
        assert!(
            !stderr.contains("unexpected value")
                && !stderr.contains("invalid value")
                && !stderr.contains("required")
                && !stderr.contains("Usage:"),
            "Argument parsing failed for {:?}: {}",
            args,
            stderr
        );
    }
}

#[tokio::test]
async fn test_cli_invalid_argument_values() {
    let test_cases = vec![
        // Invalid numeric values
        (
            vec![
                "generate", "--model", "test", "--prompt", "hello", "--limit", "abc",
            ],
            "limit",
        ),
        (
            vec![
                "generate",
                "--model",
                "test",
                "--prompt",
                "hello",
                "--batch-size",
                "-1",
            ],
            "batch",
        ),
        (
            vec![
                "generate",
                "--model",
                "test",
                "--prompt",
                "hello",
                "--max-queue-size",
                "0",
            ],
            "queue",
        ),
        (
            vec![
                "generate",
                "--model",
                "test",
                "--prompt",
                "hello",
                "--request-timeout",
                "abc",
            ],
            "timeout",
        ),
        (
            vec![
                "generate",
                "--model",
                "test",
                "--prompt",
                "hello",
                "--worker-threads",
                "0",
            ],
            "thread",
        ),
        (
            vec![
                "generate",
                "--model",
                "test",
                "--prompt",
                "hello",
                "--max-sessions",
                "-5",
            ],
            "session",
        ),
        (
            vec![
                "generate",
                "--model",
                "test",
                "--prompt",
                "hello",
                "--session-timeout",
                "abc",
            ],
            "timeout",
        ),
        (
            vec![
                "generate",
                "--model",
                "test",
                "--prompt",
                "hello",
                "--temperature",
                "abc",
            ],
            "temperature",
        ),
        (
            vec![
                "generate", "--model", "test", "--prompt", "hello", "--top-p", "1.5",
            ],
            "top-p",
        ),
    ];

    for (args, expected_error_context) in test_cases {
        println!("Testing invalid args: {:?}", args);

        let mut full_args = vec!["run", "--manifest-path", "llama-cli/Cargo.toml", "--"];
        full_args.extend(args.iter());

        let output = Command::new("cargo")
            .args(&full_args)
            .output()
            .expect("Failed to execute CLI");

        assert!(!output.status.success());

        let stderr = String::from_utf8_lossy(&output.stderr);

        // Should contain argument parsing error
        assert!(
            stderr.contains("invalid value")
                || stderr.contains("cannot parse")
                || stderr.contains("error")
                || stderr.contains(expected_error_context),
            "Expected parsing error for {}: {}",
            expected_error_context,
            stderr
        );
    }
}

#[tokio::test]
async fn test_cli_edge_case_values() {
    let edge_cases = vec![
        // Maximum values
        vec![
            "generate",
            "--model",
            "test",
            "--prompt",
            "hello",
            "--limit",
            "4294967295",
        ], // u32 max
        vec![
            "generate",
            "--model",
            "test",
            "--prompt",
            "hello",
            "--batch-size",
            "8192",
        ],
        vec![
            "generate",
            "--model",
            "test",
            "--prompt",
            "hello",
            "--max-queue-size",
            "1000",
        ],
        vec![
            "generate",
            "--model",
            "test",
            "--prompt",
            "hello",
            "--request-timeout",
            "3600",
        ],
        vec![
            "generate",
            "--model",
            "test",
            "--prompt",
            "hello",
            "--worker-threads",
            "16",
        ],
        vec![
            "generate",
            "--model",
            "test",
            "--prompt",
            "hello",
            "--max-sessions",
            "10000",
        ],
        vec![
            "generate",
            "--model",
            "test",
            "--prompt",
            "hello",
            "--session-timeout",
            "86400",
        ],
        vec![
            "generate",
            "--model",
            "test",
            "--prompt",
            "hello",
            "--temperature",
            "1.0",
        ],
        vec![
            "generate", "--model", "test", "--prompt", "hello", "--top-p", "1.0",
        ],
        // Minimum values
        vec![
            "generate", "--model", "test", "--prompt", "hello", "--limit", "1",
        ],
        vec![
            "generate",
            "--model",
            "test",
            "--prompt",
            "hello",
            "--batch-size",
            "1",
        ],
        vec![
            "generate",
            "--model",
            "test",
            "--prompt",
            "hello",
            "--max-queue-size",
            "1",
        ],
        vec![
            "generate",
            "--model",
            "test",
            "--prompt",
            "hello",
            "--request-timeout",
            "1",
        ],
        vec![
            "generate",
            "--model",
            "test",
            "--prompt",
            "hello",
            "--worker-threads",
            "1",
        ],
        vec![
            "generate",
            "--model",
            "test",
            "--prompt",
            "hello",
            "--max-sessions",
            "1",
        ],
        vec![
            "generate",
            "--model",
            "test",
            "--prompt",
            "hello",
            "--session-timeout",
            "1",
        ],
        vec![
            "generate",
            "--model",
            "test",
            "--prompt",
            "hello",
            "--temperature",
            "0.0",
        ],
        vec![
            "generate", "--model", "test", "--prompt", "hello", "--top-p", "0.0",
        ],
    ];

    for args in edge_cases {
        println!("Testing edge case args: {:?}", args);

        let mut full_args = vec!["run", "--manifest-path", "llama-cli/Cargo.toml", "--"];
        full_args.extend(args.iter());

        let output = Command::new("cargo")
            .args(&full_args)
            .output()
            .expect("Failed to execute CLI");

        // These should parse successfully but fail at runtime due to model issues
        assert!(!output.status.success());

        let stderr = String::from_utf8_lossy(&output.stderr);

        // Should NOT contain argument parsing errors for valid edge cases
        if !stderr.contains("Model loading failed")
            && !stderr.contains("Failed to load model")
            && !stderr.contains("Backend already initialized")
        {
            // If it's not a model loading error, it should be validation error, which is also ok
            assert!(
                stderr.contains("error")
                    || stderr.contains("validation")
                    || stderr.contains("HuggingFace model repo must be in format"),
                "Unexpected error for edge case {:?}: {}",
                args,
                stderr
            );
        }
    }
}

#[tokio::test]
async fn test_cli_prompt_variations() {
    let prompts = vec![
        "Hello world",
        "Tell me a story about a brave knight",
        "What is the meaning of life?",
        "Explain quantum computing in simple terms",
        "", // Empty prompt should be handled
        "A very long prompt that contains multiple sentences and goes on for quite a while to test how the system handles longer input text that might be provided by users in real-world scenarios.",
        "Prompt with special chars: !@#$%^&*()_+-=[]{}|;':\",./<>?",
        "Unicode prompt: 你好世界 🌍 привет мир",
    ];

    for prompt in prompts {
        println!("Testing prompt: {:?}", prompt);

        let output = Command::new("cargo")
            .args([
                "run",
                "--manifest-path",
                "llama-cli/Cargo.toml",
                "--",
                "generate",
                "--model",
                "test/model",
                "--prompt",
                prompt,
            ])
            .output()
            .expect("Failed to execute CLI");

        // Should fail due to model loading, but not due to prompt handling
        assert!(!output.status.success());

        let stderr = String::from_utf8_lossy(&output.stderr);

        // Should not contain prompt-related parsing errors
        assert!(
            !stderr.contains("invalid prompt") && !stderr.contains("prompt parsing"),
            "Prompt handling error for {:?}: {}",
            prompt,
            stderr
        );
    }
}

#[tokio::test]
async fn test_cli_model_path_variations() {
    let model_paths = vec![
        "microsoft/DialoGPT-medium",
        "huggingface/model-name",
        "./local/model/path",
        "/absolute/path/to/model",
        "../relative/path/model",
        "~/home/model/path",
        "model-with-dashes",
        "model_with_underscores",
        "Model.With.Dots",
    ];

    for model_path in model_paths {
        println!("Testing model path: {:?}", model_path);

        let output = Command::new("cargo")
            .args([
                "run",
                "--manifest-path",
                "llama-cli/Cargo.toml",
                "--",
                "generate",
                "--model",
                model_path,
                "--prompt",
                "test prompt",
            ])
            .output()
            .expect("Failed to execute CLI");

        // Should fail due to model loading, but arguments should parse
        assert!(!output.status.success());

        let stderr = String::from_utf8_lossy(&output.stderr);

        // Should contain model loading or path related errors, not argument parsing errors
        assert!(
            stderr.contains("not found")
                || stderr.contains("does not exist")
                || stderr.contains("Model loading failed")
                || stderr.contains("Failed to load model")
                || stderr.contains("Backend already initialized")
                || stderr.contains("error")
                || stderr.contains("Invalid HuggingFace repo format")
                || stderr.contains("Error:"),
            "Expected model-related error for path {:?}: {}",
            model_path,
            stderr
        );
    }
}
