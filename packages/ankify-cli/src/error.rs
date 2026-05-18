// See the file LICENSE for the full license governing this code.

//! Error types and result aliases for the Ankify library.

/// The main error type for the Ankify library.
#[derive(thiserror::Error, Debug)]
pub enum Error {
    /// I/O errors (file operations, network requests, etc.)
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// JSON serialization/deserialization errors
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// HTTP request errors
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    /// Typst execution errors
    #[error("Typst error: {0}")]
    Typst(String),

    /// AnkiConnect API errors
    #[error("AnkiConnect error: {0}")]
    AnkiConnect(String),

    /// Cache/auxiliary file errors
    #[error("Cache error: {0}")]
    Cache(String),

    /// Generic error with custom message
    #[error("{0}")]
    Custom(String),
}

/// Convenience Result type alias for the Ankify library.
pub type Result<T> = std::result::Result<T, Error>;

impl Error {
    /// Create a new custom error with the given message.
    pub fn custom<S: Into<String>>(message: S) -> Self {
        Error::Custom(message.into())
    }

    /// Create a new Typst error with the given message.
    pub fn typst<S: Into<String>>(message: S) -> Self {
        Error::Typst(message.into())
    }

    /// Create a new AnkiConnect error with the given message.
    pub fn anki_connect<S: Into<String>>(message: S) -> Self {
        Error::AnkiConnect(message.into())
    }

    /// Create a new cache error with the given message.
    pub fn cache<S: Into<String>>(message: S) -> Self {
        Error::Cache(message.into())
    }
}
