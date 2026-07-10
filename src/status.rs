/// Represents the current state of the Rich Presence connection.
#[derive(Debug, Clone, PartialEq)]
pub enum ActivityStatus {
    /// Connected to Discord Gateway and presence is being displayed.
    Ok,
    /// Not connected to Discord Gateway.
    Disconnected,
    /// The OAuth2 token was rejected by Discord.
    TokenInvalid,
    /// A network error occurred (timeout, connection refused, etc.).
    NetworkError,
    /// The client was created but start_activity() has not been called yet.
    NotStarted,
    /// A fatal error occurred, or the client has stopped.
    Stopped,
}