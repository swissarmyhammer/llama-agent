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
    fn should_stop(&mut self, _context: &LlamaContext, batch: &LlamaBatch) -> Option<FinishReason> {
        // Get the number of tokens in the current batch
        let tokens_in_batch = batch.n_tokens() as usize;

        // Update our token count with the tokens in this batch
        self.tokens_generated += tokens_in_batch;

        // Check if we've exceeded the maximum tokens limit
        if self.tokens_generated >= self.max_tokens {
            Some(FinishReason::Stopped("Maximum tokens reached".to_string()))
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_max_tokens_stopper_creation() {
        let max_tokens = 100;
        let stopper = MaxTokensStopper::new(max_tokens);

        assert_eq!(stopper.max_tokens, max_tokens);
        assert_eq!(stopper.tokens_generated, 0);
    }

    #[test]
    fn test_max_tokens_different_limits() {
        let test_cases = [0, 1, 10, 100, 1000, 10000];

        for max_tokens in test_cases {
            let stopper = MaxTokensStopper::new(max_tokens);
            assert_eq!(stopper.max_tokens, max_tokens);
            assert_eq!(stopper.tokens_generated, 0);
        }
    }

    #[test]
    fn test_stopper_trait_compliance() {
        // Verify MaxTokensStopper properly implements the Stopper trait
        let stopper = MaxTokensStopper::new(100);

        // Verify it can be stored as a trait object
        let _boxed: Box<dyn Stopper> = Box::new(stopper);
    }

    #[test]
    fn test_thread_safety() {
        // Test that MaxTokensStopper implements Send + Sync
        fn assert_send<T: Send>() {}
        fn assert_sync<T: Sync>() {}

        assert_send::<MaxTokensStopper>();
        assert_sync::<MaxTokensStopper>();
    }

    #[test]
    fn test_edge_cases() {
        // Test with very large max_tokens
        let stopper = MaxTokensStopper::new(usize::MAX);
        assert_eq!(stopper.max_tokens, usize::MAX);
        assert_eq!(stopper.tokens_generated, 0);

        // Test zero limit
        let zero_stopper = MaxTokensStopper::new(0);
        assert_eq!(zero_stopper.max_tokens, 0);
        assert_eq!(zero_stopper.tokens_generated, 0);
    }

    // Note: Integration tests with actual LlamaContext and LlamaBatch
    // using real model are implemented in integration_tests.rs to avoid
    // requiring model loading in unit tests.
    //
    // The should_stop method behavior with batch token counting has been
    // validated separately and works correctly with real batches.
}
