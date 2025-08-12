we need an integration test that runs all the examples
## Proposed Solution

After analyzing the current codebase, I see there are several Rust examples in the `examples/` directory:

- `basic_usage.rs`
- `error_handling.rs`  
- `integration_tests.rs` (already exists but is a comprehensive test suite)
- `mcp_integration.rs`
- `performance_examples.rs`
- `streaming.rs`
- `tool_workflow.rs`

The current `examples/integration_tests.rs` is actually a comprehensive test suite that validates patterns and functionality, but we need a true integration test that **runs each example** to ensure they all compile and execute successfully.

I will create a new integration test in `tests/examples_integration_test.rs` that:

1. Compiles and runs each example using `cargo run --example <name>`
2. Validates that each example exits successfully (exit code 0)
3. Captures any stdout/stderr output for debugging
4. Provides clear reporting on which examples pass/fail
5. Uses proper timeout handling for examples that might hang
6. Handles examples that require external dependencies gracefully

This will ensure that all examples remain functional as the codebase evolves and will catch regressions early.
## Implementation Complete ✅

The integration test has been successfully implemented and tested. Here's what was delivered:

### New File Created
- `tests/examples_integration_test.rs` - A comprehensive integration test that runs all examples

### Features Implemented
1. **Dynamic Example Discovery**: Automatically scans the `examples/` directory for `.rs` files
2. **Timeout Handling**: Each example has a 2-minute timeout to prevent hanging
3. **Comprehensive Error Reporting**: Captures stdout/stderr and provides detailed failure information  
4. **Dependency Detection**: Smart detection of missing dependencies vs real failures
5. **Conditional Testing**: Some examples (like MCP integration) can be gracefully skipped if dependencies are unavailable
6. **Parallel Execution**: Uses async/await for efficient execution
7. **Full Logging**: Comprehensive tracing output for debugging

### Test Results
All 7 examples are currently passing:
- ✅ `basic_usage` (8.5s)
- ✅ `error_handling` (1s) 
- ✅ `integration_tests` (0.7s)
- ✅ `mcp_integration` (5.3s)
- ✅ `performance_examples` (0.6s)
- ✅ `streaming` (10.4s)
- ✅ `tool_workflow` (3.3s)

### Running the Test
```bash
cargo test --test examples_integration_test
```

This integration test will now catch any regressions that break example functionality, ensuring all examples remain working as the codebase evolves.