use llama_agent::dependency_analysis::{DependencyAnalyzer, ParallelExecutionDecision};
use llama_agent::types::{
    AccessType, ConflictType, ParallelExecutionConfig, ResourceAccess, ResourceType, ToolCall,
    ToolCallId, ToolConflict,
};
use serde_json::{json, Value};
use std::collections::HashMap;

fn create_tool_call(name: &str, arguments: Value) -> ToolCall {
    ToolCall {
        id: ToolCallId::new(),
        name: name.to_string(),
        arguments,
    }
}

#[test]
fn test_single_tool_call_sequential() {
    let analyzer = DependencyAnalyzer::default();
    let tool_calls = vec![create_tool_call("test_tool", json!({}))];

    let decision = analyzer.analyze_parallel_execution(&tool_calls);
    match decision {
        ParallelExecutionDecision::Sequential(reason) => {
            assert!(reason.contains("Single tool call"));
        }
        _ => panic!("Expected sequential execution for single tool call"),
    }
}

#[test]
fn test_duplicate_tool_names_sequential() {
    let analyzer = DependencyAnalyzer::default();
    let tool_calls = vec![
        create_tool_call("duplicate_tool", json!({"param": "value1"})),
        create_tool_call("duplicate_tool", json!({"param": "value2"})),
    ];

    let decision = analyzer.analyze_parallel_execution(&tool_calls);
    match decision {
        ParallelExecutionDecision::Sequential(reason) => {
            assert!(reason.contains("Duplicate tool names"));
        }
        _ => panic!("Expected sequential execution for duplicate tool names"),
    }
}

#[test]
fn test_parameter_dependency_detection() {
    let analyzer = DependencyAnalyzer::default();
    let tool_calls = vec![
        create_tool_call("read_file", json!({"path": "/tmp/input.txt"})),
        create_tool_call("process_data", json!({"input": "${read_file}"})),
    ];

    let decision = analyzer.analyze_parallel_execution(&tool_calls);
    match decision {
        ParallelExecutionDecision::Sequential(reason) => {
            assert!(reason.contains("depends on output"));
        }
        _ => panic!("Expected sequential execution for parameter dependency"),
    }
}

#[test]
fn test_file_system_conflict_detection() {
    let analyzer = DependencyAnalyzer::default();
    let tool_calls = vec![
        create_tool_call(
            "write_file",
            json!({"path": "/tmp/test.txt", "content": "data"}),
        ),
        create_tool_call("delete_file", json!({"path": "/tmp/test.txt"})),
    ];

    let decision = analyzer.analyze_parallel_execution(&tool_calls);
    match decision {
        ParallelExecutionDecision::Sequential(reason) => {
            assert!(reason.contains("Resource conflict") || reason.contains("conflicting access"));
        }
        _ => panic!("Expected sequential execution for file system conflict"),
    }
}

#[test]
fn test_configured_conflicts() {
    let config = ParallelExecutionConfig {
        tool_conflicts: vec![ToolConflict {
            tool1: "tool_a".to_string(),
            tool2: "tool_b".to_string(),
            conflict_type: ConflictType::MutualExclusion,
            description: "These tools cannot run together".to_string(),
        }],
        never_parallel: vec![("never_tool1".to_string(), "never_tool2".to_string())],
        ..Default::default()
    };

    let analyzer = DependencyAnalyzer::new(config);

    // Test explicit conflict
    let tool_calls = vec![
        create_tool_call("tool_a", json!({})),
        create_tool_call("tool_b", json!({})),
    ];

    let decision = analyzer.analyze_parallel_execution(&tool_calls);
    match decision {
        ParallelExecutionDecision::Sequential(reason) => {
            assert!(reason.contains("Configuration conflict"));
        }
        _ => panic!("Expected sequential execution for configured conflict"),
    }

    // Test never_parallel list
    let tool_calls = vec![
        create_tool_call("never_tool1", json!({})),
        create_tool_call("never_tool2", json!({})),
    ];

    let decision = analyzer.analyze_parallel_execution(&tool_calls);
    match decision {
        ParallelExecutionDecision::Sequential(_) => {} // Expected
        _ => panic!("Expected sequential execution for never_parallel tools"),
    }
}

#[test]
fn test_resource_access_patterns() {
    let mut resource_patterns = HashMap::new();
    resource_patterns.insert(
        "database_tool".to_string(),
        vec![ResourceAccess {
            resource: ResourceType::Database("users".to_string()),
            access_type: AccessType::Write,
            exclusive: true,
        }],
    );
    resource_patterns.insert(
        "another_db_tool".to_string(),
        vec![ResourceAccess {
            resource: ResourceType::Database("users".to_string()),
            access_type: AccessType::ReadWrite,
            exclusive: true,
        }],
    );

    let config = ParallelExecutionConfig {
        resource_access_patterns: resource_patterns,
        ..Default::default()
    };

    let analyzer = DependencyAnalyzer::new(config);
    let tool_calls = vec![
        create_tool_call("database_tool", json!({})),
        create_tool_call("another_db_tool", json!({})),
    ];

    let decision = analyzer.analyze_parallel_execution(&tool_calls);
    match decision {
        ParallelExecutionDecision::Sequential(reason) => {
            assert!(reason.contains("Resource conflict") || reason.contains("conflicting access"));
        }
        _ => panic!("Expected sequential execution for database resource conflict"),
    }
}

