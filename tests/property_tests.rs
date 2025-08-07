mod common;
use llama_agent::types::*;
use proptest::prelude::*;
use std::time::Duration;

// Property-based test generators

prop_compose! {
    fn arb_session_id()(ulid in any::<u128>()) -> SessionId {
        SessionId::from_ulid(ulid::Ulid::from(ulid))
    }
}

prop_compose! {
    fn arb_tool_call_id()(ulid in any::<u128>()) -> ToolCallId {
        ToolCallId::from_ulid(ulid::Ulid::from(ulid))
    }
}

prop_compose! {
    fn arb_message_role()(role in 0..4u8) -> MessageRole {
        match role {
            0 => MessageRole::System,
            1 => MessageRole::User,
            2 => MessageRole::Assistant,
            _ => MessageRole::Tool,
        }
    }
}

prop_compose! {
    fn arb_message()(
        role in arb_message_role(),
        content in ".{1,1000}",
        tool_call_id in prop::option::of(arb_tool_call_id()),
        tool_name in prop::option::of("[a-zA-Z_][a-zA-Z0-9_]{0,49}")
    ) -> Message {
        Message {
            role,
            content,
            tool_call_id,
            tool_name,
            timestamp: std::time::SystemTime::now(),
        }
    }
}

prop_compose! {
    fn arb_tool_definition()(
        name in "[a-zA-Z_][a-zA-Z0-9_]{0,49}",
        description in ".{1,500}",
        server_name in "[a-zA-Z_][a-zA-Z0-9_-]{0,49}"
    ) -> ToolDefinition {
        ToolDefinition {
            name,
            description,
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "input": {
                        "type": "string",
                        "description": "Input parameter"
                    }
                }
            }),
            server_name,
        }
    }
}

prop_compose! {
    fn arb_mcp_server_config()(
        name in "[a-zA-Z_][a-zA-Z0-9_-]{1,49}",
        command in "[a-zA-Z_][a-zA-Z0-9_/-]{1,99}",
        args in prop::collection::vec(".{0,100}", 0..10),
        timeout_secs in prop::option::of(1u64..3600)
    ) -> MCPServerConfig {
        MCPServerConfig {
            name,
            command,
            args,
            timeout_secs,
        }
    }
}

prop_compose! {
    fn arb_model_config()(
        batch_size in 1u32..8192,
        use_hf_params in any::<bool>()
    ) -> ModelConfig {
        ModelConfig {
            source: ModelSource::Local {
                folder: std::path::PathBuf::from("/tmp"),
                filename: Some("test.gguf".to_string()),
            },
            batch_size,
            use_hf_params,
        }
    }
}

prop_compose! {
    fn arb_queue_config()(
        max_queue_size in 1usize..1000,
        request_timeout_secs in 1u64..300,
        worker_threads in 1usize..16
    ) -> QueueConfig {
        QueueConfig {
            max_queue_size,
            request_timeout: Duration::from_secs(request_timeout_secs),
            worker_threads,
        }
    }
}

prop_compose! {
    fn arb_session_config()(
        max_sessions in 1usize..10000,
        session_timeout_secs in 1u64..86400
    ) -> SessionConfig {
        SessionConfig {
            max_sessions,
            session_timeout: Duration::from_secs(session_timeout_secs),
        }
    }
}

prop_compose! {
    fn arb_agent_config()(
        model in arb_model_config(),
        queue_config in arb_queue_config(),
        mcp_servers in prop::collection::vec(arb_mcp_server_config(), 0..5),
        session_config in arb_session_config()
    ) -> AgentConfig {
        AgentConfig {
            model,
            queue_config,
            mcp_servers,
            session_config,
        }
    }
}

// Property-based tests

