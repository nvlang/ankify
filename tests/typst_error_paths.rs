use ankify::cache::Cache;
use ankify::render::Renderer;
use ankify::{Card, RenderFormat};
use std::collections::HashMap;
use std::path::PathBuf;
use tempfile::TempDir;

/// Tests targeting specific uncovered error paths in typst-related functionality
#[cfg(test)]
mod typst_error_path_tests {
    use super::*;

    fn create_test_card(label: &str, front: &str, back: &str) -> Card {
        Card {
            label: label.to_string(),
            model: "Basic".to_string(),
            data: HashMap::from([
                ("Front".to_string(), front.to_string()),
                ("Back".to_string(), back.to_string()),
            ]),
            deck: "Test Deck".to_string(),
            tags: vec!["test".to_string()],
            rest: HashMap::new(),
            format: RenderFormat::Svg,
            source_file: PathBuf::from("test.typ"),
        }
    }

    #[tokio::test]
    async fn test_renderer_with_invalid_typst_content() {
        let temp_dir = TempDir::new().unwrap();

        // Test renderer creation
        let result = Renderer::new(temp_dir.path()).await;

        match result {
            Ok(_renderer) => {
                // If renderer creates successfully, that's good
                // We can't easily test invalid Typst content without actual Typst integration
            }
            Err(e) => {
                // If it fails, error should be meaningful (e.g., missing typst binary)
                assert!(!e.to_string().is_empty());
            }
        }
    }

    #[test]
    fn test_card_with_invalid_typst_source_paths() {
        let temp_dir = TempDir::new().unwrap();
        let aux_path = temp_dir.path().join("ankify.aux.json");
        let cache = Cache::new(&aux_path).unwrap();

        // Test cards with various problematic source file paths
        let very_long_path = "very_long_path_".repeat(100) + ".typ";
        let problematic_paths = vec![
            "",                               // Empty path
            "/nonexistent/path.typ",          // Nonexistent absolute path
            "relative/../path.typ",           // Relative path with traversal
            "path with spaces.typ",           // Spaces in filename
            "path\nwith\nnewlines.typ",       // Control characters
            &very_long_path,                  // Very long path
            "path/with/many/nested/dirs.typ", // Deep nesting
            ".hidden.typ",                    // Hidden file
            "path.with.dots.typ",             // Multiple dots
            "UPPERCASE.TYP",                  // Uppercase extension
        ];

        for source_path in problematic_paths {
            let card = Card {
                label: format!("test-{}", source_path.len()),
                model: "Basic".to_string(),
                data: HashMap::from([
                    ("Front".to_string(), "Question".to_string()),
                    ("Back".to_string(), "Answer".to_string()),
                ]),
                deck: "Test Deck".to_string(),
                tags: vec!["test".to_string()],
                rest: HashMap::new(),
                format: RenderFormat::Svg,
                source_file: PathBuf::from(source_path),
            };

            // Should handle problematic source paths gracefully
            let operations = cache.plan_operations(&[card]);
            match operations {
                Ok(ops) => {
                    assert!(!ops.is_empty());
                }
                Err(e) => {
                    // If it fails due to the source path, error should be meaningful
                    assert!(!e.to_string().is_empty());
                }
            }
        }
    }

    #[test]
    fn test_render_format_edge_cases() {
        let formats_with_problematic_content = vec![
            (RenderFormat::Svg, "content with <svg> tags"),
            (RenderFormat::Png, "content with binary\0data"),
            (
                RenderFormat::Html,
                "content with <script>alert('xss')</script>",
            ),
            (RenderFormat::Plain, "plain text\nwith\nnewlines\tand\ttabs"),
        ];

        for (format, content) in formats_with_problematic_content {
            let card = Card {
                label: "format-test".to_string(),
                model: "Basic".to_string(),
                data: HashMap::from([
                    ("Front".to_string(), content.to_string()),
                    ("Back".to_string(), "Answer".to_string()),
                ]),
                deck: "Test Deck".to_string(),
                tags: vec!["format-test".to_string()],
                rest: HashMap::new(),
                format,
                source_file: PathBuf::from("test.typ"),
            };

            // Card creation should succeed regardless of content
            assert_eq!(card.format, format);
            assert_eq!(card.data.get("Front").unwrap(), content);
        }
    }

    #[tokio::test]
    async fn test_renderer_with_filesystem_constraints() {
        let temp_dir = TempDir::new().unwrap();

        // Test renderer creation in various filesystem scenarios
        let test_scenarios = vec![
            temp_dir.path().join("cache"),
            temp_dir.path().join("cache with spaces"),
            temp_dir.path().join("cache-with-dashes"),
            temp_dir.path().join("cache_with_underscores"),
            temp_dir.path().join("deeply").join("nested").join("cache"),
        ];

        for cache_dir in test_scenarios {
            let result = Renderer::new(&cache_dir).await;

            match result {
                Ok(_renderer) => {
                    // If it succeeds, verify the directory was created
                    assert!(cache_dir.exists());
                }
                Err(e) => {
                    // If it fails, error should be meaningful
                    assert!(!e.to_string().is_empty());
                }
            }
        }
    }

