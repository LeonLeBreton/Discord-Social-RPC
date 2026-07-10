use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex as AsyncMutex;
use tokio::sync::{broadcast, watch};
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message;
use futures_util::SinkExt;
use futures_util::StreamExt;
use log::{debug, error, info, warn};
use crate::presence::{build_identify_frame, build_heartbeat_frame, build_resume_frame};
use crate::status::ActivityStatus;

const GATEWAY_URL: &str = "wss://gateway.discord.gg/?v=10&encoding=json";
const MAX_RECONNECT_ATTEMPTS: u32 = 7;
const RECONNECT_BASE_DELAY_MS: u64 = 1000;
const RECONNECT_MAX_DELAY_MS: u64 = 64_000;

/// Internal state shared between the gateway task and the RpcClient.
pub(crate) struct GatewayState {
    pub status: AsyncMutex<ActivityStatus>,
    pub status_tx: watch::Sender<ActivityStatus>,
    pub status_rx: watch::Receiver<ActivityStatus>,
    pub session_id: AsyncMutex<Option<String>>,
    pub last_seq: AtomicU64,
    pub ready: AtomicBool,
    pub resume_url: AsyncMutex<Option<String>>,
    pub user_name: AsyncMutex<Option<String>>,
    pub stop_tx: watch::Sender<bool>,
    pub stop_rx: watch::Receiver<bool>,
    pub presence_broadcast: broadcast::Sender<String>,
    pub heartbeat_interval_ms: AtomicU64,
}

impl GatewayState {
    pub fn new() -> Arc<Self> {
        let (tx, _) = broadcast::channel(64);
        let (stop_tx, stop_rx) = watch::channel(false);
        let (status_tx, status_rx) = watch::channel(ActivityStatus::NotStarted);
        Arc::new(Self {
            status: AsyncMutex::new(ActivityStatus::NotStarted),
            status_tx,
            status_rx,
            session_id: AsyncMutex::new(None),
            last_seq: AtomicU64::new(0),
            ready: AtomicBool::new(false),
            resume_url: AsyncMutex::new(None),
            user_name: AsyncMutex::new(None),
            stop_tx,
            stop_rx,
            presence_broadcast: tx,
            heartbeat_interval_ms: AtomicU64::new(41250),
        })
    }

    pub fn set_status(&self, new_status: ActivityStatus) {
        let _ = self.status_tx.send(new_status.clone());
        let mut status = self.status.blocking_lock();
        *status = new_status;
    }

    pub async fn set_status_async(&self, new_status: ActivityStatus) {
        let _ = self.status_tx.send(new_status.clone());
        let mut status = self.status.lock().await;
        *status = new_status;
    }

    pub fn request_stop(&self) {
        let _ = self.stop_tx.send(true);
    }

    pub fn is_stopped(&self) -> bool {
        *self.stop_rx.borrow()
    }

    pub fn send_presence(&self, payload: String) {
        let _ = self.presence_broadcast.send(payload);
    }
}

/// Run the gateway connection loop.
pub(crate) async fn run_gateway(
    state: Arc<GatewayState>,
    app_id: String,
    token: String,
) {
    let mut reconnect_attempts = 0u32;

    loop {
        if state.is_stopped() {
            info!("gateway: stop requested, exiting loop");
            break;
        }

        info!("gateway: starting session (attempt {})", reconnect_attempts + 1);

        let result = run_session(state.clone(), &app_id, &token).await;

        match result {
            SessionResult::StopRequested => {
                info!("gateway: stop requested, exiting loop");
                state.set_status_async(ActivityStatus::Stopped).await;
                break;
            }
            SessionResult::Reconnect => {
                info!("gateway: reconnecting...");
                if !should_reconnect(&mut reconnect_attempts, &state).await {
                    break;
                }
                continue;
            }
            SessionResult::Fatal => {
                error!("gateway: fatal error, stopping");
                state.set_status_async(ActivityStatus::Stopped).await;
                break;
            }
        }
    }
}

