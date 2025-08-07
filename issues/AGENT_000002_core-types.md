# Core Types and Error Handling

Refer to ./specifications/index.md

## Objective
Implement the fundamental types and error handling infrastructure for the agent system.

## Tasks
- [ ] Create `types.rs` with core data structures (Message, MessageRole, Session, etc.)
- [ ] Implement comprehensive error types in dedicated error module
- [ ] Add proper serialization/deserialization for all types
- [ ] Use ULID instead of UUID for session IDs (per coding standards)
- [ ] Add proper timestamp handling with SystemTime
- [ ] Create basic trait definitions (AgentAPI as placeholder)

## Key Types to Implement
- Message with role, content, tool metadata, timestamp
- MessageRole enum (System, User, Assistant, Tool)
- Session with ID, messages, MCP servers, tools, timestamps
- GenerationRequest and GenerationResponse
- ToolDefinition, ToolCall, ToolResult
- StreamChunk for streaming responses
- FinishReason enum

## Error Handling
- AgentError as main error type with variants for:
  - Model errors
  - Queue errors  
  - Session errors
  - MCP errors
  - Template errors
  - Timeout and capacity errors
- Use thiserror for derive macros
- Proper error chain propagation

## Acceptance Criteria
- All core types compile and serialize properly
- Error types provide meaningful error messages
- ULID is used for session IDs instead of UUID
- Types match the specification exactly
- Proper documentation for all public types