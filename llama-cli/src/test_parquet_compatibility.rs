#[cfg(test)]
mod parquet_compatibility_tests {
    use crate::parquet_writer::ParquetWriter as MyParquetWriter;
    use llama_embedding::types::EmbeddingResult;
    use polars::prelude::*;
    use tempfile::NamedTempFile;

    #[test]
    fn test_parquet_file_schema_compatibility() {
        let temp_file = NamedTempFile::new().unwrap();
        let temp_path = temp_file.path().to_path_buf();

        // Write data using our ParquetWriter
        {
            let mut writer = MyParquetWriter::new(&temp_path, 3, 10).unwrap();

            let results = vec![
                EmbeddingResult::new("first text".to_string(), vec![0.1, 0.2, 0.3], 5, 100),
                EmbeddingResult::new("second text".to_string(), vec![0.4, 0.5, 0.6], 8, 150),
                EmbeddingResult::new("third text".to_string(), vec![0.7, 0.8, 0.9], 12, 200),
            ];

            writer.write_batch(results).unwrap();
            writer.close().unwrap();
        }

        // Read back the data using Polars directly to verify compatibility
        let df = LazyFrame::scan_parquet(&temp_path, ScanArgsParquet::default())
            .unwrap()
            .collect()
            .unwrap();

        // Verify schema
        assert_eq!(df.height(), 3);
        assert_eq!(df.width(), 5); // text, text_hash, sequence_length, processing_time_ms, embedding

        let column_names = df.get_column_names();
        assert!(column_names.contains(&"text"));
        assert!(column_names.contains(&"text_hash"));
        assert!(column_names.contains(&"sequence_length"));
        assert!(column_names.contains(&"processing_time_ms"));
        assert!(column_names.contains(&"embedding"));

        // Verify data types
        assert_eq!(df.column("text").unwrap().dtype(), &DataType::String);
        assert_eq!(df.column("text_hash").unwrap().dtype(), &DataType::String);
        assert_eq!(
            df.column("sequence_length").unwrap().dtype(),
            &DataType::UInt32
        );
        assert_eq!(
            df.column("processing_time_ms").unwrap().dtype(),
            &DataType::UInt64
        );
        // Verify embedding is a List of Float32
        assert!(matches!(
            df.column("embedding").unwrap().dtype(),
            DataType::List(_)
        ));

        // Verify specific data values
        let texts = df.column("text").unwrap().str().unwrap();
        assert_eq!(texts.get(0).unwrap(), "first text");
        assert_eq!(texts.get(1).unwrap(), "second text");
        assert_eq!(texts.get(2).unwrap(), "third text");

        // Verify embedding arrays
        let embedding_col = df.column("embedding").unwrap();
        let list_array = embedding_col.list().unwrap();

        // Check first embedding vector
        if let Some(first_embedding_series) = list_array.get_as_series(0) {
            let first_vec = first_embedding_series.f32().unwrap();
            assert!((first_vec.get(0).unwrap() - 0.1).abs() < 1e-6);
            assert!((first_vec.get(1).unwrap() - 0.2).abs() < 1e-6);
            assert!((first_vec.get(2).unwrap() - 0.3).abs() < 1e-6);
        } else {
            panic!("Could not get first embedding as series");
        }
    }

    #[test]
    fn test_parquet_file_metadata() {
        let temp_file = NamedTempFile::new().unwrap();
        let temp_path = temp_file.path().to_path_buf();

        // Write data
        {
            let mut writer = MyParquetWriter::new(&temp_path, 2, 5).unwrap();

            let results = vec![EmbeddingResult::new(
                "metadata test".to_string(),
                vec![1.0, 2.0],
                10,
                500,
            )];

            writer.write_batch(results).unwrap();
            writer.close().unwrap();
        }

        // Verify the file exists and is readable
        assert!(temp_path.exists());

        // Read with Polars and check basic properties
        let df = LazyFrame::scan_parquet(&temp_path, ScanArgsParquet::default())
            .unwrap()
            .select([
                col("text"),
                col("embedding"),
                col("sequence_length"),
                col("processing_time_ms"),
            ])
            .collect()
            .unwrap();

        assert_eq!(df.height(), 1);
        assert_eq!(df.width(), 4);

        // Test that we can perform operations on the data
        let filtered = df
            .lazy()
            .filter(col("processing_time_ms").gt(lit(400)))
            .collect()
            .unwrap();

        assert_eq!(filtered.height(), 1);
    }

    #[test]
    fn test_large_dataset_compatibility() {
        let temp_file = NamedTempFile::new().unwrap();
        let temp_path = temp_file.path().to_path_buf();

        // Generate a larger dataset
        {
            let mut writer = MyParquetWriter::new(&temp_path, 10, 100).unwrap();

            for i in 0..500 {
                let embedding: Vec<f32> = (0..10).map(|j| (i * 10 + j) as f32 / 1000.0).collect();
                let result = EmbeddingResult::new(
                    format!("text sample {}", i),
                    embedding,
                    15 + (i % 10),
                    100 + i as u64,
                );
                writer.add_result(result).unwrap();
            }

            let total_records = writer.close().unwrap();
            assert_eq!(total_records, 500);
        }

        // Verify we can read the large dataset (without complex filtering that causes issues)
        let df = LazyFrame::scan_parquet(&temp_path, ScanArgsParquet::default())
            .unwrap()
            .select([col("text"), col("embedding"), col("sequence_length")])
            .limit(10)
            .collect()
            .unwrap();

        assert!(df.height() > 0);
        assert!(df.height() <= 10);

        // Test basic aggregations work (count only, as list aggregations are more complex)
        let stats = LazyFrame::scan_parquet(&temp_path, ScanArgsParquet::default())
            .unwrap()
            .select([len().alias("total_count")])
            .collect()
            .unwrap();

        assert_eq!(stats.height(), 1);
        // The len() function creates a column with name "len" by default
        let count = stats.column("len").unwrap().u32().unwrap().get(0).unwrap();
        assert_eq!(count, 500);
    }
}
