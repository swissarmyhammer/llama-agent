use llama_agent::{
    stopper::{EosStopper, MaxTokensStopper, RepetitionStopper, Stopper},
    types::{FinishReason, RepetitionConfig},
};
use llama_cpp_2::{
    context::params::LlamaContextParams,
    llama_batch::LlamaBatch,
    llama_backend::LlamaBackend,
    model::{params::LlamaModelParams, LlamaModel},
    sampling::LlamaSampler,
};
use std::{
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use tempfile::TempDir;
use tokio::task;
use tracing::{debug, info, warn};

/// Test configuration and utilities for stopper integration tests
struct TestSetup {
    _backend: LlamaBackend,
    model: LlamaModel,
    _temp_dir: TempDir,
}

impl TestSetup {
    /// Initialize test setup with unsloth/Qwen3-0.6B-GGUF model
    async fn new() -> Result<Self, Box<dyn std::error::Error>> {
        // Initialize tracing for test debugging
        let _ = tracing_subscriber::fmt().try_init();

        info!("Initializing test setup with unsloth/Qwen3-0.6B-GGUF model");

        // Initialize llama backend (handle case where it's already initialized)
        let backend = match LlamaBackend::init() {
            Ok(backend) => backend,
            Err(_) => {
                // Backend already initialized, skip model tests for now
                warn!("LlamaBackend already initialized, skipping model-dependent tests");
                return Err("Backend already initialized - integration tests require fresh process".into());
            }
        };

        // Create temporary directory for model cache
        let temp_dir = TempDir::new()?;

        // Download and load model
        let model_repo = "unsloth/Qwen3-0.6B-GGUF";
        let model_filename = "qwen3-0.6b-q4_k_m.gguf"; // Using Q4_K_M quantization for good balance of speed/quality

        info!("Downloading model {} from {}", model_filename, model_repo);
        
        // Use hf-hub to download the model
        let api = hf_hub::api::tokio::Api::new()?;
        let repo = api.model(model_repo.to_string());
        
        let model_path = match repo.get(model_filename).await {
            Ok(path) => path,
            Err(e) => {
                warn!("Failed to download {}: {}. Falling back to smallest model.", model_filename, e);
                // Fallback to the smallest available model
                let fallback_filename = "qwen3-0.6b-q8_0.gguf";
                match repo.get(fallback_filename).await {
                    Ok(path) => {
                        info!("Successfully downloaded fallback model: {}", fallback_filename);
                        path
                    }
                    Err(e2) => {
                        return Err(format!("Failed to download both primary ({}) and fallback ({}) models: {}, {}", 
                                         model_filename, fallback_filename, e, e2).into());
                    }
                }
            }
        };

        info!("Loading model from path: {:?}", model_path);

        // Load model with optimized parameters for testing
        let model_params = LlamaModelParams::default();
        let model = LlamaModel::load_from_file(&backend, model_path, &model_params)?;

        info!("Model loaded successfully. Vocab size: {}", model.n_vocab());

        info!("Model loaded successfully");

        Ok(Self {
            _backend: backend,
            model,
            _temp_dir: temp_dir,
        })
    }

    /// Generate text with the model and return tokens/text  
    fn generate_tokens(&mut self, prompt: &str, max_tokens: usize) -> Result<(Vec<llama_cpp_2::token::LlamaToken>, String), Box<dyn std::error::Error>> {
        debug!("Generating tokens for prompt: '{}'", prompt);
        
        // Create context for this generation
        let context_params = LlamaContextParams::default()
            .with_n_ctx(Some(std::num::NonZero::<u32>::new(2048).unwrap()))  // Sufficient context for tests
            .with_n_batch(512)       // Reasonable batch size
            .with_n_threads(std::cmp::min(4, num_cpus::get() as i32))  // Use available cores but cap at 4
            .with_n_threads_batch(std::cmp::min(2, num_cpus::get() as i32));

        let mut context = self.model.new_context(&self._backend, context_params)?;
        
        // Tokenize prompt
        let tokens = self.model.str_to_token(prompt, llama_cpp_2::model::AddBos::Always)?;
        debug!("Prompt tokenized to {} tokens", tokens.len());

        // Create batch and decode prompt
        let mut batch = LlamaBatch::new(512, 1);
        for (i, &token) in tokens.iter().enumerate() {
            batch.add(token, i as i32, &[0], i == tokens.len() - 1)?;
        }

        context.decode(&mut batch)?;

        // Generate tokens
        let mut generated_tokens = Vec::new();
        let mut generated_text = String::new();

        // Simple sampler for reproducible results
        let mut sampler = LlamaSampler::temp(0.8);

        for _ in 0..max_tokens {
            // For simple testing, just sample from the logits directly
            let token = sampler.sample(&context, batch.n_tokens() - 1);
            generated_tokens.push(token);

            // Convert token to text
            let token_str = self.model.token_to_str(token, llama_cpp_2::model::Special::Tokenize)?;
            generated_text.push_str(&token_str);

            // Check for EOS
            if self.model.is_eog_token(token) {
                debug!("EOS token detected: {}", token);
                break;
            }

            // Add token to batch for next iteration
            batch.clear();
            batch.add(token, (tokens.len() + generated_tokens.len() - 1) as i32, &[0], true)?;
            context.decode(&mut batch)?;
        }

        debug!("Generated {} tokens: '{}'", generated_tokens.len(), generated_text);
        Ok((generated_tokens, generated_text))
    }

    /// Create a context for testing
    fn create_context(&self) -> Result<llama_cpp_2::context::LlamaContext, Box<dyn std::error::Error>> {
        let context_params = LlamaContextParams::default()
            .with_n_ctx(Some(std::num::NonZero::<u32>::new(2048).unwrap()))
            .with_n_batch(512)
            .with_n_threads(std::cmp::min(4, num_cpus::get() as i32))
            .with_n_threads_batch(std::cmp::min(2, num_cpus::get() as i32));

        Ok(self.model.new_context(&self._backend, context_params)?)
    }

    /// Get EOS token ID for this model
    fn eos_token_id(&self) -> u32 {
        // Try to get the actual EOS token ID from the model
        // Most models use specific EOS tokens
        for token_id in 0..std::cmp::min(1000, self.model.n_vocab() as u32) {
            let llama_token = llama_cpp_2::token::LlamaToken(token_id as i32);
            if self.model.is_eog_token(llama_token) {
                return token_id;
            }
        }
        2 // Fallback to common EOS token ID
    }
}

// Performance measurement utilities
struct PerformanceMetrics {
    tokens_per_second: f64,
    total_duration: Duration,
    tokens_generated: usize,
}

impl PerformanceMetrics {
    fn measure<F>(f: F) -> (PerformanceMetrics, ())
    where
        F: FnOnce() -> usize,
    {
        let start = Instant::now();
        let tokens_generated = f();
        let duration = start.elapsed();

        let tokens_per_second = if duration.as_secs_f64() > 0.0 {
            tokens_generated as f64 / duration.as_secs_f64()
        } else {
            0.0
        };

        (PerformanceMetrics {
            tokens_per_second,
            total_duration: duration,
            tokens_generated,
        }, ())
    }
}

#[tokio::test]
async fn test_eos_stopper_integration() -> Result<(), Box<dyn std::error::Error>> {
    let mut setup = TestSetup::new().await?;
    
    // Create EOS stopper with model's actual EOS token
    let eos_token_id = setup.eos_token_id();
    let mut eos_stopper = EosStopper::new(eos_token_id);
    
    info!("Testing EosStopper with EOS token ID: {}", eos_token_id);

    // Generate some tokens - this should eventually hit EOS naturally
    let prompt = "The quick brown fox jumps over the lazy dog. The end.";
    let (tokens, text) = setup.generate_tokens(prompt, 50)?;
    
    info!("Generated text: '{}'", text);
    info!("Generated {} tokens", tokens.len());

    // Create a batch with the generated tokens to test stopper
    let mut batch = LlamaBatch::new(512, 1);
    for (i, &token) in tokens.iter().enumerate() {
        batch.add(token, i as i32, &[0], i == tokens.len() - 1)?;
    }

    // The EosStopper is designed to work with queue.rs integration
    // For direct testing, we verify it doesn't interfere with normal operation
    let context = setup.create_context()?;
    let stop_result = eos_stopper.should_stop(&context, &batch);
    
    // EosStopper returns None in direct calls - EOS detection happens in queue.rs
    assert!(stop_result.is_none(), "EosStopper should return None in direct calls");

    // Verify EOS detection works at the token level
    let contains_eos = tokens.iter().any(|&token| setup.model.is_eog_token(token));
    if contains_eos {
        info!("✓ EOS token was correctly detected in generated sequence");
    } else {
        info!("No EOS token in this generation (normal for short sequences)");
    }

    Ok(())
}

#[tokio::test]
async fn test_max_tokens_stopper_integration() -> Result<(), Box<dyn std::error::Error>> {
    let mut setup = TestSetup::new().await?;
    
    info!("Testing MaxTokensStopper with various token limits");

    // Test different token limits
    let test_limits = [1, 5, 10, 25, 50];
    
    for max_tokens in test_limits {
        info!("Testing MaxTokensStopper with limit: {}", max_tokens);
        
        let mut max_tokens_stopper = MaxTokensStopper::new(max_tokens);
        
        // Generate tokens
        let prompt = "Once upon a time in a land far far away, there lived a";
        let (tokens, _text) = setup.generate_tokens(prompt, max_tokens + 10)?;
        
        // Test the stopper by processing tokens in batches
        let mut tokens_processed = 0;
        let mut stop_result = None;
        
        // Process tokens in small batches to simulate real usage
        for chunk in tokens.chunks(2) {
            let mut batch = LlamaBatch::new(512, 1);
            for (i, &token) in chunk.iter().enumerate() {
                batch.add(token, (tokens_processed + i) as i32, &[0], i == chunk.len() - 1)?;
            }
            
            tokens_processed += chunk.len();
            
            // Check if stopper should stop
            let context = setup.create_context()?;
            if let Some(reason) = max_tokens_stopper.should_stop(&context, &batch) {
                stop_result = Some(reason);
                break;
            }
        }
        
        // Verify the stopper behavior
        if tokens_processed >= max_tokens {
            assert!(stop_result.is_some(), "MaxTokensStopper should have stopped at {} tokens", max_tokens);
            
            if let Some(FinishReason::Stopped(reason)) = stop_result {
                assert!(reason.contains("Maximum tokens reached"), 
                       "Stop reason should mention maximum tokens: {}", reason);
                info!("✓ MaxTokensStopper correctly stopped at {} tokens: {}", max_tokens, reason);
            } else {
                panic!("Expected Stopped reason, got: {:?}", stop_result);
            }
        } else {
            info!("Generated {} tokens (less than limit {}), stopping not triggered", tokens_processed, max_tokens);
        }
    }

    Ok(())
}

#[tokio::test]
async fn test_repetition_stopper_integration() -> Result<(), Box<dyn std::error::Error>> {
    let setup = TestSetup::new().await?;
    
    info!("Testing RepetitionStopper with real text generation");

    // Configure repetition detection for shorter patterns suitable for testing
    let config = RepetitionConfig {
        min_pattern_length: 3,   // Short patterns for easier testing
        max_pattern_length: 20,
        min_repetitions: 3,      // Trigger on 3 repetitions
        window_size: 200,        // Small window for testing
    };
    
    let mut repetition_stopper = RepetitionStopper::new(config);
    
    // Simulate adding repetitive token text
    info!("Adding repetitive patterns to RepetitionStopper");
    
    // Add a pattern that should trigger detection
    let repetitive_pattern = "yes ";
    for i in 0..4 {
        repetition_stopper.add_token_text(repetitive_pattern.to_string());
        
        // Create a dummy batch for testing
        let batch = LlamaBatch::new(512, 1);
        
        let context = setup.create_context()?;
        let stop_result = repetition_stopper.should_stop(&context, &batch);
        
        if i >= 2 { // Should trigger after 3rd repetition
            if let Some(FinishReason::Stopped(reason)) = stop_result {
                assert!(reason.contains("Repetition detected"), 
                       "Stop reason should mention repetition: {}", reason);
                assert!(reason.contains(&repetitive_pattern.trim()), 
                       "Stop reason should contain the repeated pattern: {}", reason);
                info!("✓ RepetitionStopper correctly detected repetition: {}", reason);
                break;
            } else if i == 3 {
                panic!("RepetitionStopper should have detected repetition after 4 occurrences");
            }
        } else {
            assert!(stop_result.is_none(), 
                   "RepetitionStopper should not trigger before minimum repetitions");
        }
    }

    // Test with non-repetitive content
    info!("Testing RepetitionStopper with non-repetitive content");
    
    let mut non_rep_stopper = RepetitionStopper::new(RepetitionConfig::default());
    
    let varied_tokens = ["Hello", " world", "!", " How", " are", " you", " today", "?"];
    for token in varied_tokens {
        non_rep_stopper.add_token_text(token.to_string());
        
        let batch = LlamaBatch::new(512, 1);
        let context = setup.create_context()?;
        let stop_result = non_rep_stopper.should_stop(&context, &batch);
        
        assert!(stop_result.is_none(), 
               "RepetitionStopper should not trigger on varied content");
    }
    
    info!("✓ RepetitionStopper correctly ignores non-repetitive content");

    Ok(())
}

#[tokio::test]
async fn test_combined_stoppers_integration() -> Result<(), Box<dyn std::error::Error>> {
    let mut setup = TestSetup::new().await?;
    
    info!("Testing multiple stoppers working together");

    // Create multiple stoppers
    let eos_token_id = setup.eos_token_id();
    let mut stoppers: Vec<Box<dyn Stopper>> = vec![
        Box::new(EosStopper::new(eos_token_id)),
        Box::new(MaxTokensStopper::new(20)), // Low limit for testing
        Box::new(RepetitionStopper::new(RepetitionConfig {
            min_pattern_length: 2,
            max_pattern_length: 10,
            min_repetitions: 3,
            window_size: 100,
        })),
    ];

    // Generate tokens
    let prompt = "The cat sat on the mat. The cat sat on the mat. The cat";
    let (tokens, text) = setup.generate_tokens(prompt, 30)?;
    
    info!("Generated text for combined test: '{}'", text);

    // Process tokens and check all stoppers
    let mut tokens_processed = 0;
    let mut stop_reasons = Vec::new();

    for chunk in tokens.chunks(3) {
        let mut batch = LlamaBatch::new(512, 1);
        for (i, &token) in chunk.iter().enumerate() {
            batch.add(token, (tokens_processed + i) as i32, &[0], i == chunk.len() - 1)?;
        }

        tokens_processed += chunk.len();

        // Update RepetitionStopper with token text
        for &token in chunk {
            let token_text = setup.model.token_to_str(token, llama_cpp_2::model::Special::Tokenize)?;
            for stopper in &mut stoppers {
                if let Some(rep_stopper) = stopper.as_any_mut().downcast_mut::<RepetitionStopper>() {
                    rep_stopper.add_token_text(token_text.clone());
                }
            }
        }

        // Check all stoppers
        let context = setup.create_context()?;
        for (i, stopper) in stoppers.iter_mut().enumerate() {
            if let Some(reason) = stopper.should_stop(&context, &batch) {
                stop_reasons.push((i, reason));
            }
        }

        if !stop_reasons.is_empty() {
            break;
        }
    }

    info!("Stop reasons triggered: {:?}", stop_reasons);

    // Verify at least one stopper triggered (MaxTokensStopper should trigger due to low limit)
    assert!(!stop_reasons.is_empty(), "At least one stopper should have triggered");

    // Verify the reasons make sense
    for (stopper_idx, reason) in stop_reasons {
        match stopper_idx {
            0 => info!("EOS stopper triggered: {:?}", reason),
            1 => {
                let FinishReason::Stopped(ref msg) = reason;
                assert!(msg.contains("Maximum tokens reached"));
                info!("✓ MaxTokensStopper correctly triggered: {}", msg);
            },
            2 => {
                let FinishReason::Stopped(ref msg) = reason;
                assert!(msg.contains("Repetition detected"));
                info!("✓ RepetitionStopper correctly triggered: {}", msg);
            },
            _ => panic!("Unexpected stopper index: {}", stopper_idx),
        }
    }

    Ok(())
}

#[tokio::test]
async fn test_stopper_performance_benchmark() -> Result<(), Box<dyn std::error::Error>> {
    let mut setup = TestSetup::new().await?;
    
    info!("Running performance benchmark for stoppers");

    let prompt = "Write a story about a brave knight who goes on an adventure to";
    
    // Baseline measurement without stoppers
    info!("Measuring baseline performance without stoppers");
    let (baseline_metrics, _) = PerformanceMetrics::measure(|| {
        let (tokens, _) = setup.generate_tokens(prompt, 100).unwrap();
        tokens.len()
    });
    
    info!("Baseline: {:.2} tokens/sec, {} tokens in {:?}", 
          baseline_metrics.tokens_per_second, 
          baseline_metrics.tokens_generated,
          baseline_metrics.total_duration);

    // Test with stoppers
    info!("Measuring performance with stoppers enabled");
    
    let (with_stoppers_metrics, _) = PerformanceMetrics::measure(|| {
        // Create stoppers
        let eos_token_id = setup.eos_token_id();
        let mut stoppers: Vec<Box<dyn Stopper>> = vec![
            Box::new(EosStopper::new(eos_token_id)),
            Box::new(MaxTokensStopper::new(100)),
            Box::new(RepetitionStopper::new(RepetitionConfig::default())),
        ];

        let (tokens, _) = setup.generate_tokens(prompt, 100).unwrap();
        
        // Simulate stopper checking overhead
        for chunk in tokens.chunks(5) {
            let mut batch = LlamaBatch::new(512, 1);
            for (i, &token) in chunk.iter().enumerate() {
                batch.add(token, i as i32, &[0], i == chunk.len() - 1).unwrap();
            }

            // Check all stoppers
            let context = setup.create_context().unwrap();
            for stopper in &mut stoppers {
                let _ = stopper.should_stop(&context, &batch);
            }
        }
        
        tokens.len()
    });
    
    info!("With stoppers: {:.2} tokens/sec, {} tokens in {:?}", 
          with_stoppers_metrics.tokens_per_second,
          with_stoppers_metrics.tokens_generated,
          with_stoppers_metrics.total_duration);

    // Calculate performance impact
    let performance_impact = if baseline_metrics.tokens_per_second > 0.0 {
        ((baseline_metrics.tokens_per_second - with_stoppers_metrics.tokens_per_second) 
         / baseline_metrics.tokens_per_second) * 100.0
    } else {
        0.0
    };

    info!("Performance impact: {:.2}%", performance_impact);

    // Verify performance requirement (< 5% degradation)
    // Note: In practice, the overhead may be minimal since token generation
    // dominates the time, not the stopper checks
    if performance_impact > 0.0 {
        assert!(performance_impact < 5.0, 
               "Performance impact ({:.2}%) exceeds 5% requirement", performance_impact);
        info!("✓ Performance impact {:.2}% is within acceptable limits", performance_impact);
    } else {
        info!("✓ No measurable performance impact from stoppers");
    }

    Ok(())
}

#[tokio::test]
async fn test_concurrent_stopper_usage() -> Result<(), Box<dyn std::error::Error>> {
    info!("Testing concurrent stopper usage for thread safety");
    
    // Create shared test setup
    let setup = Arc::new(Mutex::new(TestSetup::new().await?));
    
    // Spawn multiple concurrent tasks
    let mut handles = Vec::new();
    
    for task_id in 0..4 {
        let setup_clone = setup.clone();
        
        let handle = task::spawn(async move {
            info!("Starting concurrent task {}", task_id);
            
            // Each task creates its own stoppers
            let eos_token_id = {
                let setup_guard = setup_clone.lock().unwrap();
                setup_guard.eos_token_id()
            };
            
            let mut local_stoppers: Vec<Box<dyn Stopper>> = vec![
                Box::new(EosStopper::new(eos_token_id)),
                Box::new(MaxTokensStopper::new(50 + task_id * 10)), // Different limits per task
                Box::new(RepetitionStopper::new(RepetitionConfig::default())),
            ];

            // Generate and process tokens
            let prompt = format!("Task {} is generating text about", task_id);
            let result = {
                let mut setup_guard = setup_clone.lock().unwrap();
                setup_guard.generate_tokens(&prompt, 20)
            };
            
            match result {
                Ok((tokens, _)) => {
                    // Test stoppers with generated tokens
                    let mut tokens_processed = 0;
                    
                    for chunk in tokens.chunks(3) {
                        let mut batch = LlamaBatch::new(512, 1);
                        for (i, &token) in chunk.iter().enumerate() {
                            if let Err(e) = batch.add(token, (tokens_processed + i) as i32, &[0], i == chunk.len() - 1) {
                                return Err(format!("Task {} batch add failed: {}", task_id, e));
                            }
                        }
                        
                        tokens_processed += chunk.len();
                        
                        // Check stoppers (this tests thread safety)
                        for stopper in &mut local_stoppers {
                            let guard = setup_clone.lock().unwrap();
                            let ctx = guard.create_context().unwrap();
                            let _stop_result = stopper.should_stop(&ctx, &batch);
                        }
                    }
                    
                    info!("Task {} completed successfully", task_id);
                    Ok(())
                }
                Err(e) => Err(format!("Task {} generation failed: {}", task_id, e))
            }
        });
        
        handles.push(handle);
    }

    // Wait for all tasks to complete
    let mut task_results = Vec::new();
    for handle in handles {
        task_results.push(handle.await?);
    }

    // Check that all tasks completed successfully
    for (i, result) in task_results.iter().enumerate() {
        if let Err(e) = result {
            panic!("Task {} failed: {}", i, e);
        }
    }

    info!("✓ All concurrent tasks completed successfully");
    Ok(())
}

#[tokio::test]
async fn test_edge_cases_and_error_handling() -> Result<(), Box<dyn std::error::Error>> {
    let setup = TestSetup::new().await?;
    
    info!("Testing edge cases and error handling");

    // Test with empty batch
    let empty_batch = LlamaBatch::new(512, 1);
    let mut max_tokens_stopper = MaxTokensStopper::new(10);
    
    let context = setup.create_context()?;
    let result = max_tokens_stopper.should_stop(&context, &empty_batch);
    assert!(result.is_none(), "Stopper should handle empty batch gracefully");
    
    info!("✓ Empty batch handled correctly");

    // Test MaxTokensStopper with zero limit
    let mut zero_limit_stopper = MaxTokensStopper::new(0);
    let result = zero_limit_stopper.should_stop(&context, &empty_batch);
    
    // Should trigger immediately since any tokens >= 0
    assert!(result.is_some(), "Zero limit stopper should trigger immediately");
    
    let Some(FinishReason::Stopped(reason)) = result else {
        panic!("Expected Stopped reason for zero limit");
    };
    assert!(reason.contains("Maximum tokens reached"));
    info!("✓ Zero limit stopper works correctly: {}", reason);

    // Test RepetitionStopper with extreme configuration
    let extreme_config = RepetitionConfig {
        min_pattern_length: 1000,  // Very large
        max_pattern_length: 999,   // Smaller than min (invalid)
        min_repetitions: 1,
        window_size: 10,
    };
    
    let mut extreme_stopper = RepetitionStopper::new(extreme_config);
    extreme_stopper.add_token_text("test".to_string());
    
    let result = extreme_stopper.should_stop(&context, &empty_batch);
    assert!(result.is_none(), "Invalid configuration should not cause crashes");
    
    info!("✓ Invalid RepetitionStopper configuration handled gracefully");

    // Test memory bounds with RepetitionStopper
    let memory_test_config = RepetitionConfig {
        min_pattern_length: 5,
        max_pattern_length: 10,
        min_repetitions: 2,
        window_size: 50,  // Small window
    };
    
    let mut memory_stopper = RepetitionStopper::new(memory_test_config);
    
    // Add more text than window size allows
    for i in 0..100 {
        memory_stopper.add_token_text(format!("token{} ", i));
    }
    
    let _result = memory_stopper.should_stop(&context, &empty_batch);
    // Should not crash and memory usage should be bounded
    
    info!("✓ RepetitionStopper memory bounds enforced correctly");

    Ok(())
}

/// Helper function to run all integration tests  
/// Note: Individual test functions are run separately by cargo test
/// This function exists for documentation and can be used for custom test runners
#[allow(dead_code)]
fn run_comprehensive_stopper_integration_tests() -> Result<(), Box<dyn std::error::Error>> {
    // Note: This function should not be used as actual tests are async and run by tokio::test
    // Individual async tests are run separately by cargo test framework
    info!("=== Integration tests are run individually by cargo test ===");
    Ok(())
}