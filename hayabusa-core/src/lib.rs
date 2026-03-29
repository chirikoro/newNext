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
pub mod error;
pub mod head;
pub mod layout;
pub mod middleware;
pub mod render;
pub mod router;
pub mod state;
pub mod static_gen;

/// Prelude: import everything you need with `use hayabusa_core::prelude::*`
pub mod prelude {
    pub use crate::app::HayabusaApp;
    pub use crate::component::{
        HeadContext, LinkRel, PageHandler, PageRequest, RenderMode, RenderResult,
    };
    pub use crate::error::HayabusaError;
    pub use crate::layout::{FnLayout, Layout, RootLayout};
    pub use crate::middleware::MiddlewareConfig;
    pub use crate::render::{
        check_etag, generate_etag, html_response, json_response, minify_html, render_page,
        render_streaming, suspense_placeholder, suspense_resolve,
    };
    pub use crate::router::{ApiMethod, RouteTable};
    pub use crate::state::AppState;
    pub use crate::static_gen::StaticGenerator;

    // Re-export macros
    pub use hayabusa_macros::html;
}
