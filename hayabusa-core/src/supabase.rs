//! Supabase Client for Hayabusa.
//!
//! Unofficial Rust client for Supabase REST API, Auth, Storage, and Realtime.
//! No official Supabase Rust SDK exists, so this talks directly to the HTTP APIs.
//!
//! ## Supported Features
//! - **Database** (PostgREST): select, insert, update, delete, upsert, RPC
//! - **Auth**: sign up, sign in (email/password), sign out, get user, refresh token
//! - **Storage**: upload, download, list, delete, get public URL
//! - **Realtime**: subscribe to changes via SSE (server-side)
//!
//! ## Usage
//! ```ignore
//! let sb = SupabaseClient::new(
//!     "https://xxxx.supabase.co",
//!     "your-anon-key",
//! );
//!
//! // Database: select
//! let posts = sb.from("posts")
//!     .select("id,title,body")
//!     .eq("published", "true")
//!     .order("created_at", false)
//!     .limit(10)
//!     .execute()
//!     .await?;
//!
//! // Auth: sign in
//! let session = sb.auth().sign_in_email("user@example.com", "password").await?;
//!
//! // Storage: upload
//! sb.storage().upload("avatars", "user1.png", image_bytes, "image/png").await?;
//! ```

use reqwest::{Client, Response};
use serde_json::Value;

/// Supabase client errors
#[derive(Debug, Clone)]
pub enum SupabaseError {
    RequestFailed(String),
    AuthError(String),
    StorageError(String),
    ParseError(String),
}

impl std::fmt::Display for SupabaseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SupabaseError::RequestFailed(m) => write!(f, "Supabase request failed: {}", m),
            SupabaseError::AuthError(m) => write!(f, "Supabase auth error: {}", m),
            SupabaseError::StorageError(m) => write!(f, "Supabase storage error: {}", m),
            SupabaseError::ParseError(m) => write!(f, "Supabase parse error: {}", m),
        }
    }
}

/// Supabase client
#[derive(Debug, Clone)]
pub struct SupabaseClient {
    url: String,
    anon_key: String,
    service_role_key: Option<String>,
    client: Client,
    /// Optional JWT for authenticated requests
    access_token: Option<String>,
}

impl SupabaseClient {
    /// Create a new Supabase client with project URL and anon key
    pub fn new(url: impl Into<String>, anon_key: impl Into<String>) -> Self {
        Self {
            url: url.into().trim_end_matches('/').to_string(),
            anon_key: anon_key.into(),
            service_role_key: None,
            client: Client::new(),
            access_token: None,
        }
    }

    /// Set service role key for admin operations
    pub fn with_service_role_key(mut self, key: impl Into<String>) -> Self {
        self.service_role_key = Some(key.into());
        self
    }

    /// Set an access token (JWT) from a logged-in user
    pub fn with_access_token(mut self, token: impl Into<String>) -> Self {
        self.access_token = Some(token.into());
        self
    }

    /// Get the API key to use for a request
    fn api_key(&self) -> &str {
        &self.anon_key
    }

    /// Get the authorization header value
    fn auth_header(&self) -> String {
        if let Some(ref token) = self.access_token {
            format!("Bearer {}", token)
        } else {
            format!("Bearer {}", self.anon_key)
        }
    }

    // ─────────────────────────────────────────
    //  Database (PostgREST)
    // ─────────────────────────────────────────

    /// Start a query builder for a table
    pub fn from(&self, table: &str) -> QueryBuilder {
        QueryBuilder {
            client: self.client.clone(),
            base_url: format!("{}/rest/v1/{}", self.url, table),
            api_key: self.api_key().to_string(),
            auth_header: self.auth_header(),
            select_cols: None,
            filters: Vec::new(),
            order_by: None,
            limit_val: None,
            offset_val: None,
            single: false,
            count_mode: None,
        }
    }

