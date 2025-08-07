# Agent Server Core Implementation

Refer to ./specifications/index.md

## Objective
Implement the main AgentServer struct that ties together all components and provides the AgentAPI interface.

## Tasks
- [ ] Create `agent.rs` with AgentServer struct
- [ ] Implement AgentAPI trait with all required methods
- [ ] Integrate ModelManager, RequestQueue, SessionManager, MCPClient
- [ ] Add server initialization and configuration
- [ ] Implement batch and streaming generation methods
- [ ] Add session management API methods
- [ ] Integrate tool discovery and execution
- [ ] Add health checking and status reporting

## Core Integration
- Combine all components into cohesive AgentServer
- Manage component lifecycle and dependencies
- Coordinate between model, queue, sessions, and MCP client
- Handle cross-component error propagation
- Ensure proper resource cleanup on shutdown

## AgentAPI Implementation
- `initialize(config: AgentConfig) -> Result<Self, AgentError>`
- `generate(request: GenerationRequest) -> Result<GenerationResponse, AgentError>`
- `generate_stream(request) -> Result<impl Stream<StreamChunk>, AgentError>`
- `create_session() -> Result<Session, AgentError>`
- `get_session(session_id: &str) -> Result<Option<Session>, AgentError>`
- `update_session(session: Session) -> Result<(), AgentError>`
- `discover_tools(session: &mut Session) -> Result<(), AgentError>`
- `execute_tool(tool_call: ToolCall, session: &Session) -> Result<ToolResult, AgentError>`
- `health() -> Result<HealthStatus, AgentError>`
- `mcp_client() -> &MCPClient`

## Request Processing Flow
- Receive generation request with session
- Render session using chat template engine
- Submit to request queue for model inference
- Process response and handle tool calls
- Update session with results
- Return final response

## Acceptance Criteria
- AgentServer initializes all components correctly
- All AgentAPI methods are implemented and working
- Component integration handles errors properly
- Request processing flow works end-to-end
- Tool discovery and execution integrate seamlessly
- Health checks provide meaningful status information
- Proper cleanup on shutdown