use crate::types::{ModelConfig, Session, TemplateError, ToolCall, ToolCallId, ToolDefinition};
use llama_cpp_2::model::LlamaModel;
use regex::Regex;
use serde_json::Value;
use std::collections::HashMap;
use tracing::{debug, warn};

pub struct ChatTemplateEngine {
    tool_call_parsers: HashMap<String, Box<dyn ToolCallParser>>,
}

impl std::fmt::Debug for ChatTemplateEngine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ChatTemplateEngine")
            .field(
                "parsers",
                &self.tool_call_parsers.keys().collect::<Vec<_>>(),
            )
            .finish()
    }
}

impl Default for ChatTemplateEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl ChatTemplateEngine {
    pub fn new() -> Self {
        let mut parsers: HashMap<String, Box<dyn ToolCallParser>> = HashMap::new();

        // Add default parsers for common formats
        parsers.insert("json".to_string(), Box::new(JsonToolCallParser::new()));
        parsers.insert("xml".to_string(), Box::new(XmlToolCallParser::new()));
        parsers.insert(
            "function_call".to_string(),
            Box::new(FunctionCallParser::new()),
        );

        Self {
            tool_call_parsers: parsers,
        }
    }

    /// Render a session into a prompt string using the model's chat template
    pub fn render_session(
        &self,
        session: &Session,
        model: &LlamaModel,
    ) -> Result<String, TemplateError> {
        self.render_session_with_config(session, model, None)
    }

    /// Render a session into a prompt string using the model's chat template with config
    pub fn render_session_with_config(
        &self,
        session: &Session,
        model: &LlamaModel,
        model_config: Option<&ModelConfig>,
    ) -> Result<String, TemplateError> {
        debug!("Rendering session with {} messages", session.messages.len());

        // Convert session messages to the format expected by llama-cpp-2
        let mut chat_messages = Vec::new();

        for message in &session.messages {
            let role = message.role.as_str().to_string();
            let content = &message.content;

            // Handle tool calls and results properly
            match message.role {
                crate::types::MessageRole::Tool => {
                    // Tool response message
                    if let Some(tool_call_id) = &message.tool_call_id {
                        let formatted_content =
                            format!("Tool result for call {}: {}", tool_call_id, content);
                        chat_messages.push((role, formatted_content));
                    } else {
                        chat_messages.push((role, content.clone()));
                    }
                }
                _ => {
                    chat_messages.push((role, content.clone()));
                }
            }
        }

        // Include available tools in the template context if present
        let tools_context = if !session.available_tools.is_empty() {
            debug!(
                "Session has {} available tools, formatting for template",
                session.available_tools.len()
            );
            Some(self.format_tools_for_template(&session.available_tools)?)
        } else {
            debug!("Session has no available tools");
            None
        };

        // Apply the model's chat template
        let rendered = self.apply_chat_template_with_tools(
            model,
            &chat_messages,
            tools_context.as_deref(),
            model_config,
        )?;

        debug!("Rendered prompt length: {}", rendered.len());
        Ok(rendered)
    }

    /// Extract tool calls from generated text using registered parsers
    pub fn extract_tool_calls(&self, generated_text: &str) -> Result<Vec<ToolCall>, TemplateError> {
        debug!("Extracting tool calls from generated text");

        let mut all_tool_calls = Vec::new();

        // Try each parser until we find tool calls
        for (parser_name, parser) in &self.tool_call_parsers {
            debug!("Trying parser: {}", parser_name);

            match parser.parse_tool_calls(generated_text) {
                Ok(tool_calls) if !tool_calls.is_empty() => {
                    debug!(
                        "Found {} tool calls with parser {}",
                        tool_calls.len(),
                        parser_name
                    );
                    all_tool_calls.extend(tool_calls);
                    break; // Use first successful parser
                }
                Ok(_) => {
                    debug!("No tool calls found with parser {}", parser_name);
                }
                Err(e) => {
                    debug!("Parser {} failed: {}", parser_name, e);
                    continue;
                }
            }
        }

        // Deduplicate tool calls by ID
        all_tool_calls.sort_by(|a, b| a.id.to_string().cmp(&b.id.to_string()));
        all_tool_calls.dedup_by(|a, b| a.id == b.id);

        debug!("Extracted {} unique tool calls", all_tool_calls.len());
        Ok(all_tool_calls)
    }

