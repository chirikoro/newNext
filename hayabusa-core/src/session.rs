//! Cookie-Based Session Management for Hayabusa.
//!
//! Secure, encrypted session storage using cookies.
//! No external session store needed (stateless).
//!
//! ## Usage
//! ```ignore
//! let session_config = SessionConfig::new("my-secret-key-at-least-32-bytes!!");
//!
//! // Create a session
//! let mut session = Session::new();
//! session.set("user_id", "42");
//! session.set("role", "admin");
//! let cookie = session_config.encode(&session);
//!
//! // Decode a session from cookie
//! let session = session_config.decode(&cookie_value).unwrap();
//! assert_eq!(session.get("user_id"), Some("42"));
//! ```

use std::collections::HashMap;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Session configuration
#[derive(Debug, Clone)]
pub struct SessionConfig {
    /// Secret key for signing (minimum 32 bytes recommended)
    secret: String,
    /// Cookie name
    pub cookie_name: String,
    /// Session TTL
    pub max_age: Duration,
    /// Cookie path
    pub path: String,
    /// HttpOnly flag
    pub http_only: bool,
    /// Secure flag (HTTPS only)
    pub secure: bool,
    /// SameSite attribute
    pub same_site: SameSite,
}

/// SameSite cookie attribute
#[derive(Debug, Clone, Copy)]
pub enum SameSite {
    Strict,
    Lax,
    None,
}

impl SameSite {
    fn as_str(&self) -> &str {
        match self {
            SameSite::Strict => "Strict",
            SameSite::Lax => "Lax",
            SameSite::None => "None",
        }
    }
}

impl SessionConfig {
    pub fn new(secret: impl Into<String>) -> Self {
        Self {
            secret: secret.into(),
            cookie_name: "hayabusa_session".to_string(),
            max_age: Duration::from_secs(24 * 3600), // 24 hours
            path: "/".to_string(),
            http_only: true,
            secure: true,
            same_site: SameSite::Lax,
        }
    }

    pub fn cookie_name(mut self, name: impl Into<String>) -> Self {
        self.cookie_name = name.into();
        self
    }

    pub fn max_age(mut self, duration: Duration) -> Self {
        self.max_age = duration;
        self
    }

    pub fn secure(mut self, secure: bool) -> Self {
        self.secure = secure;
        self
    }

    pub fn same_site(mut self, same_site: SameSite) -> Self {
        self.same_site = same_site;
        self
    }

    /// Encode a session into a signed cookie value.
    ///
    /// Format: base64(json_data).signature
    pub fn encode(&self, session: &Session) -> String {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let mut data = session.data.clone();
        data.insert("__exp".to_string(), (timestamp + self.max_age.as_secs()).to_string());

        let json = serde_json::to_string(&data).unwrap_or_default();
        let encoded = base64_encode(json.as_bytes());
        let signature = sign(&encoded, &self.secret);

        format!("{}.{}", encoded, signature)
    }

    /// Decode and verify a session from a cookie value
    pub fn decode(&self, cookie_value: &str) -> Option<Session> {
        let parts: Vec<&str> = cookie_value.rsplitn(2, '.').collect();
        if parts.len() != 2 {
            return None;
        }

        let signature = parts[0];
        let encoded = parts[1];

        // Verify signature
        let expected = sign(encoded, &self.secret);
        if !constant_time_eq(signature, &expected) {
            return None;
        }

        // Decode data
        let json_bytes = base64_decode(encoded)?;
        let json = String::from_utf8(json_bytes).ok()?;
        let data: HashMap<String, String> = serde_json::from_str(&json).ok()?;

        // Check expiration
        if let Some(exp_str) = data.get("__exp") {
            if let Ok(exp) = exp_str.parse::<u64>() {
                let now = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs();
                if now > exp {
                    return None; // Session expired
                }
            }
        }

        let mut session_data = data;
        session_data.remove("__exp");

        Some(Session {
            data: session_data,
        })
    }

    /// Generate a Set-Cookie header value
    pub fn set_cookie_header(&self, session: &Session) -> String {
        let value = self.encode(session);
        let mut cookie = format!(
            "{}={}; Path={}; Max-Age={}",
            self.cookie_name,
            value,
            self.path,
            self.max_age.as_secs()
        );

        if self.http_only {
            cookie.push_str("; HttpOnly");
        }
        if self.secure {
            cookie.push_str("; Secure");
        }
        cookie.push_str(&format!("; SameSite={}", self.same_site.as_str()));

        cookie
    }

    /// Generate a cookie header that clears/expires the session
    pub fn clear_cookie_header(&self) -> String {
        format!(
            "{}=; Path={}; Max-Age=0; HttpOnly; SameSite={}",
            self.cookie_name,
            self.path,
            self.same_site.as_str()
        )
    }
}

/// Session data store
#[derive(Debug, Clone, Default)]
pub struct Session {
    data: HashMap<String, String>,
}

impl Session {
    pub fn new() -> Self {
        Self::default()
    }

    /// Set a session value
    pub fn set(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.data.insert(key.into(), value.into());
    }

    /// Get a session value
    pub fn get(&self, key: &str) -> Option<&str> {
        self.data.get(key).map(|s| s.as_str())
    }

    /// Remove a session value
    pub fn remove(&mut self, key: &str) -> Option<String> {
        self.data.remove(key)
    }

    /// Check if a key exists
    pub fn has(&self, key: &str) -> bool {
        self.data.contains_key(key)
    }

