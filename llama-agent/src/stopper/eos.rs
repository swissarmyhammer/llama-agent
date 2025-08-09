use super::{FinishReason, Stopper};
use llama_cpp_2::{context::LlamaContext, llama_batch::LlamaBatch};

/// Stopper that detects End-of-Sequence (EOS) tokens
pub struct EosStopper {
    eos_token_id: u32,
}

impl EosStopper {
    /// Create a new EOS stopper
    pub fn new(eos_token_id: u32) -> Self {
        Self { eos_token_id }
    }
}

impl Stopper for EosStopper {
    fn should_stop(&mut self, _context: &LlamaContext, _batch: &LlamaBatch) -> Option<FinishReason> {
        // TODO: Implementation will be added in STOPPING_000004
        None
    }
}