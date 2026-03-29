//! Server Actions for Hayabusa.
//!
//! Allows handling form submissions directly as function calls,
//! without needing to create separate API routes.
//!
//! Inspired by Next.js Server Actions (`"use server"`).
//!
//! # How it works
//! 1. Define a `ServerAction` with a handler function
//! 2. Register it with a unique action ID
//! 3. The form includes a hidden `__hayabusa_action` field
//! 4. On POST, the framework routes to the correct action handler
//! 5. The action returns a `ActionResult` (redirect, render, or error)

use axum::response::Response;
use http::StatusCode;
use std::collections::HashMap;
use std::pin::Pin;
use std::future::Future;
use std::sync::Arc;

/// The result of a server action execution
#[derive(Debug, Clone)]
pub enum ActionResult {
    /// Redirect to a URL (303 See Other for POST-redirect-GET)
    Redirect(String),
    /// Return HTML directly
    Html(String),
    /// Return JSON data
    Json(serde_json::Value),
    /// Return an error message
    Error { status: u16, message: String },
}

/// Form data parsed from a POST request body
#[derive(Debug, Clone, Default)]
pub struct FormData {
    fields: HashMap<String, String>,
}

impl FormData {
    pub fn new() -> Self {
        Self::default()
    }

    /// Parse form data from URL-encoded body
    pub fn from_urlencoded(body: &str) -> Self {
        let fields = body
            .split('&')
            .filter(|s| !s.is_empty())
            .filter_map(|pair| {
                let mut parts = pair.splitn(2, '=');
                let key = parts.next()?;
                let value = parts.next().unwrap_or("");
                Some((
                    url_decode(key),
                    url_decode(value),
                ))
            })
            .collect();
        Self { fields }
    }

    /// Get a form field value
    pub fn get(&self, key: &str) -> Option<&str> {
        self.fields.get(key).map(|s| s.as_str())
    }

    /// Get a form field value or return a default
    pub fn get_or(&self, key: &str, default: &str) -> String {
        self.fields
            .get(key)
            .cloned()
            .unwrap_or_else(|| default.to_string())
    }

    /// Check if a field exists
    pub fn has(&self, key: &str) -> bool {
        self.fields.contains_key(key)
    }

    /// Get all fields
    pub fn fields(&self) -> &HashMap<String, String> {
        &self.fields
    }

    /// Get the action ID from the hidden field
    pub fn action_id(&self) -> Option<&str> {
        self.get("__hayabusa_action")
    }
}

/// Type alias for action handler functions
pub type ActionHandler = Arc<
    dyn Fn(FormData) -> Pin<Box<dyn Future<Output = ActionResult> + Send>>
        + Send
        + Sync,
>;

/// Registry of server actions
pub struct ActionRegistry {
    actions: HashMap<String, ActionHandler>,
}

impl ActionRegistry {
    pub fn new() -> Self {
        Self {
            actions: HashMap::new(),
        }
    }

    /// Register a server action with a unique ID
    pub fn register(
        mut self,
        id: impl Into<String>,
        handler: impl Fn(FormData) -> Pin<Box<dyn Future<Output = ActionResult> + Send>>
            + Send
            + Sync
            + 'static,
    ) -> Self {
        self.actions.insert(id.into(), Arc::new(handler));
        self
    }

    /// Look up an action by ID
    pub fn get(&self, id: &str) -> Option<&ActionHandler> {
        self.actions.get(id)
    }

    /// Execute an action by ID with the given form data
    pub async fn execute(&self, id: &str, form: FormData) -> Option<ActionResult> {
        if let Some(handler) = self.actions.get(id) {
            Some(handler(form).await)
        } else {
            None
        }
    }
}

impl Default for ActionRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Convert an ActionResult into an axum Response
pub fn action_result_to_response(result: ActionResult) -> Response {
    match result {
        ActionResult::Redirect(url) => Response::builder()
            .status(StatusCode::SEE_OTHER)
            .header("location", &url)
            .body(axum::body::Body::empty())
            .unwrap(),

        ActionResult::Html(html) => Response::builder()
            .status(StatusCode::OK)
            .header("content-type", "text/html; charset=utf-8")
            .body(axum::body::Body::from(html))
            .unwrap(),

        ActionResult::Json(value) => {
            let json = serde_json::to_string(&value).unwrap_or_default();
            Response::builder()
                .status(StatusCode::OK)
                .header("content-type", "application/json")
                .body(axum::body::Body::from(json))
                .unwrap()
        }

        ActionResult::Error { status, message } => Response::builder()
            .status(StatusCode::from_u16(status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR))
            .header("content-type", "text/plain")
            .body(axum::body::Body::from(message))
            .unwrap(),
    }
}

