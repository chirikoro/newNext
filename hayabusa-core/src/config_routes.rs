//! TOML-Based Route Configuration for Hayabusa.
//!
//! Define routes, redirects, rewrites, and headers in `hayabusa.toml`
//! instead of writing Rust code. Changes take effect without recompilation.
//!
//! ## hayabusa.toml Example
//! ```toml
//! [app]
//! name = "My App"
//! port = 3000
//!
//! [[routes]]
//! path = "/"
//! template = "index.html"
//! mode = "ssg"
//!
//! [[routes]]
//! path = "/blog/:slug"
//! template = "blog-post.html"
//! mode = "isr"
//! revalidate = 60
//! data = "data/posts/:slug.json"
//!
//! [[redirects]]
//! from = "/old-page"
//! to = "/new-page"
//! status = 301
//!
//! [[rewrites]]
//! from = "/api/:path*"
//! to = "https://api.example.com/:path"
//!
//! [[headers]]
//! pattern = "/api/*"
//! values = { "Access-Control-Allow-Origin" = "*" }
//! ```

use std::collections::HashMap;
use std::path::Path;

/// Full application configuration parsed from TOML
#[derive(Debug, Clone)]
pub struct AppConfig {
    pub app: AppMeta,
    pub routes: Vec<RouteConfig>,
    pub redirects: Vec<RedirectConfig>,
    pub rewrites: Vec<RewriteConfig>,
    pub headers: Vec<HeaderConfig>,
}

/// Application metadata
#[derive(Debug, Clone)]
pub struct AppMeta {
    pub name: String,
    pub port: u16,
    pub host: String,
    pub static_dir: String,
    pub template_dir: String,
}

impl Default for AppMeta {
    fn default() -> Self {
        Self {
            name: "Hayabusa App".to_string(),
            port: 3000,
            host: "0.0.0.0".to_string(),
            static_dir: "public".to_string(),
            template_dir: "templates".to_string(),
        }
    }
}

/// A route definition
#[derive(Debug, Clone)]
pub struct RouteConfig {
    pub path: String,
    pub template: Option<String>,
    pub markdown: Option<String>,
    pub mode: RenderModeConfig,
    pub revalidate: Option<u64>,
    pub data: Option<String>,
    pub layout: Option<String>,
}

/// Rendering mode for config-based routes
#[derive(Debug, Clone, PartialEq)]
pub enum RenderModeConfig {
    Ssr,
    Ssg,
    Isr,
    Streaming,
}

impl RenderModeConfig {
    fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "ssg" | "static" => RenderModeConfig::Ssg,
            "isr" => RenderModeConfig::Isr,
            "streaming" => RenderModeConfig::Streaming,
            _ => RenderModeConfig::Ssr,
        }
    }
}

/// Redirect configuration
#[derive(Debug, Clone)]
pub struct RedirectConfig {
    pub from: String,
    pub to: String,
    pub status: u16,
    pub permanent: bool,
}

/// Rewrite configuration (URL rewriting, invisible to client)
#[derive(Debug, Clone)]
pub struct RewriteConfig {
    pub from: String,
    pub to: String,
}

/// Header configuration for specific routes
#[derive(Debug, Clone)]
pub struct HeaderConfig {
    pub pattern: String,
    pub values: HashMap<String, String>,
}

impl AppConfig {
    /// Parse configuration from a TOML string
    pub fn from_toml(toml_str: &str) -> Result<Self, ConfigError> {
        let value: serde_json::Value = parse_toml_to_json(toml_str)?;

        let app = parse_app_meta(&value);
        let routes = parse_routes(&value);
        let redirects = parse_redirects(&value);
        let rewrites = parse_rewrites(&value);
        let headers = parse_headers(&value);

        Ok(Self {
            app,
            routes,
            redirects,
            rewrites,
            headers,
        })
    }

    /// Load configuration from a file
    pub fn from_file(path: impl AsRef<Path>) -> Result<Self, ConfigError> {
        let content = std::fs::read_to_string(path.as_ref()).map_err(|e| {
            ConfigError::FileError(format!("Failed to read config: {}", e))
        })?;
        Self::from_toml(&content)
    }

    /// Get a route config by path pattern
    pub fn find_route(&self, path: &str) -> Option<&RouteConfig> {
        self.routes.iter().find(|r| route_matches(&r.path, path))
    }

    /// Get redirect for a path
    pub fn find_redirect(&self, path: &str) -> Option<&RedirectConfig> {
        self.redirects.iter().find(|r| r.from == path)
    }

    /// Get rewrite for a path
    pub fn find_rewrite(&self, path: &str) -> Option<&RewriteConfig> {
        self.rewrites.iter().find(|r| route_matches(&r.from, path))
    }

