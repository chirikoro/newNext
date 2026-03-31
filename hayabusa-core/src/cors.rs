//! CORS (Cross-Origin Resource Sharing) for Hayabusa.
//!
//! Detailed CORS configuration with preflight response generation.
//!
//! ```ignore
//! use hayabusa_core::prelude::*;
//!
//! let cors = CorsConfig::new()
//!     .allow_origin("https://example.com")
//!     .allow_methods(&["GET", "POST"])
//!     .allow_credentials(true);
//! ```

// ─── CORS Config ───────────────────────────────────────────

/// CORS configuration
#[derive(Debug, Clone)]
pub struct CorsConfig {
    pub allowed_origins: Vec<String>,
    pub allowed_methods: Vec<String>,
    pub allowed_headers: Vec<String>,
    pub exposed_headers: Vec<String>,
    pub max_age: Option<u64>,
    pub allow_credentials: bool,
    pub allow_any_origin: bool,
}

impl CorsConfig {
    pub fn new() -> Self {
        Self {
            allowed_origins: Vec::new(),
            allowed_methods: vec!["GET".into(), "POST".into(), "PUT".into(), "DELETE".into(), "PATCH".into(), "OPTIONS".into()],
            allowed_headers: vec!["Content-Type".into(), "Authorization".into(), "Accept".into()],
            exposed_headers: Vec::new(),
            max_age: Some(86400),
            allow_credentials: false,
            allow_any_origin: false,
        }
    }

    /// Allow all origins (sets Access-Control-Allow-Origin: *)
    pub fn permissive() -> Self {
        Self {
            allow_any_origin: true,
            ..Self::new()
        }
    }

    /// Restrictive CORS (no origins allowed by default)
    pub fn restrictive() -> Self {
        Self {
            allowed_methods: vec!["GET".into()],
            allowed_headers: Vec::new(),
            max_age: None,
            ..Self::new()
        }
    }

    pub fn allow_origin(mut self, origin: &str) -> Self {
        self.allowed_origins.push(origin.to_string());
        self
    }

    pub fn allow_origins(mut self, origins: &[&str]) -> Self {
        for o in origins {
            self.allowed_origins.push(o.to_string());
        }
        self
    }

    pub fn allow_any(mut self) -> Self {
        self.allow_any_origin = true;
        self
    }

    pub fn allow_method(mut self, method: &str) -> Self {
        self.allowed_methods.push(method.to_uppercase());
        self
    }

    pub fn allow_methods(mut self, methods: &[&str]) -> Self {
        self.allowed_methods = methods.iter().map(|m| m.to_uppercase()).collect();
        self
    }

    pub fn allow_header(mut self, header: &str) -> Self {
        self.allowed_headers.push(header.to_string());
        self
    }

    pub fn allow_headers(mut self, headers: &[&str]) -> Self {
        self.allowed_headers = headers.iter().map(|h| h.to_string()).collect();
        self
    }

    pub fn expose_header(mut self, header: &str) -> Self {
        self.exposed_headers.push(header.to_string());
        self
    }

    pub fn max_age(mut self, secs: u64) -> Self {
        self.max_age = Some(secs);
        self
    }

    pub fn allow_credentials(mut self, allow: bool) -> Self {
        self.allow_credentials = allow;
        self
    }

    /// Check if a given origin is allowed
    pub fn is_origin_allowed(&self, origin: &str) -> bool {
        if self.allow_any_origin {
            return true;
        }
        self.allowed_origins.iter().any(|o| {
            o == origin || o == "*" || (o.starts_with("*.") && origin.ends_with(&o[1..]))
        })
    }

    /// Check if a given method is allowed
    pub fn is_method_allowed(&self, method: &str) -> bool {
        self.allowed_methods.iter().any(|m| m.eq_ignore_ascii_case(method))
    }

