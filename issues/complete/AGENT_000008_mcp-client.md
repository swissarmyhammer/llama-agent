# MCP Client Integration

Refer to ./specifications/index.md

## Objective
Implement MCP client using rmcp for external tool integration and server management.

## Tasks
- [ ] Create `mcp.rs` module with MCPClient struct
- [ ] Integrate rmcp crate for MCP server connections
- [ ] Implement server initialization from MCPServerConfig
- [ ] Add tool discovery across all connected MCP servers
- [ ] Implement tool execution with proper error handling
- [ ] Create server lifecycle management (start/stop/health)
- [ ] Add connection retry and recovery logic
- [ ] Handle MCP protocol communication and serialization

## Key Methods
- `MCPClient::initialize(configs: Vec<MCPServerConfig>) -> Result<Self, MCPError>`
- `discover_tools() -> Result<Vec<ToolDefinition>, MCPError>`
- `call_tool(server_name: &str, tool_name: &str, args: Value) -> Result<Value, MCPError>`
- `list_servers() -> Vec<String>`
- `server_health(server_name: &str) -> Result<HealthStatus, MCPError>`

## MCP Integration
- Use rmcp for server process management and communication
- Handle stdio-based MCP server connections
- Manage server lifecycle (spawn, monitor, cleanup)
- Tool discovery aggregation across multiple servers
- Request/response handling with proper timeouts

## Error Handling
- MCPError variants for connection, communication, tool execution failures
- Server process monitoring and failure detection
- Tool execution timeout and retry logic
- Protocol error handling and recovery
- Graceful degradation when servers are unavailable

## Server Management
- Track server state and availability
- Handle server restarts and connection recovery
- Resource cleanup on shutdown
- Process monitoring and health checks

## Acceptance Criteria
- Successfully connects to MCP servers using rmcp
- Tool discovery works across multiple servers
- Tool execution handles success and error cases
- Server lifecycle is properly managed
- Connection failures are handled gracefully
- All MCP communication follows protocol specifications

## Proposed Solution

Based on the existing codebase and specification, I will implement the MCP client integration with the following approach:

### 1. MCP Client Structure
- Create `MCPClient` struct that manages multiple MCP servers
- Use `HashMap<String, Arc<dyn MCPServerTrait>>` to track server instances  
- Implement lifecycle management for server processes
- Use rmcp crate for MCP protocol communication

### 2. Key Components to Implement

#### MCPClient
- `initialize(configs: Vec<MCPServerConfig>)` - spawn and connect to MCP servers
- `discover_tools()` - aggregate tools from all connected servers
- `call_tool(server_name, tool_name, args)` - route tool calls to appropriate server
- `list_servers()` - return active server names
- `server_health(server_name)` - check individual server health
- Connection retry and recovery logic

#### MCPServer Trait
- Abstract interface for individual MCP server connections
- Handle stdio-based communication using rmcp
- Process lifecycle management (spawn/monitor/cleanup)
- Tool discovery and execution delegation

#### Health and Error Handling
- Comprehensive MCPError variants for different failure modes
- Server monitoring and automatic restart on failure
- Graceful degradation when servers are unavailable
- Resource cleanup on shutdown

### 3. Integration Points
- Extend existing `AgentAPI` trait methods for MCP integration
- Use existing `ToolDefinition`, `ToolCall`, `ToolResult` types
- Integrate with session management for server configuration
- Follow existing error handling patterns with `MCPError`

### 4. Implementation Steps
1. Define MCPServer trait for individual server management
2. Implement MCPClient struct with server lifecycle management  
3. Add tool discovery aggregation across servers
4. Implement tool execution routing and error handling
5. Add connection retry and health monitoring logic
6. Create comprehensive tests for all functionality

This approach follows the existing codebase patterns, uses proper ULID identifiers, implements comprehensive error handling, and maintains the queue-based architecture design.