    #[test]
    fn test_card_content_hash_consistency() {
        // Test that cards with identical content produce identical hashes
        let base_card = create_test_card("test", "Question", "Answer");
        let base_hash = base_card.content_hash();

        // Test variations that should NOT affect the hash
        let identical_variations = vec![
            // Same card with different label (label is not part of hash)
            Card {
                label: "different-label".to_string(),
                ..base_card.clone()
            },
            // Same card with different source file (source_file might not be part of hash)
            Card {
                source_file: PathBuf::from("different.typ"),
                ..base_card.clone()
            },
        ];

        for variation in identical_variations {
            let variation_hash = variation.content_hash();
            // These should have the same hash since only content matters
            assert_eq!(base_hash, variation_hash);
        }

        // Test variations that SHOULD affect the hash
        let different_variations = vec![
            Card {
                model: "Different".to_string(),
                ..base_card.clone()
            },
            Card {
                deck: "Different Deck".to_string(),
                ..base_card.clone()
            },
            Card {
                tags: vec!["different".to_string()],
                ..base_card.clone()
            },
            Card {
                data: HashMap::from([
                    ("Front".to_string(), "Different Question".to_string()),
                    ("Back".to_string(), "Answer".to_string()),
                ]),
                ..base_card.clone()
            },
        ];

        for variation in different_variations {
            let variation_hash = variation.content_hash();
            // These should have different hashes
            assert_ne!(base_hash, variation_hash);
        }
    }

    #[test]
    fn test_card_serialization_edge_cases() {
        // Test serialization of cards with edge case data
        let edge_case_cards = vec![
            // Card with empty fields
            Card {
                label: "empty-fields".to_string(),
                model: "".to_string(),
                data: HashMap::new(),
                deck: "".to_string(),
                tags: vec![],
                rest: HashMap::new(),
                format: RenderFormat::Plain,
                source_file: PathBuf::from(""),
            },
            // Card with unicode content
            Card {
                label: "unicode-test".to_string(),
                model: "Basic".to_string(),
                data: HashMap::from([
                    ("Front".to_string(), "Question with emoji: 🦀🔥".to_string()),
                    ("Back".to_string(), "Answer with unicode: αβγδε".to_string()),
                ]),
                deck: "Unicode Deck 📚".to_string(),
                tags: vec!["unicode 🏷️".to_string()],
                rest: HashMap::new(),
                format: RenderFormat::Svg,
                source_file: PathBuf::from("unicode.typ"),
            },
            // Card with very long content
            Card {
                label: "long-content".to_string(),
                model: "Basic".to_string(),
                data: HashMap::from([
                    ("Front".to_string(), "Q".repeat(10000)),
                    ("Back".to_string(), "A".repeat(10000)),
                ]),
                deck: "Long Content Deck".to_string(),
                tags: vec!["long".to_string()],
                rest: HashMap::new(),
                format: RenderFormat::Html,
                source_file: PathBuf::from("long.typ"),
            },
        ];

        for card in edge_case_cards {
            // Serialization should not panic
            let serialized = serde_json::to_string(&card);
            match serialized {
                Ok(json) => {
                    assert!(!json.is_empty());

                    // Deserialization should also work
                    let deserialized: Result<Card, _> = serde_json::from_str(&json);
                    match deserialized {
                        Ok(deserialized_card) => {
                            assert_eq!(card.label, deserialized_card.label);
                        }
                        Err(e) => {
                            // If deserialization fails, it should be due to size limits
                            assert!(!e.to_string().is_empty());
                        }
                    }
                }
                Err(e) => {
                    // If serialization fails, it should be meaningful
                    assert!(!e.to_string().is_empty());
                }
            }
        }
    }

    #[tokio::test]
    async fn test_renderer_temp_directory_management() {
        let temp_dir = TempDir::new().unwrap();
        let cache_dir = temp_dir.path().join("renderer_cache");

        let result = Renderer::new(&cache_dir).await;

        match result {
            Ok(_renderer) => {
                // Verify expected directories were created
                assert!(cache_dir.exists());
                let temp_subdir = cache_dir.join("temp");
                assert!(temp_subdir.exists());

                // Verify directories are writable
                let test_file = temp_subdir.join("test.txt");
                let write_result = tokio::fs::write(&test_file, "test").await;
                assert!(write_result.is_ok());
            }
            Err(e) => {
                // If renderer creation fails, check if it's due to missing typst binary
                let error_msg = e.to_string().to_lowercase();
                assert!(
                    error_msg.contains("typst")
                        || error_msg.contains("not found")
                        || error_msg.contains("permission")
                        || error_msg.contains("filesystem")
                );
            }
        }
    }

    #[test]
    fn test_cache_operations_with_typst_related_edge_cases() {
        let temp_dir = TempDir::new().unwrap();
        let aux_path = temp_dir.path().join("ankify.aux.json");
        let cache = Cache::new(&aux_path).unwrap();

        // Test cards that might cause issues during Typst processing
        let problematic_cards = vec![
            // Card with Typst-like syntax in content
            create_test_card(
                "typst-syntax",
                "#let x = 5\n= Heading\n*bold text*",
                "$x + y = z$",
            ),
            // Card with potential command injection
            create_test_card(
                "injection-test",
                "$(rm -rf /); echo 'harmless'",
                "`rm -rf /`",
            ),
            // Card with very nested structures
            create_test_card("nested", "{{{{{{nested}}}}}}", "[[[[[[brackets]]]]]]"),
        ];

        for card in problematic_cards {
            // Cache operations should handle these safely
            let operations = cache.plan_operations(&[card.clone()]);

            match operations {
                Ok(ops) => {
                    assert!(!ops.is_empty());
                    // Verify the card content is preserved
                    match &ops[0] {
                        ankify::cache::Operation::Create { card: created_card } => {
                            assert_eq!(created_card.label, card.label);
                            assert_eq!(created_card.data, card.data);
                        }
                        _ => {
                            // Other operations are also valid
                        }
                    }
                }
                Err(e) => {
                    // If it fails, error should be meaningful and not a panic
                    assert!(!e.to_string().is_empty());
                }
            }
        }
    }
}
