pub mod embed;
pub mod generate;
pub mod parquet_writer;

#[cfg(test)]
mod test_embedding_dimensions;

#[cfg(test)]
mod test_parquet_compatibility;

pub use embed::{run_embed, validate_embed_args, EmbedArgs};
pub use generate::{run_generate, validate_generate_args, GenerateArgs};
pub use parquet_writer::{ParquetError, ParquetWriter};