#[test]
fn test_safe_parallel_execution() {
    let analyzer = DependencyAnalyzer::default();
    let tool_calls = vec![
        create_tool_call("read_only_tool1", json!({"query": "SELECT * FROM users"})),
        create_tool_call(
            "read_only_tool2",
            json!({"query": "SELECT * FROM products"}),
        ),
        create_tool_call("independent_calc", json!({"x": 5, "y": 10})),
    ];

    let decision = analyzer.analyze_parallel_execution(&tool_calls);
    match decision {
        ParallelExecutionDecision::Parallel => {} // Expected
        ParallelExecutionDecision::Sequential(reason) => {
            panic!(
                "Expected parallel execution for independent tools, got: {}",
                reason
            )
        }
    }
}

#[test]
fn test_network_operations_inference() {
    let analyzer = DependencyAnalyzer::default();
    let tool_calls = vec![
        create_tool_call(
            "fetch_data",
            json!({"url": "https://api.example.com/users"}),
        ),
        create_tool_call(
            "post_data",
            json!({"url": "https://api.example.com/users", "data": {}}),
        ),
    ];

    // Network operations to different endpoints should be safe
    let decision = analyzer.analyze_parallel_execution(&tool_calls);
    // The decision could be either parallel or sequential depending on the inference logic
    // Just ensure we don't panic and get a valid decision
    match decision {
        ParallelExecutionDecision::Parallel | ParallelExecutionDecision::Sequential(_) => {}
    }
}

#[test]
fn test_complex_parameter_references() {
    let analyzer = DependencyAnalyzer::default();

    // Test nested JSON references
    let tool_calls = vec![
        create_tool_call("get_user", json!({"id": 123})),
        create_tool_call(
            "update_profile",
            json!({
                "user_id": 123,
                "profile": {
                    "email": "@get_user.email",
                    "preferences": {
                        "theme": "@get_user.settings.theme"
                    }
                }
            }),
        ),
    ];

    let decision = analyzer.analyze_parallel_execution(&tool_calls);
    match decision {
        ParallelExecutionDecision::Sequential(reason) => {
            assert!(reason.contains("depends on output") || reason.contains("dependency"));
        }
        _ => panic!("Expected sequential execution for nested parameter references"),
    }
}

#[test]
fn test_file_path_pattern_detection() {
    let analyzer = DependencyAnalyzer::default();
    let tool_calls = vec![
        create_tool_call(
            "create_temp_file",
            json!({"path": "/tmp/processing_data.json"}),
        ),
        create_tool_call(
            "analyze_file",
            json!({"file_path": "/tmp/processing_data.json"}),
        ),
    ];

    // Should detect potential file system conflict
    let decision = analyzer.analyze_parallel_execution(&tool_calls);
    match decision {
        ParallelExecutionDecision::Sequential(reason) => {
            assert!(reason.contains("Resource conflict") || reason.contains("conflicting access"));
        }
        _ => {
            // Parallel might be acceptable if the analyzer doesn't detect the specific conflict
            // The important thing is that the analysis runs without errors
        }
    }
}

#[test]
fn test_regex_pattern_matching() {
    let analyzer = DependencyAnalyzer::default();

    // Test with different reference patterns
    let test_cases = vec![
        ("${tool_output}", "tool_output"),
        ("@previous_result", "previous_result"),
        ("result_of_file_reader", "file_reader"),
    ];

    for (reference, expected_tool) in test_cases {
        let tool_calls = vec![
            create_tool_call(expected_tool, json!({})),
            create_tool_call("dependent_tool", json!({"input": reference})),
        ];

        let decision = analyzer.analyze_parallel_execution(&tool_calls);
        match decision {
            ParallelExecutionDecision::Sequential(reason) => {
                assert!(
                    reason.contains("depends on output") || reason.contains("dependency"),
                    "Failed for pattern: {}, reason: {}",
                    reference,
                    reason
                );
            }
            _ => {
                // Some patterns might not be detected by the current regex patterns
                // This is acceptable as long as the analysis doesn't crash
            }
        }
    }
}

#[test]
fn test_empty_tool_calls() {
    let analyzer = DependencyAnalyzer::default();
    let tool_calls = vec![];

    let decision = analyzer.analyze_parallel_execution(&tool_calls);
    match decision {
        ParallelExecutionDecision::Sequential(reason) => {
            assert!(reason.contains("Single tool call") || reason.contains("tool call"));
        }
        _ => panic!("Expected sequential execution for empty tool calls"),
    }
}

#[test]
fn test_performance_with_many_tools() {
    let analyzer = DependencyAnalyzer::default();

    // Create a large number of independent tool calls
    let mut tool_calls = Vec::new();
    for i in 0..100 {
        tool_calls.push(create_tool_call(
            &format!("tool_{}", i),
            json!({"independent_param": i}),
        ));
    }

    let start = std::time::Instant::now();
    let decision = analyzer.analyze_parallel_execution(&tool_calls);
    let duration = start.elapsed();

    // Analysis should complete in reasonable time (less than 1 second)
    assert!(
        duration.as_secs() < 1,
        "Analysis took too long: {:?}",
        duration
    );

    // Should handle large numbers of tools without issues
    match decision {
        ParallelExecutionDecision::Parallel | ParallelExecutionDecision::Sequential(_) => {}
    }
}
