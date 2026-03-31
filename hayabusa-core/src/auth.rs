//! Built-in Authentication for Hayabusa.
//!
//! Provides JWT-based authentication, OAuth2 flows, password hashing,
//! session management, and middleware guards — similar to NextAuth.js.
//!
//! ## Usage
//! ```ignore
//! use hayabusa_core::prelude::*;
//!
//! let auth = AuthConfig::new("my-secret-key")
//!     .add_provider(AuthProvider::github("client_id", "client_secret"))
//!     .add_provider(AuthProvider::google("client_id", "client_secret"))
//!     .jwt_expiry(3600);
//! ```

use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

// ─── JWT ────────────────────────────────────────────────────

/// A minimal JWT implementation (HS256)
#[derive(Debug, Clone)]
pub struct Jwt {
    secret: String,
}

/// JWT Claims
#[derive(Debug, Clone)]
pub struct Claims {
    pub sub: String,
    pub exp: u64,
    pub iat: u64,
    pub email: Option<String>,
    pub name: Option<String>,
    pub role: Option<String>,
    pub custom: HashMap<String, String>,
}

impl Claims {
    pub fn new(sub: impl Into<String>, expiry_secs: u64) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        Self {
            sub: sub.into(),
            exp: now + expiry_secs,
            iat: now,
            email: None,
            name: None,
            role: None,
            custom: HashMap::new(),
        }
    }

    pub fn email(mut self, email: impl Into<String>) -> Self {
        self.email = Some(email.into());
        self
    }

    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    pub fn role(mut self, role: impl Into<String>) -> Self {
        self.role = Some(role.into());
        self
    }

    pub fn custom(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.custom.insert(key.into(), value.into());
        self
    }

    pub fn is_expired(&self) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        now > self.exp
    }

    fn to_json(&self) -> String {
        let mut json = format!(
            "{{\"sub\":\"{}\",\"exp\":{},\"iat\":{}",
            self.sub, self.exp, self.iat
        );
        if let Some(ref email) = self.email {
            json.push_str(&format!(",\"email\":\"{}\"", email));
        }
        if let Some(ref name) = self.name {
            json.push_str(&format!(",\"name\":\"{}\"", name));
        }
        if let Some(ref role) = self.role {
            json.push_str(&format!(",\"role\":\"{}\"", role));
        }
        for (k, v) in &self.custom {
            json.push_str(&format!(",\"{}\":\"{}\"", k, v));
        }
        json.push('}');
        json
    }

    fn from_json(json: &str) -> Option<Self> {
        let sub = extract_json_string(json, "sub")?;
        let exp = extract_json_number(json, "exp")?;
        let iat = extract_json_number(json, "iat")?;
        let email = extract_json_string(json, "email");
        let name = extract_json_string(json, "name");
        let role = extract_json_string(json, "role");
        Some(Self {
            sub,
            exp,
            iat,
            email,
            name,
            role,
            custom: HashMap::new(),
        })
    }
}

impl Jwt {
    pub fn new(secret: impl Into<String>) -> Self {
        Self {
            secret: secret.into(),
        }
    }

    /// Sign claims and produce a JWT token string
    pub fn sign(&self, claims: &Claims) -> String {
        let header = base64url_encode(b"{\"alg\":\"HS256\",\"typ\":\"JWT\"}");
        let payload = base64url_encode(claims.to_json().as_bytes());
        let message = format!("{}.{}", header, payload);
        let signature = hmac_sha256(self.secret.as_bytes(), message.as_bytes());
        let sig_b64 = base64url_encode(&signature);
        format!("{}.{}", message, sig_b64)
    }

