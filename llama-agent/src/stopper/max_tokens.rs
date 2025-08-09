use super::Stopper;
use crate::types::FinishReason;
use llama_cpp_2::{context::LlamaContext, llama_batch::LlamaBatch};

/// Stopper that limits generation to a maximum number of tokens
pub struct MaxTokensStopper {
    max_tokens: usize,
    tokens_generated: usize,
}

impl MaxTokensStopper {
    /// Create a new max tokens stopper
    pub fn new(max_tokens: usize) -> Self {
        Self {
            max_tokens,
            tokens_generated: 0,
        }
    }
}

impl Stopper for MaxTokensStopper {
    fn should_stop(
        &mut self,
        _context: &LlamaContext,
        _batch: &LlamaBatch,
    ) -> Option<FinishReason> {
        // TODO: Implementation will be added in STOPPING_000005
        None
    }
}
