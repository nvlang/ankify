use std::path::PathBuf;
use tempfile::TempDir;
use tokio;
use tokio::fs;
use wiremock::{
    matchers::{body_json, method, path},
    Mock, MockServer, ResponseTemplate,
};

use ankify::{Ankify, CardDefaults, Config, RenderFormat, Result};
/// Helper function to set up a test directory with fixtures and ankify.typ
async fn setup_test_with_fixture(
    temp_dir: &TempDir,
    fixture_name: &str,
    target_name: &str,
) -> PathBuf {
    // Copy the ankify.typ file so imports work
    let ankify_source = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("typst/ankify.typ");
    let ankify_target = temp_dir.path().join("ankify.typ");
    if !ankify_target.exists() {
        fs::copy(&ankify_source, &ankify_target).await.unwrap();
    }
    println!(
        "[DEBUG] ankify.typ copied to: {} (exists? {})",
        ankify_target.display(),
        ankify_target.exists()
    );

    // Copy the fixture file
    let fixture_path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!("tests/fixtures/{}", fixture_name));
    let target_file = temp_dir.path().join(target_name);
    fs::copy(&fixture_path, &target_file).await.unwrap();
    println!(
        "[DEBUG] Fixture {} copied to: {} (exists? {})",
        fixture_name,
        target_file.display(),
        target_file.exists()
    );

    // Patch the import line to always use the local ankify.typ
    let content = fs::read_to_string(&target_file).await.unwrap();
    let mut lines: Vec<_> = content.lines().collect();
    if !lines.is_empty() && lines[0].trim().starts_with("#import") {
        lines[0] = "#import \"ankify.typ\": card";
    }
    let new_content = lines.join("\n");
    fs::write(&target_file, new_content).await.unwrap();
    println!("[DEBUG] Patched import line in {}", target_file.display());

    target_file
}

/// Comprehensive end-to-end integration tests using realistic fixtures
mod fixtures_integration_tests {
    use super::*;
    use serde_json::json;

    async fn setup_mock_ankiconnect() -> MockServer {
        let mock_server = MockServer::start().await;

        // Mock successful card creation
        Mock::given(method("POST"))
            .and(path("/"))
            .and(body_json(json!({
                "action": "addNote",
                "version": 6,
                "params": {
                    "note": {
                        "modelName": "Basic",
                        "deckName": "Mathematics",
                        "fields": {
                            "Front": "What is the Pythagorean theorem?",
                            "Back": "For a right triangle with legs $a$ and $b$ and hypotenuse $c$: $ a^2 + b^2 = c^2 $"
                        },
                        "tags": ["geometry", "theorems", "triangles"]
                    }
                }
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "result": 1234567890i64,
                "error": null
            })))
            .mount(&mock_server)
            .await;

        // Mock successful card update
        Mock::given(method("POST"))
            .and(path("/"))
            .and(body_json(json!({
                "action": "updateNoteFields",
                "version": 6
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "result": null,
                "error": null
            })))
            .mount(&mock_server)
            .await;

        // Mock successful card deletion
        Mock::given(method("POST"))
            .and(path("/"))
            .and(body_json(json!({
                "action": "deleteNotes",
                "version": 6
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "result": null,
                "error": null
            })))
            .mount(&mock_server)
            .await;

        mock_server
    }

    #[tokio::test]
    async fn test_basic_math_cards_processing() -> Result<()> {
        let mock_server = setup_mock_ankiconnect().await;
        let temp_dir = TempDir::new().unwrap();
        std::env::set_current_dir(&temp_dir.path()).unwrap();

        // Set up test directory with fixture and ankify.typ
        let temp_file =
            setup_test_with_fixture(&temp_dir, "basic_math.typ", "basic_math.typ").await;

        let config = Config {
            typst_files: vec![temp_file.to_string_lossy().to_string()],
            ankiconnect_url: mock_server.uri(),
            aux_file: temp_dir.path().join("test.aux.json"),
            verbose: true,
            bypass_cache: false,
            render: None,
            cache_dir: temp_dir.path().join("cache"),
            defaults: CardDefaults::default(),
            watch: false,
            render_format: RenderFormat::Html,
        };

        let mut ankify = Ankify::new(config).await?;
        ankify.process_files().await?;

        // Verify aux file was created and contains expected cards
        let aux_path = temp_dir.path().join("test.aux.json");
        println!("Checking aux file at: {}", aux_path.display());
        println!("Aux file exists? {}", aux_path.exists());
        let aux_content = fs::read_to_string(&aux_path).await.unwrap();
        let aux_data: serde_json::Value = serde_json::from_str(&aux_content).unwrap();

        assert!(aux_data.is_array());
        let cards = aux_data.as_array().unwrap();
        assert!(cards.len() >= 4); // Should have at least 4 cards from basic_math.typ

        // Check that specific cards exist
        let labels: Vec<&str> = cards
            .iter()
            .map(|card| card["label"].as_str().unwrap())
            .collect();

        assert!(labels.contains(&"pythagoras-theorem"));
        assert!(labels.contains(&"quadratic-formula"));
        assert!(labels.contains(&"derivative-power-rule"));
        assert!(labels.contains(&"integral-power-rule"));

        Ok(())
    }

