use axum::Router;
use http::{header, HeaderName, HeaderValue};
use tower_http::{
    compression::CompressionLayer,
    cors::{Any, CorsLayer},
    services::ServeDir,
    set_header::SetResponseHeaderLayer,
};

/// Configuration for built-in middleware
#[derive(Debug, Clone)]
pub struct MiddlewareConfig {
    /// Enable gzip/brotli compression
    pub compression: bool,
    /// Enable CORS (permissive by default for dev)
    pub cors: bool,
    /// Static file directory (e.g., "public/")
    pub static_dir: Option<String>,
    /// Cache-Control header for static assets
    pub static_cache_max_age: u32,
    /// Security headers (X-Content-Type-Options, etc.)
    pub security_headers: bool,
    /// Content-Security-Policy header value (opt-in; setting CSP can break sites)
    pub content_security_policy: Option<String>,
    /// Referrer-Policy header value
    pub referrer_policy: Option<String>,
    /// Permissions-Policy header value
    pub permissions_policy: Option<String>,
}

impl Default for MiddlewareConfig {
    fn default() -> Self {
        Self {
            compression: true,
            cors: true,
            static_dir: Some("public".to_string()),
            static_cache_max_age: 3600,
            security_headers: true,
            content_security_policy: None,
            referrer_policy: Some("strict-origin-when-cross-origin".to_string()),
            permissions_policy: Some("camera=(), microphone=(), geolocation=()".to_string()),
        }
    }
}

/// Apply all configured middleware to the router
pub fn apply_middleware(router: Router, config: &MiddlewareConfig) -> Router {
    let mut router = router;

    // Serve static files
    if let Some(ref dir) = config.static_dir {
        router = router.fallback_service(ServeDir::new(dir));
    }

    // Compression (gzip + brotli)
    if config.compression {
        router = router.layer(CompressionLayer::new());
    }

    // CORS
    if config.cors {
        let cors = CorsLayer::new()
            .allow_origin(Any)
            .allow_methods(Any)
            .allow_headers(Any);
        router = router.layer(cors);
    }

    // Security headers
    if config.security_headers {
        router = router
            .layer(SetResponseHeaderLayer::overriding(
                header::X_CONTENT_TYPE_OPTIONS,
                HeaderValue::from_static("nosniff"),
            ))
            .layer(SetResponseHeaderLayer::overriding(
                header::X_FRAME_OPTIONS,
                HeaderValue::from_static("DENY"),
            ));

        // Content-Security-Policy（opt-in。設定すると壊れやすいので明示指定のみ）
        if let Some(csp) = config.content_security_policy.as_deref() {
            if let Ok(value) = HeaderValue::from_str(csp) {
                router = router.layer(SetResponseHeaderLayer::overriding(
                    HeaderName::from_static("content-security-policy"),
                    value,
                ));
            }
        }

        // Referrer-Policy
        if let Some(rp) = config.referrer_policy.as_deref() {
            if let Ok(value) = HeaderValue::from_str(rp) {
                router = router.layer(SetResponseHeaderLayer::overriding(
                    HeaderName::from_static("referrer-policy"),
                    value,
                ));
            }
        }

        // Permissions-Policy
        if let Some(pp) = config.permissions_policy.as_deref() {
            if let Ok(value) = HeaderValue::from_str(pp) {
                router = router.layer(SetResponseHeaderLayer::overriding(
                    HeaderName::from_static("permissions-policy"),
                    value,
                ));
            }
        }
    }

    router
}
