//! Integration tests for the Ankify sync pipeline.
//!
//! These exercise the real query → render → compile → request pipeline by
//! shelling out to `typst`, against a mock AnkiConnect server. The `ankify`
//! Typst package is installed into a throwaway local package directory, so the
//! tests do not depend on a system-wide package install.
//!
//! `typst` must be installed and on `PATH`.

use ankify::generate::PLUGIN_VERSION;
use ankify::sync::{sync, SyncConfig, SyncResult};
use serde_json::{json, Value};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, Once};
use tempfile::TempDir;
use wiremock::matchers::method;
use wiremock::{Mock, MockServer, Request, Respond, ResponseTemplate};

// ---------------------------------------------------------------------------
// Test harness
// ---------------------------------------------------------------------------

/// Directory holding the test-only install of the `ankify` Typst package.
fn package_path() -> PathBuf {
    PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("typst-packages")
}

/// Install the `ankify` Typst package into a local package directory and
/// enable the local-import rewrite. Runs once per test process.
fn setup() {
    static INIT: Once = Once::new();
    INIT.call_once(|| {
        assert!(
            std::process::Command::new("typst")
                .arg("--version")
                .output()
                .is_ok(),
            "the `typst` binary must be installed and on PATH to run these tests",
        );
        let src = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("ankify-typst");
        let dst = package_path()
            .join("local")
            .join("ankify")
            .join(PLUGIN_VERSION);
        let _ = fs::remove_dir_all(&dst);
        copy_dir(&src, &dst).expect("install ankify-typst test package");
        std::env::set_var("ANKIFY_USE_LOCAL_IMPORTS", "1");
    });
}

fn copy_dir(src: &Path, dst: &Path) -> std::io::Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let from = entry.path();
        let to = dst.join(entry.file_name());
        if from.is_dir() {
            copy_dir(&from, &to)?;
        } else {
            fs::copy(&from, &to)?;
        }
    }
    Ok(())
}

/// A mock AnkiConnect server that records every request and returns plausible
/// responses (notably, one fresh note ID per note in an `addNotes` request).
#[derive(Clone)]
struct AnkiConnectMock {
    requests: Arc<Mutex<Vec<Value>>>,
    next_id: Arc<Mutex<u64>>,
}

impl AnkiConnectMock {
    fn new() -> Self {
        Self {
            requests: Arc::new(Mutex::new(Vec::new())),
            next_id: Arc::new(Mutex::new(1_700_000_000_000)),
        }
    }

    fn alloc_id(&self) -> u64 {
        let mut id = self.next_id.lock().unwrap();
        *id += 1;
        *id
    }
}

impl Respond for AnkiConnectMock {
    fn respond(&self, request: &Request) -> ResponseTemplate {
        let body: Value = serde_json::from_slice(&request.body).unwrap_or(Value::Null);
        self.requests.lock().unwrap().push(body.clone());

        let result = match body.get("action").and_then(Value::as_str).unwrap_or("") {
            "version" => json!(6),
            "createDeck" => json!(self.alloc_id()),
            "addNotes" => {
                let count = body
                    .pointer("/params/notes")
                    .and_then(Value::as_array)
                    .map_or(0, Vec::len);
                json!((0..count).map(|_| self.alloc_id()).collect::<Vec<_>>())
            }
            "deckNames" => json!(["Default"]),
            "deckNamesAndIds" => json!({ "Default": 1 }),
            "modelNames" => json!(["Basic", "Cloze"]),
            _ => Value::Null,
        };
        ResponseTemplate::new(200).set_body_json(json!({ "result": result, "error": null }))
    }
}

/// A scratch project: a temp directory with a `notes.typ` that can be synced
/// (repeatedly) against a fresh mock AnkiConnect server.
struct TestProject {
    _dir: TempDir,
    root: PathBuf,
    source: PathBuf,
}

struct SyncOutcome {
    result: ankify::error::Result<SyncResult>,
    requests: Vec<Value>,
}

impl TestProject {
    fn new(source: &str) -> Self {
        setup();
        let dir = TempDir::new().expect("temp dir");
        let root = dir.path().canonicalize().expect("canonicalize temp dir");
        let source_path = root.join("notes.typ");
        fs::write(&source_path, source).expect("write source");
        Self {
            _dir: dir,
            root,
            source: source_path,
        }
    }

    fn write(&self, source: &str) {
        fs::write(&self.source, source).expect("rewrite source");
    }

    async fn sync(&self) -> SyncOutcome {
        let server = MockServer::start().await;
        let mock = AnkiConnectMock::new();
        Mock::given(method("POST"))
            .respond_with(mock.clone())
            .mount(&server)
            .await;

        let config = SyncConfig::new(self.source.clone())
            .with_ankiconnect_url(server.uri())
            .with_extra_args(vec![
                "--root".into(),
                self.root.display().to_string(),
                "--package-path".into(),
                package_path().display().to_string(),
            ])
            .with_keep_artifacts(true);

        let result = sync(config).await;
        let requests = mock.requests.lock().unwrap().clone();
        SyncOutcome { result, requests }
    }
}

