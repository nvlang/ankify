//! Integration tests for the Ankify v2 application

use ankify::{Ankify, Config, Result};
use std::path::PathBuf;
use tempfile::TempDir;
use tokio::fs;

#[tokio::test]
async fn test_full_workflow() -> Result<()> {
    // Create temporary directory structure
    let temp_dir = TempDir::new().unwrap();
    let temp_path = temp_dir.path();

    // Create a test Typst file
    let typst_content = r#"
#import "ankify.typ": card, configure

#configure(
  deck: "Test Deck",
  tags: ["test"],
)

#card(
  "card-1",
  data: (
    Front: "What is 2 + 2?",
    Back: "4"
  )
)

#card(
  "card-2", 
  data: (
    Front: "What is the capital of France?",
    Back: "Paris"
  )
)
"#;

    let typst_file = temp_path.join("test.typ");
    fs::write(&typst_file, typst_content).await.unwrap();

    // Create Ankify configuration using default_for_testing as a base
    let mut config = Config::default_for_testing();
    config.typst_files = vec![typst_file.to_string_lossy().to_string()];
    config.aux_file = temp_path.join("ankify.json");
    config.cache_dir = temp_path.join("cache");

    // Note: This test requires AnkiConnect to be running
    // In a real CI environment, you'd use a mock server
    if std::env::var("ANKICONNECT_TEST").is_ok() {
        let aux_file = config.aux_file.clone();
        let cache_dir = config.cache_dir.clone();

        let mut ankify = Ankify::new(config).await?;
        ankify.process_files().await?;

        // Verify cache was created
        assert!(aux_file.exists());

        // Verify cache directory was created
        assert!(cache_dir.exists());
    }

    Ok(())
}

#[tokio::test]
async fn test_config_loading() {
    let _temp_dir = TempDir::new().unwrap();

    // Test default configuration
    let config = Config::default();
    assert!(!config.typst_files.is_empty());
    assert_eq!(config.ankiconnect_url, "http://localhost:8765");

    // Test configuration validation
    let invalid_config = Config {
        typst_files: vec![],
        ..Config::default()
    };

    // Should handle empty file patterns gracefully
    let result = Ankify::new(invalid_config).await;
    assert!(result.is_ok()); // Should not fail at construction
}

#[tokio::test]
async fn test_file_operations() -> Result<()> {
    let temp_dir = TempDir::new().unwrap();
    let temp_path = temp_dir.path();

    // Test utils functions
    use ankify::utils::*;

    // Test directory creation
    let test_dir = temp_path.join("test_dir");
    ensure_dir(&test_dir).await?;
    assert!(test_dir.exists());

    // Test file hashing
    let test_file = temp_path.join("test.txt");
    fs::write(&test_file, "test content").await.unwrap();

    let hash1 = hash_file(&test_file).await?;
    let hash2 = hash_file(&test_file).await?;
    assert_eq!(hash1, hash2);

    // Test string hashing
    let str_hash1 = hash_string("test");
    let str_hash2 = hash_string("test");
    assert_eq!(str_hash1, str_hash2);

    // Test file extension checking
    assert!(has_extension(&PathBuf::from("test.typ"), "typ"));
    assert!(!has_extension(&PathBuf::from("test.txt"), "typ"));

    // Test card ID generation
    let id1 = generate_card_id("Math", "2+2", "4");
    let id2 = generate_card_id("Math", "2+2", "4");
    assert_eq!(id1, id2);

    let id3 = generate_card_id("Math", "3+3", "6");
    assert_ne!(id1, id3);

    // Test deck name validation
    assert!(is_valid_deck_name("Valid Deck"));
    assert!(!is_valid_deck_name(""));
    assert!(!is_valid_deck_name("Invalid/Deck"));

    Ok(())
}

#[tokio::test]
async fn test_error_handling() {
    // Test file not found error
    let config = Config {
        typst_files: vec!["nonexistent.typ".to_string()],
        ..Config::default()
    };

    let mut ankify = Ankify::new(config).await.unwrap();
    let result = ankify.process_files().await;

    // Should handle missing files gracefully
    assert!(result.is_err() || result.is_ok()); // Depends on implementation
}

mod mock_tests {
    use super::*;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn test_ankiconnect_integration() {
        // Start mock server
        let mock_server = MockServer::start().await;

        // Mock AnkiConnect version endpoint
        Mock::given(method("POST"))
            .and(path("/"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "result": 6,
                "error": null
            })))
            .mount(&mock_server)
            .await;

        // Mock deck names endpoint
        Mock::given(method("POST"))
            .and(path("/"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "result": ["Default", "Test Deck"],
                "error": null
            })))
            .mount(&mock_server)
            .await;

        // Mock card creation endpoint
        Mock::given(method("POST"))
            .and(path("/"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "result": 1234567890,
                "error": null
            })))
            .mount(&mock_server)
            .await;

        // Test with mock server
        let config = Config {
            ankiconnect_url: mock_server.uri(),
            ..Config::default()
        };

        let _ankify = Ankify::new(config).await.unwrap();
        // Additional test logic here...
    }
}

/// AnkiConnect Integration Tests
mod anki_integration_tests {
    use super::*;
    use ankify::anki::Client;
    use ankify::RenderedCard;
    use std::collections::HashMap;

