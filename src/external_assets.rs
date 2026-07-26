use std::collections::HashMap;
use std::sync::Mutex;
use log::warn;

const EXTERNAL_ASSETS_URL: &str =
    "https://discord.com/api/v9/applications/{}/external-assets";
const DEFAULT_CACHE_CAPACITY: usize = 4096;
const DEFAULT_CACHE_EVICT_BATCH: usize = 512;

/// Resolves external image URLs to Discord's `mp:` format.
///
/// Uses the `POST /api/v9/applications/{id}/external-assets` endpoint.
/// Entries are cached with LRU eviction.
pub struct ExternalAssetsResolver {
    cache: Mutex<HashMap<String, String>>,
    /// LRU order: front = least recently used, back = most recently used.
    lru_order: Mutex<Vec<String>>,
    capacity: usize,
    /// Maximum number of entries to evict in one pass. Clamped to `capacity`.
    evict_batch: usize,
}

impl Default for ExternalAssetsResolver {
    fn default() -> Self {
        Self::new(DEFAULT_CACHE_CAPACITY, DEFAULT_CACHE_EVICT_BATCH)
    }
}

impl ExternalAssetsResolver {
    pub fn new(capacity: usize, evict_batch: usize) -> Self {
        let evict_batch = evict_batch.min(capacity);
        Self {
            cache: Mutex::new(HashMap::with_capacity(capacity)),
            lru_order: Mutex::new(Vec::with_capacity(capacity)),
            capacity,
            evict_batch,
        }
    }

    pub fn resolve(&self, image_url: &str, app_id: &str, token: &str) -> Option<String> {
        if image_url.is_empty() {
            return None;
        }
        if image_url.starts_with("mp:") {
            return Some(image_url.to_string());
        }

        if let Some(resolved) = self.check_cache(image_url) {
            self.promote(image_url);
            return Some(resolved);
        }

        let external_asset_path = Self::fetch_external_asset_path(image_url, app_id, token)?;
        let result = format!("mp:{external_asset_path}");
        self.update_cache(image_url, &result);
        Some(result)
    }

    fn check_cache(&self, image_url: &str) -> Option<String> {
        let cache = self.cache.lock().ok()?;
        cache.get(image_url).cloned()
    }

    fn promote(&self, image_url: &str) {
        if let Ok(mut order) = self.lru_order.lock() {
            if let Some(pos) = order.iter().position(|k| k == image_url) {
                if pos + 1 < order.len() {
                    order.remove(pos);
                    order.push(image_url.to_string());
                }
            }
        }
    }

    fn fetch_external_asset_path(
        image_url: &str,
        app_id: &str,
        token: &str,
    ) -> Option<String> {
        let url = EXTERNAL_ASSETS_URL.replace("{}", app_id);
        let client = reqwest::blocking::Client::new();
        let body = serde_json::json!({ "urls": [image_url] });

        let response = client
            .post(&url)
            .header("Authorization", format!("Bearer {token}"))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .map_err(|e| {
                warn!("external-assets: request failed: {e}");
            })
            .ok()?;

        let status = response.status();
        let response_text = response.text().ok()?;

        if !status.is_success() || response_text.is_empty() {
            warn!(
                "external-assets: HTTP {status} for {image_url} with response: {response_text}"
            );
            return None;
        }

        let parsed: Vec<serde_json::Value> = serde_json::from_str(&response_text).ok()?;
        let path = parsed.first()?.get("external_asset_path")?.as_str()?;
        Some(path.to_string())
    }

    fn update_cache(&self, image_url: &str, result: &str) {
        if let Ok(mut cache) = self.cache.lock() {
            if !cache.contains_key(image_url) {
                cache.insert(image_url.to_string(), result.to_string());
                if let Ok(mut order) = self.lru_order.lock() {
                    order.push(image_url.to_string());
                }
            }

            if cache.len() > self.capacity {
                if let Ok(mut order) = self.lru_order.lock() {
                    let batch = self.evict_batch.min(order.len());
                    let to_evict: Vec<String> = order.drain(..batch).collect();
                    for k in to_evict {
                        cache.remove(&k);
                    }
                }
            }
        }
    }
}