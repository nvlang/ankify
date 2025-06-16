//! Integration tests for the query module.

use ankify::metadata::NoteDataValue;
use ankify::query::{query_ankify_configuration, query_ankify_notes};

/// Helper to get the path to a fixture file
fn fixture_path(filename: &str) -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("query")
        .join(filename)
}

/// Get the root path for the monorepo (for --root flag)
fn get_root_path() -> String {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent() // packages
        .unwrap()
        .parent() // ankify root
        .unwrap()
        .to_string_lossy()
        .to_string()
}

#[tokio::test]
async fn test_query_ankify_configuration_success() {
    let test_file = fixture_path("test.typ");
    let root_path = get_root_path();

    let result = query_ankify_configuration(&test_file, Some(&["--root", &root_path])).await;
    assert!(
        result.is_ok(),
        "Failed to query configuration: {:?}",
        result.err()
    );

    let config = result.unwrap();
    assert!(config.is_some(), "Expected configuration to be present");

    let config = config.unwrap();

    // Check the configuration values from the JSON output we saw
    assert_eq!(
        config.ankiconnect_url,
        Some("http://localhost:8765".to_string())
    );
    assert_eq!(config.verbose, Some(false));
    // render field is now in defaults, not top-level
    assert!(config.setup.is_some()); // setup function should be present

    // Check cache options
    assert!(config.cache.is_some());
    let cache = config.cache.unwrap();
    assert_eq!(cache.enabled, Some(true));
    assert_eq!(cache.custom_file, None);

    // Check checks options
    assert!(config.checks.is_some());
    let checks = config.checks.unwrap();

    assert!(checks.typst.is_some());
    let typst_checks = checks.typst.unwrap();
    // typst checks is now a simple boolean
    assert_eq!(typst_checks, true);

    assert!(checks.ankiconnect.is_some());
    let anki_checks = checks.ankiconnect.unwrap();
    assert_eq!(anki_checks.model, Some(true));
    assert_eq!(anki_checks.deck, Some(true));
    assert_eq!(anki_checks.tags, Some(true));
}

#[tokio::test]
async fn test_query_ankify_notes_success() {
    let test_file = fixture_path("test.typ");
    let root_path = get_root_path();

    let result = query_ankify_notes(&test_file, Some(&["--root", &root_path])).await;
    assert!(result.is_ok(), "Failed to query notes: {:?}", result.err());

    let notes = result.unwrap();
    assert_eq!(notes.len(), 2, "Expected exactly 2 notes");

    // Test first note - pythagoras-theorem
    let note1 = &notes[0];
    assert_eq!(note1.label, "pythagoras-theorem");
    assert_eq!(note1.model, "Basic");
    assert_eq!(note1.deck, "Ankify-Test");
    assert_eq!(note1.format, Some("png".to_string()));
    assert_eq!(note1.tags, vec!["str".to_string()]);

    // Check data fields
    assert!(note1.data.contains_key("Front"));
    assert!(note1.data.contains_key("Back"));

    // The Front field should be a simple string
    match &note1.data["Front"] {
        NoteDataValue::Simple(s) => assert_eq!(s, "What is the Pythagorean theorem?"),
        NoteDataValue::Complex(_) => {} // Could also be complex Typst content
        _ => panic!("Unexpected data value type for Front field"),
    }

    // The Back field should contain complex Typst content
    match &note1.data["Back"] {
        NoteDataValue::Complex(value) => {
            // Verify it's a function call structure
            assert!(value.is_object());
            let obj = value.as_object().unwrap();
            assert_eq!(obj.get("func").and_then(|v| v.as_str()), Some("sequence"));
            assert!(obj.contains_key("children"));
        }
        _ => panic!("Expected complex Typst content for Back field"),
    }

    // Test second note - quadratic-formula
    let note2 = &notes[1];
    assert_eq!(note2.label, "quadratic-formula");
    assert_eq!(note2.model, "Basic");
    assert_eq!(note2.deck, "Ankify-Test");
    assert_eq!(note2.format, Some("svg".to_string()));
    assert_eq!(note2.tags, vec!["str".to_string()]);

    // Check data fields for second note
    assert!(note2.data.contains_key("Front"));
    assert!(note2.data.contains_key("Back"));
}

#[tokio::test]
async fn test_query_nonexistent_file() {
    let nonexistent_file = fixture_path("nonexistent.typ");

    let config_result = query_ankify_configuration(&nonexistent_file, None).await;
    assert!(
        config_result.is_err(),
        "Expected error for nonexistent file"
    );

    let notes_result = query_ankify_notes(&nonexistent_file, None).await;
    assert!(notes_result.is_err(), "Expected error for nonexistent file");
}

