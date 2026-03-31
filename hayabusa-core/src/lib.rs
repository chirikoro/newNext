//! # Hayabusa (隼) - Rust Full-Stack Web Framework
//!
//! A high-performance full-stack web framework for Rust, inspired by Next.js.
//!
//! ## Features
//! - File-based routing with `app/` directory convention
//! - Server-Side Rendering (SSR)
//! - Static Site Generation (SSG)
//! - Incremental Static Regeneration (ISR)
//! - API routes
//! - Nested layouts
//! - Built-in middleware (compression, CORS, security headers)
//! - SEO-friendly head management
//! - Image optimization (srcset, WebP/AVIF, lazy loading, blur placeholder)
//! - Font optimization (font-display:swap, size-adjust, preload)
//! - Script loading strategies (defer, async, module, worker, afterInteractive)
//! - Navigation prefetch on hover/viewport
//!
//! ## Quick Start
//! ```ignore
//! use hayabusa_core::prelude::*;
//!
//! #[tokio::main]
//! async fn main() {
//!     HayabusaApp::new()
//!         .routes(my_routes())
//!         .port(3000)
//!         .serve()
//!         .await
//!         .unwrap();
//! }
//! ```

pub mod ab_test;
pub mod app;
pub mod auth;
pub mod background_job;
pub mod component;
pub mod config_routes;
pub mod critical_css;
pub mod csp;
pub mod data_loader;
pub mod deploy;
pub mod early_hints;
pub mod email;
pub mod error;
pub mod error_boundary;
pub mod file_upload;
pub mod font;
pub mod graphql;
pub mod head;
pub mod hot_reload;
pub mod i18n;
pub mod image;
pub mod interactivity;
pub mod layout;
pub mod logging;
pub mod markdown;
pub mod middleware;
pub mod middleware_chain;
pub mod mobile_pwa;
pub mod openai;
pub mod ppr;
pub mod rate_limit;
pub mod render;
pub mod router;
pub mod script;
pub mod seo;
pub mod server_action;
pub mod service_worker;
pub mod session;
pub mod sse;
pub mod state;
pub mod static_gen;
pub mod supabase;
pub mod template_engine;
pub mod testing;
pub mod validation;
pub mod vercel;
pub mod view_transition;
pub mod web_vitals;
pub mod websocket;

/// Prelude: import everything you need with `use hayabusa_core::prelude::*`
pub mod prelude {
    pub use crate::app::HayabusaApp;
    pub use crate::component::{
        HeadContext, LinkRel, PageHandler, PageRequest, RenderMode, RenderResult,
    };
    pub use crate::error::HayabusaError;
    pub use crate::font::{FontFormat, FontWeight, OptimizedFont};
    pub use crate::image::{ImagePriority, OptimizedImage};
    pub use crate::layout::{FnLayout, Layout, RootLayout};
    pub use crate::middleware::MiddlewareConfig;
    pub use crate::render::{
        check_etag, generate_etag, html_response, json_response, minify_html, render_page,
        render_streaming, suspense_placeholder, suspense_resolve,
    };
    pub use crate::router::{ApiMethod, RouteTable};
    pub use crate::script::{navigation_prefetch_script, OptimizedScript, ScriptStrategy};
    pub use crate::state::AppState;
    pub use crate::static_gen::StaticGenerator;

    // Server Actions
    pub use crate::server_action::{
        ActionRegistry, ActionResult, FormData, action_field, action_result_to_response,
        csrf_field, generate_csrf_token, validate_csrf_token,
    };

    // Partial Prerendering
    pub use crate::ppr::{PartialPage, dynamic_slot};

    // Per-route middleware chain
    pub use crate::middleware_chain::{
        MiddlewareChain, MiddlewareRequest, MiddlewareResult, PathMatcher,
        middleware_result_to_response, parse_cookies, extract_geo,
    };

    // i18n
    pub use crate::i18n::{I18nConfig, TranslationStore};

    // HTTP 103 Early Hints
    pub use crate::early_hints::{EarlyHints, extract_early_hints_from_head};

    // Critical CSS
    pub use crate::critical_css::{CssOptimizer, extract_critical_rules, minify_css};

    // SSE (Server-Sent Events)
    pub use crate::sse::{SseEvent, SseStream, sse_client_script, sse_from_stream};

    // Service Worker / PWA
    pub use crate::service_worker::{ServiceWorkerConfig, PwaManifest};

