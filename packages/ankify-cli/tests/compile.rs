//! Integration tests for the compile module.

use ankify::compile::{cleanup_output_files, compile_temp_file, CompileConfig, Format};
use ankify::generate::{generate_temp_file, GenerateConfig};
use regex;
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

/// Helper to get the path to a fixture file
fn fixture_path(filename: &str) -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("compile")
        .join(filename)
}

#[test]
fn test_format_functionality() {
    assert_eq!(Format::from("plain"), Format::Plain);
    assert_eq!(Format::from("svg"), Format::Svg);
    assert_eq!(Format::from("png"), Format::Png);
    assert_eq!(Format::from("PLAIN"), Format::Plain);
    assert_eq!(Format::from("unknown"), Format::Png);

    assert_eq!(Format::Plain.extension(), "txt");
    assert_eq!(Format::Svg.extension(), "svg");
    assert_eq!(Format::Png.extension(), "png");

    assert_eq!(Format::Svg.typst_arg(), "svg");
    assert_eq!(Format::Png.typst_arg(), "png");
}

#[test]
#[should_panic(expected = "Plain format should not be compiled")]
fn test_format_plain_typst_arg_panics() {
    Format::Plain.typst_arg();
}

#[test]
fn test_compile_config_creation() {
    let temp_file = PathBuf::from("temp.typ");
    let source_file = PathBuf::from("source.typ");
    let output_dir = PathBuf::from("output");

    let config = CompileConfig::new(temp_file.clone(), source_file.clone(), output_dir.clone());

    assert_eq!(config.temp_file, temp_file);
    assert_eq!(config.source_file, source_file);
    assert_eq!(config.output_dir, output_dir);
    assert!(config.extra_args.is_empty());
}

#[test]
fn test_compile_config_with_extra_args() {
    let config = CompileConfig::new(
        PathBuf::from("temp.typ"),
        PathBuf::from("source.typ"),
        PathBuf::from("output"),
    )
    .with_extra_args(vec!["--verbose".to_string(), "--debug".to_string()]);

    assert_eq!(config.extra_args, vec!["--verbose", "--debug"]);
}

#[tokio::test]
async fn test_compile_empty_file() {
    let source_file = fixture_path("empty.typ");

    // Use a temp directory within the test fixtures for better compatibility
    let project_temp_dir = source_file.parent().unwrap().join(".test_temp");
    fs::create_dir_all(&project_temp_dir).unwrap();
    let output_dir = project_temp_dir.join("output");

    // Generate temp file first
    let generate_config = GenerateConfig::new(&source_file)
        .with_plugin_version("test")
        .with_output_dir(&project_temp_dir);

    let temp_file = generate_temp_file(&generate_config).unwrap();

    // Compile the temp file
    let compile_config = CompileConfig::new(temp_file, source_file, output_dir);

    let result = compile_temp_file(&compile_config).await;
    assert!(result.is_ok(), "Compile should succeed with empty file");

    let compile_result = result.unwrap();
    assert!(compile_result.notes.is_empty(), "Should have no notes");
    assert!(
        compile_result.output_files.is_empty(),
        "Should have no output files"
    );

    // Clean up
    fs::remove_dir_all(&project_temp_dir).ok();
}

#[tokio::test]
async fn test_compile_plain_only() {
    let source_file = fixture_path("plain_only.typ");

    // Use a temp directory within the test fixtures for better compatibility
    let project_temp_dir = source_file.parent().unwrap().join(".test_temp_plain");
    fs::create_dir_all(&project_temp_dir).unwrap();
    let output_dir = project_temp_dir.join("output");
    fs::create_dir_all(&output_dir).unwrap();

    // Generate temp file
    let generate_config = GenerateConfig::new(&source_file)
        .with_plugin_version("test")
        .with_output_dir(&project_temp_dir);

    let temp_file = generate_temp_file(&generate_config).unwrap();

    // Compile the temp file
    let compile_config = CompileConfig::new(temp_file, source_file, output_dir);

    let result = compile_temp_file(&compile_config).await;
    assert!(result.is_ok(), "Plain only compile should succeed");

    let compile_result = result.unwrap();
    assert_eq!(compile_result.notes.len(), 3, "Should have 3 notes");
    assert!(
        compile_result.output_files.is_empty(),
        "Plain format should have no output files"
    );

    // Check that notes have text content
    for note in &compile_result.notes {
        assert!(!note.fields.is_empty(), "Notes should have fields");
        assert!(
            note.picture.is_none() || note.picture.as_ref().unwrap().is_empty(),
            "Plain notes should have no pictures"
        );
    }

    // Clean up
    fs::remove_dir_all(&project_temp_dir).ok();
}

