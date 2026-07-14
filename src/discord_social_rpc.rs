use std::sync::Arc;
use tokio::runtime::Runtime;
use log::info;

use crate::client::DiscordRpcClient;
use crate::error::Error;
use crate::external_assets::ExternalAssetsResolver;
use crate::gateway::GatewayState;
use crate::utils::{self, TokenRefreshResponse, CodeExchangeResponse};

// ============================================================
// DiscordSocialRpc (déplacé depuis client.rs)
// ============================================================

/// The entry point for Discord Social RPC.
///
/// This factory creates the internal tokio runtime and provides
/// methods to create rich presence clients.
#[derive(Clone)]
pub struct DiscordSocialRpc {
    pub(crate) app_id: String,
    pub(crate) runtime: Arc<Runtime>,
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

        Ok(DiscordRpcClient::new(
            self.app_id.clone(),
            token,
            self.runtime.clone(),
            state,
            std::sync::Mutex::new(None),
            ExternalAssetsResolver::new(),
        ))
    }
}

// ============================================================
// DiscordSocialRpcAdmin (nouveau)
// ============================================================

/// Admin interface for Discord OAuth2 operations.
///
/// Wraps a [`DiscordSocialRpc`] and provides token refresh and code exchange
/// methods using the client_id and client_secret stored at construction time.
#[derive(Clone)]
pub struct DiscordSocialRpcAdmin {
    client_id: String,
    client_secret: String,
    rpc: DiscordSocialRpc,
}

impl DiscordSocialRpcAdmin {
    /// Create a new DiscordSocialRpcAdmin with the given Discord client_id and client_secret.
    /// Also creates the internal DiscordSocialRpc (which needs client_id = app_id).
    pub fn new(client_id: &str, client_secret: &str) -> Result<Self, Error> {
        let rpc = DiscordSocialRpc::new(client_id)?;
        Ok(Self {
            client_id: client_id.to_string(),
            client_secret: client_secret.to_string(),
            rpc,
        })
    }

    /// Refresh a user's OAuth2 token (synchronous, uses internal runtime).
    pub fn refresh_user_token(&self, refresh_token: &str) -> Result<TokenRefreshResponse, Error> {
        let client_id = self.client_id.clone();
        let client_secret = self.client_secret.clone();
        let refresh_token = refresh_token.to_string();
        self.rpc.runtime.block_on(async {
            utils::refresh_user_token(&client_id, &client_secret, &refresh_token).await
        })
    }

    /// Exchange an authorization code for tokens (synchronous, uses internal runtime).
    pub fn exchange_code(&self, code: &str, redirect_uri: &str) -> Result<CodeExchangeResponse, Error> {
        let client_id = self.client_id.clone();
        let client_secret = self.client_secret.clone();
        let code = code.to_string();
        let redirect_uri = redirect_uri.to_string();
        self.rpc.runtime.block_on(async {
            utils::exchange_code(&client_id, &client_secret, &code, &redirect_uri).await
        })
    }

    /// Return a reference to the underlying DiscordSocialRpc for creating clients,
    /// setting activities, etc.
    pub fn rpc(&self) -> &DiscordSocialRpc {
        &self.rpc
    }
}