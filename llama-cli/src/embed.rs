use clap::Args;
use llama_loader::ModelSource;
use std::path::PathBuf;

#[derive(Args, Clone, Debug)]
#[command(about = "Generate embeddings for input texts")]
pub struct EmbedArgs {
    /// Model source (HuggingFace repo or local path)
    #[arg(long, short, help = "Model source (HuggingFace repo or local path)")]
    pub model: String,

    /// Optional model filename
    #[arg(long, help = "Optional specific model filename")]
    pub filename: Option<String>,

    /// Input text file (one text per line)
    #[arg(long, short, help = "Input text file (one text per line)")]
    pub input: PathBuf,

    /// Output Parquet file path
    #[arg(long, short, help = "Output Parquet file path")]
    pub output: PathBuf,

    /// Batch size for processing
    #[arg(long, default_value = "32", help = "Batch size for processing")]
    pub batch_size: usize,

    /// Normalize embeddings
    #[arg(long, help = "Normalize embeddings to unit length")]
    pub normalize: bool,

    /// Maximum sequence length
    #[arg(long, help = "Maximum sequence length for input texts")]
    pub max_length: Option<usize>,

    /// Enable debug output
    #[arg(long, help = "Enable debug output")]
    pub debug: bool,
}

/// Comprehensive validation function for EmbedArgs
pub fn validate_embed_args(args: &EmbedArgs) -> anyhow::Result<()> {
    // 1. Validate model using ModelSource validation
    validate_model_source(&args.model, &args.filename)?;

    // 2. Validate input file
    validate_input_file(&args.input)?;

    // 3. Validate output path
    validate_output_path(&args.output)?;

    // 4. Validate parameters
    validate_parameters(args.batch_size, args.max_length)?;

    Ok(())
}

/// Validate model source using ModelSource validation from llama-loader
fn validate_model_source(model: &str, filename: &Option<String>) -> anyhow::Result<()> {
    if model.is_empty() {
        return Err(anyhow::anyhow!(
            "Model path cannot be empty\n💡 Provide either a HuggingFace repo (e.g., 'microsoft/DialoGPT-medium') or local path"
        ));
    }

    // Create ModelSource based on CLI logic from to_embedding_config
    let model_source = if model.contains('/') && !std::path::Path::new(model).exists() {
        // Looks like HuggingFace repo
        ModelSource::HuggingFace {
            repo: model.to_string(),
            filename: filename.clone(),
        }
    } else {
        // Local path
        ModelSource::Local {
            folder: std::path::PathBuf::from(model),
            filename: filename.clone(),
        }
    };

    // Use ModelSource validation with better error handling
    model_source.validate().map_err(|e| {
        anyhow::anyhow!(
            "Model validation failed: {}\n💡 For local models: ensure path exists and contains .gguf files\n💡 For HuggingFace: use format 'org/repo' and verify repo exists",
            e
        )
    })
}

/// Validate input file exists, is readable, and contains data
fn validate_input_file(input: &std::path::Path) -> anyhow::Result<()> {
    if !input.exists() {
        return Err(anyhow::anyhow!(
            "Input file does not exist: {}\n💡 Ensure file path is correct and file exists",
            input.display()
        ));
    }

    if !input.is_file() {
        return Err(anyhow::anyhow!(
            "Input path is not a file: {}\n💡 Provide path to a text file, not a directory",
            input.display()
        ));
    }

    // Check if file is readable
    let metadata = std::fs::metadata(input).map_err(|e| {
        anyhow::anyhow!(
            "Cannot read input file metadata: {}: {}\n💡 Check file permissions and ensure file is accessible",
            input.display(),
            e
        )
    })?;

    if metadata.len() == 0 {
        return Err(anyhow::anyhow!(
            "Input file is empty: {}\n💡 Provide a text file with content (one text per line)",
            input.display()
        ));
    }

    // Test file readability by trying to open it
    std::fs::File::open(input).map_err(|e| {
        anyhow::anyhow!(
            "Cannot open input file: {}: {}\n💡 Check file permissions and ensure file is readable",
            input.display(),
            e
        )
    })?;

    Ok(())
}

