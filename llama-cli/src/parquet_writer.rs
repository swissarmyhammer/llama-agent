use llama_embedding::types::EmbeddingResult;
use polars::prelude::*;
use std::path::Path;
use thiserror::Error;
use tracing::{debug, info};

/// Error types for Parquet operations
#[derive(Error, Debug)]
pub enum ParquetError {
    #[error("Polars error: {0}")]
    Polars(#[from] PolarsError),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Schema mismatch: expected {expected} dimensions, got {actual}")]
    SchemaMismatch { expected: usize, actual: usize },

    #[error("Batch size must be greater than 0")]
    InvalidBatchSize,

    #[error("Empty embedding vector")]
    EmptyEmbedding,

    #[error("Buffer overflow: batch buffer full")]
    BufferOverflow,

    #[error("Conversion error: {0}")]
    Conversion(String),
}

/// Writer for efficiently writing embedding results to Parquet files
pub struct ParquetWriter {
    /// Output file path  
    output_path: std::path::PathBuf,
    /// Batch buffer to accumulate results before writing
    batch_buffer: Vec<EmbeddingResult>,
    /// Maximum batch size before automatic flush
    batch_size: usize,
    /// Expected embedding dimension
    embedding_dim: usize,
    /// Number of records written
    records_written: usize,
    /// Whether file has been written (for append mode)
    file_written: bool,
}

impl ParquetWriter {
    /// Create a new ParquetWriter
    ///
    /// # Arguments
    /// * `output_path` - Path where the Parquet file will be written
    /// * `embedding_dim` - Expected dimension of embedding vectors
    /// * `batch_size` - Number of records to buffer before writing a batch
    ///
    /// # Returns
    /// * `Result<Self, ParquetError>` - New ParquetWriter or error
    pub fn new(
        output_path: &Path,
        embedding_dim: usize,
        batch_size: usize,
    ) -> Result<Self, ParquetError> {
        if batch_size == 0 {
            return Err(ParquetError::InvalidBatchSize);
        }

        if embedding_dim == 0 {
            return Err(ParquetError::EmptyEmbedding);
        }

        debug!(
            "Creating ParquetWriter: path={:?}, embedding_dim={}, batch_size={}",
            output_path, embedding_dim, batch_size
        );

        info!(
            "ParquetWriter created successfully: path={:?}, embedding_dim={}",
            output_path, embedding_dim
        );

        Ok(Self {
            output_path: output_path.to_path_buf(),
            batch_buffer: Vec::with_capacity(batch_size),
            batch_size,
            embedding_dim,
            records_written: 0,
            file_written: false,
        })
    }

    /// Write a batch of embedding results
    ///
    /// # Arguments
    /// * `results` - Vector of embedding results to write
    ///
    /// # Returns
    /// * `Result<(), ParquetError>` - Success or error
    pub fn write_batch(&mut self, results: Vec<EmbeddingResult>) -> Result<(), ParquetError> {
        if results.is_empty() {
            debug!("Skipping empty batch");
            return Ok(());
        }

        debug!("Writing batch of {} results", results.len());

        // Validate embedding dimensions
        for result in results.iter() {
            if result.embedding.len() != self.embedding_dim {
                return Err(ParquetError::SchemaMismatch {
                    expected: self.embedding_dim,
                    actual: result.embedding.len(),
                });
            }
        }

        // Convert to DataFrame and write
        self.write_dataframe(&results)?;
        self.records_written += results.len();

        info!(
            "Wrote batch of {} records (total: {})",
            results.len(),
            self.records_written
        );
        Ok(())
    }

    /// Add a single result to the internal buffer and flush if necessary
    ///
    /// # Arguments
    /// * `result` - Single embedding result to add
    ///
    /// # Returns
    /// * `Result<(), ParquetError>` - Success or error
    pub fn add_result(&mut self, result: EmbeddingResult) -> Result<(), ParquetError> {
        // Validate embedding dimension
        if result.embedding.len() != self.embedding_dim {
            return Err(ParquetError::SchemaMismatch {
                expected: self.embedding_dim,
                actual: result.embedding.len(),
            });
        }

        // Add to buffer
        self.batch_buffer.push(result);

        // Flush if buffer is full
        if self.batch_buffer.len() >= self.batch_size {
            self.flush_buffer()?;
        }

        Ok(())
    }

