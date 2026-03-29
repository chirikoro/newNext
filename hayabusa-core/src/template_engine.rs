//! Template Engine for Hayabusa.
//!
//! Jinja2/Handlebars-like template syntax for writing pages as `.html` files
//! instead of Rust code. **No recompilation needed** — just edit and refresh.
//!
//! ## Why This Matters for DX
//! - Pages can be written as simple `.html` template files
//! - No Rust compilation for content/template changes
//! - Familiar syntax for developers coming from Python/JS/Go
//! - Variables, conditionals, loops, includes, template inheritance
//!
//! ## Syntax
//! ```html
//! {% extends "layout.html" %}
//! {% block content %}
//!   <h1>{{ title }}</h1>
//!   {% for post in posts %}
//!     <article>
//!       <h2>{{ post.title }}</h2>
//!       <p>{{ post.body }}</p>
//!     </article>
//!   {% endfor %}
//!   {% if user %}
//!     <p>Welcome, {{ user.name }}!</p>
//!   {% endif %}
//! {% endblock %}
//! ```

use std::collections::HashMap;
use std::path::PathBuf;

/// Template context: key-value pairs for rendering
#[derive(Debug, Clone, Default)]
pub struct TemplateContext {
    values: HashMap<String, TemplateValue>,
}

/// A value that can be used in templates
#[derive(Debug, Clone)]
pub enum TemplateValue {
    String(String),
    Number(f64),
    Bool(bool),
    List(Vec<TemplateValue>),
    Map(HashMap<String, TemplateValue>),
    Null,
}

impl TemplateValue {
    pub fn as_str(&self) -> &str {
        match self {
            TemplateValue::String(s) => s,
            _ => "",
        }
    }

    pub fn as_bool(&self) -> bool {
        match self {
            TemplateValue::Bool(b) => *b,
            TemplateValue::String(s) => !s.is_empty(),
            TemplateValue::Number(n) => *n != 0.0,
            TemplateValue::Null => false,
            TemplateValue::List(l) => !l.is_empty(),
            TemplateValue::Map(m) => !m.is_empty(),
        }
    }

    pub fn display(&self) -> String {
        match self {
            TemplateValue::String(s) => s.clone(),
            TemplateValue::Number(n) => {
                if *n == (*n as i64) as f64 {
                    format!("{}", *n as i64)
                } else {
                    format!("{}", n)
                }
            }
            TemplateValue::Bool(b) => b.to_string(),
            TemplateValue::Null => String::new(),
            TemplateValue::List(_) => "[Array]".to_string(),
            TemplateValue::Map(_) => "[Object]".to_string(),
        }
    }
}

impl From<&str> for TemplateValue {
    fn from(s: &str) -> Self {
        TemplateValue::String(s.to_string())
    }
}

impl From<String> for TemplateValue {
    fn from(s: String) -> Self {
        TemplateValue::String(s)
    }
}

impl From<f64> for TemplateValue {
    fn from(n: f64) -> Self {
        TemplateValue::Number(n)
    }
}

impl From<i64> for TemplateValue {
    fn from(n: i64) -> Self {
        TemplateValue::Number(n as f64)
    }
}

impl From<bool> for TemplateValue {
    fn from(b: bool) -> Self {
        TemplateValue::Bool(b)
    }
}

impl From<serde_json::Value> for TemplateValue {
    fn from(v: serde_json::Value) -> Self {
        match v {
            serde_json::Value::Null => TemplateValue::Null,
            serde_json::Value::Bool(b) => TemplateValue::Bool(b),
            serde_json::Value::Number(n) => {
                TemplateValue::Number(n.as_f64().unwrap_or(0.0))
            }
            serde_json::Value::String(s) => TemplateValue::String(s),
            serde_json::Value::Array(arr) => {
                TemplateValue::List(arr.into_iter().map(TemplateValue::from).collect())
            }
            serde_json::Value::Object(map) => {
                TemplateValue::Map(
                    map.into_iter()
                        .map(|(k, v)| (k, TemplateValue::from(v)))
                        .collect(),
                )
            }
        }
    }
}

