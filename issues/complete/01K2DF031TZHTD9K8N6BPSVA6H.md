get all the examples to work
get all the examples to work

## Proposed Solution

I will systematically test and fix all the examples in the `examples/` directory to ensure they compile and run successfully. Based on the documentation, there are 8 examples to validate:

1. **basic_usage.rs** - Core functionality from specification (lines 605-709)
2. **tool_workflow.rs** - Manual tool call handling 
3. **streaming.rs** - Real-time streaming responses
4. **mcp_integration.rs** - Multiple MCP server integration
5. **error_handling.rs** - Error scenarios and recovery
6. **performance_examples.rs** - Performance optimization
7. **integration_tests.rs** - Automated validation tests
8. **cli_examples.md** - Command-line usage examples

My approach will be:

1. First verify the project compiles with `cargo build`
2. Test each example individually with `cargo run --example <name>`
3. Identify and fix any compilation errors
4. Identify and fix any runtime errors
5. Ensure all examples run successfully and demonstrate their intended functionality

The examples serve as both documentation and integration tests, so getting them working is critical for validating the entire llama-agent system functionality.