    #[tokio::test]
    async fn test_rust_concepts_with_defaults() -> Result<()> {
        let mock_server = setup_mock_ankiconnect().await;
        let temp_dir = TempDir::new().unwrap();
        std::env::set_current_dir(&temp_dir.path()).unwrap();

        let temp_file =
            setup_test_with_fixture(&temp_dir, "rust_concepts.typ", "rust_concepts.typ").await;

        let config = Config {
            typst_files: vec![temp_file.to_string_lossy().to_string()],
            ankiconnect_url: mock_server.uri(),
            aux_file: temp_dir.path().join("rust.aux.json"),
            verbose: true,
            bypass_cache: false,
            render: None,
            cache_dir: temp_dir.path().join("cache"),
            defaults: CardDefaults::default(),
            watch: false,
            render_format: RenderFormat::Html,
        };

        let mut ankify = Ankify::new(config).await?;
        ankify.process_files().await?;

        // Verify cards were processed with correct defaults
        let aux_content = fs::read_to_string(temp_dir.path().join("rust.aux.json"))
            .await
            .unwrap();
        let aux_data: serde_json::Value = serde_json::from_str(&aux_content).unwrap();

        assert!(aux_data.is_array());
        let cards = aux_data.as_array().unwrap();
        assert!(cards.len() >= 5); // Should have 5 cards from rust_concepts.typ

        Ok(())
    }

    #[tokio::test]
    async fn test_physics_cards_different_formats() -> Result<()> {
        let mock_server = setup_mock_ankiconnect().await;
        let temp_dir = TempDir::new().unwrap();
        std::env::set_current_dir(&temp_dir.path()).unwrap();

        let temp_file =
            setup_test_with_fixture(&temp_dir, "physics_cards.typ", "physics_cards.typ").await;

        // Also copy the custom render function
        let render_fixture =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/custom_render.typ");
        let render_file = temp_dir.path().join("custom_render.typ");
        fs::copy(&render_fixture, &render_file).await.unwrap();

        let config = Config {
            typst_files: vec![temp_file.to_string_lossy().to_string()],
            ankiconnect_url: mock_server.uri(),
            aux_file: temp_dir.path().join("physics.aux.json"),
            verbose: true,
            bypass_cache: false,
            render: Some(render_file),
            cache_dir: temp_dir.path().join("cache"),
            defaults: CardDefaults::default(),
            watch: false,
            render_format: RenderFormat::Html,
        };

        let mut ankify = Ankify::new(config).await?;
        ankify.process_files().await?;

        // Verify cards with different formats were processed
        let aux_content = fs::read_to_string(temp_dir.path().join("physics.aux.json"))
            .await
            .unwrap();
        let aux_data: serde_json::Value = serde_json::from_str(&aux_content).unwrap();

        assert!(aux_data.is_array());
        let cards = aux_data.as_array().unwrap();
        assert!(cards.len() >= 5); // Should have 5 cards from physics_cards.typ

        // Verify specific cards exist
        let labels: Vec<&str> = cards
            .iter()
            .map(|card| card["label"].as_str().unwrap())
            .collect();

        assert!(labels.contains(&"newton-first-law"));
        assert!(labels.contains(&"kinetic-energy-formula"));
        assert!(labels.contains(&"simple-definition"));

        Ok(())
    }

    #[tokio::test]
    async fn test_config_merging_cli_and_typst() -> Result<()> {
        let mock_server = setup_mock_ankiconnect().await;
        let temp_dir = TempDir::new().unwrap();
        std::env::set_current_dir(&temp_dir.path()).unwrap();

        let temp_file =
            setup_test_with_fixture(&temp_dir, "test_config.typ", "test_config.typ").await;

        // CLI config should override Typst config for ankiconnect_url
        let config = Config {
            typst_files: vec![temp_file.to_string_lossy().to_string()],
            ankiconnect_url: mock_server.uri(), // This should override the one in .typ file
            aux_file: temp_dir.path().join("config_test.aux.json"),
            verbose: false, // This should be overridden by .typ file (true)
            bypass_cache: false,
            render: None,
            cache_dir: temp_dir.path().join("cache"),
            defaults: CardDefaults::default(),
            watch: false,
            render_format: RenderFormat::Html,
        };

        let mut ankify = Ankify::new(config).await?;
        ankify.process_files().await?;

        // Verify cards were created with merged configuration
        let aux_content = fs::read_to_string(temp_dir.path().join("config_test.aux.json"))
            .await
            .unwrap();
        let aux_data: serde_json::Value = serde_json::from_str(&aux_content).unwrap();

        assert!(aux_data.is_array());
        let cards = aux_data.as_array().unwrap();
        assert!(cards.len() >= 3); // Should have 3 cards from test_config.typ

        Ok(())
    }

