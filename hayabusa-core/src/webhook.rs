//! Webhook Support for Hayabusa.
//!
//! Signature verification and payload parsing for incoming webhooks
//! from Stripe, GitHub, Discord, Slack, and custom providers.
//!
//! ```ignore
//! use hayabusa_core::prelude::*;
//!
//! let wh = WebhookVerifier::stripe("whsec_...");
//! let valid = wh.verify(payload, &headers);
//! ```

use std::collections::HashMap;
use std::num::Wrapping;

// ─── HMAC-SHA256 (reuse from auth) ────────────────────────

fn hmac_sha256(key: &[u8], message: &[u8]) -> [u8; 32] {
    let block_size = 64usize;
    let mut padded_key = vec![0u8; block_size];
    if key.len() > block_size {
        let h = sha256(key);
        padded_key[..32].copy_from_slice(&h);
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

fn sha256(data: &[u8]) -> [u8; 32] {
    let k: [u32; 64] = [
        0x428a2f98,0x71374491,0xb5c0fbcf,0xe9b5dba5,0x3956c25b,0x59f111f1,0x923f82a4,0xab1c5ed5,
        0xd807aa98,0x12835b01,0x243185be,0x550c7dc3,0x72be5d74,0x80deb1fe,0x9bdc06a7,0xc19bf174,
        0xe49b69c1,0xefbe4786,0x0fc19dc6,0x240ca1cc,0x2de92c6f,0x4a7484aa,0x5cb0a9dc,0x76f988da,
        0x983e5152,0xa831c66d,0xb00327c8,0xbf597fc7,0xc6e00bf3,0xd5a79147,0x06ca6351,0x14292967,
        0x27b70a85,0x2e1b2138,0x4d2c6dfc,0x53380d13,0x650a7354,0x766a0abb,0x81c2c92e,0x92722c85,
        0xa2bfe8a1,0xa81a664b,0xc24b8b70,0xc76c51a3,0xd192e819,0xd6990624,0xf40e3585,0x106aa070,
        0x19a4c116,0x1e376c08,0x2748774c,0x34b0bcb5,0x391c0cb3,0x4ed8aa4a,0x5b9cca4f,0x682e6ff3,
        0x748f82ee,0x78a5636f,0x84c87814,0x8cc70208,0x90befffa,0xa4506ceb,0xbef9a3f7,0xc67178f2,
    ];
    let mut h: [Wrapping<u32>; 8] = [
        Wrapping(0x6a09e667), Wrapping(0xbb67ae85), Wrapping(0x3c6ef372), Wrapping(0xa54ff53a),
        Wrapping(0x510e527f), Wrapping(0x9b05688c), Wrapping(0x1f83d9ab), Wrapping(0x5be0cd19),
    ];
    let orig_len = data.len();
    let mut msg = data.to_vec();
    msg.push(0x80);
    while msg.len() % 64 != 56 { msg.push(0); }
    let bit_len = (orig_len as u64) * 8;
    msg.extend_from_slice(&bit_len.to_be_bytes());

    for chunk in msg.chunks(64) {
        let mut w = [Wrapping(0u32); 64];
        for i in 0..16 {
            w[i] = Wrapping(u32::from_be_bytes([chunk[4*i], chunk[4*i+1], chunk[4*i+2], chunk[4*i+3]]));
        }
        for i in 16..64 {
            let s0 = (w[i-15].0.rotate_right(7)) ^ (w[i-15].0.rotate_right(18)) ^ (w[i-15].0 >> 3);
            let s1 = (w[i-2].0.rotate_right(17)) ^ (w[i-2].0.rotate_right(19)) ^ (w[i-2].0 >> 10);
            w[i] = w[i-16] + Wrapping(s0) + w[i-7] + Wrapping(s1);
        }
        let (mut a,mut b,mut c,mut d,mut e,mut f,mut g,mut hh) = (h[0],h[1],h[2],h[3],h[4],h[5],h[6],h[7]);
        for i in 0..64 {
            let s1 = Wrapping(e.0.rotate_right(6) ^ e.0.rotate_right(11) ^ e.0.rotate_right(25));
            let ch = Wrapping((e.0 & f.0) ^ ((!e.0) & g.0));
            let t1 = hh + s1 + ch + Wrapping(k[i]) + w[i];
            let s0 = Wrapping(a.0.rotate_right(2) ^ a.0.rotate_right(13) ^ a.0.rotate_right(22));
            let maj = Wrapping((a.0 & b.0) ^ (a.0 & c.0) ^ (b.0 & c.0));
            let t2 = s0 + maj;
            hh=g; g=f; f=e; e=d+t1; d=c; c=b; b=a; a=t1+t2;
        }
        h[0]=h[0]+a; h[1]=h[1]+b; h[2]=h[2]+c; h[3]=h[3]+d;
        h[4]=h[4]+e; h[5]=h[5]+f; h[6]=h[6]+g; h[7]=h[7]+hh;
    }
    let mut result = [0u8; 32];
    for i in 0..8 { result[4*i..4*i+4].copy_from_slice(&h[i].0.to_be_bytes()); }
    result
}

fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

fn hex_decode(s: &str) -> Option<Vec<u8>> {
    if s.len() % 2 != 0 { return None; }
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i+2], 16).ok())
        .collect()
}

fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() { return false; }
    let mut diff = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

// ─── Webhook Verifier ──────────────────────────────────────

/// Webhook provider type
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum WebhookProvider {
    Stripe,
    GitHub,
    Slack,
    Discord,
    Custom,
}

/// Webhook signature verifier
#[derive(Debug, Clone)]
pub struct WebhookVerifier {
    pub provider: WebhookProvider,
    pub secret: String,
    pub tolerance_secs: Option<u64>,
}

impl WebhookVerifier {
    /// Create a Stripe webhook verifier
    pub fn stripe(secret: &str) -> Self {
        Self {
            provider: WebhookProvider::Stripe,
            secret: secret.to_string(),
            tolerance_secs: Some(300), // 5 minutes
        }
    }

    /// Create a GitHub webhook verifier
    pub fn github(secret: &str) -> Self {
        Self {
            provider: WebhookProvider::GitHub,
            secret: secret.to_string(),
            tolerance_secs: None,
        }
    }

    /// Create a Slack webhook verifier
    pub fn slack(signing_secret: &str) -> Self {
        Self {
            provider: WebhookProvider::Slack,
            secret: signing_secret.to_string(),
            tolerance_secs: Some(300),
        }
    }

    /// Create a custom HMAC-SHA256 webhook verifier
    pub fn custom(secret: &str) -> Self {
        Self {
            provider: WebhookProvider::Custom,
            secret: secret.to_string(),
            tolerance_secs: None,
        }
    }

    /// Set timestamp tolerance
    pub fn tolerance(mut self, secs: u64) -> Self {
        self.tolerance_secs = Some(secs);
        self
    }

    /// Verify a webhook signature
    pub fn verify(&self, payload: &[u8], headers: &HashMap<String, String>) -> WebhookResult {
        match self.provider {
            WebhookProvider::Stripe => self.verify_stripe(payload, headers),
            WebhookProvider::GitHub => self.verify_github(payload, headers),
            WebhookProvider::Slack => self.verify_slack(payload, headers),
            WebhookProvider::Discord | WebhookProvider::Custom => self.verify_custom_hmac(payload, headers),
        }
    }

    fn verify_stripe(&self, payload: &[u8], headers: &HashMap<String, String>) -> WebhookResult {
        let sig_header = match headers.get("stripe-signature").or(headers.get("Stripe-Signature")) {
            Some(s) => s.clone(),
            None => return WebhookResult::err("Missing Stripe-Signature header"),
        };

        // Parse t=...,v1=...
        let mut timestamp = None;
        let mut signature = None;
        for part in sig_header.split(',') {
            let mut kv = part.splitn(2, '=');
            match (kv.next(), kv.next()) {
                (Some("t"), Some(v)) => timestamp = v.parse::<u64>().ok(),
                (Some("v1"), Some(v)) => signature = Some(v.to_string()),
                _ => {}
            }
        }

        let ts = match timestamp {
            Some(t) => t,
            None => return WebhookResult::err("Invalid timestamp in Stripe signature"),
        };
        let sig = match signature {
            Some(s) => s,
            None => return WebhookResult::err("Missing v1 signature"),
        };

        // Check timestamp tolerance
        if let Some(tolerance) = self.tolerance_secs {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            if now.abs_diff(ts) > tolerance {
                return WebhookResult::err("Timestamp outside tolerance");
            }
        }

        // Compute expected signature: HMAC-SHA256(secret, "timestamp.payload")
        let signed_payload = format!("{}.{}", ts, String::from_utf8_lossy(payload));
        let expected = hmac_sha256(self.secret.as_bytes(), signed_payload.as_bytes());
        let expected_hex = hex_encode(&expected);

        if constant_time_eq(expected_hex.as_bytes(), sig.as_bytes()) {
            WebhookResult::ok(ts)
        } else {
            WebhookResult::err("Signature mismatch")
        }
    }

