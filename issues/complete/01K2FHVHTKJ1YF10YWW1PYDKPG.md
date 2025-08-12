`cargo run --example basic_usage ` is attempting tool calls, but is not doing so correctly

We need example to tracing::debug! each message as we go -- only the last repsonse was shown, which makes this impossible to debug

## Proposed Solution

After analyzing the code, the issue is that while there is some debug logging in the tool call workflow, there isn't comprehensive tracing for each message and step in the process. The user can only see the final result but not the intermediate steps that would help debug tool call issues.

### Key areas that need enhanced debug logging:

1. **Message extraction and parsing**: `chat_template.extract_tool_calls()` result details
2. **Each individual tool call processing**: Log the tool call arguments and execution details 
3. **Tool result processing**: Log each tool result as it's added back to the session
4. **Session state updates**: Log message additions during the tool call workflow

### Implementation steps:

1. Add debug logging in `process_tool_calls()` to show extracted tool calls with their arguments
2. Add debug logging in the tool execution loop to show each tool call being processed
3. Add debug logging when tool results are added back to the session as messages
4. Add debug logging in the main generation loop when tool call workflow continues

This will provide complete visibility into the tool call workflow for debugging purposes.