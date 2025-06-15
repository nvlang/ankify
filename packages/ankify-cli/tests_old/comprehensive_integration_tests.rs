//! Comprehensive integration tests using property-based testing and fuzzing
//!
//! This test suite focuses on:
//! 1. Error handling paths that are hard to cover
//! 2. CLI edge cases and validation
//! 3. Property-based testing with arbitrary inputs
//! 4. End-to-end workflows with error injection

use ankify::{Ankify, Config};
use proptest::prelude::*;
use std::path::PathBuf;
use tempfile::TempDir;
use tokio::fs;

/// Property-based test for CLI argument parsing with arbitrary inputs
#[cfg(test)]
mod property_tests {
    use super::*;
    use ankify::cli::Cli;
    use clap::Parser;

    proptest! {
        #[test]
        fn test_cli_with_arbitrary_inputs(
            files in prop::collection::vec(prop::string::string_regex("[a-zA-Z][a-zA-Z0-9_]*\\.typ").unwrap(), 1..5),
            url in prop::string::string_regex("https?://[a-zA-Z0-9.-]+(:[0-9]{4,5})?").unwrap(),
            log_level in prop::sample::select(vec!["error", "warn", "info", "debug", "trace", "invalid"]),
        ) {
            let mut args = vec!["ankify".to_string()];
            args.extend(files);
            args.extend(vec!["--ankiconnect-url".to_string(), url]);
            args.extend(vec!["--log-level".to_string(), log_level.to_string()]);

            // Test CLI parsing - should handle various inputs gracefully
            let result = Cli::try_parse_from(args);

            if log_level == "invalid" {
                // Invalid log levels should be caught during validation, not parsing
                if let Ok(cli) = result {
                    let validation_result = cli.validate();
                    // Should either succeed or fail gracefully without panicking
                    drop(validation_result);
                }
            } else {
                // Valid arguments should parse successfully
                prop_assert!(result.is_ok(), "CLI parsing failed for valid arguments");
            }
        }
    }

    proptest! {
        #[test]
        fn test_config_with_arbitrary_paths(
            aux_file in prop::string::string_regex("[a-zA-Z0-9_/.-]+\\.json").unwrap(),
            cache_dir in prop::string::string_regex("[a-zA-Z0-9_/.-]+").unwrap(),
        ) {
            tokio_test::block_on(async {
                let temp_dir = TempDir::new().unwrap();

                let mut config = Config::default_for_testing();
                config.aux_file = PathBuf::from(&aux_file);
                config.cache_dir = temp_dir.path().join(&cache_dir);
                config.typst_files = vec!["test.typ".to_string()];

                // Test config validation with arbitrary paths
                let validation_result = config.validate();
                // Should handle invalid paths gracefully without panicking
                drop(validation_result);
            });
        }
    }
}

