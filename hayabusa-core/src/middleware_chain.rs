//! Per-route middleware chain for Hayabusa.
//!
//! Provides Next.js `middleware.ts`-like functionality:
//! - Authentication checks
//! - URL redirects and rewrites
//! - Response header modification
//! - Geo-based routing
//! - Rate limiting
//! - Request logging
//!
//! Middleware runs BEFORE the page handler and can:
//! 1. Pass through (continue to next middleware or handler)
//! 2. Redirect to another URL
//! 3. Rewrite the request (change the path without redirect)
//! 4. Return a response directly (block the request)

use axum::response::Response;
use http::StatusCode;
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

/// Request context available to middleware
#[derive(Debug, Clone)]
pub struct MiddlewareRequest {
    /// The request path
    pub path: String,
    /// The request method
    pub method: String,
    /// Request headers (read-only snapshot)
    pub headers: HashMap<String, String>,
    /// Cookies
    pub cookies: HashMap<String, String>,
    /// Query parameters
    pub query: HashMap<String, String>,
    /// Geo information (if available from CDN headers)
    pub geo: Option<GeoInfo>,
}

/// Geographic information from CDN headers
#[derive(Debug, Clone, Default)]
pub struct GeoInfo {
    pub country: Option<String>,
    pub region: Option<String>,
    pub city: Option<String>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
}

/// The result of middleware execution
#[derive(Debug, Clone)]
pub enum MiddlewareResult {
    /// Continue to the next middleware or handler
    Next,
    /// Continue but with modified headers added to the response
    NextWithHeaders(Vec<(String, String)>),
    /// Redirect to a URL (default 307 Temporary Redirect)
    Redirect { url: String, permanent: bool },
    /// Rewrite: change the request path without redirecting
    Rewrite(String),
    /// Return a response directly (e.g., 403 Forbidden)
    Response { status: u16, body: String },
}

/// Type alias for middleware handler functions
pub type MiddlewareHandler = Arc<
    dyn Fn(MiddlewareRequest) -> Pin<Box<dyn Future<Output = MiddlewareResult> + Send>>
        + Send
        + Sync,
>;

/// Middleware chain configuration
pub struct MiddlewareChain {
    /// Middleware handlers with their path matchers
    entries: Vec<MiddlewareEntry>,
}

struct MiddlewareEntry {
    /// Path patterns this middleware applies to (e.g., "/dashboard/:path*")
    matcher: PathMatcher,
    /// The middleware handler
    handler: MiddlewareHandler,
}

/// Path matching for middleware
#[derive(Debug, Clone)]
pub enum PathMatcher {
    /// Match an exact path
    Exact(String),
    /// Match a path prefix (e.g., "/dashboard" matches "/dashboard/settings")
    Prefix(String),
    /// Match all paths
    All,
    /// Match paths by regex-like pattern
    Pattern(String),
    /// Match specific paths
    OneOf(Vec<String>),
}

impl PathMatcher {
    fn matches(&self, path: &str) -> bool {
        match self {
            PathMatcher::Exact(p) => path == p,
            PathMatcher::Prefix(p) => path.starts_with(p),
            PathMatcher::All => true,
            PathMatcher::Pattern(pattern) => {
                // Simple glob-like matching: * matches any segment
                let pattern_parts: Vec<&str> = pattern.split('/').collect();
                let path_parts: Vec<&str> = path.split('/').collect();

                if pattern_parts.len() > path_parts.len() {
                    return false;
                }

                for (pp, pathp) in pattern_parts.iter().zip(path_parts.iter()) {
                    if *pp == "*" || *pp == ":path*" {
                        return true; // wildcard matches rest
                    }
                    if pp != pathp {
                        return false;
                    }
                }

                pattern_parts.len() == path_parts.len()
            }
            PathMatcher::OneOf(paths) => paths.iter().any(|p| path == p || path.starts_with(p)),
        }
    }
}

impl MiddlewareChain {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    /// Add a middleware that applies to all routes
    pub fn use_global(
        self,
        handler: impl Fn(MiddlewareRequest) -> Pin<Box<dyn Future<Output = MiddlewareResult> + Send>>
            + Send
            + Sync
            + 'static,
    ) -> Self {
        self.use_for(PathMatcher::All, handler)
    }

    /// Add a middleware for a specific path prefix
    pub fn use_prefix(
        self,
        prefix: impl Into<String>,
        handler: impl Fn(MiddlewareRequest) -> Pin<Box<dyn Future<Output = MiddlewareResult> + Send>>
            + Send
            + Sync
            + 'static,
    ) -> Self {
        self.use_for(PathMatcher::Prefix(prefix.into()), handler)
    }

    /// Add a middleware for a specific path matcher
    pub fn use_for(
        mut self,
        matcher: PathMatcher,
        handler: impl Fn(MiddlewareRequest) -> Pin<Box<dyn Future<Output = MiddlewareResult> + Send>>
            + Send
            + Sync
            + 'static,
    ) -> Self {
        self.entries.push(MiddlewareEntry {
            matcher,
            handler: Arc::new(handler),
        });
        self
    }

    /// Execute the middleware chain for a given request.
    /// Returns the final MiddlewareResult after running all matching middleware.
    pub async fn execute(&self, request: &MiddlewareRequest) -> MiddlewareResult {
        let mut extra_headers: Vec<(String, String)> = Vec::new();

        for entry in &self.entries {
            if !entry.matcher.matches(&request.path) {
                continue;
            }

            let result = (entry.handler)(request.clone()).await;

            match result {
                MiddlewareResult::Next => continue,
                MiddlewareResult::NextWithHeaders(headers) => {
                    extra_headers.extend(headers);
                    continue;
                }
                // Redirect, Rewrite, or Response: stop the chain
                other => return other,
            }
        }

        if extra_headers.is_empty() {
            MiddlewareResult::Next
        } else {
            MiddlewareResult::NextWithHeaders(extra_headers)
        }
    }
}

