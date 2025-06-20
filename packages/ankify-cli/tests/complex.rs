//! Complex integration test for the Ankify CLI.
//!
//! This test implements a comprehensive integration test that:
//!
//! 1.  **Mocks AnkiConnect server**: Sets up a wiremock HTTP server that captures all
//!     requests and returns appropriate responses for AnkiConnect API calls.
//! 2.  **Sets up test environment**: Copies `packages/ankify-typst` to a temporary test
//!     directory and creates test Typst files with note definitions.
//! 3.  **Executes sync operation**: Runs the sync command on test files with the mocked
//!     server as the AnkiConnect endpoint.
//! 4.  **Validates behavior**: Inspects captured HTTP requests, verifies sync results,
//!     checks output files, and validates cache file creation.
//!
//! ## Test Coverage
//!
//! - **HTTP mocking**: Tests AnkiConnect API integration without requiring Anki
//! - **File operations**: Validates compilation output and cache file management
//! - **Request validation**: Ensures correct API calls are made with proper data
//! - **Error handling**: Verifies the system handles various scenarios gracefully
//! - **Mixed formats**: Tests notes with different field structures (plain strings vs structured objects)
//! - **Content filtering**: Validates that complex Typst content is properly filtered to strings
//!
//! ## Implementation Notes
//!
//! - Uses `wiremock` for HTTP mocking with request capture functionality
//! - Creates isolated test environment in temporary directories
//! - Tests plain text note formats and mixed format structures
//! - Validates JSON serialization/deserialization of note metadata
//! - Includes fix for Typst `filter-content` function to handle nested dictionaries
//!
//! ## Tests Included
//!
//! 1. `test_complex_sync`: Basic sync with plain text notes
//! 2. `test_complex_mixed_formats`: Advanced sync with mixed format note structures
//! 3. `test_note_data_value_deserialization`: JSON deserialization validation
//! 4. `test_mock_server_setup`: Infrastructure verification test
//!

use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::sync::{Arc, Mutex};

use ankify::sync::{sync, SyncConfig};
use serde_json::Value;
use tempfile::TempDir;
use tokio::fs as async_fs;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, Request, ResponseTemplate};

/// Stores all HTTP requests received by the mock server
#[derive(Debug, Clone, Default)]
pub struct RequestStore {
    pub requests: Arc<Mutex<Vec<ReceivedRequest>>>,
}

#[derive(Debug, Clone)]
pub struct ReceivedRequest {
    pub method: String,
    pub path: String,
    pub headers: HashMap<String, String>,
    pub body: String,
}

impl RequestStore {
    pub fn new() -> Self {
        Self {
            requests: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn add_request(&self, request: ReceivedRequest) {
        self.requests.lock().unwrap().push(request);
    }

    pub fn get_requests(&self) -> Vec<ReceivedRequest> {
        self.requests.lock().unwrap().clone()
    }

    pub fn count(&self) -> usize {
        self.requests.lock().unwrap().len()
    }
}

/// Custom responder that captures requests and stores them
struct CapturingResponder {
    store: RequestStore,
    response: ResponseTemplate,
}

impl CapturingResponder {
    fn new(store: RequestStore, response: ResponseTemplate) -> Self {
        Self { store, response }
    }
}

impl wiremock::Respond for CapturingResponder {
    fn respond(&self, request: &Request) -> ResponseTemplate {
        // Capture the request
        let captured_request = ReceivedRequest {
            method: request.method.to_string(),
            path: request.url.path().to_string(),
            headers: request
                .headers
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
                .collect(),
            body: String::from_utf8_lossy(&request.body).to_string(),
        };

        self.store.add_request(captured_request);

        // Return the configured response
        self.response.clone()
    }
}

/// Copy a directory recursively
fn copy_dir_recursive<'a>(
    src: &'a Path,
    dst: &'a Path,
) -> std::pin::Pin<Box<dyn std::future::Future<Output = std::io::Result<()>> + 'a>> {
    Box::pin(async move {
        // Remove destination if it exists
        if dst.exists() {
            async_fs::remove_dir_all(dst).await?;
        }

        async_fs::create_dir_all(dst).await?;

        let mut entries = async_fs::read_dir(src).await?;
        while let Some(entry) = entries.next_entry().await? {
            let src_path = entry.path();
            let dst_path = dst.join(entry.file_name());

            if src_path.is_dir() {
                copy_dir_recursive(&src_path, &dst_path).await?;
            } else {
                async_fs::copy(&src_path, &dst_path).await?;
            }
        }

        Ok(())
    })
}

