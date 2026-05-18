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
            "getTags" => json!(["topic", "study"]),
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

/// Width, in points, declared by an inline SVG's root `<svg>` element.
fn svg_width_pt(svg: &str) -> f64 {
    let re = regex::Regex::new(r#"<svg[^>]*\bwidth="([0-9.]+)pt""#).unwrap();
    re.captures(svg)
        .and_then(|c| c.get(1))
        .and_then(|m| m.as_str().parse().ok())
        .expect("no width on the inline <svg>")
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
    assert!(notes[0].get("picture").is_none_or(Value::is_null));
}

#[tokio::test]
async fn svg_notes_inline_themable_markup() {
    let project = TestProject::new(
        r#"#import "@local/ankify:0.1.0": note, configure
#configure(defaults: (deck: "Svg Deck"))
#note("s1", format: "svg", data: (Front: [Define $f$], Back: [$f(x) = x^2$]))
"#,
    );
    let outcome = project.sync().await;
    outcome.result.expect("sync should succeed");

    let note = &add_notes(&outcome.requests)["params"]["notes"][0];
    // SVG fields are inlined into the field value — no media files.
    assert!(
        note.get("picture").is_none_or(Value::is_null),
        "svg notes should attach no media files",
    );
    for field in ["Front", "Back"] {
        let value = note["fields"][field].as_str().expect("field value");
        assert!(value.contains("<svg"), "{field} should hold an inline SVG");
        assert!(
            value.contains("currentColor"),
            "{field}'s foreground should be themable (currentColor)",
        );
        assert!(
            !value.contains("#000000"),
            "{field} should have no hard-coded black foreground left",
        );
        assert!(
            !value.contains("#ffffff"),
            "{field} should have a transparent (unfilled) background",
        );
    }

    // Re-syncing an unchanged note must hit the cache — caching still works
    // even though the field now holds inline SVG rather than referencing a file.
    let again = project.sync().await.result.expect("re-sync should succeed");
    assert_eq!(
        (
            again.notes_added,
            again.notes_updated,
            again.notes_unchanged
        ),
        (0, 0, 1),
    );
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
    // (note index, field name, content box width in pt).
    let expected = [
        (0usize, "Back", 40.0),
        (0, "Front", 100.0),
        (1, "Back", 160.0),
        (1, "Front", 220.0),
    ];
    for (note_idx, field, box_width) in expected {
        let svg = notes[note_idx]["fields"][field]
            .as_str()
            .expect("inline svg field value");
        let actual = svg_width_pt(svg);
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
        let svg = add_notes(&outcome.requests)["params"]["notes"][0]["fields"]["Back"]
            .as_str()
            .expect("inline svg field value")
            .to_owned();
        svg_width_pt(&svg)
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

/// Renaming a note's label, with its content untouched, is recognised as the
/// same card: the existing Anki note is kept, not a duplicate added.
#[tokio::test]
async fn renaming_a_label_updates_in_place_without_duplicating() {
    let project = TestProject::new(
        r#"#import "@local/ankify:0.1.0": note
#note("old-label", format: "plain", data: (Front: "Q", Back: "A"))
"#,
    );
    let first = project.sync().await.result.expect("sync 1");
    assert_eq!(first.notes_added, 1);

    // Rename the label; the content is identical.
    project.write(
        r#"#import "@local/ankify:0.1.0": note
#note("new-label", format: "plain", data: (Front: "Q", Back: "A"))
"#,
    );
    let outcome = project.sync().await;
    let second = outcome.result.expect("sync 2");

    assert_eq!(
        (
            second.notes_added,
            second.notes_updated,
            second.notes_unchanged,
        ),
        (0, 0, 1),
        "a pure rename should add and update nothing",
    );
    assert!(
        second.warnings.is_empty(),
        "a recognised rename should not be reported as an orphan: {:?}",
        second.warnings,
    );
    assert!(
        !outcome.requests.iter().any(|r| r["action"] == "addNotes"),
        "a renamed note must not be re-added to Anki",
    );
}

/// A note removed from the document is reported as an orphan; its Anki note is
/// left untouched.
#[tokio::test]
async fn a_removed_note_is_reported_as_an_orphan() {
    let project = TestProject::new(
        r#"#import "@local/ankify:0.1.0": note
#note("keep", format: "plain", data: (Front: "Q1", Back: "A1"))
#note("drop", format: "plain", data: (Front: "Q2", Back: "A2"))
"#,
    );
    let first = project.sync().await.result.expect("sync 1");
    assert_eq!(first.notes_added, 2);

    // Remove one note from the document.
    project.write(
        r#"#import "@local/ankify:0.1.0": note
#note("keep", format: "plain", data: (Front: "Q1", Back: "A1"))
"#,
    );
    let second = project.sync().await.result.expect("sync 2");

    assert_eq!(
        (
            second.notes_added,
            second.notes_updated,
            second.notes_unchanged,
        ),
        (0, 0, 1),
    );
    assert!(
        second.warnings.iter().any(|w| w.contains("'drop'")),
        "the removed note should be reported as an orphan: {:?}",
        second.warnings,
    );
}

/// Even when every note is removed from the document, the cache entries it used
/// to have are still reported as orphans rather than vanishing silently.
#[tokio::test]
async fn emptying_the_document_reports_every_orphan() {
    let project = TestProject::new(
        r#"#import "@local/ankify:0.1.0": note
#note("first", format: "plain", data: (Front: "Q1", Back: "A1"))
#note("second", format: "plain", data: (Front: "Q2", Back: "A2"))
"#,
    );
    let first = project.sync().await.result.expect("sync 1");
    assert_eq!(first.notes_added, 2);

    // Remove every note from the document.
    project.write(
        r#"#import "@local/ankify:0.1.0": note

= A document that no longer has any flashcards
"#,
    );
    let second = project.sync().await.result.expect("sync 2");

    assert_eq!(
        (
            second.notes_added,
            second.notes_updated,
            second.notes_unchanged,
        ),
        (0, 0, 0),
    );
    for orphan in ["'first'", "'second'"] {
        assert!(
            second.warnings.iter().any(|w| w.contains(orphan)),
            "an emptied document should still report {orphan}: {:?}",
            second.warnings,
        );
    }
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

/// A field's value may be a `(format, value)` dictionary that overrides the
/// note's format per field, and formats may be mixed within a single note.
/// A note's own `deck` overrides the configured default, and several distinct
/// decks in one document each get created.
#[tokio::test]
async fn per_field_formats_and_per_note_decks() {
    let project = TestProject::new(
        r#"#import "@local/ankify:0.1.0": note, configure
#configure(defaults: (deck: "Default Deck"))
#note(
  "mixed",
  deck: "Deck A",
  data: (
    Front: (format: "plain", value: "Plain question"),
    Back: (format: "svg", value: [$a^2 + b^2 = c^2$]),
  ),
)
#note("plain-note", deck: "Deck B", format: "plain", data: (Front: "Q2", Back: "A2"))
"#,
    );
    let outcome = project.sync().await;
    let result = outcome.result.expect("sync should succeed");
    assert_eq!(result.notes_added, 2);
    // Two distinct, non-cached decks are each created.
    assert_eq!(result.decks_created, 2);

    let notes = add_notes(&outcome.requests)["params"]["notes"]
        .as_array()
        .unwrap()
        .clone();
    // Note 1: per-note deck override, plus a plain field and an SVG field
    // mixed within the same note via per-field `(format, value)` dicts.
    assert_eq!(notes[0]["deckName"], "Deck A");
    assert_eq!(notes[0]["fields"]["Front"], "Plain question");
    let back = notes[0]["fields"]["Back"]
        .as_str()
        .expect("Back field value");
    assert!(
        back.contains("<svg"),
        "the svg-format field should be inline SVG"
    );
    assert!(notes[0].get("picture").is_none_or(Value::is_null));
    // Note 2: a different per-note deck.
    assert_eq!(notes[1]["deckName"], "Deck B");
}

#[tokio::test]
async fn document_with_no_notes_is_a_no_op() {
    let project = TestProject::new(
        r#"#import "@local/ankify:0.1.0": configure
#configure(defaults: (deck: "Empty"))

= A document with prose but no flashcards
"#,
    );
    let result = project
        .sync()
        .await
        .result
        .expect("sync should succeed on a document with no notes");
    assert_eq!(
        (
            result.notes_added,
            result.notes_updated,
            result.notes_unchanged
        ),
        (0, 0, 0),
    );
}

/// An empty field must not shift the field-to-image mapping: each non-empty
/// field's image must still be the correct one even when a sibling field
/// renders to nothing.
#[tokio::test]
async fn an_empty_field_does_not_shift_the_mapping() {
    let project = TestProject::new(
        r#"#import "@local/ankify:0.1.0": note, configure
#configure(scale: 1.0, defaults: (deck: "Empty Field"))
#note("ef", format: "svg", data: (
  A: box(width: 60pt, height: 20pt, fill: black),
  B: [],
  C: box(width: 180pt, height: 20pt, fill: black),
))
"#,
    );
    let outcome = project.sync().await;
    outcome.result.expect("sync should succeed");

    let note = &add_notes(&outcome.requests)["params"]["notes"][0];
    // 5 mm of page margin on each side of the content box.
    let margin_pt = 2.0 * 5.0 / 25.4 * 72.0;
    for (field, box_width) in [("A", 60.0), ("C", 180.0)] {
        let svg = note["fields"][field]
            .as_str()
            .expect("inline svg field value");
        let actual = svg_width_pt(svg);
        let want = box_width + margin_pt;
        assert!(
            (actual - want).abs() < 3.0,
            "field {field}: image is {actual:.1}pt wide, expected ~{want:.1}pt \
             — the empty sibling field B shifted the field-to-image mapping",
        );
    }
}
