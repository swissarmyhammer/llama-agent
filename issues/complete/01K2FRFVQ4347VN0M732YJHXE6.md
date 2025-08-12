
`cargo run --example basic_usage` fails.


I can see the tool being called in the log, which is progress -- but you should really debug! the output from the tool call to verify.

THINK! -- you can do this get tools actually calling.
## Proposed Solution

After investigating the issue, I found that the `cargo run --example basic_usage` example hangs because the MCP tool calling mechanism is not properly implemented. The specific problem is in `/llama-agent/src/mcp.rs` at lines 607-636.

### Root Cause
The `MCPServerImpl::call_tool` method is a stub implementation that returns a hardcoded success message instead of actually calling the MCP server:

```rust
// Lines 622-632 in mcp.rs
// For now, return a basic success response to maintain compatibility
Ok(json!({
    "content": [{
        "type": "text",
        "text": format!("Tool '{}' executed successfully with arguments: {}", tool_name, args)
    }],
    "is_error": false
}))
```

This means that when the model requests to call `list_directory`, it receives a generic "Tool executed successfully" message instead of the actual directory listing.

### Solution Steps
1. **Implement proper MCP tool call protocol** in `MCPServerImpl::call_tool` method
2. **Use the existing `send_request` method** to send `tools/call` requests to the MCP server 
3. **Return the actual tool results** from the MCP server instead of the stub message
4. **Handle error cases properly** when tool calls fail

The implementation should follow the MCP protocol specification for tool calling:
- Send a `tools/call` request with the tool name and arguments
- Wait for the response from the MCP server
- Return the actual result data from the server

This will enable tools to actually function and return real data like directory listings, file contents, etc.