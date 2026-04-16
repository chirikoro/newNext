use axum::{
    Router,
    extract::{Path as AxumPath, Query, Request},
    routing::{delete, get, patch, post, put},
};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpListener;

use crate::component::{PageRequest, RenderMode};
use crate::error::HayabusaError;
use crate::layout::{Layout, RootLayout};
use crate::middleware::{self, MiddlewareConfig};
use crate::render;
use crate::router::{ApiMethod, RouteTable};
use crate::state::AppState;
use crate::static_gen::StaticGenerator;

/// The main application builder for Hayabusa.
pub struct HayabusaApp {
    route_table: RouteTable,
    middleware_config: MiddlewareConfig,
    state: AppState,
    port: u16,
    host: String,
    static_generator: Option<StaticGenerator>,
    custom_routers: Vec<Router>,
    nested_routers: Vec<(String, Router)>,
    custom_routes: Vec<(String, axum::routing::MethodRouter)>,
    fallback: Option<Router>,
}

impl HayabusaApp {
    pub fn new() -> Self {
        Self {
            route_table: RouteTable::new(),
            middleware_config: MiddlewareConfig::default(),
            state: AppState::new(),
            port: 3000,
            host: "127.0.0.1".to_string(),
            static_generator: None,
            custom_routers: Vec::new(),
            nested_routers: Vec::new(),
            custom_routes: Vec::new(),
            fallback: None,
        }
    }

    /// Set the route table
    pub fn routes(mut self, routes: RouteTable) -> Self {
        self.route_table = routes;
        self
    }

    /// Configure middleware
    pub fn middleware(mut self, config: MiddlewareConfig) -> Self {
        self.middleware_config = config;
        self
    }

    /// Set the port
    pub fn port(mut self, port: u16) -> Self {
        self.port = port;
        self
    }

    /// Set the host
    pub fn host(mut self, host: impl Into<String>) -> Self {
        self.host = host.into();
        self
    }

    /// Set application state
    pub fn state(mut self, state: AppState) -> Self {
        self.state = state;
        self
    }

    /// Enable static site generation
    pub fn with_static_gen(mut self, output_dir: impl Into<std::path::PathBuf>) -> Self {
        self.static_generator = Some(StaticGenerator::new(output_dir));
        self
    }

    /// Merge a custom axum Router into the application.
    ///
    /// ```ignore
    /// let api = Router::new()
    ///     .route("/api/custom", get(my_handler));
    /// HayabusaApp::new().merge(api).serve().await?;
    /// ```
    pub fn merge(mut self, router: Router) -> Self {
        self.custom_routers.push(router);
        self
    }

    /// Nest a Router under a path prefix.
    ///
    /// ```ignore
    /// let admin = Router::new()
    ///     .route("/users", get(list_users))
    ///     .route("/settings", get(settings));
    /// HayabusaApp::new().nest("/admin", admin).serve().await?;
    /// ```
    pub fn nest(mut self, path: impl Into<String>, router: Router) -> Self {
        self.nested_routers.push((path.into(), router));
        self
    }

    /// Add a single route with an axum MethodRouter.
    ///
    /// ```ignore
    /// use axum::routing::{get, post};
    /// HayabusaApp::new()
    ///     .route("/health", get(health_handler))
    ///     .route("/submit", post(submit_handler))
    ///     .serve().await?;
    /// ```
    pub fn route(mut self, path: impl Into<String>, method_router: axum::routing::MethodRouter) -> Self {
        self.custom_routes.push((path.into(), method_router));
        self
    }

    /// Set a fallback handler for unmatched routes.
    ///
    /// ```ignore
    /// let fallback = Router::new().route("/*path", get(not_found_handler));
    /// HayabusaApp::new().fallback(fallback).serve().await?;
    /// ```
    pub fn fallback(mut self, router: Router) -> Self {
        self.fallback = Some(router);
        self
    }

