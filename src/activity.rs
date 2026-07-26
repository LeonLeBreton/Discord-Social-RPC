/// Discord activity type as defined by Gateway API.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum ActivityType {
    #[default]
    Playing = 0,
    Listening = 2,
    Watching = 3,
    Competing = 5,
}

impl ActivityType {
    /// Returns the numeric code for this activity type.
    pub fn code(self) -> u32 {
        self as u32
    }
}

/// Timestamps for an activity (start and/or end).
///
/// Times are Unix epoch timestamps in milliseconds.
#[derive(Debug, Clone, Default, Copy)]
pub struct Timestamps {
    pub(crate) start: Option<i64>,
    pub(crate) end: Option<i64>,
}

impl Timestamps {
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the start timestamp (Unix epoch milliseconds).
    pub fn set_start(mut self, ms: i64) -> Self {
        self.start = Some(ms);
        self
    }

    /// Set the end timestamp (Unix epoch milliseconds).
    pub fn set_end(mut self, ms: i64) -> Self {
        self.end = Some(ms);
        self
    }
    
    pub fn start(&self) -> i64 {
        self.start.unwrap_or_default()
    }
    
    pub fn end(&self) -> i64 {
        self.end.unwrap_or_default()
    }   
}

/// Assets (images) for a Discord activity.
///
/// Images can be either:
/// - **External URL**: Use `large_image("https://...")` — resolved automatically via Discord's external assets API to an `mp:external/{hash}` path
/// - **Already resolved**: Use `large_image("mp:external/{hash}")` — if you already have a resolved path from a previous call
#[derive(Debug, Clone, Default)]
pub struct Assets {
    pub(crate) large_image: Option<String>,
    pub(crate) large_text: Option<String>,
    pub(crate) small_image: Option<String>,
    pub(crate) small_text: Option<String>,
    pub(crate) large_image_external: bool,
    pub(crate) small_image_external: bool,
}

impl Assets {
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the large image. If the key starts with `mp:` it's treated as
    /// an already-resolved path (e.g. `mp:external/{hash}`); otherwise it's resolved via Discord's external assets API.
    pub fn set_large_image(mut self, key: &str) -> Self {
        self.large_image = Some(key.to_string());
        self.large_image_external = !key.starts_with("mp:");
        self
    }

    /// Set the text displayed on hover over the large image.
    pub fn set_large_text(mut self, text: &str) -> Self {
        self.large_text = Some(text.to_string());
        self
    }

    /// Set the small image. Same behavior as `set_large_image`.
    pub fn set_small_image(mut self, key: &str) -> Self {
        self.small_image = Some(key.to_string());
        self.small_image_external = !key.starts_with("mp:");
        self
    }

    /// Set the text displayed on hover over the small image.
    pub fn set_small_text(mut self, text: &str) -> Self {
        self.small_text = Some(text.to_string());
        self
    }

    pub fn large_image(&self) -> String {
        self.large_image.clone().unwrap_or_default()
    }

    pub fn large_text(&self) -> String {
        self.large_text.clone().unwrap_or_default()
    }

    pub fn small_image(&self) -> String {
        self.small_image.clone().unwrap_or_default()
    }

    pub fn small_text(&self) -> String {
        self.small_text.clone().unwrap_or_default()
    }
}

/// A Discord Rich Presence activity, built using the builder pattern.
///
/// # Example
///
/// ```no_run
/// use discord_social_rpc::{Activity, ActivityType, Assets, Timestamps};
///
/// let activity = Activity::new()
///     .set_state("Playing Rust")
///     .set_details("Building a library")
///     .set_activity_type(ActivityType::Listening)
///     .set_assets(Assets::new().set_large_image("https://example.com/rust_logo.png"))
///     .set_timestamps(Timestamps::new().set_start(1234567890000));
/// ```
#[derive(Debug, Clone, Default)]
pub struct Activity {
    pub(crate) name: String,
    pub(crate) activity_type: ActivityType,
    pub(crate) state: Option<String>,
    pub(crate) details: Option<String>,
    pub(crate) assets: Option<Assets>,
    pub(crate) timestamps: Option<Timestamps>,
}

impl Activity {
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the activity name (top line on Discord).
    pub fn set_name(mut self, name: &str) -> Self {
        self.name = name.to_string();
        self
    }

    /// Set the activity type.
    pub fn set_activity_type(mut self, t: ActivityType) -> Self {
        self.activity_type = t;
        self
    }

    /// Set the state string (second line on Discord).
    pub fn set_state(mut self, state: &str) -> Self {
        self.state = Some(state.to_string());
        self
    }

    /// Set the details string (first line below the name).
    pub fn set_details(mut self, details: &str) -> Self {
        self.details = Some(details.to_string());
        self
    }

    /// Set the activity assets (images).
    pub fn set_assets(mut self, assets: Assets) -> Self {
        self.assets = Some(assets);
        self
    }

    /// Set the activity timestamps.
    pub fn set_timestamps(mut self, ts: Timestamps) -> Self {
        self.timestamps = Some(ts);
        self
    }
    
    pub fn name(&self) -> &str {
        &self.name
    }
    
    pub fn activity_type(&self) -> ActivityType {
        self.activity_type
    }
    
    pub fn state(&self) -> String {
        self.state.clone().unwrap_or_default()
    }
    
    pub fn details(&self) -> String {
        self.details.clone().unwrap_or_default()
    }
    
    pub fn assets(&self) -> Assets {
        self.assets.clone().unwrap_or_default()
    }
    
    pub fn timestamps(&self) -> Timestamps {
        self.timestamps.unwrap_or_default()
    }
}