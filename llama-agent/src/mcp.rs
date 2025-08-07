use crate::types::{MCPError, MCPServerConfig, ToolCall, ToolDefinition, ToolResult};
use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;
use std::process::{Child, Command, Stdio};
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::sync::{Mutex, RwLock};
use tokio::time::timeout;
use tracing::{debug, error, info, warn};

#[derive(Debug, Clone)]
pub enum HealthStatus {
    Healthy,
    Unhealthy(String),
    Unknown,
}

#[async_trait]
pub trait MCPServer: Send + Sync {
    async fn initialize(&mut self) -> Result<(), MCPError>;
    async fn list_tools(&self) -> Result<Vec<ToolDefinition>, MCPError>;
    async fn call_tool(&self, tool_name: &str, args: Value) -> Result<Value, MCPError>;
    async fn health(&self) -> Result<HealthStatus, MCPError>;
    async fn shutdown(&mut self) -> Result<(), MCPError>;
    fn name(&self) -> &str;
}

struct MCPServerImpl {
    config: MCPServerConfig,
    process: Option<Child>,
    last_health_check: Option<SystemTime>,
    initialized: bool,
}

impl MCPServerImpl {
    pub fn new(config: MCPServerConfig) -> Self {
        Self {
            config,
            process: None,
            last_health_check: None,
            initialized: false,
        }
    }

    async fn spawn_process(&mut self) -> Result<Child, MCPError> {
        debug!("Spawning MCP server process: {} {:?}", self.config.command, self.config.args);
        
        let mut cmd = Command::new(&self.config.command);
        cmd.args(&self.config.args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let process = cmd.spawn().map_err(|e| {
            MCPError::Connection(format!(
                "Failed to spawn MCP server '{}': {}",
                self.config.name, e
            ))
        })?;

        info!("Spawned MCP server '{}' with PID: {:?}", self.config.name, process.id());
        Ok(process)
    }
}

#[async_trait]
impl MCPServer for MCPServerImpl {
    async fn initialize(&mut self) -> Result<(), MCPError> {
        debug!("Initializing MCP server: {}", self.config.name);
        
        let process = self.spawn_process().await?;
        self.process = Some(process);
        self.initialized = true;
        self.last_health_check = Some(SystemTime::now());
        
        info!("Successfully initialized MCP server: {}", self.config.name);
        Ok(())
    }

    async fn list_tools(&self) -> Result<Vec<ToolDefinition>, MCPError> {
        if !self.initialized {
            return Err(MCPError::Connection(format!("Server '{}' not initialized", self.config.name)));
        }

        debug!("Listing tools for MCP server: {}", self.config.name);

        // For now, return empty tools list - this would be implemented with actual MCP communication
        // In a real implementation, this would use the rmcp library to communicate with the process
        // via stdio and send a tools/list request
        let tool_definitions = Vec::new();

        debug!("Found {} tools for server '{}'", tool_definitions.len(), self.config.name);
        Ok(tool_definitions)
    }

    async fn call_tool(&self, tool_name: &str, args: Value) -> Result<Value, MCPError> {
        if !self.initialized {
            return Err(MCPError::Connection(format!("Server '{}' not initialized", self.config.name)));
        }

        debug!("Calling tool '{}' on server '{}'", tool_name, self.config.name);

        // For now, return a placeholder result - this would be implemented with actual MCP communication
        // In a real implementation, this would use the rmcp library to communicate with the process
        // via stdio and send a tools/call request
        let result = serde_json::json!({
            "content": [
                {
                    "type": "text",
                    "text": format!("Called {} with args: {}", tool_name, args)
                }
            ]
        });

        debug!("Tool '{}' on server '{}' completed successfully", tool_name, self.config.name);
        Ok(result)
    }

    async fn health(&self) -> Result<HealthStatus, MCPError> {
        if !self.initialized {
            return Ok(HealthStatus::Unhealthy("Not initialized".to_string()));
        }

        // Check if process is still alive by checking if we have a process
        // In a real implementation, this would need proper process monitoring
        if self.process.is_some() {
            Ok(HealthStatus::Healthy)
        } else {
            Ok(HealthStatus::Unhealthy("Process not found".to_string()))
        }
    }