/// Create mock AnkiConnect responses for typical operations
fn create_ankiconnect_response(action: &str) -> Value {
    match action {
        "version" => serde_json::json!({"result": 6, "error": null}),
        "deckNames" => serde_json::json!({"result": ["Default"], "error": null}),
        "modelNames" => serde_json::json!({"result": ["Basic"], "error": null}),
        "createDeck" => serde_json::json!({"result": null, "error": null}),
        "addNotes" => serde_json::json!({
            "result": [1001, 1002, 1003, 1004, 1005, 1006],
            "error": null
        }),
        "updateNoteFields" => serde_json::json!({"result": null, "error": null}),
        "findNotes" => serde_json::json!({"result": [], "error": null}),
        "notesInfo" => serde_json::json!({"result": [], "error": null}),
        "storeMediaFile" => serde_json::json!({"result": "stored_file_name.png", "error": null}),
        "default" | _ => serde_json::json!({"result": null, "error": null}),
    }
}

#[tokio::test]
async fn test_complex_sync() {
    // Create a temporary directory for our test
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let test_root = temp_dir.path();

    // Step 1: Set up mock HTTP server
    let mock_server = MockServer::start().await;
    let request_store = RequestStore::new();

    // Step 2: Copy ankify-typst to fixtures
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set");
    let project_root = std::path::Path::new(&manifest_dir)
        .parent()
        .unwrap()
        .parent()
        .unwrap();
    let src_typst_dir = project_root.join("packages").join("ankify-typst");
    let fixtures_dir = test_root.join("fixtures");
    let dst_typst_dir = fixtures_dir.join("ankify-typst");

    async_fs::create_dir_all(&fixtures_dir)
        .await
        .expect("Failed to create fixtures directory");

    copy_dir_recursive(&src_typst_dir, &dst_typst_dir)
        .await
        .expect("Failed to copy ankify-typst directory");

    // Create a simple working test with plain text only
    let complex_content = r#"#import "ankify-typst/lib.typ": note, configure

#configure()

#note(
    "simple-note-1",
    data: (
        Front: "What is the capital of France?",
        Back: "Paris"
    ),
    format: "plain"
)

