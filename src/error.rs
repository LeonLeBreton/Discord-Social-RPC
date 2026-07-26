use thiserror::Error;

/// Errors that can occur during Discord RPC operations.
#[derive(Error, Debug)]
pub enum Error {
    /// The `OAuth2` token is invalid or was rejected by Discord.
    #[error("Invalid OAuth2 token: {0}")]
    InvalidToken(String),

    /// A network-level error occurred.
    #[error("Network error: {0}")]
    Network(String),

    /// WebSocket connection error.
    #[error("WebSocket error: {0}")]
    WebSocket(String),

    /// Discord Gateway protocol error.
    #[error("Gateway error: {0}")]
    Gateway(String),

    /// Failed to resolve external image assets.
    #[error("External assets error: {0}")]
    ExternalAssets(String),

    /// The activity was already stopped.
    #[error("Activity already stopped")]
    AlreadyStopped,

    /// `start_activity()` was not called before this operation.
    #[error("Client not started. Call start_activity() first")]
    NotStarted,

    /// JSON serialization error.
    #[error("Serialization error: {0}")]
    Serialization(String),

    /// Internal Tokio runtime error.
    #[error("Runtime error: {0}")]
    Runtime(String),
}

impl From<serde_json::Error> for Error {
    fn from(e: serde_json::Error) -> Self {
        Self::Serialization(e.to_string())
    }
}

impl From<reqwest::Error> for Error {
    fn from(e: reqwest::Error) -> Self {
        Self::Network(e.to_string())
    }
}

impl From<tokio_tungstenite::tungstenite::Error> for Error {
    fn from(e: tokio_tungstenite::tungstenite::Error) -> Self {
        Self::WebSocket(e.to_string())
    }
}