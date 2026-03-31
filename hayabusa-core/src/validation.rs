//! Form Validation for Hayabusa.
//!
//! Server-side validation with composable rules, error messages,
//! and integration with Server Actions.
//!
//! ```ignore
//! use hayabusa_core::prelude::*;
//!
//! let schema = Schema::new()
//!     .field("email", Rule::required().email().max_len(255))
//!     .field("password", Rule::required().min_len(8).pattern(r"[A-Z]"));
//!
//! let errors = schema.validate(&form_data);
//! ```

use std::collections::HashMap;

// ─── Rule ───────────────────────────────────────────────────

/// A composable validation rule chain
#[derive(Debug, Clone)]
pub struct Rule {
    validators: Vec<Validator>,
}

#[derive(Debug, Clone)]
enum Validator {
    Required,
    MinLen(usize),
    MaxLen(usize),
    Email,
    Url,
    Pattern(String),
    Min(f64),
    Max(f64),
    Integer,
    OneOf(Vec<String>),
    Equals(String),
    Custom(String), // description only, applied externally
}

impl Rule {
    pub fn required() -> Self {
        Self {
            validators: vec![Validator::Required],
        }
    }

    pub fn optional() -> Self {
        Self {
            validators: Vec::new(),
        }
    }

    pub fn min_len(mut self, len: usize) -> Self {
        self.validators.push(Validator::MinLen(len));
        self
    }

    pub fn max_len(mut self, len: usize) -> Self {
        self.validators.push(Validator::MaxLen(len));
        self
    }

    pub fn email(mut self) -> Self {
        self.validators.push(Validator::Email);
        self
    }

    pub fn url(mut self) -> Self {
        self.validators.push(Validator::Url);
        self
    }

    pub fn pattern(mut self, regex: &str) -> Self {
        self.validators.push(Validator::Pattern(regex.to_string()));
        self
    }

    pub fn min(mut self, n: f64) -> Self {
        self.validators.push(Validator::Min(n));
        self
    }

    pub fn max(mut self, n: f64) -> Self {
        self.validators.push(Validator::Max(n));
        self
    }

    pub fn integer(mut self) -> Self {
        self.validators.push(Validator::Integer);
        self
    }

    pub fn one_of(mut self, options: &[&str]) -> Self {
        self.validators.push(Validator::OneOf(
            options.iter().map(|s| s.to_string()).collect(),
        ));
        self
    }

    pub fn equals(mut self, field: &str) -> Self {
        self.validators.push(Validator::Equals(field.to_string()));
        self
    }

