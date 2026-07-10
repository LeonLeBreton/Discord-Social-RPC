//! # Discord Social RPC
//!
//! A Rust library for Discord Rich Presence using the `sdk.social_layer_presence`
//! OAuth2 scope. Provides a synchronous API — no `async`/`await` needed.
//!
//! ## Quick Start
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

mod activity;
mod client;
mod error;
mod external_assets;
mod payload;
mod presence;
mod status;

pub(crate) mod gateway;

pub use activity::{Activity, ActivityType, Assets, Timestamps};
pub use client::{DiscordRpcClient, DiscordSocialRpc};
pub use error::Error;
pub use presence::PresenceStatus;
pub use status::ActivityStatus;
