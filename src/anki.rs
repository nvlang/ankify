// See the file LICENSE for the full license governing this code.

//! AnkiConnect integration and API calls.
//!
//! This module handles communication with the AnkiConnect add-on for Anki,
//! providing type-safe wrappers around the JSON API.

use crate::{types::*, Error, Result};
use reqwest::Client as HttpClient;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// AnkiConnect API client.
#[derive(Debug)]
pub struct Client {
    http_client: HttpClient,
    base_url: String,
}

/// Request structure for AnkiConnect API calls.
#[derive(Debug, Serialize)]
struct AnkiConnectRequest<T> {
    action: String,
    version: u8,
    #[serde(skip_serializing_if = "Option::is_none")]
    params: Option<T>,
}

/// Response structure from AnkiConnect API.
#[derive(Debug, Deserialize)]
struct AnkiConnectResponse<T> {
    result: Option<T>,
    error: Option<String>,
}

/// Parameters for creating a deck.
#[derive(Debug, Serialize)]
struct CreateDeckParams {
    deck: String,
}

/// Parameters for adding a note/card.
#[derive(Debug, Serialize)]
struct AddNoteParams {
    note: AnkiNote,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    picture: Vec<PictureEntry>,
}

/// Picture entry for media file attachments.
#[derive(Debug, Serialize)]
struct PictureEntry {
    #[serde(skip_serializing_if = "Option::is_none")]
    url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<String>,
    filename: String,
    fields: Vec<String>,
}

/// Parameters for updating a note/card.
#[derive(Debug, Serialize)]
struct UpdateNoteParams {
    note: UpdateAnkiNote,
}

/// Parameters for deleting notes.
#[derive(Debug, Serialize)]
struct DeleteNotesParams {
    notes: Vec<String>,
}

/// AnkiConnect note structure for creation.
#[derive(Debug, Serialize)]
struct AnkiNote {
    #[serde(rename = "deckName")]
    deck_name: String,
    #[serde(rename = "modelName")]
    model_name: String,
    fields: HashMap<String, String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    tags: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    options: Option<NoteOptions>,
    #[serde(flatten)]
    rest: HashMap<String, serde_json::Value>,
}

/// Options for note creation.
#[derive(Debug, Serialize)]
struct NoteOptions {
    #[serde(rename = "allowDuplicate")]
    allow_duplicate: bool,
    #[serde(rename = "duplicateScope")]
    duplicate_scope: String,
}

/// AnkiConnect note structure for updates.
#[derive(Debug, Serialize)]
struct UpdateAnkiNote {
    id: String,
    fields: HashMap<String, String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    tags: Vec<String>,
}

impl Client {
    /// Create a new AnkiConnect client.
    ///
    /// # Arguments
    ///
    /// * `base_url` - The base URL for AnkiConnect (e.g., "http://localhost:8765")
    ///
    /// # Returns
    ///
    /// A new Client instance.
    pub fn new(base_url: &str) -> Result<Self> {
        let http_client = HttpClient::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(|e| Error::anki_connect(format!("Failed to create HTTP client: {}", e)))?;

        Ok(Self {
            http_client,
            base_url: base_url.to_string(),
        })
    }

    /// Test the connection to AnkiConnect.
    ///
    /// # Returns
    ///
    /// The AnkiConnect version if successful.
    pub async fn test_connection(&self) -> Result<u32> {
        let request = AnkiConnectRequest {
            action: "version".to_string(),
            version: 6,
            params: None::<()>,
        };

        let response: AnkiConnectResponse<u32> = self.send_request(request).await?;

        if let Some(version) = response.result {
            tracing::info!("Connected to AnkiConnect version {}", version);
            Ok(version)
        } else {
            Err(Error::anki_connect(response.error.unwrap_or_else(|| {
                "Failed to get AnkiConnect version: no result or error returned".to_string()
            })))
        }
    }