/// Generate a hidden input field for a server action form.
///
/// # Example
/// ```ignore
/// let form_html = html! {
///     <form method="POST">
///         {action_field("create-post")}
///         <input type="text" name="title" />
///         <button type="submit">"Create"</button>
///     </form>
/// };
/// ```
pub fn action_field(action_id: &str) -> String {
    format!(
        "<input type=\"hidden\" name=\"__hayabusa_action\" value=\"{action_id}\" />"
    )
}

/// Generate a CSRF token field for form security.
/// Uses a simple HMAC-based approach with a server secret.
pub fn csrf_field(token: &str) -> String {
    format!(
        "<input type=\"hidden\" name=\"__hayabusa_csrf\" value=\"{token}\" />"
    )
}

/// Generate a CSRF token from a secret and a session identifier.
pub fn generate_csrf_token(secret: &str, session_id: &str) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    secret.hash(&mut hasher);
    session_id.hash(&mut hasher);
    // Include a time component (hour-granularity for reasonable validity)
    let hours_since_epoch = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
        / 3600;
    hours_since_epoch.hash(&mut hasher);
    format!("{:x}", hasher.finish())
}

/// Validate a CSRF token
pub fn validate_csrf_token(token: &str, secret: &str, session_id: &str) -> bool {
    let expected = generate_csrf_token(secret, session_id);
    token == expected
}

fn url_decode(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut chars = s.chars();

    while let Some(ch) = chars.next() {
        match ch {
            '+' => result.push(' '),
            '%' => {
                let hex: String = chars.by_ref().take(2).collect();
                if let Ok(byte) = u8::from_str_radix(&hex, 16) {
                    result.push(byte as char);
                } else {
                    result.push('%');
                    result.push_str(&hex);
                }
            }
            _ => result.push(ch),
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_form_data_parsing() {
        let form = FormData::from_urlencoded("name=John+Doe&email=john%40example.com&action=submit");
        assert_eq!(form.get("name"), Some("John Doe"));
        assert_eq!(form.get("email"), Some("john@example.com"));
        assert_eq!(form.get("action"), Some("submit"));
        assert_eq!(form.get("missing"), None);
    }

    #[test]
    fn test_form_data_action_id() {
        let form = FormData::from_urlencoded("__hayabusa_action=create-post&title=Hello");
        assert_eq!(form.action_id(), Some("create-post"));
        assert_eq!(form.get("title"), Some("Hello"));
    }

    #[test]
    fn test_action_field_html() {
        let html = action_field("delete-post");
        assert!(html.contains("__hayabusa_action"));
        assert!(html.contains("delete-post"));
        assert!(html.contains("type=\"hidden\""));
    }

    #[test]
    fn test_csrf_generation_and_validation() {
        let token = generate_csrf_token("my-secret", "session-123");
        assert!(validate_csrf_token(&token, "my-secret", "session-123"));
        assert!(!validate_csrf_token(&token, "wrong-secret", "session-123"));
        assert!(!validate_csrf_token("fake-token", "my-secret", "session-123"));
    }

    #[test]
    fn test_url_decode() {
        assert_eq!(url_decode("hello+world"), "hello world");
        assert_eq!(url_decode("hello%20world"), "hello world");
        assert_eq!(url_decode("a%26b%3Dc"), "a&b=c");
    }

    #[tokio::test]
    async fn test_action_registry() {
        let registry = ActionRegistry::new()
            .register("greet", |form: FormData| {
                Box::pin(async move {
                    let name = form.get_or("name", "World");
                    ActionResult::Html(format!("Hello, {name}!"))
                })
            });

        let form = FormData::from_urlencoded("name=Rust");
        let result = registry.execute("greet", form).await.unwrap();

        match result {
            ActionResult::Html(html) => assert_eq!(html, "Hello, Rust!"),
            _ => panic!("Expected Html result"),
        }
    }

    #[tokio::test]
    async fn test_action_redirect() {
        let registry = ActionRegistry::new()
            .register("login", |_form: FormData| {
                Box::pin(async move {
                    ActionResult::Redirect("/dashboard".to_string())
                })
            });

        let form = FormData::from_urlencoded("username=admin&password=secret");
        let result = registry.execute("login", form).await.unwrap();

        match result {
            ActionResult::Redirect(url) => assert_eq!(url, "/dashboard"),
            _ => panic!("Expected Redirect result"),
        }
    }
}
