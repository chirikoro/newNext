//! Caching Layer for Hayabusa.
//!
//! In-memory TTL cache and Redis-compatible client interface.
//!
//! ```ignore
//! use hayabusa_core::prelude::*;
//!
//! let cache = MemoryCache::new();
//! cache.set("key", "value", Duration::from_secs(60));
//! let val = cache.get("key");
//! ```

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use dashmap::DashMap;

// ─── Memory Cache ───────────────────────────────────────────

/// Thread-safe in-memory cache with TTL expiration
#[derive(Debug, Clone)]
pub struct MemoryCache {
    store: Arc<DashMap<String, CacheEntry>>,
    default_ttl: Duration,
    max_entries: usize,
}

#[derive(Debug, Clone)]
struct CacheEntry {
    value: String,
    expires_at: Instant,
    created_at: Instant,
    hits: u64,
}

impl MemoryCache {
    pub fn new() -> Self {
        Self {
            store: Arc::new(DashMap::new()),
            default_ttl: Duration::from_secs(300),
            max_entries: 10_000,
        }
    }

    pub fn with_ttl(mut self, ttl: Duration) -> Self {
        self.default_ttl = ttl;
        self
    }

    pub fn with_max_entries(mut self, max: usize) -> Self {
        self.max_entries = max;
        self
    }

    /// Set a value with the default TTL
    pub fn set(&self, key: &str, value: &str, ttl: Duration) {
        self.evict_expired();
        if self.store.len() >= self.max_entries {
            self.evict_lru();
        }
        let now = Instant::now();
        self.store.insert(key.to_string(), CacheEntry {
            value: value.to_string(),
            expires_at: now + ttl,
            created_at: now,
            hits: 0,
        });
    }

    /// Set with default TTL
    pub fn set_default(&self, key: &str, value: &str) {
        self.set(key, value, self.default_ttl);
    }

    /// Get a value (returns None if expired)
    pub fn get(&self, key: &str) -> Option<String> {
        let mut entry = self.store.get_mut(key)?;
        if entry.expires_at < Instant::now() {
            drop(entry);
            self.store.remove(key);
            return None;
        }
        entry.hits += 1;
        Some(entry.value.clone())
    }

    /// Check if a key exists and is not expired
    pub fn has(&self, key: &str) -> bool {
        self.get(key).is_some()
    }

    /// Delete a key
    pub fn delete(&self, key: &str) -> bool {
        self.store.remove(key).is_some()
    }

    /// Clear all entries
    pub fn clear(&self) {
        self.store.clear();
    }

    /// Get the number of (non-expired) entries
    pub fn len(&self) -> usize {
        self.evict_expired();
        self.store.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Get or set: returns cached value, or computes and caches it
    pub fn get_or_set(&self, key: &str, ttl: Duration, compute: impl FnOnce() -> String) -> String {
        if let Some(val) = self.get(key) {
            return val;
        }
        let val = compute();
        self.set(key, &val, ttl);
        val
    }

    /// Get cache statistics
    pub fn stats(&self) -> CacheStats {
        let mut total_hits = 0u64;
        let mut expired = 0usize;
        let now = Instant::now();
        for entry in self.store.iter() {
            total_hits += entry.hits;
            if entry.expires_at < now {
                expired += 1;
            }
        }
        CacheStats {
            entries: self.store.len(),
            max_entries: self.max_entries,
            expired,
            total_hits,
        }
    }

    fn evict_expired(&self) {
        let now = Instant::now();
        self.store.retain(|_, v| v.expires_at > now);
    }

    fn evict_lru(&self) {
        // Remove the entry with the least hits
        if let Some(min_key) = self.store.iter()
            .min_by_key(|e| e.hits)
            .map(|e| e.key().clone())
        {
            self.store.remove(&min_key);
        }
    }
}

impl Default for MemoryCache {
    fn default() -> Self { Self::new() }
}

/// Cache statistics
#[derive(Debug, Clone)]
pub struct CacheStats {
    pub entries: usize,
    pub max_entries: usize,
    pub expired: usize,
    pub total_hits: u64,
}

impl CacheStats {
    pub fn to_json(&self) -> String {
        format!(
            "{{\"entries\":{},\"max_entries\":{},\"expired\":{},\"total_hits\":{}}}",
            self.entries, self.max_entries, self.expired, self.total_hits
        )
    }
}

// ─── Redis Client ───────────────────────────────────────────

/// Redis-compatible client (uses reqwest for Redis HTTP APIs like Upstash/Vercel KV)
#[derive(Debug, Clone)]
pub struct RedisClient {
    pub url: String,
    pub token: Option<String>,
}

impl RedisClient {
    /// Connect to Upstash Redis (HTTP API)
    pub fn upstash(url: &str, token: &str) -> Self {
        Self { url: url.trim_end_matches('/').to_string(), token: Some(token.to_string()) }
    }

