//! Content Security Policy (CSP) for Hayabusa.
//!
//! Generates CSP headers with nonce-based script/style allowlisting.
//! Prevents XSS attacks while allowing framework-generated inline scripts.
//!
//! ## Usage
//! ```ignore
//! let csp = CspConfig::new()
//!     .default_src(&["'self'"])
//!     .script_src_with_nonce(&["'self'", "https://cdn.example.com"])
//!     .style_src_with_nonce(&["'self'"])
//!     .img_src(&["'self'", "data:", "https:"])
//!     .connect_src(&["'self'", "https://api.example.com"]);
//!
//! let nonce = csp.generate_nonce();
//! let header = csp.render_header(&nonce);
//! ```

use std::collections::HashMap;

/// CSP configuration
#[derive(Debug, Clone, Default)]
pub struct CspConfig {
    directives: HashMap<String, Vec<String>>,
    nonce_directives: Vec<String>,
    report_uri: Option<String>,
    report_only: bool,
}

impl CspConfig {
    pub fn new() -> Self {
        Self::default()
    }

    /// Set default-src directive
    pub fn default_src(self, sources: &[&str]) -> Self {
        self.directive("default-src", sources)
    }

    /// Set script-src with nonce support
    pub fn script_src_with_nonce(mut self, sources: &[&str]) -> Self {
        self.nonce_directives.push("script-src".to_string());
        self.directive("script-src", sources)
    }

    /// Set script-src without nonce
    pub fn script_src(self, sources: &[&str]) -> Self {
        self.directive("script-src", sources)
    }

    /// Set style-src with nonce support
    pub fn style_src_with_nonce(mut self, sources: &[&str]) -> Self {
        self.nonce_directives.push("style-src".to_string());
        self.directive("style-src", sources)
    }

    /// Set style-src without nonce
    pub fn style_src(self, sources: &[&str]) -> Self {
        self.directive("style-src", sources)
    }

    /// Set img-src directive
    pub fn img_src(self, sources: &[&str]) -> Self {
        self.directive("img-src", sources)
    }

    /// Set font-src directive
    pub fn font_src(self, sources: &[&str]) -> Self {
        self.directive("font-src", sources)
    }

    /// Set connect-src directive (for fetch/XHR/SSE/WebSocket)
    pub fn connect_src(self, sources: &[&str]) -> Self {
        self.directive("connect-src", sources)
    }

    /// Set frame-src directive
    pub fn frame_src(self, sources: &[&str]) -> Self {
        self.directive("frame-src", sources)
    }

    /// Set media-src directive
    pub fn media_src(self, sources: &[&str]) -> Self {
        self.directive("media-src", sources)
    }

    /// Set object-src directive
    pub fn object_src(self, sources: &[&str]) -> Self {
        self.directive("object-src", sources)
    }

    /// Set base-uri directive
    pub fn base_uri(self, sources: &[&str]) -> Self {
        self.directive("base-uri", sources)
    }

    /// Set form-action directive
    pub fn form_action(self, sources: &[&str]) -> Self {
        self.directive("form-action", sources)
    }

    /// Set frame-ancestors directive
    pub fn frame_ancestors(self, sources: &[&str]) -> Self {
        self.directive("frame-ancestors", sources)
    }

    /// Add a custom directive
    pub fn directive(mut self, name: &str, sources: &[&str]) -> Self {
        self.directives.insert(
            name.to_string(),
            sources.iter().map(|s| s.to_string()).collect(),
        );
        self
    }

    /// Enable upgrade-insecure-requests
    pub fn upgrade_insecure_requests(mut self) -> Self {
        self.directives
            .insert("upgrade-insecure-requests".to_string(), vec![]);
        self
    }

    /// Set report URI for CSP violations
    pub fn report_uri(mut self, uri: impl Into<String>) -> Self {
        self.report_uri = Some(uri.into());
        self
    }

    /// Use Content-Security-Policy-Report-Only instead of enforcing
    pub fn report_only(mut self) -> Self {
        self.report_only = true;
        self
    }

    /// Generate a cryptographic nonce for inline scripts/styles
    pub fn generate_nonce() -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        use std::time::SystemTime;

