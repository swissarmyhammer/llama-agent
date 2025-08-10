//! # Cache Manager
//!
//! Provides efficient caching for downloaded models with LRU eviction and platform-appropriate
//! cache directories. Enables sharing between `llama-agent`, `llama-embedding`, and `llama-cli` crates.

use crate::error::ModelError;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::fs as async_fs;
use tracing::{debug, info, warn};

const DEFAULT_MAX_CACHE_SIZE_GB: u64 = 50;
const CACHE_METADATA_FILENAME: &str = "cache_metadata.json";

/// File metadata used for cache key generation
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FileMetadata {
    /// Size of the file in bytes
    pub size_bytes: u64,
    /// Last modified time as unix timestamp
    pub modified_time: u64,
}

impl FileMetadata {
    /// Create FileMetadata from a file path
    pub async fn from_path(path: &Path) -> Result<Self, ModelError> {
        let metadata = async_fs::metadata(path).await?;
        let size_bytes = metadata.len();
        let modified_time = metadata
            .modified()?
            .duration_since(UNIX_EPOCH)
            .map_err(|e| ModelError::Cache(format!("Invalid file modified time: {}", e)))?
            .as_secs();

        Ok(Self {
            size_bytes,
            modified_time,
        })
    }
}

/// Cache entry tracking information
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CacheEntry {
    /// Path to the cached model file
    pub path: PathBuf,
    /// Size of the cached file in bytes
    pub size_bytes: u64,
    /// Last access time as unix timestamp
    pub last_accessed: u64,
    /// Creation time as unix timestamp
    pub created_at: u64,
}

impl CacheEntry {
    /// Create a new cache entry
    pub fn new(path: PathBuf, size_bytes: u64) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        Self {
            path,
            size_bytes,
            last_accessed: now,
            created_at: now,
        }
    }

    /// Update the last accessed time to now
    pub fn touch(&mut self) {
        self.last_accessed = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
    }
}

/// Cache manager for model files with LRU eviction
#[derive(Debug)]
pub struct CacheManager {
    /// Directory where cached models are stored
    cache_dir: PathBuf,
    /// Maximum cache size in bytes
    max_cache_size_bytes: Option<u64>,
    /// Cache entries indexed by cache key
    entries: HashMap<String, CacheEntry>,
}

impl CacheManager {
    /// Create a new CacheManager with the specified cache directory
    pub fn new(cache_dir: PathBuf) -> Self {
        Self {
            cache_dir,
            max_cache_size_bytes: Some(DEFAULT_MAX_CACHE_SIZE_GB * 1024 * 1024 * 1024),
            entries: HashMap::new(),
        }
    }

    /// Create a CacheManager with platform-appropriate default cache directory
    pub fn with_default_cache_dir() -> Result<Self, ModelError> {
        let cache_dir = Self::get_platform_cache_dir()?;
        Ok(Self::new(cache_dir))
    }

    /// Set maximum cache size in GB
    pub fn with_max_size_gb(mut self, max_size_gb: u64) -> Self {
        self.max_cache_size_bytes = Some(max_size_gb * 1024 * 1024 * 1024);
        self
    }

    /// Disable cache size limits
    pub fn with_unlimited_size(mut self) -> Self {
        self.max_cache_size_bytes = None;
        self
    }

    /// Initialize the cache manager by loading existing metadata and ensuring directory exists
    pub async fn initialize(&mut self) -> Result<(), ModelError> {
        // Create cache directory if it doesn't exist
        async_fs::create_dir_all(&self.cache_dir).await?;

        // Load existing cache metadata
        self.load_metadata().await?;

        // Validate and clean up stale entries
        self.cleanup_stale_entries().await?;

        info!(
            "Cache manager initialized: {} entries, cache_dir={}",
            self.entries.len(),
            self.cache_dir.display()
        );

        Ok(())
    }

    /// Get platform-appropriate cache directory
    pub fn get_platform_cache_dir() -> Result<PathBuf, ModelError> {
        let cache_dir = if cfg!(target_os = "windows") {
            dirs::cache_dir()
                .or_else(|| std::env::var("LOCALAPPDATA").ok().map(PathBuf::from))
                .ok_or_else(|| {
                    ModelError::Cache("Unable to determine Windows cache directory".to_string())
                })?
                .join("llama-loader")
                .join("models")
        } else {
            // Linux/macOS
            dirs::cache_dir()
                .or_else(|| dirs::home_dir().map(|h| h.join(".cache")))
                .ok_or_else(|| {
                    ModelError::Cache("Unable to determine cache directory".to_string())
                })?
                .join("llama-loader")
                .join("models")
        };

        debug!("Platform cache directory: {}", cache_dir.display());
        Ok(cache_dir)
    }