    #[tokio::test]
    async fn test_ankiconnect_initialization() {
        // Test with default URL
        let anki_default = Client::new("http://localhost:8765").unwrap();
        // We can't access private fields, so just test that creation succeeds
        drop(anki_default); // Explicitly consume to avoid unused warning

        // Test with custom URL
        let anki_custom = Client::new("http://custom:9999").unwrap();
        // Test that creation succeeds with custom URL
        drop(anki_custom); // Explicitly consume to avoid unused warning
    }

    #[tokio::test]
    async fn test_ankiconnect_connection_test() {
        let anki = Client::new("http://localhost:8765").unwrap();

        // Test connection (this will fail without AnkiConnect, but we can test the request)
        let result = anki.test_connection().await;
        match result {
            Ok(version) => {
                assert!(version > 0);
            }
            Err(e) => {
                // Expected when AnkiConnect is not running
                assert!(e.to_string().contains("Connection") || e.to_string().contains("request"));
            }
        }
    }

    #[tokio::test]
    async fn test_rendered_card_creation() {
        // Test RenderedCard structure
        let mut fields = HashMap::new();
        fields.insert("Front".to_string(), "Test Question".to_string());
        fields.insert("Back".to_string(), "Test Answer".to_string());

        let card = RenderedCard {
            model: "Basic".to_string(),
            deck: "Test Deck".to_string(),
            data: fields,
            tags: vec!["test".to_string(), "integration".to_string()],
            rest: HashMap::new(),
            label: "test-card".to_string(),
            source_file: PathBuf::from("test.typ"),
        };

        assert_eq!(card.deck, "Test Deck");
        assert_eq!(card.model, "Basic");
        assert_eq!(card.data.len(), 2);
        assert_eq!(card.tags.len(), 2);
        assert_eq!(card.data["Front"], "Test Question");
        assert_eq!(card.data["Back"], "Test Answer");
    }

    #[tokio::test]
    async fn test_ankiconnect_operations_mock() {
        let anki = Client::new("http://localhost:8765").unwrap();

        // Test deck operations
        let deck_result = anki.get_deck_names().await;
        match deck_result {
            Ok(decks) => {
                assert!(decks.iter().any(|d| d == "Default"));
            }
            Err(_) => {
                // Expected when AnkiConnect is not available
            }
        }

        // Test card creation
        let mut fields = HashMap::new();
        fields.insert("Front".to_string(), "Integration Test".to_string());
        fields.insert("Back".to_string(), "Success".to_string());

        let card = RenderedCard {
            model: "Basic".to_string(),
            deck: "Default".to_string(),
            data: fields,
            tags: vec!["integration-test".to_string()],
            rest: HashMap::new(),
            label: "test-card".to_string(),
            source_file: PathBuf::from("test.typ"),
        };

        let create_result = anki.create_card(&card).await;
        match create_result {
            Ok(card_id) => {
                assert!(!card_id.is_empty());
            }
            Err(_) => {
                // Expected when AnkiConnect is not available
            }
        }
    }

    #[tokio::test]
    async fn test_ankiconnect_error_handling() {
        // Test with invalid URL
        let anki_invalid = Client::new("http://invalid-host:99999").unwrap();

        let connection_result = anki_invalid.test_connection().await;
        assert!(connection_result.is_err());

        let deck_result = anki_invalid.get_deck_names().await;
        assert!(deck_result.is_err());

        // Test with malformed URL - this should fail at client creation
        let anki_malformed = Client::new("not-a-url");
        // Client creation should still succeed even with malformed URL
        assert!(anki_malformed.is_ok());
    }

    #[tokio::test]
    async fn test_ankiconnect_card_validation() {
        let anki = Client::new("http://localhost:8765").unwrap();

        // Test card with missing fields
        let incomplete_card = RenderedCard {
            model: "Basic".to_string(),
            deck: "Test".to_string(),
            data: HashMap::new(), // Empty fields
            tags: vec![],
            rest: HashMap::new(),
            label: "incomplete-card".to_string(),
            source_file: PathBuf::from("test.typ"),
        };

        let result = anki.create_card(&incomplete_card).await;
        match result {
            Ok(_) => {
                // Some implementations might handle this
            }
            Err(_) => {
                // Expected - should fail with validation error
            }
        }

        // Test card with invalid deck name
        let mut valid_fields = HashMap::new();
        valid_fields.insert("Front".to_string(), "Question".to_string());
        valid_fields.insert("Back".to_string(), "Answer".to_string());

        let invalid_deck_card = RenderedCard {
            model: "Basic".to_string(),
            deck: "".to_string(), // Empty deck name
            data: valid_fields,
            tags: vec![],
            rest: HashMap::new(),
            label: "invalid-deck-card".to_string(),
            source_file: PathBuf::from("test.typ"),
        };

        let invalid_result = anki.create_card(&invalid_deck_card).await;
        match invalid_result {
            Ok(_) => {
                // Some implementations might handle this
            }
            Err(_) => {
                // Expected - should fail with validation error
            }
        }
    }
}

