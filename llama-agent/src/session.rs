use crate::types::{Session, SessionConfig, SessionError, Message};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::sync::RwLock;
use tracing::{debug, info, warn};
use uuid::Uuid;

pub struct SessionManager {
    sessions: Arc<RwLock<HashMap<String, Session>>>,
    config: SessionConfig,
}

impl SessionManager {
    pub fn new(config: SessionConfig) -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
            config,
        }
    }

    pub async fn create_session(&self) -> Result<Session, SessionError> {
        let mut sessions = self.sessions.write().await;
        
        // Check if we've reached the session limit
        if sessions.len() >= self.config.max_sessions {
            warn!("Session limit reached: {}", self.config.max_sessions);
            return Err(SessionError::LimitExceeded);
        }

        let now = SystemTime::now();
        let session = Session {
            id: Uuid::new_v4().to_string(),
            messages: Vec::new(),
            mcp_servers: Vec::new(),
            available_tools: Vec::new(),
            created_at: now,
            updated_at: now,
        };

        info!("Created new session: {}", session.id);
        sessions.insert(session.id.clone(), session.clone());

        Ok(session)
    }

    pub async fn get_session(&self, session_id: &str) -> Result<Option<Session>, SessionError> {
        let sessions = self.sessions.read().await;
        
        match sessions.get(session_id) {
            Some(session) => {
                // Check if session has expired
                if let Ok(age) = session.updated_at.elapsed() {
                    if age > self.config.session_timeout {
                        debug!("Session {} has expired (age: {:?})", session_id, age);
                        return Ok(None);
                    }
                }
                Ok(Some(session.clone()))
            }
            None => Ok(None)
        }
    }

    pub async fn update_session(&self, session: Session) -> Result<(), SessionError> {
        let mut sessions = self.sessions.write().await;
        
        // Check if session exists
        if !sessions.contains_key(&session.id) {
            return Err(SessionError::NotFound(session.id.clone()));
        }

        // Update the timestamp
        let mut updated_session = session;
        updated_session.updated_at = SystemTime::now();

        debug!("Updating session: {}", updated_session.id);
        sessions.insert(updated_session.id.clone(), updated_session);

        Ok(())
    }

    pub async fn add_message(&self, session_id: &str, message: Message) -> Result<(), SessionError> {
        let mut sessions = self.sessions.write().await;
        
        match sessions.get_mut(session_id) {
            Some(session) => {
                session.messages.push(message);
                session.updated_at = SystemTime::now();
                debug!("Added message to session {}, total messages: {}", session_id, session.messages.len());
                Ok(())
            }
            None => Err(SessionError::NotFound(session_id.to_string()))
        }
    }

    pub async fn delete_session(&self, session_id: &str) -> Result<bool, SessionError> {
        let mut sessions = self.sessions.write().await;
        
        match sessions.remove(session_id) {
            Some(_) => {
                info!("Deleted session: {}", session_id);
                Ok(true)
            }
            None => Ok(false)
        }
    }

    pub async fn list_sessions(&self) -> Result<Vec<String>, SessionError> {
        let sessions = self.sessions.read().await;
        Ok(sessions.keys().cloned().collect())
    }

    pub async fn get_session_count(&self) -> usize {
        let sessions = self.sessions.read().await;
        sessions.len()
    }

    pub async fn cleanup_expired_sessions(&self) -> Result<usize, SessionError> {
        let mut sessions = self.sessions.write().await;
        let mut expired_sessions = Vec::new();

        // Find expired sessions
        for (session_id, session) in sessions.iter() {
            if let Ok(age) = session.updated_at.elapsed() {
                if age > self.config.session_timeout {
                    expired_sessions.push(session_id.clone());
                }
            }
        }

        // Remove expired sessions
        let mut removed_count = 0;
        for session_id in expired_sessions {
            sessions.remove(&session_id);
            removed_count += 1;
            debug!("Removed expired session: {}", session_id);
        }

        if removed_count > 0 {
            info!("Cleaned up {} expired sessions", removed_count);
        }

        Ok(removed_count)
    }

    pub async fn get_session_stats(&self) -> SessionStats {
        let sessions = self.sessions.read().await;
        
        let mut total_messages = 0;
        let mut active_sessions = 0;
        let mut expired_sessions = 0;

        for session in sessions.values() {
            total_messages += session.messages.len();
            
            if let Ok(age) = session.updated_at.elapsed() {
                if age <= self.config.session_timeout {
                    active_sessions += 1;
                } else {
                    expired_sessions += 1;
                }
            }
        }

        SessionStats {
            total_sessions: sessions.len(),
            active_sessions,
            expired_sessions,
            total_messages,
            max_sessions: self.config.max_sessions,
            session_timeout: self.config.session_timeout,
        }
    }
}