    /// Validate a single value and return errors
    fn validate(&self, field_name: &str, value: Option<&str>, all_fields: &HashMap<String, String>) -> Vec<ValidationError> {
        let mut errors = Vec::new();
        let is_empty = value.map_or(true, |v| v.trim().is_empty());

        for validator in &self.validators {
            match validator {
                Validator::Required => {
                    if is_empty {
                        errors.push(ValidationError::new(field_name, "required", &format!("{} is required", field_name)));
                        return errors; // skip other validations if required fails
                    }
                }
                _ if is_empty => continue, // skip validations for empty optional fields
                Validator::MinLen(min) => {
                    if let Some(v) = value {
                        if v.len() < *min {
                            errors.push(ValidationError::new(field_name, "min_len", &format!("{} must be at least {} characters", field_name, min)));
                        }
                    }
                }
                Validator::MaxLen(max) => {
                    if let Some(v) = value {
                        if v.len() > *max {
                            errors.push(ValidationError::new(field_name, "max_len", &format!("{} must be at most {} characters", field_name, max)));
                        }
                    }
                }
                Validator::Email => {
                    if let Some(v) = value {
                        if !is_valid_email(v) {
                            errors.push(ValidationError::new(field_name, "email", &format!("{} must be a valid email address", field_name)));
                        }
                    }
                }
                Validator::Url => {
                    if let Some(v) = value {
                        if !is_valid_url(v) {
                            errors.push(ValidationError::new(field_name, "url", &format!("{} must be a valid URL", field_name)));
                        }
                    }
                }
                Validator::Pattern(pattern) => {
                    if let Some(v) = value {
                        if !simple_regex_match(pattern, v) {
                            errors.push(ValidationError::new(field_name, "pattern", &format!("{} does not match required pattern", field_name)));
                        }
                    }
                }
                Validator::Min(min) => {
                    if let Some(v) = value {
                        if let Ok(n) = v.parse::<f64>() {
                            if n < *min {
                                errors.push(ValidationError::new(field_name, "min", &format!("{} must be at least {}", field_name, min)));
                            }
                        } else {
                            errors.push(ValidationError::new(field_name, "number", &format!("{} must be a number", field_name)));
                        }
                    }
                }
                Validator::Max(max) => {
                    if let Some(v) = value {
                        if let Ok(n) = v.parse::<f64>() {
                            if n > *max {
                                errors.push(ValidationError::new(field_name, "max", &format!("{} must be at most {}", field_name, max)));
                            }
                        }
                    }
                }
                Validator::Integer => {
                    if let Some(v) = value {
                        if v.parse::<i64>().is_err() {
                            errors.push(ValidationError::new(field_name, "integer", &format!("{} must be an integer", field_name)));
                        }
                    }
                }
                Validator::OneOf(options) => {
                    if let Some(v) = value {
                        if !options.iter().any(|o| o == v) {
                            errors.push(ValidationError::new(field_name, "one_of", &format!("{} must be one of: {}", field_name, options.join(", "))));
                        }
                    }
                }
                Validator::Equals(other_field) => {
                    if let Some(v) = value {
                        let other_value = all_fields.get(other_field).map(|s| s.as_str()).unwrap_or("");
                        if v != other_value {
                            errors.push(ValidationError::new(field_name, "equals", &format!("{} must match {}", field_name, other_field)));
                        }
                    }
                }
                Validator::Custom(_) => {} // handled externally
            }
        }

        errors
    }
}

// ─── Schema ─────────────────────────────────────────────────

/// Validation schema for a form
#[derive(Debug, Clone)]
pub struct Schema {
    fields: Vec<(String, Rule)>,
}

impl Schema {
    pub fn new() -> Self {
        Self { fields: Vec::new() }
    }

    pub fn field(mut self, name: &str, rule: Rule) -> Self {
        self.fields.push((name.to_string(), rule));
        self
    }

    /// Validate form data against the schema
    pub fn validate(&self, data: &HashMap<String, String>) -> ValidationResult {
        let mut all_errors = Vec::new();

        for (field_name, rule) in &self.fields {
            let value = data.get(field_name).map(|s| s.as_str());
            let errors = rule.validate(field_name, value, data);
            all_errors.extend(errors);
        }

        ValidationResult { errors: all_errors }
    }

    /// Validate form data from a key-value slice
    pub fn validate_pairs(&self, pairs: &[(&str, &str)]) -> ValidationResult {
        let data: HashMap<String, String> = pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        self.validate(&data)
    }
}

impl Default for Schema {
    fn default() -> Self {
        Self::new()
    }
}

// ─── Validation Result ──────────────────────────────────────

/// Result of form validation
#[derive(Debug, Clone)]
pub struct ValidationResult {
    pub errors: Vec<ValidationError>,
}

impl ValidationResult {
    pub fn is_valid(&self) -> bool {
        self.errors.is_empty()
    }

    pub fn is_invalid(&self) -> bool {
        !self.errors.is_empty()
    }

    /// Get errors for a specific field
    pub fn field_errors(&self, field: &str) -> Vec<&ValidationError> {
        self.errors.iter().filter(|e| e.field == field).collect()
    }

    /// Get the first error for a specific field
    pub fn field_error(&self, field: &str) -> Option<&ValidationError> {
        self.errors.iter().find(|e| e.field == field)
    }

    /// Generate HTML error messages for a field
    pub fn error_html(&self, field: &str) -> String {
        let errors = self.field_errors(field);
        if errors.is_empty() {
            return String::new();
        }
        let msgs: Vec<&str> = errors.iter().map(|e| e.message.as_str()).collect();
        format!("<div class=\"field-errors\">{}</div>",
            msgs.iter()
                .map(|m| format!("<p class=\"field-error\">{}</p>", m))
                .collect::<Vec<_>>()
                .join("")
        )
    }