    async fn shutdown(&mut self) -> Result<(), MCPError> {
        info!("Shutting down MCP server: {}", self.config.name);

        // Terminate process
        if let Some(mut process) = self.process.take() {
            match process.kill() {
                Ok(_) => {
                    // Wait for process to exit
                    match timeout(Duration::from_secs(5), async {
                        loop {
                            match process.try_wait() {
                                Ok(Some(_)) => break,
                                Ok(None) => tokio::time::sleep(Duration::from_millis(100)).await,
                                Err(e) => return Err(e),
                            }
                        }
                        Ok::<(), std::io::Error>(())
                    }).await {
                        Ok(Ok(_)) => info!("MCP server '{}' shut down gracefully", self.config.name),
                        Ok(Err(e)) => warn!("Error waiting for MCP server '{}' shutdown: {}", self.config.name, e),
                        Err(_) => {
                            warn!("Timeout waiting for MCP server '{}' shutdown, forcing termination", self.config.name);
                            let _ = process.kill();
                        }
                    }
                }
                Err(e) => {
                    warn!("Failed to kill MCP server '{}' process: {}", self.config.name, e);
                }
            }
        }

        self.initialized = false;
        Ok(())
    }

    fn name(&self) -> &str {
        &self.config.name
    }
}

pub struct MCPClient {
    servers: Arc<RwLock<HashMap<String, Arc<Mutex<Box<dyn MCPServer>>>>>>,
    retry_config: RetryConfig,
}

#[derive(Debug, Clone)]
pub struct RetryConfig {
    pub max_retries: u32,
    pub initial_delay: Duration,
    pub max_delay: Duration,
    pub backoff_multiplier: f64,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            initial_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(5),
            backoff_multiplier: 2.0,
        }
    }
}

impl MCPClient {
    pub fn new() -> Self {
        Self {
            servers: Arc::new(RwLock::new(HashMap::new())),
            retry_config: RetryConfig::default(),
        }
    }

    pub fn with_retry_config(retry_config: RetryConfig) -> Self {
        Self {
            servers: Arc::new(RwLock::new(HashMap::new())),
            retry_config,
        }
    }

    pub async fn initialize(configs: Vec<MCPServerConfig>) -> Result<Self, MCPError> {
        let client = Self::new();
        
        for config in configs {
            config.validate()?;
            client.add_server(config).await?;
        }
        
        Ok(client)
    }

    pub async fn add_server(&self, config: MCPServerConfig) -> Result<(), MCPError> {
        let server_name = config.name.clone();
        let mut server: Box<dyn MCPServer> = Box::new(MCPServerImpl::new(config));
        
        info!("Adding MCP server: {}", server_name);
        
        // Initialize server with retries
        let mut delay = self.retry_config.initial_delay;
        let mut last_error = None;

        for attempt in 0..=self.retry_config.max_retries {
            match server.initialize().await {
                Ok(_) => break,
                Err(e) => {
                    last_error = Some(e);
                    
                    if attempt < self.retry_config.max_retries {
                        debug!("Retry attempt {} failed, waiting {:?} before next attempt", 
                               attempt + 1, delay);
                        tokio::time::sleep(delay).await;
                        
                        // Exponential backoff
                        delay = std::cmp::min(
                            self.retry_config.max_delay,
                            Duration::from_millis(
                                (delay.as_millis() as f64 * self.retry_config.backoff_multiplier) as u64
                            ),
                        );
                    }
                }
            }
        }

        if let Some(e) = last_error {
            error!("Failed to initialize MCP server '{}' after retries: {}", server_name, e);
            return Err(e);
        }

        // Store the initialized server
        let mut servers = self.servers.write().await;
        servers.insert(server_name.clone(), Arc::new(Mutex::new(server)));
        
        info!("Successfully added MCP server: {}", server_name);
        Ok(())
    }

