Try again, tools are still not calling.

cargo run --example basic_usage

You need to define an integration test that calls a tool.

You need to study https://github.com/EricLBuehler/mistral.rs, think deeply, and see how tools are called.

## Proposed Solution

Based on my analysis of the codebase, I can see that the tool calling infrastructure is in place but needs a complete integration test that demonstrates the full workflow. Here's my approach:

### Problem Analysis:
1. The `basic_usage.rs` example runs but tools are not being called successfully
2. The ChatTemplateEngine has tool call parsers (JSON, XML, FunctionCall) but they need to be tested in a real scenario
3. The generate method has a loop that detects "Tool call detected" finish reason and processes tool calls, but this isn't happening
4. The issue is that we need an end-to-end integration test that verifies the complete tool calling workflow

### Solution Steps:
1. **Create Integration Test**: Write a comprehensive integration test that:
   - Sets up an AgentServer with MCP filesystem server
   - Creates a session and discovers tools
   - Sends a message that should trigger tool usage
   - Verifies that tool calls are extracted, executed, and results processed
   - Tests the complete loop from user input → model generation → tool extraction → tool execution → response integration

2. **Test Tool Call Format Recognition**: Ensure the test verifies that:
   - Generated text contains recognizable tool call patterns
   - The JsonToolCallParser, XmlToolCallParser, or FunctionCallParser can extract them
   - Tool calls are properly executed via MCP servers
   - Results are integrated back into the conversation

3. **Verify Complete Workflow**: The test should prove that:
   - MCP server tools are discovered and available
   - Model generates text with tool calls in a recognizable format
   - Tool calls are extracted and executed successfully  
   - Tool results are added to the session and used for follow-up generation

### Implementation Plan:
- Create `/Users/wballard/github/llama-agent/tests/tool_calling_integration_test.rs`
- Test with a simple, reliable tool call scenario (filesystem operations)
- Use mocked/simplified model responses if needed to ensure tool call patterns are generated
- Verify each step of the tool calling pipeline works correctly