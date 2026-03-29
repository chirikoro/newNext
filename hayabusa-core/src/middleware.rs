use axum::Router;
use http::header;
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
}

impl Default for MiddlewareConfig {
    fn default() -> Self {
        Self {
            compression: true,
            cors: true,
            static_dir: Some("public".to_string()),
            static_cache_max_age: 3600,
            security_headers: true,
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
                header::HeaderValue::from_static("nosniff"),
            ))
            .layer(SetResponseHeaderLayer::overriding(
                header::X_FRAME_OPTIONS,
                header::HeaderValue::from_static("DENY"),
            ));
    }

    router
}