    pub async fn remove_server(&self, server_name: &str) -> Result<(), MCPError> {
        info!("Removing MCP server: {}", server_name);
        
        let mut servers = self.servers.write().await;
        if let Some(server_arc) = servers.remove(server_name) {
            let mut server = server_arc.lock().await;
            server.shutdown().await?;
            info!("Successfully removed MCP server: {}", server_name);
        } else {
            warn!("Attempted to remove non-existent MCP server: {}", server_name);
        }
        
        Ok(())
    }

    pub async fn discover_tools(&self) -> Result<Vec<ToolDefinition>, MCPError> {
        debug!("Discovering tools from all MCP servers");
        
        let servers = self.servers.read().await;
        let mut all_tools = Vec::new();
        let mut errors = Vec::new();

        for (server_name, server_arc) in servers.iter() {
            let server = server_arc.lock().await;
            
            match server.list_tools().await {
                Ok(mut tools) => {
                    debug!("Found {} tools from server '{}'", tools.len(), server_name);
                    all_tools.append(&mut tools);
                }
                Err(e) => {
                    warn!("Failed to discover tools from server '{}': {}", server_name, e);
                    errors.push(format!("Server '{}': {}", server_name, e));
                }
            }
        }

        if all_tools.is_empty() && !errors.is_empty() {
            return Err(MCPError::Connection(format!(
                "Failed to discover tools from any server. Errors: {}",
                errors.join("; ")
            )));
        }

        if !errors.is_empty() {
            warn!("Some servers failed during tool discovery: {}", errors.join("; "));
        }

        info!("Discovered {} tools from {} servers", all_tools.len(), servers.len());
        Ok(all_tools)
    }

    pub async fn list_servers(&self) -> Vec<String> {
        let servers = self.servers.read().await;
        servers.keys().cloned().collect()
    }

    pub async fn server_count(&self) -> usize {
        let servers = self.servers.read().await;
        servers.len()
    }

    pub async fn call_tool(&self, server_name: &str, tool_name: &str, args: Value) -> Result<Value, MCPError> {
        debug!("Calling tool '{}' on server '{}'", tool_name, server_name);
        
        let servers = self.servers.read().await;
        let server_arc = servers.get(server_name)
            .ok_or_else(|| MCPError::ServerNotFound(server_name.to_string()))?;
        
        let server = server_arc.lock().await;
        
        // Execute the tool call
        let result = server.call_tool(tool_name, args).await?;

        info!("Successfully called tool '{}' on server '{}'", tool_name, server_name);
        Ok(result)
    }

    pub async fn execute_tool_call(&self, tool_call: &ToolCall) -> Result<ToolResult, MCPError> {
        debug!("Executing tool call: {} (ID: {})", tool_call.name, tool_call.id);

        // Find which server has this tool
        let servers = self.servers.read().await;
        let mut target_server = None;
        
        for (server_name, server_arc) in servers.iter() {
            let server = server_arc.lock().await;
            
            // Check if this server has the requested tool
            match server.list_tools().await {
                Ok(tools) => {
                    if tools.iter().any(|tool| tool.name == tool_call.name) {
                        target_server = Some(server_name.clone());
                        break;
                    }
                }
                Err(e) => {
                    warn!("Failed to list tools from server '{}' during tool call execution: {}", server_name, e);
                }
            }
        }

        let server_name = target_server.ok_or_else(|| {
            MCPError::ToolCallFailed(format!(
                "Tool '{}' not found in any connected server", tool_call.name
            ))
        })?;

        // Execute the tool call
        match self.call_tool(&server_name, &tool_call.name, tool_call.arguments.clone()).await {
            Ok(result) => {
                Ok(ToolResult {
                    call_id: tool_call.id,
                    result,
                    error: None,
                })
            }
            Err(e) => {
                Ok(ToolResult {
                    call_id: tool_call.id,
                    result: Value::Null,
                    error: Some(format!("Tool execution failed: {}", e)),
                })
            }
        }
    }

    pub async fn server_health(&self, server_name: &str) -> Result<HealthStatus, MCPError> {
        debug!("Checking health for server: {}", server_name);
        
        let servers = self.servers.read().await;
        let server_arc = servers.get(server_name)
            .ok_or_else(|| MCPError::ServerNotFound(server_name.to_string()))?;
        
        let server = server_arc.lock().await;
        server.health().await
    }

