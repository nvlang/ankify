//! Integration tests for the generate module.

use ankify::generate::{cleanup_temp_files, generate_temp_file, GenerateConfig};
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

/// Helper to get the path to a fixture file
fn fixture_path(filename: &str) -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("generate")
        .join(filename)
}

#[test]
fn test_generate_config_creation() {
    let config = GenerateConfig::new("test.typ");
    assert_eq!(config.source_file, PathBuf::from("test.typ"));
    assert_eq!(config.plugin_version, "0.1.0");
    assert!(config.output_dir.is_none());
}

#[test]
fn test_generate_config_with_plugin_version() {
    let config = GenerateConfig::new("test.typ").with_plugin_version("test");
    assert_eq!(config.plugin_version, "test");
}

#[test]
fn test_generate_config_with_output_dir() {
    let temp_dir = TempDir::new().unwrap();
    let config = GenerateConfig::new("test.typ").with_output_dir(temp_dir.path());
    assert_eq!(config.output_dir, Some(temp_dir.path().to_path_buf()));
}

#[test]
fn test_generate_temp_file_simple() {
    let source_file = fixture_path("simple.typ");
    let temp_dir = TempDir::new().unwrap();

    let config = GenerateConfig::new(&source_file)
        .with_plugin_version("test")
        .with_output_dir(temp_dir.path());

    let result = generate_temp_file(&config);
    assert!(
        result.is_ok(),
        "Failed to generate temp file: {:?}",
        result.err()
    );

    let temp_file = result.unwrap();
    assert!(temp_file.exists(), "Generated temp file should exist");
    assert!(temp_file
        .file_name()
        .unwrap()
        .to_str()
        .unwrap()
        .ends_with("_render.typ"));

    // Check that the content was generated
    let content = fs::read_to_string(&temp_file).unwrap();
    assert!(content.contains("#import"));
    assert!(content.contains("ankify-typst/lib.typ"));
    assert!(content.contains("as simple"));
    assert!(content.contains("#hide([#simple])"));
    assert!(content.contains("#set page(height: auto)"));
    assert!(content.contains("__ankify-configuration.final().setup"));
    assert!(content.contains("__ankify-notes.final()"));
}

#[test]
fn test_generate_temp_file_with_local_plugin() {
    let source_file = fixture_path("simple.typ");
    let temp_dir = TempDir::new().unwrap();

    let config = GenerateConfig::new(&source_file)
        .with_plugin_version("test")
        .with_output_dir(temp_dir.path());

    let result = generate_temp_file(&config);
    assert!(result.is_ok());

    let temp_file = result.unwrap();
    let content = fs::read_to_string(&temp_file).unwrap();

    // Should use relative path for test development
    assert!(content.contains("ankify-typst/lib.typ\": __ankify-configuration, __ankify-notes"));
    assert!(!content.contains("@preview/ankify"));
}

#[test]
fn test_generate_temp_file_with_preview_plugin() {
    let source_file = fixture_path("simple.typ");
    let temp_dir = TempDir::new().unwrap();

    let config = GenerateConfig::new(&source_file)
        .with_plugin_version("test")
        .with_output_dir(temp_dir.path());

    let result = generate_temp_file(&config);
    assert!(result.is_ok());

    let temp_file = result.unwrap();
    let content = fs::read_to_string(&temp_file).unwrap();

    // In tests, should still use relative path due to cfg!(test)
    assert!(content.contains("ankify-typst/lib.typ"));
    assert!(!content.contains("@preview/ankify"));
}

#[test]
fn test_generate_temp_file_default_output_dir() {
    let source_file = fixture_path("simple.typ");
    let config = GenerateConfig::new(&source_file);

    let result = generate_temp_file(&config);
    assert!(result.is_ok());

    let temp_file = result.unwrap();

    // Should be in .ankify directory next to source
    let expected_parent = source_file.parent().unwrap().join(".ankify");
    assert_eq!(temp_file.parent().unwrap(), expected_parent);

    // Clean up
    let _ = fs::remove_file(&temp_file);
    let _ = fs::remove_dir(&expected_parent);
}