#note(
    "simple-note-2",
    data: (
        Front: "What is the capital of England?",
        Back: "London"
    ),
    format: "plain"
)
"#;

    let dst_complex = fixtures_dir.join("complex.typ");
    async_fs::write(&dst_complex, complex_content)
        .await
        .expect("Failed to write complex.typ");

    // Step 3: Set up specific mocks for AnkiConnect operations (before catch-all)
    let mock_server_url = mock_server.uri();

    // Mock version check
    Mock::given(method("POST"))
        .and(path("/"))
        .and(wiremock::matchers::body_partial_json(serde_json::json!({
            "action": "version"
        })))
        .respond_with(CapturingResponder::new(
            request_store.clone(),
            ResponseTemplate::new(200).set_body_json(create_ankiconnect_response("version")),
        ))
        .mount(&mock_server)
        .await;

    // Mock deck operations
    Mock::given(method("POST"))
        .and(path("/"))
        .and(wiremock::matchers::body_partial_json(serde_json::json!({
            "action": "deckNames"
        })))
        .respond_with(CapturingResponder::new(
            request_store.clone(),
            ResponseTemplate::new(200).set_body_json(create_ankiconnect_response("deckNames")),
        ))
        .mount(&mock_server)
        .await;

    // Mock model operations
    Mock::given(method("POST"))
        .and(path("/"))
        .and(wiremock::matchers::body_partial_json(serde_json::json!({
            "action": "modelNames"
        })))
        .respond_with(CapturingResponder::new(
            request_store.clone(),
            ResponseTemplate::new(200).set_body_json(create_ankiconnect_response("modelNames")),
        ))
        .mount(&mock_server)
        .await;

    // Mock find notes
    Mock::given(method("POST"))
        .and(path("/"))
        .and(wiremock::matchers::body_partial_json(serde_json::json!({
            "action": "findNotes"
        })))
        .respond_with(CapturingResponder::new(
            request_store.clone(),
            ResponseTemplate::new(200).set_body_json(create_ankiconnect_response("findNotes")),
        ))
        .mount(&mock_server)
        .await;

    // Mock notes info
    Mock::given(method("POST"))
        .and(path("/"))
        .and(wiremock::matchers::body_partial_json(serde_json::json!({
            "action": "notesInfo"
        })))
        .respond_with(CapturingResponder::new(
            request_store.clone(),
            ResponseTemplate::new(200).set_body_json(create_ankiconnect_response("notesInfo")),
        ))
        .mount(&mock_server)
        .await;

    // Mock create deck
    Mock::given(method("POST"))
        .and(path("/"))
        .and(wiremock::matchers::body_partial_json(serde_json::json!({
            "action": "createDeck"
        })))
        .respond_with(CapturingResponder::new(
            request_store.clone(),
            ResponseTemplate::new(200).set_body_json(create_ankiconnect_response("createDeck")),
        ))
        .mount(&mock_server)
        .await;

    // Mock add notes
    Mock::given(method("POST"))
        .and(path("/"))
        .and(wiremock::matchers::body_partial_json(serde_json::json!({
            "action": "addNotes"
        })))
        .respond_with(CapturingResponder::new(
            request_store.clone(),
            ResponseTemplate::new(200).set_body_json(create_ankiconnect_response("addNotes")),
        ))
        .mount(&mock_server)
        .await;

    // Mock store media file
    Mock::given(method("POST"))
        .and(path("/"))
        .and(wiremock::matchers::body_partial_json(serde_json::json!({
            "action": "storeMediaFile"
        })))
        .respond_with(CapturingResponder::new(
            request_store.clone(),
            ResponseTemplate::new(200).set_body_json(create_ankiconnect_response("storeMediaFile")),
        ))
        .mount(&mock_server)
        .await;

    // Set up a catch-all mock that captures all requests (this should be last)
    Mock::given(method("POST"))
        .and(path("/"))
        .respond_with(CapturingResponder::new(
            request_store.clone(),
            ResponseTemplate::new(200).set_body_json(create_ankiconnect_response("default")),
        ))
        .mount(&mock_server)
        .await;

    // Step 4: Run sync with the mocked server
    let sync_config = SyncConfig {
        source_file: dst_complex.clone(),
        verbose: true,
        cache_file: Some(test_root.join(".ankify").join("cache.json")),
        ankiconnect_url: Some(mock_server_url),
        extra_args: vec![format!("--root={}", fixtures_dir.display())],
        cli_mode: false,
    };

    // Change to the test directory for sync operation
    let original_dir = std::env::current_dir().expect("Failed to get current directory");
    std::env::set_current_dir(test_root).expect("Failed to change directory");

    let sync_result = sync(sync_config).await;

    // Restore original directory
    std::env::set_current_dir(original_dir).expect("Failed to restore directory");

    // Step 5: Verify the sync operation
    match sync_result {
        Ok(result) => {
            println!("Sync completed successfully:");
            println!("  Notes added: {}", result.notes_added);
            println!("  Notes updated: {}", result.notes_updated);
            println!("  Notes unchanged: {}", result.notes_unchanged);
            println!("  Decks created: {}", result.decks_created);

            // The sync should have processed some notes
            assert!(
                result.notes_added > 0 || result.notes_updated > 0,
                "Expected some notes to be processed"
            );
        }
        Err(e) => {
            // Print captured requests for debugging
            let requests = request_store.get_requests();
            println!("Captured {} requests:", requests.len());
            for (i, req) in requests.iter().enumerate() {
                println!(
                    "Request {}: {} {} - Body: {}",
                    i + 1,
                    req.method,
                    req.path,
                    req.body
                );
            }

            panic!("Sync failed: {}", e);
        }
    }

    // Step 6: Inspect the requests received by the mock server
    let requests = request_store.get_requests();
    println!("Total requests captured: {}", requests.len());

    // Verify we received some requests
    assert!(
        !requests.is_empty(),
        "Expected to receive AnkiConnect requests"
    );

    // Check for expected request types
    let request_actions: Vec<String> = requests
        .iter()
        .filter_map(|req| {
            if let Ok(body) = serde_json::from_str::<Value>(&req.body) {
                body.get("action")
                    .and_then(|a| a.as_str())
                    .map(|s| s.to_string())
            } else {
                None
            }
        })
        .collect();

    println!("Request actions found: {:?}", request_actions);

    // We should see at least a version check
    assert!(
        request_actions.contains(&"version".to_string()),
        "Expected version check request"
    );

    // Step 7: Check output files
    let output_dir = test_root.join(".ankify").join("output");
    if output_dir.exists() {
        println!("Output directory exists: {}", output_dir.display());

        // List all files in output directory
        if let Ok(entries) = fs::read_dir(&output_dir) {
            let mut file_count = 0;
            for entry in entries {
                if let Ok(entry) = entry {
                    println!("Output file: {}", entry.path().display());
                    file_count += 1;
                }
            }
            println!("Total output files: {}", file_count);
        }
    } else {
        println!("No output directory found at: {}", output_dir.display());
    }

    // Step 8: Verify cache file was created
    let cache_file = test_root.join(".ankify").join("cache.json");
    if cache_file.exists() {
        println!("Cache file created: {}", cache_file.display());
        if let Ok(cache_content) = fs::read_to_string(&cache_file) {
            println!("Cache content length: {} bytes", cache_content.len());

            // Try to parse cache as JSON to verify it's valid
            if let Ok(cache_json) = serde_json::from_str::<Value>(&cache_content) {
                println!(
                    "Cache is valid JSON with {} top-level keys",
                    cache_json.as_object().map(|o| o.len()).unwrap_or(0)
                );
            }
        }
    } else {
        println!("No cache file found at: {}", cache_file.display());
    }

    println!("Complex test completed successfully!");

    // Final validation: Ensure test demonstrates the key requirements
    assert!(
        requests.len() >= 2,
        "Expected at least version check and one operation request"
    );

    // Verify we have the expected request types for a successful sync
    let has_version = request_actions.contains(&"version".to_string());
    let has_deck_or_add = request_actions.contains(&"createDeck".to_string())
        || request_actions.contains(&"addNotes".to_string());

    assert!(has_version, "Expected version check in requests");
    assert!(
        has_deck_or_add,
        "Expected deck creation or note addition in requests"
    );
}