    pub async fn health_check_all(&self) -> HashMap<String, HealthStatus> {
        debug!("Performing health check on all servers");
        
        let servers = self.servers.read().await;
        let mut health_results = HashMap::new();

        for (server_name, server_arc) in servers.iter() {
            let server = server_arc.lock().await;
            
            match server.health().await {
                Ok(status) => {
                    health_results.insert(server_name.clone(), status);
                }
                Err(e) => {
                    warn!("Health check failed for server '{}': {}", server_name, e);
                    health_results.insert(
                        server_name.clone(), 
                        HealthStatus::Unhealthy(format!("Health check error: {}", e))
                    );
                }
            }
        }

        info!("Health check completed for {} servers", health_results.len());
        health_results
    }

    pub async fn restart_server(&self, server_name: &str) -> Result<(), MCPError> {
        info!("Restarting MCP server: {}", server_name);
        
        let servers = self.servers.read().await;
        let server_arc = servers.get(server_name)
            .ok_or_else(|| MCPError::ServerNotFound(server_name.to_string()))?
            .clone();
        
        drop(servers); // Release read lock
        
        let mut server = server_arc.lock().await;
        
        // Shutdown existing server
        if let Err(e) = server.shutdown().await {
            warn!("Error during server shutdown for '{}': {}", server_name, e);
        }
        
        // Re-initialize server with retries
        let mut delay = self.retry_config.initial_delay;
        let mut last_error = None;

        for attempt in 0..=self.retry_config.max_retries {
            match server.initialize().await {
                Ok(_) => break,
                Err(e) => {
                    last_error = Some(e);
                    
                    if attempt < self.retry_config.max_retries {
                        debug!("Retry attempt {} failed, waiting {:?} before next attempt", 
                               attempt + 1, delay);
                        tokio::time::sleep(delay).await;
                        
                        // Exponential backoff
                        delay = std::cmp::min(
                            self.retry_config.max_delay,
                            Duration::from_millis(
                                (delay.as_millis() as f64 * self.retry_config.backoff_multiplier) as u64
                            ),
                        );
                    }
                }
            }
        }

        if let Some(e) = last_error {
            error!("Failed to restart MCP server '{}' after retries: {}", server_name, e);
            return Err(e);
        }
        
        info!("Successfully restarted MCP server: {}", server_name);
        Ok(())
    }

    pub async fn shutdown_all(&self) -> Result<(), MCPError> {
        info!("Shutting down all MCP servers");
        
        let mut servers = self.servers.write().await;
        let mut errors = Vec::new();

        for (server_name, server_arc) in servers.drain() {
            let mut server = server_arc.lock().await;
            
            if let Err(e) = server.shutdown().await {
                errors.push(format!("Server '{}': {}", server_name, e));
            } else {
                info!("Successfully shut down server: {}", server_name);
            }
        }

        if !errors.is_empty() {
            return Err(MCPError::Connection(format!(
                "Errors during shutdown: {}",
                errors.join("; ")
            )));
        }

        info!("All MCP servers shut down successfully");
        Ok(())
    }
}

impl Default for MCPClient {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::ToolCallId;
    use serde_json::json;
    use tokio::time::Duration;

    // Mock server implementation for testing
    struct MockMCPServer {
        name: String,
        tools: Vec<ToolDefinition>,
        should_fail: bool,
        fail_on_health: bool,
    }

    impl MockMCPServer {
        fn new(name: &str, tools: Vec<ToolDefinition>) -> Self {
            Self {
                name: name.to_string(),
                tools,
                should_fail: false,
                fail_on_health: false,
            }
        }

        fn with_failure(mut self, should_fail: bool) -> Self {
            self.should_fail = should_fail;
            self
        }

        #[allow(dead_code)]
        fn with_health_failure(mut self, fail_on_health: bool) -> Self {
            self.fail_on_health = fail_on_health;
            self
        }
    }

    #[async_trait]
    impl MCPServer for MockMCPServer {
        async fn initialize(&mut self) -> Result<(), MCPError> {
            if self.should_fail {
                return Err(MCPError::Connection("Mock initialization failure".to_string()));
            }
            Ok(())
        }