    /// Connect from environment variables (KV_REST_API_URL, KV_REST_API_TOKEN)
    pub fn from_env() -> Option<Self> {
        let url = std::env::var("KV_REST_API_URL").ok()?;
        let token = std::env::var("KV_REST_API_TOKEN").ok()?;
        Some(Self::upstash(&url, &token))
    }

    /// Build a Redis command as an HTTP request body
    pub fn command_body(args: &[&str]) -> String {
        let arr: Vec<String> = args.iter().map(|a| format!("\"{}\"", a)).collect();
        format!("[{}]", arr.join(","))
    }

    /// GET command body
    pub fn get_body(key: &str) -> String {
        Self::command_body(&["GET", key])
    }

    /// SET command body with optional EX (expiry in seconds)
    pub fn set_body(key: &str, value: &str, ex: Option<u64>) -> String {
        if let Some(secs) = ex {
            Self::command_body(&["SET", key, value, "EX", &secs.to_string()])
        } else {
            Self::command_body(&["SET", key, value])
        }
    }

    /// DEL command body
    pub fn del_body(key: &str) -> String {
        Self::command_body(&["DEL", key])
    }

    /// INCR command body
    pub fn incr_body(key: &str) -> String {
        Self::command_body(&["INCR", key])
    }

    /// EXPIRE command body
    pub fn expire_body(key: &str, secs: u64) -> String {
        Self::command_body(&["EXPIRE", key, &secs.to_string()])
    }

    /// TTL command body
    pub fn ttl_body(key: &str) -> String {
        Self::command_body(&["TTL", key])
    }

    /// HSET command body
    pub fn hset_body(key: &str, field: &str, value: &str) -> String {
        Self::command_body(&["HSET", key, field, value])
    }

    /// HGET command body
    pub fn hget_body(key: &str, field: &str) -> String {
        Self::command_body(&["HGET", key, field])
    }

    /// API endpoint URL
    pub fn endpoint(&self) -> String {
        format!("{}/", self.url)
    }