    /// Generate CORS response headers for a given request origin
    pub fn response_headers(&self, request_origin: Option<&str>) -> Vec<(String, String)> {
        let mut headers = Vec::new();

        // Access-Control-Allow-Origin
        if self.allow_any_origin && !self.allow_credentials {
            headers.push(("Access-Control-Allow-Origin".into(), "*".into()));
        } else if let Some(origin) = request_origin {
            if self.is_origin_allowed(origin) {
                headers.push(("Access-Control-Allow-Origin".into(), origin.to_string()));
                headers.push(("Vary".into(), "Origin".into()));
            }
        }

        // Access-Control-Allow-Credentials
        if self.allow_credentials {
            headers.push(("Access-Control-Allow-Credentials".into(), "true".into()));
        }

        // Access-Control-Expose-Headers
        if !self.exposed_headers.is_empty() {
            headers.push((
                "Access-Control-Expose-Headers".into(),
                self.exposed_headers.join(", "),
            ));
        }

        headers
    }

    /// Generate preflight (OPTIONS) response headers
    pub fn preflight_headers(&self, request_origin: Option<&str>) -> Vec<(String, String)> {
        let mut headers = self.response_headers(request_origin);

        // Access-Control-Allow-Methods
        if !self.allowed_methods.is_empty() {
            headers.push((
                "Access-Control-Allow-Methods".into(),
                self.allowed_methods.join(", "),
            ));
        }

        // Access-Control-Allow-Headers
        if !self.allowed_headers.is_empty() {
            headers.push((
                "Access-Control-Allow-Headers".into(),
                self.allowed_headers.join(", "),
            ));
        }

        // Access-Control-Max-Age
        if let Some(age) = self.max_age {
            headers.push(("Access-Control-Max-Age".into(), age.to_string()));
        }

        headers
    }

    /// Check if a request is a CORS preflight
    pub fn is_preflight(method: &str, origin: Option<&str>) -> bool {
        method.eq_ignore_ascii_case("OPTIONS") && origin.is_some()
    }
}

impl Default for CorsConfig {
    fn default() -> Self {
        Self::new()
    }
}

// ─── Per-Route CORS ────────────────────────────────────────

/// CORS configuration for specific routes
#[derive(Debug, Clone)]
pub struct RouteCors {
    pub path_pattern: String,
    pub config: CorsConfig,
}

impl RouteCors {
    pub fn new(path: &str, config: CorsConfig) -> Self {
        Self {
            path_pattern: path.to_string(),
            config,
        }
    }

    /// Check if this route CORS applies to the given path
    pub fn matches(&self, path: &str) -> bool {
        if self.path_pattern.ends_with("/*") {
            let prefix = &self.path_pattern[..self.path_pattern.len() - 2];
            path.starts_with(prefix)
        } else {
            path == self.path_pattern
        }
    }
}