    /// Create a new card in Anki.
    ///
    /// # Arguments
    ///
    /// * `card` - The rendered card to create
    ///
    /// # Returns
    ///
    /// The AnkiConnect note ID of the created card.
    pub async fn create_card(&self, card: &RenderedCard) -> Result<String> {
        tracing::debug!("Creating card in deck '{}'", card.deck);

        // Ensure deck exists
        self.ensure_deck_exists(&card.deck).await?;

        // Create the note
        let note = AnkiNote {
            deck_name: card.deck.clone(),
            model_name: card.model.clone(),
            fields: card.data.clone(),
            tags: card.tags.clone(),
            options: Some(NoteOptions {
                allow_duplicate: false,
                duplicate_scope: "deck".to_string(),
            }),
            rest: card.rest.clone(),
        };

        let request = AnkiConnectRequest {
            action: "addNote".to_string(),
            version: 6,
            params: Some(AddNoteParams {
                note,
                picture: Vec::new(), // TODO: Will be populated based on card format
            }),
        };

        let response: AnkiConnectResponse<u64> = self.send_request(request).await?;

        if let Some(note_id) = response.result {
            tracing::info!("Created card with ID {}", note_id);
            Ok(note_id.to_string())
        } else {
            Err(Error::anki_connect(response.error.unwrap_or_else(|| {
                "Failed to create card: no result or error returned".to_string()
            })))
        }
    }

    /// Update an existing card in Anki.
    ///
    /// # Arguments
    ///
    /// * `anki_id` - The AnkiConnect note ID
    /// * `card` - The rendered card with updated content
    pub async fn update_card(&self, anki_id: String, card: &RenderedCard) -> Result<()> {
        tracing::debug!("Updating card {} in deck '{}'", anki_id, card.deck);

        let note = UpdateAnkiNote {
            id: anki_id.clone(),
            fields: card.data.clone(),
            tags: card.tags.clone(),
        };

        let request = AnkiConnectRequest {
            action: "updateNote".to_string(), // Use updateNote instead of updateNoteFields
            version: 6,
            params: Some(UpdateNoteParams { note }),
        };

        let response: AnkiConnectResponse<serde_json::Value> = self.send_request(request).await?;

        if response.error.is_none() {
            tracing::info!("Updated card {}", anki_id);
            Ok(())
        } else {
            Err(Error::anki_connect(response.error.unwrap_or_else(|| {
                format!("Failed to update card {}: error returned", anki_id)
            })))
        }
    }

    /// Delete a card from Anki.
    ///
    /// # Arguments
    ///
    /// * `anki_id` - The AnkiConnect note ID to delete
    pub async fn delete_card(&self, anki_id: String) -> Result<()> {
        tracing::debug!("Deleting card {}", anki_id);

        let request = AnkiConnectRequest {
            action: "deleteNotes".to_string(),
            version: 6,
            params: Some(DeleteNotesParams {
                notes: vec![anki_id.clone()],
            }),
        };

        let response: AnkiConnectResponse<serde_json::Value> = self.send_request(request).await?;

        if response.result.is_some() {
            tracing::info!("Deleted card {}", anki_id);
            Ok(())
        } else {
            Err(Error::anki_connect(response.error.unwrap_or_else(|| {
                format!(
                    "Failed to delete card {}: no result or error returned",
                    anki_id
                )
            })))
        }
    }

    /// Get all deck names from Anki.
    ///
    /// # Returns
    ///
    /// A vector of deck names.
    pub async fn get_deck_names(&self) -> Result<Vec<String>> {
        let request = AnkiConnectRequest {
            action: "deckNames".to_string(),
            version: 6,
            params: None::<()>,
        };

        let response: AnkiConnectResponse<Vec<String>> = self.send_request(request).await?;

        if let Some(deck_names) = response.result {
            Ok(deck_names)
        } else {
            Err(Error::anki_connect(response.error.unwrap_or_else(|| {
                "Failed to get deck names: no result or error returned".to_string()
            })))
        }
    }

    /// Ensure a deck exists, creating it if necessary.
    ///
    /// # Arguments
    ///
    /// * `deck_name` - The name of the deck to ensure exists
    async fn ensure_deck_exists(&self, deck_name: &str) -> Result<()> {
        // Check if deck already exists
        let existing_decks = self.get_deck_names().await?;
        if existing_decks.contains(&deck_name.to_string()) {
            return Ok(());
        }

        // Create the deck
        tracing::info!("Creating deck '{}'", deck_name);

        let request = AnkiConnectRequest {
            action: "createDeck".to_string(),
            version: 6,
            params: Some(CreateDeckParams {
                deck: deck_name.to_string(),
            }),
        };

        let response: AnkiConnectResponse<u64> = self.send_request(request).await?;

        if response.result.is_some() {
            tracing::info!("Created deck '{}'", deck_name);
            Ok(())
        } else {
            Err(Error::anki_connect(response.error.unwrap_or_else(|| {
                format!(
                    "Failed to create deck '{}': no result or error returned",
                    deck_name
                )
            })))
        }
    }