    fn verify_github(&self, payload: &[u8], headers: &HashMap<String, String>) -> WebhookResult {
        let sig = match headers.get("x-hub-signature-256").or(headers.get("X-Hub-Signature-256")) {
            Some(s) => s.clone(),
            None => return WebhookResult::err("Missing X-Hub-Signature-256 header"),
        };

        let sig_hex = sig.strip_prefix("sha256=").unwrap_or(&sig);
        let expected = hmac_sha256(self.secret.as_bytes(), payload);
        let expected_hex = hex_encode(&expected);

        if constant_time_eq(expected_hex.as_bytes(), sig_hex.as_bytes()) {
            WebhookResult::ok(0)
        } else {
            WebhookResult::err("Signature mismatch")
        }
    }

    fn verify_slack(&self, payload: &[u8], headers: &HashMap<String, String>) -> WebhookResult {
        let timestamp = match headers.get("x-slack-request-timestamp").or(headers.get("X-Slack-Request-Timestamp")) {
            Some(t) => t.clone(),
            None => return WebhookResult::err("Missing X-Slack-Request-Timestamp"),
        };
        let sig = match headers.get("x-slack-signature").or(headers.get("X-Slack-Signature")) {
            Some(s) => s.clone(),
            None => return WebhookResult::err("Missing X-Slack-Signature"),
        };

        let ts: u64 = match timestamp.parse() {
            Ok(t) => t,
            Err(_) => return WebhookResult::err("Invalid timestamp"),
        };

        // Tolerance check
        if let Some(tolerance) = self.tolerance_secs {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            if now.abs_diff(ts) > tolerance {
                return WebhookResult::err("Timestamp outside tolerance");
            }
        }

        let basestring = format!("v0:{}:{}", timestamp, String::from_utf8_lossy(payload));
        let expected = hmac_sha256(self.secret.as_bytes(), basestring.as_bytes());
        let expected_sig = format!("v0={}", hex_encode(&expected));

        if constant_time_eq(expected_sig.as_bytes(), sig.as_bytes()) {
            WebhookResult::ok(ts)
        } else {
            WebhookResult::err("Signature mismatch")
        }
    }

    fn verify_custom_hmac(&self, payload: &[u8], headers: &HashMap<String, String>) -> WebhookResult {
        let sig = match headers.get("x-signature").or(headers.get("X-Signature")) {
            Some(s) => s.clone(),
            None => return WebhookResult::err("Missing X-Signature header"),
        };

        let expected = hmac_sha256(self.secret.as_bytes(), payload);
        let expected_hex = hex_encode(&expected);

        if constant_time_eq(expected_hex.as_bytes(), sig.as_bytes()) {
            WebhookResult::ok(0)
        } else {
            WebhookResult::err("Signature mismatch")
        }
    }
}

// ─── Webhook Result ────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct WebhookResult {
    pub valid: bool,
    pub error: Option<String>,
    pub timestamp: Option<u64>,
}

impl WebhookResult {
    fn ok(timestamp: u64) -> Self {
        Self { valid: true, error: None, timestamp: if timestamp > 0 { Some(timestamp) } else { None } }
    }

    fn err(msg: &str) -> Self {
        Self { valid: false, error: Some(msg.to_string()), timestamp: None }
    }

    pub fn is_valid(&self) -> bool {
        self.valid
    }
}

// ─── Webhook Event Parser ──────────────────────────────────

/// Parsed webhook event
#[derive(Debug, Clone)]
pub struct WebhookEvent {
    pub provider: WebhookProvider,
    pub event_type: String,
    pub payload: String,
}

