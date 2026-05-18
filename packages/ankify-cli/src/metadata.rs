//! Module describing the structure of the metadata that the `query` module
//! should return.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Configuration, as specified by the metadata embedded in the Typst file and
/// queried by the `query` module.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TypstAnkifyConfiguration {
    /// AnkiConnect URL.
    ///
    /// Default: `"http://127.0.0.1:8765"`
    #[serde(rename = "ankiconnect-url")]
    pub ankiconnect_url: Option<String>,

    /// Whether to print verbose logs.
    ///
    /// Default: `false`
    pub verbose: Option<bool>,

    /// Setup function (stored as opaque value since it's a function).
    ///
    /// Default: page setup function
    pub setup: Option<serde_json::Value>,

    /// Cache options.
    pub cache: Option<CacheOptions>,

    /// Options controlling which checks to perform. By default, all checks are
    /// enabled.
    pub checks: Option<Checks>,

    /// Default values for notes.
    pub defaults: Option<NoteDefaults>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CacheOptions {
    /// Whether the cache mechanism should be used.
    ///
    /// Default: `true`
    pub enabled: Option<bool>,

    /// If specified, this will override the default cache file.
    ///
    /// Note: a path to a JSON file (absolute or relative to the Typst file
    /// being processed) is expected. If the JSON file doesn't exist, it will
    /// be created.
    ///
    /// Default: `none`
    #[serde(rename = "custom-file")]
    pub custom_file: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Checks {
    /// Checks to perform within Typst itself. These are mainly type-safety
    /// related checks.
    ///
    /// These checks have no bearing on the Ankify crate, and as such can be
    /// safely ignored. They're mentioned here for completeness's sake.
    ///
    /// By default, all checks are enabled.
    pub typst: Option<TypstChecks>,

    /// Checks to perform within the Ankify crate by interacting with
    /// AnkiConnect.
    ///
    /// By default, all checks are enabled. However, since these checks
    /// may be expensive at times, it can make sense to disable them to speed up
    /// processing if you're sure that notes in your Typst file aren't
    ///
    /// -   referencing decks that don't exist,
    /// -   referencing models that don't exist,
    /// -   referencing tags that don't exist, or
    /// -   referencing fields that don't exist in the specified model.
    pub ankiconnect: Option<AnkiConnectChecks>,
}

/// Controls whether Typst will perform validation checks.
///
/// This includes data dictionary validation and format validation.
pub type TypstChecks = bool;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AnkiConnectChecks {
    /// Whether to check if the specified model exists in Anki.
    ///
    /// Default: `true`
    pub model: Option<bool>,

    /// Whether to check if the specified deck exists in Anki.
    ///
    /// Default: `true`
    pub deck: Option<bool>,

    /// Whether to check if the specified tags exist in Anki.
    ///
    /// Default: `true`
    pub tags: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Note {
    /// The label of the note, as parsed from the Typst file's metadata.
    pub label: String,

    /// The model to use for this note.
    pub model: String,

    /// The data dictionary for this note.
    pub data: HashMap<String, NoteDataValue>,

    /// The deck to which this note belongs.
    pub deck: String,

    /// Tags associated with this note.
    pub tags: Vec<String>,

    /// Additional fields that may be present in the metadata.
    ///
    /// Values may be anything, but I'm not sure how to best represent that in
    /// Rust's type system.
    pub other: serde_json::Value,

    /// The format in which the note's fields should be rendered by default.
    pub format: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletedNote {
    /// The label of the note, as parsed from the Typst file's metadata.
    pub label: String,

    /// The model to use for this note.
    pub model: String,

    /// The data dictionary for this note.
    pub data: HashMap<String, CompletedNoteDataValueWithFormat>,

    /// The deck to which this note belongs.
    pub deck: String,

    /// Tags associated with this note.
    pub tags: Vec<String>,

    /// Additional fields that may be present in the metadata.
    ///
    /// Values may be anything, but I'm not sure how to best represent that in
    /// Rust's type system.
    pub other: serde_json::Value,

    /// The format in which the note's fields should be rendered by default.
    pub format: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum NoteDataValue {
    /// The value of the data field, as a simple string.
    Simple(Option<String>),

    /// The value of the data field, with an optional format.
    WithFormat(NoteDataValueWithFormat),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NoteDataValueWithFormat {
    /// The value of the data field.
    pub value: Option<String>,

    /// The format in which the value should be rendered.
    ///
    /// This is typically one of `"svg"`, `"png"`, or `"plain"`.
    pub format: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompletedNoteDataValueWithFormat {
    /// The value of the data field.
    pub value: Option<String>,

    /// The format in which the value should be rendered.
    ///
    /// This is typically one of `"svg"`, `"png"`, or `"plain"`.
    pub format: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NoteDefaults {
    /// The default model to use for notes.
    ///
    /// Default: `"Basic"`
    pub model: Option<String>,

    /// The default deck to which to add notes.
    ///
    /// Default: `"Default"`
    pub deck: Option<String>,

    /// Default tags to apply to notes.
    ///
    /// Default: `[]`
    pub tags: Option<Vec<String>>,

    /// Additional data that may be present in the note metadata by default.
    ///
    /// Values may be anything, but I'm not sure how to best represent that in
    /// Rust's type system.
    ///
    /// Default: `none`
    pub other: Option<serde_json::Value>,

    /// The format in which the note's fields should be rendered by default.
    ///
    /// Default: `"png"`
    pub format: Option<String>,

    /// Default render function (stored as opaque value since it's a function).
    ///
    /// Default: identity function that returns field content
    pub render: Option<serde_json::Value>,
}

/// Configuration, as specified by the metadata embedded in the Typst file and
/// queried by the `query` module.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CompletedTypstAnkifyConfiguration {
    /// AnkiConnect URL.
    ///
    /// Default: `"http://127.0.0.1:8765"`
    #[serde(rename = "ankiconnect-url")]
    pub ankiconnect_url: String,

    /// Whether to print verbose logs.
    ///
    /// Default: `false`
    pub verbose: bool,

    /// Setup function (stored as opaque value since it's a function).
    ///
    /// Default: page setup function
    pub setup: serde_json::Value,

    /// Cache options.
    pub cache: CacheOptions,

    /// Options controlling which checks to perform. By default, all checks are
    /// enabled.
    pub checks: Checks,

    /// Default values for notes.
    pub defaults: NoteDefaults,
}