    /// Get headers for a path
    pub fn get_headers(&self, path: &str) -> HashMap<String, String> {
        let mut result = HashMap::new();
        for h in &self.headers {
            if pattern_matches(&h.pattern, path) {
                result.extend(h.values.clone());
            }
        }
        result
    }
}

/// Configuration errors
#[derive(Debug, Clone)]
pub enum ConfigError {
    ParseError(String),
    FileError(String),
    ValidationError(String),
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigError::ParseError(msg) => write!(f, "Config parse error: {}", msg),
            ConfigError::FileError(msg) => write!(f, "Config file error: {}", msg),
            ConfigError::ValidationError(msg) => write!(f, "Config validation error: {}", msg),
        }
    }
}

/// Simple TOML parser that converts to JSON-like structure.
///
/// Supports: strings, numbers, booleans, arrays, tables, inline tables.
fn parse_toml_to_json(toml: &str) -> Result<serde_json::Value, ConfigError> {
    let mut root = serde_json::Map::new();
    let mut current_table: Option<String> = None;
    let mut current_array_table: Option<String> = None;
    let mut array_items: HashMap<String, Vec<serde_json::Map<String, serde_json::Value>>> =
        HashMap::new();
    let mut table_data: serde_json::Map<String, serde_json::Value> = serde_json::Map::new();

    for line in toml.lines() {
        let line = line.trim();

        // Skip empty lines and comments
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        // Array of tables [[name]]
        if line.starts_with("[[") && line.ends_with("]]") {
            // Save previous table data
            flush_table_data(
                &mut root,
                &mut table_data,
                &current_table,
                &current_array_table,
                &mut array_items,
            );

            let name = line[2..line.len() - 2].trim().to_string();
            current_array_table = Some(name);
            current_table = None;
            continue;
        }

        // Table [name]
        if line.starts_with('[') && line.ends_with(']') {
            flush_table_data(
                &mut root,
                &mut table_data,
                &current_table,
                &current_array_table,
                &mut array_items,
            );

            let name = line[1..line.len() - 1].trim().to_string();
            current_table = Some(name);
            current_array_table = None;
            continue;
        }

        // Key = Value
        if let Some(eq_pos) = line.find('=') {
            let key = line[..eq_pos].trim().to_string();
            let value_str = line[eq_pos + 1..].trim();
            let value = parse_toml_value(value_str);
            table_data.insert(key, value);
        }
    }

    // Flush remaining data
    flush_table_data(
        &mut root,
        &mut table_data,
        &current_table,
        &current_array_table,
        &mut array_items,
    );

    // Convert array items to JSON arrays
    for (name, items) in array_items {
        root.insert(
            name,
            serde_json::Value::Array(
                items.into_iter().map(serde_json::Value::Object).collect(),
            ),
        );
    }

    Ok(serde_json::Value::Object(root))
}

fn flush_table_data(
    root: &mut serde_json::Map<String, serde_json::Value>,
    table_data: &mut serde_json::Map<String, serde_json::Value>,
    current_table: &Option<String>,
    current_array_table: &Option<String>,
    array_items: &mut HashMap<String, Vec<serde_json::Map<String, serde_json::Value>>>,
) {
    if table_data.is_empty() {
        return;
    }

    let data = std::mem::take(table_data);

    if let Some(ref name) = current_array_table {
        array_items
            .entry(name.clone())
            .or_default()
            .push(data);
    } else if let Some(ref name) = current_table {
        root.insert(name.clone(), serde_json::Value::Object(data));
    } else {
        // Top-level keys
        for (k, v) in data {
            root.insert(k, v);
        }
    }
}

fn parse_toml_value(s: &str) -> serde_json::Value {
    let s = s.trim();

    // String
    if s.starts_with('"') && s.ends_with('"') {
        return serde_json::Value::String(s[1..s.len() - 1].to_string());
    }

    // Boolean
    if s == "true" {
        return serde_json::Value::Bool(true);
    }
    if s == "false" {
        return serde_json::Value::Bool(false);
    }

    // Number (integer)
    if let Ok(n) = s.parse::<i64>() {
        return serde_json::Value::Number(serde_json::Number::from(n));
    }

    // Number (float)
    if let Ok(n) = s.parse::<f64>() {
        if let Some(num) = serde_json::Number::from_f64(n) {
            return serde_json::Value::Number(num);
        }
    }

    // Inline table { key = "value", ... }
    if s.starts_with('{') && s.ends_with('}') {
        let inner = &s[1..s.len() - 1];
        let mut map = serde_json::Map::new();
        for pair in inner.split(',') {
            let pair = pair.trim();
            if let Some(eq) = pair.find('=') {
                let k = pair[..eq].trim().trim_matches('"').to_string();
                let v = parse_toml_value(pair[eq + 1..].trim());
                map.insert(k, v);
            }
        }
        return serde_json::Value::Object(map);
    }

    // Array
    if s.starts_with('[') && s.ends_with(']') {
        let inner = &s[1..s.len() - 1];
        let items: Vec<serde_json::Value> = inner
            .split(',')
            .map(|item| parse_toml_value(item.trim()))
            .collect();
        return serde_json::Value::Array(items);
    }

    // Fallback to string
    serde_json::Value::String(s.to_string())
}

