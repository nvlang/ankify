//! CLI integration tests that execute the actual binary
//! These tests cover the main.rs execution paths that are impossible to unit test

use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use tempfile::TempDir;

#[test]
fn test_cli_help() {
    let mut cmd = Command::cargo_bin("ankify").unwrap();
    cmd.arg("--help");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Advanced Typst to Anki bridge"));
}

#[test]
fn test_cli_version() {
    let mut cmd = Command::cargo_bin("ankify").unwrap();
    cmd.arg("--version");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("ankify"));
}

#[test]
fn test_cli_no_arguments() {
    let mut cmd = Command::cargo_bin("ankify").unwrap();

    // Should fail because no files provided
    cmd.assert()
        .failure()
        .stdout(predicate::str::contains("At least one Typst file"));
}

#[test]
fn test_cli_invalid_log_level() {
    let mut cmd = Command::cargo_bin("ankify").unwrap();
    cmd.args(&["test.typ", "--log-level", "invalid"]);

    // Should fail due to invalid log level - error occurs before logging is initialized
    // so no output is produced, but exit code should be non-zero
    cmd.assert().failure().code(1);
}

#[test]
fn test_cli_nonexistent_render_file() {
    let mut cmd = Command::cargo_bin("ankify").unwrap();
    cmd.args(&["test.typ", "--render", "/nonexistent/render.typ"]);

    // Should fail because render file doesn't exist
    cmd.assert()
        .failure()
        .stdout(predicate::str::contains("Render file does not exist"));
}

#[test]
fn test_cli_invalid_render_file_extension() {
    let temp_dir = TempDir::new().unwrap();
    let invalid_render = temp_dir.path().join("render.txt");
    fs::write(&invalid_render, "content").unwrap();

    let mut cmd = Command::cargo_bin("ankify").unwrap();
    cmd.args(&["test.typ", "--render", invalid_render.to_str().unwrap()]);

    // Should fail because render file doesn't have .typ extension
    cmd.assert()
        .failure()
        .stdout(predicate::str::contains("must have .typ extension"));
}

#[test]
fn test_cli_invalid_aux_directory() {
    let mut cmd = Command::cargo_bin("ankify").unwrap();
    cmd.args(&["test.typ", "--aux-file", "/nonexistent/dir/file.json"]);

    // Should fail because aux file directory doesn't exist
    cmd.assert().failure().stdout(predicate::str::contains(
        "Directory for auxiliary file does not exist",
    ));
}

#[test]
fn test_cli_valid_arguments_with_nonexistent_files() {
    let temp_dir = TempDir::new().unwrap();
    let aux_file = temp_dir.path().join("test.aux.json");

    let mut cmd = Command::cargo_bin("ankify").unwrap();
    cmd.args(&[
        "nonexistent.typ",
        "--aux-file",
        aux_file.to_str().unwrap(),
        "--log-level",
        "error", // Reduce log noise
    ]);

    // Should handle nonexistent files gracefully (success or controlled failure)
    let output = cmd.output().unwrap();

    // Either succeeds (graceful handling) or fails with a controlled error message
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        // Should not panic or crash - should have a reasonable error message
        assert!(!stderr.contains("panic"));
        assert!(!stderr.contains("thread 'main' panicked"));
    }
}

#[test]
fn test_cli_with_all_flags() {
    let temp_dir = TempDir::new().unwrap();
    let aux_file = temp_dir.path().join("full_test.aux.json");

    // Create a minimal typst file
    let typst_file = temp_dir.path().join("test.typ");
    fs::write(
        &typst_file,
        r#"
#import "ankify.typ": card
#card("test", data: (Front: "Q", Back: "A"))
"#,
    )
    .unwrap();

    // Create minimal ankify.typ
    let ankify_typ = temp_dir.path().join("ankify.typ");
    fs::write(
        &ankify_typ,
        r#"
#let card(label, ..args) = {}
#let configure(..args) = {}
"#,
    )
    .unwrap();

    let mut cmd = Command::cargo_bin("ankify").unwrap();
    cmd.current_dir(&temp_dir);
    cmd.args(&[
        "test.typ",
        "--aux-file",
        aux_file.to_str().unwrap(),
        "--verbose",
        "--bypass-cache",
        "--ankiconnect-url",
        "http://localhost:9999", // Invalid port
        "--log-level",
        "debug",
    ]);

    // Should handle all flags without crashing
    let output = cmd.output().unwrap();

    // Check that it ran without panicking
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!stderr.contains("panic"));
    assert!(!stderr.contains("thread 'main' panicked"));

    // May fail due to invalid AnkiConnect URL, but should fail gracefully
}