    /// Generate cache key based on repo, filename, and file metadata
    pub fn generate_cache_key(repo: &str, filename: &str, metadata: &FileMetadata) -> String {
        let mut hasher = Sha256::new();
        hasher.update(repo.as_bytes());
        hasher.update(b"|");
        hasher.update(filename.as_bytes());
        hasher.update(b"|");
        hasher.update(metadata.size_bytes.to_le_bytes());
        hasher.update(b"|");
        hasher.update(metadata.modified_time.to_le_bytes());

        let result = hasher.finalize();
        format!("{:x}", result)
    }

    /// Check if a model is cached and return its path
    pub async fn get_cached_model(&mut self, cache_key: &str) -> Option<PathBuf> {
        let (path_result, should_save) = if let Some(entry) = self.entries.get_mut(cache_key) {
            // Check if cached file still exists
            if entry.path.exists() {
                entry.touch();
                debug!("Cache hit for key: {}", cache_key);
                (Some(entry.path.clone()), true)
            } else {
                // File was deleted, remove from cache
                debug!(
                    "Cached file no longer exists, removing entry: {}",
                    cache_key
                );
                (None, false)
            }
        } else {
            debug!("Cache miss for key: {}", cache_key);
            return None;
        };

        if should_save {
            self.save_metadata().await.ok()?;
        } else {
            self.entries.remove(cache_key);
            self.save_metadata().await.ok()?;
        }

        path_result
    }

    /// Cache a model file
    pub async fn cache_model(
        &mut self,
        model_path: &Path,
        cache_key: &str,
    ) -> Result<(), ModelError> {
        // Ensure cache directory exists
        async_fs::create_dir_all(&self.cache_dir).await?;

        // Get file metadata
        let metadata = async_fs::metadata(model_path).await?;
        let size_bytes = metadata.len();

        // Generate target path in cache
        let filename = model_path
            .file_name()
            .ok_or_else(|| ModelError::Cache("Invalid model file path".to_string()))?;
        let cached_path =
            self.cache_dir
                .join(format!("{}_{}", cache_key, filename.to_string_lossy()));

        // Copy file to cache if it doesn't already exist there
        if !cached_path.exists() {
            debug!(
                "Caching model: {} -> {}",
                model_path.display(),
                cached_path.display()
            );
            async_fs::copy(model_path, &cached_path).await?;
        } else {
            debug!("Model already cached at: {}", cached_path.display());
        }

        // Add to cache entries
        let entry = CacheEntry::new(cached_path, size_bytes);
        self.entries.insert(cache_key.to_string(), entry);

        // Enforce cache size limits
        if let Some(max_size) = self.max_cache_size_bytes {
            self.enforce_size_limit(max_size).await?;
        }

        // Save metadata
        self.save_metadata().await?;

        info!("Model cached with key: {}", cache_key);
        Ok(())
    }

    /// Cleanup old models using LRU eviction
    pub async fn cleanup_old_models(&mut self) -> Result<(), ModelError> {
        if let Some(max_size) = self.max_cache_size_bytes {
            self.enforce_size_limit(max_size).await?;
        }
        Ok(())
    }

    /// Get current cache size in bytes
    pub fn get_cache_size_bytes(&self) -> u64 {
        self.entries.values().map(|e| e.size_bytes).sum()
    }

    /// Get number of cached models
    pub fn get_cache_count(&self) -> usize {
        self.entries.len()
    }

    /// Enforce cache size limit using LRU eviction
    async fn enforce_size_limit(&mut self, max_size_bytes: u64) -> Result<(), ModelError> {
        let current_size = self.get_cache_size_bytes();
        if current_size <= max_size_bytes {
            return Ok(());
        }

        info!(
            "Cache size ({} bytes) exceeds limit ({} bytes), starting LRU eviction",
            current_size, max_size_bytes
        );

        // Sort entries by last accessed time (oldest first)
        let mut entries_by_age: Vec<(String, CacheEntry)> = self.entries.drain().collect();
        entries_by_age.sort_by_key(|(_, entry)| entry.last_accessed);

        let mut total_size = 0u64;
        let mut kept_entries = HashMap::new();

        // Keep entries from newest to oldest until we're under the limit
        for (key, entry) in entries_by_age.into_iter().rev() {
            if total_size + entry.size_bytes <= max_size_bytes {
                total_size += entry.size_bytes;
                kept_entries.insert(key, entry);
            } else {
                // Remove the cached file
                if entry.path.exists() {
                    async_fs::remove_file(&entry.path).await?;
                    info!("Evicted cached model: {}", entry.path.display());
                }
            }
        }

        self.entries = kept_entries;
        self.save_metadata().await?;

        info!(
            "LRU eviction completed: {} entries remaining, {} bytes total",
            self.entries.len(),
            total_size
        );

        Ok(())
    }