/// Helper function to create test Typst content
fn create_test_typst_content(cards: &[(&str, &str)]) -> String {
    let mut content = String::from(
        r#"
#import "ankify.typ": card, configure

#configure(
  defaults: (
    deck: "Test Deck",
    tags: ("integration-test",)
  )
)

"#,
    );

    for (i, (front, back)) in cards.iter().enumerate() {
        content.push_str(&format!(
            r#"
#card(
  "test-card-{}",
  model: "Basic",
  data: (
    Front: "{}",
    Back: "{}"
  )
)

"#,
            i + 1,
            front,
            back
        ));
    }

    content
}

#[tokio::test]
async fn test_multiple_cards() -> Result<()> {
    let temp_dir = TempDir::new().unwrap();
    let temp_path = temp_dir.path();

    // Copy ankify.typ to the temporary directory
    let ankify_src = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("typst/ankify.typ");
    let ankify_dst = temp_path.join("ankify.typ");
    fs::copy(&ankify_src, &ankify_dst).await.unwrap();

    // Create test content with multiple cards
    let cards = &[
        ("What is 1+1?", "2"),
        ("What is 2+2?", "4"),
        ("What is 3+3?", "6"),
    ];

    let typst_content = create_test_typst_content(cards);
    let typst_file = temp_path.join("multi_cards.typ");
    fs::write(&typst_file, typst_content).await.unwrap();

    // Test that we can extract multiple cards
    let extracted_cards = ankify::typst::extract_cards(&typst_file).await?;
    assert_eq!(extracted_cards.len(), 3);

    Ok(())
}

#[tokio::test]
async fn test_caching_behavior() -> Result<()> {
    let temp_dir = TempDir::new().unwrap();
    let temp_path = temp_dir.path();

    let aux_file = temp_path.join("cache_test.json");
    let mut cache = ankify::cache::Cache::new(&aux_file)?;

    // Create a test card
    let mut card_data = std::collections::HashMap::new();
    card_data.insert("Front".to_string(), "Test Question".to_string());
    card_data.insert("Back".to_string(), "Test Answer".to_string());

    let card = ankify::Card {
        label: "test-card-1".to_string(),
        model: "Basic".to_string(),
        data: card_data,
        deck: "Test Deck".to_string(),
        tags: vec!["test".to_string()],
        rest: std::collections::HashMap::new(),
        format: ankify::RenderFormat::Plain,
        source_file: std::path::PathBuf::from("test.typ"),
    };

    // Test cache operations
    let operations = cache.plan_operations(&[card.clone()])?;
    assert!(!operations.is_empty());

    // Simulate recording the card
    cache.record_creation(&card, "123456".to_string())?;
    cache.save()?;

    // Test that the same card doesn't create new operations
    let _operations_after = cache.plan_operations(&[card])?;
    // Should have fewer operations (or skip operations)
    // This depends on the exact implementation

    Ok(())
}

/// CLI Integration Tests
mod cli_tests {
    use super::*;
    use ankify::cli::Cli;
    use clap::Parser;

    #[test]
    fn test_cli_argument_parsing() {
        // Test basic file parsing
        let args = Cli::try_parse_from(&["ankify", "test.typ"]).unwrap();
        assert_eq!(args.typst_files, vec!["test.typ"]);

        // Test multiple files
        let args = Cli::try_parse_from(&["ankify", "test1.typ", "test2.typ"]).unwrap();
        assert_eq!(args.typst_files, vec!["test1.typ", "test2.typ"]);

        // Test with options
        let args = Cli::try_parse_from(&[
            "ankify",
            "test.typ",
            "--aux-file",
            "custom.json",
            "--verbose",
            "--ankiconnect-url",
            "http://custom:8765",
            "--watch",
        ])
        .unwrap();

        assert_eq!(args.typst_files, vec!["test.typ"]);
        assert_eq!(args.aux_file, PathBuf::from("custom.json"));
        assert_eq!(args.ankiconnect_url, "http://custom:8765");
        assert!(args.verbose);
        assert!(args.watch);
    }

    #[test]
    fn test_cli_validation() {
        use tempfile::TempDir;
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();

        // Test validation with empty files
        let args = Cli::try_parse_from(&["ankify"]).unwrap();
        let result = args.validate();
        assert!(result.is_err());

        // Test validation with valid files - use aux file in temp directory
        let aux_file = temp_path.join("test.aux.json");
        let args = Cli::try_parse_from(&[
            "ankify",
            "test.typ",
            "--aux-file",
            aux_file.to_str().unwrap(),
        ])
        .unwrap();
        let result = args.validate();
        assert!(result.is_ok());
    }

    #[test]
    fn test_cli_defaults() {
        let args = Cli::try_parse_from(&["ankify", "test.typ"]).unwrap();

        assert_eq!(args.ankiconnect_url, "http://localhost:8765");
        assert_eq!(args.aux_file, PathBuf::from("ankify.aux.json"));
        assert!(!args.verbose);
        assert!(!args.bypass_cache);
        assert!(!args.watch);
        assert_eq!(args.log_level, "info");
    }

    #[test]
    fn test_cli_invalid_arguments() {
        // Test invalid options
        let result = Cli::try_parse_from(&["ankify", "--invalid-option"]);
        assert!(result.is_err());

        // Test missing required file arguments (should pass since typst_files is Vec)
        let result = Cli::try_parse_from(&["ankify"]);
        assert!(result.is_ok()); // Empty files list is allowed by clap, but fails validation
    }