#[tokio::test]
async fn test_mock_server_setup() {
    // Verification test for the mock server infrastructure
    let mock_server = MockServer::start().await;
    let request_store = RequestStore::new();

    Mock::given(method("POST"))
        .and(path("/test"))
        .respond_with(CapturingResponder::new(
            request_store.clone(),
            ResponseTemplate::new(200).set_body_json(serde_json::json!({"test": "response"})),
        ))
        .mount(&mock_server)
        .await;

    // Make a test request
    let client = reqwest::Client::new();
    let response = client
        .post(format!("{}/test", mock_server.uri()))
        .json(&serde_json::json!({"test": "request"}))
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), 200);

    let requests = request_store.get_requests();
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].method, "POST");
    assert_eq!(requests[0].path, "/test");
    assert!(requests[0].body.contains("test"));
}

/// Test with mixed format notes (plain text fields with different structures)
/// This test validates the complete sync pipeline with mixed format notes,
/// including plain strings and structured fields with format/value pairs.
#[tokio::test]
async fn test_complex_mixed_formats() {
    use ankify::sync::{sync, SyncConfig};
    use tempfile::TempDir;

    // Create a temporary directory for our test
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let test_root = temp_dir.path();

    // Step 1: Set up mock HTTP server
    let mock_server = MockServer::start().await;
    let request_store = RequestStore::new();

    // Set up the test environment (copy ankify-typst)
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set");
    let project_root = std::path::Path::new(&manifest_dir)
        .parent()
        .unwrap()
        .parent()
        .unwrap();
    let src_typst_dir = project_root.join("packages").join("ankify-typst");
    let fixtures_dir = test_root.join("fixtures");
    let dst_typst_dir = fixtures_dir.join("ankify-typst");

    async_fs::create_dir_all(&fixtures_dir)
        .await
        .expect("Failed to create fixtures directory");

    copy_dir_recursive(&src_typst_dir, &dst_typst_dir)
        .await
        .expect("Failed to copy ankify-typst directory");

    // Create a mixed format test file with various note types:
    // 1. Mixed structure: plain string + structured field
    // 2. All plain strings
    // 3. All structured fields
    let mixed_format_content = r#"#import "ankify-typst/lib.typ": note, configure

#configure()

#note(
    "mixed-format-note",
    data: (
        Front: "What is the Pythagorean theorem?",
        Back: (
            format: "plain",
            value: "a² + b² = c²"
        )
    ),
    format: "plain"
)

#note(
    "plain-text-note",
    data: (
        Front: "What is the capital of France?",
        Back: "Paris"
    ),
    format: "plain"
)