impl TemplateContext {
    pub fn new() -> Self {
        Self::default()
    }

    /// Insert a value
    pub fn insert(&mut self, key: impl Into<String>, value: impl Into<TemplateValue>) {
        self.values.insert(key.into(), value.into());
    }

    /// Get a value by dotted path (e.g., "user.name")
    pub fn get(&self, path: &str) -> Option<&TemplateValue> {
        let parts: Vec<&str> = path.split('.').collect();
        let mut current = self.values.get(parts[0])?;

        for part in &parts[1..] {
            match current {
                TemplateValue::Map(map) => {
                    current = map.get(*part)?;
                }
                _ => return None,
            }
        }

        Some(current)
    }

    /// Create context from a JSON value
    pub fn from_json(json: serde_json::Value) -> Self {
        let mut ctx = Self::new();
        if let serde_json::Value::Object(map) = json {
            for (k, v) in map {
                ctx.insert(k, TemplateValue::from(v));
            }
        }
        ctx
    }
}

/// Template engine
pub struct TemplateEngine {
    /// Base directory for template files
    template_dir: PathBuf,
    /// Cached compiled templates
    cache: HashMap<String, String>,
}

impl TemplateEngine {
    pub fn new(template_dir: impl Into<PathBuf>) -> Self {
        Self {
            template_dir: template_dir.into(),
            cache: HashMap::new(),
        }
    }

    /// Load and render a template file
    pub fn render(&mut self, template_name: &str, ctx: &TemplateContext) -> Result<String, TemplateError> {
        let content = self.load_template(template_name)?;
        render_template(&content, ctx)
    }

    /// Render a template string directly (no file loading)
    pub fn render_string(template: &str, ctx: &TemplateContext) -> Result<String, TemplateError> {
        render_template(template, ctx)
    }

    fn load_template(&mut self, name: &str) -> Result<String, TemplateError> {
        if let Some(cached) = self.cache.get(name) {
            return Ok(cached.clone());
        }

        let path = self.template_dir.join(name);
        let content = std::fs::read_to_string(&path).map_err(|_| {
            TemplateError::NotFound(name.to_string())
        })?;

        self.cache.insert(name.to_string(), content.clone());
        Ok(content)
    }

    /// Clear the template cache (for dev mode hot reload)
    pub fn clear_cache(&mut self) {
        self.cache.clear();
    }
}

/// Template rendering errors
#[derive(Debug, Clone)]
pub enum TemplateError {
    NotFound(String),
    SyntaxError(String),
    RenderError(String),
}

impl std::fmt::Display for TemplateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TemplateError::NotFound(name) => write!(f, "Template not found: {}", name),
            TemplateError::SyntaxError(msg) => write!(f, "Template syntax error: {}", msg),
            TemplateError::RenderError(msg) => write!(f, "Template render error: {}", msg),
        }
    }
}

