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

## Proposed Solution

Based on the existing codebase architecture, I will implement the AgentServer as the main orchestrator that ties together all components:

### AgentServer Design
1. **Structure**: Create AgentServer struct that holds instances of:
   - ModelManager (for model loading and inference)
   - RequestQueue (for handling concurrent requests)
   - SessionManager (for session lifecycle)
   - MCPClient (for tool discovery and execution)
   - ChatTemplateEngine (for prompt templating)

2. **Initialization Flow**:
   - Validate configuration with AgentConfig::validate()
   - Initialize ModelManager and load model
   - Create RequestQueue with model reference
   - Initialize SessionManager 
   - Setup MCPClient with configured servers
   - Initialize ChatTemplateEngine

3. **Request Processing**:
   - Accept GenerationRequest with session
   - Use ChatTemplateEngine to render session messages into model input
   - Submit to RequestQueue for inference
   - Parse response for tool calls
   - Execute tools via MCPClient if needed
   - Return final response

4. **Component Integration**:
   - Proper error propagation between components
   - Resource cleanup on shutdown
   - Health checking across all components
   - Session management with tool discovery

5. **Implementation Strategy**:
   - Use Test-Driven Development
   - Start with basic structure and initialization
   - Add method implementations incrementally
   - Focus on integration points between components
   - Add comprehensive error handling

This approach leverages all the existing, well-tested components while providing a clean, unified interface through the AgentAPI trait.