fn parse_app_meta(json: &serde_json::Value) -> AppMeta {
    let mut meta = AppMeta::default();
    if let Some(app) = json.get("app") {
        if let Some(name) = app.get("name").and_then(|v| v.as_str()) {
            meta.name = name.to_string();
        }
        if let Some(port) = app.get("port").and_then(|v| v.as_u64()) {
            meta.port = port as u16;
        }
        if let Some(host) = app.get("host").and_then(|v| v.as_str()) {
            meta.host = host.to_string();
        }
        if let Some(dir) = app.get("static_dir").and_then(|v| v.as_str()) {
            meta.static_dir = dir.to_string();
        }
        if let Some(dir) = app.get("template_dir").and_then(|v| v.as_str()) {
            meta.template_dir = dir.to_string();
        }
    }
    meta
}

fn parse_routes(json: &serde_json::Value) -> Vec<RouteConfig> {
    let mut routes = Vec::new();
    if let Some(arr) = json.get("routes").and_then(|v| v.as_array()) {
        for item in arr {
            let path = item.get("path").and_then(|v| v.as_str()).unwrap_or("/").to_string();
            let template = item.get("template").and_then(|v| v.as_str()).map(String::from);
            let markdown = item.get("markdown").and_then(|v| v.as_str()).map(String::from);
            let mode_str = item.get("mode").and_then(|v| v.as_str()).unwrap_or("ssr");
            let mode = RenderModeConfig::from_str(mode_str);
            let revalidate = item.get("revalidate").and_then(|v| v.as_u64());
            let data = item.get("data").and_then(|v| v.as_str()).map(String::from);
            let layout = item.get("layout").and_then(|v| v.as_str()).map(String::from);

            routes.push(RouteConfig {
                path,
                template,
                markdown,
                mode,
                revalidate,
                data,
                layout,
            });
        }
    }
    routes
}

fn parse_redirects(json: &serde_json::Value) -> Vec<RedirectConfig> {
    let mut redirects = Vec::new();
    if let Some(arr) = json.get("redirects").and_then(|v| v.as_array()) {
        for item in arr {
            let from = item.get("from").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let to = item.get("to").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let status = item.get("status").and_then(|v| v.as_u64()).unwrap_or(301) as u16;
            redirects.push(RedirectConfig {
                from,
                to,
                permanent: status == 301,
                status,
            });
        }
    }
    redirects
}

fn parse_rewrites(json: &serde_json::Value) -> Vec<RewriteConfig> {
    let mut rewrites = Vec::new();
    if let Some(arr) = json.get("rewrites").and_then(|v| v.as_array()) {
        for item in arr {
            let from = item.get("from").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let to = item.get("to").and_then(|v| v.as_str()).unwrap_or("").to_string();
            rewrites.push(RewriteConfig { from, to });
        }
    }
    rewrites
}

fn parse_headers(json: &serde_json::Value) -> Vec<HeaderConfig> {
    let mut headers = Vec::new();
    if let Some(arr) = json.get("headers").and_then(|v| v.as_array()) {
        for item in arr {
            let pattern = item.get("pattern").and_then(|v| v.as_str()).unwrap_or("*").to_string();
            let mut values = HashMap::new();
            if let Some(obj) = item.get("values").and_then(|v| v.as_object()) {
                for (k, v) in obj {
                    if let Some(s) = v.as_str() {
                        values.insert(k.clone(), s.to_string());
                    }
                }
            }
            headers.push(HeaderConfig { pattern, values });
        }
    }
    headers
}

/// Check if a route pattern matches a path
fn route_matches(pattern: &str, path: &str) -> bool {
    if pattern == path {
        return true;
    }

    let pat_parts: Vec<&str> = pattern.split('/').filter(|s| !s.is_empty()).collect();
    let path_parts: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();

    if pat_parts.len() != path_parts.len() {
        // Check for wildcard
        if let Some(last) = pat_parts.last() {
            if last.ends_with('*') {
                let prefix_len = pat_parts.len() - 1;
                return path_parts.len() >= prefix_len
                    && pat_parts[..prefix_len]
                        .iter()
                        .zip(path_parts.iter())
                        .all(|(p, s)| p.starts_with(':') || *p == *s);
            }
        }
        return false;
    }

    pat_parts
        .iter()
        .zip(path_parts.iter())
        .all(|(p, s)| p.starts_with(':') || *p == *s)
}