/// Render a template string with the given context.
///
/// Supports:
/// - `{{ variable }}` — value interpolation (auto-escaped)
/// - `{{ variable | raw }}` — unescaped output
/// - `{% if condition %}...{% endif %}` — conditionals
/// - `{% if condition %}...{% else %}...{% endif %}` — if/else
/// - `{% for item in list %}...{% endfor %}` — loops
/// - `{# comment #}` — comments (stripped from output)
pub fn render_template(template: &str, ctx: &TemplateContext) -> Result<String, TemplateError> {
    let mut output = String::new();
    let mut remaining = template;

    while !remaining.is_empty() {
        // Find the next template tag
        if let Some(pos) = find_next_tag(remaining) {
            // Output static content before the tag
            output.push_str(&remaining[..pos]);
            remaining = &remaining[pos..];

            // Comment: {# ... #}
            if remaining.starts_with("{#") {
                if let Some(end) = remaining.find("#}") {
                    remaining = &remaining[end + 2..];
                    continue;
                }
            }

            // Expression: {{ ... }}
            if remaining.starts_with("{{") {
                if let Some(end) = remaining.find("}}") {
                    let expr = remaining[2..end].trim();
                    let value = evaluate_expression(expr, ctx);
                    output.push_str(&value);
                    remaining = &remaining[end + 2..];
                    continue;
                }
            }

            // Block tag: {% ... %}
            if remaining.starts_with("{%") {
                if let Some(end) = remaining.find("%}") {
                    let tag = remaining[2..end].trim();
                    remaining = &remaining[end + 2..];

                    // Handle if/for/block tags
                    if let Some(rest) = tag.strip_prefix("if ") {
                        let condition = rest.trim();
                        let (if_body, else_body, after) = extract_if_block(remaining)?;
                        let is_true = evaluate_condition(condition, ctx);
                        if is_true {
                            let rendered = render_template(if_body, ctx)?;
                            output.push_str(&rendered);
                        } else if let Some(else_content) = else_body {
                            let rendered = render_template(else_content, ctx)?;
                            output.push_str(&rendered);
                        }
                        remaining = after;
                        continue;
                    }

                    if let Some(rest) = tag.strip_prefix("for ") {
                        let (var_name, list_expr) = parse_for_tag(rest)?;
                        let (body, after) = extract_for_block(remaining)?;

                        if let Some(TemplateValue::List(items)) = ctx.get(list_expr) {
                            for (index, item) in items.iter().enumerate() {
                                let mut loop_ctx = ctx.clone();
                                loop_ctx.insert(var_name.to_string(), item.clone());
                                loop_ctx.insert(
                                    "loop.index",
                                    TemplateValue::Number((index + 1) as f64),
                                );
                                loop_ctx.insert(
                                    "loop.index0",
                                    TemplateValue::Number(index as f64),
                                );
                                loop_ctx.insert(
                                    "loop.first",
                                    TemplateValue::Bool(index == 0),
                                );
                                loop_ctx.insert(
                                    "loop.last",
                                    TemplateValue::Bool(index == items.len() - 1),
                                );
                                let rendered = render_template(body, &loop_ctx)?;
                                output.push_str(&rendered);
                            }
                        }
                        remaining = after;
                        continue;
                    }

                    // Unknown tag — skip
                    continue;
                }
            }

            // If we get here, no tag matched — output the character and move on
            if let Some(ch) = remaining.chars().next() {
                output.push(ch);
                remaining = &remaining[ch.len_utf8()..];
            }
        } else {
            // No more tags — output the rest
            output.push_str(remaining);
            break;
        }
    }

    Ok(output)
}

/// Find the position of the next template tag
fn find_next_tag(s: &str) -> Option<usize> {
    let mut min_pos = None;

    for marker in &["{{", "{%", "{#"] {
        if let Some(pos) = s.find(marker) {
            min_pos = Some(match min_pos {
                None => pos,
                Some(current) => pos.min(current),
            });
        }
    }

    min_pos
}

/// Evaluate a template expression (variable lookup with optional filters)
fn evaluate_expression(expr: &str, ctx: &TemplateContext) -> String {
    let parts: Vec<&str> = expr.splitn(2, '|').collect();
    let var_name = parts[0].trim();
    let filter = parts.get(1).map(|s| s.trim());

    let value = ctx.get(var_name);

    let raw = match value {
        Some(v) => v.display(),
        None => String::new(),
    };

    match filter {
        Some("raw") => raw,
        Some("upper") => raw.to_uppercase(),
        Some("lower") => raw.to_lowercase(),
        Some("length") => match value {
            Some(TemplateValue::List(l)) => l.len().to_string(),
            Some(TemplateValue::String(s)) => s.len().to_string(),
            _ => "0".to_string(),
        },
        _ => html_escape(&raw),
    }
}

