use ankify::cache::Cache;
use ankify::{Card, FieldFormat, RenderFormat};
use std::collections::HashMap;
use std::path::PathBuf;
use tempfile::TempDir;

/// Tests targeting specific uncovered error paths in cache functionality
#[cfg(test)]
mod cache_error_path_tests {
    use super::*;

    fn create_test_card(label: &str, long_content: bool) -> Card {
        let content = if long_content {
            "content".repeat(1000)
        } else {
            "Simple content".to_string()
        };

        Card {
            label: label.to_string(),
            model: "Basic".to_string(),
            data: HashMap::from([
                ("Front".to_string(), content.clone()),
                ("Back".to_string(), content),
            ]),
            deck: "Test Deck".to_string(),
            tags: vec!["test".to_string()],
            rest: HashMap::new(),
            format: RenderFormat::Single(FieldFormat::Svg),
            source_file: PathBuf::from("test.typ"),
        }
    }

    #[test]
    fn test_cache_with_empty_card_list() {
        let temp_dir = TempDir::new().unwrap();
        let aux_path = temp_dir.path().join("ankify.aux.json");
        let cache = Cache::new(&aux_path).unwrap();

        // Test planning operations with empty card list
        let operations = cache.plan_operations(&[]).unwrap();
        assert!(operations.is_empty());
    }

    #[test]
    fn test_cache_with_large_cards() {
        let temp_dir = TempDir::new().unwrap();
        let aux_path = temp_dir.path().join("ankify.aux.json");
        let cache = Cache::new(&aux_path).unwrap();

        // Test with cards containing large content
        let cards = vec![
            create_test_card("large-card-1", true),
            create_test_card("large-card-2", true),
        ];

        let operations = cache.plan_operations(&cards).unwrap();
        assert_eq!(operations.len(), 2);

        // All should be create operations for new cards
        for operation in operations {
            assert!(matches!(operation, ankify::cache::Operation::Create { .. }));
        }
    }

    #[test]
    fn test_cache_with_special_characters() {
        let temp_dir = TempDir::new().unwrap();
        let aux_path = temp_dir.path().join("ankify.aux.json");
        let cache = Cache::new(&aux_path).unwrap();

        // Test with cards containing special characters in labels
        let special_labels = vec![
            "card-with-dashes",
            "card_with_underscores",
            "card.with.dots",
            "card with spaces",
            "card@with@symbols",
            "card#with#hash",
            "card$with$dollar",
            "card%with%percent",
        ];

        let cards: Vec<_> = special_labels
            .iter()
            .map(|&label| create_test_card(label, false))
            .collect();

        let operations = cache.plan_operations(&cards).unwrap();
        assert_eq!(operations.len(), cards.len());
    }

    #[test]
    fn test_cache_with_unicode_content() {
        let temp_dir = TempDir::new().unwrap();
        let aux_path = temp_dir.path().join("ankify.aux.json");
        let cache = Cache::new(&aux_path).unwrap();

        // Test with cards containing unicode content
        let card = Card {
            label: "unicode-card".to_string(),
            model: "Basic".to_string(),
            data: HashMap::from([
                (
                    "Front".to_string(),
                    "Question: 你好世界 🦀 Rust".to_string(),
                ),
                ("Back".to_string(), "Answer: Здравствуй мир! 🚀".to_string()),
            ]),
            deck: "Unicode Test".to_string(),
            tags: vec!["unicode".to_string(), "international".to_string()],
            rest: HashMap::new(),
            format: RenderFormat::Single(FieldFormat::Svg),
            source_file: PathBuf::from("unicode.typ"),
        };

        let operations = cache.plan_operations(&[card]).unwrap();
        assert_eq!(operations.len(), 1);
    }