#[derive(Debug, Clone)]
pub struct SessionStats {
    pub total_sessions: usize,
    pub active_sessions: usize,
    pub expired_sessions: usize,
    pub total_messages: usize,
    pub max_sessions: usize,
    pub session_timeout: Duration,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{MessageRole, SessionConfig};
    use std::time::Duration;

    fn create_test_config() -> SessionConfig {
        SessionConfig {
            max_sessions: 5,
            session_timeout: Duration::from_secs(10),
        }
    }

    fn create_test_message() -> Message {
        Message {
            role: MessageRole::User,
            content: "Hello, world!".to_string(),
            tool_call_id: None,
            tool_name: None,
            timestamp: SystemTime::now(),
        }
    }

    #[tokio::test]
    async fn test_session_manager_creation() {
        let config = create_test_config();
        let manager = SessionManager::new(config);
        
        assert_eq!(manager.get_session_count().await, 0);
        
        let sessions = manager.list_sessions().await.unwrap();
        assert!(sessions.is_empty());
    }

    #[tokio::test]
    async fn test_create_session() {
        let config = create_test_config();
        let manager = SessionManager::new(config);

        let session = manager.create_session().await.unwrap();
        
        assert!(!session.id.is_empty());
        assert!(session.messages.is_empty());
        assert!(session.mcp_servers.is_empty());
        assert!(session.available_tools.is_empty());
        assert_eq!(manager.get_session_count().await, 1);
    }

    #[tokio::test]
    async fn test_get_session() {
        let config = create_test_config();
        let manager = SessionManager::new(config);

        let session = manager.create_session().await.unwrap();
        let session_id = session.id.clone();

        // Get existing session
        let retrieved = manager.get_session(&session_id).await.unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().id, session_id);