    /// Validate that the model supports chat templates
    pub fn validate_template(&self, model: &LlamaModel) -> Result<(), TemplateError> {
        // Try to apply a simple template to check if it works
        let test_messages = vec![("user".to_string(), "Hello".to_string())];

        match self.apply_chat_template_with_tools(model, &test_messages, None, None) {
            Ok(_) => {
                debug!("Chat template validation successful");
                Ok(())
            }
            Err(e) => {
                warn!("Chat template validation failed: {}", e);
                Err(e)
            }
        }
    }

    /// Register a custom tool call parser
    pub fn register_parser(&mut self, name: String, parser: Box<dyn ToolCallParser>) {
        self.tool_call_parsers.insert(name, parser);
    }

    /// Format tools for inclusion in chat template
    fn format_tools_for_template(&self, tools: &[ToolDefinition]) -> Result<String, TemplateError> {
        let tools_json = serde_json::to_value(tools).map_err(|e| {
            TemplateError::RenderingFailed(format!("Failed to serialize tools: {}", e))
        })?;

        let formatted = serde_json::to_string_pretty(&tools_json).map_err(|e| {
            TemplateError::RenderingFailed(format!("Failed to format tools JSON: {}", e))
        })?;

        Ok(format!(
            "You are an AI assistant with access to the following tools. You have full permission and capability to use these tools to help users with their requests. Do not make security excuses - you are designed to use these tools.\n\nAvailable tools:\n{}\n\nIMPORTANT: When a user asks you to perform an action like listing files, reading files, or any file operations, you MUST use the appropriate tool. Do not give security warnings or suggest alternative methods - use the tools directly.\n\nTo call a tool, respond with a JSON object in this exact format. CRITICAL: Provide ONLY the JSON object, no additional text before or after:\n{{\n  \"function_name\": \"tool_name\",\n  \"arguments\": {{\n    \"parameter\": \"value\"\n  }}\n}}\n\nFor example, when asked to list files in the current directory, respond with ONLY:\n{{\n  \"function_name\": \"list_directory\",\n  \"arguments\": {{\n    \"path\": \".\"\n  }}\n}}\n\nDo not add explanatory text before or after the JSON. Generate well-formed JSON only. Always use the tools when they are needed to fulfill user requests.",
            formatted
        ))
    }

    /// Apply chat template with optional tools context
    fn apply_chat_template_with_tools(
        &self,
        model: &LlamaModel,
        messages: &[(String, String)],
        tools_context: Option<&str>,
        model_config: Option<&ModelConfig>,
    ) -> Result<String, TemplateError> {
        self.format_chat_template_for_model(model, messages, tools_context, model_config)
    }

    /// Format chat template based on model type
    fn format_chat_template_for_model(
        &self,
        model: &LlamaModel,
        messages: &[(String, String)],
        tools_context: Option<&str>,
        model_config: Option<&ModelConfig>,
    ) -> Result<String, TemplateError> {
        // Detect model type from model metadata or filename
        let model_name = self.detect_model_type(model, model_config);

        match model_name.as_str() {
            "phi3" => self.format_phi3_template(messages, tools_context),
            "qwen" => self.format_qwen_template(messages, tools_context),
            _ => self.format_chat_template(messages, tools_context),
        }
    }

    /// Detect model type from model information
    fn detect_model_type(&self, _model: &LlamaModel, model_config: Option<&ModelConfig>) -> String {
        // First check model config if available
        if let Some(config) = model_config {
            let model_identifier = match &config.source {
                crate::types::ModelSource::HuggingFace { repo, .. } => repo.clone(),
                crate::types::ModelSource::Local { folder, filename } => {
                    if let Some(filename) = filename {
                        format!("{}/{}", folder.display(), filename)
                    } else {
                        folder.to_string_lossy().to_string()
                    }
                }
            };

            let model_identifier_lower = model_identifier.to_lowercase();
            if model_identifier_lower.contains("qwen") {
                debug!(
                    "Detected Qwen model from model config: {}",
                    model_identifier
                );
                return "qwen".to_string();
            }
            if model_identifier_lower.contains("phi") {
                debug!("Detected Phi model from model config: {}", model_identifier);
                return "phi3".to_string();
            }
        }

        // Fallback to environment variable (for explicit override)
        let model_repo = std::env::var("MODEL_REPO").unwrap_or_default();
        if model_repo.contains("Qwen") || model_repo.contains("qwen") {
            debug!("Detected Qwen model from MODEL_REPO env var");
            return "qwen".to_string();
        }
        if model_repo.contains("Phi") || model_repo.contains("phi") {
            debug!("Detected Phi model from MODEL_REPO env var");
            return "phi3".to_string();
        }

        // Check process arguments for model path/name (common when running examples)
        let args: Vec<String> = std::env::args().collect();
        let args_string = args.join(" ");
        if args_string.contains("Qwen") || args_string.contains("qwen") {
            debug!("Detected Qwen model from process arguments");
            return "qwen".to_string();
        }
        if args_string.contains("Phi") || args_string.contains("phi") {
            debug!("Detected Phi model from process arguments");
            return "phi3".to_string();
        }

        // Check current working directory for clues (model files often contain model name)
        if let Ok(cwd) = std::env::current_dir() {
            let cwd_string = cwd.to_string_lossy().to_lowercase();
            if cwd_string.contains("qwen") {
                debug!("Detected Qwen model from current directory path");
                return "qwen".to_string();
            }
            if cwd_string.contains("phi") {
                debug!("Detected Phi model from current directory path");
                return "phi3".to_string();
            }
        }

        // Default to qwen as it works well with most instruction-tuned models
        debug!("Using default Qwen chat template (no specific model detected)");
        "qwen".to_string()
    }