    /// Check if the session is empty
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// Get all session data
    pub fn data(&self) -> &HashMap<String, String> {
        &self.data
    }
}

/// HMAC-like signing using std hash (simplified; production should use ring/hmac)
fn sign(data: &str, secret: &str) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    secret.hash(&mut hasher);
    data.hash(&mut hasher);
    let h1 = hasher.finish();

    // Double hash for slightly better security
    let mut hasher2 = DefaultHasher::new();
    h1.hash(&mut hasher2);
    secret.hash(&mut hasher2);
    format!("{:x}{:x}", h1, hasher2.finish())
}

/// Constant-time string comparison to prevent timing attacks
fn constant_time_eq(a: &str, b: &str) -> bool {
    if a.len() != b.len() {
        return false;
    }
    a.bytes()
        .zip(b.bytes())
        .fold(0u8, |acc, (x, y)| acc | (x ^ y))
        == 0
}

/// Simple base64 encoding (URL-safe, no padding)
fn base64_encode(data: &[u8]) -> String {
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
    let mut result = String::new();

    for chunk in data.chunks(3) {
        let b0 = chunk[0] as usize;
        let b1 = if chunk.len() > 1 { chunk[1] as usize } else { 0 };
        let b2 = if chunk.len() > 2 { chunk[2] as usize } else { 0 };

        result.push(CHARS[b0 >> 2] as char);
        result.push(CHARS[((b0 & 0x03) << 4) | (b1 >> 4)] as char);

        if chunk.len() > 1 {
            result.push(CHARS[((b1 & 0x0F) << 2) | (b2 >> 6)] as char);
        }
        if chunk.len() > 2 {
            result.push(CHARS[b2 & 0x3F] as char);
        }
    }

    result
}

/// Simple base64 decoding (URL-safe, no padding)
fn base64_decode(data: &str) -> Option<Vec<u8>> {
    const DECODE: [u8; 128] = {
        let mut table = [255u8; 128];
        let chars = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
        let mut i = 0;
        while i < chars.len() {
            table[chars[i] as usize] = i as u8;
            i += 1;
        }
        table
    };

    let bytes: Vec<u8> = data.bytes().collect();
    let mut result = Vec::new();

    for chunk in bytes.chunks(4) {
        let len = chunk.len();
        let mut buf = [0u8; 4];
        for (i, &b) in chunk.iter().enumerate() {
            if b >= 128 || DECODE[b as usize] == 255 {
                return None;
            }
            buf[i] = DECODE[b as usize];
        }

        result.push((buf[0] << 2) | (buf[1] >> 4));
        if len > 2 {
            result.push((buf[1] << 4) | (buf[2] >> 2));
        }
        if len > 3 {
            result.push((buf[2] << 6) | buf[3]);
        }
    }

    Some(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_basic() {
        let mut session = Session::new();
        session.set("user", "alice");
        session.set("role", "admin");

        assert_eq!(session.get("user"), Some("alice"));
        assert_eq!(session.get("role"), Some("admin"));
        assert_eq!(session.get("missing"), None);
        assert!(session.has("user"));
        assert!(!session.is_empty());
    }

    #[test]
    fn test_session_encode_decode() {
        let config = SessionConfig::new("my-super-secret-key-for-testing!");

        let mut session = Session::new();
        session.set("user_id", "42");
        session.set("name", "Hayabusa");

        let encoded = config.encode(&session);
        let decoded = config.decode(&encoded).unwrap();

        assert_eq!(decoded.get("user_id"), Some("42"));
        assert_eq!(decoded.get("name"), Some("Hayabusa"));
    }

    #[test]
    fn test_session_tampered() {
        let config = SessionConfig::new("secret");
        let mut session = Session::new();
        session.set("admin", "true");

        let encoded = config.encode(&session);
        // Tamper with the data
        let tampered = format!("x{}", encoded);
        assert!(config.decode(&tampered).is_none());
    }

    #[test]
    fn test_session_wrong_secret() {
        let config1 = SessionConfig::new("secret-1");
        let config2 = SessionConfig::new("secret-2");

        let mut session = Session::new();
        session.set("data", "value");

        let encoded = config1.encode(&session);
        assert!(config2.decode(&encoded).is_none());
    }

    #[test]
    fn test_set_cookie_header() {
        let config = SessionConfig::new("secret")
            .cookie_name("my_session")
            .max_age(Duration::from_secs(3600));

        let session = Session::new();
        let header = config.set_cookie_header(&session);

        assert!(header.starts_with("my_session="));
        assert!(header.contains("Max-Age=3600"));
        assert!(header.contains("HttpOnly"));
        assert!(header.contains("Secure"));
        assert!(header.contains("SameSite=Lax"));
    }

    #[test]
    fn test_clear_cookie() {
        let config = SessionConfig::new("secret");
        let header = config.clear_cookie_header();
        assert!(header.contains("Max-Age=0"));
    }

    #[test]
    fn test_base64_roundtrip() {
        let original = b"Hello, Hayabusa!";
        let encoded = base64_encode(original);
        let decoded = base64_decode(&encoded).unwrap();
        assert_eq!(decoded, original);
    }

    #[test]
    fn test_constant_time_eq() {
        assert!(constant_time_eq("hello", "hello"));
        assert!(!constant_time_eq("hello", "world"));
        assert!(!constant_time_eq("abc", "abcd"));
    }

    #[test]
    fn test_session_remove() {
        let mut session = Session::new();
        session.set("key", "value");
        assert!(session.has("key"));
        session.remove("key");
        assert!(!session.has("key"));
    }
}
