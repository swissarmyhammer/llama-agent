use llama_cpp_2::{context::LlamaContext, llama_batch::LlamaBatch};

use crate::types::FinishReason;

// Stopper implementations
pub mod eos;
pub mod max_tokens;
pub mod repetition;

// Re-export stopper implementations
pub use eos::EosStopper;
pub use max_tokens::MaxTokensStopper;
pub use repetition::RepetitionStopper;

/// Trait for determining when to stop generation
pub trait Stopper {
    /// Evaluate whether generation should stop
    ///
    /// # Arguments
    /// * `context` - The LLAMA context containing model state
    /// * `batch` - The current batch being processed
    ///
    /// # Returns
    /// * `Some(FinishReason)` if generation should stop
    /// * `None` if generation should continue
    fn should_stop(&mut self, context: &LlamaContext, batch: &LlamaBatch) -> Option<FinishReason>;
}
