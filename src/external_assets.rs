use std::collections::HashMap;
use std::sync::Mutex;
use log::warn;

const EXTERNAL_ASSETS_URL: &str =
    "https://discord.com/api/v9/applications/{}/external-assets";
const CACHE_CAPACITY: usize = 4096;
const CACHE_EVICT_BATCH: usize = 512;

/// Resolves external image URLs to Discord's `mp:` format.
///
/// Uses the `POST /api/v9/applications/{id}/external-assets` endpoint.
pub(crate) struct ExternalAssetsResolver {
    cache: Mutex<HashMap<String, String>>,
    insert_order: Mutex<Vec<String>>,
}

impl ExternalAssetsResolver {
    pub fn new() -> Self {
        Self {
            cache: Mutex::new(HashMap::new()),
            insert_order: Mutex::new(Vec::with_capacity(CACHE_CAPACITY)),
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
        if let Some(cached) = self.check_cache(image_url) {
            return Some(cached);
        }

        // Call external assets API
        let external_asset_path = self.fetch_external_asset_path(image_url, app_id, token)?;

        let result = format!("mp:{}", external_asset_path);

        // Update cache
        self.update_cache(image_url, &result);

        Some(result)
    }

    /// Check if the image URL is already cached.
    fn check_cache(&self, image_url: &str) -> Option<String> {
        self.cache.lock().ok()?.get(image_url).cloned()
    }

    /// Fetch an external asset path from Discord's API.
    fn fetch_external_asset_path(
        &self,
        image_url: &str,
        app_id: &str,
        token: &str,
    ) -> Option<String> {
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
                "external-assets: HTTP {} for {} with response: {}",
                status,
                &image_url,
                &response_text
            );
            return None;
        }

        let parsed: Vec<serde_json::Value> = serde_json::from_str(&response_text).ok()?;
        let path = parsed
            .first()?
            .get("external_asset_path")?
            .as_str()?;

        Some(path.to_string())
    }

    /// Insert a result into the cache, evicting the oldest entries if necessary.
    fn update_cache(&self, image_url: &str, result: &str) {
        if let Ok(mut cache) = self.cache.lock() {
            // If the key already exists, don't add duplicate to insert_order
            if !cache.contains_key(image_url) {
                cache.insert(image_url.to_string(), result.to_string());
                if let Ok(mut order) = self.insert_order.lock() {
                    order.push(image_url.to_string());
                }
            }

            // Evict oldest entries when cache exceeds capacity
            if cache.len() > CACHE_CAPACITY {
                if let Ok(mut order) = self.insert_order.lock() {
                    let to_evict: Vec<String> = order.drain(..CACHE_EVICT_BATCH).collect();
                    for k in &to_evict {
                        cache.remove(k);
                    }
                }
            }
        }
    }
}