    /// Verify and decode a JWT token
    pub fn verify(&self, token: &str) -> Result<Claims, AuthError> {
        let parts: Vec<&str> = token.split('.').collect();
        if parts.len() != 3 {
            return Err(AuthError::InvalidToken("malformed JWT".into()));
        }

        let message = format!("{}.{}", parts[0], parts[1]);
        let expected_sig = hmac_sha256(self.secret.as_bytes(), message.as_bytes());
        let expected_b64 = base64url_encode(&expected_sig);

        if !constant_time_eq(parts[2].as_bytes(), expected_b64.as_bytes()) {
            return Err(AuthError::InvalidToken("signature mismatch".into()));
        }

        let payload_bytes = base64url_decode(parts[1])
            .ok_or_else(|| AuthError::InvalidToken("invalid base64".into()))?;
        let payload_str = String::from_utf8(payload_bytes)
            .map_err(|_| AuthError::InvalidToken("invalid utf8".into()))?;

        let claims = Claims::from_json(&payload_str)
            .ok_or_else(|| AuthError::InvalidToken("invalid claims JSON".into()))?;

        if claims.is_expired() {
            return Err(AuthError::TokenExpired);
        }

        Ok(claims)
    }
}

// ─── OAuth2 Provider ────────────────────────────────────────

/// OAuth2 provider configuration
#[derive(Debug, Clone)]
pub struct AuthProvider {
    pub name: String,
    pub client_id: String,
    pub client_secret: String,
    pub authorize_url: String,
    pub token_url: String,
    pub userinfo_url: String,
    pub scopes: Vec<String>,
}

impl AuthProvider {
    /// GitHub OAuth
    pub fn github(client_id: impl Into<String>, client_secret: impl Into<String>) -> Self {
        Self {
            name: "github".to_string(),
            client_id: client_id.into(),
            client_secret: client_secret.into(),
            authorize_url: "https://github.com/login/oauth/authorize".to_string(),
            token_url: "https://github.com/login/oauth/access_token".to_string(),
            userinfo_url: "https://api.github.com/user".to_string(),
            scopes: vec!["read:user".to_string(), "user:email".to_string()],
        }
    }

    /// Google OAuth
    pub fn google(client_id: impl Into<String>, client_secret: impl Into<String>) -> Self {
        Self {
            name: "google".to_string(),
            client_id: client_id.into(),
            client_secret: client_secret.into(),
            authorize_url: "https://accounts.google.com/o/oauth2/v2/auth".to_string(),
            token_url: "https://oauth2.googleapis.com/token".to_string(),
            userinfo_url: "https://www.googleapis.com/oauth2/v3/userinfo".to_string(),
            scopes: vec!["openid".to_string(), "email".to_string(), "profile".to_string()],
        }
    }

    /// Discord OAuth
    pub fn discord(client_id: impl Into<String>, client_secret: impl Into<String>) -> Self {
        Self {
            name: "discord".to_string(),
            client_id: client_id.into(),
            client_secret: client_secret.into(),
            authorize_url: "https://discord.com/api/oauth2/authorize".to_string(),
            token_url: "https://discord.com/api/oauth2/token".to_string(),
            userinfo_url: "https://discord.com/api/users/@me".to_string(),
            scopes: vec!["identify".to_string(), "email".to_string()],
        }
    }

    /// Custom OAuth2 provider
    pub fn custom(
        name: impl Into<String>,
        client_id: impl Into<String>,
        client_secret: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            client_id: client_id.into(),
            client_secret: client_secret.into(),
            authorize_url: String::new(),
            token_url: String::new(),
            userinfo_url: String::new(),
            scopes: Vec::new(),
        }
    }

    pub fn authorize_url(mut self, url: impl Into<String>) -> Self {
        self.authorize_url = url.into();
        self
    }

    pub fn token_url(mut self, url: impl Into<String>) -> Self {
        self.token_url = url.into();
        self
    }

    pub fn userinfo_url(mut self, url: impl Into<String>) -> Self {
        self.userinfo_url = url.into();
        self
    }

    pub fn scope(mut self, scope: impl Into<String>) -> Self {
        self.scopes.push(scope.into());
        self
    }

    /// Generate the OAuth2 authorization URL
    pub fn get_authorize_url(&self, redirect_uri: &str, state: &str) -> String {
        let scopes = self.scopes.join(" ");
        format!(
            "{}?client_id={}&redirect_uri={}&scope={}&state={}&response_type=code",
            self.authorize_url,
            url_encode(&self.client_id),
            url_encode(redirect_uri),
            url_encode(&scopes),
            url_encode(state),
        )
    }
}