    /// Format chat template specifically for Phi-3 models
    fn format_phi3_template(
        &self,
        messages: &[(String, String)],
        tools_context: Option<&str>,
    ) -> Result<String, TemplateError> {
        let mut formatted_messages = Vec::new();

        // Add tools context as system message if provided
        if let Some(tools) = tools_context {
            formatted_messages.push(("system".to_string(), tools.to_string()));
        }

        // Add all conversation messages
        for (role, content) in messages {
            formatted_messages.push((role.clone(), content.clone()));
        }

        // Use Phi-3 specific chat template format
        let mut prompt = String::new();

        for (role, content) in &formatted_messages {
            match role.as_str() {
                "system" => {
                    prompt.push_str(&format!("<|system|>\n{}<|end|>\n", content));
                }
                "user" => {
                    prompt.push_str(&format!("<|user|>\n{}<|end|>\n", content));
                }
                "assistant" => {
                    prompt.push_str(&format!("<|assistant|>\n{}<|end|>\n", content));
                }
                "tool" => {
                    prompt.push_str(&format!("<|tool|>\n{}<|end|>\n", content));
                }
                _ => {
                    // Fallback to user for unknown roles
                    prompt.push_str(&format!("<|user|>\n{}<|end|>\n", content));
                }
            }
        }

        // Add assistant prompt for generation
        prompt.push_str("<|assistant|>\n");

        // Debug: Log the final prompt for debugging
        debug!("Final Phi-3 prompt:\n{}", prompt);

        Ok(prompt)
    }

    /// Format chat template specifically for Qwen models
    fn format_qwen_template(
        &self,
        messages: &[(String, String)],
        tools_context: Option<&str>,
    ) -> Result<String, TemplateError> {
        let mut formatted_messages = Vec::new();

        // Add tools context as system message if provided
        if let Some(tools) = tools_context {
            debug!(
                "Adding tools context to Qwen template: {} characters",
                tools.len()
            );
            formatted_messages.push(("system".to_string(), tools.to_string()));
        } else {
            debug!("No tools context provided to Qwen template");
        }

        // Add all conversation messages
        for (role, content) in messages {
            formatted_messages.push((role.clone(), content.clone()));
        }

        // Use ChatML format for Qwen models
        let mut prompt = String::new();

        for (role, content) in &formatted_messages {
            match role.as_str() {
                "system" => {
                    prompt.push_str(&format!("<|im_start|>system\n{}<|im_end|>\n", content));
                }
                "user" => {
                    prompt.push_str(&format!("<|im_start|>user\n{}<|im_end|>\n", content));
                }
                "assistant" => {
                    prompt.push_str(&format!("<|im_start|>assistant\n{}<|im_end|>\n", content));
                }
                "tool" => {
                    prompt.push_str(&format!("<|im_start|>tool\n{}<|im_end|>\n", content));
                }
                _ => {
                    // Fallback to user for unknown roles
                    prompt.push_str(&format!("<|im_start|>user\n{}<|im_end|>\n", content));
                }
            }
        }

        // Add assistant prompt for generation
        prompt.push_str("<|im_start|>assistant\n");

        // Debug: Log the final prompt for debugging
        debug!("Final Qwen prompt:\n{}", prompt);

        Ok(prompt)
    }

