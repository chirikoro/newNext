//! Per-Route Error and Loading Boundaries for Hayabusa.
//!
//! Provides Next.js App Router-like error.tsx and loading.tsx functionality:
//! - Error boundaries catch rendering errors and show fallback UI
//! - Loading boundaries show skeleton/placeholder during async renders
//! - Boundaries can be nested (closest boundary wins)
//!
//! ## Usage
//! ```ignore
//! let boundaries = ErrorBoundaryConfig::new()
//!     .error_boundary("/", |err| html! {
//!         <div class="error">
//!             <h1>"Something went wrong"</h1>
//!             <p>{err.message()}</p>
//!         </div>
//!     })
//!     .loading_boundary("/dashboard", || html! {
//!         <div class="skeleton">
//!             <div class="skeleton-header" />
//!             <div class="skeleton-content" />
//!         </div>
//!     });
//! ```

use std::sync::Arc;

/// Error information passed to error boundary handlers
#[derive(Debug, Clone)]
pub struct BoundaryError {
    /// HTTP status code
    pub status: u16,
    /// Error message
    pub message: String,
    /// The route path that errored
    pub path: String,
    /// Optional error detail/stack
    pub detail: Option<String>,
}

impl BoundaryError {
    pub fn not_found(path: impl Into<String>) -> Self {
        Self {
            status: 404,
            message: "Page not found".to_string(),
            path: path.into(),
            detail: None,
        }
    }

    pub fn internal(path: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            status: 500,
            message: message.into(),
            path: path.into(),
            detail: None,
        }
    }

    pub fn with_detail(mut self, detail: impl Into<String>) -> Self {
        self.detail = Some(detail.into());
        self
    }

    pub fn message(&self) -> &str {
        &self.message
    }
}

/// Error boundary handler: takes an error, returns HTML
pub type ErrorHandler = Arc<dyn Fn(&BoundaryError) -> String + Send + Sync>;

/// Loading boundary handler: returns placeholder/skeleton HTML
pub type LoadingHandler = Arc<dyn Fn() -> String + Send + Sync>;

/// Configuration for error and loading boundaries per route segment
pub struct ErrorBoundaryConfig {
    /// Error handlers keyed by path prefix
    error_handlers: Vec<(String, ErrorHandler)>,
    /// Loading handlers keyed by path prefix
    loading_handlers: Vec<(String, LoadingHandler)>,
    /// Global error handler (fallback)
    global_error: Option<ErrorHandler>,
    /// Global loading handler (fallback)
    global_loading: Option<LoadingHandler>,
}

impl ErrorBoundaryConfig {
    pub fn new() -> Self {
        Self {
            error_handlers: Vec::new(),
            loading_handlers: Vec::new(),
            global_error: Some(Arc::new(default_error_handler)),
            global_loading: Some(Arc::new(default_loading_handler)),
        }
    }

    /// Register an error boundary for a path prefix
    pub fn error_boundary(
        mut self,
        path: impl Into<String>,
        handler: impl Fn(&BoundaryError) -> String + Send + Sync + 'static,
    ) -> Self {
        self.error_handlers
            .push((path.into(), Arc::new(handler)));
        self
    }

    /// Register a loading boundary for a path prefix
    pub fn loading_boundary(
        mut self,
        path: impl Into<String>,
        handler: impl Fn() -> String + Send + Sync + 'static,
    ) -> Self {
        self.loading_handlers
            .push((path.into(), Arc::new(handler)));
        self
    }

    /// Set the global error handler (used when no segment-specific handler matches)
    pub fn global_error(
        mut self,
        handler: impl Fn(&BoundaryError) -> String + Send + Sync + 'static,
    ) -> Self {
        self.global_error = Some(Arc::new(handler));
        self
    }

    /// Set the global loading handler
    pub fn global_loading(
        mut self,
        handler: impl Fn() -> String + Send + Sync + 'static,
    ) -> Self {
        self.global_loading = Some(Arc::new(handler));
        self
    }

    /// Find the closest error handler for a given path.
    /// Matches the most specific (longest) path prefix first.
    pub fn get_error_handler(&self, path: &str) -> Option<&ErrorHandler> {
        // Find the most specific matching handler
        let mut best_match: Option<(&str, &ErrorHandler)> = None;

        for (prefix, handler) in &self.error_handlers {
            if path.starts_with(prefix.as_str()) || prefix == "/" {
                match best_match {
                    None => best_match = Some((prefix, handler)),
                    Some((current_prefix, _)) if prefix.len() > current_prefix.len() => {
                        best_match = Some((prefix, handler));
                    }
                    _ => {}
                }
            }
        }

        best_match
            .map(|(_, h)| h)
            .or(self.global_error.as_ref())
    }

    /// Find the closest loading handler for a given path
    pub fn get_loading_handler(&self, path: &str) -> Option<&LoadingHandler> {
        let mut best_match: Option<(&str, &LoadingHandler)> = None;

        for (prefix, handler) in &self.loading_handlers {
            if path.starts_with(prefix.as_str()) || prefix == "/" {
                match best_match {
                    None => best_match = Some((prefix, handler)),
                    Some((current_prefix, _)) if prefix.len() > current_prefix.len() => {
                        best_match = Some((prefix, handler));
                    }
                    _ => {}
                }
            }
        }

        best_match
            .map(|(_, h)| h)
            .or(self.global_loading.as_ref())
    }

    /// Render an error page using the appropriate boundary
    pub fn render_error(&self, error: &BoundaryError) -> String {
        if let Some(handler) = self.get_error_handler(&error.path) {
            handler(error)
        } else {
            default_error_handler(error)
        }
    }

