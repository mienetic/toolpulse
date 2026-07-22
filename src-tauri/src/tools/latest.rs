//! Fetches the latest published version for each tool from public APIs.
//!
//! Each `LatestSource` variant has a bespoke extractor because the formats
//! differ wildly (JSON arrays, TOML, GitHub release JSON, nested objects).
//! Results are cached in-memory for an hour to avoid hammering the APIs.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use serde_json::Value;
use tokio::sync::Mutex;

use super::types::*;

/// In-memory TTL cache mapping tool name -> (latest version, fetched at).
const CACHE_TTL: Duration = Duration::from_secs(60 * 60); // 1 hour

#[derive(Default)]
pub struct LatestCache {
    entries: Mutex<HashMap<String, (String, Instant)>>,
}

impl LatestCache {
    pub fn new() -> Self {
        Self::default()
    }

    /// Return a cached value if it is still fresh.
    async fn get(&self, name: &str) -> Option<String> {
        let map = self.entries.lock().await;
        if let Some((v, at)) = map.get(name) {
            if at.elapsed() < CACHE_TTL {
                return Some(v.clone());
            }
        }
        None
    }

    async fn put(&self, name: &str, value: String) {
        let mut map = self.entries.lock().await;
        map.insert(name.to_string(), (value, Instant::now()));
    }
}

/// Resolve the latest version for one tool, using the cache when possible.
///
/// Returns `None` on any network/parse error — the UI treats `None` as
/// "unknown" rather than failing the whole scan.
pub async fn fetch_latest(
    def: &ToolDefinition,
    cache: &Arc<LatestCache>,
    client: &reqwest::Client,
) -> Option<String> {
    if let Some(v) = cache.get(&def.name).await {
        return Some(v);
    }
    match &def.latest {
        LatestSource::None => None,
        LatestSource::Json { url, pointer } => {
            // Rust ships a TOML channel file; parse it as text instead of JSON.
            if def.name == "rust" {
                let v = fetch_text(client, url)
                    .await
                    .ok()
                    .and_then(|body| extract_rust_version(&body));
                if let Some(ref v) = v {
                    cache.put(&def.name, v.clone()).await;
                }
                return v;
            }
            match fetch_json(client, url).await {
                Ok(value) => {
                    let v = extract_json(&value, pointer)
                        .or_else(|| extract_bespoke_json(&def.name, &value));
                    if let Some(ref v) = v {
                        cache.put(&def.name, v.clone()).await;
                    }
                    v
                }
                Err(_) => None,
            }
        }
        LatestSource::GitHub { repo } => {
            let url = format!("https://api.github.com/repos/{repo}/releases/latest");
            match fetch_json(client, &url).await {
                Ok(value) => {
                    let v = value.get("tag_name").and_then(|t| t.as_str()).map(|s| {
                        // GitHub tags often have a leading `v`/`bun-v` prefix.
                        s.trim_start_matches("bun-v").trim_start_matches('v').to_string()
                    });
                    if let Some(ref v) = v {
                        cache.put(&def.name, v.clone()).await;
                    }
                    v
                }
                Err(_) => None,
            }
        }
    }
}

async fn fetch_json(client: &reqwest::Client, url: &str) -> Result<Value, reqwest::Error> {
    client
        .get(url)
        .header("User-Agent", "toolpulse/0.1 (https://github.com)")
        .header("Accept", "application/json")
        .timeout(Duration::from_secs(10))
        .send()
        .await?
        .json::<Value>()
        .await
}

async fn fetch_text(client: &reqwest::Client, url: &str) -> Result<String, reqwest::Error> {
    client
        .get(url)
        .header("User-Agent", "toolpulse/0.1 (https://github.com)")
        .timeout(Duration::from_secs(10))
        .send()
        .await?
        .text()
        .await
}

/// Pull the stable Rust version out of the channel TOML.
///
/// The file contains a line like `rust_version = "1.88.0"`.
fn extract_rust_version(body: &str) -> Option<String> {
    for line in body.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("rust_version") {
            if let Some(q) = rest.find('"') {
                let after = &rest[q + 1..];
                if let Some(end) = after.find('"') {
                    return Some(after[..end].to_string());
                }
            }
        }
    }
    None
}

/// Walk a JSON pointer path (`["0","version"]`) to a leaf string.
fn extract_json(value: &Value, pointer: &[String]) -> Option<String> {
    if pointer.is_empty() {
        return None;
    }
    let mut cur = value;
    for key in pointer {
        cur = cur.get(key).or_else(|| {
            // numeric index into arrays
            key.parse::<usize>().ok().and_then(|i| cur.get(i))
        })?;
    }
    cur.as_str().map(|s| s.trim_start_matches('v').to_string())
}

/// Per-tool fallback extractors for formats that don't fit a clean pointer.
///
/// Rust ships a TOML channel file; Zig's index.json keys versions by label.
/// We parse both as JSON-adjacent text since the shapes are stable.
fn extract_bespoke_json(name: &str, value: &Value) -> Option<String> {
    match name {
        "rust" => {
            // The TOML file isn't valid JSON; fetch_json will have failed,
            // so this branch is a no-op for JSON parsing. We handle Rust via
            // a dedicated text fetch in fetch_latest_rust instead.
            None
        }
        "zig" => {
            // index.json: { "0.15.1": {...}, "master": {...} }
            value
                .as_object()
                .and_then(|o| {
                    o.keys()
                        .filter(|k| *k != "master")
                        .max_by(|a, b| compare_versions(a, b))
                        .cloned()
                })
        }
        _ => None,
    }
}

/// Best-effort dotted-version comparison used to pick the newest Zig release.
fn compare_versions(a: &str, b: &str) -> std::cmp::Ordering {
    let av: Vec<u64> = a.split('.').filter_map(|s| s.parse().ok()).collect();
    let bv: Vec<u64> = b.split('.').filter_map(|s| s.parse().ok()).collect();
    let n = av.len().max(bv.len());
    for i in 0..n {
        let x = av.get(i).copied().unwrap_or(0);
        let y = bv.get(i).copied().unwrap_or(0);
        match x.cmp(&y) {
            std::cmp::Ordering::Equal => continue,
            ord => return ord,
        }
    }
    std::cmp::Ordering::Equal
}
