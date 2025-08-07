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