#[test]
fn test_generate_temp_file_nonexistent_source() {
    let temp_dir = TempDir::new().unwrap();
    let nonexistent_file = temp_dir.path().join("nonexistent.typ");

    let config = GenerateConfig::new(&nonexistent_file).with_output_dir(temp_dir.path());

    let result = generate_temp_file(&config);
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("Source file does not exist"));
}

#[test]
fn test_generate_temp_file_invalid_source_name() {
    let temp_dir = TempDir::new().unwrap();

    // Create a file with an invalid name that will fail file_stem() extraction
    // Use ".." which will have None for file_stem()
    let invalid_file = temp_dir.path().join("..");

    let config = GenerateConfig::new(&invalid_file).with_output_dir(temp_dir.path());

    let result = generate_temp_file(&config);
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("Invalid source file name"));
}

#[test]
fn test_generate_temp_file_creates_output_directory() {
    let source_file = fixture_path("simple.typ");
    let temp_dir = TempDir::new().unwrap();
    let output_dir = temp_dir.path().join("nested").join("output");

    let config = GenerateConfig::new(&source_file).with_output_dir(&output_dir);

    let result = generate_temp_file(&config);
    assert!(result.is_ok());
    assert!(output_dir.exists(), "Output directory should be created");

    let temp_file = result.unwrap();
    assert!(temp_file.exists());
}

#[test]
fn test_generate_temp_file_multiple_notes() {
    let source_file = fixture_path("multiple_notes.typ");
    let temp_dir = TempDir::new().unwrap();

    let config = GenerateConfig::new(&source_file).with_output_dir(temp_dir.path());

    let result = generate_temp_file(&config);
    assert!(result.is_ok());

    let temp_file = result.unwrap();
    let content = fs::read_to_string(&temp_file).unwrap();

    // Should have correct source name
    assert!(content.contains("as multiple_notes"));
    assert!(content.contains("#hide([#multiple_notes])"));
}

#[test]
fn test_generate_temp_file_with_setup() {
    let source_file = fixture_path("with_setup.typ");
    let temp_dir = TempDir::new().unwrap();

    let config = GenerateConfig::new(&source_file).with_output_dir(temp_dir.path());

    let result = generate_temp_file(&config);
    assert!(result.is_ok());

    let temp_file = result.unwrap();
    let content = fs::read_to_string(&temp_file).unwrap();

    // Should contain setup function call
    assert!(content.contains("(__ankify-configuration.final().setup)()"));
}

#[test]
fn test_cleanup_temp_files() {
    let temp_dir = TempDir::new().unwrap();
    let temp_path = temp_dir.path();

    // Create various test files
    fs::write(temp_path.join("test_render.typ"), "temp content").unwrap();
    fs::write(temp_path.join("another_render.typ"), "temp content").unwrap();
    fs::write(temp_path.join("normal_file.typ"), "keep this").unwrap();
    fs::write(temp_path.join("document.typ"), "keep this too").unwrap();
    fs::write(temp_path.join("file_render.typ"), "temp content").unwrap();

    // Clean up temp files
    let result = cleanup_temp_files(temp_path);
    assert!(result.is_ok(), "Cleanup should succeed");

    // Check that only render files were removed
    assert!(!temp_path.join("test_render.typ").exists());
    assert!(!temp_path.join("another_render.typ").exists());
    assert!(!temp_path.join("file_render.typ").exists());
    assert!(temp_path.join("normal_file.typ").exists());
    assert!(temp_path.join("document.typ").exists());
}

#[test]
fn test_cleanup_temp_files_empty_directory() {
    let temp_dir = TempDir::new().unwrap();
    let result = cleanup_temp_files(temp_dir.path());
    assert!(result.is_ok(), "Cleanup of empty directory should succeed");
}