impl WebhookEvent {
    /// Parse event type from headers
    pub fn from_headers(provider: WebhookProvider, headers: &HashMap<String, String>, body: &str) -> Self {
        let event_type = match provider {
            WebhookProvider::Stripe => headers.get("stripe-event-type")
                .or(headers.get("Stripe-Event-Type"))
                .cloned()
                .unwrap_or_default(),
            WebhookProvider::GitHub => headers.get("x-github-event")
                .or(headers.get("X-GitHub-Event"))
                .cloned()
                .unwrap_or_default(),
            WebhookProvider::Slack => "slack_event".to_string(),
            _ => "webhook".to_string(),
        };
        Self {
            provider,
            event_type,
            payload: body.to_string(),
        }
    }
}

// ─── Tests ──────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hmac_sha256() {
        let mac = hmac_sha256(b"key", b"message");
        let hex = hex_encode(&mac);
        assert_eq!(hex.len(), 64);
        assert!(!hex.chars().all(|c| c == '0'));
    }

    #[test]
    fn test_hex_encode_decode() {
        let data = b"hello";
        let hex = hex_encode(data);
        let decoded = hex_decode(&hex).unwrap();
        assert_eq!(decoded, data);
    }

    #[test]
    fn test_constant_time_eq() {
        assert!(constant_time_eq(b"hello", b"hello"));
        assert!(!constant_time_eq(b"hello", b"world"));
        assert!(!constant_time_eq(b"short", b"longer"));
    }

    #[test]
    fn test_github_verify() {
        let secret = "mysecret";
        let payload = b"test payload";
        let verifier = WebhookVerifier::github(secret);

        let expected = hmac_sha256(secret.as_bytes(), payload);
        let sig = format!("sha256={}", hex_encode(&expected));

        let mut headers = HashMap::new();
        headers.insert("x-hub-signature-256".to_string(), sig);

        let result = verifier.verify(payload, &headers);
        assert!(result.is_valid());
    }

    #[test]
    fn test_github_verify_bad_sig() {
        let verifier = WebhookVerifier::github("secret");
        let mut headers = HashMap::new();
        headers.insert("x-hub-signature-256".to_string(), "sha256=bad".to_string());
        let result = verifier.verify(b"payload", &headers);
        assert!(!result.is_valid());
    }

    #[test]
    fn test_github_verify_missing_header() {
        let verifier = WebhookVerifier::github("secret");
        let result = verifier.verify(b"payload", &HashMap::new());
        assert!(!result.is_valid());
        assert!(result.error.unwrap().contains("Missing"));
    }

    #[test]
    fn test_custom_verify() {
        let secret = "webhook_secret";
        let payload = b"data";
        let verifier = WebhookVerifier::custom(secret);

        let sig = hex_encode(&hmac_sha256(secret.as_bytes(), payload));
        let mut headers = HashMap::new();
        headers.insert("x-signature".to_string(), sig);

        assert!(verifier.verify(payload, &headers).is_valid());
    }

    #[test]
    fn test_stripe_missing_header() {
        let verifier = WebhookVerifier::stripe("whsec_test");
        let result = verifier.verify(b"payload", &HashMap::new());
        assert!(!result.is_valid());
    }

    #[test]
    fn test_slack_missing_headers() {
        let verifier = WebhookVerifier::slack("slack_secret");
        let result = verifier.verify(b"payload", &HashMap::new());
        assert!(!result.is_valid());
    }

    #[test]
    fn test_webhook_event() {
        let mut headers = HashMap::new();
        headers.insert("x-github-event".to_string(), "push".to_string());
        let event = WebhookEvent::from_headers(WebhookProvider::GitHub, &headers, "{}");
        assert_eq!(event.event_type, "push");
        assert_eq!(event.provider, WebhookProvider::GitHub);
    }

    #[test]
    fn test_webhook_result() {
        let ok = WebhookResult::ok(1234567890);
        assert!(ok.is_valid());
        assert_eq!(ok.timestamp, Some(1234567890));

        let err = WebhookResult::err("bad");
        assert!(!err.is_valid());
        assert_eq!(err.error, Some("bad".to_string()));
    }

    #[test]
    fn test_provider_tolerance() {
        let v = WebhookVerifier::custom("s").tolerance(60);
        assert_eq!(v.tolerance_secs, Some(60));
    }
}
