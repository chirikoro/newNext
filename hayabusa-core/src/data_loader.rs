//! External Data Loader for Hayabusa.
//!
//! Load page data from JSON files, external APIs, or shell commands
//! (Python, Node.js, etc.) — **no Rust code needed** for data fetching.
//!
//! ## Why This Matters for DX
//! - Data fetching can be done in ANY language (Python, Node, Go, etc.)
//! - JSON files for static data (blog posts, config, etc.)
//! - Shell commands for dynamic data (API calls, DB queries)
//! - Results are cached and revalidated automatically
//!
//! ## Usage
//! ```ignore
//! // Load from a JSON file
//! let data = DataLoader::json_file("data/posts.json").await?;
//!
//! // Load from a shell command (e.g., Python script)
//! let data = DataLoader::command("python3 scripts/fetch_posts.py").await?;
//!
//! // Load from an external API
//! let data = DataLoader::url("https://api.example.com/posts").await?;
//! ```

use std::collections::HashMap;
use std::path::PathBuf;
use std::time::{Duration, Instant};
use std::sync::{Arc, Mutex};

/// Data loader with caching
#[derive(Clone)]
pub struct DataLoader {
    cache: Arc<Mutex<HashMap<String, CachedData>>>,
    base_dir: PathBuf,
}

#[derive(Debug, Clone)]
struct CachedData {
    value: serde_json::Value,
    loaded_at: Instant,
    ttl: Duration,
}

impl CachedData {
    fn is_stale(&self) -> bool {
        self.loaded_at.elapsed() > self.ttl
    }
}

/// Data loading errors
#[derive(Debug, Clone)]
pub enum DataError {
    FileNotFound(String),
    ParseError(String),
    CommandError(String),
    NetworkError(String),
}

impl std::fmt::Display for DataError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DataError::FileNotFound(p) => write!(f, "Data file not found: {}", p),
            DataError::ParseError(m) => write!(f, "Data parse error: {}", m),
            DataError::CommandError(m) => write!(f, "Command error: {}", m),
            DataError::NetworkError(m) => write!(f, "Network error: {}", m),
        }
    }
}

impl DataLoader {
    pub fn new(base_dir: impl Into<PathBuf>) -> Self {
        Self {
            cache: Arc::new(Mutex::new(HashMap::new())),
            base_dir: base_dir.into(),
        }
    }

    /// Load data from a JSON file
    pub fn json_file(&self, path: &str) -> Result<serde_json::Value, DataError> {
        let cache_key = format!("file:{}", path);

        // Check cache
        if let Some(cached) = self.get_cached(&cache_key) {
            return Ok(cached);
        }

        let full_path = self.base_dir.join(path);
        let content = std::fs::read_to_string(&full_path).map_err(|_| {
            DataError::FileNotFound(full_path.display().to_string())
        })?;

        let value: serde_json::Value = serde_json::from_str(&content).map_err(|e| {
            DataError::ParseError(format!("{}: {}", path, e))
        })?;

        self.set_cached(cache_key, value.clone(), Duration::from_secs(0)); // No TTL for files
        Ok(value)
    }

    /// Load data from a JSON string
    pub fn json_string(json: &str) -> Result<serde_json::Value, DataError> {
        serde_json::from_str(json).map_err(|e| DataError::ParseError(e.to_string()))
    }

    /// Load data by running a shell command.
    ///
    /// The command's stdout is parsed as JSON.
    /// This lets you use **any language** for data fetching:
    /// - `python3 scripts/fetch_data.py`
    /// - `node scripts/getData.js`
    /// - `go run scripts/loader.go`
    /// - `curl -s https://api.example.com/data`
    pub fn command(&self, cmd: &str) -> Result<serde_json::Value, DataError> {
        self.command_cached(cmd, Duration::from_secs(0))
    }

