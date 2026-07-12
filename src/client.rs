use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};
use tokio::runtime::Runtime;
use log::{debug, info};

use crate::activity::Activity;
use crate::error::Error;
use crate::external_assets::ExternalAssetsResolver;
use crate::gateway::{run_gateway, GatewayState};
use crate::payload::build_presence_update;
use crate::presence::PresenceStatus;
use crate::status::ActivityStatus;

/// The entry point for Discord Social RPC.
///
/// This factory creates the internal tokio runtime and provides
/// methods to create rich presence clients.
///
/// # Example
///
/// ```no_run
/// use discord_social_rpc::{DiscordSocialRpc, Activity, ActivityType};
///
/// let client = DiscordSocialRpc::new("your_app_id").unwrap();
/// let rpc = client.create_new_client("your_oauth2_token").unwrap();
///
/// rpc.set_activity(
///     Activity::new()
///         .state("Playing Rust")
///         .details("Building a library")
///         .activity_type(ActivityType::Listening)
/// ).unwrap();
///
/// rpc.start_activity().unwrap();
/// println!("{:?}", rpc.activity_status());
/// rpc.stop_activity().unwrap();
/// ```
pub struct DiscordSocialRpc {
    app_id: String,
    runtime: Arc<Runtime>,
}

impl DiscordSocialRpc {
    /// Create a new DiscordSocialRpc instance with the given Discord Application ID.
    /// Creates an internal tokio runtime.
    pub fn new(app_id: &str) -> Result<Self, Error> {
        let app_id = app_id.to_string();
        let runtime = Runtime::new().map_err(|e| Error::Runtime(e.to_string()))?;

        Ok(Self {
            app_id,
            runtime: Arc::new(runtime),
        })
    }

    /// Validate the OAuth2 token format and create a new RPC client.
    ///
    /// This validates the token format but does NOT connect to Discord Gateway yet.
    /// Call `start_activity()` to establish the WebSocket connection and display activity.
    pub fn create_new_client(&self, oauth2_token: &str) -> Result<DiscordRpcClient, Error> {
        let token = oauth2_token.trim().to_string();

        if token.is_empty() || token.contains(' ') {
            return Err(Error::InvalidToken(
                "Token is empty or contains whitespace".to_string(),
            ));
        }

        if token.len() < 20 {
            return Err(Error::InvalidToken(
                "Token is too short to be a valid OAuth2 token".to_string(),
            ));
        }

        info!(
            "DiscordSocialRpc: created client for app_id={}",
            self.app_id
        );

        let state = GatewayState::new();

        Ok(DiscordRpcClient {
            app_id: self.app_id.clone(),
            token: Mutex::new(token),
            runtime: self.runtime.clone(),
            state,
            current_activity: std::sync::Mutex::new(None),
            asset_resolver: ExternalAssetsResolver::new(),
        })
    }
}

/// A client that manages a Discord Rich Presence connection.
///
/// Created via [`DiscordSocialRpc::create_new_client`].
/// Use the builder methods to configure activity, then call `start_activity()`.
pub struct DiscordRpcClient {
    app_id: String,
    token: Mutex<String>,
    runtime: Arc<Runtime>,
    state: Arc<GatewayState>,
    current_activity: std::sync::Mutex<Option<Activity>>,
    asset_resolver: ExternalAssetsResolver,
}

impl DiscordRpcClient {
    /// Build and send a presence update for a resolved activity (no re-resolution).
    fn send_activity_inner(&self, activity: &Activity) {
        let presence_json = build_presence_update(PresenceStatus::Online, &[activity.clone()]);
        self.state.send_presence(presence_json.to_string());
    }

    /// Store the activity and optionally send it if the gateway is ready.
    fn store_and_send_activity(&self, activity: Activity) {
        {
            let mut current = self.current_activity.lock().unwrap();
            *current = Some(activity.clone());
        }

        if self.state.ready.load(Ordering::SeqCst) {
            debug!("client: sending presence update via set_activity");
            self.send_activity_inner(&activity);
        } else {
            debug!("client: activity stored (not yet connected)");
        }
    }

    /// Configure the activity to display on Discord.
    ///
    /// If the client is already connected (after `start_activity()`), the
    /// activity is sent immediately via the Gateway. Otherwise it is stored
    /// and will be sent when `start_activity()` is called.
    pub fn set_activity(&self, activity: Activity) -> Result<(), Error> {
        let token = self.token.lock().unwrap().clone();

        // Resolve external images now, so we don't need to re-resolve later
        let resolved_activity = resolve_activity_images(
            &activity,
            &self.app_id,
            &token,
            &self.asset_resolver,
        );

        self.store_and_send_activity(resolved_activity);
        Ok(())
    }

