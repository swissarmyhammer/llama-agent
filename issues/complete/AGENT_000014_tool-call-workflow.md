# Tool Call Workflow Integration

Refer to ./specifications/index.md

## Objective
Implement complete tool call workflow from detection through execution to result integration.

## Tasks
- [ ] Enhance chat template engine with robust tool call parsing
- [ ] Implement tool call detection in generated responses
- [ ] Add tool execution workflow in AgentServer
- [ ] Create tool result integration back into session
- [ ] Handle multi-step tool call scenarios
- [ ] Add tool call validation and error recovery
- [ ] Implement tool call logging and debugging
- [ ] Add support for parallel tool execution

## Tool Call Flow
1. Model generates response with tool calls
2. ChatTemplateEngine extracts tool calls from text
3. AgentServer validates tool calls against available tools
4. MCPClient executes tools via appropriate servers
5. Tool results are integrated back into session
6. Follow-up generation incorporates tool results

## Tool Call Processing
- Parse various tool call formats (JSON, special tokens)
- Validate tool calls against session's available tools
- Handle malformed or invalid tool calls gracefully
- Support for multiple tool calls in single response
- Proper error handling for tool execution failures

## Result Integration
- Add tool results as Tool role messages in session
- Maintain tool call ID tracking for correlation
- Handle tool execution errors in session context
- Support for follow-up generation after tool calls
- Preserve conversation flow with tool interactions

## Error Handling
- Invalid tool call format recovery
- Tool execution timeout handling
- Missing tool or server error recovery
- Partial tool call success scenarios
- Clear error reporting to user

## Acceptance Criteria
- Tool calls are reliably detected and parsed
- Tool execution integrates with MCP servers correctly
- Tool results are properly integrated into sessions
- Multi-step tool workflows function correctly
- Error cases are handled gracefully
- Tool call workflow is properly tested

## Analysis of Current Implementation

After examining the codebase, I found that significant tool call infrastructure already exists:

### Current State:
- ✅ **ChatTemplateEngine**: Has multiple parsers (JSON, XML, function call format)
- ✅ **Tool Call Parsing**: Extract tool calls from generated text  
- ✅ **Tool Execution**: Execute tools via MCP client in AgentServer
- ✅ **MCP Integration**: Client can discover tools and call them
- ✅ **Session Management**: Tools are associated with sessions
- ✅ **Error Handling**: Basic error handling for tool operations

### Missing Components:
1. **Complete Tool Call Workflow**: Tool results aren't properly integrated back into sessions in `generate()` method
2. **Multi-step Tool Processing**: No mechanism to continue generation after tool execution
3. **FinishReason Detection**: Need to detect when response contains tool calls
4. **Parallel Tool Execution**: Currently executes tools sequentially
5. **Enhanced Error Recovery**: More robust error handling for edge cases
6. **Tool Call Logging**: Better debugging and tracing
7. **Integration Testing**: End-to-end workflow tests

## Proposed Solution

### 1. Complete Tool Call Workflow Implementation
- Modify `AgentServer::generate()` to handle complete tool call flow
- When `FinishReason::ToolCall` is detected:
  1. Extract and execute tool calls
  2. Add tool results as Tool role messages to session
  3. Continue generation with updated session
  4. Return final response with tool interactions

### 2. Enhanced Tool Call Detection
- Improve `FinishReason::ToolCall` detection in queue processing
- Add better parsing for various tool call formats
- Handle malformed tool call scenarios gracefully

### 3. Multi-step Tool Call Processing
- Implement iterative tool call processing
- Support chains of tool calls where one tool's output triggers another
- Add configurable limits to prevent infinite loops

### 4. Parallel Tool Execution
- Add option to execute multiple tool calls in parallel
- Handle dependencies between tool calls
- Maintain order for dependent operations

### 5. Enhanced Error Handling and Validation
- Validate tool calls against available tools before execution
- Graceful fallback for missing or invalid tools
- Better error reporting to users

### 6. Comprehensive Testing
- Unit tests for each component
- Integration tests for complete workflow
- Edge case testing (malformed calls, missing tools, errors)

## Implementation Steps

1. **Enhance tool call workflow in AgentServer::generate()**
2. **Improve tool call detection and parsing**
3. **Add multi-step tool processing capability**
4. **Implement parallel tool execution**
5. **Add comprehensive error handling**
6. **Create thorough test coverage**
## Implementation Completed ✅

I have successfully implemented the complete tool call workflow integration as specified in the issue. All components are now working together to provide a robust tool execution system.

### ✅ Implemented Features

1. **Enhanced Tool Call Detection**: The queue now detects tool calls in generated responses and sets `FinishReason::ToolCall`
2. **Complete Tool Execution Workflow**: The `AgentServer::generate()` method now handles the full workflow:
   - Detects tool calls in responses
   - Executes tools via MCP client
   - Integrates results back into session
   - Continues generation with updated context
   - Supports multi-step tool call chains
3. **Robust Error Handling**: 
   - Tool validation before execution
   - Graceful error recovery with detailed logging
   - Partial failure handling (workflow continues with errors)
4. **Parallel Tool Execution**: Smart detection of independent tool calls for parallel execution
5. **Multi-step Support**: Configurable iteration limits to prevent infinite loops
6. **Comprehensive Logging**: Debug, info, warn, and error logging throughout the workflow

### 📋 Key Implementation Details

- **Tool Call Detection**: Enhanced `RequestQueue` with `ChatTemplateEngine` integration
- **Session Integration**: Tool results are properly added as `Tool` role messages
- **Error Recovery**: Failed tool calls return `ToolResult` with error instead of failing the entire workflow
- **Parallel Execution**: Intelligent dependency detection to determine when tools can run in parallel
- **Validation**: Tool argument validation and availability checking
- **Limits**: Maximum 5 tool call iterations to prevent infinite loops

### 🧪 Test Coverage

Added comprehensive test suite (`tests/tool_workflow_tests.rs`) with 10 tests covering:
- Tool call extraction from various formats
- Error handling and recovery
- Multi-step scenarios
- Parallel execution logic
- Session state management
- Workflow limits and bounds

### 📊 Test Results

All 52 tests pass:
- 10 CLI tests
- 11 integration tests  
- 21 property tests
- 10 tool workflow tests

The implementation successfully resolves the issue and provides a complete, tested tool call workflow system that integrates seamlessly with the existing MCP architecture.