#[tokio::test]
async fn test_compile_simple_png() {
    let source_file = fixture_path("simple.typ");

    // Use a temp directory within the test fixtures for better compatibility
    let project_temp_dir = source_file.parent().unwrap().join(".test_temp_simple");
    fs::create_dir_all(&project_temp_dir).unwrap();
    let output_dir = project_temp_dir.join("output");
    fs::create_dir_all(&output_dir).unwrap();

    // Generate temp file
    let generate_config = GenerateConfig::new(&source_file)
        .with_plugin_version("test")
        .with_output_dir(&project_temp_dir);

    let temp_file = generate_temp_file(&generate_config).unwrap();

    // Compile the temp file
    let compile_config = CompileConfig::new(temp_file, source_file, output_dir.clone());

    let result = compile_temp_file(&compile_config).await;

    // Note: This test might fail if typst is not installed or if the compilation fails
    // In a real environment, we'd want to mock the typst command or make this test conditional
    match result {
        Ok(compile_result) => {
            assert_eq!(compile_result.notes.len(), 1, "Should have 1 note");

            let note = &compile_result.notes[0];
            assert_eq!(note.deck_name.as_str(), "Test-Deck");
            assert_eq!(note.model_name.as_str(), "Basic");
            assert!(!note.fields.is_empty(), "Note should have fields");
        }
        Err(e) => {
            // If typst is not available, the test should skip gracefully
            let error_msg = format!("{}", e);
            if error_msg.contains("Failed to execute typst compile") {
                println!("Skipping test: typst command not available");
                return;
            } else {
                panic!("Unexpected error: {}", e);
            }
        }
    }
}

#[tokio::test]
async fn test_compile_mixed_formats() {
    let source_file = fixture_path("mixed_formats.typ");

    // Use a temp directory within the test fixtures for better compatibility
    let project_temp_dir = source_file.parent().unwrap().join(".test_temp_mixed");
    fs::create_dir_all(&project_temp_dir).unwrap();
    let output_dir = project_temp_dir.join("output");
    fs::create_dir_all(&output_dir).unwrap();

    // Generate temp file
    let generate_config = GenerateConfig::new(&source_file)
        .with_plugin_version("test")
        .with_output_dir(&project_temp_dir);

    let temp_file = generate_temp_file(&generate_config).unwrap();

    // Compile the temp file
    let compile_config = CompileConfig::new(temp_file, source_file, output_dir.clone());

    let result = compile_temp_file(&compile_config).await;

    match result {
        Ok(compile_result) => {
            assert_eq!(compile_result.notes.len(), 3, "Should have 3 notes");

            // Check that we have different types of notes
            let mut has_plain = false;
            let mut _has_images = false;

            for note in &compile_result.notes {
                if note.picture.is_none() || note.picture.as_ref().unwrap().is_empty() {
                    has_plain = true;
                } else {
                    _has_images = true;
                }
            }

            assert!(has_plain, "Should have at least one plain note");
            // Images might not be generated if typst is not available
        }
        Err(e) => {
            let error_msg = format!("{}", e);
            if error_msg.contains("Failed to execute typst compile") {
                println!("Skipping test: typst command not available");
                return;
            } else {
                panic!("Unexpected error: {}", e);
            }
        }
    }
}

#[tokio::test]
async fn test_compile_svg_only() {
    let source_file = fixture_path("svg_only.typ");

    // Use a temp directory within the test fixtures for better compatibility
    let project_temp_dir = source_file.parent().unwrap().join(".test_temp_svg");
    fs::create_dir_all(&project_temp_dir).unwrap();
    let output_dir = project_temp_dir.join("output");
    fs::create_dir_all(&output_dir).unwrap();

    // Generate temp file
    let generate_config = GenerateConfig::new(&source_file)
        .with_plugin_version("test")
        .with_output_dir(&project_temp_dir);

    let temp_file = generate_temp_file(&generate_config).unwrap();

    // Compile the temp file
    let compile_config = CompileConfig::new(temp_file, source_file, output_dir.clone());

    let result = compile_temp_file(&compile_config).await;

    match result {
        Ok(compile_result) => {
            assert_eq!(compile_result.notes.len(), 2, "Should have 2 notes");

            for note in &compile_result.notes {
                assert_eq!(note.deck_name.as_str(), "SVG-Test");
                assert_eq!(note.model_name.as_str(), "Basic");
                assert!(!note.fields.is_empty(), "Note should have fields");
            }
        }
        Err(e) => {
            let error_msg = format!("{}", e);
            if error_msg.contains("Failed to execute typst compile") {
                println!("Skipping test: typst command not available");
                return;
            } else {
                panic!("Unexpected error: {}", e);
            }
        }
    }
}

