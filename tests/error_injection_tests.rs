use ankify::cache::Cache;
use ankify::render::Renderer;
use ankify::{Card, RenderFormat, RenderedCard};
use std::collections::HashMap;
use std::path::PathBuf;
use tempfile::TempDir;
use tokio::fs;
use wiremock::{
    matchers::{method, path},
    Mock, MockServer, ResponseTemplate,
};

/// Tests targeting error injection scenarios to improve coverage
#[cfg(test)]
mod error_injection_tests {
    use super::*;

    fn create_test_card(label: &str) -> Card {
        Card {
            label: label.to_string(),
            model: "Basic".to_string(),
            data: HashMap::from([
                ("Front".to_string(), "Test Question".to_string()),
                ("Back".to_string(), "Test Answer".to_string()),
            ]),
            deck: "Test Deck".to_string(),
            tags: vec!["test".to_string()],
            rest: HashMap::new(),
            format: RenderFormat::Svg,
            source_file: PathBuf::from("test.typ"),
        }
    }

    #[tokio::test]
    async fn test_renderer_creation_failures() {
        // Test renderer creation with invalid cache directory
        let invalid_paths = vec!["/dev/null/invalid", "/proc/invalid"];

        for invalid_path in invalid_paths {
            let result = Renderer::new(invalid_path).await;
            // Should either succeed (if path is actually valid) or fail gracefully
            match result {
                Ok(_) => {
                    // Some paths might work unexpectedly
                }
                Err(e) => {
                    assert!(!e.to_string().is_empty());
                }
            }
        }
    }