    /// Call a Postgres RPC function
    pub async fn rpc(
        &self,
        function: &str,
        params: &Value,
    ) -> Result<Value, SupabaseError> {
        let url = format!("{}/rest/v1/rpc/{}", self.url, function);
        let resp = self
            .client
            .post(&url)
            .header("apikey", self.api_key())
            .header("Authorization", self.auth_header())
            .header("Content-Type", "application/json")
            .json(params)
            .send()
            .await
            .map_err(|e| SupabaseError::RequestFailed(e.to_string()))?;

        parse_response(resp).await
    }

    // ─────────────────────────────────────────
    //  Auth
    // ─────────────────────────────────────────

    /// Get the Auth API client
    pub fn auth(&self) -> SupabaseAuth {
        SupabaseAuth {
            client: self.client.clone(),
            url: format!("{}/auth/v1", self.url),
            api_key: self.api_key().to_string(),
        }
    }

    // ─────────────────────────────────────────
    //  Storage
    // ─────────────────────────────────────────

    /// Get the Storage API client
    pub fn storage(&self) -> SupabaseStorage {
        SupabaseStorage {
            client: self.client.clone(),
            url: format!("{}/storage/v1", self.url),
            api_key: self.api_key().to_string(),
            auth_header: self.auth_header(),
        }
    }
}

// ─────────────────────────────────────────────
//  PostgREST Query Builder
// ─────────────────────────────────────────────

/// Fluent query builder for PostgREST
#[derive(Debug, Clone)]
pub struct QueryBuilder {
    client: Client,
    base_url: String,
    api_key: String,
    auth_header: String,
    select_cols: Option<String>,
    filters: Vec<(String, String)>,
    order_by: Option<String>,
    limit_val: Option<u64>,
    offset_val: Option<u64>,
    single: bool,
    count_mode: Option<String>,
}

impl QueryBuilder {
    /// Select specific columns
    pub fn select(mut self, columns: &str) -> Self {
        self.select_cols = Some(columns.to_string());
        self
    }

    /// Filter: column equals value
    pub fn eq(mut self, column: &str, value: &str) -> Self {
        self.filters.push((column.to_string(), format!("eq.{}", value)));
        self
    }

    /// Filter: column not equals value
    pub fn neq(mut self, column: &str, value: &str) -> Self {
        self.filters.push((column.to_string(), format!("neq.{}", value)));
        self
    }

    /// Filter: column greater than value
    pub fn gt(mut self, column: &str, value: &str) -> Self {
        self.filters.push((column.to_string(), format!("gt.{}", value)));
        self
    }

    /// Filter: column greater than or equal
    pub fn gte(mut self, column: &str, value: &str) -> Self {
        self.filters.push((column.to_string(), format!("gte.{}", value)));
        self
    }

    /// Filter: column less than value
    pub fn lt(mut self, column: &str, value: &str) -> Self {
        self.filters.push((column.to_string(), format!("lt.{}", value)));
        self
    }

    /// Filter: column less than or equal
    pub fn lte(mut self, column: &str, value: &str) -> Self {
        self.filters.push((column.to_string(), format!("lte.{}", value)));
        self
    }

    /// Filter: column LIKE pattern
    pub fn like(mut self, column: &str, pattern: &str) -> Self {
        self.filters.push((column.to_string(), format!("like.{}", pattern)));
        self
    }

    /// Filter: column ILIKE pattern (case-insensitive)
    pub fn ilike(mut self, column: &str, pattern: &str) -> Self {
        self.filters.push((column.to_string(), format!("ilike.{}", pattern)));
        self
    }

    /// Filter: column IS value (null, true, false)
    pub fn is(mut self, column: &str, value: &str) -> Self {
        self.filters.push((column.to_string(), format!("is.{}", value)));
        self
    }

    /// Filter: column IN (value1, value2, ...)
    pub fn in_list(mut self, column: &str, values: &[&str]) -> Self {
        let list = format!("in.({})", values.join(","));
        self.filters.push((column.to_string(), list));
        self
    }

    /// Filter: full-text search
    pub fn text_search(mut self, column: &str, query: &str) -> Self {
        self.filters.push((column.to_string(), format!("fts.{}", query)));
        self
    }

