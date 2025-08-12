`cargo run --example basic_usage`


is not successfull calling the tool. fix it.
## Proposed Solution

After investigating the failing `cargo run --example basic_usage` command, I've identified the root cause:

### Root Cause Analysis
1. The example is actually running successfully and initializing the AgentServer properly
2. The model is generating tool calls in JSON format, but the generated JSON is malformed
3. The model hits the maximum tool call iterations limit (5) because the JSON parsing is failing
4. The JsonToolCallParser expects proper JSON format but the model generates invalid JSON fragments

### Technical Issues Found
- The model generates text like: `{"function_name": "list_directory", "arguments": {"path": "."}}I apologize for the confusion...`
- The JSON is mixed with natural language text, making parsing fail
- The regex pattern in JsonToolCallParser is too simple to handle complex JSON with nested objects
- When JSON parsing fails, the system can't extract tool calls and continues generating until hitting the iteration limit

### Proposed Fix
1. Improve the JsonToolCallParser regex to better extract JSON objects from mixed text
2. Add better error handling and logging when tool call extraction fails
3. Modify the chat template to encourage cleaner JSON generation
4. Add fallback parsing methods for malformed JSON

This will fix the tool calling mechanism and allow the basic_usage example to work properly.