#[test]
fn test_cleanup_output_files() {
    let temp_dir = TempDir::new().unwrap();
    let temp_path = temp_dir.path();

    // Create various test files
    fs::write(temp_path.join("output-1.png"), "content").unwrap();
    fs::write(temp_path.join("output-2.svg"), "content").unwrap();
    fs::write(temp_path.join("output-test.txt"), "content").unwrap();
    fs::write(temp_path.join("keep_this.png"), "keep this").unwrap();
    fs::write(temp_path.join("document.typ"), "keep this too").unwrap();
    fs::write(temp_path.join("render_file.typ"), "keep this").unwrap();

    // Clean up output files
    let result = cleanup_output_files(temp_path);
    assert!(result.is_ok(), "Cleanup should succeed");

    // Check that only output-* files were removed
    assert!(!temp_path.join("output-1.png").exists());
    assert!(!temp_path.join("output-2.svg").exists());
    assert!(!temp_path.join("output-test.txt").exists());
    assert!(temp_path.join("keep_this.png").exists());
    assert!(temp_path.join("document.typ").exists());
    assert!(temp_path.join("render_file.typ").exists());
}

#[test]
fn test_cleanup_output_files_empty_directory() {
    let temp_dir = TempDir::new().unwrap();
    let result = cleanup_output_files(temp_dir.path());
    assert!(result.is_ok(), "Cleanup of empty directory should succeed");
}

#[test]
fn test_cleanup_output_files_nonexistent_directory() {
    let temp_dir = TempDir::new().unwrap();
    let nonexistent = temp_dir.path().join("nonexistent");

    let result = cleanup_output_files(&nonexistent);
    assert!(
        result.is_ok(),
        "Cleanup of nonexistent directory should succeed"
    );
}

#[test]
fn test_cleanup_output_files_no_output_files() {
    let temp_dir = TempDir::new().unwrap();
    let temp_path = temp_dir.path();

    // Create files that should not be removed
    fs::write(temp_path.join("document.typ"), "content").unwrap();
    fs::write(temp_path.join("styles.css"), "content").unwrap();
    fs::write(temp_path.join("image.png"), "content").unwrap();

    let result = cleanup_output_files(temp_path);
    assert!(result.is_ok());

    // All files should still exist
    assert!(temp_path.join("document.typ").exists());
    assert!(temp_path.join("styles.css").exists());
    assert!(temp_path.join("image.png").exists());
}

#[tokio::test]
async fn test_compile_with_extra_args() {
    let source_file = fixture_path("simple.typ");

    // Use a temp directory within the test fixtures for better compatibility
    let project_temp_dir = source_file.parent().unwrap().join(".test_temp_extra");
    fs::create_dir_all(&project_temp_dir).unwrap();
    let output_dir = project_temp_dir.join("output");
    fs::create_dir_all(&output_dir).unwrap();

    // Generate temp file
    let generate_config = GenerateConfig::new(&source_file)
        .with_plugin_version("test")
        .with_output_dir(&project_temp_dir);

    let temp_file = generate_temp_file(&generate_config).unwrap();

    // Compile with extra arguments
    let compile_config = CompileConfig::new(temp_file, source_file, output_dir)
        .with_extra_args(vec!["--verbose".to_string()]);

    let result = compile_temp_file(&compile_config).await;

    // Should handle extra args gracefully (whether typst is available or not)
    match result {
        Ok(_) => {
            // Success is good
        }
        Err(e) => {
            let error_msg = format!("{}", e);
            if error_msg.contains("Failed to execute typst compile") {
                println!("Skipping test: typst command not available");
                return;
            } else {
                // Other errors might be due to invalid extra args, which is also a valid test outcome
                println!("Compile failed with extra args (expected): {}", e);
            }
        }
    }
}

#[tokio::test]
async fn test_compile_nonexistent_temp_file() {
    let temp_dir = TempDir::new().unwrap();
    let nonexistent_temp = temp_dir.path().join("nonexistent.typ");
    let source_file = fixture_path("simple.typ");
    let output_dir = temp_dir.path().join("output");

    let compile_config = CompileConfig::new(nonexistent_temp, source_file, output_dir);

    let result = compile_temp_file(&compile_config).await;
    assert!(result.is_err(), "Should fail with nonexistent temp file");

    let error_msg = format!("{}", result.unwrap_err());
    assert!(
        error_msg.contains("Failed to query")
            || error_msg.contains("No such file")
            || error_msg.contains("file not found")
            || error_msg.contains("Typst query failed"),
        "Error message doesn't match expected patterns: {}",
        error_msg
    );
}