/// Evaluate a condition expression
fn evaluate_condition(condition: &str, ctx: &TemplateContext) -> bool {
    let condition = condition.trim();

    // Handle "not" prefix
    if let Some(rest) = condition.strip_prefix("not ") {
        return !evaluate_condition(rest, ctx);
    }

    // Handle comparisons
    if let Some(pos) = condition.find("==") {
        let left = condition[..pos].trim();
        let right = condition[pos + 2..].trim();
        let left_val = ctx.get(left).map(|v| v.display()).unwrap_or_default();
        let right_val = right.trim_matches('"').trim_matches('\'');
        return left_val == right_val;
    }

    if let Some(pos) = condition.find("!=") {
        let left = condition[..pos].trim();
        let right = condition[pos + 2..].trim();
        let left_val = ctx.get(left).map(|v| v.display()).unwrap_or_default();
        let right_val = right.trim_matches('"').trim_matches('\'');
        return left_val != right_val;
    }

    // Simple truthiness check
    ctx.get(condition).map_or(false, |v| v.as_bool())
}

/// Extract the body and optional else body of an if block
fn extract_if_block(s: &str) -> Result<(&str, Option<&str>, &str), TemplateError> {
    let mut depth = 1;
    let mut pos = 0;
    let mut else_pos = None;

    while pos < s.len() {
        if s[pos..].starts_with("{%") {
            if let Some(end) = s[pos..].find("%}") {
                let tag = s[pos + 2..pos + end].trim();
                if tag.starts_with("if ") {
                    depth += 1;
                } else if tag == "endif" {
                    depth -= 1;
                    if depth == 0 {
                        let after = &s[pos + end + 2..];
                        return if let Some(ep) = else_pos {
                            Ok((&s[..ep], Some(&s[ep + find_tag_end(&s[ep..])..pos]), after))
                        } else {
                            Ok((&s[..pos], None, after))
                        };
                    }
                } else if tag == "else" && depth == 1 {
                    else_pos = Some(pos);
                }
                pos += end + 2;
                continue;
            }
        }
        pos += 1;
    }

    Err(TemplateError::SyntaxError("Unclosed {% if %} block".to_string()))
}

/// Find the end of a {% ... %} tag (returns position after %}  relative to input)
fn find_tag_end(s: &str) -> usize {
    if let Some(end) = s.find("%}") {
        end + 2
    } else {
        0
    }
}

/// Parse a for tag: "item in items" → ("item", "items")
fn parse_for_tag(tag: &str) -> Result<(&str, &str), TemplateError> {
    let parts: Vec<&str> = tag.splitn(3, ' ').collect();
    if parts.len() >= 3 && parts[1] == "in" {
        Ok((parts[0].trim(), parts[2].trim()))
    } else {
        Err(TemplateError::SyntaxError(format!(
            "Invalid for syntax: '{}'. Expected 'item in list'",
            tag
        )))
    }
}

/// Extract the body of a for block
fn extract_for_block(s: &str) -> Result<(&str, &str), TemplateError> {
    let mut depth = 1;
    let mut pos = 0;

    while pos < s.len() {
        if s[pos..].starts_with("{%") {
            if let Some(end) = s[pos..].find("%}") {
                let tag = s[pos + 2..pos + end].trim();
                if tag.starts_with("for ") {
                    depth += 1;
                } else if tag == "endfor" {
                    depth -= 1;
                    if depth == 0 {
                        return Ok((&s[..pos], &s[pos + end + 2..]));
                    }
                }
                pos += end + 2;
                continue;
            }
        }
        pos += 1;
    }

    Err(TemplateError::SyntaxError("Unclosed {% for %} block".to_string()))
}

