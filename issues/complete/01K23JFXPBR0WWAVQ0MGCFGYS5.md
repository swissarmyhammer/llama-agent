
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
## Implementation Analysis

After reviewing the existing code, I found that the MCP prompts support is **already substantially implemented** but the actual protocol communication is missing. Here's the current status:

### ✅ Already Complete:
- **Types**: All prompt-related types are defined (`PromptDefinition`, `GetPromptResult`, `PromptMessage`, etc.)
- **MCP Trait**: Methods `list_prompts()` and `get_prompt()` are defined in `MCPServer` trait
- **Client Support**: `MCPClient` has full prompt discovery, caching, and execution methods
- **Session Integration**: `Session` struct includes `available_prompts` field
- **Caching System**: Prompt-to-server mapping and change detection are implemented
- **List Changed Notifications**: `notify_prompts_list_changed()` methods are implemented
- **Capabilities**: MCP initialization advertises prompts capability with `listChanged: true`
- **Tests**: Comprehensive test coverage for prompt functionality exists

### ❌ Missing Implementation:
The only missing piece is the actual MCP protocol communication in `MCPServerImpl`:

1. **`list_prompts()` method** (lines 314-336): Currently returns empty `Vec::new()`, needs to send `prompts/list` request
2. **`get_prompt()` method** (lines 338-373): Currently returns placeholder data, needs to send `prompts/get` request

These methods need to use the existing `send_request()` method to communicate with MCP servers.

## Root Cause

The issue is in `mcp.rs` lines 324-328 and 355-365 where the methods have TODO-like comments indicating they need actual MCP protocol requests instead of returning empty/placeholder data.

## Implementation Plan

1. **Implement `list_prompts()`**: Send actual `"prompts/list"` request using existing `send_request()` infrastructure
2. **Implement `get_prompt()`**: Send actual `"prompts/get"` request with prompt name and arguments
3. **Handle mutable access**: Both methods need mutable self to send requests, similar to how tools were initially structured
4. **Test the implementation**: Verify prompts work end-to-end with MCP servers

The architecture is solid - we just need to complete the protocol communication layer.
## ✅ Implementation Complete

The MCP prompts support has been successfully implemented! Here's a summary of what was accomplished:

### Changes Made:

1. **Fixed Trait Signatures**: Updated the `MCPServer` trait methods to use `&mut self` for methods that need to send MCP requests:
   - `list_tools(&mut self)` 
   - `call_tool(&mut self, ...)`
   - `list_prompts(&mut self)`
   - `get_prompt(&mut self, ...)`

2. **Implemented `list_prompts()`** (lines 316-388):
   - Sends actual `"prompts/list"` MCP request 
   - Parses response JSON to extract prompt definitions
   - Handles prompt arguments with name, description, and required fields
   - Creates `PromptDefinition` objects with server name mapping

3. **Implemented `get_prompt()`** (lines 391-564):
   - Sends `"prompts/get"` request with prompt name and optional arguments
   - Parses complex response format with message array
   - Supports all MCP prompt content types: text, image, resource
   - Handles legacy format fallback
   - Creates proper `GetPromptResult` with messages

4. **Updated Client References**: Modified all client code to use mutable server references for calling the updated methods

5. **Fixed Test Implementation**: Updated `MockMCPServer` trait implementations to match new signatures

### Architecture Preserved:

- ✅ All existing caching and change detection functionality maintained
- ✅ List changed notifications work correctly  
- ✅ Session integration with `available_prompts` field functional
- ✅ Error handling and logging consistent with existing patterns
- ✅ Full test coverage maintained (17 MCP tests passing)

### Verification:

- ✅ All tests pass (84/84)
- ✅ Code builds successfully 
- ✅ Clippy check clean (only pre-existing warnings unrelated to changes)
- ✅ Code properly formatted

The implementation follows the MCP specification exactly and integrates seamlessly with the existing codebase architecture. MCP servers can now expose prompts that will be discovered, cached, and available for use through the client.