proptest! {
    #[test]
    fn test_session_id_roundtrip(session_id in arb_session_id()) {
        let string_repr = session_id.to_string();
        let parsed: SessionId = string_repr.parse().unwrap();
        prop_assert_eq!(session_id, parsed);
    }

    #[test]
    fn test_tool_call_id_roundtrip(tool_call_id in arb_tool_call_id()) {
        let string_repr = tool_call_id.to_string();
        let parsed: ToolCallId = string_repr.parse().unwrap();
        prop_assert_eq!(tool_call_id, parsed);
    }

    #[test]
    fn test_session_id_serialization(session_id in arb_session_id()) {
        let serialized = serde_json::to_string(&session_id).unwrap();
        let deserialized: SessionId = serde_json::from_str(&serialized).unwrap();
        prop_assert_eq!(session_id, deserialized);
    }

    #[test]
    fn test_tool_call_id_serialization(tool_call_id in arb_tool_call_id()) {
        let serialized = serde_json::to_string(&tool_call_id).unwrap();
        let deserialized: ToolCallId = serde_json::from_str(&serialized).unwrap();
        prop_assert_eq!(tool_call_id, deserialized);
    }

    #[test]
    fn test_message_role_str_conversion(role in arb_message_role()) {
        let role_str = role.as_str();
        prop_assert!(["system", "user", "assistant", "tool"].contains(&role_str));
    }

    #[test]
    fn test_message_serialization(message in arb_message()) {
        let serialized = serde_json::to_string(&message).unwrap();
        let deserialized: Message = serde_json::from_str(&serialized).unwrap();

        prop_assert_eq!(message.role.as_str(), deserialized.role.as_str());
        prop_assert_eq!(message.content, deserialized.content);
        prop_assert_eq!(message.tool_call_id, deserialized.tool_call_id);
        prop_assert_eq!(message.tool_name, deserialized.tool_name);
    }

    #[test]
    fn test_tool_definition_serialization(tool_def in arb_tool_definition()) {
        let serialized = serde_json::to_string(&tool_def).unwrap();
        let deserialized: ToolDefinition = serde_json::from_str(&serialized).unwrap();

        prop_assert_eq!(tool_def.name, deserialized.name);
        prop_assert_eq!(tool_def.description, deserialized.description);
        prop_assert_eq!(tool_def.server_name, deserialized.server_name);
    }

    #[test]
    fn test_mcp_server_config_validation(config in arb_mcp_server_config()) {
        // Generated configs should be valid since they follow the constraints
        prop_assert!(config.validate().is_ok());
    }

    #[test]
    fn test_model_config_validation(config in arb_model_config()) {
        // Generated configs should be valid since they follow the constraints
        let validation_result = config.validate();
        if config.batch_size == 0 || config.batch_size > 8192 {
            prop_assert!(validation_result.is_err());
        } else {
            // May fail due to path validation, but batch size should be valid
            match validation_result {
                Ok(_) => {},
                Err(ModelError::NotFound(_)) => {
                    // Expected for /tmp path that might not exist or have the file
                },
                Err(e) => {
                    prop_assert!(false, "Unexpected validation error: {:?}", e);
                }
            }
        }
    }

    #[test]
    fn test_queue_config_validation(config in arb_queue_config()) {
        // Generated configs should be valid since they follow the constraints
        prop_assert!(config.validate().is_ok());
    }

    #[test]
    fn test_session_config_validation(config in arb_session_config()) {
        // Generated configs should be valid since they follow the constraints
        prop_assert!(config.validate().is_ok());
    }

    #[test]
    fn test_agent_config_serialization(config in arb_agent_config()) {
        let serialized = serde_json::to_string(&config).unwrap();
        let deserialized: AgentConfig = serde_json::from_str(&serialized).unwrap();

        prop_assert_eq!(config.model.batch_size, deserialized.model.batch_size);
        prop_assert_eq!(config.model.use_hf_params, deserialized.model.use_hf_params);
        prop_assert_eq!(config.queue_config.max_queue_size, deserialized.queue_config.max_queue_size);
        prop_assert_eq!(config.queue_config.worker_threads, deserialized.queue_config.worker_threads);
        prop_assert_eq!(config.session_config.max_sessions, deserialized.session_config.max_sessions);
        prop_assert_eq!(config.mcp_servers.len(), deserialized.mcp_servers.len());
    }

    #[test]
    fn test_valid_batch_size(batch_size in 1u32..8192) {
        let config = ModelConfig {
            batch_size,
            ..Default::default()
        };

        let validation_result = config.validate();

        // Valid batch sizes should pass validation (other errors may occur due to model source)
        match validation_result {
            Ok(_) => {},
            Err(ModelError::NotFound(_)) => {
                // Expected for default HuggingFace source
            },
            Err(e) => prop_assert!(false, "Unexpected error for valid batch size: {:?}", e),
        }
    }
}

// Specific property tests for edge cases