/// Check if a glob pattern matches a path
fn pattern_matches(pattern: &str, path: &str) -> bool {
    if pattern == "*" {
        return true;
    }
    if pattern.ends_with('*') {
        let prefix = &pattern[..pattern.len() - 1];
        return path.starts_with(prefix);
    }
    pattern == path
}

#[cfg(test)]
mod tests {
    use super::*;

    const EXAMPLE_TOML: &str = r#"
[app]
name = "My Blog"
port = 3000

[[routes]]
path = "/"
template = "index.html"
mode = "ssg"

[[routes]]
path = "/blog/:slug"
template = "blog-post.html"
mode = "isr"
revalidate = 60
data = "data/posts/:slug.json"

[[routes]]
path = "/about"
markdown = "content/about.md"
mode = "ssg"

[[redirects]]
from = "/old-page"
to = "/new-page"
status = 301

[[rewrites]]
from = "/api/:path"
to = "https://api.example.com/:path"

[[headers]]
pattern = "/api/*"
values = { "Access-Control-Allow-Origin" = "*" }
"#;

    #[test]
    fn test_parse_config() {
        let config = AppConfig::from_toml(EXAMPLE_TOML).unwrap();
        assert_eq!(config.app.name, "My Blog");
        assert_eq!(config.app.port, 3000);
    }

    #[test]
    fn test_parse_routes() {
        let config = AppConfig::from_toml(EXAMPLE_TOML).unwrap();
        assert_eq!(config.routes.len(), 3);

        assert_eq!(config.routes[0].path, "/");
        assert_eq!(config.routes[0].template.as_deref(), Some("index.html"));
        assert_eq!(config.routes[0].mode, RenderModeConfig::Ssg);

        assert_eq!(config.routes[1].path, "/blog/:slug");
        assert_eq!(config.routes[1].mode, RenderModeConfig::Isr);
        assert_eq!(config.routes[1].revalidate, Some(60));
    }

    #[test]
    fn test_parse_markdown_route() {
        let config = AppConfig::from_toml(EXAMPLE_TOML).unwrap();
        assert_eq!(config.routes[2].markdown.as_deref(), Some("content/about.md"));
    }

    #[test]
    fn test_parse_redirects() {
        let config = AppConfig::from_toml(EXAMPLE_TOML).unwrap();
        assert_eq!(config.redirects.len(), 1);
        assert_eq!(config.redirects[0].from, "/old-page");
        assert_eq!(config.redirects[0].to, "/new-page");
        assert_eq!(config.redirects[0].status, 301);
        assert!(config.redirects[0].permanent);
    }

    #[test]
    fn test_parse_rewrites() {
        let config = AppConfig::from_toml(EXAMPLE_TOML).unwrap();
        assert_eq!(config.rewrites.len(), 1);
        assert_eq!(config.rewrites[0].from, "/api/:path");
    }

    #[test]
    fn test_parse_headers() {
        let config = AppConfig::from_toml(EXAMPLE_TOML).unwrap();
        assert_eq!(config.headers.len(), 1);
        assert_eq!(config.headers[0].pattern, "/api/*");
        assert_eq!(
            config.headers[0].values.get("Access-Control-Allow-Origin"),
            Some(&"*".to_string())
        );
    }

    #[test]
    fn test_find_route() {
        let config = AppConfig::from_toml(EXAMPLE_TOML).unwrap();
        assert!(config.find_route("/").is_some());
        assert!(config.find_route("/blog/hello-world").is_some());
        assert!(config.find_route("/nonexistent").is_none());
    }

    #[test]
    fn test_find_redirect() {
        let config = AppConfig::from_toml(EXAMPLE_TOML).unwrap();
        assert!(config.find_redirect("/old-page").is_some());
        assert!(config.find_redirect("/new-page").is_none());
    }

    #[test]
    fn test_get_headers() {
        let config = AppConfig::from_toml(EXAMPLE_TOML).unwrap();
        let headers = config.get_headers("/api/users");
        assert_eq!(headers.get("Access-Control-Allow-Origin"), Some(&"*".to_string()));

        let headers = config.get_headers("/about");
        assert!(headers.is_empty());
    }

    #[test]
    fn test_route_matching() {
        assert!(route_matches("/", "/"));
        assert!(route_matches("/blog/:slug", "/blog/hello"));
        assert!(!route_matches("/blog/:slug", "/blog/a/b"));
        assert!(route_matches("/about", "/about"));
        assert!(!route_matches("/about", "/contact"));
    }

    #[test]
    fn test_default_app_meta() {
        let config = AppConfig::from_toml("").unwrap();
        assert_eq!(config.app.port, 3000);
        assert_eq!(config.app.name, "Hayabusa App");
    }
}