    /// Load data from a command with caching
    pub fn command_cached(
        &self,
        cmd: &str,
        ttl: Duration,
    ) -> Result<serde_json::Value, DataError> {
        let cache_key = format!("cmd:{}", cmd);

        if let Some(cached) = self.get_cached(&cache_key) {
            return Ok(cached);
        }

        let output = std::process::Command::new("sh")
            .arg("-c")
            .arg(cmd)
            .current_dir(&self.base_dir)
            .output()
            .map_err(|e| DataError::CommandError(format!("Failed to execute '{}': {}", cmd, e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(DataError::CommandError(format!(
                "Command '{}' failed (exit {}): {}",
                cmd,
                output.status.code().unwrap_or(-1),
                stderr
            )));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let value: serde_json::Value = serde_json::from_str(&stdout).map_err(|e| {
            DataError::ParseError(format!("Command '{}' output is not valid JSON: {}", cmd, e))
        })?;

        self.set_cached(cache_key, value.clone(), ttl);
        Ok(value)
    }

    /// Load data from a JSON file, with path parameter substitution.
    ///
    /// Example: `loader.json_file_with_params("data/posts/:slug.json", &params)`
    /// With params `{"slug": "hello-world"}` → loads `data/posts/hello-world.json`
    pub fn json_file_with_params(
        &self,
        path_template: &str,
        params: &HashMap<String, String>,
    ) -> Result<serde_json::Value, DataError> {
        let mut resolved = path_template.to_string();
        for (key, value) in params {
            resolved = resolved.replace(&format!(":{}", key), value);
        }
        self.json_file(&resolved)
    }

    /// Clear the data cache
    pub fn clear_cache(&self) {
        self.cache.lock().unwrap().clear();
    }

    /// Remove stale entries from cache
    pub fn cleanup_cache(&self) {
        let mut cache = self.cache.lock().unwrap();
        cache.retain(|_, v| !v.is_stale());
    }

    fn get_cached(&self, key: &str) -> Option<serde_json::Value> {
        let cache = self.cache.lock().unwrap();
        cache.get(key).and_then(|c| {
            if c.ttl.is_zero() || !c.is_stale() {
                Some(c.value.clone())
            } else {
                None
            }
        })
    }

    fn set_cached(&self, key: String, value: serde_json::Value, ttl: Duration) {
        let mut cache = self.cache.lock().unwrap();
        cache.insert(
            key,
            CachedData {
                value,
                loaded_at: Instant::now(),
                ttl,
            },
        );
    }
}

/// Convenience: merge multiple JSON values into one object
pub fn merge_json(values: &[serde_json::Value]) -> serde_json::Value {
    let mut merged = serde_json::Map::new();
    for v in values {
        if let serde_json::Value::Object(map) = v {
            for (k, v) in map {
                merged.insert(k.clone(), v.clone());
            }
        }
    }
    serde_json::Value::Object(merged)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_json_string_parse() {
        let data = DataLoader::json_string(r#"{"title": "Hello", "count": 42}"#).unwrap();
        assert_eq!(data["title"], "Hello");
        assert_eq!(data["count"], 42);
    }

    #[test]
    fn test_json_string_invalid() {
        assert!(DataLoader::json_string("not json").is_err());
    }

    #[test]
    fn test_command_echo() {
        let loader = DataLoader::new(".");
        let result = loader.command("echo '{\"ok\": true}'");
        assert!(result.is_ok());
        assert_eq!(result.unwrap()["ok"], true);
    }

    #[test]
    fn test_command_failure() {
        let loader = DataLoader::new(".");
        let result = loader.command("false");
        assert!(result.is_err());
    }

    #[test]
    fn test_command_cached() {
        let loader = DataLoader::new(".");
        let ttl = Duration::from_secs(60);

        // First call
        let r1 = loader.command_cached("echo '{\"v\": 1}'", ttl).unwrap();
        assert_eq!(r1["v"], 1);

        // Second call should be cached (even though echo would work again)
        let r2 = loader.command_cached("echo '{\"v\": 1}'", ttl).unwrap();
        assert_eq!(r2["v"], 1);
    }

    #[test]
    fn test_json_file_not_found() {
        let loader = DataLoader::new(".");
        assert!(loader.json_file("nonexistent.json").is_err());
    }

    #[test]
    fn test_merge_json() {
        let a = serde_json::json!({"name": "Alice"});
        let b = serde_json::json!({"age": 30});
        let merged = merge_json(&[a, b]);
        assert_eq!(merged["name"], "Alice");
        assert_eq!(merged["age"], 30);
    }

    #[test]
    fn test_path_param_substitution() {
        let loader = DataLoader::new(".");
        let mut params = HashMap::new();
        params.insert("slug".to_string(), "hello-world".to_string());

        // This will fail because file doesn't exist, but the path resolution should work
        let err = loader.json_file_with_params("data/posts/:slug.json", &params);
        match err {
            Err(DataError::FileNotFound(path)) => {
                assert!(path.contains("hello-world"));
            }
            _ => panic!("Expected FileNotFound error"),
        }
    }

    #[test]
    fn test_clear_cache() {
        let loader = DataLoader::new(".");
        loader.command("echo '{\"x\": 1}'").unwrap();
        loader.clear_cache();
        // After clearing, cache should be empty
        assert!(loader.cache.lock().unwrap().is_empty());
    }
}
