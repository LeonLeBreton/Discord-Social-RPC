use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Invalid OAuth2 token: {0}")]
    InvalidToken(String),

    #[error("Network error: {0}")]
    Network(String),

    #[error("WebSocket error: {0}")]
    WebSocket(String),

    #[error("Gateway error: {0}")]
    Gateway(String),

    #[error("Failed to resolve external assets: {0}")]
    ExternalAssets(String),

    #[error("Activity already stopped")]
    AlreadyStopped,

    #[error("Client not started. Call start_activity() first")]
    NotStarted,

    #[error("Serialization error: {0}")]
    Serialization(String),

    #[error("Internal runtime error: {0}")]
    Runtime(String),
}

impl From<serde_json::Error> for Error {
    fn from(e: serde_json::Error) -> Self {
        Error::Serialization(e.to_string())
    }
}

impl From<reqwest::Error> for Error {
    fn from(e: reqwest::Error) -> Self {
        Error::Network(e.to_string())
    }
}

impl From<tokio_tungstenite::tungstenite::Error> for Error {
    fn from(e: tokio_tungstenite::tungstenite::Error) -> Self {
        Error::WebSocket(e.to_string())
    }
}