    /// Send a request to AnkiConnect and parse the response.
    async fn send_request<T, R>(
        &self,
        request: AnkiConnectRequest<T>,
    ) -> Result<AnkiConnectResponse<R>>
    where
        T: Serialize,
        R: for<'de> Deserialize<'de>,
    {
        let response = self
            .http_client
            .post(&self.base_url)
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(Error::anki_connect(format!(
                "HTTP error {}: {}",
                response.status(),
                response.text().await.unwrap_or_default()
            )));
        }

        let anki_response: AnkiConnectResponse<R> = response.json().await?;

        if let Some(error) = &anki_response.error {
            return Err(Error::anki_connect(error.clone()));
        }

        Ok(anki_response)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::path::PathBuf;
    use wiremock::{
        matchers::{body_json, body_partial_json, method, path},
        Mock, MockServer, ResponseTemplate,
    };

    async fn setup_mock_server() -> MockServer {
        MockServer::start().await
    }

    #[tokio::test]
    async fn test_create_client() {
        let client = Client::new("http://localhost:8765").unwrap();
        assert_eq!(client.base_url, "http://localhost:8765");
    }

    #[tokio::test]
    async fn test_test_connection_success() {
        let mock_server = setup_mock_server().await;

        Mock::given(method("POST"))
            .and(path("/"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "result": 6,
                "error": null
            })))
            .mount(&mock_server)
            .await;

        let client = Client::new(&mock_server.uri()).unwrap();
        let version = client.test_connection().await.unwrap();
        assert_eq!(version, 6);
    }

    #[tokio::test]
    async fn test_test_connection_error() {
        let mock_server = setup_mock_server().await;

        Mock::given(method("POST"))
            .and(path("/"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "result": null,
                "error": "Connection failed"
            })))
            .mount(&mock_server)
            .await;

        let client = Client::new(&mock_server.uri()).unwrap();
        let result = client.test_connection().await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_create_card_success() {
        let mock_server = setup_mock_server().await;

        // Mock deck names request
        Mock::given(method("POST"))
            .and(path("/"))
            .and(body_json(serde_json::json!({
                "action": "deckNames",
                "version": 6
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "result": ["Test Deck"],
                "error": null
            })))
            .mount(&mock_server)
            .await;

        // Mock add note request
        Mock::given(method("POST"))
            .and(path("/"))
            .and(body_partial_json(serde_json::json!({
                "action": "addNote",
                "version": 6
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "result": 1234567890_u64,
                "error": null
            })))
            .mount(&mock_server)
            .await;

        let client = Client::new(&mock_server.uri()).unwrap();
        let card = RenderedCard {
            model: "Basic".to_string(),
            data: [
                ("Front".to_string(), "Question".to_string()),
                ("Back".to_string(), "Answer".to_string()),
            ]
            .into(),
            deck: "Test Deck".to_string(),
            tags: vec![],
            rest: HashMap::new(),
            label: "test-card-1".to_string(),
            source_file: PathBuf::from("test.typ"),
            media_files: HashMap::new(),
        };

        let note_id = client.create_card(&card).await.unwrap();
        assert_eq!(note_id, "1234567890");
    }

    #[tokio::test]
    async fn test_get_deck_names() {
        let mock_server = setup_mock_server().await;

        Mock::given(method("POST"))
            .and(path("/"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "result": ["Default", "Test Deck", "Another Deck"],
                "error": null
            })))
            .mount(&mock_server)
            .await;

        let client = Client::new(&mock_server.uri()).unwrap();
        let decks = client.get_deck_names().await.unwrap();
        assert_eq!(decks, vec!["Default", "Test Deck", "Another Deck"]);
    }
}