// ─── Auth Config ────────────────────────────────────────────

/// Main authentication configuration
#[derive(Debug, Clone)]
pub struct AuthConfig {
    pub secret: String,
    pub providers: Vec<AuthProvider>,
    pub jwt_expiry: u64,
    pub refresh_expiry: u64,
    pub callback_base: String,
    pub login_page: String,
    pub protected_paths: Vec<String>,
}

impl AuthConfig {
    pub fn new(secret: impl Into<String>) -> Self {
        Self {
            secret: secret.into(),
            providers: Vec::new(),
            jwt_expiry: 3600,
            refresh_expiry: 86400 * 7,
            callback_base: "/auth/callback".to_string(),
            login_page: "/login".to_string(),
            protected_paths: Vec::new(),
        }
    }

    pub fn add_provider(mut self, provider: AuthProvider) -> Self {
        self.providers.push(provider);
        self
    }

    pub fn jwt_expiry(mut self, secs: u64) -> Self {
        self.jwt_expiry = secs;
        self
    }

    pub fn refresh_expiry(mut self, secs: u64) -> Self {
        self.refresh_expiry = secs;
        self
    }

    pub fn protect(mut self, path: impl Into<String>) -> Self {
        self.protected_paths.push(path.into());
        self
    }

    pub fn login_page(mut self, path: impl Into<String>) -> Self {
        self.login_page = path.into();
        self
    }

    /// Get the callback URL for a specific provider
    pub fn callback_url(&self, provider_name: &str) -> String {
        format!("{}/{}", self.callback_base, provider_name)
    }

    /// Create a JWT instance using this config's secret
    pub fn jwt(&self) -> Jwt {
        Jwt::new(&self.secret)
    }

    /// Check if a path requires authentication
    pub fn is_protected(&self, path: &str) -> bool {
        self.protected_paths.iter().any(|p| {
            if p.ends_with('*') {
                path.starts_with(&p[..p.len() - 1])
            } else {
                path == p
            }
        })
    }

    /// Generate auth middleware guard HTML redirect
    pub fn guard_redirect(&self) -> String {
        format!(
            "<meta http-equiv=\"refresh\" content=\"0;url={}\">",
            self.login_page
        )
    }

    /// Generate login page HTML with OAuth buttons
    pub fn login_page_html(&self, base_url: &str) -> String {
        let mut html = String::from("<div class=\"auth-login\">\n");
        html.push_str("  <h2>Sign In</h2>\n");

        for provider in &self.providers {
            let state = generate_state();
            let redirect = format!("{}{}/{}", base_url, self.callback_base, provider.name);
            let url = provider.get_authorize_url(&redirect, &state);
            html.push_str(&format!(
                "  <a href=\"{}\" class=\"auth-btn auth-btn-{}\">Sign in with {}</a>\n",
                url,
                provider.name,
                capitalize(&provider.name)
            ));
        }

        html.push_str("</div>");
        html
    }
}

// ─── Password Hashing ──────────────────────────────────────

/// Simple password hashing using HMAC-SHA256 with salt
/// (For production, use argon2 or bcrypt crate)
pub fn hash_password(password: &str, salt: &str) -> String {
    let salted = format!("{}:{}", salt, password);
    let hash = sha256(salted.as_bytes());
    hex_encode(&hash)
}

/// Verify a password against a hash
pub fn verify_password(password: &str, salt: &str, hash: &str) -> bool {
    let computed = hash_password(password, salt);
    constant_time_eq(computed.as_bytes(), hash.as_bytes())
}

