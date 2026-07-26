use std::time::Duration;
use std::sync::Arc;
use std::sync::atomic::Ordering;
use tokio::net::TcpStream;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::tungstenite::protocol::CloseFrame;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};
use futures_util::stream::{SplitSink, SplitStream, StreamExt};
use futures_util::SinkExt;
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

type WsStream = WebSocketStream<MaybeTlsStream<TcpStream>>;
type WsWriter = SplitSink<WsStream, Message>;
type WsReader = SplitStream<WsStream>;

pub async fn run_gateway(state: Arc<GatewayState>, app_id: String, token: String) {
    let mut attempts = 0u32;

    loop {
        if state.is_stopped() {
            info!("gateway: stop requested");
            break;
        }
        info!("gateway: connecting (attempt {})", attempts + 1);

        match run_session(&state, &app_id, &token).await {
            SessionResult::Reconnect => {
                if !should_reconnect(&mut attempts, &state).await {
                    break;
                }
            }
            SessionResult::Stop | SessionResult::Fatal => {
                state.set_async(ActivityStatus::Stopped);
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
            state.set_async(ActivityStatus::NetworkError);
            return SessionResult::Reconnect;
        }
    };

    state.set_async(ActivityStatus::Disconnected);
    let (mut write, mut read) = ws.split();

    let hb_interval = match wait_for_hello(&mut read, state).await {
        Ok(i) => i,
        Err(r) => return r,
    };

    let mut presence_rx = state.presence_broadcast.subscribe();
    let mut stop_rx = state.stop_rx.clone();

    if let Err(r) = send_identify_or_resume(&mut write, state, app_id, token).await {
        return r;
    }

    let jitter = jitter_ms(hb_interval);
    let mut missed_acks = 0u32;
    let mut heartbeat = tokio::time::interval_at(
        tokio::time::Instant::now() + Duration::from_millis(jitter),
        Duration::from_millis(hb_interval),
    );
    heartbeat.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

    loop {
        tokio::select! {
            _stop = stop_rx.changed() => {
                if *stop_rx.borrow() { return SessionResult::Stop; }
            }
            _tick = heartbeat.tick() => {
                if let Err(r) = send_heartbeat(&mut write, state, &mut missed_acks).await {
                    return r;
                }
            }
            r = presence_rx.recv() => {
                if let Err(r) = handle_presence(r, &mut write).await {
                    return r;
                }
            }
            msg = read.next() => {
                if let Err(r) = handle_ws_message(msg, &mut write, state, &mut missed_acks).await {
                    return r;
                }
            }
        }
    }
}

/// Wait for the initial HELLO opcode and extract the heartbeat interval.
async fn wait_for_hello(
    read: &mut WsReader,
    state: &Arc<GatewayState>,
) -> Result<u64, SessionResult> {
    match read.next().await {
        Some(Ok(Message::Text(t))) => match serde_json::from_str::<serde_json::Value>(&t) {
            Ok(v) => {
                let hb_interval = v["d"]["heartbeat_interval"]
                    .as_u64()
                    .unwrap_or(41_250)
                    .max(5_000);
                state
                    .heartbeat_interval_ms
                    .store(hb_interval, Ordering::SeqCst);
                debug!("gateway: HELLO received, heartbeat={hb_interval}ms");
                Ok(hb_interval)
            }
            Err(e) => {
                warn!("gateway: bad HELLO frame: {e}");
                Err(SessionResult::Reconnect)
            }
        },
        Some(Ok(Message::Close(frame))) => {
            Err(handle_close_frame(frame, true, state).await)
        }
        Some(Ok(_)) => {
            warn!("gateway: unexpected message before HELLO");
            Err(SessionResult::Reconnect)
        }
        Some(Err(e)) => {
            warn!("gateway: error before HELLO: {e}");
            Err(SessionResult::Reconnect)
        }
        None => {
            warn!("gateway: stream ended before HELLO");
            Err(SessionResult::Reconnect)
        }
    }
}

/// Send IDENTIFY or RESUME based on current session state.
async fn send_identify_or_resume(
    write: &mut WsWriter,
    state: &Arc<GatewayState>,
    app_id: &str,
    token: &str,
) -> Result<(), SessionResult> {
    let do_resume = state.session_id.lock().await.is_some()
        && state.ready.load(Ordering::SeqCst);

    if do_resume {
        let sid = state.session_id.lock().await.clone().unwrap_or_default();
        let seq = state.last_seq.load(Ordering::SeqCst);
        let frame = build_resume_frame(token, &sid, seq);
        if write.send(Message::Text(frame.to_string().into())).await.is_err() {
            return Err(SessionResult::Reconnect);
        }
        info!("gateway: RESUME sent (sid={})", &sid[..sid.len().min(8)]);
    } else {
        let frame = build_identify_frame(app_id, token);
        if write.send(Message::Text(frame.to_string().into())).await.is_err() {
            return Err(SessionResult::Reconnect);
        }
        info!("gateway: IDENTIFY sent");
    }
    Ok(())
}

/// Send a heartbeat frame and check for missed acknowledgements.
async fn send_heartbeat(
    write: &mut WsWriter,
    state: &Arc<GatewayState>,
    missed_acks: &mut u32,
) -> Result<(), SessionResult> {
    let seq = state.last_seq.load(Ordering::SeqCst);
    if write
        .send(Message::Text(build_heartbeat_frame(Some(seq)).to_string().into()))
        .await
        .is_err()
    {
        return Err(SessionResult::Reconnect);
    }
    if *missed_acks >= 3 {
        warn!("gateway: {missed_acks} missed heartbeats, reconnecting");
        return Err(SessionResult::Reconnect);
    }
    *missed_acks += 1;
    Ok(())
}

/// Process a single WebSocket message received in the main loop.
async fn handle_ws_message(
    msg: Option<Result<Message, tokio_tungstenite::tungstenite::Error>>,
    write: &mut WsWriter,
    state: &Arc<GatewayState>,
    missed_acks: &mut u32,
) -> Result<(), SessionResult> {
    let Some(Ok(msg)) = msg else {
        warn!("gateway: ws stream ended or errored");
        return Err(SessionResult::Reconnect);
    };

    match msg {
        Message::Text(t) => match handle_event(&t, state).await {
            Event::Continue => {}
            Event::Ready | Event::Resumed => {
                state.ready.store(true, Ordering::SeqCst);
                state.set_async(ActivityStatus::Ok);
            }
            Event::HeartbeatAck => {
                *missed_acks = 0;
            }
            Event::InvalidSession(resumable) => {
                if !resumable {
                    *state.session_id.lock().await = None;
                    state.ready.store(false, Ordering::SeqCst);
                }
                return Err(SessionResult::Reconnect);
            }
            Event::Reconnect => {
                return Err(SessionResult::Reconnect);
            }
        },
        Message::Close(frame) => {
            return Err(handle_close_frame(frame, false, state).await);
        }
        Message::Ping(d) => {
            let _ = write.send(Message::Pong(d)).await;
        }
        _ => {} // Binary / other
    }
    Ok(())
}

/// Handle a close frame, shared between pre-hello and in-session contexts.
/// When `before_hello` is true, returns `SessionResult` directly.
/// When `before_hello` is false, returns `Result<(), SessionResult>` for use in `handle_ws_message`.
async fn handle_close_frame(
    frame: Option<CloseFrame>,
    before_hello: bool,
    state: &Arc<GatewayState>,
) -> SessionResult {
    let Some(f) = frame else {
        return SessionResult::Reconnect;
    };

    let code: u16 = f.code.into();
    warn!("gateway: close code={code} reason={}", f.reason);

    match code {
        4004 => {
            state.set_async(ActivityStatus::TokenInvalid);
            SessionResult::Fatal
        }
        4009 if !before_hello => {
            // Session invalidity — only relevant during an active session
            *state.session_id.lock().await = None;
            state.ready.store(false, Ordering::SeqCst);
            SessionResult::Reconnect
        }
        _ => SessionResult::Reconnect,
    }
}

/// Receive and forward a presence update to the WebSocket.
async fn handle_presence(
    r: Result<String, tokio::sync::broadcast::error::RecvError>,
    write: &mut WsWriter,
) -> Result<(), SessionResult> {
    match r {
        Ok(p) => {
            if write.send(Message::Text(p.into())).await.is_err() {
                Err(SessionResult::Reconnect)
            } else {
                Ok(())
            }
        }
        Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
            warn!("gateway: presence channel lagged by {n}");
            Ok(())
        }
        Err(tokio::sync::broadcast::error::RecvError::Closed) => {
            Err(SessionResult::Stop)
        }
    }
}

// --- Reconnection ---

async fn should_reconnect(attempts: &mut u32, state: &Arc<GatewayState>) -> bool {
    if state.is_stopped() {
        return false;
    }
    if *attempts >= MAX_RECONNECT_ATTEMPTS {
        warn!("gateway: max reconnect attempts ({MAX_RECONNECT_ATTEMPTS}) reached");
        state.set_async(ActivityStatus::NetworkError);
        return false;
    }
    *attempts += 1;
    let delay = reconnect_delay(*attempts);
    info!(
        "gateway: reconnecting in {delay}ms (attempt {}/MAX_RECONNECT_ATTEMPTS)",
        *attempts
    );
    tokio::time::sleep(Duration::from_millis(delay)).await;
    true
}

fn reconnect_delay(attempt: u32) -> u64 {
    let base = RECONNECT_BASE_MS * (1u64 << (attempt - 1).min(6));
    base.min(RECONNECT_MAX_MS)
}

fn jitter_ms(base: u64) -> u64 {
    // Random jitter between 50% and 100% of base
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .subsec_nanos();
    let frac = u64::from(nanos ^ (nanos << 10) ^ (nanos >> 5)) % base;
    base / 2 + frac
}