/// Validate output path and ensure directory is writable
fn validate_output_path(output: &std::path::Path) -> anyhow::Result<()> {
    // Validate output file extension
    if let Some(extension) = output.extension() {
        if extension != "parquet" {
            return Err(anyhow::anyhow!(
                "Output file must have .parquet extension, got: {}\n💡 Use a .parquet extension for the output file",
                output.display()
            ));
        }
    } else {
        return Err(anyhow::anyhow!(
            "Output file must have .parquet extension: {}\n💡 Add .parquet extension to the output filename",
            output.display()
        ));
    }

    // Ensure output directory exists, creating it if necessary
    if let Some(parent) = output.parent() {
        if !parent.exists() {
            std::fs::create_dir_all(parent).map_err(|e| {
                anyhow::anyhow!(
                    "Failed to create output directory '{}': {}\n💡 Check parent directory permissions and disk space",
                    parent.display(),
                    e
                )
            })?;
        }

        // Test write permissions by attempting to create a temporary file
        let temp_file = parent.join(".embed_test_write");
        if let Err(e) = std::fs::File::create(&temp_file) {
            return Err(anyhow::anyhow!(
                "Cannot write to output directory '{}': {}\n💡 Check directory permissions and disk space",
                parent.display(),
                e
            ));
        }
        // Clean up test file
        let _ = std::fs::remove_file(temp_file);
    }

    Ok(())
}

/// Validate embedding parameters
fn validate_parameters(batch_size: usize, max_length: Option<usize>) -> anyhow::Result<()> {
    // Validate batch size
    if batch_size == 0 {
        return Err(anyhow::anyhow!(
            "Batch size must be greater than 0\n💡 Use a reasonable batch size like 32 or 64"
        ));
    }

    if batch_size > 1024 {
        return Err(anyhow::anyhow!(
            "Batch size is too large: {}. Maximum recommended is 1024\n💡 Large batch sizes may cause memory issues",
            batch_size
        ));
    }

    // Validate max_length if specified
    if let Some(max_length) = max_length {
        if max_length == 0 {
            return Err(anyhow::anyhow!(
                "Max length must be greater than 0\n💡 Use a reasonable sequence length like 512"
            ));
        }
        if max_length > 8192 {
            return Err(anyhow::anyhow!(
                "Max length is too large: {}. Maximum recommended is 8192\n💡 Very long sequences may cause memory issues and slow processing",
                max_length
            ));
        }
        if max_length < 32 {
            return Err(anyhow::anyhow!(
                "Max length is too small: {}. Minimum recommended is 32\n💡 Very short sequences may not capture meaningful semantics",
                max_length
            ));
        }
    }

    Ok(())
}

use crate::parquet_writer::ParquetWriter;
use indicatif::{ProgressBar, ProgressStyle};
use llama_embedding::{BatchProcessor, EmbeddingConfig, EmbeddingModel};
use std::sync::Arc;
use std::time::Instant;
use tracing::info;

impl EmbedArgs {
    /// Convert CLI args to embedding configuration
    fn to_embedding_config(&self) -> EmbeddingConfig {
        let model_source =
            if self.model.contains('/') && !std::path::Path::new(&self.model).exists() {
                // Looks like HuggingFace repo
                ModelSource::HuggingFace {
                    repo: self.model.clone(),
                    filename: self.filename.clone(),
                }
            } else {
                // Local path
                ModelSource::Local {
                    folder: std::path::PathBuf::from(&self.model),
                    filename: self.filename.clone(),
                }
            };

        EmbeddingConfig {
            model_source,
            normalize_embeddings: self.normalize,
            max_sequence_length: self.max_length,
            debug: self.debug,
        }
    }
}

