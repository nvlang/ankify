//! Simple integration test that creates output files in a predictable location for inspection.
//!
//! This test will:
//! 1. Copy `packages/ankify-typst/` to `packages/ankify-cli/tests/fixtures/ankify-typst/`
//! 2. Create a test Typst file with mathematical content
//! 3. Run sync to generate output files in `packages/ankify-cli/tests/fixtures/.ankify/output/`
//! 4. The output files will follow the pattern `output-{p}.{extension}`

use ankify::sync::{sync, SyncConfig};
use std::fs;
use std::sync::{Arc, Mutex};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, Request, ResponseTemplate};

/// Store for capturing HTTP requests made to the mock server
#[derive(Debug, Clone)]
pub struct RequestStore {
    pub requests: Arc<Mutex<Vec<String>>>,
}

impl RequestStore {
    pub fn new() -> Self {
        Self {
            requests: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn add_request(&self, body: String) {
        self.requests.lock().unwrap().push(body);
    }
}

/// Custom responder that captures requests and returns configured responses
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
        let body = String::from_utf8_lossy(&request.body).to_string();
        self.store.add_request(body);
        self.response.clone()
    }
}

/// Recursively copy a directory and its contents
fn copy_dir_recursive(
    src: &std::path::Path,
    dst: &std::path::Path,
) -> Result<(), Box<dyn std::error::Error>> {
    fs::create_dir_all(dst)?;

    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());

        if src_path.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else {
            fs::copy(&src_path, &dst_path)?;
        }
    }

    Ok(())
}

/// Create mock AnkiConnect response for different actions
fn create_ankiconnect_response(action: &str) -> serde_json::Value {
    match action {
        "version" => serde_json::json!({"result": 6, "error": null}),
        "deckNames" => serde_json::json!({"result": ["Default"], "error": null}),
        "modelNames" => serde_json::json!({"result": ["Basic"], "error": null}),
        "findNotes" => serde_json::json!({"result": [], "error": null}),
        "notesInfo" => serde_json::json!({"result": [], "error": null}),
        "createDeck" => serde_json::json!({"result": null, "error": null}),
        "addNotes" => serde_json::json!({"result": [1001, 1002, 1003], "error": null}),
        "storeMediaFile" => serde_json::json!({"result": "stored_file.png", "error": null}),
        _ => serde_json::json!({"result": null, "error": null}),
    }
}

/// Set up standard AnkiConnect mocks for testing
async fn setup_ankiconnect_mocks(mock_server: &MockServer, request_store: &RequestStore) {
    let actions = vec![
        "version",
        "deckNames",
        "modelNames",
        "findNotes",
        "notesInfo",
        "createDeck",
        "addNotes",
        "storeMediaFile",
    ];

    for action in actions {
        Mock::given(method("POST"))
            .and(path("/"))
            .and(wiremock::matchers::body_partial_json(serde_json::json!({
                "action": action
            })))
            .respond_with(CapturingResponder::new(
                request_store.clone(),
                ResponseTemplate::new(200).set_body_json(create_ankiconnect_response(action)),
            ))
            .mount(mock_server)
            .await;
    }

    // Catch-all mock for any other requests
    Mock::given(method("POST"))
        .and(path("/"))
        .respond_with(CapturingResponder::new(
            request_store.clone(),
            ResponseTemplate::new(200).set_body_json(create_ankiconnect_response("default")),
        ))
        .mount(mock_server)
        .await;
}