/// Generate a random-ish salt (uses timestamp + counter for uniqueness)
pub fn generate_salt() -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    let seed = format!("salt-{}-{}", now.as_nanos(), now.as_micros());
    let hash = sha256(seed.as_bytes());
    hex_encode(&hash[..16])
}

// ─── Auth Errors ────────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum AuthError {
    InvalidToken(String),
    TokenExpired,
    InvalidCredentials,
    ProviderError(String),
    Unauthorized,
}

impl std::fmt::Display for AuthError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AuthError::InvalidToken(msg) => write!(f, "Invalid token: {}", msg),
            AuthError::TokenExpired => write!(f, "Token expired"),
            AuthError::InvalidCredentials => write!(f, "Invalid credentials"),
            AuthError::ProviderError(msg) => write!(f, "Provider error: {}", msg),
            AuthError::Unauthorized => write!(f, "Unauthorized"),
        }
    }
}

// ─── CSRF State ─────────────────────────────────────────────

fn generate_state() -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    let seed = format!("state-{}", now.as_nanos());
    let hash = sha256(seed.as_bytes());
    hex_encode(&hash[..16])
}

// ─── Crypto Helpers ─────────────────────────────────────────

fn sha256(data: &[u8]) -> [u8; 32] {
    // SHA-256 implementation
    let mut h: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a,
        0x510e527f, 0x9b05688c, 0x1f83d9ab, 0x5be0cd19,
    ];

    let k: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5,
        0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174,
        0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
        0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7, 0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967,
        0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85,
        0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
        0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
        0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2,
    ];

    // Pre-processing: pad message
    let bit_len = (data.len() as u64) * 8;
    let mut padded = data.to_vec();
    padded.push(0x80);
    while (padded.len() % 64) != 56 {
        padded.push(0);
    }
    padded.extend_from_slice(&bit_len.to_be_bytes());

    // Process each 512-bit block
    for chunk in padded.chunks(64) {
        let mut w = [0u32; 64];
        for i in 0..16 {
            w[i] = u32::from_be_bytes([
                chunk[i * 4],
                chunk[i * 4 + 1],
                chunk[i * 4 + 2],
                chunk[i * 4 + 3],
            ]);
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16].wrapping_add(s0).wrapping_add(w[i - 7]).wrapping_add(s1);
        }

        let (mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut hh) =
            (h[0], h[1], h[2], h[3], h[4], h[5], h[6], h[7]);

        for i in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ ((!e) & g);
            let temp1 = hh.wrapping_add(s1).wrapping_add(ch).wrapping_add(k[i]).wrapping_add(w[i]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let temp2 = s0.wrapping_add(maj);

            hh = g; g = f; f = e; e = d.wrapping_add(temp1);
            d = c; c = b; b = a; a = temp1.wrapping_add(temp2);
        }

        h[0] = h[0].wrapping_add(a);
        h[1] = h[1].wrapping_add(b);
        h[2] = h[2].wrapping_add(c);
        h[3] = h[3].wrapping_add(d);
        h[4] = h[4].wrapping_add(e);
        h[5] = h[5].wrapping_add(f);
        h[6] = h[6].wrapping_add(g);
        h[7] = h[7].wrapping_add(hh);
    }

    let mut result = [0u8; 32];
    for i in 0..8 {
        result[i * 4..i * 4 + 4].copy_from_slice(&h[i].to_be_bytes());
    }
    result
}

fn hmac_sha256(key: &[u8], message: &[u8]) -> [u8; 32] {
    let block_size = 64;
    let mut padded_key = vec![0u8; block_size];

    if key.len() > block_size {
        let hash = sha256(key);
        padded_key[..32].copy_from_slice(&hash);
    } else {
        padded_key[..key.len()].copy_from_slice(key);
    }

    let mut ipad = vec![0x36u8; block_size];
    let mut opad = vec![0x5cu8; block_size];
    for i in 0..block_size {
        ipad[i] ^= padded_key[i];
        opad[i] ^= padded_key[i];
    }

    ipad.extend_from_slice(message);
    let inner_hash = sha256(&ipad);

    opad.extend_from_slice(&inner_hash);
    sha256(&opad)
}