    /// Flush any remaining buffered results
    ///
    /// # Returns
    /// * `Result<(), ParquetError>` - Success or error
    pub fn flush(&mut self) -> Result<(), ParquetError> {
        debug!("Flushing ParquetWriter");

        // Flush any buffered results
        if !self.batch_buffer.is_empty() {
            self.flush_buffer()?;
        }

        info!("ParquetWriter flushed successfully");
        Ok(())
    }

    /// Close the writer and return file info
    ///
    /// # Returns
    /// * `Result<usize, ParquetError>` - Number of records written or error
    pub fn close(mut self) -> Result<usize, ParquetError> {
        debug!("Closing ParquetWriter");

        // Flush any remaining data
        self.flush()?;

        info!(
            "ParquetWriter closed successfully: {} records written to {:?}",
            self.records_written, self.output_path
        );

        Ok(self.records_written)
    }

    /// Get the number of records written so far
    pub fn records_written(&self) -> usize {
        self.records_written
    }

    /// Get the embedding dimension
    pub fn embedding_dimension(&self) -> usize {
        self.embedding_dim
    }

    /// Get the batch size
    pub fn batch_size(&self) -> usize {
        self.batch_size
    }

    /// Flush the internal buffer
    fn flush_buffer(&mut self) -> Result<(), ParquetError> {
        if self.batch_buffer.is_empty() {
            return Ok(());
        }

        let results = std::mem::take(&mut self.batch_buffer);
        self.write_dataframe(&results)?;
        self.records_written += results.len();

        debug!("Flushed buffer: {} records", results.len());
        Ok(())
    }

