//! Rate Limiting for Hayabusa.
//!
//! Token bucket algorithm for per-IP or per-route rate limiting.
//! Prevents abuse without blocking legitimate users.
//!
//! ## Usage
//! ```ignore
//! let limiter = RateLimiter::new()
//!     .default_limit(100, Duration::from_secs(60))  // 100 req/min globally
//!     .route_limit("/api/*", 30, Duration::from_secs(60))  // 30 req/min for API
//!     .route_limit("/login", 5, Duration::from_secs(300)); // 5 req/5min for login
//! ```

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Rate limiter using token bucket algorithm
#[derive(Clone)]
pub struct RateLimiter {
    /// Default rate limit
    default: RateLimit,
    /// Per-route rate limits
    route_limits: Vec<(String, RateLimit)>,
    /// Token buckets per client key
    buckets: Arc<Mutex<HashMap<String, TokenBucket>>>,
}

/// Rate limit configuration
#[derive(Debug, Clone, Copy)]
pub struct RateLimit {
    /// Maximum number of requests in the window
    pub max_requests: u64,
    /// Time window duration
    pub window: Duration,
}

/// Token bucket state for a single client
#[derive(Debug, Clone)]
struct TokenBucket {
    tokens: f64,
    max_tokens: f64,
    refill_rate: f64, // tokens per second
    last_refill: Instant,
}

impl TokenBucket {
    fn new(max_tokens: u64, window: Duration) -> Self {
        let refill_rate = max_tokens as f64 / window.as_secs_f64();
        Self {
            tokens: max_tokens as f64,
            max_tokens: max_tokens as f64,
            refill_rate,
            last_refill: Instant::now(),
        }
    }

    fn try_consume(&mut self) -> bool {
        // Refill tokens based on elapsed time
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_refill).as_secs_f64();
        self.tokens = (self.tokens + elapsed * self.refill_rate).min(self.max_tokens);
        self.last_refill = now;

        if self.tokens >= 1.0 {
            self.tokens -= 1.0;
            true
        } else {
            false
        }
    }

    fn remaining(&self) -> u64 {
        self.tokens as u64
    }

    fn retry_after(&self) -> Duration {
        if self.tokens >= 1.0 {
            Duration::from_secs(0)
        } else {
            let needed = 1.0 - self.tokens;
            Duration::from_secs_f64(needed / self.refill_rate)
        }
    }
}

/// Result of a rate limit check
#[derive(Debug, Clone)]
pub struct RateLimitResult {
    /// Whether the request is allowed
    pub allowed: bool,
    /// Remaining requests in the current window
    pub remaining: u64,
    /// The rate limit that was applied
    pub limit: u64,
    /// Time until the next token is available (if blocked)
    pub retry_after: Duration,
}