        async fn list_tools(&self) -> Result<Vec<ToolDefinition>, MCPError> {
            if self.should_fail {
                return Err(MCPError::Protocol("Mock tool listing failure".to_string()));
            }
            Ok(self.tools.clone())
        }

        async fn call_tool(&self, tool_name: &str, args: Value) -> Result<Value, MCPError> {
            if self.should_fail {
                return Err(MCPError::ToolCallFailed(format!("Mock tool call failure for {}", tool_name)));
            }

            // Simulate successful tool execution
            if self.tools.iter().any(|tool| tool.name == tool_name) {
                Ok(json!({
                    "result": format!("Mock result for {} with args: {}", tool_name, args),
                    "success": true
                }))
            } else {
                Err(MCPError::ToolCallFailed(format!("Tool '{}' not found", tool_name)))
            }
        }

        async fn health(&self) -> Result<HealthStatus, MCPError> {
            if self.fail_on_health {
                Err(MCPError::Connection("Health check failed".to_string()))
            } else if self.should_fail {
                Ok(HealthStatus::Unhealthy("Mock server unhealthy".to_string()))
            } else {
                Ok(HealthStatus::Healthy)
            }
        }

        async fn shutdown(&mut self) -> Result<(), MCPError> {
            Ok(())
        }

        fn name(&self) -> &str {
            &self.name
        }
    }

    #[tokio::test]
    async fn test_mcp_client_initialization() {
        let client = MCPClient::new();
        assert_eq!(client.server_count().await, 0);
        assert!(client.list_servers().await.is_empty());
    }

    #[tokio::test]
    async fn test_retry_config() {
        let config = RetryConfig::default();
        assert_eq!(config.max_retries, 3);
        assert_eq!(config.initial_delay, Duration::from_millis(100));
        assert_eq!(config.backoff_multiplier, 2.0);

        let custom_config = RetryConfig {
            max_retries: 5,
            initial_delay: Duration::from_millis(50),
            max_delay: Duration::from_secs(10),
            backoff_multiplier: 1.5,
        };
        
        let client = MCPClient::with_retry_config(custom_config.clone());
        assert_eq!(client.retry_config.max_retries, 5);
    }

    #[tokio::test]
    async fn test_health_status() {
        let healthy = HealthStatus::Healthy;
        let unhealthy = HealthStatus::Unhealthy("Test error".to_string());
        let unknown = HealthStatus::Unknown;

        match healthy {
            HealthStatus::Healthy => assert!(true),
            _ => panic!("Expected Healthy status"),
        }

        match unhealthy {
            HealthStatus::Unhealthy(msg) => assert_eq!(msg, "Test error"),
            _ => panic!("Expected Unhealthy status"),
        }

        match unknown {
            HealthStatus::Unknown => assert!(true),
            _ => panic!("Expected Unknown status"),
        }
    }

    #[tokio::test]
    async fn test_tool_definition_creation() {
        let tool = ToolDefinition {
            name: "test_tool".to_string(),
            description: "A test tool".to_string(),
            parameters: json!({"type": "object"}),
            server_name: "test_server".to_string(),
        };

        assert_eq!(tool.name, "test_tool");
        assert_eq!(tool.description, "A test tool");
        assert_eq!(tool.server_name, "test_server");
    }

    #[tokio::test]
    async fn test_mock_server_functionality() {
        let tools = vec![
            ToolDefinition {
                name: "list_files".to_string(),
                description: "List files in directory".to_string(),
                parameters: json!({"type": "object"}),
                server_name: "test_server".to_string(),
            }
        ];

        let mut server = MockMCPServer::new("test_server", tools);
        
        // Test initialization
        assert!(server.initialize().await.is_ok());
        
        // Test tool listing
        let listed_tools = server.list_tools().await.unwrap();
        assert_eq!(listed_tools.len(), 1);
        assert_eq!(listed_tools[0].name, "list_files");
        
        // Test tool calling
        let result = server.call_tool("list_files", json!({"path": "/tmp"})).await.unwrap();
        assert!(result.get("success").unwrap().as_bool().unwrap());
        
        // Test health check
        let health = server.health().await.unwrap();
        match health {
            HealthStatus::Healthy => assert!(true),
            _ => panic!("Expected healthy status"),
        }
        
        // Test shutdown
        assert!(server.shutdown().await.is_ok());
    }