    /// Write embedding results as a DataFrame to Parquet
    fn write_dataframe(&mut self, results: &[EmbeddingResult]) -> Result<(), ParquetError> {
        if results.is_empty() {
            return Ok(());
        }

        let num_records = results.len();
        debug!("Converting {} results to DataFrame", num_records);

        // Extract fields into separate vectors
        let texts: Vec<&str> = results.iter().map(|r| r.text.as_str()).collect();
        let text_hashes: Vec<&str> = results.iter().map(|r| r.text_hash.as_str()).collect();
        let sequence_lengths: Vec<u32> = results.iter().map(|r| r.sequence_length as u32).collect();
        let processing_times: Vec<u64> = results.iter().map(|r| r.processing_time_ms).collect();

        // Note: We'll add embeddings as separate columns below

        // Create the main DataFrame
        let mut df = DataFrame::new(vec![
            Series::new("text", texts),
            Series::new("text_hash", text_hashes),
            Series::new("sequence_length", sequence_lengths),
            Series::new("processing_time_ms", processing_times),
        ])?;

        // Add embeddings as a struct column with fixed fields
        let embedding_cols: Vec<Series> = (0..self.embedding_dim)
            .map(|i| {
                let values: Vec<f32> = results.iter().map(|r| r.embedding[i]).collect();
                Series::new(&format!("emb_{}", i), values)
            })
            .collect();

        // Add each embedding dimension as a separate column
        for col in embedding_cols {
            df = df.lazy().with_column(col.lit()).collect()?;
        }

        debug!(
            "DataFrame created: {} rows, {} columns",
            df.height(),
            df.width()
        );

        // Write to Parquet file
        if self.file_written {
            // Append mode - read existing file, concatenate, and write
            let existing_df =
                LazyFrame::scan_parquet(&self.output_path, ScanArgsParquet::default())?
                    .collect()?;

            let combined_df =
                concat([existing_df.lazy(), df.lazy()], UnionArgs::default())?.collect()?;

            let rows = combined_df.height();
            combined_df
                .lazy()
                .sink_parquet(&self.output_path, ParquetWriteOptions::default())?;

            debug!("DataFrame written to Parquet: {} rows (appended)", rows);
        } else {
            // First write
            let rows = df.height();
            df.lazy()
                .sink_parquet(&self.output_path, ParquetWriteOptions::default())?;
            self.file_written = true;
            debug!("DataFrame written to Parquet: {} rows (first write)", rows);
        };

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use llama_embedding::types::EmbeddingResult;
    use tempfile::NamedTempFile;

    #[test]
    fn test_parquet_writer_creation() {
        let temp_file = NamedTempFile::new().unwrap();
        let writer = ParquetWriter::new(temp_file.path(), 384, 32);

        assert!(writer.is_ok());
        let writer = writer.unwrap();
        assert_eq!(writer.embedding_dimension(), 384);
        assert_eq!(writer.batch_size(), 32);
        assert_eq!(writer.records_written(), 0);
    }

    #[test]
    fn test_parquet_writer_invalid_params() {
        let temp_file = NamedTempFile::new().unwrap();

        // Test zero batch size
        let result = ParquetWriter::new(temp_file.path(), 384, 0);
        assert!(matches!(result, Err(ParquetError::InvalidBatchSize)));

        // Test zero embedding dimension
        let result = ParquetWriter::new(temp_file.path(), 0, 32);
        assert!(matches!(result, Err(ParquetError::EmptyEmbedding)));
    }

    #[test]
    fn test_write_single_batch() {
        let temp_file = NamedTempFile::new().unwrap();
        let mut writer = ParquetWriter::new(temp_file.path(), 3, 32).unwrap();

        let results = vec![
            EmbeddingResult::new("hello world".to_string(), vec![0.1, 0.2, 0.3], 2, 100),
            EmbeddingResult::new("goodbye world".to_string(), vec![0.4, 0.5, 0.6], 2, 150),
        ];

        let result = writer.write_batch(results);
        assert!(result.is_ok());
        assert_eq!(writer.records_written(), 2);

        let records = writer.close();
        assert!(records.is_ok());
        assert_eq!(records.unwrap(), 2);
    }

    #[test]
    fn test_dimension_mismatch() {
        let temp_file = NamedTempFile::new().unwrap();
        let mut writer = ParquetWriter::new(temp_file.path(), 3, 32).unwrap();

        let results = vec![
            EmbeddingResult::new("hello".to_string(), vec![0.1, 0.2], 1, 100), // Wrong dimension
        ];

        let result = writer.write_batch(results);
        assert!(matches!(
            result,
            Err(ParquetError::SchemaMismatch {
                expected: 3,
                actual: 2
            })
        ));
    }

    #[test]
    fn test_add_result_with_auto_flush() {
        let temp_file = NamedTempFile::new().unwrap();
        let mut writer = ParquetWriter::new(temp_file.path(), 2, 2).unwrap(); // Small batch size

        // Add results one by one
        let result1 = EmbeddingResult::new("text1".to_string(), vec![0.1, 0.2], 1, 50);
        let result2 = EmbeddingResult::new("text2".to_string(), vec![0.3, 0.4], 1, 60);
        let result3 = EmbeddingResult::new("text3".to_string(), vec![0.5, 0.6], 1, 70);

        // First two should trigger auto-flush
        writer.add_result(result1).unwrap();
        assert_eq!(writer.records_written(), 0); // Not flushed yet

        writer.add_result(result2).unwrap();
        assert_eq!(writer.records_written(), 2); // Auto-flushed

        writer.add_result(result3).unwrap();
        assert_eq!(writer.records_written(), 2); // Third one is buffered

        let total_records = writer.close().unwrap();
        assert_eq!(total_records, 3); // Should have written 3 total records
    }

    #[test]
    fn test_file_output_readable() {
        let temp_file = NamedTempFile::new().unwrap();
        let temp_path = temp_file.path().to_path_buf();

        {
            let mut writer = ParquetWriter::new(&temp_path, 2, 10).unwrap();

            let results = vec![
                EmbeddingResult::new("test 1".to_string(), vec![1.0, 2.0], 2, 100),
                EmbeddingResult::new("test 2".to_string(), vec![3.0, 4.0], 2, 200),
            ];

            writer.write_batch(results).unwrap();
            writer.close().unwrap();
        }

        // Try to read the file back with Polars
        let df = LazyFrame::scan_parquet(&temp_path, ScanArgsParquet::default())
            .unwrap()
            .collect()
            .unwrap();

        assert_eq!(df.height(), 2);
        assert!(df.get_column_names().contains(&"text"));
        assert!(df.get_column_names().contains(&"text_hash"));
        assert!(df.get_column_names().contains(&"sequence_length"));
        assert!(df.get_column_names().contains(&"processing_time_ms"));
        assert!(df.get_column_names().contains(&"emb_0"));
        assert!(df.get_column_names().contains(&"emb_1"));
    }
}