    /// Remove entries for files that no longer exist
    async fn cleanup_stale_entries(&mut self) -> Result<(), ModelError> {
        let mut stale_keys = Vec::new();

        for (key, entry) in &self.entries {
            if !entry.path.exists() {
                stale_keys.push(key.clone());
            }
        }

        if !stale_keys.is_empty() {
            for key in stale_keys {
                self.entries.remove(&key);
                debug!("Removed stale cache entry: {}", key);
            }
            self.save_metadata().await?;
        }

        Ok(())
    }

    /// Load cache metadata from disk
    async fn load_metadata(&mut self) -> Result<(), ModelError> {
        let metadata_path = self.cache_dir.join(CACHE_METADATA_FILENAME);

        if !metadata_path.exists() {
            debug!("No existing cache metadata found");
            return Ok(());
        }

        match async_fs::read_to_string(&metadata_path).await {
            Ok(content) => match serde_json::from_str::<HashMap<String, CacheEntry>>(&content) {
                Ok(entries) => {
                    self.entries = entries;
                    debug!("Loaded {} cache entries from metadata", self.entries.len());
                }
                Err(e) => {
                    warn!("Failed to parse cache metadata, starting fresh: {}", e);
                    self.entries.clear();
                }
            },
            Err(e) => {
                warn!("Failed to read cache metadata, starting fresh: {}", e);
                self.entries.clear();
            }
        }

        Ok(())
    }