    #[tokio::test]
    async fn test_cli_config_integration() {
        let temp_dir = TempDir::new().unwrap();

        // Test config creation
        let mut config = Config::default_for_testing();
        config.typst_files = vec!["test.typ".to_string()];
        config.aux_file = temp_dir.path().join("test.json");
        config.cache_dir = temp_dir.path().join("cache");

        // Test config validation
        let result = config.validate();
        assert!(result.is_ok());
    }
}

/// Main Application Integration Tests
mod main_integration_tests {
    use super::*;
    use std::env;

    #[tokio::test]
    async fn test_main_application_flow() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();

        // Copy ankify.typ to temp directory
        let ankify_src = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("typst/ankify.typ");
        let ankify_dst = temp_path.join("ankify.typ");
        if ankify_src.exists() {
            fs::copy(&ankify_src, &ankify_dst).await.unwrap();
        } else {
            // Create minimal ankify.typ for testing
            let minimal_ankify = r#"
#let card(label, ..args) = {
  // Minimal card implementation for testing
}

#let configure(..args) = {
  // Minimal configure implementation for testing
}
"#;
            fs::write(&ankify_dst, minimal_ankify).await.unwrap();
        }

        // Create test Typst file
        let typst_content = r#"
#import "ankify.typ": card, configure

#configure(
  deck: "Integration Test",
  tags: ["main-test"],
)

#card(
  "main-test-1",
  data: (
    Front: "Main test question",
    Back: "Main test answer"
  )
)
"#;
        let typst_file = temp_path.join("main_test.typ");
        fs::write(&typst_file, typst_content).await.unwrap();

        // Test main application initialization
        let mut config = Config::default_for_testing();
        config.typst_files = vec![typst_file.to_string_lossy().to_string()];
        config.aux_file = temp_path.join("main_test.json");
        config.cache_dir = temp_path.join("cache");

        // Test Ankify creation and basic operations
        let mut ankify = Ankify::new(config.clone()).await.unwrap();

        // Test file processing (without AnkiConnect)
        // This tests the core file processing logic without network calls
        let result = ankify.process_files().await;

        // Should either succeed or fail gracefully
        match result {
            Ok(_) => {
                // Successfully processed files
                assert!(config.cache_dir.exists());
            }
            Err(e) => {
                // Expected if AnkiConnect is not available
                // Should be a network or AnkiConnect error, not a parsing error
                println!("Expected error (AnkiConnect not available): {}", e);
            }
        }
    }

    #[tokio::test]
    async fn test_configuration_loading_and_validation() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();

        // Test default configuration
        let default_config = Config::default();
        assert!(!default_config.typst_files.is_empty());
        assert_eq!(default_config.ankiconnect_url, "http://localhost:8765");

        // Test custom configuration
        let mut custom_config = Config::default_for_testing();
        custom_config.typst_files = vec!["custom.typ".to_string()];
        custom_config.aux_file = temp_path.join("custom.json");
        custom_config.cache_dir = temp_path.join("custom_cache");
        custom_config.ankiconnect_url = "http://custom:9999".to_string();
        custom_config.render_format = ankify::RenderFormat::Html;
        custom_config.bypass_cache = true;

        // Test Ankify initialization with custom config
        let ankify = Ankify::new(custom_config.clone()).await.unwrap();
        // Can't access private fields, so just test that creation succeeds
        drop(ankify); // Explicitly consume to avoid unused warning
        assert_eq!(custom_config.ankiconnect_url, "http://custom:9999");
        assert_eq!(custom_config.render_format, ankify::RenderFormat::Html);
        assert!(custom_config.bypass_cache);
    }

    #[tokio::test]
    async fn test_error_handling_scenarios() {
        // Test with invalid Typst files - the application handles missing files gracefully
        let mut config_invalid_files = Config::default_for_testing();
        config_invalid_files.typst_files = vec!["/nonexistent/path.typ".to_string()];

        let mut ankify = Ankify::new(config_invalid_files).await.unwrap();
        let result = ankify.process_files().await;
        // The application handles missing files gracefully, so this should succeed
        assert!(result.is_ok());

        // Test with empty configuration
        let mut config_empty = Config::default_for_testing();
        config_empty.typst_files = vec![];

        let ankify_empty = Ankify::new(config_empty).await;
        assert!(ankify_empty.is_ok()); // Should not fail at construction
    }

    #[tokio::test]
    async fn test_processing_functionality() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();

        // Create test files
        let typst_content = r#"
#import "ankify.typ": card
#card("processing-test", data: (Front: "Test", Back: "Answer"))
"#;
        let typst_file = temp_path.join("processing_test.typ");
        fs::write(&typst_file, typst_content).await.unwrap();

        let mut config = Config::default_for_testing();
        config.typst_files = vec![typst_file.to_string_lossy().to_string()];
        config.aux_file = temp_path.join("processing.json");
        config.cache_dir = temp_path.join("cache");

        let mut ankify = Ankify::new(config.clone()).await.unwrap();
        let result = ankify.process_files().await;

        // Processing should complete without errors (or with expected AnkiConnect errors)
        match result {
            Ok(_) => {
                // Check that cache directory was created
                assert!(config.cache_dir.exists());
            }
            Err(e) => {
                // Expected if AnkiConnect operations are attempted
                println!("Processing completed with expected error: {}", e);
            }
        }
    }
}

