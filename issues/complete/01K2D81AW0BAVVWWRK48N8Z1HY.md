The embeddings parquet needs one column for embeddings that is a fixed size float array, not a column per embedding dimension.

## Proposed Solution

After analyzing the current implementation in `llama-cli/src/parquet_writer.rs`, I can see that it currently creates separate columns for each embedding dimension (e.g., `emb_0`, `emb_1`, `emb_2`). This approach creates a variable number of columns based on embedding dimension, which is inefficient.

The solution is to:

1. **Replace individual embedding columns with a single fixed-size array column**: Instead of creating `emb_0`, `emb_1`, etc., create one column called `embedding` that stores the entire vector as a fixed-size float array.

2. **Use Polars List datatype with fixed length**: Polars supports List types that can store arrays. We'll use `List(Float32)` to store the embedding vectors as arrays within a single column.

3. **Update the schema creation logic**: Modify the `write_dataframe` method (lines 244-254) to create a single embedding column instead of multiple individual columns.

4. **Update tests**: Modify the test expectations to look for the `embedding` column instead of `emb_0`, `emb_1`, etc.

This will result in:
- More efficient storage (single column vs N columns)
- Cleaner schema that doesn't change based on embedding dimension
- Better compatibility with standard ML parquet formats
- Easier querying and processing of embeddings as vectors

The changes will be primarily in the `write_dataframe` method of `ParquetWriter` where the embedding vector is converted to DataFrame columns.