#[test]
fn test_cleanup_temp_files_nonexistent_directory() {
    let temp_dir = TempDir::new().unwrap();
    let nonexistent = temp_dir.path().join("nonexistent");

    let result = cleanup_temp_files(&nonexistent);
    assert!(
        result.is_ok(),
        "Cleanup of nonexistent directory should succeed"
    );
}

#[test]
fn test_cleanup_temp_files_no_render_files() {
    let temp_dir = TempDir::new().unwrap();
    let temp_path = temp_dir.path();

    // Create files that should not be removed
    fs::write(temp_path.join("document.typ"), "content").unwrap();
    fs::write(temp_path.join("styles.css"), "content").unwrap();
    fs::write(temp_path.join("render.txt"), "content").unwrap();

    let result = cleanup_temp_files(temp_path);
    assert!(result.is_ok());

    // All files should still exist
    assert!(temp_path.join("document.typ").exists());
    assert!(temp_path.join("styles.css").exists());
    assert!(temp_path.join("render.txt").exists());
}

#[test]
fn test_end_to_end_generate_and_cleanup() {
    let source_file = fixture_path("simple.typ");
    let temp_dir = TempDir::new().unwrap();

    let config = GenerateConfig::new(&source_file)
        .with_plugin_version("test")
        .with_output_dir(temp_dir.path());

    // Generate temp file
    let temp_file = generate_temp_file(&config).unwrap();
    assert!(temp_file.exists());

    // Verify content
    let content = fs::read_to_string(&temp_file).unwrap();
    assert!(content.contains("ankify-typst/lib.typ"));
    assert!(content.contains("as simple"));

    // Create additional temp file to test cleanup
    fs::write(temp_dir.path().join("extra_render.typ"), "extra").unwrap();

    // Cleanup
    cleanup_temp_files(temp_dir.path()).unwrap();

    // Both temp files should be gone
    assert!(!temp_file.exists());
    assert!(!temp_dir.path().join("extra_render.typ").exists());
}

#[test]
fn test_generate_with_complex_path() {
    let temp_dir = TempDir::new().unwrap();
    let nested_dir = temp_dir.path().join("nested").join("deep");
    fs::create_dir_all(&nested_dir).unwrap();

    let source_file = nested_dir.join("complex-name.typ");
    fs::write(&source_file,
        "#import \"../../../../ankify-typst/lib.typ\": note\n#note(\"test\", data: (Front: \"Q\", Back: \"A\"))"
    ).unwrap();

    let output_dir = temp_dir.path().join("output");
    let config = GenerateConfig::new(&source_file).with_output_dir(&output_dir);

    let result = generate_temp_file(&config);
    assert!(result.is_ok());

    let temp_file = result.unwrap();
    assert!(temp_file.exists());

    let content = fs::read_to_string(&temp_file).unwrap();
    assert!(content.contains("as complex-name"));
    assert!(content.contains("#hide([#complex-name])"));
}

#[test]
fn test_generate_preserves_file_stem() {
    let temp_dir = TempDir::new().unwrap();

    // Test various file name patterns
    let test_cases = vec![
        ("simple.typ", "simple"),
        ("my-document.typ", "my-document"),
        ("file_with_underscores.typ", "file_with_underscores"),
        ("123numbers.typ", "123numbers"),
    ];

    for (filename, expected_stem) in test_cases {
        let source_file = temp_dir.path().join(filename);
        fs::write(&source_file, "// test content").unwrap();

        let output_dir = temp_dir.path().join("output");
        let config = GenerateConfig::new(&source_file).with_output_dir(&output_dir);

        let temp_file = generate_temp_file(&config).unwrap();
        let content = fs::read_to_string(&temp_file).unwrap();

        assert!(content.contains(&format!("as {}", expected_stem)));
        assert!(content.contains(&format!("#hide([#{}])", expected_stem)));

        // Clean up for next iteration
        fs::remove_file(&temp_file).unwrap();
    }
}
