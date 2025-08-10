use clap::Args;
use std::path::PathBuf;

#[derive(Args, Clone)]
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

// Placeholder validation function for EmbedArgs
pub fn validate_embed_args(args: &EmbedArgs) -> anyhow::Result<()> {
    // Validate model path
    if args.model.is_empty() {
        return Err(anyhow::anyhow!("Model path cannot be empty"));
    }

    // Validate input file exists
    if !args.input.exists() {
        return Err(anyhow::anyhow!(
            "Input file does not exist: {}",
            args.input.display()
        ));
    }

    // Validate batch size
    if args.batch_size == 0 {
        return Err(anyhow::anyhow!("Batch size must be greater than 0"));
    }

    // Validate output directory exists or can be created
    if let Some(parent) = args.output.parent() {
        if !parent.exists() {
            return Err(anyhow::anyhow!(
                "Output directory does not exist: {}",
                parent.display()
            ));
        }
    }

    // Validate max_length if specified
    if let Some(max_length) = args.max_length {
        if max_length == 0 {
            return Err(anyhow::anyhow!("Max length must be greater than 0"));
        }
        if max_length > 8192 {
            return Err(anyhow::anyhow!(
                "Max length is too large: {}. Maximum recommended is 8192",
                max_length
            ));
        }
    }

    Ok(())
}

use llama_embedding::{BatchProcessor, EmbeddingConfig, EmbeddingModel};
use llama_loader::ModelSource;
use std::sync::Arc;
use std::time::Instant;
use crate::parquet_writer::ParquetWriter;
use tracing::info;
use indicatif::{ProgressBar, ProgressStyle};

impl EmbedArgs {
    /// Convert CLI args to embedding configuration
    fn to_embedding_config(&self) -> EmbeddingConfig {
        let model_source = if self.model.contains('/') && !std::path::Path::new(&self.model).exists() {
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
    let mut embedding_model = EmbeddingModel::new(config).await
        .map_err(|e| anyhow::anyhow!("Failed to initialize embedding model: {}", e))?;
    
    // 4. Load the model
    embedding_model.load_model().await
        .map_err(|e| anyhow::anyhow!("Failed to load model: {}", e))?;
    
    let load_time = load_start.elapsed();
    
    // 5. Get embedding dimensions for Parquet schema
    let embedding_dim = embedding_model.get_embedding_dimension()
        .ok_or_else(|| anyhow::anyhow!("Could not determine embedding dimensions"))?;
    
    println!("Model loaded successfully in {:.1}s ({} dimensions)", 
             load_time.as_secs_f64(), embedding_dim);

    // 6. Set up batch processor and Parquet writer
    let model = Arc::new(embedding_model);
    let mut processor = BatchProcessor::new(model.clone(), args.batch_size);
    let mut parquet_writer = ParquetWriter::new(&args.output, embedding_dim, args.batch_size)
        .map_err(|e| anyhow::anyhow!("Failed to create Parquet writer: {}", e))?;

    // 7. Count total lines for progress tracking
    let total_lines = count_non_empty_lines(&args.input).await?;
    println!("Processing {} texts with batch size {}...", total_lines, args.batch_size);

    // 8. Create progress bar
    let progress_bar = ProgressBar::new(total_lines as u64);
    progress_bar.set_style(
        ProgressStyle::default_bar()
            .template("[{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({percent}%) {msg}")?
            .progress_chars("██▌ ")
    );

    let processing_start = Instant::now();
    let mut total_processed = 0;

    // 9. Process file and write to Parquet with progress tracking
    processor.process_file_streaming(&args.input, |batch| {
        let batch_size = batch.len();
        total_processed += batch_size;
        
        // Write batch to Parquet
        parquet_writer.write_batch(batch)
            .map_err(|e| llama_embedding::EmbeddingError::batch_processing(format!("Parquet write error: {}", e)))?;
        
        // Update progress
        progress_bar.set_position(total_processed as u64);
        if total_processed % (args.batch_size * 5) == 0 {  // Update message every 5 batches
            let elapsed = processing_start.elapsed();
            let throughput = if elapsed.as_secs() > 0 {
                total_processed as f64 / elapsed.as_secs_f64()
            } else {
                0.0
            };
            progress_bar.set_message(format!("{:.1} texts/s", throughput));
        }
        
        Ok(())
    }).await
    .map_err(|e| anyhow::anyhow!("Failed to process file: {}", e))?;

    // 10. Finalize progress bar and close writer
    progress_bar.finish_with_message("Processing complete");
    let records_written = parquet_writer.close()
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
    println!("Average processing time: {:.1}ms per text", 
             total_time.as_millis() as f64 / total_processed as f64);
    println!("Throughput: {:.1} texts/s", throughput);
    println!("Output written to: {} ({} records)", args.output.display(), records_written);

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
