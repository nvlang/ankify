use ankify::{Ankify, Config};
use serde_json::json;
use tempfile::TempDir;
use tokio::fs;
use wiremock::{
    matchers::{body_json, body_partial_json, method, path},
    Mock, MockServer, ResponseTemplate,
};

/// End-to-end integration test that follows the complete production workflow
/// with a mocked AnkiConnect server.
///
/// This test validates:
/// 1. Typst file extraction and parsing
/// 2. Card metadata extraction
/// 3. Cache planning logic
/// 4. Card rendering with Typst
/// 5. AnkiConnect API communication (mocked)
/// 6. Cache state management
#[tokio::test]
async fn test_complete_e2e_workflow() -> Result<(), Box<dyn std::error::Error>> {
    // Setup mock AnkiConnect server
    let mock_server = MockServer::start().await;

    // Mock AnkiConnect version check
    Mock::given(method("POST"))
        .and(path("/"))
        .and(body_json(json!({
            "action": "version",
            "version": 6
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "result": 6,
            "error": null
        })))
        .mount(&mock_server)
        .await;

    // Mock deck names request - returns existing decks
    Mock::given(method("POST"))
        .and(path("/"))
        .and(body_json(json!({
            "action": "deckNames",
            "version": 6
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "result": ["Default", "Mathematics"],
            "error": null
        })))
        .mount(&mock_server)
        .await;

    // Mock addNote requests - returns a note ID
    Mock::given(method("POST"))
        .and(path("/"))
        .and(body_partial_json(json!({
            "action": "addNote",
            "version": 6
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "result": 1234567890u64,
            "error": null
        })))
        .mount(&mock_server)
        .await;

    // Setup test environment
    let temp_dir = TempDir::new()?;
    let temp_path = temp_dir.path();

    // Copy the real ankify.typ library file from fixtures
    let fixture_ankify = std::path::Path::new("tests/fixtures/ankify.typ");
    let ankify_typ_path = temp_path.join("ankify.typ");
    fs::copy(fixture_ankify, &ankify_typ_path).await?;

    // Create test Typst file with real cards
    let typst_content = r#"
#import "ankify.typ": card

= Math Study Cards

#card("pythagorean-theorem", 
  deck: "Mathematics",
  tags: ("geometry", "theorems"), 
  data: (
    Front: "What is the Pythagorean theorem?",
    Back: "For a right triangle with legs $a$ and $b$ and hypotenuse $c$: $ a^2 + b^2 = c^2 $"
  )
)

#card("quadratic-formula",
  deck: "Mathematics", 
  tags: ("algebra", "formulas"),
  data: (
    Front: "What is the quadratic formula?",
    Back: "For equation $a x^2 + b x + c = 0$: $ x = (-b ± sqrt(b^2 - 4 a c)) / (2 a) $"
  )
)
"#;
    let typst_file = temp_path.join("math_cards.typ");
    fs::write(&typst_file, typst_content).await?;

    // Configure Ankify to use our test files and mock server
    let config = Config {
        typst_files: vec![typst_file.to_string_lossy().to_string()],
        ankiconnect_url: mock_server.uri(),
        aux_file: temp_path.join("test.aux.json"),
        cache_dir: temp_path.join("cache"),
        verbose: true,
        bypass_cache: false,
        render: None,
        defaults: ankify::CardDefaults::default(),
        render_format: ankify::RenderFormat::Single(ankify::FieldFormat::Svg),
        watch: false,
    };

    println!("=== Starting E2E Test ===");
    println!("Temp dir: {}", temp_path.display());
    println!("Typst file: {}", typst_file.display());
    println!("Mock server: {}", mock_server.uri());

    // Create Ankify instance and process files
    let mut ankify = Ankify::new(config.clone()).await?;

    println!("=== Processing files (first run) ===");
    ankify.process_files().await?;

    // Verify results
    println!("=== Verifying results ===");

    // Check that aux file was created
    assert!(config.aux_file.exists(), "Aux file should exist");

    // Check that cache directory was created
    assert!(config.cache_dir.exists(), "Cache directory should exist");

    // Check aux file contents
    let aux_content = fs::read_to_string(&config.aux_file).await?;
    let aux_data: serde_json::Value = serde_json::from_str(&aux_content)?;
    println!(
        "Aux file contents: {}",
        serde_json::to_string_pretty(&aux_data)?
    );

    // Should have entries for both cards
    let entries = aux_data.as_array().expect("Aux file should contain array");
    assert_eq!(entries.len(), 2, "Should have 2 card entries");

    // Verify card labels are present
    let labels: Vec<String> = entries
        .iter()
        .map(|entry| entry["label"].as_str().unwrap().to_string())
        .collect();
    assert!(labels.contains(&"pythagorean-theorem".to_string()));
    assert!(labels.contains(&"quadratic-formula".to_string()));

    println!("=== Processing files (second run - should be cached) ===");

    // Process again - should use cache and skip operations
    let mut ankify2 = Ankify::new(config.clone()).await?;
    ankify2.process_files().await?;

    println!("=== E2E Test Completed Successfully! ===");

    Ok(())
}