    /// Save cache metadata to disk
    async fn save_metadata(&self) -> Result<(), ModelError> {
        let metadata_path = self.cache_dir.join(CACHE_METADATA_FILENAME);

        let content = serde_json::to_string_pretty(&self.entries)
            .map_err(|e| ModelError::Cache(format!("Failed to serialize metadata: {}", e)))?;

        async_fs::write(&metadata_path, content).await?;
        debug!("Saved cache metadata with {} entries", self.entries.len());

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use tokio::fs::File;
    use tokio::io::AsyncWriteExt;

    async fn create_test_file(path: &Path, content: &[u8]) -> Result<(), std::io::Error> {
        let mut file = File::create(path).await?;
        file.write_all(content).await?;
        file.sync_all().await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_cache_manager_creation() {
        let temp_dir = TempDir::new().unwrap();
        let cache_manager = CacheManager::new(temp_dir.path().to_path_buf());

        assert_eq!(cache_manager.cache_dir, temp_dir.path());
        assert_eq!(
            cache_manager.max_cache_size_bytes,
            Some(DEFAULT_MAX_CACHE_SIZE_GB * 1024 * 1024 * 1024)
        );
    }

    #[tokio::test]
    async fn test_platform_cache_dir() {
        let cache_dir = CacheManager::get_platform_cache_dir().unwrap();
        assert!(cache_dir.to_string_lossy().contains("llama-loader"));
        assert!(cache_dir.to_string_lossy().contains("models"));
    }

    #[tokio::test]
    async fn test_cache_key_generation() {
        let metadata = FileMetadata {
            size_bytes: 1024,
            modified_time: 1234567890,
        };

        let key1 = CacheManager::generate_cache_key("repo/model", "model.gguf", &metadata);
        let key2 = CacheManager::generate_cache_key("repo/model", "model.gguf", &metadata);
        let key3 = CacheManager::generate_cache_key("repo/other", "model.gguf", &metadata);

        // Same inputs should produce same key
        assert_eq!(key1, key2);
        // Different inputs should produce different keys
        assert_ne!(key1, key3);
        // Keys should be hex strings
        assert!(key1.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[tokio::test]
    async fn test_file_metadata_from_path() {
        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("test.gguf");
        let test_content = b"test model content";

        create_test_file(&test_file, test_content).await.unwrap();

        let metadata = FileMetadata::from_path(&test_file).await.unwrap();
        assert_eq!(metadata.size_bytes, test_content.len() as u64);
        assert!(metadata.modified_time > 0);
    }

    #[tokio::test]
    async fn test_cache_operations() {
        let temp_dir = TempDir::new().unwrap();
        let mut cache_manager = CacheManager::new(temp_dir.path().join("cache"));
        cache_manager.initialize().await.unwrap();

        // Create a test model file
        let model_dir = temp_dir.path().join("models");
        tokio::fs::create_dir_all(&model_dir).await.unwrap();
        let model_file = model_dir.join("test.gguf");
        let test_content = b"test model content for caching";
        create_test_file(&model_file, test_content).await.unwrap();

        // Generate cache key
        let metadata = FileMetadata::from_path(&model_file).await.unwrap();
        let cache_key = CacheManager::generate_cache_key("test/repo", "test.gguf", &metadata);

        // Should be cache miss initially
        let result = cache_manager.get_cached_model(&cache_key).await;
        assert!(result.is_none());

        // Cache the model
        cache_manager
            .cache_model(&model_file, &cache_key)
            .await
            .unwrap();

        // Should be cache hit now
        let result = cache_manager.get_cached_model(&cache_key).await;
        assert!(result.is_some());
        let cached_path = result.unwrap();
        assert!(cached_path.exists());

        // Verify cached file content
        let cached_content = tokio::fs::read(&cached_path).await.unwrap();
        assert_eq!(cached_content, test_content);

        // Verify cache statistics
        assert_eq!(cache_manager.get_cache_count(), 1);
        assert_eq!(
            cache_manager.get_cache_size_bytes(),
            test_content.len() as u64
        );
    }

    #[tokio::test]
    async fn test_cache_size_limits() {
        let temp_dir = TempDir::new().unwrap();
        let mut cache_manager =
            CacheManager::new(temp_dir.path().join("cache")).with_max_size_gb(0); // Very small limit to force eviction
        cache_manager.max_cache_size_bytes = Some(50); // 50 bytes limit
        cache_manager.initialize().await.unwrap();

        // Create test model files
        let model_dir = temp_dir.path().join("models");
        tokio::fs::create_dir_all(&model_dir).await.unwrap();

        let model1 = model_dir.join("model1.gguf");
        let model2 = model_dir.join("model2.gguf");
        let content1 = b"content1_30bytes_exactly_here"; // 30 bytes
        let content2 = b"content2_30bytes_exactly_here"; // 30 bytes

        create_test_file(&model1, content1).await.unwrap();
        create_test_file(&model2, content2).await.unwrap();

        // Cache first model
        let metadata1 = FileMetadata::from_path(&model1).await.unwrap();
        let key1 = CacheManager::generate_cache_key("test/repo1", "model1.gguf", &metadata1);
        cache_manager.cache_model(&model1, &key1).await.unwrap();
        assert_eq!(cache_manager.get_cache_count(), 1);

        // Cache second model - should trigger eviction of first
        let metadata2 = FileMetadata::from_path(&model2).await.unwrap();
        let key2 = CacheManager::generate_cache_key("test/repo2", "model2.gguf", &metadata2);
        cache_manager.cache_model(&model2, &key2).await.unwrap();

        // Should only have one model cached due to size limit
        assert_eq!(cache_manager.get_cache_count(), 1);
        assert!(cache_manager.get_cached_model(&key2).await.is_some());
        assert!(cache_manager.get_cached_model(&key1).await.is_none());
    }

    #[tokio::test]
    async fn test_cache_persistence() {
        let temp_dir = TempDir::new().unwrap();
        let cache_dir = temp_dir.path().join("cache");

        // Create and populate cache manager
        {
            let mut cache_manager = CacheManager::new(cache_dir.clone());
            cache_manager.initialize().await.unwrap();

            // Create and cache a model
            let model_dir = temp_dir.path().join("models");
            tokio::fs::create_dir_all(&model_dir).await.unwrap();
            let model_file = model_dir.join("persistent_test.gguf");
            let test_content = b"persistent test content";
            create_test_file(&model_file, test_content).await.unwrap();

            let metadata = FileMetadata::from_path(&model_file).await.unwrap();
            let cache_key = CacheManager::generate_cache_key(
                "test/persistent",
                "persistent_test.gguf",
                &metadata,
            );
            cache_manager
                .cache_model(&model_file, &cache_key)
                .await
                .unwrap();

            assert_eq!(cache_manager.get_cache_count(), 1);
        }

        // Create new cache manager with same directory - should load existing data
        {
            let mut cache_manager = CacheManager::new(cache_dir);
            cache_manager.initialize().await.unwrap();

            // Should have loaded the cached model
            assert_eq!(cache_manager.get_cache_count(), 1);
        }
    }
}