    #[tokio::test]
    async fn test_mock_server_failures() {
        let tools = vec![
            ToolDefinition {
                name: "failing_tool".to_string(),
                description: "A tool that fails".to_string(),
                parameters: json!({"type": "object"}),
                server_name: "failing_server".to_string(),
            }
        ];

        let mut failing_server = MockMCPServer::new("failing_server", tools)
            .with_failure(true);
        
        // Test initialization failure
        assert!(failing_server.initialize().await.is_err());
        
        // Reset failure for other tests
        failing_server.should_fail = false;
        assert!(failing_server.initialize().await.is_ok());
        
        // Test tool listing failure
        failing_server.should_fail = true;
        assert!(failing_server.list_tools().await.is_err());
        
        // Test tool call failure
        assert!(failing_server.call_tool("failing_tool", json!({})).await.is_err());
        
        // Test health check failure
        let health = failing_server.health().await.unwrap();
        match health {
            HealthStatus::Unhealthy(_) => assert!(true),
            _ => panic!("Expected unhealthy status"),
        }
    }

    #[tokio::test]
    async fn test_tool_call_and_result() {
        let call_id = ToolCallId::new();
        let tool_call = ToolCall {
            id: call_id,
            name: "test_tool".to_string(),
            arguments: json!({"param": "value"}),
        };

        let result = ToolResult {
            call_id: tool_call.id,
            result: json!({"status": "success"}),
            error: None,
        };

        assert_eq!(result.call_id, call_id);
        assert!(result.error.is_none());
        
        let error_result = ToolResult {
            call_id: tool_call.id,
            result: Value::Null,
            error: Some("Tool execution failed".to_string()),
        };

        assert_eq!(error_result.call_id, call_id);
        assert!(error_result.error.is_some());
    }

    #[tokio::test]
    async fn test_server_config_validation() {
        let valid_config = MCPServerConfig {
            name: "filesystem".to_string(),
            command: "npx".to_string(),
            args: vec!["-y".to_string(), "@modelcontextprotocol/server-filesystem".to_string()],
        };

        assert!(valid_config.validate().is_ok());

        let invalid_config = MCPServerConfig {
            name: "".to_string(),
            command: "npx".to_string(),
            args: vec![],
        };

        assert!(invalid_config.validate().is_err());
    }

    #[tokio::test]
    async fn test_mcp_client_default() {
        let client1 = MCPClient::default();
        let client2 = MCPClient::new();
        
        assert_eq!(client1.server_count().await, client2.server_count().await);
    }

    #[tokio::test]
    async fn test_concurrent_operations() {
        let client = Arc::new(MCPClient::new());
        let mut handles = vec![];

        // Simulate concurrent server operations
        for i in 0..5 {
            let client_clone = client.clone();
            let handle = tokio::spawn(async move {
                let _server_name = format!("concurrent_server_{}", i);
                let servers = client_clone.list_servers().await;
                servers.len()
            });
            handles.push(handle);
        }

        // Wait for all operations to complete
        for handle in handles {
            let result = handle.await.unwrap();
            assert_eq!(result, 0); // No servers initially
        }
    }

    #[tokio::test]
    async fn test_error_propagation() {
        // Test that errors are properly wrapped and propagated
        let connection_error = MCPError::Connection("Test connection error".to_string());
        let protocol_error = MCPError::Protocol("Test protocol error".to_string());
        let server_not_found = MCPError::ServerNotFound("test_server".to_string());
        let tool_call_failed = MCPError::ToolCallFailed("Test tool call failure".to_string());

        assert!(matches!(connection_error, MCPError::Connection(_)));
        assert!(matches!(protocol_error, MCPError::Protocol(_)));
        assert!(matches!(server_not_found, MCPError::ServerNotFound(_)));
        assert!(matches!(tool_call_failed, MCPError::ToolCallFailed(_)));
    }
}
