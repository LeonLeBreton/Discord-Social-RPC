/// Discord activity type as defined by the Gateway API.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ActivityType {
    Playing = 0,
    Listening = 2,
    Watching = 3,
    Competing = 5,
}

impl ActivityType {
    pub fn code(self) -> u32 {
        self as u32
    }
}

impl Default for ActivityType {
    fn default() -> Self {
        ActivityType::Listening
    }
}

/// Timestamps for the activity (start and/or end).
#[derive(Debug, Clone, Default)]
pub struct Timestamps {
    /// Unix timestamp in milliseconds for activity start.
    pub start: Option<i64>,
    /// Unix timestamp in milliseconds for activity end.
    pub end: Option<i64>,
}

impl Timestamps {
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the start timestamp (unix epoch milliseconds).
    pub fn start(mut self, ms: i64) -> Self {
        self.start = Some(ms);
        self
    }

    /// Set the end timestamp (unix epoch milliseconds).
    pub fn end(mut self, ms: i64) -> Self {
        self.end = Some(ms);
        self
    }
}

/// Assets (images) for the activity.
///
/// Images can be:
/// - Pre-registered on Discord's developer portal: use `large_image("mp:your_image_name")`
/// - External URLs: use `large_image_external("https://...")` which will be resolved
///   automatically via Discord's external assets API.
#[derive(Debug, Clone, Default)]
pub struct Assets {
    /// Large image asset key (either "mp:..." or an external URL).
    pub large_image: Option<String>,
    /// Text displayed when hovering over the large image.
    pub large_text: Option<String>,
    /// Small image asset key (either "mp:..." or an external URL).
    pub small_image: Option<String>,
    /// Text displayed when hovering over the small image.
    pub small_text: Option<String>,
    /// Internal flag: if true, large_image is an external URL that needs resolution.
    pub(crate) large_image_external: bool,
    /// Internal flag: if true, small_image is an external URL that needs resolution.
    pub(crate) small_image_external: bool,
}

impl Assets {
    pub fn new() -> Self {
        Self::default()
    }

    /// Set a pre-registered large image by its `mp:` key.
    pub fn large_image(mut self, key: &str) -> Self {
        self.large_image = Some(key.to_string());
        self.large_image_external = !key.starts_with("mp:");
        self
    }

    /// Set the large image text.
    pub fn large_text(mut self, text: &str) -> Self {
        self.large_text = Some(text.to_string());
        self
    }

    /// Set a pre-registered small image by its `mp:` key.
    pub fn small_image(mut self, key: &str) -> Self {
        self.small_image = Some(key.to_string());
        self.small_image_external = !key.starts_with("mp:");
        self
    }

    /// Set the small image text.
    pub fn small_text(mut self, text: &str) -> Self {
        self.small_text = Some(text.to_string());
        self
    }
}

/// A rich presence activity to display on Discord.
///
/// Built using the builder pattern:
/// ```no_run
/// use discord_social_rpc::Activity;
///
/// let activity = Activity::new()
///     .state("Playing Rust")
///     .details("Building a library")
///     .activity_type(discord_social_rpc::ActivityType::Listening);
/// ```
#[derive(Debug, Clone, Default)]
pub struct Activity {
    /// The activity's name (shown as the first line).
    pub(crate) name: String,
    /// The activity's type.
    pub(crate) activity_type: ActivityType,
    /// State string (second line).
    pub(crate) state: Option<String>,
    /// Details string (first line, below name).
    pub(crate) details: Option<String>,
    /// Assets (images) for the activity.
    pub(crate) assets: Option<Assets>,
    /// Timestamps for the activity.
    pub(crate) timestamps: Option<Timestamps>,
}

impl Activity {
    pub fn new() -> Self {
        Self {
            name: String::new(),
            activity_type: ActivityType::Listening,
            state: None,
            details: None,
            assets: None,
            timestamps: None,
        }
    }

    /// Set the activity name. Shown as the top line on Discord.
    pub fn name(mut self, name: &str) -> Self {
        self.name = name.to_string();
        self
    }

    /// Set the activity type (Listening, Playing, Watching, Competing).
    pub fn activity_type(mut self, t: ActivityType) -> Self {
        self.activity_type = t;
        self
    }

    /// Set the state string (second line in the rich presence).
    pub fn state(mut self, state: &str) -> Self {
        self.state = Some(state.to_string());
        self
    }

    /// Set the details string (first line in the rich presence, below the name).
    pub fn details(mut self, details: &str) -> Self {
        self.details = Some(details.to_string());
        self
    }

    /// Set the assets (images) for this activity.
    pub fn assets(mut self, assets: Assets) -> Self {
        self.assets = Some(assets);
        self
    }

    /// Set the timestamps for this activity.
    pub fn timestamps(mut self, ts: Timestamps) -> Self {
        self.timestamps = Some(ts);
        self
    }
}