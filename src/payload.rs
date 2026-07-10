use serde::Serialize;
use crate::activity::Activity;
use crate::presence::PresenceStatus;

// --- Internal payload types ---

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

// --- Builder functions ---

/// Build a PRESENCE UPDATE (Gateway op 3) payload.
pub(crate) fn build_presence_update(
    status: PresenceStatus,
    activities: &[Activity],
) -> serde_json::Value {
    serde_json::json!({
        "op": 3,
        "d": PresenceUpdatePayload {
            since: 0,
            activities: activities.iter().map(activity_to_payload).collect(),
            status: status.as_str().to_string(),
            afk: false,
        }
    })
}

/// Build an IDENTIFY frame (Gateway op 2).
pub(crate) fn build_identify_frame(app_id: &str, token: &str) -> serde_json::Value {
    let bearer = ensure_bearer(token);
    serde_json::json!({
        "op": 2,
        "d": {
            "token": bearer,
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

/// Build a heartbeat frame (Gateway op 1).
pub(crate) fn build_heartbeat_frame(seq: Option<u64>) -> serde_json::Value {
    serde_json::json!({ "op": 1, "d": seq })
}

/// Build a RESUME frame (Gateway op 6).
pub(crate) fn build_resume_frame(
    token: &str,
    session_id: &str,
    seq: u64,
) -> serde_json::Value {
    serde_json::json!({
        "op": 6,
        "d": {
            "token": ensure_bearer(token),
            "session_id": session_id,
            "seq": seq
        }
    })
}

// --- Helpers ---

/// Ensures the token has a `Bearer ` prefix.
fn ensure_bearer(token: &str) -> String {
    if token.starts_with("Bearer ") {
        token.to_string()
    } else {
        format!("Bearer {}", token)
    }
}

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
        timestamps: activity.timestamps.as_ref().map(|ts| TimestampPayload {
            start: ts.start,
            end: ts.end,
        }),
        assets: activity.assets.as_ref().map(|a| AssetsPayload {
            large_image: a.large_image.clone(),
            large_text: a.large_text.clone(),
            small_image: a.small_image.clone(),
            small_text: a.small_text.clone(),
        }),
    }
}