/// Find the (single) `addNotes` request in a captured request list.
fn add_notes(requests: &[Value]) -> &Value {
    requests
        .iter()
        .find(|r| r["action"] == "addNotes")
        .expect("an addNotes request")
}

/// Width, in points, declared by an SVG file's root `<svg>` element.
fn svg_width_pt(path: &str) -> f64 {
    let svg = fs::read_to_string(path).expect("read svg");
    let re = regex::Regex::new(r#"<svg[^>]*\bwidth="([0-9.]+)pt""#).unwrap();
    re.captures(&svg)
        .and_then(|c| c.get(1))
        .and_then(|m| m.as_str().parse().ok())
        .unwrap_or_else(|| panic!("no width on <svg> in {path}"))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[tokio::test]
async fn plain_notes_carry_exact_field_text() {
    let project = TestProject::new(
        r#"#import "@local/ankify:0.1.0": note, configure
#configure(defaults: (deck: "Plain Deck", tags: ("topic",)))
#note("p1", format: "plain", data: (Front: "Question one", Back: "Answer one"))
#note("p2", format: "plain", data: (Front: "Question two", Back: "Answer two"))
"#,
    );
    let outcome = project.sync().await;
    let result = outcome.result.expect("sync should succeed");
    assert_eq!(result.notes_added, 2);

    let notes = add_notes(&outcome.requests)["params"]["notes"]
        .as_array()
        .expect("notes array")
        .clone();
    assert_eq!(notes.len(), 2);

    // Notes are sent in document order.
    assert_eq!(notes[0]["deckName"], "Plain Deck");
    assert_eq!(notes[0]["fields"]["Front"], "Question one");
    assert_eq!(notes[0]["fields"]["Back"], "Answer one");
    assert_eq!(notes[0]["tags"], json!(["topic"]));
    assert_eq!(notes[1]["fields"]["Front"], "Question two");
    assert_eq!(notes[1]["fields"]["Back"], "Answer two");

    // Plain-text notes attach no media.
    assert!(notes[0].get("picture").map_or(true, Value::is_null));
}

#[tokio::test]
async fn svg_notes_attach_valid_image_media() {
    let project = TestProject::new(
        r#"#import "@local/ankify:0.1.0": note, configure
#configure(defaults: (deck: "Svg Deck"))
#note("s1", format: "svg", data: (Front: [Define $f$], Back: [$f(x) = x^2$]))
"#,
    );
    let outcome = project.sync().await;
    outcome.result.expect("sync should succeed");

    let note = &add_notes(&outcome.requests)["params"]["notes"][0];
    let pictures = note["picture"].as_array().expect("pictures");
    assert_eq!(pictures.len(), 2);

    for picture in pictures {
        let path = picture["path"].as_str().expect("picture path");
        assert!(
            Path::new(path).is_absolute(),
            "media path must be absolute so AnkiConnect can resolve it: {path}",
        );
        assert!(path.ends_with(".svg"));
        let content = fs::read_to_string(path).expect("read media file");
        assert!(content.contains("<svg"), "media file is not an SVG: {path}");

        // The field text is emptied; the image is carried via `picture`.
        let field = picture["fields"][0].as_str().unwrap();
        assert_eq!(note["fields"][field], "");
        assert!(picture["filename"].as_str().unwrap().ends_with(".svg"));
    }
}

#[tokio::test]
async fn png_notes_attach_valid_image_media() {
    let project = TestProject::new(
        r#"#import "@local/ankify:0.1.0": note, configure
#configure(defaults: (deck: "Png Deck"))
#note("g1", format: "png", data: (Front: [A picture], Back: [$1 + 1 = 2$]))
"#,
    );
    let outcome = project.sync().await;
    outcome.result.expect("sync should succeed");

    let note = &add_notes(&outcome.requests)["params"]["notes"][0];
    let pictures = note["picture"].as_array().expect("pictures");
    assert_eq!(pictures.len(), 2);
    for picture in pictures {
        let path = picture["path"].as_str().unwrap();
        assert!(path.ends_with(".png"));
        let bytes = fs::read(path).expect("read media file");
        assert!(
            bytes.starts_with(&[0x89, 0x50, 0x4E, 0x47]),
            "media file is not a PNG: {path}",
        );
    }
}

/// Regression test for the field-to-image mapping.
///
/// With prose in the source document, each note field's rendered image must
/// still be the *correct* one. A previous version rendered the source document
/// first, so its (unpredictable) page count shifted every field onto the wrong
/// page. Here each field is a black box of a distinct width, so the width of
/// the produced image uniquely identifies which field it came from.
#[tokio::test]
async fn field_images_map_to_the_correct_note_field() {
    let project = TestProject::new(
        r#"#import "@local/ankify:0.1.0": note, configure
#configure(scale: 1.0, defaults: (deck: "Mapping"))

= A heading that renders before any notes

#lorem(60)

#note("n1", format: "svg", data: (
  Back: box(width: 40pt, height: 20pt, fill: black),
  Front: box(width: 100pt, height: 20pt, fill: black),
))

#lorem(60)

#note("n2", format: "svg", data: (
  Back: box(width: 160pt, height: 20pt, fill: black),
  Front: box(width: 220pt, height: 20pt, fill: black),
))
"#,
    );
    let outcome = project.sync().await;
    outcome.result.expect("sync should succeed");

    let notes = add_notes(&outcome.requests)["params"]["notes"]
        .as_array()
        .unwrap()
        .clone();
    assert_eq!(notes.len(), 2);

    // 5 mm of page margin on each side of the content box.
    let margin_pt = 2.0 * 5.0 / 25.4 * 72.0;
    // (note index, picture index, field name, content box width in pt).
    // Pictures are attached in alphabetical field order (Back, then Front).
    let expected = [
        (0usize, 0usize, "Back", 40.0),
        (0, 1, "Front", 100.0),
        (1, 0, "Back", 160.0),
        (1, 1, "Front", 220.0),
    ];
    for (note_idx, pic_idx, field, box_width) in expected {
        let picture = &notes[note_idx]["picture"][pic_idx];
        assert_eq!(picture["fields"][0], field, "picture attached to wrong field");
        let path = picture["path"].as_str().unwrap();
        let actual = svg_width_pt(path);
        let want = box_width + margin_pt;
        assert!(
            (actual - want).abs() < 3.0,
            "note {note_idx} field {field}: image is {actual:.1}pt wide, expected ~{want:.1}pt \
             — the field-to-image mapping is wrong",
        );
    }
}