    #[tokio::test]
    async fn test_cache_operations_consistency() {
        let temp_dir = TempDir::new().unwrap();
        let aux_path = temp_dir.path().join("ankify.aux.json");

        let mut cache = Cache::new(&aux_path).unwrap();
        let card = create_test_card("consistency-test", false);

        // First operation should be create
        let operations1 = cache.plan_operations(&[card.clone()]).unwrap();
        assert_eq!(operations1.len(), 1);
        assert!(matches!(
            &operations1[0],
            ankify::cache::Operation::Create { .. }
        ));

        // Simulate the card being processed by recording its creation in the cache
        cache
            .record_creation(&card, "test-anki-id".to_string())
            .unwrap();
        cache.save().await.unwrap();

        // Same card should result in skip operation (no changes)
        let operations2 = cache.plan_operations(&[card]).unwrap();
        assert_eq!(operations2.len(), 1);
        assert!(
            matches!(&operations2[0], ankify::cache::Operation::Skip { .. }),
            "Expected Skip for consistency-test, got {:?}",
            operations2[0]
        );
    }

    #[test]
    fn test_cache_edge_case_file_paths() {
        // Test with various edge case file paths
        let temp_dir = TempDir::new().unwrap();

        let edge_case_paths = vec![
            "ankify.aux.json",
            "ankify-aux.json",
            "ankify_aux.json",
            "ANKIFY.AUX.JSON",
            "ankify.aux.JSON",
            "aux.json",
            ".aux.json",
            "ankify.json",
        ];

        for path_name in edge_case_paths {
            let aux_path = temp_dir.path().join(path_name);
            let result = Cache::new(&aux_path);

            // Should successfully create cache regardless of filename
            assert!(result.is_ok());

            let cache = result.unwrap();
            let card = create_test_card("path-test", false);
            let operations = cache.plan_operations(&[card]).unwrap();
            assert_eq!(operations.len(), 1);
        }
    }

    #[tokio::test]
    async fn test_cache_multiple_save_load_cycles() {
        let temp_dir = TempDir::new().unwrap();
        let aux_path = temp_dir.path().join("ankify.aux.json");

        let card = create_test_card("cycle-test", false);

        // Create cache, add card, save
        {
            let mut cache = Cache::new(&aux_path).unwrap();
            let operations = cache.plan_operations(&[card.clone()]).unwrap();
            assert_eq!(operations.len(), 1);
            assert!(matches!(
                &operations[0],
                ankify::cache::Operation::Create { .. }
            ));
            // Record the card creation before saving
            cache
                .record_creation(&card, "test-anki-id".to_string())
                .unwrap();
            cache.save().await.unwrap();
        }

        // Load cache again, should recognize existing card
        {
            let cache = Cache::new(&aux_path).unwrap();
            let operations = cache.plan_operations(&[card.clone()]).unwrap();
            assert_eq!(operations.len(), 1);
            assert!(
                matches!(&operations[0], ankify::cache::Operation::Skip { .. }),
                "Expected Skip after load for cycle-test, got {:?}",
                operations[0]
            );
        }

        // Modify card content, should detect update needed
        {
            let mut modified_card = card.clone();
            modified_card
                .data
                .insert("Front".to_string(), "Modified content".to_string());

            let cache = Cache::new(&aux_path).unwrap();
            let operations = cache.plan_operations(&[modified_card]).unwrap();
            assert_eq!(operations.len(), 1);
            // Should be update or create operation
            assert!(matches!(
                &operations[0],
                ankify::cache::Operation::Update { .. } | ankify::cache::Operation::Create { .. }
            ));
        }
    }

    #[test]
    fn test_cache_with_malformed_existing_file() {
        let temp_dir = TempDir::new().unwrap();
        let aux_path = temp_dir.path().join("ankify.aux.json");

        // Create a malformed JSON file
        std::fs::write(&aux_path, "{ invalid json content").unwrap();

        // Cache should handle malformed file gracefully
        let result = Cache::new(&aux_path);

        // Depending on implementation, might succeed with empty cache or fail gracefully
        match result {
            Ok(cache) => {
                let card = create_test_card("malformed-test", false);
                let operations = cache.plan_operations(&[card]).unwrap();
                assert_eq!(operations.len(), 1);
            }
            Err(e) => {
                // Error should be meaningful
                assert!(!e.to_string().is_empty());
            }
        }
    }
}