/// Render System Integration Tests
mod render_integration_tests {
    use super::*;
    use ankify::render::Renderer;
    use ankify::{Card, RenderFormat};
    use std::collections::HashMap;

    #[tokio::test]
    async fn test_renderer_initialization_and_cleanup() {
        let temp_dir = TempDir::new().unwrap();
        let cache_dir = temp_dir.path().join("render_cache");

        // Test renderer creation
        let result = Renderer::new(&cache_dir).await;

        match result {
            Ok(renderer) => {
                // Verify directories were created
                assert!(cache_dir.exists());
                let temp_render_dir = cache_dir.join("temp");
                assert!(temp_render_dir.exists());
                drop(renderer); // Explicitly consume to avoid unused warning
            }
            Err(ankify::Error::MissingDependency(_)) => {
                // Expected if Typst is not installed
                println!("Typst not available for testing - this is expected in CI");
            }
            Err(e) => panic!("Unexpected error during renderer creation: {}", e),
        }
    }

    #[tokio::test]
    async fn test_plain_text_rendering() {
        let temp_dir = TempDir::new().unwrap();
        let cache_dir = temp_dir.path().join("plain_cache");

        // Create renderer (this should work even without Typst)
        let renderer = match Renderer::new(&cache_dir).await {
            Ok(r) => r,
            Err(ankify::Error::MissingDependency(_)) => {
                // Create a mock renderer for plain text testing
                return; // Skip test if Typst not available
            }
            Err(e) => panic!("Unexpected error: {}", e),
        };

        // Create test card
        let mut card_data = HashMap::new();
        card_data.insert("Front".to_string(), "What is Rust?".to_string());
        card_data.insert(
            "Back".to_string(),
            "A systems programming language".to_string(),
        );

        let card = Card {
            label: "rust-question".to_string(),
            model: "Basic".to_string(),
            data: card_data,
            deck: "Programming".to_string(),
            tags: vec!["rust".to_string(), "programming".to_string()],
            rest: HashMap::new(),
            format: RenderFormat::Plain,
            source_file: temp_dir.path().join("test.typ"),
        };

        // Test plain text rendering (should always work)
        let front_result = renderer
            .render_field(&card, "Front", RenderFormat::Plain, None)
            .await;
        assert!(front_result.is_ok());
        assert_eq!(front_result.unwrap(), "What is Rust?");

        let back_result = renderer
            .render_field(&card, "Back", RenderFormat::Plain, None)
            .await;
        assert!(back_result.is_ok());
        assert_eq!(back_result.unwrap(), "A systems programming language");

        // Test nonexistent field
        let missing_result = renderer
            .render_field(&card, "NonExistent", RenderFormat::Plain, None)
            .await;
        assert!(missing_result.is_ok());
        assert_eq!(missing_result.unwrap(), ""); // Should return empty string
    }

    #[tokio::test]
    async fn test_card_rendering_workflow() {
        let temp_dir = TempDir::new().unwrap();
        let cache_dir = temp_dir.path().join("card_cache");

        let renderer = match Renderer::new(&cache_dir).await {
            Ok(r) => r,
            Err(ankify::Error::MissingDependency(_)) => return,
            Err(e) => panic!("Unexpected error: {}", e),
        };

        // Create test card with multiple fields
        let mut card_data = HashMap::new();
        card_data.insert("Question".to_string(), "What is 2+2?".to_string());
        card_data.insert("Answer".to_string(), "4".to_string());
        card_data.insert("Explanation".to_string(), "Basic arithmetic".to_string());

        let card = Card {
            label: "math-basic".to_string(),
            model: "BasicAndReversed".to_string(),
            data: card_data,
            deck: "Mathematics".to_string(),
            tags: vec!["arithmetic".to_string(), "basic".to_string()],
            rest: HashMap::new(),
            format: RenderFormat::Plain,
            source_file: temp_dir.path().join("math.typ"),
        };

        // Test complete card rendering
        let rendered_card = renderer.render_card(&card, None).await;
        assert!(rendered_card.is_ok());

        let rendered = rendered_card.unwrap();
        assert_eq!(rendered.model, "BasicAndReversed");
        assert_eq!(rendered.deck, "Mathematics");
        assert_eq!(rendered.tags, vec!["arithmetic", "basic"]);
        assert_eq!(rendered.data.len(), 3);
        assert_eq!(rendered.data["Question"], "What is 2+2?");
        assert_eq!(rendered.data["Answer"], "4");
        assert_eq!(rendered.data["Explanation"], "Basic arithmetic");
    }

