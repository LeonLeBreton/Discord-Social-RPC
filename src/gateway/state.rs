use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64};
use tokio::sync::{broadcast, watch, Mutex};

use crate::status::ActivityStatus;

/// Shared state between the gateway task and `DiscordRpcClient`.
pub struct GatewayState {
    pub status_tx: watch::Sender<ActivityStatus>,
    pub status_rx: watch::Receiver<ActivityStatus>,

    pub session_id: Mutex<Option<String>>,
    pub last_seq: AtomicU64,
    pub ready: AtomicBool,
    pub resume_url: Mutex<Option<String>>,
    pub user_name: Mutex<Option<String>>,

    stop_tx: watch::Sender<bool>,
    pub(crate) stop_rx: watch::Receiver<bool>,

    pub presence_broadcast: broadcast::Sender<String>,
    pub heartbeat_interval_ms: AtomicU64,
}

impl GatewayState {
    pub fn new() -> Arc<Self> {
        let (presence_tx, _) = broadcast::channel(64);
        let (stop_tx, stop_rx) = watch::channel(false);
        let (status_tx, status_rx) = watch::channel(ActivityStatus::NotStarted);

        Arc::new(Self {
            status_tx,
            status_rx,
            session_id: Mutex::new(None),
            last_seq: AtomicU64::new(0),
            ready: AtomicBool::new(false),
            resume_url: Mutex::new(None),
            user_name: Mutex::new(None),
            stop_tx,
            stop_rx,
            presence_broadcast: presence_tx,
            heartbeat_interval_ms: AtomicU64::new(41_250),
        })
    }

    pub fn set_sync(&self, status: ActivityStatus) {
        let _ = self.status_tx.send(status);
    }

    pub fn set_async(&self, status: ActivityStatus) {
        let _ = self.status_tx.send(status);
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