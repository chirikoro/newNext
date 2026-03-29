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

pub mod app;
pub mod component;
pub mod critical_css;
pub mod csp;
pub mod early_hints;
pub mod error;
pub mod error_boundary;
pub mod font;
pub mod head;
pub mod i18n;
pub mod image;
pub mod layout;
pub mod middleware;
pub mod middleware_chain;
pub mod ppr;
pub mod render;
pub mod router;
pub mod script;
pub mod server_action;
pub mod rate_limit;
pub mod service_worker;
pub mod session;
pub mod sse;
pub mod state;
pub mod static_gen;
pub mod view_transition;
pub mod web_vitals;

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

    // Re-export macros
    pub use hayabusa_macros::html;
}