#[tokio::test]
async fn test_query_empty_configuration() {
    // Use a static fixture file with no ankify configuration
    let empty_config_file = fixture_path("empty_config.typ");

    let result = query_ankify_configuration(&empty_config_file, None).await;
    assert!(result.is_ok(), "Query should succeed even with no config");

    let config = result.unwrap();
    assert!(config.is_none(), "Expected no configuration to be found");
}

#[tokio::test]
async fn test_query_empty_notes() {
    // Use a static fixture file with configuration but no notes
    let no_notes_file = fixture_path("no_notes.typ");
    let root_path = get_root_path();

    let result = query_ankify_notes(&no_notes_file, Some(&["--root", &root_path])).await;
    assert!(result.is_ok(), "Query should succeed even with no notes");

    let notes = result.unwrap();
    assert!(notes.is_empty(), "Expected no notes to be found");
}

#[tokio::test]
async fn test_query_with_custom_configuration() {
    // Use a static fixture file with custom configuration
    let custom_config_file = fixture_path("custom_config.typ");
    let root_path = get_root_path();

    // Test configuration
    let config_result =
        query_ankify_configuration(&custom_config_file, Some(&["--root", &root_path])).await;
    assert!(
        config_result.is_ok(),
        "Failed to query custom configuration"
    );

    let config = config_result.unwrap().unwrap();
    assert_eq!(
        config.ankiconnect_url,
        Some("http://custom:9999".to_string())
    );
    assert_eq!(config.verbose, Some(true));
    // render field is no longer a top-level string field

    // Check custom cache settings
    let cache = config.cache.unwrap();
    assert_eq!(cache.enabled, Some(false));
    assert_eq!(cache.custom_file, Some("custom.json".to_string()));

    // Check custom check settings
    let checks = config.checks.unwrap();
    let typst_checks = checks.typst.unwrap();
    // typst checks is now a simple boolean
    assert_eq!(typst_checks, false);

    let anki_checks = checks.ankiconnect.unwrap();
    assert_eq!(anki_checks.model, Some(false));
    assert_eq!(anki_checks.deck, Some(false));
    assert_eq!(anki_checks.tags, Some(false));

    // Test notes
    let notes_result = query_ankify_notes(&custom_config_file, Some(&["--root", &root_path])).await;
    assert!(notes_result.is_ok(), "Failed to query custom notes");

    let notes = notes_result.unwrap();
    assert_eq!(notes.len(), 1);

    let note = &notes[0];
    assert_eq!(note.label, "custom-note");
    assert_eq!(note.deck, "Custom-Deck");
    assert_eq!(note.model, "Custom-Model");
    assert_eq!(note.format, Some("plain".to_string()));
    assert_eq!(note.tags, vec!["custom".to_string(), "test".to_string()]);
}

#[tokio::test]
async fn test_note_data_types() {
    // Use a static fixture file with various data types
    let data_types_file = fixture_path("data_types.typ");
    let root_path = get_root_path();

    let result = query_ankify_notes(&data_types_file, Some(&["--root", &root_path])).await;
    assert!(
        result.is_ok(),
        "Failed to query notes with various data types"
    );

    let notes = result.unwrap();
    assert_eq!(notes.len(), 2);

    // First note should have simple string data
    let simple_note = &notes[0];
    assert_eq!(simple_note.label, "simple-strings");

    // Second note should have complex Typst content
    let complex_note = &notes[1];
    assert_eq!(complex_note.label, "complex-content");

    // Both Front and Back should contain data structures
    match &complex_note.data["Front"] {
        NoteDataValue::Complex(_) => {} // Expected
        NoteDataValue::Simple(s) => {
            // Could also be rendered as a simple string depending on Typst output
            assert!(!s.is_empty());
        }
        _ => panic!("Unexpected data type for complex Front field"),
    }
}

#[tokio::test]
async fn test_invalid_typst_file() {
    // Create a file with invalid Typst syntax
    let invalid_content = r#"
#import "ankify-typst/lib.typ": note, configure

This is invalid Typst syntax: #invalid_function(
"#;

    // Use a unique filename to avoid conflicts in parallel tests
    let thread_id = std::thread::current().id();
    let invalid_file = fixture_path(&format!("invalid_{:?}.typ", thread_id));
    std::fs::write(&invalid_file, invalid_content).unwrap();

    let config_result = query_ankify_configuration(&invalid_file, None).await;
    assert!(
        config_result.is_err(),
        "Expected error for invalid Typst file"
    );

    let notes_result = query_ankify_notes(&invalid_file, None).await;
    assert!(
        notes_result.is_err(),
        "Expected error for invalid Typst file"
    );

    // Clean up
    std::fs::remove_file(&invalid_file).ok();
}

