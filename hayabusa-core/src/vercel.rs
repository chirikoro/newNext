//! Vercel Build Output API v3 Adapter for Hayabusa.
//!
//! Generates `.vercel/output/` directory structure so Hayabusa apps
//! can deploy to Vercel with zero additional configuration.
//!
//! ## How it works
//!
//! Vercel's [Build Output API v3](https://vercel.com/docs/build-output-api/v3)
//! allows any framework to deploy by writing files to `.vercel/output/`:
//!
//! - `config.json` — routing rules, redirects, rewrites, headers
//! - `static/` — pre-rendered HTML, CSS, JS, images
//! - `functions/` — serverless function handlers (Rust binary as custom runtime)
//!
//! ## Usage
//! ```ignore
//! use hayabusa_core::prelude::*;
//!
//! let adapter = VercelAdapter::new()
//!     .project_dir(".")
//!     .region("hnd1")  // Tokyo
//!     .memory(256)
//!     .max_duration(10);
//!
//! // Generate .vercel/output/ from hayabusa.toml config
//! adapter.build(&config).unwrap();
//! ```

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::config_routes::{AppConfig, RenderModeConfig};

// ─── Vercel Adapter ─────────────────────────────────────────

/// Main Vercel deployment adapter
#[derive(Debug, Clone)]
pub struct VercelAdapter {
    /// Project root directory
    pub project_dir: PathBuf,
    /// Preferred deployment region (e.g., "hnd1" for Tokyo)
    pub region: Option<String>,
    /// Memory limit in MB for serverless functions (default: 256)
    pub memory: u32,
    /// Max execution duration in seconds (default: 10)
    pub max_duration: u32,
    /// Whether to use Vercel Image Optimization
    pub image_optimization: bool,
    /// Custom environment variables
    pub env: HashMap<String, String>,
    /// Target binary name (from Cargo.toml)
    pub binary_name: String,
    /// Enable ISR (uses Vercel's prerender config)
    pub enable_isr: bool,
    /// Enable Edge Middleware via WASM
    pub enable_edge_middleware: bool,
    /// Vercel framework preset identifier
    pub framework: Option<String>,
}

impl Default for VercelAdapter {
    fn default() -> Self {
        Self {
            project_dir: PathBuf::from("."),
            region: None,
            memory: 256,
            max_duration: 10,
            image_optimization: true,
            env: HashMap::new(),
            binary_name: "hayabusa-app".to_string(),
            enable_isr: true,
            enable_edge_middleware: false,
            framework: None,
        }
    }
}

impl VercelAdapter {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn project_dir(mut self, dir: impl Into<PathBuf>) -> Self {
        self.project_dir = dir.into();
        self
    }

    pub fn region(mut self, region: impl Into<String>) -> Self {
        self.region = Some(region.into());
        self
    }

    pub fn memory(mut self, mb: u32) -> Self {
        self.memory = mb;
        self
    }

    pub fn max_duration(mut self, seconds: u32) -> Self {
        self.max_duration = seconds;
        self
    }

    pub fn binary_name(mut self, name: impl Into<String>) -> Self {
        self.binary_name = name.into();
        self
    }