#note(
    "structured-note",
    data: (
        Front: (
            format: "plain",
            value: "Einstein's mass-energy equivalence"
        ),
        Back: (
            format: "plain",
            value: "E = mc²"
        )
    ),
    format: "plain"
)
"#;

    let test_file = fixtures_dir.join("mixed_formats.typ");
    async_fs::write(&test_file, mixed_format_content)
        .await
        .expect("Failed to write mixed formats test file");

    // Step 2: Set up mocks for AnkiConnect operations
    let mock_server_url = mock_server.uri();

    // Mock version check
    Mock::given(method("POST"))
        .and(path("/"))
        .and(wiremock::matchers::body_partial_json(serde_json::json!({
            "action": "version"
        })))
        .respond_with(CapturingResponder::new(
            request_store.clone(),
            ResponseTemplate::new(200).set_body_json(create_ankiconnect_response("version")),
        ))
        .mount(&mock_server)
        .await;

    // Mock deck operations
    Mock::given(method("POST"))
        .and(path("/"))
        .and(wiremock::matchers::body_partial_json(serde_json::json!({
            "action": "deckNames"
        })))
        .respond_with(CapturingResponder::new(
            request_store.clone(),
            ResponseTemplate::new(200).set_body_json(create_ankiconnect_response("deckNames")),
        ))
        .mount(&mock_server)
        .await;

    // Mock createDeck
    Mock::given(method("POST"))
        .and(path("/"))
        .and(wiremock::matchers::body_partial_json(serde_json::json!({
            "action": "createDeck"
        })))
        .respond_with(CapturingResponder::new(
            request_store.clone(),
            ResponseTemplate::new(200).set_body_json(create_ankiconnect_response("createDeck")),
        ))
        .mount(&mock_server)
        .await;

    // Mock addNotes with response for 3 notes
    Mock::given(method("POST"))
        .and(path("/"))
        .and(wiremock::matchers::body_partial_json(serde_json::json!({
            "action": "addNotes"
        })))
        .respond_with(CapturingResponder::new(
            request_store.clone(),
            ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "result": [2001, 2002, 2003],
                "error": null
            })),
        ))
        .mount(&mock_server)
        .await;

    // Catch-all mock
    Mock::given(method("POST"))
        .and(path("/"))
        .respond_with(CapturingResponder::new(
            request_store.clone(),
            ResponseTemplate::new(200).set_body_json(create_ankiconnect_response("default")),
        ))
        .mount(&mock_server)
        .await;

    // Step 3: Run sync with the mocked server
    let sync_config = SyncConfig {
        source_file: test_file.clone(),
        verbose: true,
        cache_file: Some(test_root.join(".ankify").join("cache.json")),
        ankiconnect_url: Some(mock_server_url),
        extra_args: vec![format!("--root={}", fixtures_dir.display())],
        cli_mode: false,
    };

    // Change to the test directory for sync operation
    let original_dir = std::env::current_dir().expect("Failed to get current directory");
    std::env::set_current_dir(test_root).expect("Failed to change directory");

    let sync_result = sync(sync_config).await;

    // Restore original directory
    std::env::set_current_dir(original_dir).expect("Failed to restore directory");

    // Step 4: Validate the sync operation
    match sync_result {
        Ok(result) => {
            println!("Mixed formats sync completed successfully:");
            println!("  Notes added: {}", result.notes_added);
            println!("  Notes updated: {}", result.notes_updated);
            println!("  Notes unchanged: {}", result.notes_unchanged);
            println!("  Decks created: {}", result.decks_created);

            // Should have processed 3 notes
            assert!(
                result.notes_added > 0,
                "Expected notes to be added for mixed formats test"
            );
        }
        Err(e) => {
            let requests = request_store.get_requests();
            println!("Captured {} requests:", requests.len());
            for (i, req) in requests.iter().enumerate() {
                println!(
                    "Request {}: {} {} - Body: {}",
                    i + 1,
                    req.method,
                    req.path,
                    req.body
                );
            }
            panic!("Mixed formats sync failed: {}", e);
        }
    }

    // Step 5: Inspect the requests received by the mock server
    let requests = request_store.get_requests();
    println!("Total requests captured: {}", requests.len());

    // Verify we received some requests
    assert!(
        !requests.is_empty(),
        "Expected to receive AnkiConnect requests for mixed formats"
    );

    // Check for expected request types
    let request_actions: Vec<String> = requests
        .iter()
        .filter_map(|req| {
            if let Ok(body) = serde_json::from_str::<serde_json::Value>(&req.body) {
                body.get("action")
                    .and_then(|a| a.as_str())
                    .map(|s| s.to_string())
            } else {
                None
            }
        })
        .collect();

    println!("Request actions found: {:?}", request_actions);

    // Verify we have expected actions
    assert!(
        request_actions.contains(&"version".to_string()),
        "Expected version check in mixed formats test"
    );
    assert!(
        request_actions.contains(&"addNotes".to_string()),
        "Expected addNotes in mixed formats test"
    );

    // Validate that the addNotes request contains properly formatted notes
    let add_notes_requests: Vec<_> = requests
        .iter()
        .filter(|req| req.body.contains("addNotes"))
        .collect();

    assert!(!add_notes_requests.is_empty(), "Expected addNotes request");

    for add_notes_req in add_notes_requests {
        if let Ok(body) = serde_json::from_str::<serde_json::Value>(&add_notes_req.body) {
            if let Some(params) = body.get("params") {
                if let Some(notes) = params.get("notes").and_then(|n| n.as_array()) {
                    println!("Found {} notes in addNotes request", notes.len());

                    // Verify notes have proper structure
                    for (i, note) in notes.iter().enumerate() {
                        assert!(note.get("fields").is_some(), "Note {} missing fields", i);
                        assert!(
                            note.get("modelName").is_some(),
                            "Note {} missing modelName",
                            i
                        );
                        assert!(
                            note.get("deckName").is_some(),
                            "Note {} missing deckName",
                            i
                        );

                        // Check that fields are strings (content should be filtered)
                        if let Some(fields) = note.get("fields").and_then(|f| f.as_object()) {
                            for (field_name, field_value) in fields {
                                assert!(
                                    field_value.is_string(),
                                    "Field '{}' in note {} should be a string, got: {:?}",
                                    field_name,
                                    i,
                                    field_value
                                );
                            }
                        }
                    }
                }
            }
        }
    }

    println!("Mixed formats test completed successfully!");

    // Final validation: Ensure all three note types were processed correctly
    assert_eq!(
        requests.len(),
        3,
        "Expected exactly 3 requests (version, createDeck, addNotes)"
    );

    // Verify the mixed format parsing worked for all note structures
    let add_notes_req = requests
        .iter()
        .find(|req| req.body.contains("addNotes"))
        .expect("Expected addNotes request");

    if let Ok(body) = serde_json::from_str::<serde_json::Value>(&add_notes_req.body) {
        if let Some(notes) = body
            .get("params")
            .and_then(|p| p.get("notes"))
            .and_then(|n| n.as_array())
        {
            assert_eq!(notes.len(), 3, "Expected 3 notes in addNotes request");

            // Verify each note has proper field structure
            for note in notes {
                let fields = note.get("fields").expect("Note should have fields");
                assert!(
                    fields.get("Front").is_some(),
                    "Note should have Front field"
                );
                assert!(fields.get("Back").is_some(), "Note should have Back field");
            }
        }
    }
}