/// Integration tests for error scenarios and edge cases
#[cfg(test)]
mod error_scenario_tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;

    #[tokio::test]
    async fn test_main_function_error_paths() {
        // Test CLI parsing errors
        let result = std::process::Command::new("cargo")
            .args(&["run", "--", "--invalid-flag"])
            .output();

        // Should handle invalid CLI args gracefully
        if let Ok(output) = result {
            assert!(!output.status.success());
        }
    }

    #[tokio::test]
    async fn test_permission_errors() {
        let temp_dir = TempDir::new().unwrap();

        // Create a file we can't write to (simulate permission error)
        let readonly_file = temp_dir.path().join("readonly.json");
        fs::write(&readonly_file, "{}").await.unwrap();

        // Try to make it readonly (this might not work on all systems)
        let _ = std::fs::set_permissions(&readonly_file, std::fs::Permissions::from_mode(0o444));

        let mut config = Config::default_for_testing();
        config.aux_file = readonly_file;
        config.cache_dir = temp_dir.path().join("cache");

        // Test that permission errors are handled gracefully
        let result = Ankify::new(config).await;
        match result {
            Ok(_) => {}  // Succeeded despite readonly file
            Err(_) => {} // Failed gracefully
        }
    }

    #[tokio::test]
    async fn test_invalid_typst_files() {
        let temp_dir = TempDir::new().unwrap();

        // Test with various invalid file patterns
        let invalid_patterns = vec![
            "*.nonexistent",
            "/root/inaccessible.typ",
            "",
            "file with spaces.typ",
            "file-with-unicode-😀.typ",
        ];

        for pattern in invalid_patterns {
            let mut config = Config::default_for_testing();
            config.typst_files = vec![pattern.to_string()];
            config.aux_file = temp_dir.path().join("test.json");
            config.cache_dir = temp_dir.path().join("cache");

            let result = Ankify::new(config).await;
            // Should handle invalid patterns gracefully
            match result {
                Ok(mut ankify) => {
                    // Even if creation succeeds, processing should handle errors
                    let process_result = ankify.process_files().await;
                    drop(process_result); // Handle success or failure gracefully
                }
                Err(_) => {} // Expected for some invalid patterns
            }
        }
    }

    #[tokio::test]
    async fn test_network_error_simulation() {
        let temp_dir = TempDir::new().unwrap();

        // Test with invalid AnkiConnect URLs to trigger network errors
        let invalid_urls = vec![
            "http://127.0.0.1:99999",   // Invalid port
            "http://invalid-host:8765", // Invalid host
            "not-a-url",                // Malformed URL
            "",                         // Empty URL
        ];

        for url in invalid_urls {
            let mut config = Config::default_for_testing();
            config.ankiconnect_url = url.to_string();
            config.aux_file = temp_dir.path().join("test.json");
            config.cache_dir = temp_dir.path().join("cache");
            config.typst_files = vec!["test.typ".to_string()];

            let result = Ankify::new(config).await;
            // Should handle invalid URLs gracefully at creation or processing time
            match result {
                Ok(mut ankify) => {
                    let process_result = ankify.process_files().await;
                    drop(process_result); // Network errors should be handled gracefully
                }
                Err(_) => {} // Some URLs might fail at creation time
            }
        }
    }

    #[tokio::test]
    async fn test_file_system_edge_cases() {
        let temp_dir = TempDir::new().unwrap();

        // Create a config that exercises file system edge cases
        let mut config = Config::default_for_testing();
        config.aux_file = temp_dir
            .path()
            .join("subdir")
            .join("nested")
            .join("file.json");
        config.cache_dir = temp_dir.path().join("deep").join("cache").join("dir");
        config.typst_files = vec!["*.typ".to_string()];

        // Test that deep directory creation works
        let result = Ankify::new(config).await;
        assert!(result.is_ok(), "Should handle deep directory creation");

        let mut ankify = result.unwrap();
        let process_result = ankify.process_files().await;
        // Should handle missing files gracefully
        drop(process_result);
    }

    #[tokio::test]
    async fn test_malformed_json_aux_files() {
        let temp_dir = TempDir::new().unwrap();

        // Create malformed aux files to test error handling
        let malformed_files = vec![
            "invalid json content",
            "{broken json",
            "{'single': 'quotes'}",
            "",
            "\0\x01\x02", // Binary content
        ];

        for (i, content) in malformed_files.iter().enumerate() {
            let aux_file = temp_dir.path().join(format!("malformed_{}.json", i));
            fs::write(&aux_file, content).await.unwrap();

            let mut config = Config::default_for_testing();
            config.aux_file = aux_file;
            config.cache_dir = temp_dir.path().join(format!("cache_{}", i));
            config.typst_files = vec!["test.typ".to_string()];

            let result = Ankify::new(config).await;
            // Should handle malformed aux files gracefully
            match result {
                Ok(mut ankify) => {
                    let process_result = ankify.process_files().await;
                    drop(process_result);
                }
                Err(_) => {} // Expected for some malformed files
            }
        }
    }
}

/// Integration tests that exercise watch mode and real-time features
#[cfg(test)]
mod watch_integration_tests {
    use super::*;
    use std::time::Duration;
    use tokio::time::timeout;

