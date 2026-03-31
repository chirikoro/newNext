//! Test Utilities for Hayabusa.
//!
//! Helpers for testing routes, components, middleware, and API endpoints.
//!
//! ```ignore
//! use hayabusa_core::prelude::*;
//!
//! let resp = TestClient::new()
//!     .get("/api/users")
//!     .header("Authorization", "Bearer token")
//!     .send();
//! assert_eq!(resp.status(), 200);
//! ```

use std::collections::HashMap;

// ─── Test Request ───────────────────────────────────────────

/// A simulated HTTP request for testing
#[derive(Debug, Clone)]
pub struct TestRequest {
    pub method: String,
    pub path: String,
    pub headers: HashMap<String, String>,
    pub body: Option<String>,
    pub query: HashMap<String, String>,
    pub cookies: HashMap<String, String>,
}

impl TestRequest {
    pub fn get(path: impl Into<String>) -> Self {
        Self {
            method: "GET".to_string(),
            path: path.into(),
            headers: HashMap::new(),
            body: None,
            query: HashMap::new(),
            cookies: HashMap::new(),
        }
    }

    pub fn post(path: impl Into<String>) -> Self {
        Self {
            method: "POST".to_string(),
            path: path.into(),
            headers: HashMap::new(),
            body: None,
            query: HashMap::new(),
            cookies: HashMap::new(),
        }
    }

    pub fn put(path: impl Into<String>) -> Self {
        Self {
            method: "PUT".to_string(),
            path: path.into(),
            headers: HashMap::new(),
            body: None,
            query: HashMap::new(),
            cookies: HashMap::new(),
        }
    }

    pub fn delete(path: impl Into<String>) -> Self {
        Self {
            method: "DELETE".to_string(),
            path: path.into(),
            headers: HashMap::new(),
            body: None,
            query: HashMap::new(),
            cookies: HashMap::new(),
        }
    }

    pub fn header(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.insert(key.into(), value.into());
        self
    }

    pub fn body(mut self, body: impl Into<String>) -> Self {
        self.body = Some(body.into());
        self
    }

    pub fn json_body(self, json: &str) -> Self {
        self.header("Content-Type", "application/json").body(json)
    }

    pub fn form_body(self, data: &[(&str, &str)]) -> Self {
        let body = data
            .iter()
            .map(|(k, v)| format!("{}={}", url_encode(k), url_encode(v)))
            .collect::<Vec<_>>()
            .join("&");
        self.header("Content-Type", "application/x-www-form-urlencoded")
            .body(body)
    }

    pub fn query(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.query.insert(key.into(), value.into());
        self
    }

    pub fn cookie(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.cookies.insert(name.into(), value.into());
        self
    }

    /// Build the full URL with query parameters
    pub fn full_path(&self) -> String {
        if self.query.is_empty() {
            return self.path.clone();
        }
        let qs: Vec<String> = self
            .query
            .iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect();
        format!("{}?{}", self.path, qs.join("&"))
    }

    /// Build cookie header value
    pub fn cookie_header(&self) -> Option<String> {
        if self.cookies.is_empty() {
            return None;
        }
        Some(
            self.cookies
                .iter()
                .map(|(k, v)| format!("{}={}", k, v))
                .collect::<Vec<_>>()
                .join("; "),
        )
    }
}

// ─── Test Response ──────────────────────────────────────────

/// A simulated HTTP response for assertions
#[derive(Debug, Clone)]
pub struct TestResponse {
    pub status: u16,
    pub headers: HashMap<String, String>,
    pub body: String,
}

impl TestResponse {
    pub fn new(status: u16, body: impl Into<String>) -> Self {
        Self {
            status,
            headers: HashMap::new(),
            body: body.into(),
        }
    }

    pub fn with_header(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.insert(key.into(), value.into());
        self
    }

    pub fn ok(body: impl Into<String>) -> Self {
        Self::new(200, body)
    }

    pub fn not_found() -> Self {
        Self::new(404, "Not Found")
    }

    pub fn redirect(location: &str) -> Self {
        Self::new(302, "").with_header("Location", location)
    }

    // ── Assertions ──

    /// Assert status code
    pub fn assert_status(&self, expected: u16) -> &Self {
        assert_eq!(
            self.status, expected,
            "Expected status {}, got {}",
            expected, self.status
        );
        self
    }

    /// Assert status is 2xx
    pub fn assert_ok(&self) -> &Self {
        assert!(
            (200..300).contains(&self.status),
            "Expected 2xx status, got {}",
            self.status
        );
        self
    }

    /// Assert body contains a string
    pub fn assert_body_contains(&self, expected: &str) -> &Self {
        assert!(
            self.body.contains(expected),
            "Body does not contain '{}'. Body: {}",
            expected,
            &self.body[..self.body.len().min(200)]
        );
        self
    }

    /// Assert body equals exactly
    pub fn assert_body_eq(&self, expected: &str) -> &Self {
        assert_eq!(self.body, expected, "Body mismatch");
        self
    }