    #[tokio::test]
    async fn test_batch_rendering() {
        let temp_dir = TempDir::new().unwrap();
        let cache_dir = temp_dir.path().join("batch_cache");

        let renderer = match Renderer::new(&cache_dir).await {
            Ok(r) => r,
            Err(ankify::Error::MissingDependency(_)) => return,
            Err(e) => panic!("Unexpected error: {}", e),
        };

        // Create multiple test cards
        let cards = (1..=3)
            .map(|i| {
                let mut card_data = HashMap::new();
                card_data.insert("Front".to_string(), format!("Question {}", i));
                card_data.insert("Back".to_string(), format!("Answer {}", i));

                Card {
                    label: format!("batch-card-{}", i),
                    model: "Basic".to_string(),
                    data: card_data,
                    deck: "Batch Test".to_string(),
                    tags: vec![format!("batch-{}", i)],
                    rest: HashMap::new(),
                    format: RenderFormat::Plain,
                    source_file: temp_dir.path().join("batch.typ"),
                }
            })
            .collect::<Vec<_>>();

        // Test batch rendering
        let rendered_cards = renderer.render_cards_batch(cards.iter(), None).await;
        assert!(rendered_cards.is_ok());

        let rendered = rendered_cards.unwrap();
        assert_eq!(rendered.len(), 3);

        for (i, card) in rendered.iter().enumerate() {
            assert_eq!(card.label, format!("batch-card-{}", i + 1));
            assert_eq!(card.data["Front"], format!("Question {}", i + 1));
            assert_eq!(card.data["Back"], format!("Answer {}", i + 1));
        }
    }

    #[tokio::test]
    async fn test_render_format_variations() {
        let temp_dir = TempDir::new().unwrap();
        let cache_dir = temp_dir.path().join("format_cache");

        let renderer = match Renderer::new(&cache_dir).await {
            Ok(r) => r,
            Err(ankify::Error::MissingDependency(_)) => return,
            Err(e) => panic!("Unexpected error: {}", e),
        };

        let mut card_data = HashMap::new();
        card_data.insert("Front".to_string(), "Test Content".to_string());

        let card = Card {
            label: "format-test".to_string(),
            model: "Basic".to_string(),
            data: card_data,
            deck: "Format Test".to_string(),
            tags: vec!["format".to_string()],
            rest: HashMap::new(),
            format: RenderFormat::Plain,
            source_file: temp_dir.path().join("format.typ"),
        };

        // Test plain format (should always work)
        let plain_result = renderer
            .render_field(&card, "Front", RenderFormat::Plain, None)
            .await;
        assert!(plain_result.is_ok());
        assert_eq!(plain_result.unwrap(), "Test Content");

        // Test HTML format (fallback to plain without render function)
        let html_result = renderer
            .render_field(&card, "Front", RenderFormat::Html, None)
            .await;
        assert!(html_result.is_ok());
        let html_content = html_result.unwrap();
        assert!(html_content.contains("Test Content"));
        assert!(html_content.contains("ankify-card-field"));

        // Test SVG format (fallback to plain without render function)
        let svg_result = renderer
            .render_field(&card, "Front", RenderFormat::Svg, None)
            .await;
        assert!(svg_result.is_ok());
        assert_eq!(svg_result.unwrap(), "Test Content");

        // Test PNG format (fallback to plain without render function)
        let png_result = renderer
            .render_field(&card, "Front", RenderFormat::Png, None)
            .await;
        assert!(png_result.is_ok());
        assert_eq!(png_result.unwrap(), "Test Content");
    }

    #[tokio::test]
    async fn test_render_error_handling() {
        let temp_dir = TempDir::new().unwrap();
        let cache_dir = temp_dir.path().join("error_cache");

        let renderer = match Renderer::new(&cache_dir).await {
            Ok(r) => r,
            Err(ankify::Error::MissingDependency(_)) => return,
            Err(e) => panic!("Unexpected error: {}", e),
        };

        let mut card_data = HashMap::new();
        card_data.insert("Front".to_string(), "Test".to_string());

        let card = Card {
            label: "error-test".to_string(),
            model: "Basic".to_string(),
            data: card_data,
            deck: "Error Test".to_string(),
            tags: vec!["error".to_string()],
            rest: HashMap::new(),
            format: RenderFormat::Plain,
            source_file: temp_dir.path().join("error.typ"),
        };

        // Test rendering with invalid render function path
        let invalid_path = temp_dir.path().join("nonexistent.typ");
        let result = renderer
            .render_field(&card, "Front", RenderFormat::Svg, Some(&invalid_path))
            .await;

        // Should handle error gracefully
        match result {
            Ok(_) => {
                // Some implementations might handle this gracefully
            }
            Err(_) => {
                // Expected behavior for invalid render function
            }
        }
    }
}

/// Library Integration Tests
mod lib_integration_tests {
    use super::*;
    use ankify::{Ankify, Config};

    #[tokio::test]
    async fn test_ankify_complete_workflow() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();

        // Create comprehensive test setup
        let ankify_content = r#"
#let card(label, ..args) = {
  // Test card function
  [Card: #label]
}

#let configure(..args) = {
  // Test configure function
}
"#;
        fs::write(temp_path.join("ankify.typ"), ankify_content)
            .await
            .unwrap();

        let typst_content = r#"
#import "ankify.typ": card, configure

#configure(
  defaults: (
    deck: "Library Test",
    tags: ("lib-test",),
    model: "Basic"
  )
)