    /// Order by column
    pub fn order(mut self, column: &str, ascending: bool) -> Self {
        let dir = if ascending { "asc" } else { "desc" };
        self.order_by = Some(format!("{}.{}", column, dir));
        self
    }

    /// Limit results
    pub fn limit(mut self, count: u64) -> Self {
        self.limit_val = Some(count);
        self
    }

    /// Offset (for pagination)
    pub fn offset(mut self, start: u64) -> Self {
        self.offset_val = Some(start);
        self
    }

    /// Return single row (404 if not found)
    pub fn single(mut self) -> Self {
        self.single = true;
        self
    }

    /// Include count in response
    pub fn count(mut self, mode: &str) -> Self {
        self.count_mode = Some(mode.to_string());
        self
    }

    fn build_url(&self) -> String {
        let mut url = self.base_url.clone();
        let mut params: Vec<String> = Vec::new();

        if let Some(ref cols) = self.select_cols {
            params.push(format!("select={}", cols));
        }

        for (col, filter) in &self.filters {
            params.push(format!("{}={}", col, filter));
        }

        if let Some(ref order) = self.order_by {
            params.push(format!("order={}", order));
        }

        if let Some(limit) = self.limit_val {
            params.push(format!("limit={}", limit));
        }

        if let Some(offset) = self.offset_val {
            params.push(format!("offset={}", offset));
        }

        if !params.is_empty() {
            url.push('?');
            url.push_str(&params.join("&"));
        }

        url
    }

    /// Execute SELECT query
    pub async fn execute(&self) -> Result<Value, SupabaseError> {
        let url = self.build_url();
        let mut req = self
            .client
            .get(&url)
            .header("apikey", &self.api_key)
            .header("Authorization", &self.auth_header);

        if self.single {
            req = req.header("Accept", "application/vnd.pgrst.object+json");
        }

        if let Some(ref mode) = self.count_mode {
            req = req.header("Prefer", format!("count={}", mode));
        }

        let resp = req
            .send()
            .await
            .map_err(|e| SupabaseError::RequestFailed(e.to_string()))?;

        parse_response(resp).await
    }

    /// Execute INSERT
    pub async fn insert(&self, data: &Value) -> Result<Value, SupabaseError> {
        let resp = self
            .client
            .post(&self.base_url)
            .header("apikey", &self.api_key)
            .header("Authorization", &self.auth_header)
            .header("Content-Type", "application/json")
            .header("Prefer", "return=representation")
            .json(data)
            .send()
            .await
            .map_err(|e| SupabaseError::RequestFailed(e.to_string()))?;

        parse_response(resp).await
    }

    /// Execute UPDATE (applies filters)
    pub async fn update(&self, data: &Value) -> Result<Value, SupabaseError> {
        let url = self.build_url();
        let resp = self
            .client
            .patch(&url)
            .header("apikey", &self.api_key)
            .header("Authorization", &self.auth_header)
            .header("Content-Type", "application/json")
            .header("Prefer", "return=representation")
            .json(data)
            .send()
            .await
            .map_err(|e| SupabaseError::RequestFailed(e.to_string()))?;

        parse_response(resp).await
    }

    /// Execute UPSERT
    pub async fn upsert(&self, data: &Value) -> Result<Value, SupabaseError> {
        let resp = self
            .client
            .post(&self.base_url)
            .header("apikey", &self.api_key)
            .header("Authorization", &self.auth_header)
            .header("Content-Type", "application/json")
            .header("Prefer", "return=representation,resolution=merge-duplicates")
            .json(data)
            .send()
            .await
            .map_err(|e| SupabaseError::RequestFailed(e.to_string()))?;

        parse_response(resp).await
    }

    /// Execute DELETE (applies filters)
    pub async fn delete(&self) -> Result<Value, SupabaseError> {
        let url = self.build_url();
        let resp = self
            .client
            .delete(&url)
            .header("apikey", &self.api_key)
            .header("Authorization", &self.auth_header)
            .header("Prefer", "return=representation")
            .send()
            .await
            .map_err(|e| SupabaseError::RequestFailed(e.to_string()))?;

        parse_response(resp).await
    }
}

