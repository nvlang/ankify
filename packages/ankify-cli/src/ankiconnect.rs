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

new_type!(CardId, u64);
new_type!(NoteId, u64);
new_type!(DeckId, u64);

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

new_type!(FieldValue, String);

impl FieldValue {
    pub fn new(value: String) -> Self {
        FieldValue(value)
    }

    pub fn as_str(&self) -> &str {
        &self.0
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

new_type!(Profile, String);
new_type!(Query, String);
new_type!(MediaFilename, String);
new_type!(MediaData, String);
new_type!(MediaPath, String);
new_type!(MediaUrl, String);

// Base request/response structures
#[derive(Debug, Serialize)]
pub struct AnkiRequest<T> {
    action: String,
    version: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    params: Option<T>,
    #[serde(skip_serializing_if = "Option::is_none")]
    key: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct AnkiResponse<T> {
    pub result: Option<T>,
    pub error: Option<String>,
}

// Media file structure
#[derive(Debug, Serialize)]
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
#[derive(Debug, Serialize)]
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

#[derive(Debug, Serialize)]
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

#[derive(Debug, Serialize)]
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

#[derive(Debug, Serialize)]
pub struct DuplicateScopeOptions {
    #[serde(rename = "deckName", skip_serializing_if = "Option::is_none")]
    pub deck_name: Option<String>,
    #[serde(rename = "checkChildren", skip_serializing_if = "Option::is_none")]
    pub check_children: Option<bool>,
    #[serde(rename = "checkAllModels", skip_serializing_if = "Option::is_none")]
    pub check_all_models: Option<bool>,
}

// Response structures
#[derive(Debug, Deserialize)]
pub struct NoteInfo {
    #[serde(rename = "noteId")]
    pub note_id: NoteId,
    pub profile: Profile,
    #[serde(rename = "modelName")]
    pub model_name: Model,
    pub tags: Vec<Tag>,
    pub fields: HashMap<Field, FieldInfo>,
    pub cards: Vec<CardId>,
    pub r#mod: u64,
}

#[derive(Debug, Deserialize)]
pub struct FieldInfo {
    pub value: FieldValue,
    pub order: u32,
}

#[derive(Debug, Deserialize)]
pub struct CardInfo {
    pub answer: String,
    pub question: String,
    #[serde(rename = "deckName")]
    pub deck_name: Deck,
    #[serde(rename = "modelName")]
    pub model_name: Model,
    #[serde(rename = "fieldOrder")]
    pub field_order: u32,
    pub fields: HashMap<String, FieldInfo>,
    pub css: String,
    #[serde(rename = "cardId")]
    pub card_id: CardId,
    pub interval: u32,
    pub note: NoteId,
    pub ord: u32,
    pub r#type: u32,
    pub queue: u32,
    pub due: u32,
    pub reps: u32,
    pub lapses: u32,
    pub left: u32,
    #[serde(rename = "mod")]
    pub modified: u64,
}

#[derive(Debug, Deserialize)]
pub struct PermissionResponse {
    pub permission: String,
    #[serde(rename = "requireApiKey")]
    pub require_api_key: Option<bool>,
    pub version: Option<u32>,
}

// API Actions enum
#[derive(Debug)]
pub enum AnkiAction {
    // Card Actions
    FindCards {
        query: Query,
    },
    CardsInfo {
        cards: Vec<CardId>,
    },
    CardsToNotes {
        cards: Vec<CardId>,
    },
    SetDueDate {
        cards: Vec<CardId>,
        days: String,
    },

    // Deck Actions
    DeckNames,
    DeckNamesAndIds,
    CreateDeck {
        deck: String,
    },
    ChangeDeck {
        cards: Vec<CardId>,
        deck: Deck,
    },
    DeleteDecks {
        decks: Vec<String>,
        cards_too: bool,
    },

    // Note Actions
    AddNote {
        note: Note,
    },
    AddNotes {
        notes: Vec<Note>,
    },
    CanAddNotes {
        notes: Vec<Note>,
    },
    UpdateNote {
        note: NoteUpdate,
    },
    UpdateNoteFields {
        note: NoteUpdate,
    },
    UpdateNoteTags {
        note: NoteId,
        tags: Vec<Tag>,
    },
    DeleteNotes {
        notes: Vec<NoteId>,
    },
    FindNotes {
        query: Query,
    },
    NotesInfo {
        notes: Vec<NoteId>,
    },
    GetTags,

    // Model Actions
    ModelNames,

    // Media Actions
    StoreMediaFile {
        filename: String,
        data: Option<String>,
        path: Option<String>,
        url: Option<String>,
    },
    RetrieveMediaFile {
        filename: String,
    },
    DeleteMediaFile {
        filename: String,
    },
    GetMediaDirPath,

    // Miscellaneous Actions
    RequestPermission,
    Version,
    Sync,
    GetProfiles,
    GetActiveProfile,
    LoadProfile {
        name: String,
    },
    ReloadCollection,
}

// Main client
pub struct AnkiConnect {
    url: String,
    api_key: Option<String>,
}

impl AnkiConnect {
    pub fn new() -> Self {
        Self {
            url: DEFAULT_URL.to_string(),
            api_key: None,
        }
    }

    pub fn with_url(url: String) -> Self {
        Self { url, api_key: None }
    }

    pub fn with_api_key(mut self, api_key: String) -> Self {
        self.api_key = Some(api_key);
        self
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
            key: self.api_key.clone(),
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
            AnkiAction::FindCards { query } => {
                self.build_request("findCards", Some(serde_json::json!({ "query": query })))
            }
            AnkiAction::CardsInfo { cards } => {
                self.build_request("cardsInfo", Some(serde_json::json!({ "cards": cards })))
            }
            AnkiAction::CardsToNotes { cards } => {
                self.build_request("cardsToNotes", Some(serde_json::json!({ "cards": cards })))
            }
            AnkiAction::SetDueDate { cards, days } => self.build_request(
                "setDueDate",
                Some(serde_json::json!({ "cards": cards, "days": days })),
            ),
            AnkiAction::DeckNames => self.build_request::<()>("deckNames", None),
            AnkiAction::DeckNamesAndIds => self.build_request::<()>("deckNamesAndIds", None),
            AnkiAction::CreateDeck { deck } => {
                self.build_request("createDeck", Some(serde_json::json!({ "deck": deck })))
            }
            AnkiAction::ChangeDeck { cards, deck } => self.build_request(
                "changeDeck",
                Some(serde_json::json!({ "cards": cards, "deck": deck })),
            ),
            AnkiAction::DeleteDecks { decks, cards_too } => self.build_request(
                "deleteDecks",
                Some(serde_json::json!({ "decks": decks, "cardsToo": cards_too })),
            ),
            AnkiAction::AddNote { note } => {
                self.build_request("addNote", Some(serde_json::json!({ "note": note })))
            }
            AnkiAction::AddNotes { notes } => {
                self.build_request("addNotes", Some(serde_json::json!({ "notes": notes })))
            }
            AnkiAction::CanAddNotes { notes } => {
                self.build_request("canAddNotes", Some(serde_json::json!({ "notes": notes })))
            }
            AnkiAction::UpdateNote { note } => {
                self.build_request("updateNote", Some(serde_json::json!({ "note": note })))
            }
            AnkiAction::UpdateNoteFields { note } => self.build_request(
                "updateNoteFields",
                Some(serde_json::json!({ "note": note })),
            ),
            AnkiAction::UpdateNoteTags { note, tags } => self.build_request(
                "updateNoteTags",
                Some(serde_json::json!({ "note": note, "tags": tags })),
            ),
            AnkiAction::DeleteNotes { notes } => {
                self.build_request("deleteNotes", Some(serde_json::json!({ "notes": notes })))
            }
            AnkiAction::FindNotes { query } => {
                self.build_request("findNotes", Some(serde_json::json!({ "query": query })))
            }
            AnkiAction::NotesInfo { notes } => {
                self.build_request("notesInfo", Some(serde_json::json!({ "notes": notes })))
            }
            AnkiAction::GetTags => self.build_request::<()>("getTags", None),
            AnkiAction::ModelNames => self.build_request::<()>("modelNames", None),
            AnkiAction::StoreMediaFile {
                filename,
                data,
                path,
                url,
            } => {
                let mut params = serde_json::json!({ "filename": filename });
                if let Some(data) = data {
                    params["data"] = serde_json::Value::String(data);
                }
                if let Some(path) = path {
                    params["path"] = serde_json::Value::String(path);
                }
                if let Some(url) = url {
                    params["url"] = serde_json::Value::String(url);
                }
                self.build_request("storeMediaFile", Some(params))
            }
            AnkiAction::RetrieveMediaFile { filename } => self.build_request(
                "retrieveMediaFile",
                Some(serde_json::json!({ "filename": filename })),
            ),
            AnkiAction::DeleteMediaFile { filename } => self.build_request(
                "deleteMediaFile",
                Some(serde_json::json!({ "filename": filename })),
            ),
            AnkiAction::GetMediaDirPath => self.build_request::<()>("getMediaDirPath", None),
            AnkiAction::RequestPermission => self.build_request::<()>("requestPermission", None),
            AnkiAction::Version => self.build_request::<()>("version", None),
            AnkiAction::Sync => self.build_request::<()>("sync", None),
            AnkiAction::GetProfiles => self.build_request::<()>("getProfiles", None),
            AnkiAction::GetActiveProfile => self.build_request::<()>("getActiveProfile", None),
            AnkiAction::LoadProfile { name } => {
                self.build_request("loadProfile", Some(serde_json::json!({ "name": name })))
            }
            AnkiAction::ReloadCollection => self.build_request::<()>("reloadCollection", None),
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