    /// Authorization header
    pub fn auth_header(&self) -> Option<(String, String)> {
        self.token.as_ref().map(|t| ("Authorization".to_string(), format!("Bearer {}", t)))
    }
}

// ─── Cache Middleware Helper ────────────────────────────────

/// HTTP response cache key generator
pub fn cache_key(method: &str, path: &str, vary_headers: &[(&str, &str)]) -> String {
    let mut key = format!("{}:{}", method, path);
    for (name, value) in vary_headers {
        key.push_str(&format!(":{}={}", name, value));
    }
    key
}

/// Generate Cache-Control header value
pub fn cache_control(max_age: u64, stale_while_revalidate: Option<u64>, public: bool) -> String {
    let visibility = if public { "public" } else { "private" };
    let mut header = format!("{}, max-age={}", visibility, max_age);
    if let Some(swr) = stale_while_revalidate {
        header.push_str(&format!(", stale-while-revalidate={}", swr));
    }
    header
}

// ─── Tests ──────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_set_get() {
        let cache = MemoryCache::new();
        cache.set("key", "value", Duration::from_secs(60));
        assert_eq!(cache.get("key"), Some("value".to_string()));
    }

    #[test]
    fn test_cache_miss() {
        let cache = MemoryCache::new();
        assert_eq!(cache.get("nonexistent"), None);
    }

    #[test]
    fn test_cache_expired() {
        let cache = MemoryCache::new();
        cache.set("key", "value", Duration::from_millis(1));
        std::thread::sleep(Duration::from_millis(10));
        assert_eq!(cache.get("key"), None);
    }

    #[test]
    fn test_cache_delete() {
        let cache = MemoryCache::new();
        cache.set("key", "value", Duration::from_secs(60));
        assert!(cache.delete("key"));
        assert!(!cache.has("key"));
    }

    #[test]
    fn test_cache_clear() {
        let cache = MemoryCache::new();
        cache.set("a", "1", Duration::from_secs(60));
        cache.set("b", "2", Duration::from_secs(60));
        cache.clear();
        assert!(cache.is_empty());
    }

    #[test]
    fn test_cache_get_or_set() {
        let cache = MemoryCache::new();
        let v1 = cache.get_or_set("k", Duration::from_secs(60), || "computed".to_string());
        assert_eq!(v1, "computed");
        let v2 = cache.get_or_set("k", Duration::from_secs(60), || "should_not_run".to_string());
        assert_eq!(v2, "computed");
    }

    #[test]
    fn test_cache_stats() {
        let cache = MemoryCache::new();
        cache.set("a", "1", Duration::from_secs(60));
        cache.get("a");
        cache.get("a");
        let stats = cache.stats();
        assert_eq!(stats.entries, 1);
        assert_eq!(stats.total_hits, 2);
    }

    #[test]
    fn test_cache_max_entries() {
        let cache = MemoryCache::new().with_max_entries(2);
        cache.set("a", "1", Duration::from_secs(60));
        cache.set("b", "2", Duration::from_secs(60));
        cache.set("c", "3", Duration::from_secs(60)); // should evict one
        assert!(cache.len() <= 2);
    }

    #[test]
    fn test_redis_command_body() {
        let body = RedisClient::command_body(&["SET", "key", "value"]);
        assert_eq!(body, "[\"SET\",\"key\",\"value\"]");
    }

    #[test]
    fn test_redis_set_with_expiry() {
        let body = RedisClient::set_body("key", "val", Some(60));
        assert!(body.contains("EX"));
        assert!(body.contains("60"));
    }

    #[test]
    fn test_redis_hset() {
        let body = RedisClient::hset_body("hash", "field", "value");
        assert!(body.contains("HSET"));
    }

    #[test]
    fn test_redis_client() {
        let client = RedisClient::upstash("https://redis.example.com", "token123");
        assert_eq!(client.endpoint(), "https://redis.example.com/");
        assert!(client.auth_header().unwrap().1.contains("token123"));
    }

    #[test]
    fn test_cache_key() {
        let key = cache_key("GET", "/api/users", &[("Accept-Language", "ja")]);
        assert_eq!(key, "GET:/api/users:Accept-Language=ja");
    }

    #[test]
    fn test_cache_control() {
        let cc = cache_control(3600, Some(60), true);
        assert_eq!(cc, "public, max-age=3600, stale-while-revalidate=60");
    }

    #[test]
    fn test_cache_control_private() {
        let cc = cache_control(0, None, false);
        assert_eq!(cc, "private, max-age=0");
    }

    #[test]
    fn test_cache_has() {
        let cache = MemoryCache::new();
        cache.set("x", "y", Duration::from_secs(60));
        assert!(cache.has("x"));
        assert!(!cache.has("z"));
    }

    #[test]
    fn test_stats_json() {
        let stats = CacheStats { entries: 5, max_entries: 100, expired: 1, total_hits: 42 };
        let json = stats.to_json();
        assert!(json.contains("\"entries\":5"));
        assert!(json.contains("\"total_hits\":42"));
    }
}