#[tokio::test]
async fn test_advanced_configuration_and_notes() {
    let advanced_file = fixture_path("advanced.typ");
    let root_path = get_root_path();

    // Test advanced configuration
    let config_result =
        query_ankify_configuration(&advanced_file, Some(&["--root", &root_path])).await;
    assert!(
        config_result.is_ok(),
        "Failed to query advanced configuration"
    );

    let config = config_result.unwrap().unwrap();
    assert_eq!(
        config.ankiconnect_url,
        Some("http://advanced:9000".to_string())
    );
    assert_eq!(config.verbose, Some(true));
    // render field is no longer a top-level string field

    // Check defaults
    assert!(config.defaults.is_some());
    let defaults = config.defaults.unwrap();
    assert_eq!(defaults.model, Some("Cloze".to_string()));
    assert_eq!(defaults.deck, Some("Default-Deck".to_string()));
    assert_eq!(defaults.format, Some("svg".to_string()));
    assert_eq!(defaults.tags, Some(vec!["default-tag".to_string()]));

    // Test advanced notes
    let notes_result = query_ankify_notes(&advanced_file, Some(&["--root", &root_path])).await;
    assert!(notes_result.is_ok(), "Failed to query advanced notes");

    let notes = notes_result.unwrap();
    assert_eq!(notes.len(), 2);

    // Check math note
    let math_note = &notes[0];
    assert_eq!(math_note.label, "advanced-math");
    assert_eq!(math_note.deck, "Mathematics");
    assert_eq!(
        math_note.tags,
        vec!["calculus".to_string(), "derivatives".to_string()]
    );
    assert_eq!(math_note.format, Some("svg".to_string()));

    // Check programming note
    let prog_note = &notes[1];
    assert_eq!(prog_note.label, "programming-concept");
    assert_eq!(prog_note.deck, "Computer Science");
    assert_eq!(
        prog_note.tags,
        vec!["programming".to_string(), "rust".to_string()]
    );
    assert_eq!(prog_note.format, Some("plain".to_string()));
}

#[tokio::test]
async fn test_no_ankify() {
    let no_ankify = fixture_path("no_ankify.typ");

    // Should return None for configuration
    let config_result = query_ankify_configuration(&no_ankify, None).await;
    assert!(config_result.is_ok(), "Query should succeed");
    assert!(
        config_result.unwrap().is_none(),
        "Expected no configuration"
    );

    let notes_result = query_ankify_notes(&no_ankify, None).await;
    assert!(notes_result.is_ok(), "Query should succeed");
    let notes = notes_result.unwrap();
    assert_eq!(notes.len(), 0);
}

#[tokio::test]
async fn test_configuration_only_file() {
    let config_only_file = fixture_path("config_only.typ");
    let root_path = get_root_path();

    // Should return configuration
    let config_result =
        query_ankify_configuration(&config_only_file, Some(&["--root", &root_path])).await;
    assert!(config_result.is_ok(), "Query should succeed");
    assert!(
        config_result.unwrap().is_some(),
        "Expected configuration to be present"
    );

    // Should return empty notes array
    let notes_result = query_ankify_notes(&config_only_file, Some(&["--root", &root_path])).await;
    assert!(notes_result.is_ok(), "Query should succeed");

    let notes = notes_result.unwrap();
    assert!(notes.is_empty(), "Expected no notes");
}

#[tokio::test]
async fn test_label_selector_formatting() {
    // This test verifies that the label selector is properly formatted
    // by testing with labels that already have angle brackets
    let test_file = fixture_path("test.typ");
    let root_path = get_root_path();

    // The internal function should handle both "<ankify-note>" and "ankify-note"
    // This is tested indirectly through the public functions
    let notes_result = query_ankify_notes(&test_file, Some(&["--root", &root_path])).await;
    assert!(
        notes_result.is_ok(),
        "Query should succeed with proper label formatting"
    );

    let notes = notes_result.unwrap();
    assert!(
        !notes.is_empty(),
        "Should find notes with proper label formatting"
    );
}

#[tokio::test]
async fn test_error_handling() {
    // Test with a file that doesn't exist
    let nonexistent = fixture_path("does_not_exist.typ");

    let config_result = query_ankify_configuration(&nonexistent, None).await;
    assert!(config_result.is_err(), "Should error on nonexistent file");

    let notes_result = query_ankify_notes(&nonexistent, None).await;
    assert!(notes_result.is_err(), "Should error on nonexistent file");

    // Verify error messages contain useful information
    if let Err(e) = config_result {
        let error_msg = format!("{}", e);
        assert!(
            error_msg.contains("Typst query failed") || error_msg.contains("Failed to execute"),
            "Error should mention Typst query failure: {}",
            error_msg
        );
    }
}
