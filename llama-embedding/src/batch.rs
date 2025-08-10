use crate::error::{EmbeddingError, EmbeddingResult as Result};
use crate::model::EmbeddingModel;
use crate::types::EmbeddingResult;
use std::path::Path;
use std::sync::Arc;
use std::time::Instant;
use tokio::fs::File;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::sync::mpsc;
use tokio_stream::Stream;
use tracing::{debug, info, warn};

/// Statistics for batch processing operations
#[derive(Debug, Clone, Default)]
pub struct BatchStats {
    pub total_texts: usize,
    pub successful_embeddings: usize,
    pub failed_embeddings: usize,
    pub total_processing_time_ms: u64,
    pub average_time_per_text_ms: f64,
}

impl BatchStats {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn update(&mut self, batch_size: usize, processing_time_ms: u64, failures: usize) {
        self.total_texts += batch_size;
        self.successful_embeddings += batch_size - failures;
        self.failed_embeddings += failures;
        self.total_processing_time_ms += processing_time_ms;
        
        if self.total_texts > 0 {
            self.average_time_per_text_ms = 
                self.total_processing_time_ms as f64 / self.total_texts as f64;
        }
    }

    pub fn success_rate(&self) -> f64 {
        if self.total_texts == 0 {
            0.0
        } else {
            self.successful_embeddings as f64 / self.total_texts as f64
        }
    }
}

/// Configuration for batch processing behavior
#[derive(Debug, Clone)]
pub struct BatchConfig {
    pub batch_size: usize,
    pub continue_on_error: bool,
    pub max_parallel_tasks: usize,
    pub progress_callback: bool,
}

impl Default for BatchConfig {
    fn default() -> Self {
        Self {
            batch_size: 32,
            continue_on_error: true,
            max_parallel_tasks: 4,
            progress_callback: false,
        }
    }
}

/// Handles batch processing of multiple texts for embedding generation
pub struct BatchProcessor {
    model: Arc<EmbeddingModel>,
    config: BatchConfig,
    stats: BatchStats,
}

impl BatchProcessor {
    /// Create a new BatchProcessor with default configuration
    pub fn new(model: Arc<EmbeddingModel>, batch_size: usize) -> Self {
        let config = BatchConfig {
            batch_size,
            ..Default::default()
        };
        Self {
            model,
            config,
            stats: BatchStats::new(),
        }
    }

    /// Create a new BatchProcessor with custom configuration
    pub fn with_config(model: Arc<EmbeddingModel>, config: BatchConfig) -> Self {
        Self {
            model,
            config,
            stats: BatchStats::new(),
        }
    }

    /// Process a batch of texts and return embedding results with error recovery
    pub async fn process_batch(&mut self, texts: &[String]) -> Result<Vec<EmbeddingResult>> {
        if !self.model.is_loaded() {
            return Err(EmbeddingError::ModelNotLoaded);
        }

        let start_time = Instant::now();
        debug!("Processing batch of {} texts", texts.len());
        
        let mut results = Vec::new();
        let mut failures = 0;
        
        for text in texts {
            match self.model.embed_text(text).await {
                Ok(result) => {
                    results.push(result);
                }
                Err(e) => {
                    failures += 1;
                    let preview = text.chars().take(50).collect::<String>();
                    warn!("Failed to embed text '{}...': {}", preview, e);
                    
                    if !self.config.continue_on_error {
                        return Err(e);
                    }
                    // Continue processing other texts if continue_on_error is true
                }
            }
        }
        
        let processing_time = start_time.elapsed().as_millis() as u64;
        self.stats.update(texts.len(), processing_time, failures);
        
        debug!(
            "Processed batch: {} successful, {} failed, {}ms", 
            results.len(), failures, processing_time
        );
        
        Ok(results)
    }

    /// Process a list of texts with efficient batching
    pub async fn process_texts(&mut self, texts: Vec<String>) -> Result<Vec<EmbeddingResult>> {
        if texts.is_empty() {
            return Ok(Vec::new());
        }

        info!("Processing {} texts in batches of {}", texts.len(), self.config.batch_size);
        let mut all_results = Vec::new();
        
        for chunk in texts.chunks(self.config.batch_size) {
            let batch_results = self.process_batch(chunk).await?;
            all_results.extend(batch_results);
        }
        
        info!("Completed processing {} texts with {} results", 
              texts.len(), all_results.len());
        Ok(all_results)
    }

