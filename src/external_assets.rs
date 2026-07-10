use std::collections::HashMap;
use std::sync::Mutex;
use log::warn;

const EXTERNAL_ASSETS_URL: &str =
    "https://discord.com/api/v9/applications/{}/external-assets";
const CACHE_MAX_SIZE: usize = 128;

/// Resolves external image URLs to Discord's `mp:` format.
///
/// Uses the `POST /api/v9/applications/{id}/external-assets` endpoint.
pub(crate) struct ExternalAssetsResolver {
    cache: Mutex<HashMap<String, String>>,
}

impl ExternalAssetsResolver {
    pub fn new() -> Self {
        Self {
            cache: Mutex::new(HashMap::new()),
        }
    }

    /// Resolve an image URL. Returns `None` if resolution fails.
    ///
    /// URLs starting with `mp:` are returned as-is (already resolved).
    pub fn resolve(&self, image_url: &str, app_id: &str, token: &str) -> Option<String> {
        if image_url.is_empty() {
            return None;
        }
        if image_url.starts_with("mp:") {
            return Some(image_url.to_string());
        }

        // Check cache
        {
            if let Some(cached) = self.cache.lock().ok()?.get(image_url) {
                return Some(cached.clone());
            }
        }

        // Call external assets API
        let url = EXTERNAL_ASSETS_URL.replace("{}", app_id);
        let client = reqwest::blocking::Client::new();
        let body = serde_json::json!({ "urls": [image_url] });

        let response = client
            .post(&url)
            .header("Authorization", format!("Bearer {}", token))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .map_err(|e| {
                warn!("external-assets: request failed: {}", e);
            })
            .ok()?;

        let status = response.status();
        let response_text = response.text().ok()?;

        if !status.is_success() || response_text.is_empty() {
            warn!(
                "external-assets: HTTP {} for {}",
                status,
                &image_url[..60.min(image_url.len())]
            );
            return None;
        }

        let parsed: Vec<serde_json::Value> = serde_json::from_str(&response_text).ok()?;
        let path = parsed
            .first()?
            .get("external_asset_path")?
            .as_str()?;

        let result = format!("mp:{}", path);

        // Update cache
        if let Ok(mut cache) = self.cache.lock() {
            cache.insert(image_url.to_string(), result.clone());
            if cache.len() > CACHE_MAX_SIZE {
                let keys: Vec<String> = cache.keys().take(10).cloned().collect();
                for k in keys {
                    cache.remove(&k);
                }
            }
        }

        Some(result)
    }
}