    /// Build the axum Router from the route table
    fn build_router(self) -> (Router, SocketAddr) {
        let mut router = Router::new();
        let layouts = Arc::new(self.route_table.layouts);
        let static_gen = self
            .static_generator
            .map(|sg| Arc::new(sg));

        // Register page routes
        for page_route in self.route_table.pages {
            let handler = Arc::new(page_route.handler);
            let render_mode = page_route.render_mode.clone();
            let layouts_ref = layouts.clone();
            let static_gen_ref = static_gen.clone();
            let pattern = page_route.path_pattern.clone();

            let axum_handler = move |
                path_params: Option<AxumPath<HashMap<String, String>>>,
                Query(query): Query<HashMap<String, String>>,
                req: Request,
            | {
                let handler = handler.clone();
                let render_mode = render_mode.clone();
                let layouts_ref = layouts_ref.clone();
                let static_gen_ref = static_gen_ref.clone();
                let pattern = pattern.clone();

                async move {
                    // Extract If-None-Match for ETag conditional requests (304 support)
                    let if_none_match = req.headers()
                        .get(http::header::IF_NONE_MATCH)
                        .and_then(|v| v.to_str().ok())
                        .map(|s| s.to_string());

                    let params = path_params
                        .map(|AxumPath(p)| p)
                        .unwrap_or_default();

                    let mut request = PageRequest::new(pattern.clone());
                    request.params = params;
                    request.query = query;

                    match render_mode {
                        RenderMode::Ssr => {
                            let result = handler(request).await;
                            let root_layout = RootLayout::default();
                            let layout_refs: Vec<&dyn Layout> = if layouts_ref.contains_key("/") {
                                vec![layouts_ref["/"].layout.as_ref()]
                            } else {
                                vec![&root_layout as &dyn Layout]
                            };
                            render::render_page(&result, &layout_refs, &RenderMode::Ssr)
                        }
                        RenderMode::Ssg { ref revalidate } => {
                            if let Some(ref sg) = static_gen_ref {
                                let root_layout = RootLayout::default();
                                let layout_refs: Vec<&dyn Layout> = vec![&root_layout as &dyn Layout];
                                let html = sg
                                    .get_or_render(
                                        &pattern,
                                        &handler,
                                        &layout_refs,
                                        revalidate.clone(),
                                    )
                                    .await;

                                // ETag check for cached content
                                let etag = render::generate_etag(&html);
                                if let Some(resp) = render::check_etag(
                                    if_none_match.as_deref(),
                                    &etag,
                                ) {
                                    return resp;
                                }

                                let minified = render::minify_html(&html);
                                let cache_header = if revalidate.is_some() {
                                    let secs = revalidate.unwrap().as_secs();
                                    format!("public, s-maxage={}, stale-while-revalidate={}", secs, secs * 2)
                                } else {
                                    "public, max-age=31536000, immutable".to_string()
                                };

                                axum::response::Response::builder()
                                    .status(200)
                                    .header("content-type", "text/html; charset=utf-8")
                                    .header("cache-control", cache_header)
                                    .header("etag", &etag)
                                    .header("vary", "Accept-Encoding")
                                    .body(axum::body::Body::from(minified))
                                    .unwrap()
                            } else {
                                let result = handler(request).await;
                                let root_layout = RootLayout::default();
                                let layout_refs: Vec<&dyn Layout> = vec![&root_layout as &dyn Layout];
                                render::render_page(&result, &layout_refs, &render_mode)
                            }
                        }
                        RenderMode::Streaming => {
                            let result = handler(request).await;
                            let stream = Box::pin(futures::stream::once(async move {
                                result.html
                            }));
                            render::render_streaming(&result.head, stream)
                        }
                    }
                }
            };

            // Register with axum
            let axum_path = page_route.path_pattern.clone();
            router = router.route(&axum_path, get(axum_handler));
        }

        // Register API routes
        for api_route in self.route_table.api_routes {
            let handler = Arc::new(api_route.handler);

            let axum_handler = move |req: Request| {
                let handler = handler.clone();
                async move { handler(req).await }
            };

            match api_route.method {
                ApiMethod::Get => {
                    router = router.route(&api_route.path_pattern, get(axum_handler));
                }
                ApiMethod::Post => {
                    router = router.route(&api_route.path_pattern, post(axum_handler));
                }
                ApiMethod::Put => {
                    router = router.route(&api_route.path_pattern, put(axum_handler));
                }
                ApiMethod::Delete => {
                    router = router.route(&api_route.path_pattern, delete(axum_handler));
                }
                ApiMethod::Patch => {
                    router = router.route(&api_route.path_pattern, patch(axum_handler));
                }
                ApiMethod::Any => {
                    router = router.route(&api_route.path_pattern, get(axum_handler.clone()));
                    router = router.route(&api_route.path_pattern, post(axum_handler));
                }
            }
        }

        // Apply custom routes
        for (path, method_router) in self.custom_routes {
            router = router.route(&path, method_router);
        }

        // Merge custom routers
        for custom in self.custom_routers {
            router = router.merge(custom);
        }

        // Nest routers under path prefixes
        for (path, nested) in self.nested_routers {
            router = router.nest(&path, nested);
        }

        // Apply fallback
        if let Some(fb) = self.fallback {
            router = router.fallback_service(fb);
        }

        // Apply middleware
        router = middleware::apply_middleware(router, &self.middleware_config);

        let addr: SocketAddr = format!("{}:{}", self.host, self.port)
            .parse()
            .expect("Invalid address");

        (router, addr)
    }

    /// Start the server
    pub async fn serve(self) -> Result<(), HayabusaError> {
        tracing_subscriber::fmt()
            .with_env_filter(
                tracing_subscriber::EnvFilter::try_from_default_env()
                    .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
            )
            .init();

        let port = self.port;
        let host = self.host.clone();
        let (router, addr) = self.build_router();

        tracing::info!("Hayabusa server starting at http://{}:{}", host, port);

        let listener = TcpListener::bind(addr)
            .await
            .map_err(|e| HayabusaError::Io(e))?;

        axum::serve(listener, router)
            .await
            .map_err(|e| HayabusaError::Internal(e.to_string()))?;

        Ok(())
    }
}

impl Default for HayabusaApp {
    fn default() -> Self {
        Self::new()
    }
}