/// Main embed command implementation
pub async fn run_embed_command(args: EmbedArgs) -> anyhow::Result<()> {
    // 1. Validate input arguments
    validate_embed_args(&args)?;

    info!("Starting embed command");
    info!("Model: {}", args.model);
    info!("Input: {:?}", args.input);
    info!("Output: {:?}", args.output);
    info!("Batch size: {}", args.batch_size);

    println!("Loading model: {}", args.model);
    let load_start = Instant::now();

    // 2. Create embedding config from CLI args
    let config = args.to_embedding_config();

    // 3. Initialize embedding model
    let mut embedding_model = EmbeddingModel::new(config)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to initialize embedding model: {}", e))?;

    // 4. Load the model
    embedding_model
        .load_model()
        .await
        .map_err(|e| anyhow::anyhow!("Failed to load model: {}", e))?;

    let load_time = load_start.elapsed();

    // 5. Get embedding dimensions for Parquet schema
    let embedding_dim = embedding_model
        .get_embedding_dimension()
        .ok_or_else(|| anyhow::anyhow!("Could not determine embedding dimensions"))?;

    println!(
        "Model loaded successfully in {:.1}s ({} dimensions)",
        load_time.as_secs_f64(),
        embedding_dim
    );

    // 6. Set up batch processor and Parquet writer
    let model = Arc::new(embedding_model);
    let mut processor = BatchProcessor::new(model.clone(), args.batch_size);
    let mut parquet_writer = ParquetWriter::new(&args.output, embedding_dim, args.batch_size)
        .map_err(|e| anyhow::anyhow!("Failed to create Parquet writer: {}", e))?;

    // 7. Count total lines for progress tracking
    let total_lines = count_non_empty_lines(&args.input).await?;
    println!(
        "Processing {} texts with batch size {}...",
        total_lines, args.batch_size
    );

    // 8. Create progress bar
    let progress_bar = ProgressBar::new(total_lines as u64);
    progress_bar.set_style(
        ProgressStyle::default_bar()
            .template("[{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({percent}%) {msg}")?
            .progress_chars("██▌ "),
    );

    let processing_start = Instant::now();
    let mut total_processed = 0;

    // 9. Process file and write to Parquet with progress tracking
    processor
        .process_file_streaming(&args.input, |batch| {
            let batch_size = batch.len();
            total_processed += batch_size;

            // Write batch to Parquet
            parquet_writer.write_batch(batch).map_err(|e| {
                llama_embedding::EmbeddingError::batch_processing(format!(
                    "Parquet write error: {}",
                    e
                ))
            })?;

            // Update progress
            progress_bar.set_position(total_processed as u64);
            if total_processed % (args.batch_size * 5) == 0 {
                // Update message every 5 batches
                let elapsed = processing_start.elapsed();
                let throughput = if elapsed.as_secs() > 0 {
                    total_processed as f64 / elapsed.as_secs_f64()
                } else {
                    0.0
                };
                progress_bar.set_message(format!("{:.1} texts/s", throughput));
            }

            Ok(())
        })
        .await
        .map_err(|e| anyhow::anyhow!("Failed to process file: {}", e))?;

    // 10. Finalize progress bar and close writer
    progress_bar.finish_with_message("Processing complete");
    let records_written = parquet_writer
        .close()
        .map_err(|e| anyhow::anyhow!("Failed to close Parquet writer: {}", e))?;

    let total_time = processing_start.elapsed();
    let throughput = if total_time.as_secs() > 0 {
        total_processed as f64 / total_time.as_secs_f64()
    } else {
        0.0
    };

    // 11. Show summary
    println!();
    println!("Processing complete!");
    println!("Total embeddings: {}", total_processed);
    println!("Processing time: {:.1}s", total_time.as_secs_f64());
    println!(
        "Average processing time: {:.1}ms per text",
        total_time.as_millis() as f64 / total_processed as f64
    );
    println!("Throughput: {:.1} texts/s", throughput);
    println!(
        "Output written to: {} ({} records)",
        args.output.display(),
        records_written
    );

    // Calculate and show file size
    if let Ok(metadata) = std::fs::metadata(&args.output) {
        let size_mb = metadata.len() as f64 / (1024.0 * 1024.0);
        println!("File size: {:.1} MB", size_mb);
    }

    Ok(())
}