fn base64url_encode(data: &[u8]) -> String {
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
    let mut result = String::new();
    let mut i = 0;
    while i < data.len() {
        let b0 = data[i] as usize;
        let b1 = if i + 1 < data.len() { data[i + 1] as usize } else { 0 };
        let b2 = if i + 2 < data.len() { data[i + 2] as usize } else { 0 };

        result.push(CHARS[b0 >> 2] as char);
        result.push(CHARS[((b0 & 0x03) << 4) | (b1 >> 4)] as char);

        if i + 1 < data.len() {
            result.push(CHARS[((b1 & 0x0F) << 2) | (b2 >> 6)] as char);
        }
        if i + 2 < data.len() {
            result.push(CHARS[b2 & 0x3F] as char);
        }
        i += 3;
    }
    result
}

fn base64url_decode(input: &str) -> Option<Vec<u8>> {
    let mut result = Vec::new();
    let bytes: Vec<u8> = input
        .bytes()
        .filter_map(|b| {
            match b {
                b'A'..=b'Z' => Some(b - b'A'),
                b'a'..=b'z' => Some(b - b'a' + 26),
                b'0'..=b'9' => Some(b - b'0' + 52),
                b'-' => Some(62),
                b'_' => Some(63),
                _ => None,
            }
        })
        .collect();

    let mut i = 0;
    while i < bytes.len() {
        if i + 1 < bytes.len() {
            result.push((bytes[i] << 2) | (bytes[i + 1] >> 4));
        }
        if i + 2 < bytes.len() {
            result.push(((bytes[i + 1] & 0x0F) << 4) | (bytes[i + 2] >> 2));
        }
        if i + 3 < bytes.len() {
            result.push(((bytes[i + 2] & 0x03) << 6) | bytes[i + 3]);
        }
        i += 4;
    }
    Some(result)
}

fn hex_encode(data: &[u8]) -> String {
    data.iter().map(|b| format!("{:02x}", b)).collect()
}

fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

fn url_encode(s: &str) -> String {
    let mut result = String::new();
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                result.push(b as char);
            }
            _ => {
                result.push_str(&format!("%{:02X}", b));
            }
        }
    }
    result
}

fn extract_json_string(json: &str, key: &str) -> Option<String> {
    let pattern = format!("\"{}\":\"", key);
    let start = json.find(&pattern)? + pattern.len();
    let end = json[start..].find('"')? + start;
    Some(json[start..end].to_string())
}

fn extract_json_number(json: &str, key: &str) -> Option<u64> {
    let pattern = format!("\"{}\":", key);
    let start = json.find(&pattern)? + pattern.len();
    let rest = json[start..].trim_start();
    let end = rest.find(|c: char| !c.is_ascii_digit()).unwrap_or(rest.len());
    rest[..end].parse().ok()
}

fn capitalize(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        None => String::new(),
        Some(f) => f.to_uppercase().to_string() + c.as_str(),
    }
}