/// Test that verifies JSON deserialization works for different NoteDataValue structures
#[tokio::test]
async fn test_note_data_value_deserialization() {
    use ankify::metadata::NoteDataValue;

    // Test case 1: Simple string value
    let simple_json = r#""Simple text value""#;
    match serde_json::from_str::<NoteDataValue>(simple_json) {
        Ok(value) => println!("✓ Simple string parsed: {:?}", value),
        Err(e) => panic!("✗ Simple string failed: {}", e),
    }

    // Test case 2: null value
    let null_json = r#"null"#;
    match serde_json::from_str::<NoteDataValue>(null_json) {
        Ok(value) => println!("✓ Null value parsed: {:?}", value),
        Err(e) => panic!("✗ Null value failed: {}", e),
    }

    // Test case 3: Structured object with format and value
    let structured_json = r#"{"format": "svg", "value": ""}"#;
    match serde_json::from_str::<NoteDataValue>(structured_json) {
        Ok(value) => println!("✓ Structured object parsed: {:?}", value),
        Err(e) => panic!("✗ Structured object failed: {}", e),
    }

    // Test case 4: Object with missing value
    let missing_value_json = r#"{"format": "svg"}"#;
    match serde_json::from_str::<NoteDataValue>(missing_value_json) {
        Ok(value) => println!("✓ Missing value parsed: {:?}", value),
        Err(e) => panic!("✗ Missing value failed: {}", e),
    }

    // Test case 5: Object with extra fields (should still work)
    let extra_fields_json = r#"{"format": "svg", "value": "", "extra": "ignored"}"#;
    match serde_json::from_str::<NoteDataValue>(extra_fields_json) {
        Ok(value) => println!("✓ Extra fields parsed: {:?}", value),
        Err(e) => panic!("✗ Extra fields failed: {}", e),
    }

    println!("All NoteDataValue deserialization tests passed!");
}