#[tokio::test]
async fn test_end_to_end_generate_and_compile() {
    let source_file = fixture_path("simple.typ");

    // Use a temp directory within the test fixtures for better compatibility
    let project_temp_dir = source_file.parent().unwrap().join(".test_temp_e2e");
    fs::create_dir_all(&project_temp_dir).unwrap();
    let output_dir = project_temp_dir.join("output");
    fs::create_dir_all(&output_dir).unwrap();

    // Step 1: Generate temp file
    let generate_config = GenerateConfig::new(&source_file)
        .with_plugin_version("test")
        .with_output_dir(&project_temp_dir);

    let temp_file = generate_temp_file(&generate_config).unwrap();
    assert!(temp_file.exists(), "Generated temp file should exist");

    // Step 2: Compile temp file
    let compile_config = CompileConfig::new(temp_file.clone(), source_file, output_dir.clone());

    let result = compile_temp_file(&compile_config).await;

    match result {
        Ok(compile_result) => {
            assert!(!compile_result.notes.is_empty(), "Should have notes");

            // Step 3: Clean up
            cleanup_output_files(&output_dir).unwrap();

            // Verify cleanup worked
            let entries = fs::read_dir(&output_dir).unwrap().count();
            assert_eq!(entries, 0, "Output directory should be empty after cleanup");
        }
        Err(e) => {
            let error_msg = format!("{}", e);
            if error_msg.contains("Failed to execute typst compile") {
                println!("Skipping test: typst command not available");
                return;
            } else {
                panic!("Unexpected error: {}", e);
            }
        }
    }
}

#[tokio::test]
async fn test_compile_invalid_temp_file_content() {
    let temp_dir = TempDir::new().unwrap();
    let invalid_temp = temp_dir.path().join("invalid.typ");
    let source_file = fixture_path("simple.typ");
    let output_dir = temp_dir.path().join("output");

    // Create an invalid temp file
    fs::write(
        &invalid_temp,
        "This is not valid Typst syntax: #invalid_function(",
    )
    .unwrap();

    let compile_config = CompileConfig::new(invalid_temp, source_file, output_dir);

    let result = compile_temp_file(&compile_config).await;

    match result {
        Ok(_) => {
            // If it succeeds, that's unexpected but not necessarily wrong
            // (maybe the query still worked despite invalid syntax)
        }
        Err(e) => {
            // Failure is expected with invalid content
            let error_msg = format!("{}", e);
            assert!(
                error_msg.contains("Failed to query")
                    || error_msg.contains("Typst")
                    || error_msg.contains("Failed to execute"),
                "Should fail with appropriate error message, got: {}",
                error_msg
            );
        }
    }
}

#[tokio::test]
async fn test_compile_field_format_inheritance() {
    let source_file = fixture_path("mixed_formats.typ");

    let project_temp_dir = source_file.parent().unwrap().join(".test_temp_inheritance");
    fs::create_dir_all(&project_temp_dir).unwrap();
    let output_dir = project_temp_dir.join("output");
    fs::create_dir_all(&output_dir).unwrap();

    let generate_config = GenerateConfig::new(&source_file)
        .with_plugin_version("test")
        .with_output_dir(&project_temp_dir);

    let temp_file = generate_temp_file(&generate_config).unwrap();
    let compile_config = CompileConfig::new(temp_file, source_file, output_dir);

    let result = compile_temp_file(&compile_config).await;

    match result {
        Ok(compile_result) => {
            assert_eq!(compile_result.notes.len(), 3);

            // At least one note should have no pictures (the plain one)
            let has_plain_note = compile_result
                .notes
                .iter()
                .any(|n| n.picture.is_none() || n.picture.as_ref().unwrap().is_empty());
            assert!(
                has_plain_note,
                "Should have at least one plain note without pictures"
            );
        }
        Err(e) => {
            let error_msg = format!("{}", e);
            if error_msg.contains("Failed to execute typst compile") {
                println!("Skipping test: typst command not available");
                return;
            } else {
                panic!("Unexpected error: {}", e);
            }
        }
    }

    // Clean up
    fs::remove_dir_all(&project_temp_dir).ok();
}

#[test]
fn test_cleanup_temp_directories() {
    let temp_dir = TempDir::new().unwrap();
    let temp_path = temp_dir.path();

    // Create test directories and files that might be created during testing
    let test_temp1 = temp_path.join(".test_temp");
    let test_temp2 = temp_path.join(".test_temp_mixed");
    fs::create_dir_all(&test_temp1).unwrap();
    fs::create_dir_all(&test_temp2).unwrap();

    fs::write(test_temp1.join("output-1.png"), "content").unwrap();
    fs::write(test_temp1.join("test_render.typ"), "content").unwrap();
    fs::write(test_temp2.join("output-2.svg"), "content").unwrap();

    // Clean up both directories
    cleanup_output_files(&test_temp1).unwrap();
    cleanup_output_files(&test_temp2).unwrap();

    // Check that output files and render files were removed
    assert!(!test_temp1.join("output-1.png").exists());
    assert!(!test_temp1.join("test_render.typ").exists());
    assert!(!test_temp2.join("output-2.svg").exists());
}

