/// Represents the current state of the Discord Rich Presence connection.
#[derive(Debug, Clone, PartialEq)]
pub enum ActivityStatus {
    /// Connected to Discord Gateway and presence is displayed.
    Ok,
    
    /// Not connected to Discord Gateway.
    Disconnected,
    
    /// OAuth2 token was rejected by Discord.
    TokenInvalid,
    
    /// A network error occurred (timeout, connection refused).
    NetworkError,
    
    /// Client created but `start_activity()` has not been called.
    NotStarted,

    /// Connection stopped (after `stop_activity()`).
    Stopped,
}