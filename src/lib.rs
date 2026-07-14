//! # Discord Social RPC
//!
//! A Rust library for Discord Rich Presence using the `sdk.social_layer_presence`
//! OAuth2 scope. Provides a synchronous API — no `async`/`await` needed.
//!
//! ## Quick Start — Rich Presence
//!
//! ```no_run
//! use discord_social_rpc::{
//!     DiscordSocialRpc, Activity, ActivityType, Assets,
//! };
//!
//! let client = DiscordSocialRpc::new("your_app_id").unwrap();
//! let rpc = client.create_new_client("your_oauth2_token").unwrap();
//!
//! rpc.set_activity(
//!     Activity::new()
//!         .state("Playing Rust")
//!         .details("Building a library")
//!         .activity_type(ActivityType::Listening)
//!         .assets(Assets::new().large_image("mp:rust_logo"))
//! ).unwrap();
//!
//! rpc.start_activity().unwrap();
//! println!("{:?}", rpc.activity_status());
//! rpc.stop_activity().unwrap();
//! ```
//!
//! ## Admin — Token Refresh & Code Exchange
//!
//! ```no_run
//! use discord_social_rpc::DiscordSocialRpcAdmin;
//!
//! // Create the admin with your Discord client_id and client_secret
//! let admin = DiscordSocialRpcAdmin::new("your_client_id", "your_client_secret")
//!     .expect("Failed to create admin");
//!
//! // Refresh a user's OAuth2 token
//! let refresh_resp = admin.refresh_user_token("user_refresh_token")
//!     .expect("Failed to refresh token");
//! println!("New access token: {}", refresh_resp.access_token);
//!
//! // Exchange an authorization code for tokens
//! let exchange_resp = admin.exchange_code("auth_code", "https://your-redirect-uri")
//!     .expect("Failed to exchange code");
//! println!("Access token: {}", exchange_resp.access_token);
//! ```

mod activity;
mod client;
mod discord_social_rpc;
mod error;
mod external_assets;
mod payload;
mod presence;
mod status;
pub mod utils;

pub(crate) mod gateway;

pub use activity::{Activity, ActivityType, Assets, Timestamps};
pub use client::DiscordRpcClient;
pub use discord_social_rpc::{DiscordSocialRpc, DiscordSocialRpcAdmin};
pub use error::Error;
pub use presence::PresenceStatus;
pub use status::ActivityStatus;
pub use utils::{CodeExchangeResponse, TokenRefreshResponse};
