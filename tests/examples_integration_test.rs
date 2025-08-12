//! Integration tests that run all examples to ensure they work correctly.
//!
//! This test suite ensures that all examples in the examples/ directory
//! compile and execute successfully. It serves as a regression test
//! to catch breaking changes that affect example functionality.

use std::collections::HashMap;
use std::process::Stdio;
use std::time::{Duration, Instant};
use tokio::process::Command as TokioCommand;
use tokio::time::timeout;
use tracing::{error, info, warn};

/// Configuration for running example tests
struct ExampleTestConfig {
    /// Maximum time to allow an example to run
    timeout: Duration,
    /// Whether to capture stdout/stderr output
    capture_output: bool,
    /// Examples that are expected to require external dependencies
    /// and should be skipped if those dependencies are unavailable
    conditional_examples: Vec<&'static str>,
}

impl Default for ExampleTestConfig {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(120), // 2 minutes max per example
            capture_output: true,
            conditional_examples: vec!["mcp_integration"], // May require MCP servers
        }
    }
}

#[derive(Debug)]
struct ExampleTestResult {
    #[allow(dead_code)] // Used in debug output and error reporting
    name: String,
    success: bool,
    duration: Duration,
    #[allow(dead_code)] // May be used in future debugging features
    exit_code: Option<i32>,
    stdout: String,
    stderr: String,
    error_message: Option<String>,
}

/// Test that all examples can be compiled and executed successfully
#[tokio::test]
async fn test_all_examples_execute_successfully() {
    // Initialize logging
    tracing_subscriber::fmt::init();

    info!("Starting examples integration test");

    let config = ExampleTestConfig::default();
    let example_names = get_example_names().expect("Failed to get example names");

    info!("Found {} examples to test", example_names.len());

    let mut results = HashMap::new();
    let mut passed = 0;
    let mut failed = 0;
    let mut skipped = 0;

    for example_name in &example_names {
        info!("Testing example: {}", example_name);

        let result = run_example(example_name, &config).await;

        match &result {
            Ok(test_result) if test_result.success => {
                info!(
                    "✓ Example '{}' passed in {:?}",
                    example_name, test_result.duration
                );
                passed += 1;
            }
            Ok(test_result) => {
                if config.conditional_examples.contains(&example_name.as_str())
                    && is_dependency_missing(test_result)
                {
                    warn!(
                        "⚠ Example '{}' skipped (missing dependencies)",
                        example_name
                    );
                    skipped += 1;
                } else {
                    error!(
                        "❌ Example '{}' failed: {:?}",
                        example_name, test_result.error_message
                    );
                    failed += 1;
                }
            }
            Err(e) => {
                error!("❌ Example '{}' failed to run: {}", example_name, e);
                failed += 1;
            }
        }

        results.insert(example_name.clone(), result);
    }

    // Print summary
    println!("\n{}", "=".repeat(60));
    println!("Examples Integration Test Summary:");
    println!("  ✓ Passed: {}", passed);
    println!("  ❌ Failed: {}", failed);
    println!("  ⚠ Skipped: {}", skipped);
    println!("  Total: {}", example_names.len());

    if failed > 0 {
        println!("\n❌ Failed examples:");
        for (name, result) in &results {
            if let Ok(test_result) = result {
                if !test_result.success && !config.conditional_examples.contains(&name.as_str()) {
                    println!(
                        "  - {}: {}",
                        name,
                        test_result
                            .error_message
                            .as_ref()
                            .unwrap_or(&"Unknown error".to_string())
                    );

                    // Print stderr if it contains useful info
                    if !test_result.stderr.is_empty() {
                        let stderr_preview = if test_result.stderr.len() > 200 {
                            format!("{}...", &test_result.stderr[..200])
                        } else {
                            test_result.stderr.clone()
                        };
                        println!("    stderr: {}", stderr_preview);
                    }
                }
            }
        }
    }

    info!(
        "Examples integration test completed: {} passed, {} failed, {} skipped",
        passed, failed, skipped
    );

    // Test should fail if any examples failed (not including conditional skips)
    assert_eq!(failed, 0, "Some examples failed to execute successfully");
}