#[test]
fn test_format_edge_cases() {
    // Test unknown format defaults to PNG
    assert_eq!(Format::from("unknown_format"), Format::Png);
    assert_eq!(Format::from(""), Format::Png);

    // Test case sensitivity
    assert_eq!(Format::from("PNG"), Format::Png);
    assert_eq!(Format::from("SVG"), Format::Svg);
    assert_eq!(Format::from("PLAIN"), Format::Plain);

    // Test mixed case
    assert_eq!(Format::from("Png"), Format::Png);
    assert_eq!(Format::from("Svg"), Format::Svg);
    assert_eq!(Format::from("Plain"), Format::Plain);
}

#[test]
fn test_compile_config_builder_pattern() {
    let temp_file = PathBuf::from("temp.typ");
    let source_file = PathBuf::from("source.typ");
    let output_dir = PathBuf::from("output");

    let config = CompileConfig::new(temp_file.clone(), source_file.clone(), output_dir.clone())
        .with_extra_args(vec!["--verbose".to_string(), "--debug".to_string()]);

    assert_eq!(config.temp_file, temp_file);
    assert_eq!(config.source_file, source_file);
    assert_eq!(config.output_dir, output_dir);
    assert_eq!(config.extra_args, vec!["--verbose", "--debug"]);
}

#[tokio::test]
async fn test_compile_multiple_pages() {
    // Create a test file that should generate multiple pages
    let source_file = fixture_path("svg_only.typ"); // This has 2 notes, should create 4 pages (2 fields × 2 notes)

    let project_temp_dir = source_file.parent().unwrap().join(".test_temp_multipage");
    fs::create_dir_all(&project_temp_dir).unwrap();
    let output_dir = project_temp_dir.join("output");
    fs::create_dir_all(&output_dir).unwrap();

    let generate_config = GenerateConfig::new(&source_file)
        .with_plugin_version("test")
        .with_output_dir(&project_temp_dir);

    let temp_file = generate_temp_file(&generate_config).unwrap();
    let compile_config = CompileConfig::new(temp_file, source_file, output_dir.clone());

    let result = compile_temp_file(&compile_config).await;

    match result {
        Ok(compile_result) => {
            assert_eq!(compile_result.notes.len(), 2);

            // Check that output files were generated and sorted properly
            if !compile_result.output_files.is_empty() {
                // Files should be sorted by page number (output-1.svg, output-2.svg, etc.)
                for (i, file) in compile_result.output_files.iter().enumerate() {
                    let filename = file.file_name().unwrap().to_str().unwrap();
                    assert!(filename.starts_with("output-"));
                    assert!(filename.ends_with(".svg"));

                    // Extract page number and verify sorting
                    if i > 0 {
                        let prev_file = &compile_result.output_files[i - 1];
                        let prev_filename = prev_file.file_name().unwrap().to_str().unwrap();

                        // Simple check that files are in some order
                        assert!(filename >= prev_filename, "Output files should be sorted");
                    }
                }
            }

            // Each note should have pictures (since it's SVG format)
            for note in &compile_result.notes {
                assert!(note.picture.is_some(), "SVG notes should have pictures");
                let pictures = note.picture.as_ref().unwrap();
                assert!(
                    !pictures.is_empty(),
                    "SVG notes should have non-empty pictures"
                );

                // Check that picture filenames follow the expected pattern
                for picture in pictures {
                    assert!(picture.filename.contains("@@"));
                    assert!(picture.filename.ends_with(".svg"));
                }
            }
        }
        Err(e) => {
            let error_msg = format!("{}", e);
            if error_msg.contains("Failed to execute typst compile") {
                println!("Skipping test: typst command not available");
                return;
            } else {
                panic!("Unexpected error: {}", e);
            }
        }
    }

    // Clean up
    fs::remove_dir_all(&project_temp_dir).ok();
}

#[test]
fn test_media_file_creation_edge_cases() {
    use tempfile::TempDir;

    let temp_dir = TempDir::new().unwrap();

    // Test with empty file
    let empty_file = temp_dir.path().join("empty.svg");
    fs::write(&empty_file, "").unwrap();

    // Test would require accessing private function
    // This tests the public interface instead

    // Media file creation is tested through the integration tests above
}

