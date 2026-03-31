//! Environment Configuration for Hayabusa.
//!
//! Load `.env` files and provide typed configuration access.
//!
//! ```ignore
//! use hayabusa_core::prelude::*;
//!
//! let config = EnvConfig::load(".env").unwrap();
//! let port: u16 = config.get_or("PORT", 3000);
//! ```

use std::collections::HashMap;

// ─── .env Parser ───────────────────────────────────────────

/// Parse a `.env` file content into key-value pairs
pub fn parse_dotenv(content: &str) -> HashMap<String, String> {
    let mut map = HashMap::new();
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if let Some(eq_pos) = trimmed.find('=') {
            let key = trimmed[..eq_pos].trim().to_string();
            let mut value = trimmed[eq_pos + 1..].trim().to_string();
            // Strip surrounding quotes
            if (value.starts_with('"') && value.ends_with('"'))
                || (value.starts_with('\'') && value.ends_with('\''))
            {
                value = value[1..value.len() - 1].to_string();
            }
            // Handle escape sequences in double-quoted values
            if trimmed[eq_pos + 1..].trim().starts_with('"') {
                value = value.replace("\\n", "\n").replace("\\t", "\t");
            }
            if !key.is_empty() {
                map.insert(key, value);
            }
        }
    }
    map
}

// ─── EnvConfig ─────────────────────────────────────────────

/// Configuration loaded from environment variables and `.env` files
#[derive(Debug, Clone)]
pub struct EnvConfig {
    values: HashMap<String, String>,
    env_name: String,
}

impl EnvConfig {
    /// Create a new empty config
    pub fn new() -> Self {
        Self {
            values: HashMap::new(),
            env_name: "development".to_string(),
        }
    }

    /// Load from a `.env` file content string
    pub fn from_str(content: &str) -> Self {
        Self {
            values: parse_dotenv(content),
            env_name: "development".to_string(),
        }
    }

    /// Load from environment variables (std::env::vars)
    pub fn from_env() -> Self {
        Self {
            values: std::env::vars().collect(),
            env_name: std::env::var("HAYABUSA_ENV")
                .or_else(|_| std::env::var("NODE_ENV"))
                .or_else(|_| std::env::var("RUST_ENV"))
                .unwrap_or_else(|_| "development".to_string()),
        }
    }

    /// Merge another config (other values override self)
    pub fn merge(mut self, other: &EnvConfig) -> Self {
        for (k, v) in &other.values {
            self.values.insert(k.clone(), v.clone());
        }
        self
    }

    /// Set the environment name
    pub fn env_name(mut self, name: &str) -> Self {
        self.env_name = name.to_string();
        self
    }

    /// Get a string value
    pub fn get(&self, key: &str) -> Option<&str> {
        self.values.get(key).map(|s| s.as_str())
    }

    /// Get a value or return a default
    pub fn get_or<'a>(&'a self, key: &str, default: &'a str) -> &'a str {
        self.values.get(key).map(|s| s.as_str()).unwrap_or(default)
    }

    /// Get a value, panic if missing
    pub fn require(&self, key: &str) -> &str {
        self.values
            .get(key)
            .unwrap_or_else(|| panic!("Required env var '{}' is not set", key))
    }

    /// Get a value parsed as the target type
    pub fn get_parsed<T: std::str::FromStr>(&self, key: &str) -> Option<T> {
        self.values.get(key).and_then(|v| v.parse().ok())
    }

    /// Get a parsed value with default
    pub fn get_or_parsed<T: std::str::FromStr>(&self, key: &str, default: T) -> T {
        self.get_parsed(key).unwrap_or(default)
    }

    /// Get a boolean value (true, 1, yes, on → true)
    pub fn get_bool(&self, key: &str) -> Option<bool> {
        self.values.get(key).map(|v| {
            matches!(v.to_lowercase().as_str(), "true" | "1" | "yes" | "on")
        })
    }

    /// Get the current environment name
    pub fn current_env(&self) -> &str {
        &self.env_name
    }

    /// Check if running in development
    pub fn is_dev(&self) -> bool {
        matches!(self.env_name.as_str(), "development" | "dev")
    }

    /// Check if running in production
    pub fn is_prod(&self) -> bool {
        matches!(self.env_name.as_str(), "production" | "prod")
    }

    /// Check if running in test
    pub fn is_test(&self) -> bool {
        matches!(self.env_name.as_str(), "test" | "testing")
    }