impl Default for MiddlewareChain {
    fn default() -> Self {
        Self::new()
    }
}

/// Convert a MiddlewareResult into an axum Response (for redirect/response results)
pub fn middleware_result_to_response(result: &MiddlewareResult) -> Option<Response> {
    match result {
        MiddlewareResult::Redirect { url, permanent } => {
            let status = if *permanent {
                StatusCode::PERMANENT_REDIRECT
            } else {
                StatusCode::TEMPORARY_REDIRECT
            };
            Some(
                Response::builder()
                    .status(status)
                    .header("location", url.as_str())
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
        }
        MiddlewareResult::Response { status, body } => Some(
            Response::builder()
                .status(StatusCode::from_u16(*status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR))
                .header("content-type", "text/plain; charset=utf-8")
                .body(axum::body::Body::from(body.clone()))
                .unwrap(),
        ),
        _ => None,
    }
}

/// Parse cookies from a Cookie header value
pub fn parse_cookies(cookie_header: &str) -> HashMap<String, String> {
    cookie_header
        .split(';')
        .filter_map(|pair| {
            let mut parts = pair.trim().splitn(2, '=');
            let key = parts.next()?.trim().to_string();
            let value = parts.next().unwrap_or("").trim().to_string();
            Some((key, value))
        })
        .collect()
}

/// Extract geo information from common CDN headers
pub fn extract_geo(headers: &HashMap<String, String>) -> Option<GeoInfo> {
    // Check for Cloudflare headers
    let country = headers
        .get("cf-ipcountry")
        .or_else(|| headers.get("x-vercel-ip-country"))
        .cloned();

    let city = headers
        .get("cf-ipcity")
        .or_else(|| headers.get("x-vercel-ip-city"))
        .cloned();

    let region = headers
        .get("x-vercel-ip-country-region")
        .cloned();

    let latitude = headers
        .get("x-vercel-ip-latitude")
        .and_then(|v| v.parse().ok());

    let longitude = headers
        .get("x-vercel-ip-longitude")
        .and_then(|v| v.parse().ok());

    if country.is_some() || city.is_some() {
        Some(GeoInfo {
            country,
            region,
            city,
            latitude,
            longitude,
        })
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_path_matcher_exact() {
        let m = PathMatcher::Exact("/about".to_string());
        assert!(m.matches("/about"));
        assert!(!m.matches("/about/team"));
        assert!(!m.matches("/"));
    }

    #[test]
    fn test_path_matcher_prefix() {
        let m = PathMatcher::Prefix("/dashboard".to_string());
        assert!(m.matches("/dashboard"));
        assert!(m.matches("/dashboard/settings"));
        assert!(!m.matches("/about"));
    }

    #[test]
    fn test_path_matcher_all() {
        let m = PathMatcher::All;
        assert!(m.matches("/anything"));
        assert!(m.matches("/"));
    }

    #[test]
    fn test_parse_cookies() {
        let cookies = parse_cookies("session=abc123; theme=dark; lang=en");
        assert_eq!(cookies.get("session"), Some(&"abc123".to_string()));
        assert_eq!(cookies.get("theme"), Some(&"dark".to_string()));
        assert_eq!(cookies.get("lang"), Some(&"en".to_string()));
    }

    #[tokio::test]
    async fn test_middleware_chain_redirect() {
        let chain = MiddlewareChain::new()
            .use_prefix("/admin", |_req| {
                Box::pin(async move {
                    MiddlewareResult::Redirect {
                        url: "/login".to_string(),
                        permanent: false,
                    }
                })
            });

        let req = MiddlewareRequest {
            path: "/admin/dashboard".to_string(),
            method: "GET".to_string(),
            headers: HashMap::new(),
            cookies: HashMap::new(),
            query: HashMap::new(),
            geo: None,
        };

        let result = chain.execute(&req).await;
        match result {
            MiddlewareResult::Redirect { url, permanent } => {
                assert_eq!(url, "/login");
                assert!(!permanent);
            }
            _ => panic!("Expected redirect"),
        }
    }

    #[tokio::test]
    async fn test_middleware_chain_pass_through() {
        let chain = MiddlewareChain::new()
            .use_prefix("/admin", |_req| {
                Box::pin(async move {
                    MiddlewareResult::Redirect {
                        url: "/login".to_string(),
                        permanent: false,
                    }
                })
            });

        let req = MiddlewareRequest {
            path: "/about".to_string(),
            method: "GET".to_string(),
            headers: HashMap::new(),
            cookies: HashMap::new(),
            query: HashMap::new(),
            geo: None,
        };

        let result = chain.execute(&req).await;
        assert!(matches!(result, MiddlewareResult::Next));
    }

    #[tokio::test]
    async fn test_middleware_chain_headers() {
        let chain = MiddlewareChain::new()
            .use_global(|_req| {
                Box::pin(async move {
                    MiddlewareResult::NextWithHeaders(vec![
                        ("x-custom".to_string(), "value".to_string()),
                    ])
                })
            });

        let req = MiddlewareRequest {
            path: "/any".to_string(),
            method: "GET".to_string(),
            headers: HashMap::new(),
            cookies: HashMap::new(),
            query: HashMap::new(),
            geo: None,
        };

        let result = chain.execute(&req).await;
        match result {
            MiddlewareResult::NextWithHeaders(headers) => {
                assert_eq!(headers.len(), 1);
                assert_eq!(headers[0].0, "x-custom");
            }
            _ => panic!("Expected NextWithHeaders"),
        }
    }
}