    pub fn env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.env.insert(key.into(), value.into());
        self
    }

    pub fn image_optimization(mut self, enabled: bool) -> Self {
        self.image_optimization = enabled;
        self
    }

    /// Generate the `.vercel/output/` directory structure.
    ///
    /// This produces:
    /// - `.vercel/output/config.json`
    /// - `.vercel/output/static/` (SSG pages + public assets)
    /// - `.vercel/output/functions/` (SSR/API serverless functions)
    pub fn build(&self, config: &AppConfig) -> Result<VercelBuildOutput, VercelError> {
        let output_dir = self.project_dir.join(".vercel").join("output");

        // Classify routes
        let mut static_routes = Vec::new();
        let mut serverless_routes = Vec::new();
        let mut isr_routes = Vec::new();

        for route in &config.routes {
            match route.mode {
                RenderModeConfig::Ssg => {
                    static_routes.push(route.path.clone());
                }
                RenderModeConfig::Isr => {
                    isr_routes.push(IrsRouteConfig {
                        path: route.path.clone(),
                        revalidate: route.revalidate.unwrap_or(60),
                    });
                }
                RenderModeConfig::Ssr | RenderModeConfig::Streaming => {
                    serverless_routes.push(route.path.clone());
                }
            }
        }

        // Generate config.json
        let vercel_config = self.generate_config(config, &isr_routes);

        // Generate function configs
        let functions = self.generate_functions(&serverless_routes, &isr_routes);

        // Generate vercel.json (project-level)
        let vercel_json = self.generate_vercel_json();

        // Generate Dockerfile for Rust build
        let dockerfile = self.generate_build_script();

        Ok(VercelBuildOutput {
            output_dir,
            config: vercel_config,
            static_routes,
            serverless_routes,
            isr_routes,
            functions,
            vercel_json,
            dockerfile,
        })
    }

    /// Generate `.vercel/output/config.json`
    fn generate_config(
        &self,
        config: &AppConfig,
        isr_routes: &[IrsRouteConfig],
    ) -> String {
        let mut json = String::from("{\n  \"version\": 3");

        // Routes
        json.push_str(",\n  \"routes\": [");
        let mut routes_parts = Vec::new();

        // Static asset handling
        routes_parts.push(
            "    { \"src\": \"/static/(.*)\", \"headers\": { \"Cache-Control\": \"public, max-age=31536000, immutable\" } }".to_string()
        );

        // Redirects from config
        for redirect in &config.redirects {
            let permanent = if redirect.permanent { "true" } else { "false" };
            routes_parts.push(format!(
                "    {{ \"src\": \"{}\", \"status\": {}, \"headers\": {{ \"Location\": \"{}\" }} }}",
                escape_route_pattern(&redirect.from),
                redirect.status,
                redirect.to
            ));
            let _ = permanent; // used conceptually for status code
        }

        // Rewrites from config
        for rewrite in &config.rewrites {
            routes_parts.push(format!(
                "    {{ \"src\": \"{}\", \"dest\": \"{}\" }}",
                escape_route_pattern(&rewrite.from),
                rewrite.to
            ));
        }

        // Custom headers from config
        for header in &config.headers {
            let header_obj: Vec<String> = header
                .values
                .iter()
                .map(|(k, v)| format!("\"{}\": \"{}\"", k, v))
                .collect();
            routes_parts.push(format!(
                "    {{ \"src\": \"{}\", \"headers\": {{ {} }} }}",
                escape_route_pattern(&header.pattern),
                header_obj.join(", ")
            ));
        }

        // SSR/Streaming routes → serverless functions
        for route in &config.routes {
            match route.mode {
                RenderModeConfig::Ssr | RenderModeConfig::Streaming => {
                    let func_name = path_to_function_name(&route.path);
                    routes_parts.push(format!(
                        "    {{ \"src\": \"{}\", \"dest\": \"/api/{}\" }}",
                        escape_route_pattern(&route.path),
                        func_name
                    ));
                }
                _ => {}
            }
        }

        json.push_str(&format!("\n{}\n  ]", routes_parts.join(",\n")));

        // ISR / Prerender config
        if !isr_routes.is_empty() && self.enable_isr {
            json.push_str(",\n  \"overrides\": {");
            let overrides: Vec<String> = isr_routes
                .iter()
                .map(|r| {
                    let func_name = path_to_function_name(&r.path);
                    format!(
                        "    \"{}.func\": {{ \"path\": \"{}\", \"contentType\": \"text/html; charset=utf-8\" }}",
                        func_name,
                        r.path.trim_start_matches('/')
                    )
                })
                .collect();
            json.push_str(&format!("\n{}\n  }}", overrides.join(",\n")));
        }

        // Image optimization
        if self.image_optimization {
            json.push_str(",\n  \"images\": {");
            json.push_str("\n    \"sizes\": [640, 750, 828, 1080, 1200, 1920, 2048, 3840],");
            json.push_str("\n    \"domains\": [],");
            json.push_str("\n    \"formats\": [\"image/avif\", \"image/webp\"],");
            json.push_str("\n    \"minimumCacheTTL\": 60");
            json.push_str("\n  }");
        }

        json.push_str("\n}");
        json
    }

    /// Generate serverless function configurations
    fn generate_functions(
        &self,
        serverless_routes: &[String],
        isr_routes: &[IrsRouteConfig],
    ) -> Vec<VercelFunction> {
        let mut functions = Vec::new();

        // SSR routes as serverless functions
        for route in serverless_routes {
            let func_name = path_to_function_name(route);
            functions.push(VercelFunction {
                name: func_name,
                runtime: "provided.al2".to_string(), // Amazon Linux 2 custom runtime
                handler: "bootstrap".to_string(),
                memory: self.memory,
                max_duration: self.max_duration,
                region: self.region.clone(),
                config_json: self.generate_function_config(None),
                entry_point: format!("fn/{}", path_to_function_name(route)),
            });
        }

        // ISR routes: serverless function + prerender config
        for isr in isr_routes {
            let func_name = path_to_function_name(&isr.path);
            functions.push(VercelFunction {
                name: func_name.clone(),
                runtime: "provided.al2".to_string(),
                handler: "bootstrap".to_string(),
                memory: self.memory,
                max_duration: self.max_duration,
                region: self.region.clone(),
                config_json: self.generate_function_config(Some(isr.revalidate)),
                entry_point: format!("fn/{}", func_name),
            });
        }

        functions
    }

    /// Generate `.vc-config.json` for a single function
    fn generate_function_config(&self, isr_revalidate: Option<u64>) -> String {
        let mut json = String::from("{\n");
        json.push_str(&format!("  \"runtime\": \"provided.al2\",\n"));
        json.push_str(&format!("  \"handler\": \"bootstrap\",\n"));
        json.push_str(&format!("  \"memory\": {},\n", self.memory));
        json.push_str(&format!("  \"maxDuration\": {}", self.max_duration));

        if let Some(ref region) = self.region {
            json.push_str(&format!(",\n  \"regions\": [\"{}\"]", region));
        }

        if let Some(revalidate) = isr_revalidate {
            json.push_str(&format!(",\n  \"supportsResponseStreaming\": true"));
            // ISR prerender config is separate file
            let _ = revalidate;
        }

        if !self.env.is_empty() {
            let env_entries: Vec<String> = self
                .env
                .iter()
                .map(|(k, v)| format!("    \"{}\": \"{}\"", k, v))
                .collect();
            json.push_str(&format!(
                ",\n  \"environment\": {{\n{}\n  }}",
                env_entries.join(",\n")
            ));
        }

        json.push_str("\n}");
        json
    }

    /// Generate ISR prerender config file content
    pub fn generate_prerender_config(revalidate: u64) -> String {
        format!(
            "{{\n  \"expiration\": {},\n  \"allowQuery\": [\"__nextDataReq\"],\n  \"group\": 1,\n  \"bypassToken\": null\n}}",
            revalidate
        )
    }

    /// Generate `vercel.json` for the project root
    fn generate_vercel_json(&self) -> String {
        let mut json = String::from("{\n");
        json.push_str("  \"buildCommand\": \"cargo build --release --target x86_64-unknown-linux-gnu\",\n");
        json.push_str("  \"outputDirectory\": \".vercel/output\",\n");

        if let Some(ref framework) = self.framework {
            json.push_str(&format!("  \"framework\": \"{}\",\n", framework));
        }

        json.push_str("  \"installCommand\": \"curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y && . $HOME/.cargo/env\",\n");

        // Functions config
        json.push_str("  \"functions\": {\n");
        json.push_str(&format!("    \"api/**\": {{\n"));
        json.push_str(&format!("      \"memory\": {},\n", self.memory));
        json.push_str(&format!("      \"maxDuration\": {}\n", self.max_duration));
        json.push_str("    }\n");
        json.push_str("  },\n");

        // Regions
        if let Some(ref region) = self.region {
            json.push_str(&format!("  \"regions\": [\"{}\"],\n", region));
        }

        // Headers for static assets
        json.push_str("  \"headers\": [\n");
        json.push_str("    {\n");
        json.push_str("      \"source\": \"/static/(.*)\",\n");
        json.push_str("      \"headers\": [\n");
        json.push_str("        { \"key\": \"Cache-Control\", \"value\": \"public, max-age=31536000, immutable\" }\n");
        json.push_str("      ]\n");
        json.push_str("    }\n");
        json.push_str("  ]\n");

        json.push_str("}");
        json
    }

    /// Generate a build script / Dockerfile for compiling Rust on Vercel
    fn generate_build_script(&self) -> String {
        format!(
r#"#!/bin/bash
# Hayabusa Vercel Build Script
# This script compiles the Rust binary for deployment on Vercel.
#
# Vercel runs builds on Amazon Linux 2 (x86_64).
# The binary is placed in .vercel/output/functions/ as "bootstrap".

set -euo pipefail

echo "==> Installing Rust toolchain..."
if ! command -v cargo &> /dev/null; then
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source "$HOME/.cargo/env"
fi

echo "==> Building {binary} (release)..."
cargo build --release --bin {binary}

echo "==> Preparing .vercel/output/ structure..."
OUTDIR=".vercel/output"
mkdir -p "$OUTDIR/static"
mkdir -p "$OUTDIR/functions/api/ssr.func"

# Copy static assets
if [ -d "public" ]; then
    cp -r public/* "$OUTDIR/static/" 2>/dev/null || true
fi

# Copy pre-rendered SSG pages
if [ -d "out" ]; then
    cp -r out/* "$OUTDIR/static/" 2>/dev/null || true
fi

# Copy binary as bootstrap (custom runtime convention)
cp "target/release/{binary}" "$OUTDIR/functions/api/ssr.func/bootstrap"
chmod +x "$OUTDIR/functions/api/ssr.func/bootstrap"

# Write function config
cat > "$OUTDIR/functions/api/ssr.func/.vc-config.json" <<'VCEOF'
{{
  "runtime": "provided.al2",
  "handler": "bootstrap",
  "memory": {memory},
  "maxDuration": {max_duration},
  "supportsResponseStreaming": true,
  "launcherType": "Nodejs"
}}
VCEOF

echo "==> Build complete!"
echo "    Static files: $OUTDIR/static/"
echo "    Functions:    $OUTDIR/functions/"
"#,
            binary = self.binary_name,
            memory = self.memory,
            max_duration = self.max_duration,
        )
    }

    /// Write all build output files to disk
    pub fn write_output(&self, output: &VercelBuildOutput) -> Result<(), VercelError> {
        let out = &output.output_dir;

        // Create directory structure
        create_dir_all(out)?;
        create_dir_all(&out.join("static"))?;
        create_dir_all(&out.join("functions"))?;

        // Write config.json
        write_file(&out.join("config.json"), &output.config)?;

        // Write function configs
        for func in &output.functions {
            let func_dir = out.join("functions").join(format!("{}.func", func.name));
            create_dir_all(&func_dir)?;
            write_file(&func_dir.join(".vc-config.json"), &func.config_json)?;
        }

        // Write vercel.json at project root
        write_file(
            &self.project_dir.join("vercel.json"),
            &output.vercel_json,
        )?;

        // Write build script
        write_file(
            &self.project_dir.join("vercel-build.sh"),
            &output.dockerfile,
        )?;

        Ok(())
    }
}

// ─── Build Output Types ─────────────────────────────────────

/// Complete Vercel build output
#[derive(Debug, Clone)]
pub struct VercelBuildOutput {
    pub output_dir: PathBuf,
    pub config: String,
    pub static_routes: Vec<String>,
    pub serverless_routes: Vec<String>,
    pub isr_routes: Vec<IrsRouteConfig>,
    pub functions: Vec<VercelFunction>,
    pub vercel_json: String,
    pub dockerfile: String,
}

/// A serverless function definition
#[derive(Debug, Clone)]
pub struct VercelFunction {
    pub name: String,
    pub runtime: String,
    pub handler: String,
    pub memory: u32,
    pub max_duration: u32,
    pub region: Option<String>,
    pub config_json: String,
    pub entry_point: String,
}

/// ISR route configuration
#[derive(Debug, Clone)]
pub struct IrsRouteConfig {
    pub path: String,
    pub revalidate: u64,
}

/// Vercel adapter errors
#[derive(Debug, Clone)]
pub enum VercelError {
    IoError(String),
    ConfigError(String),
}

impl std::fmt::Display for VercelError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VercelError::IoError(msg) => write!(f, "Vercel IO error: {}", msg),
            VercelError::ConfigError(msg) => write!(f, "Vercel config error: {}", msg),
        }
    }
}

// ─── Vercel Edge Middleware ─────────────────────────────────

/// Configuration for Vercel Edge Middleware (WASM-based)
#[derive(Debug, Clone)]
pub struct VercelEdgeMiddleware {
    /// Route patterns this middleware applies to
    pub matchers: Vec<String>,
    /// Middleware name
    pub name: String,
}

impl VercelEdgeMiddleware {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            matchers: Vec::new(),
            name: name.into(),
        }
    }

    pub fn matcher(mut self, pattern: impl Into<String>) -> Self {
        self.matchers.push(pattern.into());
        self
    }

    /// Generate the middleware config JSON
    pub fn config_json(&self) -> String {
        let matchers: Vec<String> = self
            .matchers
            .iter()
            .map(|m| format!("    {{ \"source\": \"{}\" }}", m))
            .collect();
        format!(
            "{{\n  \"name\": \"{}\",\n  \"matchers\": [\n{}\n  ]\n}}",
            self.name,
            matchers.join(",\n")
        )
    }
}

// ─── Vercel Cron Jobs ───────────────────────────────────────

/// Vercel Cron Job configuration
#[derive(Debug, Clone)]
pub struct VercelCron {
    pub path: String,
    pub schedule: String,
}

impl VercelCron {
    pub fn new(path: impl Into<String>, schedule: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            schedule: schedule.into(),
        }
    }

    /// Generate cron config for vercel.json
    pub fn to_json(&self) -> String {
        format!(
            "{{ \"path\": \"{}\", \"schedule\": \"{}\" }}",
            self.path, self.schedule
        )
    }
}

/// Generate the `crons` section for vercel.json
pub fn crons_json(crons: &[VercelCron]) -> String {
    if crons.is_empty() {
        return "[]".to_string();
    }
    let entries: Vec<String> = crons.iter().map(|c| format!("    {}", c.to_json())).collect();
    format!("[\n{}\n  ]", entries.join(",\n"))
}

// ─── Vercel KV / Blob / Postgres helpers ────────────────────

/// Helper to generate environment variable references for Vercel integrations.
/// These are set automatically when you add Vercel KV, Blob, or Postgres from the dashboard.
#[derive(Debug, Clone)]
pub struct VercelStorage;

impl VercelStorage {
    /// Environment variable names for Vercel KV (Redis-compatible)
    pub fn kv_env_vars() -> Vec<&'static str> {
        vec![
            "KV_URL",
            "KV_REST_API_URL",
            "KV_REST_API_TOKEN",
            "KV_REST_API_READ_ONLY_TOKEN",
        ]
    }

    /// Environment variable names for Vercel Postgres
    pub fn postgres_env_vars() -> Vec<&'static str> {
        vec![
            "POSTGRES_URL",
            "POSTGRES_PRISMA_URL",
            "POSTGRES_URL_NON_POOLING",
            "POSTGRES_USER",
            "POSTGRES_HOST",
            "POSTGRES_PASSWORD",
            "POSTGRES_DATABASE",
        ]
    }

    /// Environment variable names for Vercel Blob
    pub fn blob_env_vars() -> Vec<&'static str> {
        vec!["BLOB_READ_WRITE_TOKEN"]
    }

    /// Generate a helper snippet to read Vercel KV URL from env
    pub fn kv_connection_snippet() -> &'static str {
        r#"// Vercel KV (Redis-compatible) connection
let kv_url = std::env::var("KV_REST_API_URL").expect("KV_REST_API_URL not set");
let kv_token = std::env::var("KV_REST_API_TOKEN").expect("KV_REST_API_TOKEN not set");"#
    }

    /// Generate a helper snippet to read Vercel Postgres URL from env
    pub fn postgres_connection_snippet() -> &'static str {
        r#"// Vercel Postgres connection
let database_url = std::env::var("POSTGRES_URL").expect("POSTGRES_URL not set");"#
    }
}

// ─── Helper Functions ───────────────────────────────────────

/// Convert a route path to a valid function name
/// e.g., "/blog/:slug" -> "blog-slug", "/" -> "index"
fn path_to_function_name(path: &str) -> String {
    if path == "/" {
        return "index".to_string();
    }
    path.trim_start_matches('/')
        .replace(':', "")
        .replace('/', "-")
        .replace('*', "catch")
}

/// Escape route pattern for Vercel's regex-based routing
/// e.g., "/blog/:slug" -> "/blog/([^/]+)"
fn escape_route_pattern(pattern: &str) -> String {
    let segments: Vec<&str> = pattern.split('/').filter(|s| !s.is_empty()).collect();
    if segments.is_empty() {
        return "/".to_string();
    }
    let converted: Vec<String> = segments
        .iter()
        .map(|s| {
            if s.starts_with(':') {
                if s.ends_with('*') {
                    "(.*)".to_string()
                } else {
                    "([^/]+)".to_string()
                }
            } else if *s == "*" {
                "(.*)".to_string()
            } else {
                s.to_string()
            }
        })
        .collect();
    format!("/{}", converted.join("/"))
}

fn create_dir_all(path: &Path) -> Result<(), VercelError> {
    std::fs::create_dir_all(path)
        .map_err(|e| VercelError::IoError(format!("Failed to create {}: {}", path.display(), e)))
}

fn write_file(path: &Path, content: &str) -> Result<(), VercelError> {
    std::fs::write(path, content)
        .map_err(|e| VercelError::IoError(format!("Failed to write {}: {}", path.display(), e)))
}

// ─── Tests ──────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config_routes::AppConfig;

    const TEST_TOML: &str = r#"
[app]
name = "Test App"
port = 3000

[[routes]]
path = "/"
template = "index.html"
mode = "ssg"

[[routes]]
path = "/about"
template = "about.html"
mode = "ssg"

[[routes]]
path = "/blog/:slug"
template = "blog.html"
mode = "isr"
revalidate = 60

[[routes]]
path = "/app/:path*"
template = "app.html"
mode = "ssr"

[[routes]]
path = "/api/data"
mode = "ssr"

[[redirects]]
from = "/old"
to = "/new"
status = 301

[[rewrites]]
from = "/proxy/:path"
to = "https://api.example.com/:path"

[[headers]]
pattern = "/api/*"
values = { "Access-Control-Allow-Origin" = "*" }
"#;

    fn test_config() -> AppConfig {
        AppConfig::from_toml(TEST_TOML).unwrap()
    }

    #[test]
    fn test_adapter_default() {
        let adapter = VercelAdapter::new();
        assert_eq!(adapter.memory, 256);
        assert_eq!(adapter.max_duration, 10);
        assert!(adapter.image_optimization);
    }

    #[test]
    fn test_adapter_builder() {
        let adapter = VercelAdapter::new()
            .region("hnd1")
            .memory(512)
            .max_duration(30)
            .binary_name("my-app")
            .env("DATABASE_URL", "postgres://localhost/db");

        assert_eq!(adapter.region, Some("hnd1".to_string()));
        assert_eq!(adapter.memory, 512);
        assert_eq!(adapter.max_duration, 30);
        assert_eq!(adapter.binary_name, "my-app");
        assert_eq!(adapter.env.get("DATABASE_URL").unwrap(), "postgres://localhost/db");
    }

    #[test]
    fn test_build_output_routes() {
        let adapter = VercelAdapter::new();
        let config = test_config();
        let output = adapter.build(&config).unwrap();

        // "/" and "/about" are SSG
        assert_eq!(output.static_routes, vec!["/", "/about"]);
        // "/app/:path*" and "/api/data" are SSR
        assert_eq!(output.serverless_routes, vec!["/app/:path*", "/api/data"]);
        // "/blog/:slug" is ISR
        assert_eq!(output.isr_routes.len(), 1);
        assert_eq!(output.isr_routes[0].path, "/blog/:slug");
        assert_eq!(output.isr_routes[0].revalidate, 60);
    }

    #[test]
    fn test_config_json_contains_routes() {
        let adapter = VercelAdapter::new();
        let config = test_config();
        let output = adapter.build(&config).unwrap();

        assert!(output.config.contains("\"version\": 3"));
        assert!(output.config.contains("\"routes\""));
        assert!(output.config.contains("\"images\""));
        // Redirects
        assert!(output.config.contains("/old"));
        // Rewrites
        assert!(output.config.contains("/proxy/"));
        // Headers
        assert!(output.config.contains("Access-Control-Allow-Origin"));
    }

    #[test]
    fn test_config_json_image_optimization() {
        let adapter = VercelAdapter::new().image_optimization(false);
        let config = test_config();
        let output = adapter.build(&config).unwrap();
        assert!(!output.config.contains("\"images\""));
    }

    #[test]
    fn test_functions_generated() {
        let adapter = VercelAdapter::new().region("hnd1");
        let config = test_config();
        let output = adapter.build(&config).unwrap();

        // SSR: app-pathcatch, api-data
        // ISR: blog-slug
        assert_eq!(output.functions.len(), 3);

        let names: Vec<&str> = output.functions.iter().map(|f| f.name.as_str()).collect();
        assert!(names.contains(&"app-pathcatch"));
        assert!(names.contains(&"api-data"));
        assert!(names.contains(&"blog-slug"));

        // All have region
        for func in &output.functions {
            assert_eq!(func.region, Some("hnd1".to_string()));
            assert!(func.config_json.contains("\"hnd1\""));
        }
    }

    #[test]
    fn test_function_config_with_env() {
        let adapter = VercelAdapter::new()
            .env("API_KEY", "secret123");
        let config_str = adapter.generate_function_config(None);
        assert!(config_str.contains("\"environment\""));
        assert!(config_str.contains("API_KEY"));
    }

    #[test]
    fn test_vercel_json() {
        let adapter = VercelAdapter::new().region("iad1");
        let config = test_config();
        let output = adapter.build(&config).unwrap();

        assert!(output.vercel_json.contains("buildCommand"));
        assert!(output.vercel_json.contains("cargo build --release"));
        assert!(output.vercel_json.contains("\"iad1\""));
    }

    #[test]
    fn test_build_script() {
        let adapter = VercelAdapter::new().binary_name("my-server");
        let config = test_config();
        let output = adapter.build(&config).unwrap();

        assert!(output.dockerfile.contains("my-server"));
        assert!(output.dockerfile.contains("cargo build --release"));
        assert!(output.dockerfile.contains(".vercel/output"));
        assert!(output.dockerfile.contains("bootstrap"));
    }

    #[test]
    fn test_path_to_function_name() {
        assert_eq!(path_to_function_name("/"), "index");
        assert_eq!(path_to_function_name("/blog/:slug"), "blog-slug");
        assert_eq!(path_to_function_name("/api/data"), "api-data");
        assert_eq!(path_to_function_name("/app/:path*"), "app-pathcatch");
    }

    #[test]
    fn test_escape_route_pattern() {
        assert_eq!(escape_route_pattern("/"), "/");
        assert_eq!(escape_route_pattern("/blog/:slug"), "/blog/([^/]+)");
        assert_eq!(escape_route_pattern("/api/:path*"), "/api/(.*)");
        assert_eq!(escape_route_pattern("/about"), "/about");
    }

    #[test]
    fn test_edge_middleware() {
        let mw = VercelEdgeMiddleware::new("auth")
            .matcher("/dashboard/(.*)")
            .matcher("/api/(.*)");
        let json = mw.config_json();
        assert!(json.contains("\"auth\""));
        assert!(json.contains("/dashboard/(.*)"));
        assert!(json.contains("/api/(.*)"));
    }

    #[test]
    fn test_cron_jobs() {
        let crons = vec![
            VercelCron::new("/api/cron/cleanup", "0 0 * * *"),
            VercelCron::new("/api/cron/sync", "*/15 * * * *"),
        ];
        let json = crons_json(&crons);
        assert!(json.contains("/api/cron/cleanup"));
        assert!(json.contains("0 0 * * *"));
        assert!(json.contains("*/15 * * * *"));
    }

    #[test]
    fn test_cron_empty() {
        assert_eq!(crons_json(&[]), "[]");
    }

    #[test]
    fn test_prerender_config() {
        let config = VercelAdapter::generate_prerender_config(60);
        assert!(config.contains("\"expiration\": 60"));
    }

    #[test]
    fn test_vercel_storage_env_vars() {
        assert!(VercelStorage::kv_env_vars().contains(&"KV_URL"));
        assert!(VercelStorage::postgres_env_vars().contains(&"POSTGRES_URL"));
        assert!(VercelStorage::blob_env_vars().contains(&"BLOB_READ_WRITE_TOKEN"));
    }

    #[test]
    fn test_output_dir() {
        let adapter = VercelAdapter::new().project_dir("/my/project");
        let config = test_config();
        let output = adapter.build(&config).unwrap();
        assert_eq!(output.output_dir, PathBuf::from("/my/project/.vercel/output"));
    }
}