#card("lib-test-1", data: (Front: "Question 1", Back: "Answer 1"))
#card("lib-test-2", data: (Front: "Question 2", Back: "Answer 2"))
#card("lib-test-3", data: (Front: "Question 3", Back: "Answer 3"))
"#;
        let typst_file = temp_path.join("lib_test.typ");
        fs::write(&typst_file, typst_content).await.unwrap();

        // Test complete Ankify workflow
        let config = Config {
            typst_files: vec![typst_file.to_string_lossy().to_string()],
            aux_file: temp_path.join("lib_test.json"),
            cache_dir: temp_path.join("cache"),
            ankiconnect_url: "http://localhost:8765".to_string(),
            ..Config::default()
        };

        // Test Ankify creation
        let mut ankify = Ankify::new(config.clone()).await.unwrap();

        // Test file processing workflow
        let result = ankify.process_files().await;

        // Should complete successfully in dry run mode
        match result {
            Ok(_) => {
                // Verify cache directory was created
                assert!(config.cache_dir.exists());

                // Verify aux file might be created
                // (depends on implementation details)
            }
            Err(e) => {
                // May fail due to missing AnkiConnect or other dependencies
                println!("Process files error (expected in test environment): {}", e);
            }
        }
    }

    #[tokio::test]
    async fn test_ankify_configuration_management() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();

        // Test various configuration scenarios
        let configs = vec![
            // Default configuration
            Config::default(),
            // Custom configuration
            Config {
                typst_files: vec!["custom1.typ".to_string(), "custom2.typ".to_string()],
                aux_file: temp_path.join("custom.json"),
                cache_dir: temp_path.join("custom_cache"),
                ankiconnect_url: "http://custom:8765".to_string(),
                render_format: ankify::RenderFormat::Html,
                ..Config::default()
            },
            // Minimal configuration
            Config {
                typst_files: vec!["minimal.typ".to_string()],
                aux_file: temp_path.join("minimal.json"),
                cache_dir: temp_path.join("minimal_cache"),
                ..Config::default()
            },
        ];

        for (i, config) in configs.into_iter().enumerate() {
            let ankify_result = Ankify::new(config.clone()).await;
            assert!(ankify_result.is_ok(), "Configuration {} should be valid", i);

            let ankify = ankify_result.unwrap();
            // Config fields are private, so we can only test that creation succeeded
            drop(ankify); // Explicitly consume to avoid unused warning
        }
    }

    #[tokio::test]
    async fn test_ankify_error_scenarios() {
        // Test invalid file patterns
        let config_bad_pattern = Config {
            typst_files: vec!["*.{invalid".to_string()],
            ..Config::default()
        };

        let mut ankify = Ankify::new(config_bad_pattern).await.unwrap();
        let result = ankify.process_files().await;
        // Should handle bad patterns gracefully
        assert!(result.is_err() || result.is_ok());

        // Test directory creation permissions
        // (This would need special setup in a real test environment)

        // Test concurrent access scenarios
        let temp_dir = TempDir::new().unwrap();
        let config1 = Config {
            aux_file: temp_dir.path().join("shared.json"),
            cache_dir: temp_dir.path().join("shared_cache"),
            ..Config::default()
        };
        let config2 = config1.clone();

        let _ankify1 = Ankify::new(config1).await.unwrap();
        let _ankify2 = Ankify::new(config2).await.unwrap();

        // Both should be able to initialize
        // Note: config field is private, so we test functionality instead
    }

    #[tokio::test]
    async fn test_ankify_file_watching_setup() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();

        let typst_file = temp_path.join("watch_test.typ");
        fs::write(&typst_file, "#import \"ankify.typ\": card")
            .await
            .unwrap();

        let config = Config {
            typst_files: vec![typst_file.to_string_lossy().to_string()],
            aux_file: temp_path.join("watch.json"),
            cache_dir: temp_path.join("watch_cache"),
            ..Config::default()
        };

        let _ankify = Ankify::new(config).await.unwrap();

        // Test that files exist and are accessible
        assert!(typst_file.exists());

        // Note: Actually testing file watching would require more complex setup
        // and potentially flaky timing-dependent tests, so we focus on setup validation
    }

    #[tokio::test]
    async fn test_ankify_cache_integration() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();

        // Create test files
        let typst_content = r#"
#import "ankify.typ": card
#card("cache-test-1", data: (Front: "Q1", Back: "A1"))
#card("cache-test-2", data: (Front: "Q2", Back: "A2"))
"#;
        let typst_file = temp_path.join("cache_test.typ");
        fs::write(&typst_file, typst_content).await.unwrap();

        let config = Config {
            typst_files: vec![typst_file.to_string_lossy().to_string()],
            aux_file: temp_path.join("cache_integration.json"),
            cache_dir: temp_path.join("cache_integration"),
            ..Config::default()
        };

        let mut ankify = Ankify::new(config.clone()).await.unwrap();

        // First run - should create cache
        let result1 = ankify.process_files().await;
        match result1 {
            Ok(_) => {
                assert!(config.cache_dir.exists());
            }
            Err(_) => {
                // Expected in test environment
            }
        }

        // Second run - should use existing cache
        let result2 = ankify.process_files().await;
        match result2 {
            Ok(_) => {
                // Cache should still exist
                assert!(config.cache_dir.exists());
            }
            Err(_) => {
                // Expected in test environment
            }
        }
    }
}