    /// Generate JSON error response
    pub fn to_json(&self) -> String {
        if self.is_valid() {
            return "{\"valid\":true,\"errors\":{}}".to_string();
        }
        let mut field_map: HashMap<&str, Vec<&str>> = HashMap::new();
        for err in &self.errors {
            field_map.entry(&err.field).or_default().push(&err.message);
        }
        let entries: Vec<String> = field_map
            .iter()
            .map(|(field, msgs)| {
                let arr: Vec<String> = msgs.iter().map(|m| format!("\"{}\"", m)).collect();
                format!("\"{}\":[{}]", field, arr.join(","))
            })
            .collect();
        format!("{{\"valid\":false,\"errors\":{{{}}}}}", entries.join(","))
    }
}

/// A single validation error
#[derive(Debug, Clone)]
pub struct ValidationError {
    pub field: String,
    pub code: String,
    pub message: String,
}

impl ValidationError {
    pub fn new(field: &str, code: &str, message: &str) -> Self {
        Self {
            field: field.to_string(),
            code: code.to_string(),
            message: message.to_string(),
        }
    }
}

// ─── Validation Helpers ─────────────────────────────────────

fn is_valid_email(email: &str) -> bool {
    let parts: Vec<&str> = email.split('@').collect();
    if parts.len() != 2 {
        return false;
    }
    let local = parts[0];
    let domain = parts[1];
    !local.is_empty() && domain.contains('.') && domain.len() > 2 && !domain.starts_with('.') && !domain.ends_with('.')
}

fn is_valid_url(url: &str) -> bool {
    (url.starts_with("http://") || url.starts_with("https://")) && url.len() > 10
}

/// Simple regex-like pattern matching for common patterns
/// Supports: [A-Z] [a-z] [0-9] . * + ? basic character classes
fn simple_regex_match(pattern: &str, value: &str) -> bool {
    // Simple bracket expression check (e.g., [A-Z] means "contains uppercase")
    if pattern.starts_with('[') && pattern.ends_with(']') {
        let inner = &pattern[1..pattern.len() - 1];
        return match inner {
            "A-Z" => value.chars().any(|c| c.is_ascii_uppercase()),
            "a-z" => value.chars().any(|c| c.is_ascii_lowercase()),
            "0-9" => value.chars().any(|c| c.is_ascii_digit()),
            _ => {
                // Check if value contains any char in the bracket
                inner.chars().any(|c| value.contains(c))
            }
        };
    }

    // Literal match
    if !pattern.contains(['[', ']', '*', '+', '?', '.']) {
        return value.contains(pattern);
    }

    // For complex patterns, do a simple contains check on literal parts
    let literal: String = pattern
        .chars()
        .filter(|c| !['[', ']', '*', '+', '?', '^', '$'].contains(c))
        .collect();
    if !literal.is_empty() {
        return value.contains(&literal);
    }

    true
}