    // Content Security Policy
    pub use crate::csp::{CspConfig, add_nonce_to_scripts, add_nonce_to_styles, strict_csp};

    // Error & Loading Boundaries
    pub use crate::error_boundary::{
        BoundaryError, ErrorBoundaryConfig, skeleton, skeleton_css,
    };

    // View Transitions
    pub use crate::view_transition::ViewTransitionConfig;

    // Core Web Vitals
    pub use crate::web_vitals::{
        PerformanceBudget, web_vitals_script, server_timing_header,
    };

    // Rate Limiting
    pub use crate::rate_limit::RateLimiter;

    // Session Management
    pub use crate::session::{Session, SessionConfig};

    // Template Engine (DX: write pages as .html files)
    pub use crate::template_engine::{TemplateContext, TemplateEngine, TemplateValue};

    // Markdown Pages (DX: write pages as .md files)
    pub use crate::markdown::{MarkdownPage, markdown_to_html};

    // TOML Config Routes (DX: define routes in hayabusa.toml)
    pub use crate::config_routes::AppConfig;

    // Hot Reload (DX: instant browser refresh)
    pub use crate::hot_reload::{hot_reload_script, reload_type_for_file, dev_error_overlay};

    // Data Loader (DX: load data from JSON/Python/Node/any language)
    pub use crate::data_loader::DataLoader;

    // Supabase Client (unofficial)
    pub use crate::supabase::SupabaseClient;

    // OpenAI Client (unofficial)
    pub use crate::openai::{OpenAiClient, cosine_similarity, define_tool};

    // Mobile PWA
    pub use crate::mobile_pwa::{
        MobilePwaConfig, TouchGestureConfig,
        safe_area_css, touch_target_css, mobile_sw_additions,
    };

    // Vercel Adapter (Build Output API v3)
    pub use crate::vercel::{VercelAdapter, VercelEdgeMiddleware, VercelCron, VercelStorage};

    // Deploy Adapters (Fly.io, Docker, Railway, AWS, Cloudflare)
    pub use crate::deploy::{
        FlyIoAdapter, DockerAdapter, RailwayAdapter, AwsAdapter, CloudflareAdapter,
        DeployGenerator,
    };

    // Client-Side Interactivity (htmx / Alpine.js / Petite-Vue)
    pub use crate::interactivity::{
        AlpineComponent, ClientFramework, HtmxAttrs, InteractivityConfig,
        alpine_modal, alpine_tabs, alpine_toast_system, alpine_toggle,
        htmx_form, htmx_infinite_scroll, htmx_live_search,
    };

    // Authentication (JWT, OAuth2, password hashing)
    pub use crate::auth::{AuthConfig, AuthProvider, Jwt, Claims, hash_password, verify_password};

    // SEO (Sitemap, RSS, Open Graph, JSON-LD, robots.txt)
    pub use crate::seo::{
        Sitemap, RssFeed, RssItem, OpenGraph, JsonLd, RobotsTxt, RobotsRule,
        canonical_tag, hreflang_tags, Priority, ChangeFreq,
    };

    // Form Validation
    pub use crate::validation::{Schema, Rule, ValidationResult};

    // WebSocket
    pub use crate::websocket::{WsHub, WsRoom, WsEndpoint, WsMessage, ws_client_script, WsClientOptions};

    // Test Utilities
    pub use crate::testing::{TestRequest, TestResponse, TestClient, HtmlAssert, Benchmark};

    // Structured Logging
    pub use crate::logging::{Logger, LogLevel, RequestLog, Span};

    // Email
    pub use crate::email::{Email, EmailClient, email_template};

    // Background Jobs
    pub use crate::background_job::{Job, JobQueue, JobPriority, RecurringJob};

    // A/B Testing & Feature Flags
    pub use crate::ab_test::{Experiments, Experiment, FeatureFlag, ab_test_script, variant_css};

    // GraphQL
    pub use crate::graphql::{GqlSchema, GqlType, GqlField, GqlClient, GqlResponse, parse_query};

    // File Upload
    pub use crate::file_upload::{
        UploadConfig, UploadedFile, MultipartPart, parse_boundary, parse_multipart,
        validate_upload, validate_uploads, unique_filename, sanitize_filename, upload_form,
    };

    // Re-export macros
    pub use hayabusa_macros::html;
}