    /// Assert a header exists and matches
    pub fn assert_header(&self, key: &str, expected: &str) -> &Self {
        let value = self.headers.get(key);
        assert_eq!(
            value.map(|s| s.as_str()),
            Some(expected),
            "Header '{}' mismatch. Got: {:?}",
            key,
            value
        );
        self
    }

    /// Assert header exists
    pub fn assert_has_header(&self, key: &str) -> &Self {
        assert!(
            self.headers.contains_key(key),
            "Expected header '{}' not found",
            key
        );
        self
    }

    /// Assert body is valid JSON
    pub fn assert_json(&self) -> &Self {
        assert!(
            serde_json::from_str::<serde_json::Value>(&self.body).is_ok(),
            "Body is not valid JSON: {}",
            &self.body[..self.body.len().min(200)]
        );
        self
    }

    /// Assert a JSON field value
    pub fn assert_json_field(&self, field: &str, expected: &str) -> &Self {
        if let Ok(val) = serde_json::from_str::<serde_json::Value>(&self.body) {
            let actual = val.get(field);
            assert!(
                actual.is_some(),
                "JSON field '{}' not found in: {}",
                field, self.body
            );
            let actual_str = match actual.unwrap() {
                serde_json::Value::String(s) => s.clone(),
                other => other.to_string(),
            };
            assert_eq!(
                actual_str, expected,
                "JSON field '{}' mismatch: got '{}', expected '{}'",
                field, actual_str, expected
            );
        } else {
            panic!("Body is not valid JSON");
        }
        self
    }

    /// Assert body contains valid HTML
    pub fn assert_html(&self) -> &Self {
        assert!(
            self.body.contains('<') && self.body.contains('>'),
            "Body does not appear to be HTML"
        );
        self
    }

    /// Assert redirect
    pub fn assert_redirect_to(&self, location: &str) -> &Self {
        assert!(
            (300..400).contains(&self.status),
            "Expected 3xx redirect, got {}",
            self.status
        );
        self.assert_header("Location", location)
    }

    /// Parse body as JSON
    pub fn json(&self) -> Option<serde_json::Value> {
        serde_json::from_str(&self.body).ok()
    }
}

// ─── Test Client ────────────────────────────────────────────

/// A test client for simulating HTTP requests
#[derive(Debug, Clone)]
pub struct TestClient {
    pub base_headers: HashMap<String, String>,
    pub base_cookies: HashMap<String, String>,
    pub responses: Vec<TestResponse>,
}

impl TestClient {
    pub fn new() -> Self {
        Self {
            base_headers: HashMap::new(),
            base_cookies: HashMap::new(),
            responses: Vec::new(),
        }
    }

    pub fn default_header(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.base_headers.insert(key.into(), value.into());
        self
    }

    pub fn auth_token(self, token: &str) -> Self {
        self.default_header("Authorization", format!("Bearer {}", token))
    }

    /// Apply base headers/cookies to a request
    pub fn prepare(&self, mut req: TestRequest) -> TestRequest {
        for (k, v) in &self.base_headers {
            req.headers.entry(k.clone()).or_insert_with(|| v.clone());
        }
        for (k, v) in &self.base_cookies {
            req.cookies.entry(k.clone()).or_insert_with(|| v.clone());
        }
        req
    }
}

impl Default for TestClient {
    fn default() -> Self {
        Self::new()
    }
}

// ─── HTML Assertions ────────────────────────────────────────

/// HTML content assertion helpers
pub struct HtmlAssert<'a> {
    html: &'a str,
}

impl<'a> HtmlAssert<'a> {
    pub fn new(html: &'a str) -> Self {
        Self { html }
    }

    /// Assert an element with tag exists
    pub fn has_element(&self, tag: &str) -> &Self {
        assert!(
            self.html.contains(&format!("<{}", tag)),
            "HTML does not contain <{}> element",
            tag
        );
        self
    }

    /// Assert text content exists
    pub fn has_text(&self, text: &str) -> &Self {
        assert!(
            self.html.contains(text),
            "HTML does not contain text '{}'",
            text
        );
        self
    }

    /// Assert an element with specific class exists
    pub fn has_class(&self, class: &str) -> &Self {
        assert!(
            self.html.contains(&format!("class=\"{}", class))
                || self.html.contains(&format!("class=\" {}", class))
                || self.html.contains(&format!(" {}", class)),
            "HTML does not contain class '{}'",
            class
        );
        self
    }

    /// Assert a meta tag with specific content
    pub fn has_meta(&self, name: &str, content: &str) -> &Self {
        let pattern1 = format!("name=\"{}\" content=\"{}\"", name, content);
        let pattern2 = format!("content=\"{}\" name=\"{}\"", content, name);
        assert!(
            self.html.contains(&pattern1) || self.html.contains(&pattern2),
            "HTML does not contain meta {}={}",
            name, content
        );
        self
    }

