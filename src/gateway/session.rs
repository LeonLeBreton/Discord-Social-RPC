use std::time::Duration;
use std::sync::Arc;
use std::sync::atomic::Ordering;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message;
use futures_util::{SinkExt, StreamExt};
use log::{debug, info, warn};

use super::state::GatewayState;
use super::events::{Event, handle_event};
use crate::payload::{build_identify_frame, build_heartbeat_frame, build_resume_frame};
use crate::status::ActivityStatus;

const GATEWAY_URL: &str = "wss://gateway.discord.gg/?v=10&encoding=json";
const MAX_RECONNECT_ATTEMPTS: u32 = 7;
const RECONNECT_BASE_MS: u64 = 1_000;
const RECONNECT_MAX_MS: u64 = 64_000;

enum SessionResult {
    Stop,
    Reconnect,
    Fatal,
}

pub(crate) async fn run_gateway(state: Arc<GatewayState>, app_id: String, token: String) {
    let mut attempts = 0u32;

    loop {
        if state.is_stopped() {
            info!("gateway: stop requested");
            break;
        }
        info!("gateway: connecting (attempt {})", attempts + 1);

        match run_session(&state, &app_id, &token).await {
            SessionResult::Stop => {
                state.set_async(ActivityStatus::Stopped).await;
                break;
            }
            SessionResult::Reconnect => {
                if !should_reconnect(&mut attempts, &state).await {
                    break;
                }
            }
            SessionResult::Fatal => {
                state.set_async(ActivityStatus::Stopped).await;
                break;
            }
        }
    }
}