        // Get non-existent session
        let non_existent = manager.get_session("non-existent-id").await.unwrap();
        assert!(non_existent.is_none());
    }

    #[tokio::test]
    async fn test_update_session() {
        let config = create_test_config();
        let manager = SessionManager::new(config);

        let mut session = manager.create_session().await.unwrap();
        let session_id = session.id.clone();
        let original_updated_at = session.updated_at;
        
        // Wait a bit to ensure timestamp difference
        tokio::time::sleep(Duration::from_millis(10)).await;
        
        session.messages.push(create_test_message());
        
        let result = manager.update_session(session).await;
        assert!(result.is_ok());

        // Verify the session was updated
        let updated_session = manager.get_session(&session_id).await.unwrap().unwrap();
        assert_eq!(updated_session.messages.len(), 1);
        assert!(updated_session.updated_at > original_updated_at);
    }

    #[tokio::test]
    async fn test_update_non_existent_session() {
        let config = create_test_config();
        let manager = SessionManager::new(config);

        let session = Session {
            id: "non-existent".to_string(),
            messages: Vec::new(),
            mcp_servers: Vec::new(),
            available_tools: Vec::new(),
            created_at: SystemTime::now(),
            updated_at: SystemTime::now(),
        };

        let result = manager.update_session(session).await;
        assert!(matches!(result, Err(SessionError::NotFound(_))));
    }

    #[tokio::test]
    async fn test_add_message() {
        let config = create_test_config();
        let manager = SessionManager::new(config);

        let session = manager.create_session().await.unwrap();
        let session_id = session.id.clone();

        let message = create_test_message();
        let result = manager.add_message(&session_id, message).await;
        assert!(result.is_ok());

        let updated_session = manager.get_session(&session_id).await.unwrap().unwrap();
        assert_eq!(updated_session.messages.len(), 1);
        assert_eq!(updated_session.messages[0].content, "Hello, world!");
    }

    #[tokio::test]
    async fn test_add_message_to_non_existent_session() {
        let config = create_test_config();
        let manager = SessionManager::new(config);

        let message = create_test_message();
        let result = manager.add_message("non-existent", message).await;
        assert!(matches!(result, Err(SessionError::NotFound(_))));
    }

    #[tokio::test]
    async fn test_delete_session() {
        let config = create_test_config();
        let manager = SessionManager::new(config);

        let session = manager.create_session().await.unwrap();
        let session_id = session.id.clone();

        // Delete existing session
        let result = manager.delete_session(&session_id).await.unwrap();
        assert!(result);
        assert_eq!(manager.get_session_count().await, 0);

        // Delete non-existent session
        let result = manager.delete_session("non-existent").await.unwrap();
        assert!(!result);
    }

    #[tokio::test]
    async fn test_session_limit() {
        let config = SessionConfig {
            max_sessions: 2,
            session_timeout: Duration::from_secs(10),
        };
        let manager = SessionManager::new(config);

        // Create sessions up to the limit
        let _session1 = manager.create_session().await.unwrap();
        let _session2 = manager.create_session().await.unwrap();
        
        // Try to create one more session - should fail
        let result = manager.create_session().await;
        assert!(matches!(result, Err(SessionError::LimitExceeded)));
        assert_eq!(manager.get_session_count().await, 2);
    }

    #[tokio::test]
    async fn test_list_sessions() {
        let config = create_test_config();
        let manager = SessionManager::new(config);

        let session1 = manager.create_session().await.unwrap();
        let session2 = manager.create_session().await.unwrap();

        let sessions = manager.list_sessions().await.unwrap();
        assert_eq!(sessions.len(), 2);
        assert!(sessions.contains(&session1.id));
        assert!(sessions.contains(&session2.id));
    }

    #[tokio::test]
    async fn test_session_expiration() {
        let config = SessionConfig {
            max_sessions: 10,
            session_timeout: Duration::from_millis(50), // Very short timeout
        };
        let manager = SessionManager::new(config);

        let session = manager.create_session().await.unwrap();
        let session_id = session.id.clone();

        // Session should exist initially
        let retrieved = manager.get_session(&session_id).await.unwrap();
        assert!(retrieved.is_some());

        // Wait for session to expire
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Session should now be considered expired
        let expired = manager.get_session(&session_id).await.unwrap();
        assert!(expired.is_none());
    }

    #[tokio::test]
    async fn test_cleanup_expired_sessions() {
        let config = SessionConfig {
            max_sessions: 10,
            session_timeout: Duration::from_millis(50), // Very short timeout
        };
        let manager = SessionManager::new(config);

        // Create some sessions
        manager.create_session().await.unwrap();
        manager.create_session().await.unwrap();
        assert_eq!(manager.get_session_count().await, 2);

        // Wait for sessions to expire
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Cleanup expired sessions
        let removed = manager.cleanup_expired_sessions().await.unwrap();
        assert_eq!(removed, 2);
        assert_eq!(manager.get_session_count().await, 0);
    }

    #[tokio::test]
    async fn test_get_session_stats() {
        let config = create_test_config();
        let manager = SessionManager::new(config);

        // Create some sessions with messages
        let session1 = manager.create_session().await.unwrap();
        let session2 = manager.create_session().await.unwrap();
        
        manager.add_message(&session1.id, create_test_message()).await.unwrap();
        manager.add_message(&session2.id, create_test_message()).await.unwrap();
        manager.add_message(&session2.id, create_test_message()).await.unwrap();

        let stats = manager.get_session_stats().await;
        assert_eq!(stats.total_sessions, 2);
        assert_eq!(stats.active_sessions, 2);
        assert_eq!(stats.expired_sessions, 0);
        assert_eq!(stats.total_messages, 3);
        assert_eq!(stats.max_sessions, 5);
    }

    #[test]
    fn test_session_stats_debug() {
        let stats = SessionStats {
            total_sessions: 5,
            active_sessions: 3,
            expired_sessions: 2,
            total_messages: 10,
            max_sessions: 10,
            session_timeout: Duration::from_secs(3600),
        };

        let debug_str = format!("{:?}", stats);
        assert!(debug_str.contains("total_sessions: 5"));
        assert!(debug_str.contains("active_sessions: 3"));
        assert!(debug_str.contains("total_messages: 10"));
    }
}