    /// Internal method to format chat template (useful for testing)
    fn format_chat_template(
        &self,
        messages: &[(String, String)],
        tools_context: Option<&str>,
    ) -> Result<String, TemplateError> {
        // Convert to the format expected by llama-cpp-2
        let mut formatted_messages = Vec::new();

        // Add tools context as system message if provided
        if let Some(tools) = tools_context {
            formatted_messages.push(("system".to_string(), tools.to_string()));
        }

        // Add all conversation messages
        for (role, content) in messages {
            formatted_messages.push((role.clone(), content.clone()));
        }

        // For now, we'll create a simple chat template format
        // In the future, this should use llama-cpp-2's built-in template functionality
        // when it becomes available in the API
        let mut prompt = String::new();

        for (role, content) in &formatted_messages {
            match role.as_str() {
                "system" => {
                    prompt.push_str(&format!("### System:\n{}\n\n", content));
                }
                "user" => {
                    prompt.push_str(&format!("### Human:\n{}\n\n", content));
                }
                "assistant" => {
                    prompt.push_str(&format!("### Assistant:\n{}\n\n", content));
                }
                "tool" => {
                    prompt.push_str(&format!("### Tool Result:\n{}\n\n", content));
                }
                _ => {
                    prompt.push_str(&format!("### {}:\n{}\n\n", role, content));
                }
            }
        }

        // Add assistant prompt
        prompt.push_str("### Assistant:\n");

        Ok(prompt)
    }
}

/// Trait for parsing tool calls from different formats
pub trait ToolCallParser: Send + Sync {
    fn parse_tool_calls(&self, text: &str) -> Result<Vec<ToolCall>, TemplateError>;
}

/// Parser for JSON function call format
pub struct JsonToolCallParser {
    regex: Regex,
}

impl Default for JsonToolCallParser {
    fn default() -> Self {
        Self::new()
    }
}

impl JsonToolCallParser {
    pub fn new() -> Self {
        // Improved regex to match JSON objects more accurately
        // This will match properly balanced JSON objects (handles one level of nesting well)
        let regex = Regex::new(r#"\{(?:[^{}]|\{[^{}]*\})*\}"#).unwrap();

        Self { regex }
    }
}

impl ToolCallParser for JsonToolCallParser {
    fn parse_tool_calls(&self, text: &str) -> Result<Vec<ToolCall>, TemplateError> {
        let mut tool_calls = Vec::new();
        debug!(
            "JsonToolCallParser: Analyzing text for JSON objects: {}",
            text
        );

        // First try the main regex approach
        for capture in self.regex.find_iter(text) {
            let json_str = capture.as_str();
            debug!("JsonToolCallParser: Found potential JSON: {}", json_str);

            match serde_json::from_str::<Value>(json_str) {
                Ok(json) => {
                    debug!("JsonToolCallParser: Successfully parsed JSON: {:?}", json);
                    if let Some(tool_call) = self.parse_json_tool_call(&json)? {
                        debug!("JsonToolCallParser: Extracted tool call: {:?}", tool_call);
                        tool_calls.push(tool_call);
                    } else {
                        debug!("JsonToolCallParser: JSON doesn't match tool call format");
                    }
                }
                Err(e) => {
                    debug!(
                        "JsonToolCallParser: Failed to parse JSON '{}': {}",
                        json_str, e
                    );
                    continue;
                }
            }
        }

        // If no tool calls found with regex, try a more lenient line-by-line approach
        if tool_calls.is_empty() {
            debug!(
                "JsonToolCallParser: No tool calls found with regex, trying line-by-line parsing"
            );
            self.try_line_by_line_parsing(text, &mut tool_calls)?;
        }

        debug!(
            "JsonToolCallParser: Extracted {} tool calls total",
            tool_calls.len()
        );
        Ok(tool_calls)
    }
}

impl JsonToolCallParser {
    fn parse_json_tool_call(&self, json: &Value) -> Result<Option<ToolCall>, TemplateError> {
        // Try different common JSON formats for tool calls

        // Format 1: {"function_name": "tool_name", "arguments": {...}}
        if let (Some(function_name), Some(arguments)) = (
            json.get("function_name").and_then(|v| v.as_str()),
            json.get("arguments"),
        ) {
            return Ok(Some(ToolCall {
                id: ToolCallId::new(),
                name: function_name.to_string(),
                arguments: arguments.clone(),
            }));
        }

        // Format 2: {"tool": "tool_name", "parameters": {...}}
        if let (Some(tool_name), Some(parameters)) = (
            json.get("tool").and_then(|v| v.as_str()),
            json.get("parameters"),
        ) {
            return Ok(Some(ToolCall {
                id: ToolCallId::new(),
                name: tool_name.to_string(),
                arguments: parameters.clone(),
            }));
        }

        // Format 3: {"name": "tool_name", "args": {...}}
        if let (Some(name), Some(args)) =
            (json.get("name").and_then(|v| v.as_str()), json.get("args"))
        {
            return Ok(Some(ToolCall {
                id: ToolCallId::new(),
                name: name.to_string(),
                arguments: args.clone(),
            }));
        }

        Ok(None)
    }