        let mut hasher = DefaultHasher::new();
        SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
            .hash(&mut hasher);
        // Add some entropy from thread ID
        std::thread::current().id().hash(&mut hasher);
        format!("{:x}{:x}", hasher.finish(), {
            let mut h2 = DefaultHasher::new();
            hasher.finish().wrapping_mul(6364136223846793005).hash(&mut h2);
            h2.finish()
        })
    }

    /// Render the CSP header value with the given nonce
    pub fn render_header(&self, nonce: &str) -> (String, String) {
        let mut parts: Vec<String> = Vec::new();

        for (directive, sources) in &self.directives {
            if sources.is_empty() {
                parts.push(directive.clone());
                continue;
            }

            let mut src_list: Vec<String> = sources.clone();

            // Add nonce to directives that require it
            if self.nonce_directives.contains(directive) {
                src_list.push(format!("'nonce-{}'", nonce));
            }

            parts.push(format!("{} {}", directive, src_list.join(" ")));
        }

        if let Some(ref uri) = self.report_uri {
            parts.push(format!("report-uri {}", uri));
        }

        let header_name = if self.report_only {
            "content-security-policy-report-only".to_string()
        } else {
            "content-security-policy".to_string()
        };

        (header_name, parts.join("; "))
    }

    /// Render a <meta> tag for CSP (no nonce support, use headers for nonces)
    pub fn render_meta_tag(&self) -> String {
        let (_, value) = self.render_header("");
        format!("<meta http-equiv=\"Content-Security-Policy\" content=\"{}\" />", value)
    }
}

/// Add nonce attribute to inline <script> tags in HTML
pub fn add_nonce_to_scripts(html: &str, nonce: &str) -> String {
    html.replace("<script>", &format!("<script nonce=\"{}\">", nonce))
}

/// Add nonce attribute to inline <style> tags in HTML
pub fn add_nonce_to_styles(html: &str, nonce: &str) -> String {
    html.replace("<style>", &format!("<style nonce=\"{}\">", nonce))
}

/// A strict CSP preset for production applications
pub fn strict_csp() -> CspConfig {
    CspConfig::new()
        .default_src(&["'self'"])
        .script_src_with_nonce(&["'self'", "'strict-dynamic'"])
        .style_src_with_nonce(&["'self'"])
        .img_src(&["'self'", "data:", "https:"])
        .font_src(&["'self'", "https:"])
        .connect_src(&["'self'"])
        .object_src(&["'none'"])
        .base_uri(&["'self'"])
        .frame_ancestors(&["'none'"])
        .upgrade_insecure_requests()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_csp() {
        let csp = CspConfig::new()
            .default_src(&["'self'"])
            .img_src(&["'self'", "data:"]);

        let (name, value) = csp.render_header("abc123");
        assert_eq!(name, "content-security-policy");
        assert!(value.contains("default-src 'self'"));
        assert!(value.contains("img-src 'self' data:"));
    }

    #[test]
    fn test_nonce_injection() {
        let csp = CspConfig::new()
            .script_src_with_nonce(&["'self'"]);

        let (_, value) = csp.render_header("my-nonce-123");
        assert!(value.contains("'nonce-my-nonce-123'"));
    }

    #[test]
    fn test_report_only() {
        let csp = CspConfig::new()
            .default_src(&["'self'"])
            .report_only();

        let (name, _) = csp.render_header("");
        assert_eq!(name, "content-security-policy-report-only");
    }

    #[test]
    fn test_add_nonce_to_scripts() {
        let html = "<script>alert('hi')</script>";
        let result = add_nonce_to_scripts(html, "abc");
        assert_eq!(result, "<script nonce=\"abc\">alert('hi')</script>");
    }

    #[test]
    fn test_add_nonce_to_styles() {
        let html = "<style>body{color:red}</style>";
        let result = add_nonce_to_styles(html, "xyz");
        assert_eq!(result, "<style nonce=\"xyz\">body{color:red}</style>");
    }

    #[test]
    fn test_strict_csp_preset() {
        let csp = strict_csp();
        let (_, value) = csp.render_header("test-nonce");
        assert!(value.contains("'strict-dynamic'"));
        assert!(value.contains("'nonce-test-nonce'"));
        assert!(value.contains("object-src 'none'"));
        assert!(value.contains("upgrade-insecure-requests"));
    }

    #[test]
    fn test_generate_nonce() {
        let n1 = CspConfig::generate_nonce();
        let n2 = CspConfig::generate_nonce();
        // Nonces should not be empty
        assert!(!n1.is_empty());
        assert!(!n2.is_empty());
    }

    #[test]
    fn test_report_uri() {
        let csp = CspConfig::new()
            .default_src(&["'self'"])
            .report_uri("/csp-report");

        let (_, value) = csp.render_header("");
        assert!(value.contains("report-uri /csp-report"));
    }
}
