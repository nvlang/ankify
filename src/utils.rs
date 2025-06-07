//! Utility functions and helper types
//!
//! This module provides common utilities used throughout the Ankify application,
//! including file operations, hashing, path manipulation, and validation functions.

use crate::error::{Error, Result};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use tokio::fs;
use tracing::{debug, instrument};

/// Calculate SHA-256 hash of file content
///
/// # Arguments
///
/// * `path` - Path to the file to hash
///
/// # Returns
///
/// Returns a hex-encoded SHA-256 hash of the file content.
///
/// # Errors
///
/// Returns an error if the file cannot be read or if I/O operations fail.
///
/// # Examples
///
/// ```rust,no_run
/// use ankify::utils::hash_file;
///
/// # async fn example() -> ankify::Result<()> {
/// let hash = hash_file("document.typ").await?;
/// println!("File hash: {}", hash);
/// # Ok(())
/// # }
/// ```
#[instrument]
pub async fn hash_file(path: impl AsRef<Path> + std::fmt::Debug) -> Result<String> {
    let path = path.as_ref();
    debug!(?path, "Calculating file hash");

    let content = fs::read(path).await?;
    let hash = Sha256::digest(&content);
    Ok(format!("{:x}", hash))
}

/// Calculate SHA-256 hash of string content
///
/// # Arguments
///
/// * `content` - Content to hash
///
/// # Returns
///
/// Returns a hex-encoded SHA-256 hash of the content.
///
/// # Examples
///
/// ```rust
/// use ankify::utils::hash_string;
///
/// let hash = hash_string("Hello, world!");
/// assert_eq!(hash.len(), 64); // SHA-256 produces 64-character hex strings
/// ```
pub fn hash_string(content: &str) -> String {
    let hash = Sha256::digest(content.as_bytes());
    format!("{:x}", hash)
}

/// Ensure a directory exists, creating it if necessary
///
/// # Arguments
///
/// * `path` - Path to the directory
///
/// # Errors
///
/// Returns an error if the directory cannot be created or if permissions are insufficient.
///
/// # Examples
///
/// ```rust,no_run
/// use ankify::utils::ensure_dir;
///
/// # async fn example() -> ankify::Result<()> {
/// ensure_dir("./cache/cards").await?;
/// # Ok(())
/// # }
/// ```
#[instrument]
pub async fn ensure_dir(path: impl AsRef<Path> + std::fmt::Debug) -> Result<()> {
    let path = path.as_ref();
    if !path.exists() {
        debug!(?path, "Creating directory");
        fs::create_dir_all(path).await?;
    }
    Ok(())
}

/// Find files matching a pattern in a directory
///
/// # Arguments
///
/// * `dir` - Directory to search in
/// * `pattern` - Glob pattern to match (e.g., "*.typ")
/// * `recursive` - Whether to search recursively
///
/// # Returns
///
/// Returns a vector of paths matching the pattern.
///
/// # Errors
///
/// Returns an error if the directory cannot be read or if the pattern is invalid.
///
/// # Examples
///
/// ```rust,no_run
/// use ankify::utils::find_files;
///
/// # async fn example() -> ankify::Result<()> {
/// let typst_files = find_files("./documents", "*.typ", true).await?;
/// println!("Found {} Typst files", typst_files.len());
/// # Ok(())
/// # }
/// ```
#[instrument]
pub async fn find_files(
    dir: impl AsRef<Path> + std::fmt::Debug,
    pattern: &str,
    recursive: bool,
) -> Result<Vec<PathBuf>> {
    let dir = dir.as_ref();
    debug!(?dir, pattern, recursive, "Finding files");

    let glob_pattern = if recursive {
        format!("{}/**/{}", dir.display(), pattern)
    } else {
        format!("{}/{}", dir.display(), pattern)
    };

    let paths = glob::glob(&glob_pattern)
        .map_err(|e| Error::InvalidInput(format!("Invalid glob pattern: {}", e)))?
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(|e| Error::FileSystem(format!("Glob error: {}", e)))?;

    Ok(paths)
}

/// Validate that a file has a specific extension
///
/// # Arguments
///
/// * `path` - Path to validate
/// * `expected_ext` - Expected file extension (without the dot)
///
/// # Returns
///
/// Returns `true` if the file has the expected extension.
///
/// # Examples
///
/// ```rust
/// use ankify::utils::has_extension;
/// use std::path::Path;
///
/// assert!(has_extension(Path::new("document.typ"), "typ"));
/// assert!(!has_extension(Path::new("document.txt"), "typ"));
/// ```
pub fn has_extension(path: &Path, expected_ext: &str) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.eq_ignore_ascii_case(expected_ext))
        .unwrap_or(false)
}

