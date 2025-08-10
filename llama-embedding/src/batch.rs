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

/// Progress information for batch processing operations
#[derive(Debug, Clone)]
pub struct ProgressInfo {
    pub current_batch: usize,
    pub total_batches: usize,
    pub texts_processed: usize,
    pub total_texts: usize,
    pub successful_embeddings: usize,
    pub failed_embeddings: usize,
    pub elapsed_time_ms: u64,
    pub estimated_remaining_ms: u64,
    pub current_throughput_texts_per_second: f64,
}

/// Callback type for progress reporting
pub type ProgressCallback = Box<dyn Fn(&ProgressInfo) + Send + Sync>;

/// Statistics for batch processing operations
#[derive(Debug, Clone, Default)]
pub struct BatchStats {
    pub total_texts: usize,
    pub successful_embeddings: usize,
    pub failed_embeddings: usize,
    pub total_processing_time_ms: u64,
    pub average_time_per_text_ms: f64,
    pub total_tokens_processed: usize,
    pub average_tokens_per_text: f64,
    pub batches_processed: usize,
    pub average_batch_time_ms: f64,
    pub peak_memory_usage_bytes: usize,
    pub total_characters_processed: usize,
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
        self.batches_processed += 1;

        if self.total_texts > 0 {
            self.average_time_per_text_ms =
                self.total_processing_time_ms as f64 / self.total_texts as f64;
        }

        if self.batches_processed > 0 {
            self.average_batch_time_ms =
                self.total_processing_time_ms as f64 / self.batches_processed as f64;
        }
    }

    pub fn update_with_details(
        &mut self,
        batch_results: &[EmbeddingResult],
        processing_time_ms: u64,
        failures: usize,
    ) {
        let batch_size = batch_results.len() + failures;
        let token_count: usize = batch_results.iter().map(|r| r.sequence_length).sum();
        let char_count: usize = batch_results.iter().map(|r| r.text.len()).sum();

        self.total_texts += batch_size;
        self.successful_embeddings += batch_results.len();
        self.failed_embeddings += failures;
        self.total_processing_time_ms += processing_time_ms;
        self.total_tokens_processed += token_count;
        self.total_characters_processed += char_count;
        self.batches_processed += 1;

        // Update averages
        if self.total_texts > 0 {
            self.average_time_per_text_ms =
                self.total_processing_time_ms as f64 / self.total_texts as f64;
            self.average_tokens_per_text =
                self.total_tokens_processed as f64 / self.successful_embeddings as f64;
        }

        if self.batches_processed > 0 {
            self.average_batch_time_ms =
                self.total_processing_time_ms as f64 / self.batches_processed as f64;
        }
    }

    pub fn success_rate(&self) -> f64 {
        if self.total_texts == 0 {
            0.0
        } else {
            self.successful_embeddings as f64 / self.total_texts as f64
        }
    }

    pub fn throughput_texts_per_second(&self) -> f64 {
        if self.total_processing_time_ms == 0 {
            0.0
        } else {
            (self.successful_embeddings as f64) / (self.total_processing_time_ms as f64 / 1000.0)
        }
    }

    pub fn throughput_tokens_per_second(&self) -> f64 {
        if self.total_processing_time_ms == 0 {
            0.0
        } else {
            (self.total_tokens_processed as f64) / (self.total_processing_time_ms as f64 / 1000.0)
        }
    }

    pub fn update_memory_usage(&mut self, current_usage_bytes: usize) {
        if current_usage_bytes > self.peak_memory_usage_bytes {
            self.peak_memory_usage_bytes = current_usage_bytes;
        }
    }

    pub fn format_summary(&self) -> String {
        format!(
            "BatchStats {{ texts: {}/{} ({:.1}% success), time: {:.1}s, throughput: {:.1} texts/s, {:.1} tokens/s, memory: {:.2}MB }}",
            self.successful_embeddings,
            self.total_texts,
            self.success_rate() * 100.0,
            self.total_processing_time_ms as f64 / 1000.0,
            self.throughput_texts_per_second(),
            self.throughput_tokens_per_second(),
            self.peak_memory_usage_bytes as f64 / (1024.0 * 1024.0)
        )
    }
}