// ─── Tests ──────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_jwt_sign_verify() {
        let jwt = Jwt::new("secret");
        let claims = Claims::new("user123", 3600);
        let token = jwt.sign(&claims);
        let decoded = jwt.verify(&token).unwrap();
        assert_eq!(decoded.sub, "user123");
    }

    #[test]
    fn test_jwt_with_claims() {
        let jwt = Jwt::new("secret");
        let claims = Claims::new("user1", 3600)
            .email("test@example.com")
            .name("Test User")
            .role("admin");
        let token = jwt.sign(&claims);
        let decoded = jwt.verify(&token).unwrap();
        assert_eq!(decoded.email, Some("test@example.com".to_string()));
        assert_eq!(decoded.name, Some("Test User".to_string()));
        assert_eq!(decoded.role, Some("admin".to_string()));
    }

    #[test]
    fn test_jwt_invalid_signature() {
        let jwt1 = Jwt::new("secret1");
        let jwt2 = Jwt::new("secret2");
        let claims = Claims::new("user1", 3600);
        let token = jwt1.sign(&claims);
        assert!(jwt2.verify(&token).is_err());
    }

    #[test]
    fn test_jwt_expired() {
        let jwt = Jwt::new("secret");
        let claims = Claims {
            sub: "user".to_string(),
            exp: 0, // expired
            iat: 0,
            email: None,
            name: None,
            role: None,
            custom: HashMap::new(),
        };
        let token = jwt.sign(&claims);
        match jwt.verify(&token) {
            Err(AuthError::TokenExpired) => {}
            _ => panic!("expected TokenExpired"),
        }
    }

    #[test]
    fn test_jwt_malformed() {
        let jwt = Jwt::new("secret");
        assert!(jwt.verify("not.a.valid.token").is_err());
        assert!(jwt.verify("abc").is_err());
    }

    #[test]
    fn test_password_hash() {
        let salt = "random_salt";
        let hash = hash_password("mypassword", salt);
        assert!(verify_password("mypassword", salt, &hash));
        assert!(!verify_password("wrongpassword", salt, &hash));
    }

    #[test]
    fn test_generate_salt() {
        let s1 = generate_salt();
        assert_eq!(s1.len(), 32); // 16 bytes hex
    }

    #[test]
    fn test_github_provider() {
        let provider = AuthProvider::github("id", "secret");
        assert_eq!(provider.name, "github");
        let url = provider.get_authorize_url("http://localhost/callback", "state123");
        assert!(url.contains("github.com"));
        assert!(url.contains("id"));
        assert!(url.contains("state123"));
    }

    #[test]
    fn test_google_provider() {
        let provider = AuthProvider::google("id", "secret");
        assert_eq!(provider.name, "google");
        assert!(provider.authorize_url.contains("google"));
    }

    #[test]
    fn test_discord_provider() {
        let provider = AuthProvider::discord("id", "secret");
        assert_eq!(provider.name, "discord");
        assert!(provider.authorize_url.contains("discord"));
    }

    #[test]
    fn test_auth_config() {
        let config = AuthConfig::new("secret")
            .add_provider(AuthProvider::github("id", "sec"))
            .jwt_expiry(7200)
            .protect("/dashboard/*")
            .protect("/admin/*");

        assert_eq!(config.providers.len(), 1);
        assert_eq!(config.jwt_expiry, 7200);
        assert!(config.is_protected("/dashboard/settings"));
        assert!(config.is_protected("/admin/users"));
        assert!(!config.is_protected("/public"));
    }

    #[test]
    fn test_auth_callback_url() {
        let config = AuthConfig::new("secret");
        assert_eq!(config.callback_url("github"), "/auth/callback/github");
    }

    #[test]
    fn test_login_page_html() {
        let config = AuthConfig::new("secret")
            .add_provider(AuthProvider::github("id", "sec"))
            .add_provider(AuthProvider::google("id", "sec"));
        let html = config.login_page_html("http://localhost:3000");
        assert!(html.contains("Sign in with Github"));
        assert!(html.contains("Sign in with Google"));
    }

    #[test]
    fn test_url_encode() {
        assert_eq!(url_encode("hello world"), "hello%20world");
        assert_eq!(url_encode("a+b=c"), "a%2Bb%3Dc");
    }

    #[test]
    fn test_sha256() {
        let hash = sha256(b"");
        let hex = hex_encode(&hash);
        assert_eq!(hex, "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855");
    }

    #[test]
    fn test_claims_expired() {
        let c = Claims {
            sub: "u".into(), exp: 0, iat: 0,
            email: None, name: None, role: None, custom: HashMap::new(),
        };
        assert!(c.is_expired());

        let c2 = Claims::new("u", 9999);
        assert!(!c2.is_expired());
    }
}