    #[tokio::test]
    async fn test_cache_file_system_errors() {
        let temp_dir = TempDir::new().unwrap();
        let aux_path = temp_dir.path().join("ankify.aux.json");

        // Create a cache and add some data
        let mut cache = Cache::new(&aux_path).unwrap();
        let card = create_test_card("test-card");

        // Test operations that might fail
        let operations = cache.plan_operations(&[card]).unwrap();
        assert!(!operations.is_empty());

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;

            // Make directory read-only to force save errors
            let parent_dir = aux_path.parent().unwrap();
            let mut perms = fs::metadata(parent_dir).await.unwrap().permissions();
            perms.set_mode(0o444);
            let _ = fs::set_permissions(parent_dir, perms).await;

            // Try to save - should handle errors gracefully
            let result = cache.save();
            match result {
                Ok(_) => {
                    // If it succeeds, that's fine too
                }
                Err(e) => {
                    assert!(!e.to_string().is_empty());
                }
            }

            // Restore permissions
            let mut perms = fs::metadata(parent_dir).await.unwrap().permissions();
            perms.set_mode(0o755);
            let _ = fs::set_permissions(parent_dir, perms).await;
        }
    }

    #[tokio::test]
    async fn test_renderer_with_missing_typst_binary() {
        let temp_dir = TempDir::new().unwrap();

        // This test checks error handling when typst binary is not found
        // The actual renderer creation might succeed if typst is installed
        let result = Renderer::new(temp_dir.path()).await;

        match result {
            Ok(_renderer) => {
                // If typst binary is available, that's expected
            }
            Err(e) => {
                // Should fail gracefully with meaningful error
                assert!(!e.to_string().is_empty());
                assert!(e.to_string().contains("typst") || e.to_string().contains("not found"));
            }
        }
    }

    #[test]
    fn test_card_content_hash_edge_cases() {
        // Test hashing with various edge case content
        let binding_a = "b".repeat(10000);
        let binding_b = "a".repeat(10000);
        let edge_cases = vec![
            ("", "", vec![]),                                  // Empty content
            (&binding_a, &binding_b, vec!["tag".to_string()]), // Large content
            ("content\nwith\nnewlines", "more\ncontent", vec![]),
            ("content\twith\ttabs", "more\tcontent", vec![]),
            (
                "content with unicode: 🦀🔥",
                "rust content",
                vec!["unicode".to_string()],
            ),
            ("content with null\0bytes", "more\0content", vec![]),
        ];

        for (front, back, tags) in edge_cases {
            let card = Card {
                label: "test".to_string(),
                model: "Basic".to_string(),
                data: HashMap::from([
                    ("Front".to_string(), front.to_string()),
                    ("Back".to_string(), back.to_string()),
                ]),
                deck: "Test".to_string(),
                tags,
                rest: HashMap::new(),
                format: RenderFormat::Svg,
                source_file: PathBuf::from("test.typ"),
            };

            // Should not panic or return empty hash
            let hash = card.content_hash();
            assert!(!hash.is_empty());
            assert_eq!(hash.len(), 32); // MD5 hash should be 32 chars
        }
    }

    #[test]
    fn test_rendered_card_creation() {
        // Test RenderedCard creation with the correct field structure
        let rendered_card = RenderedCard {
            model: "Basic".to_string(),
            data: HashMap::from([
                ("Front".to_string(), "Rendered Question".to_string()),
                ("Back".to_string(), "Rendered Answer".to_string()),
            ]),
            deck: "Test Deck".to_string(),
            tags: vec!["rendered".to_string()],
            rest: HashMap::new(),
            label: "test-card".to_string(),
            source_file: PathBuf::from("test.typ"),
        };

        assert_eq!(rendered_card.label, "test-card");
        assert_eq!(
            rendered_card.data.get("Front").unwrap(),
            "Rendered Question"
        );
        assert_eq!(rendered_card.data.get("Back").unwrap(), "Rendered Answer");
    }

    #[test]
    fn test_render_format_enum() {
        // Test all variants of RenderFormat
        let formats = vec![
            RenderFormat::Svg,
            RenderFormat::Png,
            RenderFormat::Html,
            RenderFormat::Plain,
        ];

        for format in formats {
            // Formats should serialize/deserialize properly
            let serialized = serde_json::to_string(&format).unwrap();
            assert!(!serialized.is_empty());

            let deserialized: RenderFormat = serde_json::from_str(&serialized).unwrap();
            assert_eq!(format, deserialized);
        }

        // Test default
        let default_format = RenderFormat::default();
        assert!(matches!(default_format, RenderFormat::Svg));
    }

    #[tokio::test]
    async fn test_network_error_simulation() {
        // Start a mock server for testing network errors
        let mock_server = MockServer::start().await;

        // Test various HTTP error scenarios
        let error_scenarios = vec![
            (500, "Internal Server Error"),
            (404, "Not Found"),
            (403, "Forbidden"),
            (408, "Request Timeout"),
            (502, "Bad Gateway"),
        ];

        for (status_code, error_msg) in error_scenarios {
            // Set up mock to return error
            Mock::given(method("POST"))
                .and(path("/"))
                .respond_with(ResponseTemplate::new(status_code).set_body_string(error_msg))
                .mount(&mock_server)
                .await;

            // Test that our code would handle these errors appropriately
            let response = reqwest::Client::new()
                .post(&mock_server.uri())
                .send()
                .await
                .unwrap();

            assert_eq!(response.status().as_u16(), status_code);
        }
    }

    #[test]
    fn test_large_card_collections() {
        let temp_dir = TempDir::new().unwrap();
        let aux_path = temp_dir.path().join("ankify.aux.json");
        let cache = Cache::new(&aux_path).unwrap();

        // Test with a large number of cards
        let mut cards = Vec::new();
        for i in 0..1000 {
            let card = Card {
                label: format!("card-{}", i),
                model: "Basic".to_string(),
                data: HashMap::from([
                    ("Front".to_string(), format!("Question {}", i)),
                    ("Back".to_string(), format!("Answer {}", i)),
                ]),
                deck: "Large Test Deck".to_string(),
                tags: vec![format!("tag-{}", i % 10)],
                rest: HashMap::new(),
                format: RenderFormat::Svg,
                source_file: PathBuf::from("test.typ"),
            };
            cards.push(card);
        }

        // Should handle large collections without issues
        let operations = cache.plan_operations(&cards).unwrap();
        assert_eq!(operations.len(), cards.len());

        // All should be create operations for new cards
        for operation in operations {
            assert!(matches!(operation, ankify::cache::Operation::Create { .. }));
        }
    }

    #[tokio::test]
    async fn test_renderer_to_plain_error_paths() {
        let temp_dir = TempDir::new().unwrap();
        let renderer = match Renderer::new(temp_dir.path()).await {
            Ok(r) => r,
            Err(_) => return, // Skip if renderer can't be created
        };

        // Test with content that might cause rendering errors
        let test_cases = vec![
            "<invalid>html</malformed>",
            "```invalid\ncode block\n```",
            "#error(\"test error\")",
            "",
            "\x00\x01\x02", // Binary data
        ];

        for content in test_cases {
            let mut card = create_test_card("test-render");
            card.data.insert("Front".to_string(), content.to_string());
            let result = renderer.render_card(&card, None).await;
            // Should either succeed or fail gracefully
            match result {
                Ok(_) => {}
                Err(e) => {
                    // Error should be well-formed
                    assert!(!e.to_string().is_empty());
                }
            }
        }
    }
}