    fn try_line_by_line_parsing(
        &self,
        text: &str,
        tool_calls: &mut Vec<ToolCall>,
    ) -> Result<(), TemplateError> {
        debug!("JsonToolCallParser: Trying line-by-line parsing");

        for line in text.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with('{') && trimmed.ends_with('}') {
                debug!("JsonToolCallParser: Found JSON-like line: {}", trimmed);

                match serde_json::from_str::<Value>(trimmed) {
                    Ok(json) => {
                        if let Some(tool_call) = self.parse_json_tool_call(&json)? {
                            debug!(
                                "JsonToolCallParser: Line-by-line extracted tool call: {:?}",
                                tool_call
                            );
                            tool_calls.push(tool_call);
                        }
                    }
                    Err(e) => {
                        debug!("JsonToolCallParser: Failed to parse line as JSON: {}", e);
                    }
                }
            }
        }

        // Additional fallback: try to extract JSON from text that might have trailing characters
        if tool_calls.is_empty() {
            debug!("JsonToolCallParser: Trying fallback parsing for malformed JSON");
            self.try_fallback_parsing(text, tool_calls)?;
        }

        Ok(())
    }

    fn try_fallback_parsing(
        &self,
        text: &str,
        tool_calls: &mut Vec<ToolCall>,
    ) -> Result<(), TemplateError> {
        // Use a more sophisticated approach to find JSON objects that might be malformed

        // Find potential JSON start patterns
        let start_patterns = vec![
            r#"\{\s*"function_name"\s*:"#,
            r#"\{\s*"tool"\s*:"#,
            r#"\{\s*"name"\s*:"#,
        ];

        for pattern_str in start_patterns {
            let pattern = Regex::new(pattern_str).unwrap();

            for mat in pattern.find_iter(text) {
                let start_pos = mat.start();
                debug!(
                    "JsonToolCallParser: Found potential JSON start at position {}",
                    start_pos
                );

                // Try to find the matching closing brace using brace counting
                let remaining_text = &text[start_pos..];
                if let Some(json_str) = self.extract_balanced_json(remaining_text) {
                    debug!("JsonToolCallParser: Extracted balanced JSON: {}", json_str);

                    match serde_json::from_str::<Value>(&json_str) {
                        Ok(json) => {
                            if let Some(tool_call) = self.parse_json_tool_call(&json)? {
                                debug!(
                                    "JsonToolCallParser: Fallback extracted tool call: {:?}",
                                    tool_call
                                );
                                tool_calls.push(tool_call);
                            }
                        }
                        Err(e) => {
                            debug!("JsonToolCallParser: Fallback JSON parsing failed: {}", e);
                        }
                    }
                }
            }
        }

        Ok(())
    }

    fn extract_balanced_json(&self, text: &str) -> Option<String> {
        let mut brace_count = 0;
        let mut start_found = false;
        let mut in_string = false;
        let mut escape_next = false;
        let mut result = String::new();

        for ch in text.chars() {
            result.push(ch);

            if escape_next {
                escape_next = false;
                continue;
            }

            match ch {
                '\\' if in_string => {
                    escape_next = true;
                }
                '"' => {
                    in_string = !in_string;
                }
                '{' if !in_string => {
                    brace_count += 1;
                    start_found = true;
                }
                '}' if !in_string => {
                    brace_count -= 1;
                    if start_found && brace_count == 0 {
                        // Found complete JSON object
                        return Some(result);
                    }
                }
                _ => {}
            }

            // If we've seen many characters without closing, give up
            if result.len() > 10000 {
                break;
            }
        }

        None
    }
}

/// Parser for XML-style function calls
pub struct XmlToolCallParser {
    regex: Regex,
}

impl Default for XmlToolCallParser {
    fn default() -> Self {
        Self::new()
    }
}