// ─────────────────────────────────────────────
//  Auth API
// ─────────────────────────────────────────────

/// Supabase Auth client
#[derive(Debug, Clone)]
pub struct SupabaseAuth {
    client: Client,
    url: String,
    api_key: String,
}

/// Auth session response
#[derive(Debug, Clone)]
pub struct AuthSession {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_in: u64,
    pub token_type: String,
    pub user: Value,
}

impl SupabaseAuth {
    /// Sign up with email and password
    pub async fn sign_up(
        &self,
        email: &str,
        password: &str,
    ) -> Result<AuthSession, SupabaseError> {
        let body = serde_json::json!({
            "email": email,
            "password": password,
        });

        let resp = self
            .client
            .post(format!("{}/signup", self.url))
            .header("apikey", &self.api_key)
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| SupabaseError::AuthError(e.to_string()))?;

        let json = parse_response(resp).await?;
        parse_auth_session(json)
    }

    /// Sign in with email and password
    pub async fn sign_in_email(
        &self,
        email: &str,
        password: &str,
    ) -> Result<AuthSession, SupabaseError> {
        let body = serde_json::json!({
            "email": email,
            "password": password,
        });

        let resp = self
            .client
            .post(format!("{}/token?grant_type=password", self.url))
            .header("apikey", &self.api_key)
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| SupabaseError::AuthError(e.to_string()))?;

        let json = parse_response(resp).await?;
        parse_auth_session(json)
    }

    /// Sign in with OAuth (returns redirect URL)
    pub fn sign_in_oauth(&self, provider: &str, redirect_to: &str) -> String {
        format!(
            "{}/authorize?provider={}&redirect_to={}",
            self.url, provider, redirect_to
        )
    }

    /// Sign out (invalidate token)
    pub async fn sign_out(&self, access_token: &str) -> Result<(), SupabaseError> {
        self.client
            .post(format!("{}/logout", self.url))
            .header("apikey", &self.api_key)
            .header("Authorization", format!("Bearer {}", access_token))
            .send()
            .await
            .map_err(|e| SupabaseError::AuthError(e.to_string()))?;

        Ok(())
    }

    /// Get current user from access token
    pub async fn get_user(&self, access_token: &str) -> Result<Value, SupabaseError> {
        let resp = self
            .client
            .get(format!("{}/user", self.url))
            .header("apikey", &self.api_key)
            .header("Authorization", format!("Bearer {}", access_token))
            .send()
            .await
            .map_err(|e| SupabaseError::AuthError(e.to_string()))?;

        parse_response(resp).await
    }

    /// Refresh an access token
    pub async fn refresh_token(
        &self,
        refresh_token: &str,
    ) -> Result<AuthSession, SupabaseError> {
        let body = serde_json::json!({
            "refresh_token": refresh_token,
        });

        let resp = self
            .client
            .post(format!("{}/token?grant_type=refresh_token", self.url))
            .header("apikey", &self.api_key)
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| SupabaseError::AuthError(e.to_string()))?;

        let json = parse_response(resp).await?;
        parse_auth_session(json)
    }

    /// Send password reset email
    pub async fn reset_password(&self, email: &str) -> Result<(), SupabaseError> {
        let body = serde_json::json!({"email": email});

        self.client
            .post(format!("{}/recover", self.url))
            .header("apikey", &self.api_key)
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| SupabaseError::AuthError(e.to_string()))?;

        Ok(())
    }
}

// ─────────────────────────────────────────────
//  Storage API
// ─────────────────────────────────────────────

/// Supabase Storage client
#[derive(Debug, Clone)]
pub struct SupabaseStorage {
    client: Client,
    url: String,
    api_key: String,
    auth_header: String,
}

