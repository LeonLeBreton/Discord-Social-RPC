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
#[derive(Debug, Clone, Default)]
pub struct Timestamps {
    pub start: Option<i64>,
    pub end: Option<i64>,
}

impl Timestamps {
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the start timestamp (Unix epoch milliseconds).
    pub fn start(mut self, ms: i64) -> Self {
        self.start = Some(ms);
        self
    }

    /// Set the end timestamp (Unix epoch milliseconds).
    pub fn end(mut self, ms: i64) -> Self {
        self.end = Some(ms);
        self
    }
}

/// Assets (images) for a Discord activity.
///
/// Images can be either:
/// - **Pre-registered**: Use `large_image("mp:your_image_name")`
/// - **External URL**: Use `large_image("https://...")` — resolved automatically
#[derive(Debug, Clone, Default)]
pub struct Assets {
    pub large_image: Option<String>,
    pub large_text: Option<String>,
    pub small_image: Option<String>,
    pub small_text: Option<String>,
    pub(crate) large_image_external: bool,
    pub(crate) small_image_external: bool,
}

impl Assets {
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the large image. If the key starts with `mp:` it's treated as
    /// a pre-registered asset; otherwise it's resolved as an external URL.
    pub fn large_image(mut self, key: &str) -> Self {
        self.large_image = Some(key.to_string());
        self.large_image_external = !key.starts_with("mp:");
        self
    }

    /// Set the text displayed on hover over the large image.
    pub fn large_text(mut self, text: &str) -> Self {
        self.large_text = Some(text.to_string());
        self
    }

    /// Set the small image. See `large_image` for URL handling.
    pub fn small_image(mut self, key: &str) -> Self {
        self.small_image = Some(key.to_string());
        self.small_image_external = !key.starts_with("mp:");
        self
    }

    /// Set the text displayed on hover over the small image.
    pub fn small_text(mut self, text: &str) -> Self {
        self.small_text = Some(text.to_string());
        self
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
///     .state("Playing Rust")
///     .details("Building a library")
///     .activity_type(ActivityType::Listening)
///     .assets(Assets::new().large_image("mp:rust_logo"))
///     .timestamps(Timestamps::new().start(1234567890000));
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
    pub fn name(mut self, name: &str) -> Self {
        self.name = name.to_string();
        self
    }

    /// Set the activity type.
    pub fn activity_type(mut self, t: ActivityType) -> Self {
        self.activity_type = t;
        self
    }

    /// Set the state string (second line on Discord).
    pub fn state(mut self, state: &str) -> Self {
        self.state = Some(state.to_string());
        self
    }

    /// Set the details string (first line below the name).
    pub fn details(mut self, details: &str) -> Self {
        self.details = Some(details.to_string());
        self
    }

    /// Set the activity assets (images).
    pub fn assets(mut self, assets: Assets) -> Self {
        self.assets = Some(assets);
        self
    }

    /// Set the activity timestamps.
    pub fn timestamps(mut self, ts: Timestamps) -> Self {
        self.timestamps = Some(ts);
        self
    }
}