async fn run_session(
    state: &Arc<GatewayState>,
    app_id: &str,
    token: &str,
) -> SessionResult {
    let url = state
        .resume_url
        .lock()
        .await
        .clone()
        .unwrap_or_else(|| GATEWAY_URL.to_string());

    let (ws, _) = match connect_async(&url).await {
        Ok(r) => r,
        Err(e) => {
            warn!("gateway: connection to {url} failed: {e}");
            state.set_async(ActivityStatus::NetworkError).await;
            return SessionResult::Reconnect;
        }
    };

    state.set_async(ActivityStatus::Disconnected).await;
    let (mut write, mut read) = ws.split();

    // --- Wait for HELLO ---
    let hello = match read.next().await {
        Some(Ok(Message::Text(t))) => match serde_json::from_str::<serde_json::Value>(&t) {
            Ok(v) => v,
            Err(e) => {
                warn!("gateway: bad HELLO frame: {e}");
                return SessionResult::Reconnect;
            }
        },
        Some(Ok(Message::Close(f))) => {
            if let Some(f) = f {
                let code: u16 = f.code.into();
                warn!("gateway: closed before HELLO: code={code} reason={}", f.reason);
                if code == 4004 {
                    state.set_async(ActivityStatus::TokenInvalid).await;
                    return SessionResult::Fatal;
                }
            }
            return SessionResult::Reconnect;
        }
        Some(Ok(_)) => {
            warn!("gateway: unexpected message before HELLO");
            return SessionResult::Reconnect;
        }
        Some(Err(e)) => {
            warn!("gateway: error before HELLO: {e}");
            return SessionResult::Reconnect;
        }
        None => {
            warn!("gateway: stream ended before HELLO");
            return SessionResult::Reconnect;
        }
    };

    let hb_interval = hello["d"]["heartbeat_interval"]
        .as_u64()
        .unwrap_or(41_250)
        .max(5_000);
    state
        .heartbeat_interval_ms
        .store(hb_interval, Ordering::SeqCst);
    debug!("gateway: HELLO received, heartbeat={hb_interval}ms");

    // --- Subscribe to channels ---
    let mut presence_rx = state.presence_broadcast.subscribe();
    let mut stop_rx = state.stop_rx.clone();

    // --- Identify or Resume ---
    let do_resume = state
        .session_id
        .lock()
        .await
        .is_some()
        && state.ready.load(Ordering::SeqCst);

    if do_resume {
        let sid = state.session_id.lock().await.clone().unwrap_or_default();
        let seq = state.last_seq.load(Ordering::SeqCst);
        let frame = build_resume_frame(token, &sid, seq);
        if write.send(Message::Text(frame.to_string())).await.is_err() {
            return SessionResult::Reconnect;
        }
        info!("gateway: RESUME sent (sid={})", &sid[..sid.len().min(8)]);
    } else {
        let frame = build_identify_frame(app_id, token);
        if write.send(Message::Text(frame.to_string())).await.is_err() {
            return SessionResult::Reconnect;
        }
        info!("gateway: IDENTIFY sent");
    }

    // --- Main loop ---
    let jitter = jitter_ms(hb_interval);
    let mut missed_acks = 0u32;
    let mut heartbeat = tokio::time::interval_at(
        tokio::time::Instant::now() + Duration::from_millis(jitter),
        Duration::from_millis(hb_interval),
    );
    heartbeat.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

    loop {
        tokio::select! {
            _stop = stop_rx.changed() => if *stop_rx.borrow() { return SessionResult::Stop },

            _tick = heartbeat.tick() => {
                let seq = state.last_seq.load(Ordering::SeqCst);
                if write.send(Message::Text(
                    build_heartbeat_frame(Some(seq)).to_string()
                )).await.is_err() {
                    break;
                }
                if missed_acks >= 3 {
                    warn!("gateway: {missed_acks} missed heartbeats, reconnecting");
                    return SessionResult::Reconnect;
                }
                missed_acks += 1;
            },

            r = presence_rx.recv() => {
                match r {
                    Ok(p) => {
                        if write.send(Message::Text(p)).await.is_err() { break; }
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                        warn!("gateway: presence channel lagged by {n}");
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                        return SessionResult::Stop;
                    }
                }
            },

            msg = read.next() => {
                match msg {
                    Some(Ok(Message::Text(t))) => match handle_event(&t, state).await {
                        Event::Continue => {}
                        Event::Ready | Event::Resumed => {
                            state.ready.store(true, Ordering::SeqCst);
                            state.set_async(ActivityStatus::Ok).await;
                        }
                        Event::HeartbeatAck => missed_acks = 0,
                        Event::InvalidSession(resumable) => {
                            if !resumable {
                                *state.session_id.lock().await = None;
                                state.ready.store(false, Ordering::SeqCst);
                            }
                            return SessionResult::Reconnect;
                        }
                        Event::Reconnect => return SessionResult::Reconnect,
                    },
                    Some(Ok(Message::Close(frame))) => {
                        if let Some(f) = frame {
                            let code: u16 = f.code.into();
                            warn!("gateway: close code={code} reason={}", f.reason);
                            match code {
                                4004 => { state.set_async(ActivityStatus::TokenInvalid).await; return SessionResult::Fatal }
                                4009 => { *state.session_id.lock().await = None; state.ready.store(false, Ordering::SeqCst); return SessionResult::Reconnect }
                                _ => return SessionResult::Reconnect
                            }
                        }
                        return SessionResult::Reconnect;
                    }
                    Some(Ok(Message::Ping(d))) => { let _ = write.send(Message::Pong(d)).await; }
                    Some(Ok(Message::Pong(_))) => {}
                    Some(Ok(_)) => {} // Binary / other
                    Some(Err(e)) => { warn!("gateway: ws error: {e}"); return SessionResult::Reconnect }
                    None => { warn!("gateway: stream ended"); return SessionResult::Reconnect }
                }
            },
        }
    }

    SessionResult::Reconnect
}

// --- Reconnection ---

async fn should_reconnect(attempts: &mut u32, state: &Arc<GatewayState>) -> bool {
    if state.is_stopped() {
        return false;
    }
    if *attempts >= MAX_RECONNECT_ATTEMPTS {
        warn!("gateway: max reconnect attempts ({MAX_RECONNECT_ATTEMPTS}) reached");
        state.set_async(ActivityStatus::NetworkError).await;
        return false;
    }
    *attempts += 1;
    let delay = reconnect_delay(*attempts);
    info!("gateway: reconnecting in {delay}ms (attempt {}/MAX_RECONNECT_ATTEMPTS)", *attempts);
    tokio::time::sleep(Duration::from_millis(delay)).await;
    true
}

fn reconnect_delay(attempt: u32) -> u64 {
    let base = RECONNECT_BASE_MS * (1u64 << (attempt - 1).min(6));
    base.min(RECONNECT_MAX_MS)
}

fn jitter_ms(base: u64) -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    let frac = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .subsec_nanos() as f64
        / 1_000_000_000.0;
    (base as f64 * (0.5 + 0.5 * frac)) as u64
}