    /// Set a value
    pub fn set(&mut self, key: &str, value: &str) {
        self.values.insert(key.to_string(), value.to_string());
    }

    /// Check if a key exists
    pub fn has(&self, key: &str) -> bool {
        self.values.contains_key(key)
    }

    /// Get all keys
    pub fn keys(&self) -> Vec<&str> {
        self.values.keys().map(|k| k.as_str()).collect()
    }

    /// Get the number of entries
    pub fn len(&self) -> usize {
        self.values.len()
    }

    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }

    /// Convert to JSON string
    pub fn to_json(&self) -> String {
        let entries: Vec<String> = self.values.iter().map(|(k, v)| {
            format!("\"{}\":\"{}\"", k, v.replace('\\', "\\\\").replace('"', "\\\""))
        }).collect();
        format!("{{{}}}", entries.join(","))
    }
}

impl Default for EnvConfig {
    fn default() -> Self {
        Self::new()
    }
}

// ─── Typed Config Struct Builder ───────────────────────────

/// Define a typed configuration section
#[derive(Debug, Clone)]
pub struct TypedConfig {
    pub prefix: String,
    pub fields: Vec<ConfigField>,
}

#[derive(Debug, Clone)]
pub struct ConfigField {
    pub name: String,
    pub env_key: String,
    pub default: Option<String>,
    pub required: bool,
    pub description: String,
}

impl TypedConfig {
    pub fn new(prefix: &str) -> Self {
        Self {
            prefix: prefix.to_string(),
            fields: Vec::new(),
        }
    }

    pub fn field(mut self, name: &str, required: bool, default: Option<&str>, desc: &str) -> Self {
        let env_key = if self.prefix.is_empty() {
            name.to_uppercase()
        } else {
            format!("{}_{}", self.prefix.to_uppercase(), name.to_uppercase())
        };
        self.fields.push(ConfigField {
            name: name.to_string(),
            env_key,
            default: default.map(Into::into),
            required,
            description: desc.to_string(),
        });
        self
    }

    /// Validate that all required fields are present in the config
    pub fn validate(&self, config: &EnvConfig) -> Result<(), Vec<String>> {
        let missing: Vec<String> = self.fields.iter()
            .filter(|f| f.required && !config.has(&f.env_key) && f.default.is_none())
            .map(|f| format!("{} ({})", f.env_key, f.description))
            .collect();
        if missing.is_empty() {
            Ok(())
        } else {
            Err(missing)
        }
    }

    /// Get a field value from config, falling back to default
    pub fn get_field<'a>(&'a self, config: &'a EnvConfig, name: &str) -> Option<&'a str> {
        let field = self.fields.iter().find(|f| f.name == name)?;
        config.get(&field.env_key).or(field.default.as_deref())
    }

    /// Generate a `.env.example` file content
    pub fn to_env_example(&self) -> String {
        let mut out = String::new();
        for f in &self.fields {
            out.push_str(&format!("# {} {}\n", f.description, if f.required { "(required)" } else { "(optional)" }));
            if let Some(ref default) = f.default {
                out.push_str(&format!("# {}={}\n\n", f.env_key, default));
            } else {
                out.push_str(&format!("{}=\n\n", f.env_key));
            }
        }
        out
    }
}

// ─── Tests ──────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_dotenv() {
        let content = r#"