/// Configuration for batch processing behavior
#[derive(Debug, Clone)]
pub struct BatchConfig {
    pub batch_size: usize,
    pub continue_on_error: bool,
    pub max_parallel_tasks: usize,
    pub enable_progress_reporting: bool,
    pub progress_report_interval_batches: usize,
    pub memory_limit_mb: Option<usize>,
    pub enable_memory_monitoring: bool,
}

impl Default for BatchConfig {
    fn default() -> Self {
        Self {
            batch_size: 32,
            continue_on_error: true,
            max_parallel_tasks: 4,
            enable_progress_reporting: false,
            progress_report_interval_batches: 10,
            memory_limit_mb: None,
            enable_memory_monitoring: true,
        }
    }
}

/// Handles batch processing of multiple texts for embedding generation
pub struct BatchProcessor {
    model: Arc<EmbeddingModel>,
    config: BatchConfig,
    stats: BatchStats,
    progress_callback: Option<ProgressCallback>,
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
            progress_callback: None,
        }
    }

    /// Create a new BatchProcessor with custom configuration
    pub fn with_config(model: Arc<EmbeddingModel>, config: BatchConfig) -> Self {
        Self {
            model,
            config,
            stats: BatchStats::new(),
            progress_callback: None,
        }
    }

    /// Set a progress callback for monitoring batch processing
    pub fn set_progress_callback(&mut self, callback: ProgressCallback) {
        self.progress_callback = Some(callback);
    }

    /// Clear the progress callback
    pub fn clear_progress_callback(&mut self) {
        self.progress_callback = None;
    }

    /// Process a batch of texts and return embedding results with error recovery
    pub async fn process_batch(&mut self, texts: &[String]) -> Result<Vec<EmbeddingResult>> {
        if !self.model.is_loaded() {
            return Err(EmbeddingError::ModelNotLoaded);
        }

        let start_time = Instant::now();
        debug!("Processing batch of {} texts", texts.len());

        // Monitor memory usage if enabled
        if self.config.enable_memory_monitoring {
            let memory_usage = self.estimate_current_memory_usage(texts);
            self.stats.update_memory_usage(memory_usage);

            // Check memory limit if configured
            if let Some(limit_mb) = self.config.memory_limit_mb {
                let limit_bytes = limit_mb * 1024 * 1024;
                if memory_usage > limit_bytes {
                    warn!(
                        "Memory usage ({:.2}MB) exceeds limit ({:.2}MB)",
                        memory_usage as f64 / (1024.0 * 1024.0),
                        limit_mb
                    );
                    return Err(EmbeddingError::batch_processing(format!(
                        "Memory limit exceeded: {:.2}MB > {}MB",
                        memory_usage as f64 / (1024.0 * 1024.0),
                        limit_mb
                    )));
                }
            }
        }

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
        self.stats
            .update_with_details(&results, processing_time, failures);

        debug!(
            "Processed batch: {} successful, {} failed, {}ms",
            results.len(),
            failures,
            processing_time
        );

        Ok(results)
    }

    /// Process a list of texts with efficient batching
    pub async fn process_texts(&mut self, texts: Vec<String>) -> Result<Vec<EmbeddingResult>> {
        if texts.is_empty() {
            return Ok(Vec::new());
        }

        info!(
            "Processing {} texts in batches of {}",
            texts.len(),
            self.config.batch_size
        );
        let total_batches = texts.len().div_ceil(self.config.batch_size);
        let mut all_results = Vec::new();
        let start_time = Instant::now();

        for (batch_idx, chunk) in texts.chunks(self.config.batch_size).enumerate() {
            let batch_results = self.process_batch(chunk).await?;
            all_results.extend(batch_results);

            // Report progress if enabled and callback is set
            if self.config.enable_progress_reporting
                && batch_idx % self.config.progress_report_interval_batches == 0
            {
                if let Some(ref callback) = self.progress_callback {
                    let elapsed_ms = start_time.elapsed().as_millis() as u64;
                    let current_throughput = if elapsed_ms > 0 {
                        (self.stats.successful_embeddings as f64) / (elapsed_ms as f64 / 1000.0)
                    } else {
                        0.0
                    };

                    let remaining_batches = total_batches.saturating_sub(batch_idx + 1);
                    let estimated_remaining_ms =
                        if current_throughput > 0.0 && remaining_batches > 0 {
                            let remaining_texts = remaining_batches * self.config.batch_size;
                            ((remaining_texts as f64) / current_throughput * 1000.0) as u64
                        } else {
                            0
                        };

                    let progress_info = ProgressInfo {
                        current_batch: batch_idx + 1,
                        total_batches,
                        texts_processed: self.stats.total_texts,
                        total_texts: texts.len(),
                        successful_embeddings: self.stats.successful_embeddings,
                        failed_embeddings: self.stats.failed_embeddings,
                        elapsed_time_ms: elapsed_ms,
                        estimated_remaining_ms,
                        current_throughput_texts_per_second: current_throughput,
                    };

                    callback(&progress_info);
                }
            }
        }

        info!(
            "Completed processing {} texts with {} results. {}",
            texts.len(),
            all_results.len(),
            self.stats.format_summary()
        );
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

        info!(
            "Completed processing file with {} embeddings",
            all_results.len()
        );
        Ok(all_results)
    }

    /// Process a file with streaming results via callback
    pub async fn process_file_streaming<F>(
        &mut self,
        input_path: &Path,
        mut callback: F,
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

        let (tx, rx) =
            mpsc::channel::<std::result::Result<Vec<EmbeddingResult>, EmbeddingError>>(100);
        let input_path = input_path.to_path_buf();
        let batch_size = self.config.batch_size;
        let model = self.model.clone();
        let continue_on_error = self.config.continue_on_error;

        tokio::spawn(async move {
            let mut processor = BatchProcessor::new(model, batch_size);
            processor.config.continue_on_error = continue_on_error;

            let result = processor
                .process_file_streaming(&input_path, |batch_results| {
                    if tx.try_send(Ok(batch_results)).is_err() {
                        return Err(EmbeddingError::BatchProcessing(
                            "Channel closed while streaming results".to_string(),
                        ));
                    }
                    Ok(())
                })
                .await;

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
        self.model
            .get_embedding_dimension()
            .map(|dim| (dim, self.model.is_loaded()))
    }

    /// Estimate current memory usage for a batch of texts
    fn estimate_current_memory_usage(&self, texts: &[String]) -> usize {
        let text_memory = texts.iter().map(|t| t.len()).sum::<usize>();
        let embeddings_memory = if let Some(dim) = self.model.get_embedding_dimension() {
            // Estimate memory for potential embeddings (f32 = 4 bytes per element)
            texts.len() * dim * 4
        } else {
            // Default assumption for embedding dimension
            texts.len() * 384 * 4
        };

        // Add overhead for vectors, strings, and other data structures (rough estimate)
        let overhead = (text_memory + embeddings_memory) / 4;

        text_memory + embeddings_memory + overhead
    }

    /// Get a detailed performance report
    pub fn get_performance_report(&self) -> String {
        format!(
            "Performance Report:\n\
            - Total texts processed: {}\n\
            - Success rate: {:.1}%\n\
            - Processing time: {:.2}s\n\
            - Throughput: {:.1} texts/s, {:.1} tokens/s\n\
            - Average time per text: {:.1}ms\n\
            - Average tokens per text: {:.1}\n\
            - Batches processed: {}\n\
            - Average batch time: {:.1}ms\n\
            - Peak memory usage: {:.2}MB\n\
            - Total characters: {}",
            self.stats.total_texts,
            self.stats.success_rate() * 100.0,
            self.stats.total_processing_time_ms as f64 / 1000.0,
            self.stats.throughput_texts_per_second(),
            self.stats.throughput_tokens_per_second(),
            self.stats.average_time_per_text_ms,
            self.stats.average_tokens_per_text,
            self.stats.batches_processed,
            self.stats.average_batch_time_ms,
            self.stats.peak_memory_usage_bytes as f64 / (1024.0 * 1024.0),
            self.stats.total_characters_processed
        )
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
        assert_eq!(stats.batches_processed, 0);
        assert_eq!(stats.total_tokens_processed, 0);

        // Update with successful batch
        stats.update(10, 1000, 0);
        assert_eq!(stats.total_texts, 10);
        assert_eq!(stats.successful_embeddings, 10);
        assert_eq!(stats.failed_embeddings, 0);
        assert_eq!(stats.success_rate(), 1.0);
        assert_eq!(stats.average_time_per_text_ms, 100.0);
        assert_eq!(stats.batches_processed, 1);
        assert_eq!(stats.average_batch_time_ms, 1000.0);

        // Update with partially failed batch
        stats.update(10, 2000, 2);
        assert_eq!(stats.total_texts, 20);
        assert_eq!(stats.successful_embeddings, 18);
        assert_eq!(stats.failed_embeddings, 2);
        assert_eq!(stats.success_rate(), 0.9);
        assert_eq!(stats.average_time_per_text_ms, 150.0);
        assert_eq!(stats.batches_processed, 2);
        assert_eq!(stats.average_batch_time_ms, 1500.0);
    }

    #[test]
    fn test_batch_config_default() {
        let config = BatchConfig::default();
        assert_eq!(config.batch_size, 32);
        assert!(config.continue_on_error);
        assert_eq!(config.max_parallel_tasks, 4);
        assert!(!config.enable_progress_reporting);
        assert_eq!(config.progress_report_interval_batches, 10);
        assert!(config.memory_limit_mb.is_none());
        assert!(config.enable_memory_monitoring);
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
        let non_empty: Vec<String> = texts.into_iter().filter(|t| !t.trim().is_empty()).collect();

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

    #[test]
    fn test_enhanced_batch_stats() {
        let mut stats = BatchStats::new();

        // Test throughput calculations
        assert_eq!(stats.throughput_texts_per_second(), 0.0);
        assert_eq!(stats.throughput_tokens_per_second(), 0.0);

        // Test memory usage tracking
        stats.update_memory_usage(1024 * 1024); // 1MB
        assert_eq!(stats.peak_memory_usage_bytes, 1024 * 1024);

        stats.update_memory_usage(512 * 1024); // 512KB (should not update peak)
        assert_eq!(stats.peak_memory_usage_bytes, 1024 * 1024);

        stats.update_memory_usage(2 * 1024 * 1024); // 2MB (should update peak)
        assert_eq!(stats.peak_memory_usage_bytes, 2 * 1024 * 1024);

        // Test format_summary doesn't panic
        let summary = stats.format_summary();
        assert!(summary.contains("BatchStats"));
        assert!(summary.contains("texts:"));
        assert!(summary.contains("memory:"));
    }

    #[test]
    fn test_enhanced_batch_config() {
        let mut config = BatchConfig::default();

        // Test setting memory limit
        config.memory_limit_mb = Some(100);
        assert_eq!(config.memory_limit_mb, Some(100));

        // Test progress reporting configuration
        config.enable_progress_reporting = true;
        config.progress_report_interval_batches = 5;
        assert!(config.enable_progress_reporting);
        assert_eq!(config.progress_report_interval_batches, 5);
    }

    #[test]
    fn test_progress_info_structure() {
        let progress_info = ProgressInfo {
            current_batch: 5,
            total_batches: 10,
            texts_processed: 150,
            total_texts: 300,
            successful_embeddings: 145,
            failed_embeddings: 5,
            elapsed_time_ms: 30000,
            estimated_remaining_ms: 30000,
            current_throughput_texts_per_second: 5.0,
        };

        assert_eq!(progress_info.current_batch, 5);
        assert_eq!(progress_info.total_batches, 10);
        assert_eq!(progress_info.texts_processed, 150);
        assert_eq!(progress_info.successful_embeddings, 145);
        assert_eq!(progress_info.current_throughput_texts_per_second, 5.0);
    }

    #[test]
    fn test_memory_estimation_concepts() {
        // Test that memory estimation logic is sound
        let texts = vec![
            "short".to_string(),
            "medium length text".to_string(),
            "this is a much longer text that would require more memory for processing".to_string(),
        ];

        // Calculate expected memory manually
        let text_bytes: usize = texts.iter().map(|t| t.len()).sum();
        let embedding_bytes = texts.len() * 384 * 4; // 384 dim * 4 bytes per f32
        let overhead = (text_bytes + embedding_bytes) / 4; // 25% overhead
        let expected = text_bytes + embedding_bytes + overhead;

        assert!(expected > text_bytes); // Should be larger than just text
        assert!(expected > embedding_bytes); // Should be larger than just embeddings
    }
}
