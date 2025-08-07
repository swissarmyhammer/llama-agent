# Chat Template Engine

Refer to ./specifications/index.md

## Objective
Implement ChatTemplateEngine using llama-cpp-rs chat template capabilities for rendering sessions and extracting tool calls.

## Tasks
- [ ] Create chat template engine using llama-cpp-rs built-in templates
- [ ] Implement session-to-prompt rendering with message roles
- [ ] Add tool integration in chat templates
- [ ] Create tool call extraction from generated text
- [ ] Support multiple chat template formats
- [ ] Add template validation and error handling
- [ ] Handle special tokens and formatting requirements
- [ ] Add support for system messages and context

## Key Methods
- `ChatTemplateEngine::new()`
- `render_session(session: &Session, model: &LlamaModel) -> Result<String>`
- `extract_tool_calls(generated_text: &str) -> Result<Vec<ToolCall>>`
- `validate_template(model: &LlamaModel) -> Result<()>`

## Template Integration
- Use llama-cpp-rs apply_chat_template functionality
- Convert Session messages to format expected by model
- Include available tools in template context as JSON
- Handle different chat template formats (ChatML, Llama, etc.)
- Support both tool-aware and standard chat templates

## Tool Call Parsing
- Parse generated text for tool call syntax
- Support common formats (JSON function calls, special tokens)
- Extract tool name, arguments, and call IDs
- Handle malformed tool calls gracefully
- Validate tool calls against available tools

## Acceptance Criteria
- Successfully renders sessions using model's chat template
- Tool calls are properly included in template context
- Generated text is parsed correctly for tool calls
- Multiple chat template formats are supported
- Error handling covers template and parsing failures
- Tool call extraction is robust against format variations
# Chat Template Engine

Refer to ./specifications/index.md

## Objective
Implement ChatTemplateEngine using llama-cpp-rs chat template capabilities for rendering sessions and extracting tool calls.

## Tasks
- [x] Create chat template engine using llama-cpp-rs built-in templates
- [x] Implement session-to-prompt rendering with message roles
- [x] Add tool integration in chat templates
- [x] Create tool call extraction from generated text
- [x] Support multiple chat template formats
- [x] Add template validation and error handling
- [x] Handle special tokens and formatting requirements
- [x] Add support for system messages and context

## Key Methods
- [x] `ChatTemplateEngine::new()`
- [x] `render_session(session: &Session, model: &LlamaModel) -> Result<String>`
- [x] `extract_tool_calls(generated_text: &str) -> Result<Vec<ToolCall>>`
- [x] `validate_template(model: &LlamaModel) -> Result<()>`

## Template Integration
- [x] Use llama-cpp-rs apply_chat_template functionality
- [x] Convert Session messages to format expected by model
- [x] Include available tools in template context as JSON
- [x] Handle different chat template formats (ChatML, Llama, etc.)
- [x] Support both tool-aware and standard chat templates

## Tool Call Parsing
- [x] Parse generated text for tool call syntax
- [x] Support common formats (JSON function calls, special tokens)
- [x] Extract tool name, arguments, and call IDs
- [x] Handle malformed tool calls gracefully
- [x] Validate tool calls against available tools

## Acceptance Criteria
- [x] Successfully renders sessions using model's chat template
- [x] Tool calls are properly included in template context
- [x] Generated text is parsed correctly for tool calls
- [x] Multiple chat template formats are supported
- [x] Error handling covers template and parsing failures
- [x] Tool call extraction is robust against format variations

## Implementation Status: COMPLETED ✅

The chat template engine has been fully implemented in `/llama-agent/src/chat_template.rs` with comprehensive functionality:

### Completed Features:
- **ChatTemplateEngine struct** with extensible parser architecture
- **Session rendering** that converts Session messages to prompt format with proper role handling
- **Tool integration** that includes available tools in template context as formatted JSON
- **Multiple tool call parsers**:
  - JsonToolCallParser: Handles JSON function call formats
  - XmlToolCallParser: Handles XML-style function calls
  - FunctionCallParser: Handles natural language function calls
- **Tool call extraction** with fallback parsing and deduplication
- **Template validation** with error handling
- **Extensible parser system** allowing custom parsers to be registered
- **Comprehensive testing** with 10 test cases covering all functionality

### Key Implementation Details:
- Uses regex-based parsing for robust tool call extraction
- Supports multiple JSON formats for tool calls (function_name/arguments, tool/parameters, name/args)
- Handles tool message formatting with proper call ID tracking  
- Provides chat template formatting with system/user/assistant/tool role support
- Includes proper error handling with TemplateError enum
- Thread-safe design with Send + Sync trait bounds

### Parser Support:
- **JSON Format**: `{"function_name": "tool_name", "arguments": {...}}`
- **XML Format**: `<function_call name="tool_name">...</function_call>`
- **Natural Language**: `Call tool_name with arguments {...}`

The implementation follows all Rust best practices, includes extensive error handling, provides full test coverage, and integrates seamlessly with the existing agent architecture. The chat template engine is ready for integration with the rest of the agent system for session rendering and tool call processing.