enum SessionResult {
    StopRequested,
    Reconnect,
    Fatal,
}

async fn run_session(
    state: Arc<GatewayState>,
    app_id: &str,
    token: &str,
) -> SessionResult {
    let url = {
        let resume_url_opt = state.resume_url.lock().await;
        resume_url_opt
            .as_ref()
            .cloned()
            .unwrap_or_else(|| GATEWAY_URL.to_string())
    };

    info!("gateway: connecting to {}", url);
    let ws_stream = match connect_async(&url).await {
        Ok((ws, _)) => ws,
        Err(e) => {
            warn!("gateway: connect failed: {}", e);
            state.set_status_async(ActivityStatus::NetworkError).await;
            return SessionResult::Reconnect;
        }
    };

    state.set_status_async(ActivityStatus::Disconnected).await;
    let (mut write, mut read) = ws_stream.split();

    // Wait for HELLO (op 10) - read first message
    let hello_frame = match read.next().await {
        Some(Ok(Message::Text(text))) => {
            match serde_json::from_str::<serde_json::Value>(&text) {
                Ok(val) => val,
                Err(e) => {
                    warn!("gateway: failed to parse HELLO frame: {}", e);
                    return SessionResult::Reconnect;
                }
            }
        }
        Some(Ok(Message::Close(frame))) => {
            if let Some(f) = frame {
                warn!("gateway: received close before HELLO: code={}, reason={}", f.code, f.reason);
                let code: u16 = f.code.into();
                if code == 4004 {
                    state.set_status_async(ActivityStatus::TokenInvalid).await;
                    return SessionResult::Fatal;
                }
            }
            return SessionResult::Reconnect;
        }
        Some(Ok(other)) => {
            warn!("gateway: unexpected message before HELLO: {:?}", other);
            return SessionResult::Reconnect;
        }
        Some(Err(e)) => {
            warn!("gateway: error before HELLO: {}", e);
            return SessionResult::Reconnect;
        }
        None => {
            warn!("gateway: stream ended before HELLO");
            return SessionResult::Reconnect;
        }
    };

    let heartbeat_interval = hello_frame["d"]["heartbeat_interval"]
        .as_u64()
        .unwrap_or(41250);
    state.heartbeat_interval_ms.store(heartbeat_interval, Ordering::SeqCst);
    debug!("gateway: HELLO received, heartbeat_interval={}ms", heartbeat_interval);

    // Subscribe to presence broadcast and stop watch
    let mut presence_rx = state.presence_broadcast.subscribe();
    let mut stop_rx = state.stop_rx.clone();

    // Decide identify vs resume
    let should_resume = {
        let sid = state.session_id.lock().await;
        sid.is_some() && state.ready.load(Ordering::SeqCst)
    };

    if should_resume {
        let session_id = state.session_id.lock().await;
        let seq = state.last_seq.load(Ordering::SeqCst);
        if let Some(sid) = session_id.as_ref() {
            let resume_frame = build_resume_frame(token, sid, seq);
            if let Err(e) = write.send(Message::Text(resume_frame.to_string())).await {
                warn!("gateway: failed to send RESUME: {}", e);
                return SessionResult::Reconnect;
            }
            info!("gateway: RESUME sent (session_id={})", &sid[..sid.len().min(8)]);
        } else {
            let identify_frame = build_identify_frame(app_id, token);
            if let Err(e) = write.send(Message::Text(identify_frame.to_string())).await {
                warn!("gateway: failed to send IDENTIFY: {}", e);
                return SessionResult::Reconnect;
            }
            info!("gateway: IDENTIFY sent");
        }
    } else {
        let identify_frame = build_identify_frame(app_id, token);
        if let Err(e) = write.send(Message::Text(identify_frame.to_string())).await {
            warn!("gateway: failed to send IDENTIFY: {}", e);
            return SessionResult::Reconnect;
        }
        info!("gateway: IDENTIFY sent");
    }

    let jitter = rand_jitter(heartbeat_interval);
    let heartbeat_interval = heartbeat_interval.max(5000);
    let mut missed_acks = 0u32;
    let max_missed_acks = 3;

    let mut heartbeat_timer = tokio::time::interval_at(
        tokio::time::Instant::now() + Duration::from_millis(jitter),
        Duration::from_millis(heartbeat_interval),
    );
    heartbeat_timer.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

    loop {
        tokio::select! {
            biased;

            _ = stop_rx.changed() => {
                if *stop_rx.borrow() {
                    info!("gateway: stop signal received in session");
                    return SessionResult::StopRequested;
                }
            }

            _ = heartbeat_timer.tick() => {
                let seq = state.last_seq.load(Ordering::SeqCst);
                let frame = build_heartbeat_frame(Some(seq));
                debug!("gateway: sending heartbeat (seq={})", seq);
                if let Err(e) = write.send(Message::Text(frame.to_string())).await {
                    warn!("gateway: heartbeat send failed: {}", e);
                    break;
                }
                if missed_acks >= max_missed_acks {
                    warn!("gateway: missed {} heartbeats, reconnecting", missed_acks);
                    return SessionResult::Reconnect;
                }
                missed_acks += 1;
            }

            presence_result = presence_rx.recv() => {
                match presence_result {
                    Ok(payload) => {
                        debug!("gateway: sending presence update ({} bytes)", payload.len());
                        if let Err(e) = write.send(Message::Text(payload)).await {
                            warn!("gateway: presence send failed: {}", e);
                            break;
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(n)) => {
                        warn!("gateway: presence channel lagged by {} messages", n);
                    }
                    Err(broadcast::error::RecvError::Closed) => {
                        info!("gateway: presence channel closed, stopping session");
                        return SessionResult::StopRequested;
                    }
                }
            }

            msg = read.next() => {
                match msg {
                    Some(Ok(Message::Text(text))) => {
                        match handle_gateway_message(&text, &state).await {
                            GatewayMessageResult::Continue => {}
                            GatewayMessageResult::Ready => {
                                state.ready.store(true, Ordering::SeqCst);
                                state.set_status_async(ActivityStatus::Ok).await;
                                info!("gateway: READY received");
                            }
                            GatewayMessageResult::Resumed => {
                                state.ready.store(true, Ordering::SeqCst);
                                state.set_status_async(ActivityStatus::Ok).await;
                                info!("gateway: RESUMED received");
                            }
                            GatewayMessageResult::HeartbeatAck => {
                                missed_acks = 0;
                            }
                            GatewayMessageResult::InvalidSession(resumable) => {
                                warn!("gateway: INVALID_SESSION (resumable={})", resumable);
                                if !resumable {
                                    *state.session_id.lock().await = None;
                                    state.ready.store(false, Ordering::SeqCst);
                                }
                                return SessionResult::Reconnect;
                            }
                            GatewayMessageResult::Reconnect => {
                                warn!("gateway: server requested RECONNECT");
                                return SessionResult::Reconnect;
                            }
                            GatewayMessageResult::Fatal => {
                                return SessionResult::Fatal;
                            }
                        }
                    }
                    Some(Ok(Message::Close(frame))) => {
                        if let Some(f) = frame {
                            warn!("gateway: close frame: code={}, reason={}", f.code, f.reason);
                            let code: u16 = f.code.into();
                            match code {
                                4004 => {
                                    state.set_status_async(ActivityStatus::TokenInvalid).await;
                                    return SessionResult::Fatal;
                                }
                                4009 => {
                                    *state.session_id.lock().await = None;
                                    state.ready.store(false, Ordering::SeqCst);
                                    return SessionResult::Reconnect;
                                }
                                _ => return SessionResult::Reconnect,
                            }
                        }
                        return SessionResult::Reconnect;
                    }
                    Some(Ok(Message::Ping(data))) => {
                        let _ = write.send(Message::Pong(data)).await;
                    }
                    Some(Ok(Message::Pong(_))) => {}
                    Some(Ok(Message::Binary(_))) => {
                        debug!("gateway: received binary frame, ignoring");
                    }
                    Some(Ok(_)) => {
                        debug!("gateway: received unknown frame type");
                    }
                    Some(Err(e)) => {
                        warn!("gateway: websocket error: {}", e);
                        return SessionResult::Reconnect;
                    }
                    None => {
                        warn!("gateway: websocket stream ended");
                        return SessionResult::Reconnect;
                    }
                }
            }
        }
    }

    SessionResult::Reconnect
}

enum GatewayMessageResult {
    Continue,
    Ready,
    Resumed,
    HeartbeatAck,
    InvalidSession(bool),
    Reconnect,
    Fatal,
}

async fn handle_gateway_message(text: &str, state: &Arc<GatewayState>) -> GatewayMessageResult {
    let json: serde_json::Value = match serde_json::from_str(text) {
        Ok(v) => v,
        Err(e) => {
            warn!("gateway: invalid JSON: {}", e);
            return GatewayMessageResult::Continue;
        }
    };

    let op = json["op"].as_u64().unwrap_or(99);
    let data = &json["d"];
    let type_name = json["t"].as_str();

    if let Some(seq) = json["s"].as_u64() {
        if seq > 0 {
            state.last_seq.store(seq, Ordering::SeqCst);
        }
    }

    match op {
        0 => {
            match type_name {
                Some("READY") => {
                    if let Some(session_id) = data["session_id"].as_str() {
                        *state.session_id.lock().await = Some(session_id.to_string());
                    }
                    if let Some(resume_url) = data["resume_gateway_url"].as_str() {
                        if !resume_url.is_empty() {
                            *state.resume_url.lock().await = Some(resume_url.to_string());
                        }
                    }
                    if let Some(user) = data.get("user") {
                        if let Some(username) = user["username"].as_str() {
                            *state.user_name.lock().await = Some(username.to_string());
                        } else if let Some(global_name) = user["global_name"].as_str() {
                            *state.user_name.lock().await = Some(global_name.to_string());
                        }
                    }
                    GatewayMessageResult::Ready
                }
                Some("RESUMED") => GatewayMessageResult::Resumed,
                Some("USER_UPDATE") => {
                    if let Some(username) = data["username"].as_str() {
                        *state.user_name.lock().await = Some(username.to_string());
                    }
                    GatewayMessageResult::Continue
                }
                _ => GatewayMessageResult::Continue,
            }
        }
        7 => GatewayMessageResult::Reconnect,
        9 => {
            let resumable = data.as_bool().unwrap_or(false);
            GatewayMessageResult::InvalidSession(resumable)
        }
        11 => GatewayMessageResult::HeartbeatAck,
        _ => GatewayMessageResult::Continue,
    }
}

async fn should_reconnect(attempts: &mut u32, state: &Arc<GatewayState>) -> bool {
    if state.is_stopped() {
        return false;
    }
    if *attempts >= MAX_RECONNECT_ATTEMPTS {
        warn!("gateway: max reconnect attempts ({}) reached", MAX_RECONNECT_ATTEMPTS);
        state.set_status_async(ActivityStatus::NetworkError).await;
        return false;
    }
    *attempts += 1;
    let delay = reconnect_delay_ms(*attempts);
    info!("gateway: reconnecting in {}ms (attempt {}/{})", delay, *attempts, MAX_RECONNECT_ATTEMPTS);
    tokio::time::sleep(Duration::from_millis(delay)).await;
    true
}

fn reconnect_delay_ms(attempt: u32) -> u64 {
    let base = RECONNECT_BASE_DELAY_MS * (1u64 << (attempt - 1).min(6));
    base.min(RECONNECT_MAX_DELAY_MS)
}

fn rand_jitter(base_ms: u64) -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .subsec_nanos();
    (base_ms as f64 * (0.5 + 0.5 * (nanos as f64 / 1_000_000_000.0))) as u64
}