# Discord Social RPC

A Rust library for **Discord Rich Presence** using the `sdk.social_layer_presence` OAuth2 scope.

This library provides a **synchronous API**. It handles the entire Discord Gateway WebSocket connection under the hood, allowing you to set and manage Rich Presence activities with just a few lines of code.

---

## Features

- **Synchronous API** — uses an internal Tokio runtime, no `async` in your code.
- **Rich Presence builder** — configure activities, assets, timestamps, and presence status with a clean builder pattern.
- **External image resolution** — automatically uploads external image URLs to Discord via the external assets API, with built-in caching.
- **Full lifecycle management** — connect, update, and disconnect from Discord Gateway seamlessly.
- **OAuth2 token validation** — format validation before connecting.
- **Comprehensive error handling** — typed errors covering network, WebSocket, Gateway protocol, token issues, and more.
- **Admin interface** — refresh user OAuth2 tokens and exchange authorization codes using your Discord client credentials.

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

## Quick Start — Rich Presence

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
        .set_name("My Game")
        .set_state("Exploring the world")
        .set_details("Level 42")
        .set_activity_type(ActivityType::Playing)
        .set_assets(
            Assets::new()
                .set_large_image("https://example.com/banner.png")
                .set_large_text("My Game Banner"),
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

## Admin — Token Refresh & Code Exchange

```rust,no_run
use discord_social_rpc::DiscordSocialRpcAdmin;

// Create the admin with your Discord client_id and client_secret
let admin = DiscordSocialRpcAdmin::new("your_client_id", "your_client_secret")
    .expect("Failed to create admin");

// Refresh a user's OAuth2 token
let refresh_resp = admin.refresh_user_token("user_refresh_token")
    .expect("Failed to refresh token");
println!("New access token: {}", refresh_resp.access_token);

// Exchange an authorization code for tokens
let exchange_resp = admin.exchange_code("auth_code", "https://your-redirect-uri")
    .expect("Failed to exchange code");
println!("Access token: {}", exchange_resp.access_token);
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

### `DiscordSocialRpcAdmin`

Admin interface for Discord OAuth2 operations. Wraps a [`DiscordSocialRpc`] and provides token refresh and code exchange methods using the client credentials stored at construction time.

```rust,no_run
let admin = DiscordSocialRpcAdmin::new("your_client_id", "your_client_secret")?;
```

| Method | Description |
|--------|-------------|
| `new(client_id, client_secret)` | Create a new admin instance with the given Discord client ID and client secret. Also creates the internal `DiscordSocialRpc`. |
| `refresh_user_token(refresh_token)` | Refresh a user's OAuth2 access token using the refresh token. Returns a `TokenRefreshResponse`. Synchronous — uses the internal runtime. |
| `exchange_code(code, redirect_uri)` | Exchange an OAuth2 authorization code for access and refresh tokens. Returns a `CodeExchangeResponse`. Synchronous — uses the internal runtime. |
| `rpc()` | Return a reference to the underlying `DiscordSocialRpc` for creating clients, setting activities, etc. |

### `TokenRefreshResponse`

Response returned by `refresh_user_token()`.

| Field | Type | Description |
|-------|------|-------------|
| `access_token` | `String` | New OAuth2 access token. |
| `refresh_token` | `Option<String>` | New refresh token (may be `None` if Discord does not rotate it). |
| `expires_in` | `u64` | Lifetime of the access token in seconds. |

### `CodeExchangeResponse`

Response returned by `exchange_code()`.

| Field | Type | Description |
|-------|------|-------------|
| `access_token` | `String` | OAuth2 access token. |
| `refresh_token` | `String` | Refresh token for obtaining new access tokens. |
| `expires_in` | `u64` | Lifetime of the access token in seconds. |

### `DiscordRpcClient`

Manages a single Rich Presence connection. Created via `DiscordSocialRpc::create_new_client()`.

```rust,no_run
let rpc = client.create_new_client("your_oauth2_token")?;
```

| Method | Description |
|--------|-------------|
| `set_activity(activity)` | Configure the activity. If connected, sends immediately via Gateway. Otherwise stores it until `start_activity()`. **If the activity is empty** (`Activity::new()` / `Activity::default()`), the current activity is cleared — nothing is displayed but the client stays connected. |
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

## Complete Example — Rich Presence

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

## Complete Example — Admin

```rust,no_run
use discord_social_rpc::DiscordSocialRpcAdmin;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize the admin with your Discord client credentials
    let admin = DiscordSocialRpcAdmin::new(
        "your_client_id",
        "your_client_secret",
    )?;

    // Exchange an authorization code for tokens
    let exchange = admin.exchange_code(
        "authorization_code_from_oauth2_flow",
        "https://your-redirect-uri",
    )?;

    println!("Access token: {}", exchange.access_token);
    println!("Refresh token: {}", exchange.refresh_token);

    // Later: refresh the token using the refresh token
    let refresh = admin.refresh_user_token(&exchange.refresh_token)?;
    println!("New access token: {}", refresh.access_token);

    // Also create RPC clients from the admin's underlying DiscordSocialRpc
    let rpc = admin.rpc().create_new_client(&exchange.access_token)?;
    // ... use rpc as normal

    Ok(())
}
```

---

## How It Works

### Rich Presence

1. **`DiscordSocialRpc::new()`** creates an internal Tokio runtime and stores your Application ID.
2. **`create_new_client()`** validates the OAuth2 token format and creates a `DiscordRpcClient` with shared Gateway state.
3. **`set_activity()`** configures the activity to display. If the Gateway is already connected, it sends the presence immediately. Otherwise, it stores the activity for later.
4. **`start_activity()`** spawns a Gateway task on the internal runtime that:
   - Establishes a WebSocket connection to `wss://gateway.discord.gg/`.
   - Sends an IDENTIFY payload with the OAuth2 token using the `sdk.social_layer_presence` scope.
   - Waits for the READY event.
   - Sends the stored presence update.
5. **`stop_activity()`** sends an empty presence (clearing the activity), signals the Gateway task to stop, and resets all state.

### Admin

1. **`DiscordSocialRpcAdmin::new()`** creates a `DiscordSocialRpc` internally and stores your client ID and client secret.
2. **`refresh_user_token()`** sends a `POST /api/v10/oauth2/token` request with `grant_type=refresh_token` to Discord's OAuth2 API and returns a new access token (and optionally a new refresh token).
3. **`exchange_code()`** sends a `POST /api/v10/oauth2/token` request with `grant_type=authorization_code` to Discord's OAuth2 API and returns an access token and a refresh token.
4. **`rpc()`** returns a reference to the underlying `DiscordSocialRpc`, allowing you to create RPC clients from freshly obtained access tokens.

All admin methods are synchronous — they use the same internal Tokio runtime as the rest of the library.

---

## License

This project is licensed under the MIT License. See the [LICENSE](LICENSE) file for details.