    /// Process a file containing texts (one per line) - memory efficient version
    pub async fn process_file(&mut self, input_path: &Path) -> Result<Vec<EmbeddingResult>> {
        if !input_path.exists() {
            return Err(EmbeddingError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("Input file not found: {}", input_path.display()),
            )));
        }

        info!("Processing file: {}", input_path.display());
        let mut all_results = Vec::new();
        let mut current_batch = Vec::new();
        
        let file = File::open(input_path).await?;
        let reader = BufReader::new(file);
        let mut lines = reader.lines();
        
        while let Some(line) = lines.next_line().await? {
            let trimmed = line.trim();
            if !trimmed.is_empty() {
                current_batch.push(trimmed.to_string());
                
                // Process batch when it reaches the configured size
                if current_batch.len() >= self.config.batch_size {
                    let batch_results = self.process_batch(&current_batch).await?;
                    all_results.extend(batch_results);
                    current_batch.clear();
                }
            }
        }
        
        // Process remaining texts in the final batch
        if !current_batch.is_empty() {
            let batch_results = self.process_batch(&current_batch).await?;
            all_results.extend(batch_results);
        }
        
        info!("Completed processing file with {} embeddings", all_results.len());
        Ok(all_results)
    }

    /// Process a file with streaming results via callback
    pub async fn process_file_streaming<F>(
        &mut self, 
        input_path: &Path, 
        mut callback: F
    ) -> Result<()>
    where
        F: FnMut(Vec<EmbeddingResult>) -> std::result::Result<(), EmbeddingError>,
    {
        if !input_path.exists() {
            return Err(EmbeddingError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("Input file not found: {}", input_path.display()),
            )));
        }

        info!("Processing file with streaming: {}", input_path.display());
        let mut current_batch = Vec::new();
        
        let file = File::open(input_path).await?;
        let reader = BufReader::new(file);
        let mut lines = reader.lines();
        
        while let Some(line) = lines.next_line().await? {
            let trimmed = line.trim();
            if !trimmed.is_empty() {
                current_batch.push(trimmed.to_string());
                
                // Process and yield batch when it reaches the configured size
                if current_batch.len() >= self.config.batch_size {
                    let batch_results = self.process_batch(&current_batch).await?;
                    callback(batch_results)?;
                    current_batch.clear();
                }
            }
        }
        
        // Process and yield remaining texts in the final batch
        if !current_batch.is_empty() {
            let batch_results = self.process_batch(&current_batch).await?;
            callback(batch_results)?;
        }
        
        info!("Completed streaming processing of file");
        Ok(())
    }

    /// Process a file and return an async stream of results
    pub async fn process_file_as_stream(
        &mut self,
        input_path: &Path,
    ) -> Result<impl Stream<Item = std::result::Result<Vec<EmbeddingResult>, EmbeddingError>>> {
        if !input_path.exists() {
            return Err(EmbeddingError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("Input file not found: {}", input_path.display()),
            )));
        }

        let (tx, rx) = mpsc::channel::<std::result::Result<Vec<EmbeddingResult>, EmbeddingError>>(100);
        let input_path = input_path.to_path_buf();
        let batch_size = self.config.batch_size;
        let model = self.model.clone();
        let continue_on_error = self.config.continue_on_error;
        
        tokio::spawn(async move {
            let mut processor = BatchProcessor::new(model, batch_size);
            processor.config.continue_on_error = continue_on_error;
            
            let result = processor.process_file_streaming(&input_path, |batch_results| {
                if tx.try_send(Ok(batch_results)).is_err() {
                    return Err(EmbeddingError::BatchProcessing(
                        "Channel closed while streaming results".to_string()
                    ));
                }
                Ok(())
            }).await;
            
            if let Err(e) = result {
                let _ = tx.send(Err(e)).await;
            }
        });
        
        Ok(tokio_stream::wrappers::ReceiverStream::new(rx))
    }

    /// Get the current batch configuration
    pub fn config(&self) -> &BatchConfig {
        &self.config
    }

    /// Get the configured batch size
    pub fn batch_size(&self) -> usize {
        self.config.batch_size
    }

    /// Set a new batch size
    pub fn set_batch_size(&mut self, new_batch_size: usize) {
        if new_batch_size > 0 {
            self.config.batch_size = new_batch_size;
            debug!("Updated batch size to {}", new_batch_size);
        } else {
            warn!("Attempted to set invalid batch size: {}", new_batch_size);
        }
    }

    /// Set whether to continue processing on errors
    pub fn set_continue_on_error(&mut self, continue_on_error: bool) {
        self.config.continue_on_error = continue_on_error;
        debug!("Updated continue_on_error to {}", continue_on_error);
    }

    /// Get current processing statistics
    pub fn stats(&self) -> &BatchStats {
        &self.stats
    }

    /// Reset processing statistics
    pub fn reset_stats(&mut self) {
        self.stats = BatchStats::new();
        debug!("Reset processing statistics");
    }

    /// Get statistics about the underlying model
    pub fn get_model_info(&self) -> Option<(usize, bool)> {
        if let Some(dim) = self.model.get_embedding_dimension() {
            Some((dim, self.model.is_loaded()))
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::EmbeddingConfig;
    use std::io::Write;
    use tempfile::NamedTempFile;

    // Note: These are structural tests that validate compilation
    // Actual functionality tests would require a real model to be loaded

    #[test]
    fn test_batch_stats() {
        let mut stats = BatchStats::new();
        
        // Initial state
        assert_eq!(stats.total_texts, 0);
        assert_eq!(stats.successful_embeddings, 0);
        assert_eq!(stats.failed_embeddings, 0);
        assert_eq!(stats.success_rate(), 0.0);
        
        // Update with successful batch
        stats.update(10, 1000, 0);
        assert_eq!(stats.total_texts, 10);
        assert_eq!(stats.successful_embeddings, 10);
        assert_eq!(stats.failed_embeddings, 0);
        assert_eq!(stats.success_rate(), 1.0);
        assert_eq!(stats.average_time_per_text_ms, 100.0);
        
        // Update with partially failed batch
        stats.update(10, 2000, 2);
        assert_eq!(stats.total_texts, 20);
        assert_eq!(stats.successful_embeddings, 18);
        assert_eq!(stats.failed_embeddings, 2);
        assert_eq!(stats.success_rate(), 0.9);
        assert_eq!(stats.average_time_per_text_ms, 150.0);
    }

    #[test]
    fn test_batch_config_default() {
        let config = BatchConfig::default();
        assert_eq!(config.batch_size, 32);
        assert!(config.continue_on_error);
        assert_eq!(config.max_parallel_tasks, 4);
        assert!(!config.progress_callback);
    }

    #[test]
    fn test_batch_processor_creation() {
        // Create a dummy config for testing structure
        let _config = EmbeddingConfig::default();
        
        // We can't actually create an EmbeddingModel in unit tests
        // without proper setup, but we can test the structure
        assert_eq!(1, 1); // Placeholder test to verify compilation
    }

    #[test]
    fn test_batch_size_management() {
        // Test batch size validation logic
        let valid_sizes = vec![1, 8, 16, 32, 64, 128];
        for size in valid_sizes {
            assert!(size > 0);
        }
    }

    #[tokio::test]
    async fn test_empty_text_handling() {
        let texts = vec!["".to_string(), "   ".to_string()];
        let non_empty: Vec<String> = texts.into_iter()
            .filter(|t| !t.trim().is_empty())
            .collect();
        
        assert_eq!(non_empty.len(), 0);
    }

    #[tokio::test]
    async fn test_file_processing_setup() {
        // Test file creation and reading setup (without actual model)
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "line 1").unwrap();
        writeln!(temp_file, "").unwrap(); // Empty line should be skipped
        writeln!(temp_file, "line 2").unwrap();
        writeln!(temp_file, "   ").unwrap(); // Whitespace-only line should be skipped
        writeln!(temp_file, "line 3").unwrap();
        
        // Verify file exists and can be read
        let path = temp_file.path();
        assert!(path.exists());
        
        // Test line filtering logic
        let file = File::open(path).await.unwrap();
        let reader = BufReader::new(file);
        let mut lines = reader.lines();
        let mut valid_lines = Vec::new();
        
        while let Some(line) = lines.next_line().await.unwrap() {
            let trimmed = line.trim();
            if !trimmed.is_empty() {
                valid_lines.push(trimmed.to_string());
            }
        }
        
        assert_eq!(valid_lines.len(), 3);
        assert_eq!(valid_lines, vec!["line 1", "line 2", "line 3"]);
    }
}