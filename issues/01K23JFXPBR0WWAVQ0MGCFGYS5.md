
Add prompts support (https://modelcontextprotocol.io/specification/2025-06-18/server/prompts).

Keep track of prompts that are exposed by the mcp servers via out mcp client, similar to how we track tools.

Make sure to handle the list changed!

## Proposed Solution

I will implement MCP prompts support following the MCP specification at https://modelcontextprotocol.io/specification/2025-06-18/server/prompts.

### Implementation Steps:

1. **Add Prompt Types**: Create `PromptDefinition`, `PromptMessage`, `PromptArgument` and related types in `types.rs`

2. **Extend MCPServer trait**: Add methods for `list_prompts()` and `get_prompt()` similar to existing tool methods

3. **Implement Protocol Methods**: Add `prompts/list` and `prompts/get` request handling in `MCPServerImpl`

4. **Add Client Support**: Extend `MCPClient` with prompt discovery, caching, and change notifications similar to tools

5. **Update Session**: Add `available_prompts` field to `Session` struct to track discovered prompts

6. **Add Capabilities**: Update MCP initialization to advertise prompts capability with `listChanged` support

7. **Implement Caching**: Add prompt-to-server mapping cache and previous prompts cache for change detection

8. **Handle Notifications**: Support `notifications/prompts/list_changed` similar to tools

This mirrors the existing tool support architecture but for prompts, ensuring consistency and proper handling of the list changed notifications as requested in the issue.