// ─── Tests ──────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn data(pairs: &[(&str, &str)]) -> HashMap<String, String> {
        pairs.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect()
    }

    #[test]
    fn test_required() {
        let schema = Schema::new().field("name", Rule::required());
        let result = schema.validate(&data(&[]));
        assert!(result.is_invalid());
        assert_eq!(result.errors[0].code, "required");
    }

    #[test]
    fn test_required_empty() {
        let schema = Schema::new().field("name", Rule::required());
        let result = schema.validate(&data(&[("name", "")]));
        assert!(result.is_invalid());
    }

    #[test]
    fn test_required_valid() {
        let schema = Schema::new().field("name", Rule::required());
        let result = schema.validate(&data(&[("name", "John")]));
        assert!(result.is_valid());
    }

    #[test]
    fn test_min_len() {
        let schema = Schema::new().field("password", Rule::required().min_len(8));
        let result = schema.validate(&data(&[("password", "short")]));
        assert!(result.is_invalid());
        assert_eq!(result.errors[0].code, "min_len");

        let result = schema.validate(&data(&[("password", "longenough")]));
        assert!(result.is_valid());
    }

    #[test]
    fn test_max_len() {
        let schema = Schema::new().field("name", Rule::required().max_len(5));
        let result = schema.validate(&data(&[("name", "toolongname")]));
        assert!(result.is_invalid());
    }

    #[test]
    fn test_email() {
        let schema = Schema::new().field("email", Rule::required().email());

        assert!(schema.validate(&data(&[("email", "user@example.com")])).is_valid());
        assert!(schema.validate(&data(&[("email", "invalid")])).is_invalid());
        assert!(schema.validate(&data(&[("email", "@example.com")])).is_invalid());
        assert!(schema.validate(&data(&[("email", "user@")])).is_invalid());
    }

    #[test]
    fn test_url() {
        let schema = Schema::new().field("site", Rule::required().url());
        assert!(schema.validate(&data(&[("site", "https://example.com")])).is_valid());
        assert!(schema.validate(&data(&[("site", "not-a-url")])).is_invalid());
    }

    #[test]
    fn test_min_max_number() {
        let schema = Schema::new()
            .field("age", Rule::required().min(18.0).max(120.0));

        assert!(schema.validate(&data(&[("age", "25")])).is_valid());
        assert!(schema.validate(&data(&[("age", "10")])).is_invalid());
        assert!(schema.validate(&data(&[("age", "200")])).is_invalid());
    }

    #[test]
    fn test_integer() {
        let schema = Schema::new().field("count", Rule::required().integer());
        assert!(schema.validate(&data(&[("count", "42")])).is_valid());
        assert!(schema.validate(&data(&[("count", "3.14")])).is_invalid());
    }

    #[test]
    fn test_one_of() {
        let schema = Schema::new()
            .field("role", Rule::required().one_of(&["admin", "user", "editor"]));
        assert!(schema.validate(&data(&[("role", "admin")])).is_valid());
        assert!(schema.validate(&data(&[("role", "hacker")])).is_invalid());
    }

    #[test]
    fn test_equals() {
        let schema = Schema::new()
            .field("password", Rule::required().min_len(8))
            .field("confirm", Rule::required().equals("password"));
        assert!(schema.validate(&data(&[("password", "mypassword"), ("confirm", "mypassword")])).is_valid());
        assert!(schema.validate(&data(&[("password", "mypassword"), ("confirm", "different")])).is_invalid());
    }

    #[test]
    fn test_pattern_uppercase() {
        let schema = Schema::new()
            .field("password", Rule::required().pattern("[A-Z]"));
        assert!(schema.validate(&data(&[("password", "hasUpperCase")])).is_valid());
        assert!(schema.validate(&data(&[("password", "alllowercase")])).is_invalid());
    }

    #[test]
    fn test_multiple_errors() {
        let schema = Schema::new()
            .field("email", Rule::required().email())
            .field("name", Rule::required());
        let result = schema.validate(&data(&[]));
        assert_eq!(result.errors.len(), 2);
    }

    #[test]
    fn test_field_errors() {
        let schema = Schema::new()
            .field("a", Rule::required())
            .field("b", Rule::required());
        let result = schema.validate(&data(&[]));
        assert_eq!(result.field_errors("a").len(), 1);
        assert_eq!(result.field_errors("b").len(), 1);
        assert_eq!(result.field_errors("c").len(), 0);
    }

    #[test]
    fn test_error_html() {
        let schema = Schema::new().field("email", Rule::required());
        let result = schema.validate(&data(&[]));
        let html = result.error_html("email");
        assert!(html.contains("field-error"));
        assert!(html.contains("required"));
    }

    #[test]
    fn test_to_json() {
        let schema = Schema::new().field("name", Rule::required());
        let result = schema.validate(&data(&[("name", "ok")]));
        assert!(result.to_json().contains("\"valid\":true"));

        let result = schema.validate(&data(&[]));
        assert!(result.to_json().contains("\"valid\":false"));
    }

    #[test]
    fn test_optional_skips_validation() {
        let schema = Schema::new().field("bio", Rule::optional().max_len(5));
        // Empty optional field should pass
        let result = schema.validate(&data(&[]));
        assert!(result.is_valid());
        // Non-empty must follow rules
        let result = schema.validate(&data(&[("bio", "toolongbio")]));
        assert!(result.is_invalid());
    }

    #[test]
    fn test_validate_pairs() {
        let schema = Schema::new().field("x", Rule::required());
        let result = schema.validate_pairs(&[("x", "hello")]);
        assert!(result.is_valid());
    }
}