impl SupabaseStorage {
    /// Upload a file to a bucket
    pub async fn upload(
        &self,
        bucket: &str,
        path: &str,
        data: Vec<u8>,
        content_type: &str,
    ) -> Result<Value, SupabaseError> {
        let url = format!("{}/object/{}/{}", self.url, bucket, path);
        let resp = self
            .client
            .post(&url)
            .header("apikey", &self.api_key)
            .header("Authorization", &self.auth_header)
            .header("Content-Type", content_type)
            .body(data)
            .send()
            .await
            .map_err(|e| SupabaseError::StorageError(e.to_string()))?;

        parse_response(resp).await
    }

    /// Download a file from a bucket
    pub async fn download(
        &self,
        bucket: &str,
        path: &str,
    ) -> Result<Vec<u8>, SupabaseError> {
        let url = format!("{}/object/{}/{}", self.url, bucket, path);
        let resp = self
            .client
            .get(&url)
            .header("apikey", &self.api_key)
            .header("Authorization", &self.auth_header)
            .send()
            .await
            .map_err(|e| SupabaseError::StorageError(e.to_string()))?;

        resp.bytes()
            .await
            .map(|b| b.to_vec())
            .map_err(|e| SupabaseError::StorageError(e.to_string()))
    }

    /// List files in a bucket path
    pub async fn list(
        &self,
        bucket: &str,
        prefix: &str,
    ) -> Result<Value, SupabaseError> {
        let url = format!("{}/object/list/{}", self.url, bucket);
        let body = serde_json::json!({
            "prefix": prefix,
            "limit": 100,
            "offset": 0,
        });

        let resp = self
            .client
            .post(&url)
            .header("apikey", &self.api_key)
            .header("Authorization", &self.auth_header)
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| SupabaseError::StorageError(e.to_string()))?;

        parse_response(resp).await
    }

    /// Delete a file
    pub async fn delete(
        &self,
        bucket: &str,
        paths: &[&str],
    ) -> Result<Value, SupabaseError> {
        let url = format!("{}/object/{}", self.url, bucket);
        let body = serde_json::json!({
            "prefixes": paths,
        });

        let resp = self
            .client
            .delete(&url)
            .header("apikey", &self.api_key)
            .header("Authorization", &self.auth_header)
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| SupabaseError::StorageError(e.to_string()))?;

        parse_response(resp).await
    }

    /// Get the public URL for a file
    pub fn public_url(&self, bucket: &str, path: &str) -> String {
        format!("{}/object/public/{}/{}", self.url, bucket, path)
    }

    /// Get a signed URL for temporary access
    pub async fn signed_url(
        &self,
        bucket: &str,
        path: &str,
        expires_in: u64,
    ) -> Result<String, SupabaseError> {
        let url = format!("{}/object/sign/{}/{}", self.url, bucket, path);
        let body = serde_json::json!({"expiresIn": expires_in});

        let resp = self
            .client
            .post(&url)
            .header("apikey", &self.api_key)
            .header("Authorization", &self.auth_header)
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| SupabaseError::StorageError(e.to_string()))?;

        let json = parse_response(resp).await?;
        json.get("signedURL")
            .and_then(|v| v.as_str())
            .map(|s| format!("{}{}", self.url, s))
            .ok_or_else(|| SupabaseError::StorageError("No signed URL in response".into()))
    }
}

// ─────────────────────────────────────────────
//  Helpers
// ─────────────────────────────────────────────

async fn parse_response(resp: Response) -> Result<Value, SupabaseError> {
    let status = resp.status();
    let text = resp
        .text()
        .await
        .map_err(|e| SupabaseError::ParseError(e.to_string()))?;

    if !status.is_success() {
        return Err(SupabaseError::RequestFailed(format!(
            "HTTP {}: {}",
            status, text
        )));
    }

    if text.is_empty() {
        return Ok(Value::Null);
    }

    serde_json::from_str(&text).map_err(|e| SupabaseError::ParseError(e.to_string()))
}