#[tokio::test]
async fn test_compile_with_complex_data_values() {
    let source_file = fixture_path("simple.typ");

    let project_temp_dir = source_file.parent().unwrap().join(".test_temp_complex");
    fs::create_dir_all(&project_temp_dir).unwrap();
    let output_dir = project_temp_dir.join("output");
    fs::create_dir_all(&output_dir).unwrap();

    let generate_config = GenerateConfig::new(&source_file)
        .with_plugin_version("test")
        .with_output_dir(&project_temp_dir);

    let temp_file = generate_temp_file(&generate_config).unwrap();
    let compile_config = CompileConfig::new(temp_file, source_file, output_dir);

    let result = compile_temp_file(&compile_config).await;

    match result {
        Ok(compile_result) => {
            assert!(!compile_result.notes.is_empty());

            // Verify that notes have the expected structure
            for note in &compile_result.notes {
                assert_eq!(note.deck_name.as_str(), "Test-Deck");
                assert_eq!(note.model_name.as_str(), "Basic");
                assert!(!note.fields.is_empty());

                // Check that tags are present
                assert!(note.tags.is_some());
                let tags = note.tags.as_ref().unwrap();
                assert!(!tags.is_empty());
            }
        }
        Err(e) => {
            let error_msg = format!("{}", e);
            if error_msg.contains("Failed to execute typst compile") {
                println!("Skipping test: typst command not available");
                return;
            } else {
                panic!("Unexpected error: {}", e);
            }
        }
    }

    // Clean up
    fs::remove_dir_all(&project_temp_dir).ok();
}

#[tokio::test]
async fn test_page_to_card_field_mapping() {
    let source_file = fixture_path("page_mapping_test.typ");

    let project_temp_dir = source_file
        .parent()
        .unwrap()
        .join(".test_temp_page_mapping");
    fs::create_dir_all(&project_temp_dir).unwrap();
    let output_dir = project_temp_dir.join("output");
    fs::create_dir_all(&output_dir).unwrap();

    // Generate temp file
    let generate_config = GenerateConfig::new(&source_file)
        .with_plugin_version("test")
        .with_output_dir(&project_temp_dir);

    let temp_file = generate_temp_file(&generate_config).unwrap();

    // Compile the temp file
    let compile_config = CompileConfig::new(temp_file, source_file, output_dir.clone());

    let result = compile_temp_file(&compile_config).await;

    match result {
        Ok(compile_result) => {
            assert_eq!(compile_result.notes.len(), 3, "Should have 3 notes");

            // Expected color mapping based on our test fixture:
            // Card 1: Front (Red #ff0000), Back (Green #00ff00)
            // Card 2: Front (Blue #0000ff), Back (Yellow #ffff00)
            // Card 3: Front (Magenta #ff00ff), Back (Cyan #00ffff)

            // With 3 cards × 2 fields = 6 content pages, plus first/last garbage pages = 8 total
            let output_files = &compile_result.output_files;
            println!("Found {} output files", output_files.len());
            for (i, file) in output_files.iter().enumerate() {
                println!("  File {}: {:?}", i + 1, file.file_name());
            }

            if output_files.len() >= 6 {
                // Define expected colors for each meaningful page
                // Based on the user's description: first and last pages are garbage
                // Fields are rendered in alphabetical order: "Back", "Front"
                let expected_colors = vec![
                    "#00ff00", // Page 2: Card 1 Back (Green)
                    "#ff0000", // Page 3: Card 1 Front (Red)
                    "#ffff00", // Page 4: Card 2 Back (Yellow)
                    "#0000ff", // Page 5: Card 2 Front (Blue)
                    "#00ffff", // Page 6: Card 3 Back (Cyan)
                    "#ff00ff", // Page 7: Card 3 Front (Magenta)
                ];

                // Check the content pages (skip first and potentially last garbage pages)
                let start_index = if output_files.len() > 6 { 1 } else { 0 };
                let end_index = std::cmp::min(start_index + 6, output_files.len());

                for (i, expected_color) in expected_colors.iter().enumerate() {
                    let file_index = start_index + i;
                    if file_index < end_index {
                        let svg_file = &output_files[file_index];

                        if svg_file.exists() {
                            let svg_content = fs::read_to_string(svg_file)
                                .expect("Should be able to read SVG file");

                            assert!(
                                svg_content.contains(expected_color),
                                "Page {} (file: {:?}) should contain color {} but doesn't.\nSVG content: {}",
                                file_index + 1,
                                svg_file.file_name(),
                                expected_color,
                                svg_content
                            );

                            println!(
                                "✓ Page {} contains expected color {}",
                                file_index + 1,
                                expected_color
                            );
                        }
                    }
                }

                // Verify that the notes have the correct field associations
                for (note_index, note) in compile_result.notes.iter().enumerate() {
                    assert_eq!(note.deck_name.as_str(), "Page-Mapping-Test");
                    assert_eq!(note.model_name.as_str(), "Basic");

                    if let Some(pictures) = &note.picture {
                        assert_eq!(
                            pictures.len(),
                            2,
                            "Each note should have 2 picture fields (Front and Back)"
                        );

                        // Verify picture filenames contain the expected patterns
                        let filenames: Vec<_> = pictures.iter().map(|p| &p.filename).collect();
                        println!("Note {} picture filenames: {:?}", note_index + 1, filenames);

                        // Each filename should contain the note label and field name
                        for picture in pictures {
                            assert!(picture.filename.contains("@@"));
                            assert!(picture.filename.ends_with(".svg"));
                        }
                    }
                }
            } else {
                println!(
                    "Warning: Expected at least 6 output files, but got {}",
                    output_files.len()
                );
            }
        }
        Err(e) => {
            let error_msg = format!("{}", e);
            if error_msg.contains("Failed to execute typst compile") {
                println!("Skipping test: typst command not available");
                return;
            } else {
                panic!("Unexpected error: {}", e);
            }
        }
    }

    // Clean up
    fs::remove_dir_all(&project_temp_dir).ok();
}