proptest! {
    #[test]
    fn test_empty_content_message(role in arb_message_role()) {
        let message = Message {
            role,
            content: String::new(),
            tool_call_id: None,
            tool_name: None,
            timestamp: std::time::SystemTime::now(),
        };

        // Should be able to serialize/deserialize messages with empty content
        let serialized = serde_json::to_string(&message).unwrap();
        let deserialized: Message = serde_json::from_str(&serialized).unwrap();

        prop_assert_eq!(&message.content, &deserialized.content);
        prop_assert!(message.content.is_empty());
    }

    #[test]
    fn test_long_content_message(content in ".{1000,10000}") {
        let message = Message {
            role: MessageRole::User,
            content,
            tool_call_id: None,
            tool_name: None,
            timestamp: std::time::SystemTime::now(),
        };

        // Should handle long content properly
        let serialized = serde_json::to_string(&message).unwrap();
        let deserialized: Message = serde_json::from_str(&serialized).unwrap();

        prop_assert_eq!(&message.content, &deserialized.content);
        prop_assert!(message.content.len() >= 1000);
    }

    #[test]
    fn test_special_characters_in_content(content in r"[^\x00-\x1F\x7F]{1,100}") {
        let message = Message {
            role: MessageRole::Assistant,
            content,
            tool_call_id: None,
            tool_name: None,
            timestamp: std::time::SystemTime::now(),
        };

        // Should handle various Unicode characters
        let serialized = serde_json::to_string(&message).unwrap();
        let deserialized: Message = serde_json::from_str(&serialized).unwrap();

        prop_assert_eq!(message.content, deserialized.content);
    }

    #[test]
    fn test_extreme_batch_sizes(batch_size in 1u32..100000) {
        let config = ModelConfig {
            batch_size,
            ..Default::default()
        };

        let validation_result = config.validate();

        if batch_size == 0 {
            prop_assert!(validation_result.is_err());
        } else if batch_size > 8192 {
            prop_assert!(validation_result.is_err());
        } else {
            // Other validation might fail due to default model source, but batch size is valid
            match validation_result {
                Ok(_) => {},
                Err(ModelError::NotFound(_)) => {
                    // Expected for default HuggingFace source
                },
                Err(e) => prop_assert!(false, "Unexpected error for valid batch size: {:?}", e),
            }
        }
    }

    #[test]
    fn test_extreme_timeouts(timeout_secs in 0u64..86400) {
        let config = QueueConfig {
            max_queue_size: 100,
            request_timeout: Duration::from_secs(timeout_secs),
            worker_threads: 1,
        };

        let validation_result = config.validate();

        if timeout_secs == 0 {
            prop_assert!(validation_result.is_err());
        } else {
            prop_assert!(validation_result.is_ok());
        }
    }
}

// Tests for invariants

#[tokio::test]
async fn test_session_id_uniqueness() {
    // Generate many session IDs and ensure they are unique
    let mut session_ids = std::collections::HashSet::new();

    for _ in 0..1000 {
        let session_id = SessionId::new();
        assert!(
            session_ids.insert(session_id),
            "Duplicate session ID generated: {}",
            session_id
        );
    }

    assert_eq!(session_ids.len(), 1000);
}

#[tokio::test]
async fn test_tool_call_id_uniqueness() {
    // Generate many tool call IDs and ensure they are unique
    let mut tool_call_ids = std::collections::HashSet::new();

    for _ in 0..1000 {
        let tool_call_id = ToolCallId::new();
        assert!(
            tool_call_ids.insert(tool_call_id),
            "Duplicate tool call ID generated: {}",
            tool_call_id
        );
    }

    assert_eq!(tool_call_ids.len(), 1000);
}

#[tokio::test]
async fn test_message_timestamp_ordering() {
    // Test that message timestamps are ordered correctly
    let mut messages = Vec::new();

    for i in 0..10 {
        tokio::time::sleep(Duration::from_millis(1)).await;

        let message = Message {
            role: MessageRole::User,
            content: format!("Message {}", i),
            tool_call_id: None,
            tool_name: None,
            timestamp: std::time::SystemTime::now(),
        };
        messages.push(message);
    }

    // Verify timestamps are in order (allowing for some clock jitter)
    for i in 1..messages.len() {
        let prev_time = messages[i - 1].timestamp;
        let curr_time = messages[i].timestamp;

        // Should be greater than or equal (allowing for clock resolution limits)
        assert!(
            curr_time >= prev_time,
            "Message {} timestamp {:?} is before message {} timestamp {:?}",
            i,
            curr_time,
            i - 1,
            prev_time
        );
    }
}
