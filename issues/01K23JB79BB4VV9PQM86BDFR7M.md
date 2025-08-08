The MCP spec and MCP tools support list change. Let's make sure we do to:

https://modelcontextprotocol.io/specification/2025-06-18/server/tools#list-changed-notification

## Proposed Solution

Based on the MCP specification for list changed notifications and analysis of the current codebase, I need to implement:

### 1. Add support for `listChanged` capability in initialization
- Modify the initialization handshake (line 207-216 in mcp.rs) to include `listChanged: true` in the tools capability
- This tells the MCP client that our server supports list changed notifications

### 2. Add notification handling infrastructure  
- Create a mechanism to send notifications without expecting a response (unlike requests)
- Add a `send_notification` method to `MCPServerImpl` 
- Implement `send_tools_list_changed` method to send the specific notification

### 3. Integrate with tool list changes
- Modify `discover_tools` method in `MCPClient` to trigger notifications when tool lists change
- Add logic to detect when tools are added/removed and send appropriate notifications
- Consider caching previous tool lists to detect changes

### 4. Update the MCPServer trait
- Add method to send list changed notifications to connected clients
- This allows servers to proactively notify about tool changes

The notification format according to MCP spec:
```json
{
  "jsonrpc": "2.0", 
  "method": "notifications/tools/list_changed"
}
```

This implementation will make the llama-agent MCP implementation compliant with the latest MCP specification for dynamic tool discovery.