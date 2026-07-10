# Discord Social RPC

[![Crates.io](https://img.shields.io/crates/v/discord_social_rpc)](https://crates.io/crates/discord_social_rpc)
[![docs.rs](https://img.shields.io/docsrs/discord_social_rpc)](https://docs.rs/discord_social_rpc)
[![License](https://img.shields.io/badge/license-MIT-blue)](LICENSE)

A Rust library for **Discord Rich Presence** using the `sdk.social_layer_presence` OAuth2 scope.

This library provides a **synchronous API** — no `async`/`await` needed. It handles the entire Discord Gateway WebSocket connection under the hood, allowing you to set and manage Rich Presence activities with just a few lines of code.

---

## Features

- **Synchronous API** — uses an internal Tokio runtime, no `async` in your code.
- **Rich Presence builder** — configure activities, assets, timestamps, and presence status with a clean builder pattern.
- **External image resolution** — automatically uploads external image URLs to Discord via the external assets API, with built-in caching.
- **Full lifecycle management** — connect, update, and disconnect from Discord Gateway seamlessly.
- **OAuth2 token validation** — format validation before connecting.
- **Comprehensive error handling** — typed errors covering network, WebSocket, Gateway protocol, token issues, and more.

---

## Prerequisites

- Rust **2021 edition** or later.
- A **Discord Application** with access to the `sdk.social_layer_presence` OAuth2 scope.
- An **OAuth2 access token** obtained through Discord's OAuth2 flow with the `sdk.social_layer_presence` scope.

---

## Installation

Add the following to your `Cargo.toml`:

```toml
[dependencies]
discord_social_rpc = "0.1.0"
```

---

## Quick Start

```rust,no_run
use discord_social_rpc::{
    DiscordSocialRpc, Activity, ActivityType, Assets, PresenceStatus,
};

// 1. Create the entry point with your Discord Application ID
let client = DiscordSocialRpc::new("your_app_id").unwrap();

// 2. Create a new RPC client with your OAuth2 token
let rpc = client.create_new_client("your_oauth2_token").unwrap();

// 3. Configure the activity
rpc.set_activity(
    Activity::new()
        .name("My Game")
        .state("Exploring the world")
        .details("Level 42")
        .activity_type(ActivityType::Playing)
        .assets(
            Assets::new()
                .large_image("https://example.com/banner.png")
                .large_text("My Game Banner"),
        ),
)
.unwrap();

// 4. Connect to Discord Gateway and display the activity
rpc.start_activity().unwrap();

// 5. Check the connection status
println!("{:?}", rpc.activity_status());

// 6. Get the connected user's name (available after READY)
if let Some(name) = rpc.user_name() {
    println!("Connected as {}", name);
}

// 7. Stop the activity and disconnect
rpc.stop_activity().unwrap();
```

---

## API Overview

### `DiscordSocialRpc`

The factory that creates the internal Tokio runtime and provides methods to create RPC clients.

```rust,no_run
let client = DiscordSocialRpc::new("your_app_id")?;
```

| Method | Description |
|--------|-------------|
| `new(app_id)` | Create a new instance with the given Discord Application ID. |
| `create_new_client(oauth2_token)` | Validate the token format and create a `DiscordRpcClient`. Does **not** connect yet. |

### `DiscordRpcClient`

Manages a single Rich Presence connection. Created via `DiscordSocialRpc::create_new_client()`.

```rust,no_run
let rpc = client.create_new_client("your_oauth2_token")?;
```

| Method | Description |
|--------|-------------|
| `set_activity(activity)` | Configure the activity. If connected, sends immediately via Gateway. Otherwise stores it until `start_activity()`. |
| `start_activity()` | Connect to Discord Gateway, authenticate, and start displaying the activity. Blocks until READY or error. |
| `stop_activity()` | Clear the activity, disconnect from Gateway, and reset internal state. |
| `activity_status()` | Get the current connection status (`ActivityStatus`). |
| `user_name()` | Get the connected user's Discord name (best-effort, available after READY). |

### `Activity`

A Discord Rich Presence activity, built using the builder pattern.

```rust,no_run
let activity = Activity::new()
    .name("My App")
    .state("Doing something")
    .details("In the menu")
    .activity_type(ActivityType::Playing)
    .assets(Assets::new().large_image("mp:my_asset"))
    .timestamps(Timestamps::new().start(1234567890000));
```

| Method | Description |
|--------|-------------|
| `new()` | Create a new empty activity. |
| `name(name)` | Set the activity name (top line on Discord). Default: `"Playing Discord Social RPC"`. |
| `state(state)` | Set the state string (second line). |
| `details(details)` | Set the details string (first line below the name). |
| `activity_type(t)` | Set the activity type. |
| `assets(assets)` | Set activity images. |
| `timestamps(ts)` | Set start/end timestamps. |

### `ActivityType`

| Variant | Code | Description |
|---------|------|-------------|
| `Playing` | 0 | "Playing ..." |
| `Listening` | 2 | "Listening to ..." |
| `Watching` | 3 | "Watching ..." |
| `Competing` | 5 | "Competing in ..." |

### `Assets`

Images for the activity. Images can be:

- **Pre-registered** — use `mp:your_image_name` (assets uploaded to Discord Developer Portal).
- **External URL** — pass a full URL like `https://example.com/image.png`. The library automatically resolves it to an `mp:` path via Discord's external assets API.

```rust,no_run
let assets = Assets::new()
    .large_image("mp:official_logo")    // pre-registered asset
    .large_text("My App Logo")
    .small_image("https://example.com/icon.png")  // auto-resolved
    .small_text("Status icon");
```

### `Timestamps`

Unix epoch timestamps in milliseconds.

```rust,no_run
let ts = Timestamps::new()
    .start(1234567890000)
    .end(1234567899999);
```

### `PresenceStatus`

Controls the user's online status displayed alongside the activity.

| Variant | Wire format |
|---------|-------------|
| `Online` | `"online"` |
| `Idle` | `"idle"` |
| `DoNotDisturb` | `"dnd"` |
| `Invisible` | `"invisible"` |

### `ActivityStatus`

Represents the current state of the connection.

| Variant | Description |
|---------|-------------|
| `Ok` | Connected to Gateway and presence is displayed. |
| `Disconnected` | Not connected to Gateway. |
| `TokenInvalid` | OAuth2 token rejected by Discord. |
| `NetworkError` | Network error (timeout, connection refused). |
| `NotStarted` | Client created but `start_activity()` not yet called. |
| `Stopped` | Connection stopped after `stop_activity()`. |

### `Error`

All operations return `Result<T, Error>`. The `Error` enum provides specific error variants:

| Variant | Description |
|---------|-------------|
| `InvalidToken` | Token is empty, malformed, or rejected by Discord. |
| `Network` | Network-level error (timeout, DNS resolution failure, etc.). |
| `WebSocket` | WebSocket connection error. |
| `Gateway` | Discord Gateway protocol error. |
| `ExternalAssets` | Failed to resolve an external image URL. |
| `AlreadyStopped` | Activity was already stopped. |
| `NotStarted` | `start_activity()` was not called before the operation. |
| `Serialization` | JSON serialization/deserialization error. |
| `Runtime` | Internal Tokio runtime error. |

---

## External Image Resolution

When you pass an image URL (not starting with `mp:`) to `Assets::large_image()` or `Assets::small_image()`, the library:

1. Checks its internal cache for a previously resolved `mp:` path.
2. If not cached, calls `POST /api/v9/applications/{app_id}/external-assets` with the URL.
3. Stores the resolved `mp:` path in the cache (max 128 entries, with LRU eviction).
4. Uses the resolved path in the Gateway presence update.

This happens automatically during `set_activity()` and `start_activity()` — no manual steps required.

---

## Complete Example

```rust,no_run
use std::{thread, time::Duration};
use discord_social_rpc::{
    DiscordSocialRpc, Activity, ActivityType, Assets, Timestamps,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize the RPC factory
    let discord = DiscordSocialRpc::new("123456789012345678")?;

    // Create a client
    let rpc = discord.create_new_client("your_oauth2_token")?;

    // Build an activity
    let activity = Activity::new()
        .name("My Application")
        .state("Doing something cool")
        .details("Working on features")
        .activity_type(ActivityType::Playing)
        .assets(
            Assets::new()
                .large_image("mp:app_logo")
                .large_text("My App"),
        )
        .timestamps(
            Timestamps::new()
                .start(std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_millis() as i64),
        );

    // Set and display the activity
    rpc.set_activity(activity)?;
    rpc.start_activity()?;

    println!("Presence active! Status: {:?}", rpc.activity_status());

    // Keep the activity alive for 30 seconds
    thread::sleep(Duration::from_secs(30));

    // Stop gracefully
    rpc.stop_activity()?;
    println!("Presence stopped.");

    Ok(())
}
```

---

## Project Structure

```
discord_social_rpc/
├── Cargo.toml
├── README.md
└── src/
    ├── lib.rs              # Crate root, public API exports
    ├── activity.rs         # Activity, ActivityType, Assets, Timestamps
    ├── client.rs           # DiscordSocialRpc (factory), DiscordRpcClient
    ├── error.rs            # Error enum
    ├── external_assets.rs  # ExternalAssetsResolver (image upload)
    ├── payload.rs          # JSON payload building for Gateway
    ├── presence.rs         # PresenceStatus
    ├── status.rs           # ActivityStatus
    └── gateway/
        ├── mod.rs          # Gateway module entry point
        ├── events.rs       # Gateway event handling
        ├── session.rs      # WebSocket session management
        └── state.rs        # Gateway shared state
```

---

## How It Works

1. **`DiscordSocialRpc::new()`** creates an internal Tokio runtime and stores your Application ID.
2. **`create_new_client()`** validates the OAuth2 token format and creates a `DiscordRpcClient` with shared Gateway state.
3. **`set_activity()`** configures the activity to display. If the Gateway is already connected, it sends the presence immediately. Otherwise, it stores the activity for later.
4. **`start_activity()`** spawns a Gateway task on the internal runtime that:
   - Establishes a WebSocket connection to `wss://gateway.discord.gg/`.
   - Sends an IDENTIFY payload with the OAuth2 token using the `sdk.social_layer_presence` scope.
   - Waits for the READY event.
   - Sends the stored presence update.
5. **`stop_activity()`** sends an empty presence (clearing the activity), signals the Gateway task to stop, and resets all state.

---

## License

This project is licensed under the MIT License. See the [LICENSE](LICENSE) file for details.