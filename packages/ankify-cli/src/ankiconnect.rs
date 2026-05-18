//! AnkiConnect API client.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

const API_VERSION: u32 = 6;
const DEFAULT_URL: &str = "http://127.0.0.1:8765";

macro_rules! new_type {
    ($name:ident, $inner:ty) => {
        #[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
        pub struct $name(pub $inner);
    };
}

new_type!(NoteId, u64);

/// A string which, if checks are turned on, is guaranteed to be a valid Anki
/// model name.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Model(String);

impl Model {
    pub fn new(name: String) -> Self {
        Model(name)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// A string which, if checks are turned on, is guaranteed to be a valid Anki
/// deck name.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Deck(String);

impl Deck {
    pub fn new(name: String) -> Self {
        Deck(name)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// A string which, if checks are turned on, is guaranteed to be a valid Anki
/// field name.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Field(String);

impl Field {
    pub fn new(name: String) -> Self {
        Field(name)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

new_type!(FieldValue, Option<String>);

impl FieldValue {
    pub fn new(value: Option<String>) -> Self {
        FieldValue(value)
    }
}

/// A string which, if checks are turned on, is guaranteed to be a valid Anki
/// tag name.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Tag(String);

impl Tag {
    pub fn new(name: String) -> Self {
        Tag(name)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

// Base request/response structures
#[derive(Debug, Serialize)]
pub struct AnkiRequest<T> {
    action: String,
    version: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    params: Option<T>,
}

#[derive(Debug, Deserialize)]
pub struct AnkiResponse<T> {
    pub result: Option<T>,
    pub error: Option<String>,
}

// Media file structure
#[derive(Debug, Clone, Serialize)]
pub struct MediaFile {
    pub filename: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub skip_hash: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fields: Option<Vec<Field>>,
}

// Note structures
#[derive(Debug, Clone, Serialize)]
pub struct Note {
    #[serde(rename = "deckName")]
    pub deck_name: Deck,
    #[serde(rename = "modelName")]
    pub model_name: Model,
    pub fields: HashMap<Field, FieldValue>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<Tag>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub options: Option<NoteOptions>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub audio: Option<Vec<MediaFile>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub video: Option<Vec<MediaFile>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub picture: Option<Vec<MediaFile>>,
}

#[derive(Debug, Clone, Serialize)]
pub struct NoteUpdate {
    pub id: NoteId,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fields: Option<HashMap<Field, FieldValue>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<Tag>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "modelName")]
    pub model_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub audio: Option<Vec<MediaFile>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub video: Option<Vec<MediaFile>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub picture: Option<Vec<MediaFile>>,
}

#[derive(Debug, Clone, Serialize)]
pub struct NoteOptions {
    #[serde(rename = "allowDuplicate", skip_serializing_if = "Option::is_none")]
    pub allow_duplicate: Option<bool>,
    #[serde(rename = "duplicateScope", skip_serializing_if = "Option::is_none")]
    pub duplicate_scope: Option<String>,
    #[serde(
        rename = "duplicateScopeOptions",
        skip_serializing_if = "Option::is_none"
    )]
    pub duplicate_scope_options: Option<DuplicateScopeOptions>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DuplicateScopeOptions {
    #[serde(rename = "deckName", skip_serializing_if = "Option::is_none")]
    pub deck_name: Option<String>,
    #[serde(rename = "checkChildren", skip_serializing_if = "Option::is_none")]
    pub check_children: Option<bool>,
    #[serde(rename = "checkAllModels", skip_serializing_if = "Option::is_none")]
    pub check_all_models: Option<bool>,
}

/// The AnkiConnect actions this tool issues.
#[derive(Debug)]
pub enum AnkiAction {
    /// Create a deck (a no-op if it already exists).
    CreateDeck { deck: String },
    /// Add a batch of new notes.
    AddNotes { notes: Vec<Note> },
    /// Update an existing note's fields and tags.
    UpdateNote { note: NoteUpdate },
    /// List the names of every deck.
    DeckNames,
    /// List the names of every note type (model).
    ModelNames,
    /// List every tag in the collection.
    GetTags,
    /// Report the AnkiConnect API version.
    Version,
}

// Main client
pub struct AnkiConnect {
    url: String,
}

impl AnkiConnect {
    pub fn new() -> Self {
        Self {
            url: DEFAULT_URL.to_string(),
        }
    }

    pub fn with_url(url: String) -> Self {
        Self { url }
    }

    pub fn build_request<T: Serialize>(
        &self,
        action: &str,
        params: Option<T>,
    ) -> serde_json::Value {
        let request = AnkiRequest {
            action: action.to_string(),
            version: API_VERSION,
            params,
        };
        serde_json::to_value(request).expect("Failed to serialize request")
    }

    pub fn parse_response<T: for<'de> Deserialize<'de>>(
        &self,
        response: &str,
    ) -> Result<T, AnkiError> {
        let parsed: AnkiResponse<T> =
            serde_json::from_str(response).map_err(|e| AnkiError::ParseError(e.to_string()))?;

        if let Some(error) = parsed.error {
            return Err(AnkiError::ApiError(error));
        }

        parsed.result.ok_or(AnkiError::NoResult)
    }

    pub fn action_to_request(&self, action: AnkiAction) -> serde_json::Value {
        match action {
            AnkiAction::CreateDeck { deck } => {
                self.build_request("createDeck", Some(serde_json::json!({ "deck": deck })))
            }
            AnkiAction::AddNotes { notes } => {
                self.build_request("addNotes", Some(serde_json::json!({ "notes": notes })))
            }
            AnkiAction::UpdateNote { note } => {
                self.build_request("updateNote", Some(serde_json::json!({ "note": note })))
            }
            AnkiAction::DeckNames => self.build_request::<()>("deckNames", None),
            AnkiAction::ModelNames => self.build_request::<()>("modelNames", None),
            AnkiAction::GetTags => self.build_request::<()>("getTags", None),
            AnkiAction::Version => self.build_request::<()>("version", None),
        }
    }

    pub fn url(&self) -> &str {
        &self.url
    }
}

#[derive(Debug, thiserror::Error)]
pub enum AnkiError {
    #[error("API error: {0}")]
    ApiError(String),
    #[error("Parse error: {0}")]
    ParseError(String),
    #[error("No result in response")]
    NoResult,
}

impl Default for AnkiConnect {
    fn default() -> Self {
        Self::new()
    }
}
