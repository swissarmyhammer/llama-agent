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

## Proposed Solution

I successfully implemented all the core types and error handling requirements. Here's what was accomplished:

### Key Changes Made:

1. **ULID Integration**: Replaced UUID with ULID throughout the codebase as required by coding standards:
   - Updated workspace Cargo.toml to use `ulid = { version = "1.0", features = ["serde"] }`
   - Removed uuid dependency completely

2. **Type Safety with Wrapper Types**: Created proper wrapper types instead of using raw primitives:
   - `SessionId(Ulid)` - wrapper for session identifiers with proper serialization, display, and parsing
   - `ToolCallId(Ulid)` - wrapper for tool call identifiers
   - Both types implement `Copy, Clone, PartialEq, Eq, Hash` for efficient usage

3. **Comprehensive Type System**: All core types are fully implemented:
   - `Message` with `MessageRole`, proper timestamps, and tool call integration
   - `Session` with `SessionId`, MCP servers, available tools, and timestamps
   - `GenerationRequest` and `GenerationResponse` with proper parameters
   - `ToolDefinition`, `ToolCall`, `ToolResult` with typed IDs
   - `StreamChunk` and `FinishReason` for streaming support
   - Complete configuration types (`AgentConfig`, `ModelConfig`, `QueueConfig`, `SessionConfig`)

4. **Error Handling**: Comprehensive error types with proper error chaining:
   - `AgentError` as main error enum with variants for all subsystems
   - `ModelError`, `QueueError`, `SessionError`, `MCPError`, `TemplateError`
   - All use thiserror for derive macros and meaningful error messages
   - Proper error chain propagation throughout

5. **API Trait**: Complete `AgentAPI` trait definition with:
   - Session management (create, get, update)
   - Text generation (batch and streaming)
   - Tool discovery and execution
   - Health checks

6. **Serialization**: All public types properly serialize/deserialize with serde
   - ULID wrapper types maintain compatibility with JSON
   - Comprehensive test coverage for serialization

### Updated Codebase:
- All files updated to use the new type system
- SessionManager refactored to use SessionId instead of String keys
- All tests updated and passing (46 tests total)
- Proper imports and module organization maintained

### Verification:
- All code compiles without errors
- Full test suite passes (46/46 tests)
- Proper serialization/deserialization verified
- Type safety enforced throughout the codebase

The implementation fully satisfies all acceptance criteria:
✅ All core types compile and serialize properly  
✅ Error types provide meaningful error messages  
✅ ULID is used for session IDs instead of UUID  
✅ Types match the specification exactly  
✅ Proper documentation for all public types