/// Test that demonstrates cache behavior with file changes
#[tokio::test]
async fn test_e2e_with_file_changes() -> Result<(), Box<dyn std::error::Error>> {
    // Setup mock AnkiConnect server
    let mock_server = MockServer::start().await;

    // Mock version check
    Mock::given(method("POST"))
        .and(path("/"))
        .and(body_json(json!({
            "action": "version",
            "version": 6
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "result": 6,
            "error": null
        })))
        .mount(&mock_server)
        .await;

    // Mock deck names request
    Mock::given(method("POST"))
        .and(path("/"))
        .and(body_json(json!({
            "action": "deckNames",
            "version": 6
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "result": ["Default"],
            "error": null
        })))
        .mount(&mock_server)
        .await;

    // Mock all addNote/updateNote requests
    Mock::given(method("POST"))
        .and(path("/"))
        .and(body_partial_json(json!({
            "action": "addNote",
            "version": 6
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "result": 1234567890u64,
            "error": null
        })))
        .mount(&mock_server)
        .await;

    Mock::given(method("POST"))
        .and(path("/"))
        .and(body_partial_json(json!({
            "action": "updateNote",
            "version": 6
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "result": null,
            "error": null
        })))
        .mount(&mock_server)
        .await;

    let temp_dir = TempDir::new()?;
    let temp_path = temp_dir.path();

    // Copy the real ankify.typ library file from fixtures
    let fixture_ankify = std::path::Path::new("tests/fixtures/ankify.typ");
    fs::copy(fixture_ankify, temp_path.join("ankify.typ")).await?;

    // Create initial Typst file
    let initial_content = r#"
#import "ankify.typ": card

#card("test-card", 
  data: (
    Front: "Original question",
    Back: "Original answer"
  )
)
"#;
    let typst_file = temp_path.join("test.typ");
    fs::write(&typst_file, initial_content).await?;

    let config = Config {
        typst_files: vec![typst_file.to_string_lossy().to_string()],
        ankiconnect_url: mock_server.uri(),
        aux_file: temp_path.join("cache_test.aux.json"),
        cache_dir: temp_path.join("cache"),
        verbose: true,
        bypass_cache: false,
        render: None,
        defaults: ankify::CardDefaults::default(),
        render_format: ankify::RenderFormat::Single(ankify::FieldFormat::Svg),
        watch: false,
    };

    // First run - create card
    println!("=== First run ===");
    let mut ankify = Ankify::new(config.clone()).await?;
    ankify.process_files().await?;

    // Modify the file content
    let modified_content = r#"
#import "ankify.typ": card

#card("test-card", 
  data: (
    Front: "Modified question", 
    Back: "Modified answer"
  )
)
"#;
    fs::write(&typst_file, modified_content).await?;

    // Second run - should detect change and update
    println!("=== Second run with changes ===");
    let mut ankify2 = Ankify::new(config.clone()).await?;
    ankify2.process_files().await?;

    println!("=== Cache change test completed! ===");

    Ok(())
}