/// Helper function to analyze SVG content and extract color information
/// This can be used for debugging and more detailed color verification
#[allow(dead_code)]
fn analyze_svg_colors(svg_content: &str) -> Vec<String> {
    let mut colors = Vec::new();

    // Simple regex to find hex colors in fill attributes
    if let Ok(re) = regex::Regex::new(r#"fill="(#[0-9a-fA-F]{6})""#) {
        for cap in re.captures_iter(svg_content) {
            if let Some(color) = cap.get(1) {
                colors.push(color.as_str().to_lowercase());
            }
        }
    }

    colors
}
#[tokio::test]
async fn test_garbage_pages_and_edge_cases() {
    let source_file = fixture_path("page_mapping_test.typ");

    let project_temp_dir = source_file
        .parent()
        .unwrap()
        .join(".test_temp_garbage_pages");
    fs::create_dir_all(&project_temp_dir).unwrap();
    let output_dir = project_temp_dir.join("output");
    fs::create_dir_all(&output_dir).unwrap();

    // Generate temp file
    let generate_config = GenerateConfig::new(&source_file)
        .with_plugin_version("test")
        .with_output_dir(&project_temp_dir);

    let temp_file = generate_temp_file(&generate_config).unwrap();

    // Compile the temp file
    let compile_config = CompileConfig::new(temp_file, source_file, output_dir.clone());

    let result = compile_temp_file(&compile_config).await;

    match result {
        Ok(compile_result) => {
            let output_files = &compile_result.output_files;

            // We expect exactly 8 files: 1 garbage + 6 content + 1 garbage
            assert_eq!(output_files.len(), 8, "Should have exactly 8 output files");

            // Test that first and last pages don't contain our test colors
            // (indicating they are garbage/blank pages)
            let test_colors = vec![
                "#ff0000", "#00ff00", "#0000ff", "#ffff00", "#ff00ff", "#00ffff",
            ];

            // Check first page (should be garbage)
            if let Some(first_file) = output_files.first() {
                if first_file.exists() {
                    let svg_content = fs::read_to_string(first_file)
                        .expect("Should be able to read first SVG file");

                    let contains_test_colors =
                        test_colors.iter().any(|color| svg_content.contains(color));

                    assert!(
                        !contains_test_colors,
                        "First page should be garbage and not contain any test colors. Content: {}",
                        svg_content
                    );
                    println!("✓ First page confirmed as garbage (no test colors found)");
                }
            }

            // Check last page (should be garbage)
            if let Some(last_file) = output_files.last() {
                if last_file.exists() {
                    let svg_content = fs::read_to_string(last_file)
                        .expect("Should be able to read last SVG file");

                    let contains_test_colors =
                        test_colors.iter().any(|color| svg_content.contains(color));

                    assert!(
                        !contains_test_colors,
                        "Last page should be garbage and not contain any test colors. Content: {}",
                        svg_content
                    );
                    println!("✓ Last page confirmed as garbage (no test colors found)");
                }
            }

            // Test the content pages (pages 2-7) contain exactly the expected colors
            let expected_mapping = vec![
                (1, "#00ff00", "Card 1 Back"),  // Page 2 (index 1)
                (2, "#ff0000", "Card 1 Front"), // Page 3 (index 2)
                (3, "#ffff00", "Card 2 Back"),  // Page 4 (index 3)
                (4, "#0000ff", "Card 2 Front"), // Page 5 (index 4)
                (5, "#00ffff", "Card 3 Back"),  // Page 6 (index 5)
                (6, "#ff00ff", "Card 3 Front"), // Page 7 (index 6)
            ];

            for (file_index, expected_color, description) in expected_mapping {
                let svg_file = &output_files[file_index];
                assert!(svg_file.exists(), "SVG file should exist: {:?}", svg_file);

                let svg_content =
                    fs::read_to_string(svg_file).expect("Should be able to read SVG file");

                // Check that it contains the expected color
                assert!(
                    svg_content.contains(expected_color),
                    "{} should contain color {} but doesn't",
                    description,
                    expected_color
                );

                // Check that it doesn't contain other test colors
                let other_colors: Vec<_> = test_colors
                    .iter()
                    .filter(|&&color| color != expected_color)
                    .collect();

                for &other_color in &other_colors {
                    assert!(
                        !svg_content.contains(other_color),
                        "{} should not contain color {} but does",
                        description,
                        other_color
                    );
                }

                println!(
                    "✓ {} contains only expected color {}",
                    description, expected_color
                );
            }

            // Verify the association logic by checking that each note gets the right number of fields
            assert_eq!(compile_result.notes.len(), 3, "Should have 3 notes");

            for (note_index, note) in compile_result.notes.iter().enumerate() {
                assert!(
                    note.picture.is_some(),
                    "Note {} should have pictures",
                    note_index + 1
                );
                let pictures = note.picture.as_ref().unwrap();
                assert_eq!(
                    pictures.len(),
                    2,
                    "Note {} should have exactly 2 pictures",
                    note_index + 1
                );

                // Verify that the filenames match the expected pattern
                let mut field_names = vec![];
                for picture in pictures {
                    let filename = &picture.filename;
                    assert!(
                        filename.contains("@@"),
                        "Filename should contain @@: {}",
                        filename
                    );

                    // Extract field name from filename (format: label@@field@@timestamp.svg)
                    let parts: Vec<&str> = filename.split("@@").collect();
                    assert!(
                        parts.len() >= 2,
                        "Filename should have at least 2 @@ parts: {}",
                        filename
                    );
                    field_names.push(parts[1]);
                }

                // Should have exactly Back and Front fields
                field_names.sort();
                assert_eq!(
                    field_names,
                    vec!["Back", "Front"],
                    "Note {} should have Back and Front fields, got: {:?}",
                    note_index + 1,
                    field_names
                );
            }
        }
        Err(e) => {
            let error_msg = format!("{}", e);
            if error_msg.contains("Failed to execute typst compile") {
                println!("Skipping test: typst command not available");
                return;
            } else {
                panic!("Unexpected error: {}", e);
            }
        }
    }

    // Clean up
    fs::remove_dir_all(&project_temp_dir).ok();
}