/// Count non-empty lines in a file for progress tracking
async fn count_non_empty_lines(input_path: &std::path::Path) -> anyhow::Result<usize> {
    use tokio::fs::File;
    use tokio::io::{AsyncBufReadExt, BufReader};

    let file = File::open(input_path).await?;
    let reader = BufReader::new(file);
    let mut lines = reader.lines();
    let mut count = 0;

    while let Some(line) = lines.next_line().await? {
        if !line.trim().is_empty() {
            count += 1;
        }
    }

    Ok(count)
}

/// Legacy function name for compatibility
pub async fn run_embed(args: EmbedArgs) -> anyhow::Result<()> {
    run_embed_command(args).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::{tempdir, NamedTempFile};

    /// Create a valid EmbedArgs for testing
    fn create_valid_embed_args() -> anyhow::Result<(EmbedArgs, tempfile::TempDir)> {
        let temp_dir = tempdir()?;

        // Create a test input file
        let input_file = temp_dir.path().join("input.txt");
        fs::write(&input_file, "Hello world\nTest content\n")?;

        let args = EmbedArgs {
            model: "microsoft/DialoGPT-medium".to_string(),
            filename: None,
            input: input_file,
            output: temp_dir.path().join("output.parquet"),
            batch_size: 32,
            normalize: false,
            max_length: Some(512),
            debug: false,
        };

        Ok((args, temp_dir))
    }

    #[test]
    fn test_validate_embed_args_valid() {
        let (args, _temp_dir) = create_valid_embed_args().expect("Failed to create test args");
        let result = validate_embed_args(&args);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_model_source_empty() {
        let result = validate_model_source("", &None);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Model path cannot be empty"));
    }

    #[test]
    fn test_validate_model_source_invalid_huggingface() {
        let result = validate_model_source("invalid-repo", &None);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Model validation failed"));
    }

    #[test]
    fn test_validate_model_source_valid_huggingface() {
        let result = validate_model_source("microsoft/DialoGPT-medium", &None);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_model_source_nonexistent_local() {
        // Use a path without '/' so it's treated as local, not HuggingFace
        let result = validate_model_source("nonexistent_local_model", &None);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Model validation failed"));
    }

    #[test]
    fn test_validate_model_source_valid_local() -> anyhow::Result<()> {
        let temp_dir = tempdir()?;
        let result = validate_model_source(temp_dir.path().to_str().unwrap(), &None);
        assert!(result.is_ok());
        Ok(())
    }

    #[test]
    fn test_validate_input_file_nonexistent() {
        let nonexistent_path = PathBuf::from("/nonexistent/file.txt");
        let result = validate_input_file(&nonexistent_path);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Input file does not exist"));
    }

    #[test]
    fn test_validate_input_file_directory() -> anyhow::Result<()> {
        let temp_dir = tempdir()?;
        let result = validate_input_file(temp_dir.path());
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Input path is not a file"));
        Ok(())
    }

    #[test]
    fn test_validate_input_file_empty() -> anyhow::Result<()> {
        let temp_file = NamedTempFile::new()?;
        let result = validate_input_file(temp_file.path());
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Input file is empty"));
        Ok(())
    }

    #[test]
    fn test_validate_input_file_valid() -> anyhow::Result<()> {
        let temp_file = NamedTempFile::new()?;
        fs::write(temp_file.path(), "Hello world\nTest content\n")?;
        let result = validate_input_file(temp_file.path());
        assert!(result.is_ok());
        Ok(())
    }

    #[test]
    fn test_validate_output_path_no_extension() {
        let output_path = PathBuf::from("/tmp/output");
        let result = validate_output_path(&output_path);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("must have .parquet extension"));
    }

    #[test]
    fn test_validate_output_path_wrong_extension() {
        let output_path = PathBuf::from("/tmp/output.txt");
        let result = validate_output_path(&output_path);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("must have .parquet extension"));
    }

    #[test]
    fn test_validate_output_path_valid() -> anyhow::Result<()> {
        let temp_dir = tempdir()?;
        let output_path = temp_dir.path().join("output.parquet");
        let result = validate_output_path(&output_path);
        assert!(result.is_ok());
        Ok(())
    }

    #[test]
    fn test_validate_parameters_zero_batch_size() {
        let result = validate_parameters(0, None);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Batch size must be greater than 0"));
    }

    #[test]
    fn test_validate_parameters_large_batch_size() {
        let result = validate_parameters(2048, None);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Batch size is too large"));
    }

    #[test]
    fn test_validate_parameters_zero_max_length() {
        let result = validate_parameters(32, Some(0));
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Max length must be greater than 0"));
    }

    #[test]
    fn test_validate_parameters_large_max_length() {
        let result = validate_parameters(32, Some(10000));
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Max length is too large"));
    }

    #[test]
    fn test_validate_parameters_small_max_length() {
        let result = validate_parameters(32, Some(16));
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Max length is too small"));
    }

    #[test]
    fn test_validate_parameters_valid() {
        let result = validate_parameters(32, Some(512));
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_parameters_valid_no_max_length() {
        let result = validate_parameters(64, None);
        assert!(result.is_ok());
    }

    /// Integration test with all validation components
    #[test]
    fn test_validate_embed_args_comprehensive() -> anyhow::Result<()> {
        let temp_dir = tempdir()?;

        // Create test input file
        let input_file = temp_dir.path().join("input.txt");
        fs::write(&input_file, "Hello world\nTest content\nMore text\n")?;

        // Test with various configurations
        let test_cases = vec![
            // Valid HuggingFace model
            EmbedArgs {
                model: "microsoft/DialoGPT-medium".to_string(),
                filename: None,
                input: input_file.clone(),
                output: temp_dir.path().join("output1.parquet"),
                batch_size: 32,
                normalize: false,
                max_length: Some(512),
                debug: false,
            },
            // Valid local model (using temp dir as placeholder)
            EmbedArgs {
                model: temp_dir.path().to_string_lossy().to_string(),
                filename: None,
                input: input_file.clone(),
                output: temp_dir.path().join("output2.parquet"),
                batch_size: 64,
                normalize: true,
                max_length: None,
                debug: true,
            },
        ];

        for args in test_cases {
            let result = validate_embed_args(&args);
            assert!(result.is_ok(), "Validation failed for args: {:?}", args);
        }

        Ok(())
    }

    /// Test error message quality and actionable suggestions
    #[test]
    fn test_error_messages_contain_suggestions() {
        // Test empty model
        let result = validate_model_source("", &None);
        assert!(result.is_err());
        let error_msg = result.unwrap_err().to_string();
        assert!(error_msg.contains("💡"));
        assert!(error_msg.contains("HuggingFace repo"));

        // Test wrong output extension
        let output_path = PathBuf::from("/tmp/output.txt");
        let result = validate_output_path(&output_path);
        assert!(result.is_err());
        let error_msg = result.unwrap_err().to_string();
        assert!(error_msg.contains("💡"));
        assert!(error_msg.contains(".parquet"));

        // Test zero batch size
        let result = validate_parameters(0, None);
        assert!(result.is_err());
        let error_msg = result.unwrap_err().to_string();
        assert!(error_msg.contains("💡"));
        assert!(error_msg.contains("reasonable batch size"));
    }
}