# Comment
DATABASE_URL=postgres://localhost/mydb
PORT=3000
SECRET_KEY="my_secret"
SINGLE='quoted'
"#;
        let map = parse_dotenv(content);
        assert_eq!(map.get("DATABASE_URL").unwrap(), "postgres://localhost/mydb");
        assert_eq!(map.get("PORT").unwrap(), "3000");
        assert_eq!(map.get("SECRET_KEY").unwrap(), "my_secret");
        assert_eq!(map.get("SINGLE").unwrap(), "quoted");
    }

    #[test]
    fn test_parse_dotenv_escape() {
        let content = "MSG=\"hello\\nworld\"";
        let map = parse_dotenv(content);
        assert_eq!(map.get("MSG").unwrap(), "hello\nworld");
    }

    #[test]
    fn test_parse_dotenv_comments_blank() {
        let content = "# comment\n\nKEY=value\n# another comment";
        let map = parse_dotenv(content);
        assert_eq!(map.len(), 1);
        assert_eq!(map.get("KEY").unwrap(), "value");
    }

    #[test]
    fn test_env_config_basic() {
        let config = EnvConfig::from_str("PORT=8080\nDEBUG=true");
        assert_eq!(config.get("PORT"), Some("8080"));
        assert_eq!(config.get("MISSING"), None);
    }

    #[test]
    fn test_env_config_get_or() {
        let config = EnvConfig::from_str("A=1");
        assert_eq!(config.get_or("A", "default"), "1");
        assert_eq!(config.get_or("B", "default"), "default");
    }

    #[test]
    fn test_env_config_parsed() {
        let config = EnvConfig::from_str("PORT=3000\nBAD=abc");
        assert_eq!(config.get_parsed::<u16>("PORT"), Some(3000));
        assert_eq!(config.get_parsed::<u16>("BAD"), None);
        assert_eq!(config.get_or_parsed::<u16>("MISSING", 8080), 8080);
    }

    #[test]
    fn test_env_config_bool() {
        let config = EnvConfig::from_str("A=true\nB=1\nC=yes\nD=on\nE=false\nF=0");
        assert_eq!(config.get_bool("A"), Some(true));
        assert_eq!(config.get_bool("B"), Some(true));
        assert_eq!(config.get_bool("C"), Some(true));
        assert_eq!(config.get_bool("D"), Some(true));
        assert_eq!(config.get_bool("E"), Some(false));
        assert_eq!(config.get_bool("F"), Some(false));
    }

    #[test]
    fn test_env_config_environment() {
        let config = EnvConfig::new().env_name("production");
        assert!(config.is_prod());
        assert!(!config.is_dev());
        assert!(!config.is_test());
    }

    #[test]
    fn test_env_config_merge() {
        let base = EnvConfig::from_str("A=1\nB=2");
        let overlay = EnvConfig::from_str("B=3\nC=4");
        let merged = base.merge(&overlay);
        assert_eq!(merged.get("A"), Some("1"));
        assert_eq!(merged.get("B"), Some("3"));
        assert_eq!(merged.get("C"), Some("4"));
    }

    #[test]
    fn test_env_config_set_has() {
        let mut config = EnvConfig::new();
        assert!(!config.has("X"));
        config.set("X", "42");
        assert!(config.has("X"));
        assert_eq!(config.get("X"), Some("42"));
    }

    #[test]
    fn test_env_config_len() {
        let config = EnvConfig::from_str("A=1\nB=2\nC=3");
        assert_eq!(config.len(), 3);
        assert!(!config.is_empty());
        assert!(EnvConfig::new().is_empty());
    }

    #[test]
    fn test_env_config_to_json() {
        let config = EnvConfig::from_str("KEY=value");
        let json = config.to_json();
        assert!(json.contains("\"KEY\":\"value\""));
    }

    #[test]
    fn test_typed_config() {
        let tc = TypedConfig::new("DB")
            .field("host", true, None, "Database host")
            .field("port", false, Some("5432"), "Database port");
        assert_eq!(tc.fields[0].env_key, "DB_HOST");
        assert_eq!(tc.fields[1].env_key, "DB_PORT");
    }

    #[test]
    fn test_typed_config_validate() {
        let tc = TypedConfig::new("DB")
            .field("host", true, None, "Database host")
            .field("port", false, Some("5432"), "Database port");
        let config = EnvConfig::from_str("DB_HOST=localhost");
        assert!(tc.validate(&config).is_ok());
        let empty = EnvConfig::new();
        assert!(tc.validate(&empty).is_err());
    }

    #[test]
    fn test_typed_config_get_field() {
        let tc = TypedConfig::new("APP")
            .field("port", false, Some("3000"), "Server port");
        let config = EnvConfig::new();
        assert_eq!(tc.get_field(&config, "port"), Some("3000"));
        let config2 = EnvConfig::from_str("APP_PORT=8080");
        assert_eq!(tc.get_field(&config2, "port"), Some("8080"));
    }

    #[test]
    fn test_typed_config_env_example() {
        let tc = TypedConfig::new("DB")
            .field("url", true, None, "Database URL");
        let example = tc.to_env_example();
        assert!(example.contains("DB_URL="));
        assert!(example.contains("(required)"));
    }

    #[test]
    fn test_require_panics() {
        let config = EnvConfig::from_str("KEY=val");
        assert_eq!(config.require("KEY"), "val");
    }

    #[test]
    #[should_panic(expected = "Required env var 'MISSING' is not set")]
    fn test_require_missing_panics() {
        let config = EnvConfig::new();
        config.require("MISSING");
    }
}
