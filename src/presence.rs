/// User presence status on Discord.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PresenceStatus {
    Online,
    Idle,
    DoNotDisturb,
    Invisible,
}

impl PresenceStatus {
    /// Returns the wire format string for this status.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Online => "online",
            Self::Idle => "idle",
            Self::DoNotDisturb => "dnd",
            Self::Invisible => "invisible",
        }
    }
}