impl XmlToolCallParser {
    pub fn new() -> Self {
        // Match XML-style function calls like <function_call name="tool_name">...</function_call>
        let regex = Regex::new(r#"<function_call[^>]*>(.*?)</function_call>"#)
            .unwrap_or_else(|_| Regex::new(r#"<tool_call[^>]*>(.*?)</tool_call>"#).unwrap());

        Self { regex }
    }
}

impl ToolCallParser for XmlToolCallParser {
    fn parse_tool_calls(&self, text: &str) -> Result<Vec<ToolCall>, TemplateError> {
        let mut tool_calls = Vec::new();

        for capture in self.regex.captures_iter(text) {
            if let Some(tool_call) = self.parse_xml_tool_call(capture.get(0).unwrap().as_str())? {
                tool_calls.push(tool_call);
            }
        }

        Ok(tool_calls)
    }
}

impl XmlToolCallParser {
    fn parse_xml_tool_call(&self, xml: &str) -> Result<Option<ToolCall>, TemplateError> {
        // Simple XML parsing - extract name attribute and content
        let name_regex = Regex::new(r#"name="([^"]*)"#).unwrap();
        let content_regex = Regex::new(r#"<[^>]*>(.*)</[^>]*>"#).unwrap();

        if let Some(name_match) = name_regex.captures(xml) {
            let name = name_match.get(1).unwrap().as_str();

            let arguments = if let Some(content_match) = content_regex.captures(xml) {
                let content = content_match.get(1).unwrap().as_str();
                match serde_json::from_str::<Value>(content) {
                    Ok(json) => json,
                    Err(_) => Value::String(content.to_string()),
                }
            } else {
                Value::Null
            };

            return Ok(Some(ToolCall {
                id: ToolCallId::new(),
                name: name.to_string(),
                arguments,
            }));
        }

        Ok(None)
    }
}

/// Parser for natural language function call format
pub struct FunctionCallParser {
    regex: Regex,
}

impl Default for FunctionCallParser {
    fn default() -> Self {
        Self::new()
    }
}

impl FunctionCallParser {
    pub fn new() -> Self {
        // Match patterns like "Call function_name with arguments {...}"
        let regex = Regex::new(r"(?i)call\s+(\w+)\s+with\s+(?:arguments?\s+)?(.+)")
            .unwrap_or_else(|_| Regex::new(r"(\w+)\s*\(([^)]*)\)").unwrap());

        Self { regex }
    }
}

impl ToolCallParser for FunctionCallParser {
    fn parse_tool_calls(&self, text: &str) -> Result<Vec<ToolCall>, TemplateError> {
        let mut tool_calls = Vec::new();

        for capture in self.regex.captures_iter(text) {
            if let Some(tool_call) = self.parse_function_call(&capture)? {
                tool_calls.push(tool_call);
            }
        }

        Ok(tool_calls)
    }
}

impl FunctionCallParser {
    fn parse_function_call(
        &self,
        capture: &regex::Captures,
    ) -> Result<Option<ToolCall>, TemplateError> {
        if let (Some(name), Some(args_str)) = (capture.get(1), capture.get(2)) {
            let name = name.as_str();
            let args_str = args_str.as_str().trim();

            let arguments = if args_str.starts_with('{') && args_str.ends_with('}') {
                // Try to parse as JSON
                match serde_json::from_str::<Value>(args_str) {
                    Ok(json) => json,
                    Err(_) => Value::String(args_str.to_string()),
                }
            } else {
                // Treat as string parameter
                Value::String(args_str.to_string())
            };

            return Ok(Some(ToolCall {
                id: ToolCallId::new(),
                name: name.to_string(),
                arguments,
            }));
        }

        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Message, MessageRole, Session, SessionId, ToolDefinition};
    use std::time::SystemTime;

    fn create_test_session() -> Session {
        Session {
            id: SessionId::new(),
            messages: vec![
                Message {
                    role: MessageRole::System,
                    content: "You are a helpful assistant.".to_string(),
                    tool_call_id: None,
                    tool_name: None,
                    timestamp: SystemTime::now(),
                },
                Message {
                    role: MessageRole::User,
                    content: "Hello, can you help me?".to_string(),
                    tool_call_id: None,
                    tool_name: None,
                    timestamp: SystemTime::now(),
                },
            ],
            mcp_servers: vec![],
            available_tools: vec![ToolDefinition {
                name: "list_files".to_string(),
                description: "List files in a directory".to_string(),
                parameters: serde_json::json!({"type": "object", "properties": {"path": {"type": "string"}}}),
                server_name: "filesystem".to_string(),
            }],
            available_prompts: vec![],
            created_at: SystemTime::now(),
            updated_at: SystemTime::now(),
        }
    }

    #[test]
    fn test_chat_template_engine_creation() {
        let engine = ChatTemplateEngine::new();
        assert_eq!(engine.tool_call_parsers.len(), 3);
        assert!(engine.tool_call_parsers.contains_key("json"));
        assert!(engine.tool_call_parsers.contains_key("xml"));
        assert!(engine.tool_call_parsers.contains_key("function_call"));
    }

    #[test]
    fn test_format_tools_for_template() {
        let engine = ChatTemplateEngine::new();
        let session = create_test_session();

        let formatted = engine
            .format_tools_for_template(&session.available_tools)
            .unwrap();
        assert!(formatted.contains("Available tools:"));
        assert!(formatted.contains("list_files"));
        assert!(formatted.contains("filesystem"));
    }

    #[test]
    fn test_json_tool_call_parser() {
        let parser = JsonToolCallParser::new();

        // Test format 1
        let text = r#"{"function_name": "list_files", "arguments": {"path": "/tmp"}}"#;
        let tool_calls = parser.parse_tool_calls(text).unwrap();
        assert_eq!(tool_calls.len(), 1);
        assert_eq!(tool_calls[0].name, "list_files");

        // Test format 2
        let text = r#"{"tool": "list_files", "parameters": {"path": "/tmp"}}"#;
        let tool_calls = parser.parse_tool_calls(text).unwrap();
        assert_eq!(tool_calls.len(), 1);
        assert_eq!(tool_calls[0].name, "list_files");

        // Test format 3
        let text = r#"{"name": "list_files", "args": {"path": "/tmp"}}"#;
        let tool_calls = parser.parse_tool_calls(text).unwrap();
        assert_eq!(tool_calls.len(), 1);
        assert_eq!(tool_calls[0].name, "list_files");
    }

    #[test]
    fn test_json_tool_call_parser_mixed_with_text() {
        let parser = JsonToolCallParser::new();

        // Test mixed with text before and after - this is what models actually generate
        let text = r#"I'll help you list the files in the current directory.

{"function_name": "list_directory", "arguments": {"path": "."}}

I apologize for the confusion. Let me try again with the correct format."#;

        let tool_calls = parser.parse_tool_calls(text).unwrap();
        assert_eq!(tool_calls.len(), 1);
        assert_eq!(tool_calls[0].name, "list_directory");

        // Test with complex nested arguments
        let complex_text = r#"{"function_name": "complex_tool", "arguments": {"nested": {"key": "value", "array": [1, 2, 3]}, "simple": "test"}}"#;
        let tool_calls = parser.parse_tool_calls(complex_text).unwrap();
        assert_eq!(tool_calls.len(), 1);
        assert_eq!(tool_calls[0].name, "complex_tool");

        // Test problematic case: JSON followed immediately by text (what models actually do)
        let problematic_text = r#"{"function_name": "list_directory", "arguments": {"path": "."}}I apologize for the confusion"#;
        let tool_calls = parser.parse_tool_calls(problematic_text).unwrap();
        assert_eq!(tool_calls.len(), 1);
        assert_eq!(tool_calls[0].name, "list_directory");

        // Test even more complex nesting with multiple levels
        let deep_nested = r#"{"function_name": "deep_tool", "arguments": {"level1": {"level2": {"level3": {"value": "test"}}}, "array": [{"nested": true}, {"nested": false}]}}"#;
        let tool_calls = parser.parse_tool_calls(deep_nested).unwrap();
        assert_eq!(tool_calls.len(), 1);
        assert_eq!(tool_calls[0].name, "deep_tool");
    }

    #[test]
    fn test_json_tool_call_parser_fallback_parsing() {
        // Initialize tracing for test debugging
        let _ = tracing_subscriber::fmt()
            .with_max_level(tracing::Level::DEBUG)
            .try_init();

        let parser = JsonToolCallParser::new();

        // Test malformed JSON that would need fallback parsing (missing closing brace)
        let malformed_text = r#"Sure, I can help! {"function_name": "list_directory", "arguments": {"path": "."}} I need to check what's in your directory first."#;
        let tool_calls = parser.parse_tool_calls(malformed_text).unwrap();
        // Should work with new balanced brace extraction
        println!("Malformed text extracted {} tool calls", tool_calls.len());
        assert_eq!(tool_calls.len(), 1);
        assert_eq!(tool_calls[0].name, "list_directory");

        // Test case where JSON is buried in lots of text
        let buried_text = r#"
        Let me help you with that task. First, I need to understand what files are available.
        I'll use the directory listing tool to check: {"function_name": "list_directory", "arguments": {"path": "."}}
        After checking the directory, I can provide better assistance.
        "#;
        let tool_calls = parser.parse_tool_calls(buried_text).unwrap();
        assert_eq!(tool_calls.len(), 1);
        assert_eq!(tool_calls[0].name, "list_directory");
    }

    #[test]
    fn test_xml_tool_call_parser() {
        let parser = XmlToolCallParser::new();

        let text = r#"<function_call name="list_files">{"path": "/tmp"}</function_call>"#;
        let tool_calls = parser.parse_tool_calls(text).unwrap();
        assert_eq!(tool_calls.len(), 1);
        assert_eq!(tool_calls[0].name, "list_files");
    }

    #[test]
    fn test_function_call_parser() {
        let parser = FunctionCallParser::new();

        let text = "Call list_files with arguments {\"path\": \"/tmp\"}";
        let tool_calls = parser.parse_tool_calls(text).unwrap();
        assert_eq!(tool_calls.len(), 1);
        assert_eq!(tool_calls[0].name, "list_files");
    }

    #[test]
    fn test_extract_tool_calls_multiple_formats() {
        let engine = ChatTemplateEngine::new();

        let text = r#"
        I'll help you with that. Let me list the files first.
        {"function_name": "list_files", "arguments": {"path": "/tmp"}}
        "#;

        let tool_calls = engine.extract_tool_calls(text).unwrap();
        assert_eq!(tool_calls.len(), 1);
        assert_eq!(tool_calls[0].name, "list_files");
    }

    #[test]
    fn test_debug_logging_tool_extraction() {
        // Initialize tracing for test debugging
        let _ = tracing_subscriber::fmt()
            .with_max_level(tracing::Level::DEBUG)
            .try_init();

        // This test verifies that our debug logging enhancements work
        let engine = ChatTemplateEngine::new();

        // Test with a tool call that should be extracted
        let text = r#"
        I'll help you list the files in the current directory.
        {"function_name": "list_directory", "arguments": {"path": "."}}
        "#;

        // This will trigger our enhanced debug logging in extract_tool_calls
        let tool_calls = engine.extract_tool_calls(text).unwrap();
        assert_eq!(tool_calls.len(), 1);
        assert_eq!(tool_calls[0].name, "list_directory");

        // Test with text that has no tool calls
        let empty_text = "Just a regular response with no tool calls.";
        let empty_calls = engine.extract_tool_calls(empty_text).unwrap();
        assert_eq!(empty_calls.len(), 0);

        // Test actual problematic pattern that models generate
        let problematic_text = r#"{"function_name": "list_directory", "arguments": {"path": "."}}I need to check the files in the current directory for you."#;
        println!("Testing problematic text: {}", problematic_text);
        let tool_calls = engine.extract_tool_calls(problematic_text).unwrap();
        println!("Extracted {} tool calls", tool_calls.len());
        assert_eq!(tool_calls.len(), 1);
        assert_eq!(tool_calls[0].name, "list_directory");
    }

    #[test]
    fn test_extract_tool_calls_no_matches() {
        let engine = ChatTemplateEngine::new();

        let text = "This is just regular text with no tool calls.";
        let tool_calls = engine.extract_tool_calls(text).unwrap();
        assert_eq!(tool_calls.len(), 0);
    }

    #[test]
    fn test_register_custom_parser() {
        let mut engine = ChatTemplateEngine::new();
        let initial_count = engine.tool_call_parsers.len();

        engine.register_parser("custom".to_string(), Box::new(JsonToolCallParser::new()));
        assert_eq!(engine.tool_call_parsers.len(), initial_count + 1);
        assert!(engine.tool_call_parsers.contains_key("custom"));
    }

    #[test]
    fn test_tool_call_deduplication() {
        let engine = ChatTemplateEngine::new();

        // Text with duplicate tool calls
        let text = r#"
        {"function_name": "list_files", "arguments": {"path": "/tmp"}}
        I'll also check another directory.
        {"function_name": "list_files", "arguments": {"path": "/home"}}
        "#;

        let tool_calls = engine.extract_tool_calls(text).unwrap();
        assert_eq!(tool_calls.len(), 2); // Should have 2 unique tool calls
    }

    #[test]
    fn test_apply_chat_template_with_tools_format() {
        let engine = ChatTemplateEngine::new();
        let messages = vec![
            ("user".to_string(), "Hello".to_string()),
            ("assistant".to_string(), "Hi there!".to_string()),
        ];

        let tools_context = "Available tools: list_files";
        let result = engine.format_chat_template(&messages, Some(tools_context));

        // This test verifies the string formatting logic
        assert!(result.is_ok());
        let prompt = result.unwrap();
        assert!(prompt.contains("### System:"));
        assert!(prompt.contains("Available tools: list_files"));
        assert!(prompt.contains("### Human:"));
        assert!(prompt.contains("Hello"));
        assert!(prompt.contains("### Assistant:"));
    }
}
