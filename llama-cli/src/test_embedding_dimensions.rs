#[cfg(test)]
mod dimension_tests {
    use crate::parquet_writer::*;
    use llama_embedding::types::EmbeddingResult;
    use tempfile::NamedTempFile;

    #[test]
    fn test_embedding_dimension_128() {
        let temp_file = NamedTempFile::new().unwrap();
        let mut writer = ParquetWriter::new(temp_file.path(), 128, 10).unwrap();

        let embedding: Vec<f32> = (0..128).map(|i| i as f32 * 0.01).collect();
        let result = EmbeddingResult::new("test 128".to_string(), embedding, 5, 100);

        writer.add_result(result).unwrap();
        let total_records = writer.close().unwrap();
        assert_eq!(total_records, 1);
    }

    #[test]
    fn test_embedding_dimension_384() {
        let temp_file = NamedTempFile::new().unwrap();
        let mut writer = ParquetWriter::new(temp_file.path(), 384, 10).unwrap();

        let embedding: Vec<f32> = (0..384).map(|i| (i as f32).sin()).collect();
        let result = EmbeddingResult::new("test 384".to_string(), embedding, 12, 250);

        writer.add_result(result).unwrap();
        let total_records = writer.close().unwrap();
        assert_eq!(total_records, 1);
    }

    #[test]
    fn test_embedding_dimension_768() {
        let temp_file = NamedTempFile::new().unwrap();
        let mut writer = ParquetWriter::new(temp_file.path(), 768, 5).unwrap();

        let embedding: Vec<f32> = (0..768).map(|i| (i as f32 / 768.0).tanh()).collect();
        let results = vec![
            EmbeddingResult::new("test 768 first".to_string(), embedding.clone(), 15, 400),
            EmbeddingResult::new("test 768 second".to_string(), embedding, 18, 450),
        ];

        writer.write_batch(results).unwrap();
        let total_records = writer.close().unwrap();
        assert_eq!(total_records, 2);
    }

    #[test]
    fn test_embedding_dimension_1024() {
        let temp_file = NamedTempFile::new().unwrap();
        let mut writer = ParquetWriter::new(temp_file.path(), 1024, 3).unwrap();

        let embedding: Vec<f32> = (0..1024)
            .map(|i| if i % 2 == 0 { 1.0 } else { -1.0 })
            .collect();
        let result = EmbeddingResult::new("test 1024".to_string(), embedding, 20, 800);

        writer.add_result(result).unwrap();
        let total_records = writer.close().unwrap();
        assert_eq!(total_records, 1);
    }

    #[test]
    fn test_mixed_batch_sizes() {
        let temp_file = NamedTempFile::new().unwrap();
        let mut writer = ParquetWriter::new(temp_file.path(), 256, 2).unwrap();

        // Test with different batch sizes but same embedding dimension
        for i in 0..5 {
            let embedding: Vec<f32> = (0..256).map(|j| (i as f32 + j as f32) / 256.0).collect();
            let result = EmbeddingResult::new(
                format!("test mixed batch {}", i),
                embedding,
                10 + i,
                100 * (i + 1) as u64,
            );
            writer.add_result(result).unwrap();
        }

        let total_records = writer.close().unwrap();
        assert_eq!(total_records, 5);
    }
}