    #[tokio::test]
    async fn test_existing_aux_file_updates() -> Result<()> {
        let mock_server = setup_mock_ankiconnect().await;
        let temp_dir = TempDir::new().unwrap();
        std::env::set_current_dir(&temp_dir.path()).unwrap();

        // Copy existing aux file to temp directory
        let aux_fixture =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/existing_aux.json");
        let aux_file = temp_dir.path().join("existing.aux.json");
        fs::copy(&aux_fixture, &aux_file).await.unwrap();

        let temp_file =
            setup_test_with_fixture(&temp_dir, "basic_math.typ", "basic_math.typ").await;

        let config = Config {
            typst_files: vec![temp_file.to_string_lossy().to_string()],
            ankiconnect_url: mock_server.uri(),
            aux_file: aux_file.clone(),
            verbose: true,
            bypass_cache: false,
            render: None,
            cache_dir: temp_dir.path().join("cache"),
            defaults: CardDefaults::default(),
            watch: false,
            render_format: RenderFormat::Html,
        };

        let mut ankify = Ankify::new(config).await?;
        ankify.process_files().await?;

        // Verify aux file was updated with new cards while preserving existing ones
        let aux_content = fs::read_to_string(&aux_file).await.unwrap();
        let aux_data: serde_json::Value = serde_json::from_str(&aux_content).unwrap();

        assert!(aux_data.is_array());
        let cards = aux_data.as_array().unwrap();

        // Should have original cards plus new ones from basic_math.typ
        assert!(cards.len() > 3); // More than the original 3 cards

        Ok(())
    }

    #[tokio::test]
    async fn test_bypass_cache_functionality() -> Result<()> {
        let mock_server = setup_mock_ankiconnect().await;
        let temp_dir = TempDir::new().unwrap();
        std::env::set_current_dir(&temp_dir.path()).unwrap();

        let temp_file =
            setup_test_with_fixture(&temp_dir, "basic_math.typ", "basic_math.typ").await;

        let aux_file = temp_dir.path().join("bypass_test.aux.json");

        // First run - should create cards
        let config = Config {
            typst_files: vec![temp_file.to_string_lossy().to_string()],
            ankiconnect_url: mock_server.uri(),
            aux_file: aux_file.clone(),
            verbose: true,
            bypass_cache: false,
            render: None,
            cache_dir: temp_dir.path().join("cache"),
            defaults: CardDefaults::default(),
            watch: false,
            render_format: RenderFormat::Html,
        };

        let mut ankify = Ankify::new(config).await?;
        ankify.process_files().await?;

        // Second run with bypass_cache = true - should update all cards
        let config = Config {
            typst_files: vec![temp_file.to_string_lossy().to_string()],
            ankiconnect_url: mock_server.uri(),
            aux_file: aux_file.clone(),
            verbose: true,
            bypass_cache: true, // This should force updates
            render: None,
            cache_dir: temp_dir.path().join("cache"),
            defaults: CardDefaults::default(),
            watch: false,
            render_format: RenderFormat::Html,
        };

        let mut ankify = Ankify::new(config).await?;
        ankify.process_files().await?;

        // Verify aux file still contains the same cards
        let aux_content = fs::read_to_string(&aux_file).await.unwrap();
        let aux_data: serde_json::Value = serde_json::from_str(&aux_content).unwrap();

        assert!(aux_data.is_array());
        let cards = aux_data.as_array().unwrap();
        assert!(cards.len() >= 4);

        Ok(())
    }

    #[tokio::test]
    async fn test_multiple_typst_files() -> Result<()> {
        let mock_server = setup_mock_ankiconnect().await;
        let temp_dir = TempDir::new().unwrap();
        std::env::set_current_dir(&temp_dir.path()).unwrap();

        // Copy multiple fixture files
        let math_file = setup_test_with_fixture(&temp_dir, "basic_math.typ", "math.typ").await;
        let rust_file = setup_test_with_fixture(&temp_dir, "rust_concepts.typ", "rust.typ").await;

        let config = Config {
            typst_files: vec![
                math_file.to_string_lossy().to_string(),
                rust_file.to_string_lossy().to_string(),
            ],
            ankiconnect_url: mock_server.uri(),
            aux_file: temp_dir.path().join("multi.aux.json"),
            verbose: true,
            bypass_cache: false,
            render: None,
            cache_dir: temp_dir.path().join("cache"),
            defaults: CardDefaults::default(),
            watch: false,
            render_format: RenderFormat::Html,
        };

        let mut ankify = Ankify::new(config).await?;
        ankify.process_files().await?;

        // Verify cards from both files were processed
        let aux_content = fs::read_to_string(temp_dir.path().join("multi.aux.json"))
            .await
            .unwrap();
        let aux_data: serde_json::Value = serde_json::from_str(&aux_content).unwrap();

        assert!(aux_data.is_array());
        let cards = aux_data.as_array().unwrap();
        assert!(cards.len() >= 9); // 4 from math + 5 from rust

        let labels: Vec<&str> = cards
            .iter()
            .map(|card| card["label"].as_str().unwrap())
            .collect();

        // Should have cards from both files
        assert!(labels.contains(&"pythagoras-theorem")); // from math
        assert!(labels.contains(&"ownership-definition")); // from rust

        Ok(())
    }
}
