use serde::Serialize;
use crate::activity::{Activity, Assets, Timestamps};

/// User status on Discord.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PresenceStatus {
    Online,
    Idle,
    DoNotDisturb,
    Invisible,
}

impl PresenceStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            PresenceStatus::Online => "online",
            PresenceStatus::Idle => "idle",
            PresenceStatus::DoNotDisturb => "dnd",
            PresenceStatus::Invisible => "invisible",
        }
    }
}

/// The full structure sent to Discord Gateway op 3 (PRESENCE UPDATE).
#[derive(Serialize)]
pub(crate) struct PresenceUpdatePayload {
    pub since: u64,
    pub activities: Vec<ActivityPayload>,
    pub status: String,
    pub afk: bool,
}

#[derive(Serialize)]
pub(crate) struct ActivityPayload {
    pub name: String,
    #[serde(rename = "type")]
    pub activity_type: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamps: Option<TimestampPayload>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub assets: Option<AssetsPayload>,
}

#[derive(Serialize)]
pub(crate) struct TimestampPayload {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end: Option<i64>,
}

#[derive(Serialize)]
pub(crate) struct AssetsPayload {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub large_image: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub large_text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub small_image: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub small_text: Option<String>,
}

/// Build the JSON string for a PRESENCE UPDATE (Gateway op 3).
pub(crate) fn build_presence_update(
    status: PresenceStatus,
    activities: &[Activity],
) -> serde_json::Value {
    let payload = PresenceUpdatePayload {
        since: 0,
        activities: activities.iter().map(activity_to_payload).collect(),
        status: status.as_str().to_string(),
        afk: false,
    };

    serde_json::json!({
        "op": 3,
        "d": payload
    })
}

/// Build the JSON string for a single activity payload.
fn activity_to_payload(activity: &Activity) -> ActivityPayload {
    let name = if activity.name.is_empty() {
        match activity.activity_type {
            crate::ActivityType::Playing => "Playing",
            crate::ActivityType::Listening => "Listening",
            crate::ActivityType::Watching => "Watching",
            crate::ActivityType::Competing => "Competing",
        }
        .to_string()
    } else {
        activity.name.clone()
    };

    ActivityPayload {
        name,
        activity_type: activity.activity_type.code(),
        state: activity.state.clone(),
        details: activity.details.clone(),
        timestamps: activity.timestamps.as_ref().map(ts_to_payload),
        assets: activity.assets.as_ref().map(assets_to_payload),
    }
}

fn ts_to_payload(ts: &Timestamps) -> TimestampPayload {
    TimestampPayload {
        start: ts.start,
        end: ts.end,
    }
}

fn assets_to_payload(assets: &Assets) -> AssetsPayload {
    AssetsPayload {
        large_image: assets.large_image.clone(),
        large_text: assets.large_text.clone(),
        small_image: assets.small_image.clone(),
        small_text: assets.small_text.clone(),
    }
}

/// Build the Gateway identify frame (op 2).
///
/// The token must include the "Bearer " prefix for OAuth2 tokens.
pub(crate) fn build_identify_frame(app_id: &str, token: &str) -> serde_json::Value {
    // Discord Gateway expects "Bearer <access_token>" format for OAuth2 tokens
    let bearer_token = if token.starts_with("Bearer ") {
        token.to_string()
    } else {
        format!("Bearer {}", token)
    };

    serde_json::json!({
        "op": 2,
        "d": {
            "token": bearer_token,
            "intents": 0,
            "properties": {
                "os": "linux",
                "browser": "discord_social_rpc",
                "device": app_id
            },
            "compress": false
        }
    })
}

/// Build the Gateway heartbeat frame (op 1).
pub(crate) fn build_heartbeat_frame(seq: Option<u64>) -> serde_json::Value {
    serde_json::json!({
        "op": 1,
        "d": seq
    })
}

/// Build the Gateway resume frame (op 6).
pub(crate) fn build_resume_frame(token: &str, session_id: &str, seq: u64) -> serde_json::Value {
    let bearer_token = if token.starts_with("Bearer ") {
        token.to_string()
    } else {
        format!("Bearer {}", token)
    };

    serde_json::json!({
        "op": 6,
        "d": {
            "token": bearer_token,
            "session_id": session_id,
            "seq": seq
        }
    })
}