/// `configure(scale: ...)` enlarges rendered card images, and the default
/// scale already makes them larger than the unscaled content.
#[tokio::test]
async fn configurable_scale_resizes_card_images() {
    // Each note's single field is a 100pt-wide box; 5 mm of margin per side.
    let margin_pt = 2.0 * 5.0 / 25.4 * 72.0;
    let doc = |configure: &str| {
        format!(
            r#"#import "@local/ankify:0.1.0": note, configure
{configure}
#note("sc", format: "svg", data: (Back: box(width: 100pt, height: 30pt, fill: black)))
"#
        )
    };

    async fn card_width(source: String) -> f64 {
        let project = TestProject::new(&source);
        let outcome = project.sync().await;
        outcome.result.expect("sync should succeed");
        let path = add_notes(&outcome.requests)["params"]["notes"][0]["picture"][0]["path"]
            .as_str()
            .expect("picture path")
            .to_owned();
        svg_width_pt(&path)
    }

    let unscaled = card_width(doc("#configure(scale: 1.0)")).await;
    let default = card_width(doc("#configure()")).await;
    let tripled = card_width(doc("#configure(scale: 3.0)")).await;

    assert!(
        (unscaled - (100.0 + margin_pt)).abs() < 3.0,
        "scale 1.0: image is {unscaled:.1}pt wide",
    );
    assert!(
        (default - (150.0 + margin_pt)).abs() < 3.0,
        "default scale (1.5): image is {default:.1}pt wide",
    );
    assert!(
        (tripled - (300.0 + margin_pt)).abs() < 5.0,
        "scale 3.0: image is {tripled:.1}pt wide",
    );
}

#[tokio::test]
async fn incremental_sync_adds_updates_and_skips() {
    let v1 = r#"#import "@local/ankify:0.1.0": note, configure
#configure(defaults: (deck: "Incremental"))
#note("i1", format: "plain", data: (Front: "Q1", Back: "A1"))
#note("i2", format: "plain", data: (Front: "Q2", Back: "A2"))
"#;
    let project = TestProject::new(v1);

    let r1 = project.sync().await.result.expect("sync 1");
    assert_eq!(
        (r1.notes_added, r1.notes_updated, r1.notes_unchanged),
        (2, 0, 0),
    );

    // Re-sync with no changes.
    let r2 = project.sync().await.result.expect("sync 2");
    assert_eq!(
        (r2.notes_added, r2.notes_updated, r2.notes_unchanged),
        (0, 0, 2),
    );

    // Edit one note's content.
    project.write(
        r#"#import "@local/ankify:0.1.0": note, configure
#configure(defaults: (deck: "Incremental"))
#note("i1", format: "plain", data: (Front: "Q1", Back: "A1 revised"))
#note("i2", format: "plain", data: (Front: "Q2", Back: "A2"))
"#,
    );
    let r3 = project.sync().await.result.expect("sync 3");
    assert_eq!(
        (r3.notes_added, r3.notes_updated, r3.notes_unchanged),
        (0, 1, 1),
    );

    // A further re-sync must be stable: the updated note's cache entry was
    // refreshed, so it is no longer detected as changed.
    let r4 = project.sync().await.result.expect("sync 4");
    assert_eq!(
        (r4.notes_added, r4.notes_updated, r4.notes_unchanged),
        (0, 0, 2),
    );
}

#[tokio::test]
async fn document_without_configure_does_not_crash() {
    let project = TestProject::new(
        r#"#import "@local/ankify:0.1.0": note
#note("nc1", format: "plain", data: (Front: "Q", Back: "A"))
"#,
    );
    let result = project
        .sync()
        .await
        .result
        .expect("sync must not crash when configure() is absent");
    assert_eq!(result.notes_added, 1);
}