    /// Render a loading state using the appropriate boundary
    pub fn render_loading(&self, path: &str) -> String {
        if let Some(handler) = self.get_loading_handler(path) {
            handler()
        } else {
            default_loading_handler()
        }
    }
}

impl Default for ErrorBoundaryConfig {
    fn default() -> Self {
        Self::new()
    }
}

/// Default error handler with styled HTML
fn default_error_handler(error: &BoundaryError) -> String {
    let title = match error.status {
        404 => "Page Not Found",
        403 => "Forbidden",
        500 => "Internal Server Error",
        503 => "Service Unavailable",
        _ => "Error",
    };

    format!(
        r#"<div style="display:flex;flex-direction:column;align-items:center;justify-content:center;min-height:50vh;font-family:system-ui,sans-serif;text-align:center;padding:2rem">
<h1 style="font-size:4rem;margin:0;opacity:0.3">{status}</h1>
<h2 style="margin:0.5rem 0">{title}</h2>
<p style="opacity:0.7;max-width:400px">{message}</p>
<a href="/" style="margin-top:1rem;color:inherit">← Back to Home</a>
</div>"#,
        status = error.status,
        title = title,
        message = error.message,
    )
}

/// Default loading handler with CSS-only skeleton animation
fn default_loading_handler() -> String {
    r#"<div style="padding:2rem;max-width:800px;margin:0 auto">
<style>
@keyframes shimmer{0%{background-position:-200% 0}100%{background-position:200% 0}}
.hayabusa-skeleton{background:linear-gradient(90deg,#f0f0f0 25%,#e0e0e0 50%,#f0f0f0 75%);background-size:200% 100%;animation:shimmer 1.5s infinite;border-radius:4px}
</style>
<div class="hayabusa-skeleton" style="height:2rem;width:60%;margin-bottom:1rem"></div>
<div class="hayabusa-skeleton" style="height:1rem;width:100%;margin-bottom:0.5rem"></div>
<div class="hayabusa-skeleton" style="height:1rem;width:90%;margin-bottom:0.5rem"></div>
<div class="hayabusa-skeleton" style="height:1rem;width:95%;margin-bottom:1.5rem"></div>
<div class="hayabusa-skeleton" style="height:12rem;width:100%;margin-bottom:1rem"></div>
<div class="hayabusa-skeleton" style="height:1rem;width:80%;margin-bottom:0.5rem"></div>
<div class="hayabusa-skeleton" style="height:1rem;width:85%"></div>
</div>"#
        .to_string()
}

/// Generate a skeleton element with customizable dimensions
pub fn skeleton(width: &str, height: &str) -> String {
    format!(
        "<div class=\"hayabusa-skeleton\" style=\"width:{};height:{}\"></div>",
        width, height
    )
}

/// Generate the CSS for skeleton animations (include once per page)
pub fn skeleton_css() -> &'static str {
    "@keyframes shimmer{0%{background-position:-200% 0}100%{background-position:200% 0}}.hayabusa-skeleton{background:linear-gradient(90deg,#f0f0f0 25%,#e0e0e0 50%,#f0f0f0 75%);background-size:200% 100%;animation:shimmer 1.5s infinite;border-radius:4px}"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_error_handler() {
        let error = BoundaryError::not_found("/nonexistent");
        let html = default_error_handler(&error);
        assert!(html.contains("404"));
        assert!(html.contains("Page Not Found"));
    }

    #[test]
    fn test_default_loading_handler() {
        let html = default_loading_handler();
        assert!(html.contains("shimmer"));
        assert!(html.contains("hayabusa-skeleton"));
    }

    #[test]
    fn test_error_boundary_specificity() {
        let config = ErrorBoundaryConfig::new()
            .error_boundary("/", |_| "root error".to_string())
            .error_boundary("/dashboard", |_| "dashboard error".to_string())
            .error_boundary("/dashboard/settings", |_| "settings error".to_string());

        let err = BoundaryError::internal("/dashboard/settings/profile", "fail");

        // Should match /dashboard/settings (most specific)
        let html = config.render_error(&err);
        assert_eq!(html, "settings error");

        // /dashboard/analytics should match /dashboard
        let err2 = BoundaryError::internal("/dashboard/analytics", "fail");
        let html2 = config.render_error(&err2);
        assert_eq!(html2, "dashboard error");

        // /about should match /
        let err3 = BoundaryError::internal("/about", "fail");
        let html3 = config.render_error(&err3);
        assert_eq!(html3, "root error");
    }

    #[test]
    fn test_loading_boundary() {
        let config = ErrorBoundaryConfig::new()
            .loading_boundary("/", || "loading root...".to_string())
            .loading_boundary("/dashboard", || "loading dashboard...".to_string());

        assert_eq!(config.render_loading("/dashboard/page"), "loading dashboard...");
        assert_eq!(config.render_loading("/about"), "loading root...");
    }

    #[test]
    fn test_boundary_error_constructors() {
        let err = BoundaryError::not_found("/test");
        assert_eq!(err.status, 404);
        assert_eq!(err.path, "/test");

        let err = BoundaryError::internal("/api", "db connection failed")
            .with_detail("connection timeout after 30s");
        assert_eq!(err.status, 500);
        assert!(err.detail.is_some());
    }

    #[test]
    fn test_skeleton_helper() {
        let s = skeleton("100%", "2rem");
        assert!(s.contains("width:100%"));
        assert!(s.contains("height:2rem"));
        assert!(s.contains("hayabusa-skeleton"));
    }

    #[test]
    fn test_skeleton_css() {
        let css = skeleton_css();
        assert!(css.contains("@keyframes shimmer"));
        assert!(css.contains("hayabusa-skeleton"));
    }
}
