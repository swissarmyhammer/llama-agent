pub mod embed;
pub mod generate;

pub use embed::{run_embed, validate_embed_args, EmbedArgs};
pub use generate::{run_generate, validate_generate_args, GenerateArgs};
