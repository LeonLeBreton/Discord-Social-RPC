use std::collections::HashMap;
use std::sync::Mutex;
use log::{debug, warn};

const EXTERNAL_ASSETS_API: &str = "https://discord.com/api/v9/applications/{}/external-assets";
const CACHE_MAX_SIZE: usize = 128;

/// Resolves external image URLs to Discord's `mp:` format
/// via the POST /api/v9/applications/{id}/external-assets endpoint.
pub(crate) struct ExternalAssetsResolver {
    cache: Mutex<HashMap<String, String>>,
}

impl ExternalAssetsResolver {
    pub fn new() -> Self {
        Self {
            cache: Mutex::new(HashMap::new()),
        }
    }

    /// Resolve an image URL. Returns `None` if resolution fails or the URL is blank.
    /// If the URL already starts with "mp:", it's returned as-is.
    pub fn resolve(
        &self,
        image_url: &str,
        app_id: &str,
        token: &str,
    ) -> Option<String> {
        if image_url.is_empty() {
            return None;
        }
        if image_url.starts_with("mp:") {
            return Some(image_url.to_string());
        }

        // Check cache
        {
            let cache = self.cache.lock().unwrap();
            if let Some(cached) = cache.get(image_url) {
                debug!("external-assets: cache hit for {}", &image_url[..60.min(image_url.len())]);
                return Some(cached.clone());
            }
        }

        debug!("external-assets: cache miss for {}, calling API", &image_url[..60.min(image_url.len())]);

        // Call external assets API synchronously using reqwest blocking
        let url = EXTERNAL_ASSETS_API.replace("{}", app_id);
        let client = reqwest::blocking::Client::new();

        let body = serde_json::json!({
            "urls": [image_url]
        });

        let response = match client
            .post(&url)
            .header("Authorization", format!("Bearer {}", token))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
        {
            Ok(resp) => resp,
            Err(e) => {
                warn!("external-assets: HTTP request failed: {}", e);
                return None;
            }
        };

        let status = response.status();
        let response_text = match response.text() {
            Ok(t) => t,
            Err(e) => {
                warn!("external-assets: failed to read response body: {}", e);
                return None;
            }
        };

        if !status.is_success() || response_text.is_empty() {
            warn!(
                "external-assets: HTTP {} for {}: {}",
                status,
                &image_url[..60.min(image_url.len())],
                &response_text[..200.min(response_text.len())]
            );
            return None;
        }

        // Parse response: array of { "url": "...", "external_asset_path": "..." }
        let parsed: Vec<serde_json::Value> = match serde_json::from_str(&response_text) {
            Ok(v) => v,
            Err(e) => {
                warn!("external-assets: failed to parse response: {}", e);
                return None;
            }
        };

        let asset_path = parsed
            .first()
            .and_then(|v| v.get("external_asset_path"))
            .and_then(|v| v.as_str());

        if let Some(path) = asset_path {
            let result = format!("mp:{}", path);
            // Cache it
            {
                let mut cache = self.cache.lock().unwrap();
                cache.insert(image_url.to_string(), result.clone());
                // Trim cache if too large
                if cache.len() > CACHE_MAX_SIZE {
                    let keys: Vec<String> = cache.keys().take(10).cloned().collect();
                    for k in keys {
                        cache.remove(&k);
                    }
                }
            }
            debug!("external-assets: resolved {} -> {}", &image_url[..60.min(image_url.len())], result);
            Some(result)
        } else {
            warn!(
                "external-assets: no external_asset_path in response for {}: {}",
                &image_url[..60.min(image_url.len())],
                &response_text[..200.min(response_text.len())]
            );
            None
        }
    }

    pub fn clear_cache(&self) {
        let mut cache = self.cache.lock().unwrap();
        cache.clear();
    }
}