/// Generate a unique identifier for a card
///
/// Creates a unique identifier based on card content and metadata.
/// This ensures that cards can be reliably identified across updates.
///
/// # Arguments
///
/// * `deck` - Deck name
/// * `front` - Front content
/// * `back` - Back content
///
/// # Returns
///
/// Returns a unique string identifier for the card.
///
/// # Examples
///
/// ```rust
/// use ankify::utils::generate_card_id;
///
/// let id = generate_card_id("Math", "What is 2+2?", "4");
/// assert!(!id.is_empty());
/// ```
pub fn generate_card_id(deck: &str, front: &str, back: &str) -> String {
    let combined = format!("{}:{}:{}", deck, front, back);
    let hash = hash_string(&combined);
    format!("card_{}", &hash[..12]) // Use first 12 characters for readability
}

/// Validate a deck name
///
/// Ensures that deck names conform to Anki's requirements and best practices.
///
/// # Arguments
///
/// * `deck_name` - Name to validate
///
/// # Returns
///
/// Returns `true` if the deck name is valid.
///
/// # Examples
///
/// ```rust
/// use ankify::utils::is_valid_deck_name;
///
/// assert!(is_valid_deck_name("Math"));
/// assert!(is_valid_deck_name("Languages::French"));
/// assert!(!is_valid_deck_name(""));
/// assert!(!is_valid_deck_name("Deck/With/Slashes"));
/// ```
pub fn is_valid_deck_name(deck_name: &str) -> bool {
    if deck_name.is_empty() || deck_name.len() > 100 {
        return false;
    }

    // Check for invalid characters
    let invalid_chars = ['/', '<', '>', '"', '|', '?', '*'];
    if deck_name.chars().any(|c| invalid_chars.contains(&c)) {
        return false;
    }

    // Cannot start or end with whitespace
    if deck_name.starts_with(' ') || deck_name.ends_with(' ') {
        return false;
    }

    true
}

/// Sanitize HTML content for Anki
///
/// Ensures that HTML content is safe to use in Anki cards by escaping
/// potentially dangerous elements while preserving formatting.
///
/// # Arguments
///
/// * `html` - HTML content to sanitize
///
/// # Returns
///
/// Returns sanitized HTML content.
///
/// # Examples
///
/// ```rust
/// use ankify::utils::sanitize_html;
///
/// let safe = sanitize_html("<b>Bold</b> text");
/// assert_eq!(safe, "<b>Bold</b> text");
///
/// let dangerous = sanitize_html("<script>alert('xss')</script>");
/// assert!(!dangerous.contains("<script>"));
/// ```
pub fn sanitize_html(html: &str) -> String {
    // Basic HTML sanitization - in a real implementation, you'd use a proper
    // HTML sanitization library like ammonia
    html.replace("<script", "&lt;script")
        .replace("</script>", "&lt;/script&gt;")
        .replace("javascript:", "")
        .replace("on", "")
}

/// Format file size in human-readable format
///
/// # Arguments
///
/// * `bytes` - Size in bytes
///
/// # Returns
///
/// Returns a human-readable string representation of the file size.
///
/// # Examples
///
/// ```rust
/// use ankify::utils::format_file_size;
///
/// assert_eq!(format_file_size(1024), "1.0 KB");
/// assert_eq!(format_file_size(1536), "1.5 KB");
/// assert_eq!(format_file_size(1048576), "1.0 MB");
/// ```
pub fn format_file_size(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    const THRESHOLD: f64 = 1024.0;

    if bytes == 0 {
        return "0 B".to_string();
    }

    let bytes_f = bytes as f64;
    let unit_index = (bytes_f.log2() / THRESHOLD.log2()).floor() as usize;
    let unit_index = unit_index.min(UNITS.len() - 1);

    let size = bytes_f / THRESHOLD.powi(unit_index as i32);

    if unit_index == 0 {
        format!("{} {}", bytes, UNITS[unit_index])
    } else {
        format!("{:.1} {}", size, UNITS[unit_index])
    }
}

/// Escape special characters in card content
///
/// Ensures that card content with special characters is properly handled
/// when converting to different formats.
///
/// # Arguments
///
/// * `content` - Content to escape
///
/// # Returns
///
/// Returns escaped content safe for use in card fields.
///
/// # Examples
///
/// ```rust
/// use ankify::utils::escape_card_content;
///
/// let escaped = escape_card_content("Question with \"quotes\" and <tags>");
/// assert!(escaped.contains("&quot;"));
/// assert!(escaped.contains("&lt;"));
/// ```
pub fn escape_card_content(content: &str) -> String {
    content
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#x27;")
}