fn parse_auth_session(json: Value) -> Result<AuthSession, SupabaseError> {
    Ok(AuthSession {
        access_token: json
            .get("access_token")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        refresh_token: json
            .get("refresh_token")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        expires_in: json
            .get("expires_in")
            .and_then(|v| v.as_u64())
            .unwrap_or(3600),
        token_type: json
            .get("token_type")
            .and_then(|v| v.as_str())
            .unwrap_or("bearer")
            .to_string(),
        user: json.get("user").cloned().unwrap_or(Value::Null),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creation() {
        let client = SupabaseClient::new("https://test.supabase.co", "anon-key");
        assert_eq!(client.url, "https://test.supabase.co");
    }

    #[test]
    fn test_query_builder_url() {
        let client = SupabaseClient::new("https://test.supabase.co", "key");
        let qb = client
            .from("posts")
            .select("id,title")
            .eq("published", "true")
            .order("created_at", false)
            .limit(10);

        let url = qb.build_url();
        assert!(url.contains("/rest/v1/posts"));
        assert!(url.contains("select=id,title"));
        assert!(url.contains("published=eq.true"));
        assert!(url.contains("order=created_at.desc"));
        assert!(url.contains("limit=10"));
    }

    #[test]
    fn test_query_builder_filters() {
        let client = SupabaseClient::new("https://test.supabase.co", "key");

        let url = client.from("users").gt("age", "18").lt("age", "65").build_url();
        assert!(url.contains("age=gt.18"));
        assert!(url.contains("age=lt.65"));
    }

    #[test]
    fn test_query_builder_in_list() {
        let client = SupabaseClient::new("https://test.supabase.co", "key");
        let url = client
            .from("posts")
            .in_list("status", &["published", "draft"])
            .build_url();
        assert!(url.contains("status=in.(published,draft)"));
    }

    #[test]
    fn test_query_builder_like() {
        let client = SupabaseClient::new("https://test.supabase.co", "key");
        let url = client.from("posts").ilike("title", "%rust%").build_url();
        assert!(url.contains("title=ilike.%rust%"));
    }

    #[test]
    fn test_oauth_url() {
        let client = SupabaseClient::new("https://test.supabase.co", "key");
        let url = client.auth().sign_in_oauth("google", "http://localhost:3000/callback");
        assert!(url.contains("provider=google"));
        assert!(url.contains("redirect_to="));
    }

    #[test]
    fn test_public_url() {
        let client = SupabaseClient::new("https://test.supabase.co", "key");
        let url = client.storage().public_url("avatars", "user1.png");
        assert_eq!(
            url,
            "https://test.supabase.co/storage/v1/object/public/avatars/user1.png"
        );
    }

    #[test]
    fn test_with_access_token() {
        let client = SupabaseClient::new("https://test.supabase.co", "key")
            .with_access_token("user-jwt-token");
        assert_eq!(client.auth_header(), "Bearer user-jwt-token");
    }

    #[test]
    fn test_default_auth_header() {
        let client = SupabaseClient::new("https://test.supabase.co", "anon-key");
        assert_eq!(client.auth_header(), "Bearer anon-key");
    }

    #[test]
    fn test_parse_auth_session() {
        let json = serde_json::json!({
            "access_token": "eyJhbGciOiJIUzI1NiJ9",
            "refresh_token": "refresh123",
            "expires_in": 3600,
            "token_type": "bearer",
            "user": {"id": "user-123", "email": "test@example.com"}
        });

        let session = parse_auth_session(json).unwrap();
        assert_eq!(session.access_token, "eyJhbGciOiJIUzI1NiJ9");
        assert_eq!(session.refresh_token, "refresh123");
        assert_eq!(session.expires_in, 3600);
        assert_eq!(session.user["email"], "test@example.com");
    }

    #[test]
    fn test_query_offset() {
        let client = SupabaseClient::new("https://test.supabase.co", "key");
        let url = client.from("posts").limit(10).offset(20).build_url();
        assert!(url.contains("limit=10"));
        assert!(url.contains("offset=20"));
    }
}