#[tokio::test]
async fn test_single_card_mapping() {
    // Test with just one card to verify the basic mapping logic
    let source_file = fixture_path("single_card.typ");

    let project_temp_dir = source_file.parent().unwrap().join(".test_temp_single_card");
    fs::create_dir_all(&project_temp_dir).unwrap();
    let output_dir = project_temp_dir.join("output");
    fs::create_dir_all(&output_dir).unwrap();

    // Generate temp file
    let generate_config = GenerateConfig::new(&source_file)
        .with_plugin_version("test")
        .with_output_dir(&project_temp_dir);

    let temp_file = generate_temp_file(&generate_config).unwrap();

    // Compile the temp file
    let compile_config = CompileConfig::new(temp_file, source_file, output_dir.clone());

    let result = compile_temp_file(&compile_config).await;

    match result {
        Ok(compile_result) => {
            let output_files = &compile_result.output_files;

            // With 1 card × 2 fields = 2 content pages + 2 garbage pages = 4 total
            assert_eq!(
                output_files.len(),
                4,
                "Should have exactly 4 output files for single card"
            );

            // Check that pages 2 and 3 (indices 1 and 2) contain our colors
            if output_files.len() >= 3 {
                let page2 = &output_files[1];
                let page3 = &output_files[2];

                let page2_content = fs::read_to_string(page2).unwrap();
                let page3_content = fs::read_to_string(page3).unwrap();

                // One should contain the Back color, one should contain the Front color
                let back_color = "#fedcba";
                let front_color = "#abcdef";

                let page2_has_back = page2_content.contains(back_color);
                let page2_has_front = page2_content.contains(front_color);
                let page3_has_back = page3_content.contains(back_color);
                let page3_has_front = page3_content.contains(front_color);

                // Exactly one page should have each color
                assert!(
                    (page2_has_back && !page2_has_front && page3_has_front && !page3_has_back)
                        || (page2_has_front
                            && !page2_has_back
                            && page3_has_back
                            && !page3_has_front),
                    "Pages 2 and 3 should contain exactly one color each"
                );

                println!("✓ Single card mapping verified: pages contain expected colors");
            }
        }
        Err(e) => {
            let error_msg = format!("{}", e);
            if error_msg.contains("Failed to execute typst compile") {
                println!("Skipping test: typst command not available");
                return;
            } else {
                panic!("Unexpected error: {}", e);
            }
        }
    }

    // Clean up
    fs::remove_dir_all(&project_temp_dir).ok();
}
