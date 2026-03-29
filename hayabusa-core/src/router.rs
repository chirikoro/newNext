use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::component::{PageHandler, RenderMode};

/// A discovered route entry from the file-based routing scan
#[derive(Debug, Clone)]
pub struct RouteEntry {
    /// URL path pattern (e.g., "/blog/:slug")
    pub path_pattern: String,
    /// The type of route
    pub kind: RouteKind,
    /// Render mode for this route
    pub render_mode: RenderMode,
    /// Source file path (relative to app/ directory)
    pub source: PathBuf,
}

#[derive(Debug, Clone, PartialEq)]
pub enum RouteKind {
    /// A page route (page.rs)
    Page,
    /// A layout route (layout.rs)
    Layout,
    /// An API route (api/*.rs)
    Api,
    /// An error page (error.rs)
    Error,
}

/// Route table: maps URL patterns to handlers
pub struct RouteTable {
    pub pages: Vec<PageRoute>,
    pub api_routes: Vec<ApiRoute>,
    pub layouts: HashMap<String, LayoutEntry>,
}

pub struct PageRoute {
    pub path_pattern: String,
    pub handler: PageHandler,
    pub render_mode: RenderMode,
}

pub struct ApiRoute {
    pub path_pattern: String,
    pub method: ApiMethod,
    pub handler: Box<
        dyn Fn(axum::extract::Request) -> std::pin::Pin<
            Box<dyn std::future::Future<Output = axum::response::Response> + Send>,
        > + Send
            + Sync,
    >,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ApiMethod {
    Get,
    Post,
    Put,
    Delete,
    Patch,
    Any,
}

pub struct LayoutEntry {
    /// Path segment this layout applies to (e.g., "/" for root, "/blog" for blog section)
    pub segment: String,
    pub layout: Box<dyn crate::layout::Layout>,
}

impl RouteTable {
    pub fn new() -> Self {
        Self {
            pages: Vec::new(),
            api_routes: Vec::new(),
            layouts: HashMap::new(),
        }
    }

    pub fn page(
        mut self,
        path: impl Into<String>,
        render_mode: RenderMode,
        handler: PageHandler,
    ) -> Self {
        self.pages.push(PageRoute {
            path_pattern: path.into(),
            handler,
            render_mode,
        });
        self
    }

    pub fn api(
        mut self,
        method: ApiMethod,
        path: impl Into<String>,
        handler: Box<
            dyn Fn(axum::extract::Request) -> std::pin::Pin<
                Box<dyn std::future::Future<Output = axum::response::Response> + Send>,
            > + Send
                + Sync,
        >,
    ) -> Self {
        self.api_routes.push(ApiRoute {
            path_pattern: path.into(),
            method,
            handler,
        });
        self
    }

    pub fn layout(mut self, segment: impl Into<String>, layout: Box<dyn crate::layout::Layout>) -> Self {
        let seg = segment.into();
        self.layouts.insert(
            seg.clone(),
            LayoutEntry {
                segment: seg,
                layout,
            },
        );
        self
    }
}

/// Scan the app/ directory and discover route entries.
///
/// Convention:
/// - `app/page.rs` → `/`
/// - `app/about/page.rs` → `/about`
/// - `app/blog/[slug]/page.rs` → `/blog/:slug`
/// - `app/api/health.rs` → `/api/health`
/// - `app/layout.rs` → layout for `/`
/// - `app/blog/layout.rs` → layout for `/blog`
pub fn discover_routes(app_dir: &Path) -> Vec<RouteEntry> {
    let mut entries = Vec::new();

    if !app_dir.exists() {
        return entries;
    }

    scan_directory(app_dir, app_dir, &mut entries);
    entries
}

fn scan_directory(base: &Path, current: &Path, entries: &mut Vec<RouteEntry>) {
    let Ok(read_dir) = std::fs::read_dir(current) else {
        return;
    };

    let mut dir_entries: Vec<_> = read_dir.filter_map(|e| e.ok()).collect();
    dir_entries.sort_by_key(|e| e.file_name());

    for entry in dir_entries {
        let path = entry.path();
        let file_name = entry.file_name();
        let file_name_str = file_name.to_string_lossy();

        if path.is_dir() {
            scan_directory(base, &path, entries);
        } else if file_name_str.ends_with(".rs") {
            let relative = path.strip_prefix(base).unwrap_or(&path);
            let url_path = file_path_to_url(relative);

            let kind = if file_name_str == "page.rs" {
                RouteKind::Page
            } else if file_name_str == "layout.rs" {
                RouteKind::Layout
            } else if file_name_str == "error.rs" {
                RouteKind::Error
            } else if relative.starts_with("api") || relative.starts_with("api/") {
                RouteKind::Api
            } else {
                continue;
            };

            entries.push(RouteEntry {
                path_pattern: url_path,
                kind,
                render_mode: RenderMode::default(),
                source: relative.to_path_buf(),
            });
        }
    }
}

/// Convert a file path to a URL pattern.
///
/// Examples:
/// - `page.rs` → `/`
/// - `about/page.rs` → `/about`
/// - `blog/[slug]/page.rs` → `/blog/:slug`
/// - `api/health.rs` → `/api/health`
fn file_path_to_url(path: &Path) -> String {
    let mut segments: Vec<String> = Vec::new();

    for component in path.components() {
        let s = component.as_os_str().to_string_lossy().to_string();

        if s == "page.rs" || s == "layout.rs" || s == "error.rs" {
            continue;
        }

        // Strip .rs extension for API routes
        let s = s.strip_suffix(".rs").unwrap_or(&s).to_string();

        // Convert [param] to :param
        if s.starts_with('[') && s.ends_with(']') {
            let param = &s[1..s.len() - 1];
            segments.push(format!(":{param}"));
        } else {
            segments.push(s);
        }
    }

    if segments.is_empty() {
        "/".to_string()
    } else {
        format!("/{}", segments.join("/"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn test_file_path_to_url() {
        assert_eq!(file_path_to_url(Path::new("page.rs")), "/");
        assert_eq!(file_path_to_url(Path::new("about/page.rs")), "/about");
        assert_eq!(
            file_path_to_url(Path::new("blog/[slug]/page.rs")),
            "/blog/:slug"
        );
        assert_eq!(
            file_path_to_url(Path::new("api/health.rs")),
            "/api/health"
        );
        assert_eq!(file_path_to_url(Path::new("api/posts.rs")), "/api/posts");
    }

    #[test]
    fn test_file_path_to_url_nested() {
        assert_eq!(
            file_path_to_url(Path::new("docs/[category]/[id]/page.rs")),
            "/docs/:category/:id"
        );
    }
}