/// Normalize line endings to Unix style
///
/// Converts Windows (CRLF) and Mac (CR) line endings to Unix (LF) style
/// for consistent processing across platforms.
///
/// # Arguments
///
/// * `content` - Content to normalize
///
/// # Returns
///
/// Returns content with normalized line endings.
///
/// # Examples
///
/// ```rust
/// use ankify::utils::normalize_line_endings;
///
/// let normalized = normalize_line_endings("Line 1\r\nLine 2\rLine 3\n");
/// assert_eq!(normalized, "Line 1\nLine 2\nLine 3\n");
/// ```
pub fn normalize_line_endings(content: &str) -> String {
    content.replace("\r\n", "\n").replace('\r', "\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::{NamedTempFile, TempDir};
    use tokio::fs;

    #[tokio::test]
    async fn test_hash_file() {
        let mut temp_file = NamedTempFile::new().unwrap();
        let content = b"Hello, world!";
        std::io::Write::write_all(&mut temp_file, content).unwrap();

        let hash = hash_file(temp_file.path()).await.unwrap();
        assert_eq!(hash.len(), 64); // SHA-256 produces 64-character hex strings

        // Same content should produce same hash
        let hash2 = hash_file(temp_file.path()).await.unwrap();
        assert_eq!(hash, hash2);
    }

    #[test]
    fn test_hash_string() {
        let hash1 = hash_string("test");
        let hash2 = hash_string("test");
        let hash3 = hash_string("different");

        assert_eq!(hash1, hash2);
        assert_ne!(hash1, hash3);
        assert_eq!(hash1.len(), 64);
    }

    #[tokio::test]
    async fn test_ensure_dir() {
        let temp_dir = TempDir::new().unwrap();
        let test_path = temp_dir.path().join("new_dir");

        assert!(!test_path.exists());
        ensure_dir(&test_path).await.unwrap();
        assert!(test_path.exists());

        // Should not error if directory already exists
        ensure_dir(&test_path).await.unwrap();
    }

    #[test]
    fn test_has_extension() {
        assert!(has_extension(Path::new("test.typ"), "typ"));
        assert!(has_extension(Path::new("test.TYP"), "typ")); // Case insensitive
        assert!(!has_extension(Path::new("test.txt"), "typ"));
        assert!(!has_extension(Path::new("test"), "typ"));
    }

    #[test]
    fn test_generate_card_id() {
        let id1 = generate_card_id("Math", "2+2", "4");
        let id2 = generate_card_id("Math", "2+2", "4");
        let id3 = generate_card_id("Math", "3+3", "6");

        assert_eq!(id1, id2);
        assert_ne!(id1, id3);
        assert!(id1.starts_with("card_"));
    }

    #[test]
    fn test_is_valid_deck_name() {
        assert!(is_valid_deck_name("Math"));
        assert!(is_valid_deck_name("Languages::French"));
        assert!(is_valid_deck_name("Subject 101"));

        assert!(!is_valid_deck_name(""));
        assert!(!is_valid_deck_name("Deck/With/Slashes"));
        assert!(!is_valid_deck_name(" Leading space"));
        assert!(!is_valid_deck_name("Trailing space "));
        assert!(!is_valid_deck_name(&"a".repeat(101))); // Too long
    }

    #[test]
    fn test_sanitize_html() {
        assert_eq!(sanitize_html("<b>Bold</b>"), "<b>Bold</b>");
        assert!(!sanitize_html("<script>alert('xss')</script>").contains("<script>"));
        assert!(!sanitize_html("javascript:alert('xss')").contains("javascript:"));
    }

    #[test]
    fn test_format_file_size() {
        assert_eq!(format_file_size(0), "0 B");
        assert_eq!(format_file_size(1024), "1.0 KB");
        assert_eq!(format_file_size(1536), "1.5 KB");
        assert_eq!(format_file_size(1048576), "1.0 MB");
        assert_eq!(format_file_size(1073741824), "1.0 GB");
    }

    #[test]
    fn test_escape_card_content() {
        let escaped = escape_card_content("Test with \"quotes\" and <tags>");
        assert!(escaped.contains("&quot;"));
        assert!(escaped.contains("&lt;"));
        assert!(escaped.contains("&gt;"));
    }

    #[test]
    fn test_normalize_line_endings() {
        assert_eq!(normalize_line_endings("Line 1\r\nLine 2"), "Line 1\nLine 2");
        assert_eq!(normalize_line_endings("Line 1\rLine 2"), "Line 1\nLine 2");
        assert_eq!(normalize_line_endings("Line 1\nLine 2"), "Line 1\nLine 2");
    }

    #[tokio::test]
    async fn test_find_files() {
        let temp_dir = TempDir::new().unwrap();

        // Create test files
        let test_file1 = temp_dir.path().join("test1.typ");
        let test_file2 = temp_dir.path().join("test2.typ");
        let other_file = temp_dir.path().join("other.txt");

        fs::write(&test_file1, "content1").await.unwrap();
        fs::write(&test_file2, "content2").await.unwrap();
        fs::write(&other_file, "content3").await.unwrap();

        let found = find_files(temp_dir.path(), "*.typ", false).await.unwrap();
        assert_eq!(found.len(), 2);
        assert!(found.contains(&test_file1));
        assert!(found.contains(&test_file2));
        assert!(!found.contains(&other_file));
    }
}