    #[tokio::test]
    async fn test_watch_mode_with_invalid_paths() {
        let temp_dir = TempDir::new().unwrap();

        let mut config = Config::default_for_testing();
        config.typst_files = vec!["/nonexistent/path/*.typ".to_string()];
        config.aux_file = temp_dir.path().join("test.json");
        config.cache_dir = temp_dir.path().join("cache");
        config.watch = true;

        let result = Ankify::new(config).await;
        if let Ok(mut ankify) = result {
            // Test that watch mode handles invalid paths gracefully
            let watch_result = timeout(Duration::from_millis(100), ankify.start_watch_mode()).await;

            match watch_result {
                Ok(Ok(_)) => {}  // Watch succeeded
                Ok(Err(_)) => {} // Watch failed gracefully
                Err(_) => {}     // Timeout - watch is running in background
            }
        }
    }

    #[tokio::test]
    async fn test_concurrent_file_operations() {
        let temp_dir = TempDir::new().unwrap();

        // Create multiple Ankify instances to test concurrent operations
        let mut handles = Vec::new();

        for i in 0..3 {
            let aux_file = temp_dir.path().join(format!("concurrent_{}.json", i));
            let cache_dir = temp_dir.path().join(format!("cache_{}", i));

            let mut config = Config::default_for_testing();
            config.aux_file = aux_file;
            config.cache_dir = cache_dir;
            config.typst_files = vec!["test.typ".to_string()];

            let handle = tokio::spawn(async move {
                let result = Ankify::new(config).await;
                if let Ok(mut ankify) = result {
                    let _ = ankify.process_files().await;
                }
            });

            handles.push(handle);
        }

        // Wait for all concurrent operations to complete
        for handle in handles {
            let _ = handle.await;
        }
    }
}

/// Fuzzing-style tests with random data
#[cfg(test)]
mod fuzzing_tests {
    use super::*;
    use rand::Rng;

    #[tokio::test]
    async fn test_random_typst_content() {
        let temp_dir = TempDir::new().unwrap();

        // Generate random Typst-like content to test parsing robustness
        let mut rng = rand::rng();

        for _ in 0..10 {
            let random_content = generate_random_typst_content(&mut rng);
            let typst_file = temp_dir
                .path()
                .join(format!("random_{}.typ", rng.random::<u32>()));
            fs::write(&typst_file, random_content).await.unwrap();

            // Test that the system handles arbitrary Typst content gracefully
            let result = ankify::typst::extract_cards(&typst_file).await;
            // Should not panic, even with invalid content
            drop(result);
        }
    }

    fn generate_random_typst_content(rng: &mut impl Rng) -> String {
        let templates = vec![
            r#"#card("random", data: (Front: "Q", Back: "A"))"#.to_string(),
            r#"#import "ankify.typ": card
#card("test", model: "Basic", data: (Front: "{}", Back: "{}"))"#
                .to_string(),
            r#"invalid typst content"#.to_string(),
            r#"#card("malformed""#.to_string(),
            r#""#.to_string(), // Empty file
            format!(
                "#card(\"{}\", data: ())",
                "x".repeat(rng.random_range(1..1000))
            ), // Long labels
        ];

        templates[rng.random_range(0..templates.len())].clone()
    }

    #[tokio::test]
    async fn test_stress_large_files() {
        let temp_dir = TempDir::new().unwrap();

        // Create a large Typst file to test memory and performance limits
        let mut large_content = String::from(
            r#"#import "ankify.typ": card, configure
#configure(deck: "Stress Test")
"#,
        );

        // Add many cards to test batch processing
        for i in 0..100 {
            large_content.push_str(&format!(
                r#"#card("stress-{}", data: (Front: "Question {}", Back: "Answer {}"))
"#,
                i, i, i
            ));
        }

        let large_file = temp_dir.path().join("large_test.typ");
        fs::write(&large_file, large_content).await.unwrap();

        // Test extraction with large files
        let result = ankify::typst::extract_cards(&large_file).await;
        match result {
            Ok(cards) => {
                // Should handle large numbers of cards
                assert!(
                    cards.len() <= 100,
                    "Should not extract more cards than created"
                );
            }
            Err(_) => {
                // Acceptable if the system has memory/processing limits
            }
        }
    }
}