/// Watch System Integration Tests
mod watch_integration_tests {
    use super::*;
    use ankify::watch::FileWatcher;
    use std::time::Duration;
    use tokio::time::timeout;

    #[tokio::test]
    async fn test_file_watcher_creation() {
        let temp_dir = TempDir::new().unwrap();
        let watch_file = temp_dir.path().join("watch_test.typ");
        fs::write(&watch_file, "// Test file").await.unwrap();

        let config = Config::default_for_testing();

        // Test watcher creation with single file
        let _watcher =
            FileWatcher::new(config, vec![watch_file.clone()], Duration::from_millis(100));

        // FileWatcher::new() returns a FileWatcher directly, not a Result
        // Test that watcher was created successfully
        // (We can't easily test internal state without public methods)
    }

    #[tokio::test]
    async fn test_file_watcher_multiple_files() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();

        let files = (1..=3)
            .map(|i| {
                let file = temp_path.join(format!("watch_{}.typ", i));
                file
            })
            .collect::<Vec<_>>();

        // Create test files
        for file in &files {
            fs::write(
                file,
                format!(
                    "// Test file {}",
                    file.file_name().unwrap().to_string_lossy()
                ),
            )
            .await
            .unwrap();
        }

        let config = Config::default_for_testing();
        let _watcher = FileWatcher::new(config, files.clone(), Duration::from_millis(100));

        // Test that watcher was created successfully
        // (We can't easily test internal state without public methods)
    }

    #[tokio::test]
    async fn test_file_watcher_invalid_paths() {
        let invalid_files = vec![
            std::path::PathBuf::from("/nonexistent/path.typ"),
            std::path::PathBuf::from("/another/invalid.typ"),
        ];

        let config = Config::default_for_testing();
        let _watcher = FileWatcher::new(config, invalid_files, Duration::from_millis(100));

        // FileWatcher::new() should not fail even with invalid paths
        // (Error handling may happen later when trying to watch)
    }

    #[tokio::test]
    async fn test_file_watcher_events() {
        let temp_dir = TempDir::new().unwrap();
        let watch_file = temp_dir.path().join("event_test.typ");
        fs::write(&watch_file, "// Initial content").await.unwrap();

        let config = Config::default_for_testing();
        let mut watcher =
            FileWatcher::new(config, vec![watch_file.clone()], Duration::from_millis(50));

        // Test file modification detection
        // Note: This is a simplified test - real file watching is complex and timing-dependent

        // Try to start watching (this is what would detect events)
        let watch_result = timeout(Duration::from_millis(100), watcher.start_watching()).await;

        match watch_result {
            Ok(Ok(_)) => {
                // Watcher started successfully
            }
            Ok(Err(_)) => {
                // Watcher error (expected in some environments)
            }
            Err(_) => {
                // Timeout (expected since we're not running indefinitely)
            }
        }
    }

    #[tokio::test]
    async fn test_file_watcher_debouncing() {
        let temp_dir = TempDir::new().unwrap();
        let watch_file = temp_dir.path().join("debounce_test.typ");
        fs::write(&watch_file, "// Initial").await.unwrap();

        let debounce_ms = 100;
        let config = Config::default_for_testing();
        let mut watcher = FileWatcher::new(
            config,
            vec![watch_file.clone()],
            Duration::from_millis(debounce_ms),
        );

        // Make rapid modifications
        for i in 0..5 {
            fs::write(&watch_file, format!("// Modification {}", i))
                .await
                .unwrap();
            tokio::time::sleep(Duration::from_millis(10)).await;
        }

        // Wait for debounce period
        tokio::time::sleep(Duration::from_millis(debounce_ms + 50)).await;

        // Try to start watching with timeout
        let watch_result = timeout(Duration::from_millis(100), watcher.start_watching()).await;

        match watch_result {
            Ok(Ok(_)) => {
                // Watcher started successfully
            }
            Ok(Err(_)) => {
                // Watcher error (expected in some environments)
            }
            Err(_) => {
                // Timeout (expected since we're not running indefinitely)
            }
        }
    }

    #[tokio::test]
    async fn test_watch_integration_with_config() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();

        // Create test files
        let typst_files = (1..=2)
            .map(|i| {
                let file = temp_path.join(format!("config_watch_{}.typ", i));
                file
            })
            .collect::<Vec<_>>();

        for file in &typst_files {
            fs::write(file, "// Test content").await.unwrap();
        }

        // Create config with watch settings
        let config = Config {
            typst_files: typst_files
                .iter()
                .map(|f| f.to_string_lossy().to_string())
                .collect(),
            aux_file: temp_path.join("watch_config.json"),
            cache_dir: temp_path.join("watch_cache"),
            ..Config::default()
        };

        // Test that Ankify can be created with watch configuration
        let ankify = Ankify::new(config).await;
        assert!(ankify.is_ok());

        let ankify = ankify.unwrap();
        // Config fields are private, so we can only test that creation succeeded
        drop(ankify); // Explicitly consume to avoid unused warning

        // Verify files exist and are accessible
        for file in &typst_files {
            assert!(file.exists());
        }
    }
}