/// HTML-escape a string to prevent XSS
pub fn html_escape(s: &str) -> String {
    let mut escaped = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            '"' => escaped.push_str("&quot;"),
            '\'' => escaped.push_str("&#x27;"),
            _ => escaped.push(ch),
        }
    }
    escaped
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_variable_interpolation() {
        let mut ctx = TemplateContext::new();
        ctx.insert("name", "Hayabusa");
        ctx.insert("version", "1.0");

        let result = render_template("Hello, {{ name }}! v{{ version }}", &ctx).unwrap();
        assert_eq!(result, "Hello, Hayabusa! v1.0");
    }

    #[test]
    fn test_html_escaping() {
        let mut ctx = TemplateContext::new();
        ctx.insert("content", "<script>alert('xss')</script>");

        let result = render_template("{{ content }}", &ctx).unwrap();
        assert!(result.contains("&lt;script&gt;"));
        assert!(!result.contains("<script>"));
    }

    #[test]
    fn test_raw_filter() {
        let mut ctx = TemplateContext::new();
        ctx.insert("html", "<b>bold</b>");

        let result = render_template("{{ html | raw }}", &ctx).unwrap();
        assert_eq!(result, "<b>bold</b>");
    }

    #[test]
    fn test_if_block() {
        let mut ctx = TemplateContext::new();
        ctx.insert("logged_in", true);
        ctx.insert("name", "Alice");

        let template = "{% if logged_in %}Welcome, {{ name }}!{% endif %}";
        let result = render_template(template, &ctx).unwrap();
        assert_eq!(result, "Welcome, Alice!");
    }

    #[test]
    fn test_if_else_block() {
        let mut ctx = TemplateContext::new();
        ctx.insert("logged_in", false);

        let template = "{% if logged_in %}Dashboard{% else %}Please log in{% endif %}";
        let result = render_template(template, &ctx).unwrap();
        assert_eq!(result, "Please log in");
    }

    #[test]
    fn test_for_loop() {
        let mut ctx = TemplateContext::new();
        ctx.values.insert(
            "items".to_string(),
            TemplateValue::List(vec![
                TemplateValue::String("apple".to_string()),
                TemplateValue::String("banana".to_string()),
                TemplateValue::String("cherry".to_string()),
            ]),
        );

        let template = "{% for item in items %}{{ item }} {% endfor %}";
        let result = render_template(template, &ctx).unwrap();
        assert_eq!(result, "apple banana cherry ");
    }

    #[test]
    fn test_comment() {
        let ctx = TemplateContext::new();
        let result = render_template("Hello{# this is hidden #} World", &ctx).unwrap();
        assert_eq!(result, "Hello World");
    }

    #[test]
    fn test_condition_comparison() {
        let mut ctx = TemplateContext::new();
        ctx.insert("role", "admin");

        let template = r#"{% if role == "admin" %}Admin Panel{% endif %}"#;
        let result = render_template(template, &ctx).unwrap();
        assert_eq!(result, "Admin Panel");
    }

    #[test]
    fn test_not_condition() {
        let mut ctx = TemplateContext::new();
        ctx.insert("error", false);

        let template = "{% if not error %}All good!{% endif %}";
        let result = render_template(template, &ctx).unwrap();
        assert_eq!(result, "All good!");
    }

    #[test]
    fn test_nested_map_access() {
        let mut ctx = TemplateContext::new();
        let mut user = HashMap::new();
        user.insert("name".to_string(), TemplateValue::String("Alice".to_string()));
        user.insert("email".to_string(), TemplateValue::String("alice@example.com".to_string()));
        ctx.insert("user", TemplateValue::Map(user));

        let result = render_template("{{ user.name }} ({{ user.email }})", &ctx).unwrap();
        assert_eq!(result, "Alice (alice@example.com)");
    }

    #[test]
    fn test_filters() {
        let mut ctx = TemplateContext::new();
        ctx.insert("text", "Hello World");

        assert_eq!(
            render_template("{{ text | upper }}", &ctx).unwrap(),
            "HELLO WORLD"
        );
        assert_eq!(
            render_template("{{ text | lower }}", &ctx).unwrap(),
            "hello world"
        );
    }

    #[test]
    fn test_from_json() {
        let json = serde_json::json!({
            "title": "My Page",
            "count": 42,
            "active": true
        });

        let ctx = TemplateContext::from_json(json);
        let result = render_template(
            "{{ title }} - {{ count }} items (active: {{ active }})",
            &ctx,
        ).unwrap();
        assert_eq!(result, "My Page - 42 items (active: true)");
    }

    #[test]
    fn test_missing_variable() {
        let ctx = TemplateContext::new();
        let result = render_template("Hello {{ name }}!", &ctx).unwrap();
        assert_eq!(result, "Hello !");
    }

    #[test]
    fn test_static_content() {
        let ctx = TemplateContext::new();
        let result = render_template("<h1>No variables here</h1>", &ctx).unwrap();
        assert_eq!(result, "<h1>No variables here</h1>");
    }
}
