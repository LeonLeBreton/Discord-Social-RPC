use std::sync::Arc;
use std::sync::atomic::Ordering;

use super::state::GatewayState;

pub(crate) enum Event {
    Continue,
    Ready,
    Resumed,
    HeartbeatAck,
    InvalidSession(bool),
    Reconnect,
}

pub(crate) async fn handle_event(text: &str, state: &Arc<GatewayState>) -> Event {
    let Ok(json) = serde_json::from_str::<serde_json::Value>(text) else {
        return Event::Continue;
    };

    let op = json["op"].as_u64().unwrap_or(99);
    let d = &json["d"];
    let t = json["t"].as_str();

    if let Some(seq) = json["s"].as_u64().filter(|&s| s > 0) {
        state.last_seq.store(seq, Ordering::SeqCst);
    }

    match op {
        0 => match t {
            Some("READY") => {
                if let Some(sid) = d["session_id"].as_str() {
                    *state.session_id.lock().await = Some(sid.to_string());
                }
                if let Some(url) = d["resume_gateway_url"].as_str().filter(|u| !u.is_empty()) {
                    *state.resume_url.lock().await = Some(url.to_string());
                }
                if let Some(user) = d.get("user") {
                    let name = user["username"]
                        .as_str()
                        .or_else(|| user["global_name"].as_str());
                    if let Some(n) = name {
                        *state.user_name.lock().await = Some(n.to_string());
                    }
                }
                Event::Ready
            }
            Some("RESUMED") => Event::Resumed,
            Some("USER_UPDATE") => {
                if let Some(name) = d["username"].as_str() {
                    *state.user_name.lock().await = Some(name.to_string());
                }
                Event::Continue
            }
            _ => Event::Continue,
        },
        7 => Event::Reconnect,
        9 => Event::InvalidSession(d.as_bool().unwrap_or(false)),
        11 => Event::HeartbeatAck,
        _ => Event::Continue,
    }
}