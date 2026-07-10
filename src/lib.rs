//! # Discord Social RPC
//!
//! A Rust library for Discord Rich Presence using the `sdk.social_layer_presence`
//! OAuth2 scope. This library provides a simple synchronous API for managing
//! Discord rich presence without needing a bot token.
//!
//! ## Quick Start
//!
//! ```no_run
//! use discord_social_rpc::{
//!     DiscordSocialRpc, Activity, ActivityType, Assets, Timestamps,
//! };
//!
//! // Create the factory (this creates an internal tokio runtime)
//! let client = DiscordSocialRpc::new("your_discord_app_id").unwrap();
//!
//! // Create a client with your OAuth2 token (validates format, no connection yet)
//! let rpc = client.create_new_client("your_oauth2_token").unwrap();
//!
//! // Configure your activity
//! rpc.set_activity(
//!     Activity::new()
//!         .state("Playing with Rust")
//!         .details("Building discord_social_rpc")
//!         .activity_type(ActivityType::Playing)
//!         .assets(
//!             Assets::new()
//!                 .large_image("mp:rust_logo")  // pre-registered asset
//!                 .large_text("Rust Programming Language")
//!         )
//!         .timestamps(
//!             Timestamps::new().start(1234567890000)
//!         )
//! ).unwrap();
//!
//! // Connect to Discord and start displaying
//! rpc.start_activity().unwrap();
//!
//! // Check status
//! match rpc.activity_status() {
//!     discord_social_rpc::ActivityStatus::Ok => println!("Connected!"),
//!     other => println!("Status: {:?}", other),
//! }
//!
//! // Get user info (best-effort)
//! if let Some(name) = rpc.user_name() {
//!     println!("Connected as: {}", name);
//! }
//!
//! // Update activity in real-time
//! rpc.set_activity(
//!     Activity::new()
//!         .state("Debugging")
//!         .details("Fixing a bug")
//! ).unwrap();
//!
//! // Clean shutdown
//! rpc.stop_activity().unwrap();
//! ```

mod activity;
mod client;
mod error;
mod external_assets;
mod gateway;
mod presence;
mod status;

pub use activity::{Activity, ActivityType, Assets, Timestamps};
pub use client::{DiscordRpcClient, DiscordSocialRpc};
pub use error::Error;
pub use status::ActivityStatus;
pub use presence::PresenceStatus;