/// Get the list of example names by scanning the examples directory
fn get_example_names() -> Result<Vec<String>, Box<dyn std::error::Error>> {
    use std::fs;
    use std::path::Path;

    let examples_dir = Path::new("examples");
    if !examples_dir.exists() {
        return Err("Examples directory does not exist".into());
    }

    let mut example_names = Vec::new();

    for entry in fs::read_dir(examples_dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.extension().and_then(|s| s.to_str()) == Some("rs") {
            if let Some(file_stem) = path.file_stem().and_then(|s| s.to_str()) {
                example_names.push(file_stem.to_string());
            }
        }
    }

    example_names.sort();
    Ok(example_names)
}

/// Run a single example and return the test result
async fn run_example(
    example_name: &str,
    config: &ExampleTestConfig,
) -> Result<ExampleTestResult, Box<dyn std::error::Error>> {
    let start_time = Instant::now();

    let mut cmd = TokioCommand::new("cargo");
    cmd.arg("run").arg("--example").arg(example_name);

    if config.capture_output {
        cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
    }

    info!(
        "Running: cargo run --example {} (timeout: {:?})",
        example_name, config.timeout
    );

    // Implement timeout handling
    let output_result = timeout(config.timeout, cmd.output()).await;

    let duration = start_time.elapsed();

    let (success, exit_code, stdout, stderr, error_message) = match output_result {
        Ok(Ok(output)) => {
            let stdout = String::from_utf8_lossy(&output.stdout).to_string();
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            let success = output.status.success();
            let exit_code = output.status.code();

            let error_message = if !success {
                Some(format!("Exit code: {:?}, stderr: {}", exit_code, stderr))
            } else {
                None
            };

            (success, exit_code, stdout, stderr, error_message)
        }
        Ok(Err(e)) => {
            let error_msg = format!("Failed to execute command: {}", e);
            (false, None, String::new(), String::new(), Some(error_msg))
        }
        Err(_) => {
            let error_msg = format!("Example timed out after {:?}", config.timeout);
            (false, None, String::new(), String::new(), Some(error_msg))
        }
    };

    Ok(ExampleTestResult {
        name: example_name.to_string(),
        success,
        duration,
        exit_code,
        stdout,
        stderr,
        error_message,
    })
}

/// Check if the test result indicates missing dependencies rather than a real failure
fn is_dependency_missing(result: &ExampleTestResult) -> bool {
    let stderr_lower = result.stderr.to_lowercase();
    let stdout_lower = result.stdout.to_lowercase();

    // Common patterns that indicate missing dependencies
    let dependency_patterns = [
        "connection refused",
        "no such host",
        "timeout",
        "mcp server",
        "not available",
        "not found",
        "unable to connect",
    ];

    dependency_patterns
        .iter()
        .any(|pattern| stderr_lower.contains(pattern) || stdout_lower.contains(pattern))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_example_names() {
        let names = get_example_names().expect("Should get example names");
        assert!(!names.is_empty(), "Should have at least one example");
        assert!(
            names.contains(&"basic_usage".to_string()),
            "Should include basic_usage example"
        );
    }

    #[test]
    fn test_is_dependency_missing() {
        let result_with_connection_error = ExampleTestResult {
            name: "test".to_string(),
            success: false,
            duration: Duration::from_secs(1),
            exit_code: Some(1),
            stdout: "".to_string(),
            stderr: "Connection refused".to_string(),
            error_message: None,
        };

        assert!(is_dependency_missing(&result_with_connection_error));

        let result_with_real_error = ExampleTestResult {
            name: "test".to_string(),
            success: false,
            duration: Duration::from_secs(1),
            exit_code: Some(1),
            stdout: "".to_string(),
            stderr: "panic at main.rs:42".to_string(),
            error_message: None,
        };

        assert!(!is_dependency_missing(&result_with_real_error));
    }
}