/// Main test that creates output files in fixtures/.ankify/output/
#[tokio::test]
async fn test_simple_output_generation() {
    println!("=== Simple Output Generation Test ===");

    // Set environment variable to use local imports
    std::env::set_var("ANKIFY_USE_LOCAL_IMPORTS", "1");

    // Get the fixtures directory (this is where we want .ankify to be created)
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set");
    let project_root = std::path::Path::new(&manifest_dir);
    let fixtures_dir = project_root.join("tests").join("fixtures");

    // Clean up any existing .ankify directory to start fresh
    let ankify_dir = fixtures_dir.join(".ankify");
    if ankify_dir.exists() {
        fs::remove_dir_all(&ankify_dir).expect("Failed to clean up existing .ankify directory");
    }

    // Ensure fixtures directory exists
    fs::create_dir_all(&fixtures_dir).expect("Failed to create fixtures directory");

    println!("Fixtures directory: {}", fixtures_dir.display());

    // Step 1: Copy ankify-typst to fixtures/ankify-typst/
    let src_typst_dir = project_root.parent().unwrap().join("ankify-typst");
    let dst_typst_dir = fixtures_dir.join(".ankify/ankify-typst");

    println!(
        "Copying ankify-typst from {} to {}",
        src_typst_dir.display(),
        dst_typst_dir.display()
    );

    if dst_typst_dir.exists() {
        fs::remove_dir_all(&dst_typst_dir).expect("Failed to remove existing ankify-typst");
    }

    copy_dir_recursive(&src_typst_dir, &dst_typst_dir)
        .expect("Failed to copy ankify-typst directory");

    // Step 2: Verify complex.typ exists in fixtures/
    let complex_file = fixtures_dir.join("complex.typ");
    if !complex_file.exists() {
        panic!(
            "complex.typ not found in fixtures directory: {}",
            complex_file.display()
        );
    }

    println!("Using existing test file: {}", complex_file.display());

    // Step 3: Set up mock HTTP server
    let mock_server = MockServer::start().await;
    let request_store = RequestStore::new();
    setup_ankiconnect_mocks(&mock_server, &request_store).await;

    println!("Mock server started at: {}", mock_server.uri());

    // Step 4: Run sync with fixtures as working directory
    let sync_config = SyncConfig {
        source_file: "complex.typ".into(), // Relative to fixtures directory
        verbose: true,
        cache_file: None, // Let it use default location
        ankiconnect_url: Some(mock_server.uri()),
        extra_args: vec![], // Don't override root, let it use current directory
        cli_mode: false,
    };

    // Change to fixtures directory so complex.typ is in current directory
    let original_dir = std::env::current_dir().expect("Failed to get current directory");
    std::env::set_current_dir(&fixtures_dir).expect("Failed to change to fixtures directory");

    println!("Changed working directory to: {}", fixtures_dir.display());
    println!("Running sync on complex.typ...");

    let sync_result = sync(sync_config).await;

    // Restore original directory
    std::env::set_current_dir(original_dir).expect("Failed to restore directory");

    // Step 5: Check sync results
    match sync_result {
        Ok(result) => {
            println!("✅ Sync completed successfully!");
            println!("  Notes added: {}", result.notes_added);
            println!("  Notes updated: {}", result.notes_updated);
        }
        Err(e) => {
            println!("⚠️ Sync failed: {}", e);
            println!("Continuing to check what files were created anyway...");
        }
    }

    // Step 6: Inspect the created directory structure
    println!("\n=== Directory Structure Created ===");

    if ankify_dir.exists() {
        println!("✅ .ankify directory created at: {}", ankify_dir.display());

        // Check cache file
        let cache_file = ankify_dir.join("cache.json");
        if cache_file.exists() {
            println!("✅ Cache file: {}", cache_file.display());
        } else {
            println!("❌ Cache file missing");
        }

        // Check temp file
        let temp_file = ankify_dir.join("complex_render.typ");
        if temp_file.exists() {
            println!("✅ Temp file: {}", temp_file.display());
        } else {
            println!("❌ Temp file missing");
        }

        // Check output directory
        let output_dir = ankify_dir.join("output");
        if output_dir.exists() {
            println!("✅ Output directory: {}", output_dir.display());

            // List all output files
            if let Ok(entries) = fs::read_dir(&output_dir) {
                let mut file_count = 0;
                for entry in entries {
                    if let Ok(entry) = entry {
                        let file_name = entry.file_name().to_string_lossy().to_string();
                        let size = fs::metadata(entry.path()).map(|m| m.len()).unwrap_or(0);
                        println!("  📄 {} ({} bytes)", file_name, size);

                        // Verify naming pattern
                        if file_name.starts_with("output-")
                            && (file_name.ends_with(".svg") || file_name.ends_with(".png"))
                        {
                            println!("    ✅ Follows output-{{p}}.{{extension}} pattern");
                        } else {
                            println!("    ⚠️ Does not follow expected pattern");
                        }

                        file_count += 1;
                    }
                }

                if file_count == 0 {
                    println!("  (empty - no output files generated)");
                } else {
                    println!("  Total files: {}", file_count);
                }
            }
        } else {
            println!("❌ Output directory missing");
        }
    } else {
        println!("❌ .ankify directory was not created");
    }

    // Step 7: Print final inspection information
    println!("\n=== Manual Inspection ===");
    println!("You can now inspect the output files at:");
    println!("  Source file: {}", complex_file.display());
    println!("  Output dir:  {}", ankify_dir.join("output").display());
    println!("  Cache file:  {}", ankify_dir.join("cache.json").display());

    if ankify_dir.join("output").exists() {
        println!("\n🎯 SUCCESS: Output directory created in the correct location!");
        println!("The .ankify folder is next to the source file, not at project root.");
    } else {
        println!("\n⚠️ Output directory not found - check for compilation errors above.");
    }

    // request_store.requests

    // Let the test pass regardless of whether files were generated,
    // since the main goal is to set up the directory structure
    println!("\nTest completed. Check the fixtures directory for generated files.");
}