// ─── Tests ──────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cors_default() {
        let cors = CorsConfig::new();
        assert!(!cors.allow_any_origin);
        assert!(cors.allowed_methods.contains(&"GET".to_string()));
        assert!(cors.allowed_methods.contains(&"POST".to_string()));
    }

    #[test]
    fn test_cors_permissive() {
        let cors = CorsConfig::permissive();
        assert!(cors.allow_any_origin);
        assert!(cors.is_origin_allowed("https://anything.com"));
    }

    #[test]
    fn test_cors_restrictive() {
        let cors = CorsConfig::restrictive();
        assert_eq!(cors.allowed_methods, vec!["GET"]);
        assert!(cors.allowed_headers.is_empty());
    }

    #[test]
    fn test_origin_check() {
        let cors = CorsConfig::new()
            .allow_origin("https://example.com")
            .allow_origin("https://app.example.com");
        assert!(cors.is_origin_allowed("https://example.com"));
        assert!(cors.is_origin_allowed("https://app.example.com"));
        assert!(!cors.is_origin_allowed("https://evil.com"));
    }

    #[test]
    fn test_wildcard_subdomain() {
        let cors = CorsConfig::new().allow_origin("*.example.com");
        assert!(cors.is_origin_allowed("https://app.example.com"));
        assert!(cors.is_origin_allowed("api.example.com"));
        assert!(!cors.is_origin_allowed("https://evil.com"));
    }

    #[test]
    fn test_method_check() {
        let cors = CorsConfig::new().allow_methods(&["GET", "POST"]);
        assert!(cors.is_method_allowed("GET"));
        assert!(cors.is_method_allowed("get"));
        assert!(cors.is_method_allowed("POST"));
        assert!(!cors.is_method_allowed("DELETE"));
    }

    #[test]
    fn test_response_headers_any_origin() {
        let cors = CorsConfig::permissive();
        let headers = cors.response_headers(Some("https://example.com"));
        assert!(headers.iter().any(|(k, v)| k == "Access-Control-Allow-Origin" && v == "*"));
    }

    #[test]
    fn test_response_headers_specific_origin() {
        let cors = CorsConfig::new().allow_origin("https://example.com");
        let headers = cors.response_headers(Some("https://example.com"));
        assert!(headers.iter().any(|(k, v)| k == "Access-Control-Allow-Origin" && v == "https://example.com"));
        assert!(headers.iter().any(|(k, _)| k == "Vary"));
    }

    #[test]
    fn test_response_headers_disallowed_origin() {
        let cors = CorsConfig::new().allow_origin("https://example.com");
        let headers = cors.response_headers(Some("https://evil.com"));
        assert!(!headers.iter().any(|(k, _)| k == "Access-Control-Allow-Origin"));
    }

    #[test]
    fn test_credentials() {
        let cors = CorsConfig::new()
            .allow_origin("https://example.com")
            .allow_credentials(true);
        let headers = cors.response_headers(Some("https://example.com"));
        assert!(headers.iter().any(|(k, v)| k == "Access-Control-Allow-Credentials" && v == "true"));
        // When credentials are true, should not use *
        assert!(!headers.iter().any(|(k, v)| k == "Access-Control-Allow-Origin" && v == "*"));
    }

    #[test]
    fn test_preflight_headers() {
        let cors = CorsConfig::new()
            .allow_origin("https://example.com")
            .allow_methods(&["GET", "POST"])
            .allow_headers(&["Content-Type", "X-Custom"])
            .max_age(3600);
        let headers = cors.preflight_headers(Some("https://example.com"));
        assert!(headers.iter().any(|(k, v)| k == "Access-Control-Allow-Methods" && v.contains("GET")));
        assert!(headers.iter().any(|(k, v)| k == "Access-Control-Allow-Headers" && v.contains("X-Custom")));
        assert!(headers.iter().any(|(k, v)| k == "Access-Control-Max-Age" && v == "3600"));
    }

    #[test]
    fn test_exposed_headers() {
        let cors = CorsConfig::new()
            .allow_any()
            .expose_header("X-Request-Id")
            .expose_header("X-Total-Count");
        let headers = cors.response_headers(None);
        assert!(headers.iter().any(|(k, v)| k == "Access-Control-Expose-Headers" && v.contains("X-Request-Id")));
    }

    #[test]
    fn test_is_preflight() {
        assert!(CorsConfig::is_preflight("OPTIONS", Some("https://example.com")));
        assert!(!CorsConfig::is_preflight("GET", Some("https://example.com")));
        assert!(!CorsConfig::is_preflight("OPTIONS", None));
    }

    #[test]
    fn test_route_cors() {
        let rc = RouteCors::new("/api/*", CorsConfig::permissive());
        assert!(rc.matches("/api/users"));
        assert!(rc.matches("/api/posts/123"));
        assert!(!rc.matches("/web/page"));
    }

    #[test]
    fn test_route_cors_exact() {
        let rc = RouteCors::new("/api/health", CorsConfig::new());
        assert!(rc.matches("/api/health"));
        assert!(!rc.matches("/api/health/deep"));
    }
}