impl RateLimiter {
    pub fn new() -> Self {
        Self {
            default: RateLimit {
                max_requests: 100,
                window: Duration::from_secs(60),
            },
            route_limits: Vec::new(),
            buckets: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Set the default rate limit
    pub fn default_limit(mut self, max_requests: u64, window: Duration) -> Self {
        self.default = RateLimit {
            max_requests,
            window,
        };
        self
    }

    /// Set a rate limit for a specific route pattern
    pub fn route_limit(
        mut self,
        pattern: impl Into<String>,
        max_requests: u64,
        window: Duration,
    ) -> Self {
        self.route_limits.push((
            pattern.into(),
            RateLimit {
                max_requests,
                window,
            },
        ));
        self
    }

    /// Check if a request is allowed and consume a token
    pub fn check(&self, client_key: &str, path: &str) -> RateLimitResult {
        let limit = self.get_limit_for_path(path);
        let bucket_key = format!("{}:{}", client_key, self.get_bucket_prefix(path));

        let mut buckets = self.buckets.lock().unwrap();
        let bucket = buckets
            .entry(bucket_key)
            .or_insert_with(|| TokenBucket::new(limit.max_requests, limit.window));

        let allowed = bucket.try_consume();
        let remaining = bucket.remaining();
        let retry_after = bucket.retry_after();

        RateLimitResult {
            allowed,
            remaining,
            limit: limit.max_requests,
            retry_after,
        }
    }

    /// Get the rate limit for a path (most specific match wins)
    fn get_limit_for_path(&self, path: &str) -> RateLimit {
        for (pattern, limit) in &self.route_limits {
            if path_matches_pattern(path, pattern) {
                return *limit;
            }
        }
        self.default
    }

    /// Get the bucket prefix for a path
    fn get_bucket_prefix(&self, path: &str) -> String {
        for (pattern, _) in &self.route_limits {
            if path_matches_pattern(path, pattern) {
                return pattern.clone();
            }
        }
        "__default__".to_string()
    }

    /// Clean up expired buckets to prevent memory leaks
    pub fn cleanup(&self, max_age: Duration) {
        let mut buckets = self.buckets.lock().unwrap();
        let now = Instant::now();
        buckets.retain(|_, bucket| {
            now.duration_since(bucket.last_refill) < max_age
        });
    }

    /// Render rate limit headers for a response
    pub fn render_headers(result: &RateLimitResult) -> Vec<(String, String)> {
        let mut headers = vec![
            ("x-ratelimit-limit".to_string(), result.limit.to_string()),
            (
                "x-ratelimit-remaining".to_string(),
                result.remaining.to_string(),
            ),
        ];

        if !result.allowed {
            headers.push((
                "retry-after".to_string(),
                result.retry_after.as_secs().max(1).to_string(),
            ));
        }

        headers
    }
}

impl Default for RateLimiter {
    fn default() -> Self {
        Self::new()
    }
}

/// Simple glob-like path matching
fn path_matches_pattern(path: &str, pattern: &str) -> bool {
    if pattern.ends_with('*') {
        let prefix = &pattern[..pattern.len() - 1];
        path.starts_with(prefix)
    } else {
        path == pattern
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rate_limiter_allows_within_limit() {
        let limiter = RateLimiter::new()
            .default_limit(5, Duration::from_secs(60));

        for _ in 0..5 {
            let result = limiter.check("192.168.1.1", "/");
            assert!(result.allowed);
        }
    }

    #[test]
    fn test_rate_limiter_blocks_over_limit() {
        let limiter = RateLimiter::new()
            .default_limit(3, Duration::from_secs(60));

        for _ in 0..3 {
            limiter.check("192.168.1.1", "/");
        }

        let result = limiter.check("192.168.1.1", "/");
        assert!(!result.allowed);
        assert!(result.retry_after > Duration::from_secs(0));
    }

    #[test]
    fn test_per_route_limit() {
        let limiter = RateLimiter::new()
            .default_limit(100, Duration::from_secs(60))
            .route_limit("/api/*", 2, Duration::from_secs(60));

        // API route has lower limit
        limiter.check("user1", "/api/data");
        limiter.check("user1", "/api/data");
        let result = limiter.check("user1", "/api/data");
        assert!(!result.allowed);

        // Non-API route still has high limit
        let result = limiter.check("user1", "/about");
        assert!(result.allowed);
    }

    #[test]
    fn test_different_clients_separate_buckets() {
        let limiter = RateLimiter::new()
            .default_limit(2, Duration::from_secs(60));

        limiter.check("client1", "/");
        limiter.check("client1", "/");
        let r1 = limiter.check("client1", "/");
        assert!(!r1.allowed);

        // Different client has own bucket
        let r2 = limiter.check("client2", "/");
        assert!(r2.allowed);
    }

    #[test]
    fn test_rate_limit_headers() {
        let result = RateLimitResult {
            allowed: false,
            remaining: 0,
            limit: 100,
            retry_after: Duration::from_secs(30),
        };

        let headers = RateLimiter::render_headers(&result);
        assert!(headers.iter().any(|(k, v)| k == "x-ratelimit-limit" && v == "100"));
        assert!(headers.iter().any(|(k, v)| k == "x-ratelimit-remaining" && v == "0"));
        assert!(headers.iter().any(|(k, _)| k == "retry-after"));
    }

    #[test]
    fn test_remaining_count() {
        let limiter = RateLimiter::new()
            .default_limit(5, Duration::from_secs(60));

        let r = limiter.check("user", "/");
        assert_eq!(r.remaining, 4);
        let r = limiter.check("user", "/");
        assert_eq!(r.remaining, 3);
    }

    #[test]
    fn test_path_matching() {
        assert!(path_matches_pattern("/api/users", "/api/*"));
        assert!(path_matches_pattern("/api/", "/api/*"));
        assert!(!path_matches_pattern("/about", "/api/*"));
        assert!(path_matches_pattern("/login", "/login"));
        assert!(!path_matches_pattern("/login/reset", "/login"));
    }
}