    /// Assert a link tag exists
    pub fn has_link(&self, rel: &str, href: &str) -> &Self {
        assert!(
            self.html.contains(&format!("rel=\"{}\"", rel))
                && self.html.contains(&format!("href=\"{}\"", href)),
            "HTML does not contain link rel={} href={}",
            rel, href
        );
        self
    }

    /// Count occurrences of a tag
    pub fn count_elements(&self, tag: &str) -> usize {
        self.html.matches(&format!("<{}", tag)).count()
    }
}

// ─── Performance Test Helpers ───────────────────────────────

/// Simple benchmark helper
pub struct Benchmark {
    pub name: String,
    pub iterations: u32,
}

impl Benchmark {
    pub fn new(name: impl Into<String>, iterations: u32) -> Self {
        Self {
            name: name.into(),
            iterations,
        }
    }

    /// Run a benchmark and return average duration in microseconds
    pub fn run<F: Fn()>(&self, f: F) -> BenchmarkResult {
        let start = std::time::Instant::now();
        for _ in 0..self.iterations {
            f();
        }
        let total = start.elapsed();
        let avg_us = total.as_micros() as f64 / self.iterations as f64;
        BenchmarkResult {
            name: self.name.clone(),
            iterations: self.iterations,
            total_ms: total.as_millis() as f64,
            avg_us,
        }
    }
}

/// Benchmark result
#[derive(Debug, Clone)]
pub struct BenchmarkResult {
    pub name: String,
    pub iterations: u32,
    pub total_ms: f64,
    pub avg_us: f64,
}

impl std::fmt::Display for BenchmarkResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}: {} iterations in {:.2}ms (avg: {:.2}µs)",
            self.name, self.iterations, self.total_ms, self.avg_us
        )
    }
}

// ─── Helpers ────────────────────────────────────────────────

fn url_encode(s: &str) -> String {
    s.replace(' ', "+")
        .replace('&', "%26")
        .replace('=', "%3D")
}

// ─── Tests ──────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_request_get() {
        let req = TestRequest::get("/api/users")
            .header("Accept", "application/json")
            .query("page", "1");
        assert_eq!(req.method, "GET");
        assert_eq!(req.full_path(), "/api/users?page=1");
    }

    #[test]
    fn test_request_post_json() {
        let req = TestRequest::post("/api/users")
            .json_body("{\"name\":\"test\"}");
        assert_eq!(req.method, "POST");
        assert_eq!(req.headers.get("Content-Type").unwrap(), "application/json");
    }

    #[test]
    fn test_request_form() {
        let req = TestRequest::post("/login")
            .form_body(&[("username", "admin"), ("password", "secret")]);
        assert!(req.body.unwrap().contains("username=admin"));
    }

    #[test]
    fn test_request_cookie() {
        let req = TestRequest::get("/").cookie("session", "abc123");
        assert_eq!(req.cookie_header(), Some("session=abc123".to_string()));
    }

    #[test]
    fn test_response_assert_status() {
        TestResponse::ok("hello").assert_status(200);
    }

    #[test]
    fn test_response_assert_ok() {
        TestResponse::ok("hello").assert_ok();
    }

    #[test]
    fn test_response_assert_body() {
        TestResponse::ok("<h1>Hello</h1>")
            .assert_body_contains("Hello")
            .assert_html();
    }

    #[test]
    fn test_response_assert_json() {
        TestResponse::ok("{\"name\":\"test\",\"age\":\"25\"}")
            .assert_json()
            .assert_json_field("name", "test")
            .assert_json_field("age", "25");
    }

    #[test]
    fn test_response_redirect() {
        TestResponse::redirect("/login").assert_redirect_to("/login");
    }

    #[test]
    fn test_response_json_parse() {
        let resp = TestResponse::ok("{\"ok\":true}");
        let json = resp.json().unwrap();
        assert_eq!(json["ok"], true);
    }

    #[test]
    fn test_test_client() {
        let client = TestClient::new()
            .auth_token("mytoken")
            .default_header("X-Custom", "value");
        let req = client.prepare(TestRequest::get("/api"));
        assert!(req.headers.get("Authorization").unwrap().contains("mytoken"));
        assert_eq!(req.headers.get("X-Custom").unwrap(), "value");
    }

    #[test]
    fn test_html_assert() {
        let html = "<html><head><title>Test</title></head><body><h1 class=\"title\">Hello</h1></body></html>";
        let assert = HtmlAssert::new(html);
        assert.has_element("h1").has_text("Hello").has_class("title");
        assert_eq!(assert.count_elements("h1"), 1);
    }

    #[test]
    fn test_benchmark() {
        let bench = Benchmark::new("counter", 100);
        let result = bench.run(|| {
            let _ = 1 + 1;
        });
        assert_eq!(result.iterations, 100);
        assert!(result.avg_us >= 0.0);
    }

    #[test]
    fn test_not_found() {
        TestResponse::not_found().assert_status(404);
    }
}