    /// Connect to Discord Gateway and start displaying the configured activity.
    ///
    /// This establishes a WebSocket connection to Discord, identifies the
    /// client, and sends the current presence.
    pub fn start_activity(&self) -> Result<(), Error> {
        let state = self.state.clone();
        let app_id = self.app_id.clone();
        let token = self.token.lock().unwrap().clone();

        // Set initial status
        self.state.set_sync(ActivityStatus::Disconnected);

        // Launch the gateway task
        self.runtime.spawn(async move {
            run_gateway(state, app_id, token).await;
        });

        // Wait for either READY or an error condition
        self.runtime.block_on(async {
            let mut status_rx = self.state.status_rx.clone();

            loop {
                tokio::select! {
                    _ = status_rx.changed() => {
                        let status = status_rx.borrow().clone();
                        match status {
                            ActivityStatus::Ok => {
                                // Send the stored activity if any
                                let activity = {
                                    let current = self.current_activity.lock().unwrap();
                                    current.clone()
                                };
                                if let Some(act) = activity {
                                    self.send_activity_inner(&act);
                                }
                                return Ok(());
                            }
                            ActivityStatus::TokenInvalid => {
                                return Err(Error::InvalidToken(
                                    "Token rejected by Discord Gateway. Make sure the OAuth2 token has the 'sdk.social_layer_presence' scope.".to_string(),
                                ));
                            }
                            ActivityStatus::NetworkError => {
                                return Err(Error::Network(
                                    "Failed to connect to Discord Gateway (network issue)".to_string(),
                                ));
                            }
                            ActivityStatus::Stopped => {
                                return Err(Error::AlreadyStopped);
                            }
                            _ => {}
                        }
                    }
                    _ = tokio::time::sleep(std::time::Duration::from_secs(15)) => {
                        return Err(Error::Network(
                            "Timed out waiting for gateway READY (15s) - check your internet connection".to_string(),
                        ));
                    }
                }
            }
        })?;

        info!("client: activity started successfully");
        Ok(())
    }

    /// Get the current connection/activity status.
    pub fn activity_status(&self) -> ActivityStatus {
        self.state.status_rx.borrow().clone()
    }

    /// Get the user name if available (best-effort, available after READY).
    pub fn user_name(&self) -> Option<String> {
        self.runtime
            .block_on(async { self.state.user_name.lock().await.clone() })
    }

    /// Update the OAuth2 token used by this client.
    /// Useful for token refresh without needing to recreate the client.
    /// The gateway task already has its own copy; this ensures future
    /// identify/resume attempts use the new token.
    pub fn set_token(&self, new_token: &str) {
        let mut token = self.token.lock().unwrap();
        *token = new_token.to_string();
        info!("client: token updated");
    }

    /// Stop displaying the activity and disconnect from Discord Gateway.
    ///
    /// This sends an empty presence update (clears the activity) and
    /// closes the WebSocket connection.
    pub fn stop_activity(&self) -> Result<(), Error> {
        info!("client: stopping activity");

        // Send empty presence to clear activity
        let clear_presence = build_presence_update(PresenceStatus::Online, &[]);
        let payload = clear_presence.to_string();
        self.state.send_presence(payload);

        // Signal the gateway to stop
        self.state.request_stop();

        // Wait briefly for the gateway to clean up
        self.runtime.block_on(async {
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        });

        // Reset state
        self.state.ready.store(false, Ordering::SeqCst);
        self.state.set_sync(ActivityStatus::Stopped);

        // Clear activity
        {
            let mut current = self.current_activity.lock().unwrap();
            *current = None;
        }

        info!("client: activity stopped");
        Ok(())
    }
}

/// Resolve a single image field if it represents an external URL.
fn resolve_single_image(
    image: &mut Option<String>,
    external: &mut bool,
    resolver: &ExternalAssetsResolver,
    app_id: &str,
    token: &str,
) {
    if !*external {
        return;
    }
    if let Some(url) = image {
        if let Some(resolved_url) = resolver.resolve(url, app_id, token) {
            *image = Some(resolved_url);
        }
    }
    *external = false;
}

/// Resolve any external image URLs in the activity assets.
fn resolve_activity_images(
    activity: &Activity,
    app_id: &str,
    token: &str,
    resolver: &ExternalAssetsResolver,
) -> Activity {
    let mut resolved = activity.clone();

    if let Some(assets) = resolved.assets.as_mut() {
        resolve_single_image(
            &mut assets.large_image,
            &mut assets.large_image_external,
            resolver,
            app_id,
            token,
        );
        resolve_single_image(
            &mut assets.small_image,
            &mut assets.small_image_external,
            resolver,
            app_id,
            token,
        );
    }

    resolved
}
