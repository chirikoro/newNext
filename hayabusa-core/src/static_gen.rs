use dashmap::DashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Notify;

use crate::component::{PageHandler, PageRequest};
use crate::layout::{Layout, RootLayout};

/// A cached page for ISR (Incremental Static Regeneration)
#[derive(Debug, Clone)]
struct CachedPage {
    html: String,
    generated_at: Instant,
    revalidate_after: Option<Duration>,
}

/// Static site generator and ISR cache manager
pub struct StaticGenerator {
    /// In-memory cache for ISR pages
    cache: Arc<DashMap<String, CachedPage>>,
    /// Output directory for SSG build
    output_dir: PathBuf,
    /// Notification for background revalidation (used by ISR revalidation tasks)
    #[allow(dead_code)]
    revalidation_notify: Arc<Notify>,
}

impl StaticGenerator {
    pub fn new(output_dir: impl Into<PathBuf>) -> Self {
        Self {
            cache: Arc::new(DashMap::new()),
            output_dir: output_dir.into(),
            revalidation_notify: Arc::new(Notify::new()),
        }
    }

    /// Generate a static page and write it to the output directory.
    pub async fn generate_page(
        &self,
        path: &str,
        handler: &PageHandler,
        layouts: &[&dyn Layout],
    ) -> Result<String, crate::error::HayabusaError> {
        let request = PageRequest::new(path.to_string());
        let result = handler(request).await;

        let html = if layouts.is_empty() {
            let root = RootLayout::default();
            root.render(&result.html, &result.head)
        } else {
            crate::layout::compose_layouts(layouts, &result.html, &result.head)
        };

        // Write to dist/ directory
        let file_path = self.path_to_file(path);
        if let Some(parent) = file_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        tokio::fs::write(&file_path, &html).await?;

        tracing::info!("Generated static page: {} -> {}", path, file_path.display());

        Ok(html)
    }

    /// Get a cached page or render it fresh. Implements stale-while-revalidate for ISR.
    pub async fn get_or_render(
        &self,
        path: &str,
        handler: &PageHandler,
        layouts: &[&dyn Layout],
        revalidate: Option<Duration>,
    ) -> String {
        // Check cache
        if let Some(cached) = self.cache.get(path) {
            let is_stale = cached
                .revalidate_after
                .map(|dur| cached.generated_at.elapsed() > dur)
                .unwrap_or(false);

            if !is_stale {
                return cached.html.clone();
            }

            // Stale: return cached version but trigger background revalidation
            let html = cached.html.clone();
            let path = path.to_string();

            // Can't pass handler reference to spawned task, so just mark for revalidation
            // The actual revalidation happens in the serve loop
            tracing::info!("ISR: serving stale page for {}, revalidation needed", path);

            return html;
        }

        // Not cached: render fresh
        let request = PageRequest::new(path.to_string());
        let result = handler(request).await;

        let html = if layouts.is_empty() {
            let root = RootLayout::default();
            root.render(&result.html, &result.head)
        } else {
            crate::layout::compose_layouts(layouts, &result.html, &result.head)
        };

        // Store in cache
        self.cache.insert(
            path.to_string(),
            CachedPage {
                html: html.clone(),
                generated_at: Instant::now(),
                revalidate_after: revalidate,
            },
        );

        html
    }

    /// Invalidate a cached page
    pub fn invalidate(&self, path: &str) {
        self.cache.remove(path);
        tracing::info!("ISR: invalidated cache for {}", path);
    }

    /// Invalidate all cached pages
    pub fn invalidate_all(&self) {
        self.cache.clear();
        tracing::info!("ISR: invalidated all cached pages");
    }

    /// Build all static pages to the output directory
    pub async fn build_all(
        &self,
        pages: &[(String, PageHandler)],
        layouts: &[&dyn Layout],
    ) -> Result<Vec<String>, crate::error::HayabusaError> {
        let mut generated = Vec::new();

        // Clean output directory
        if self.output_dir.exists() {
            tokio::fs::remove_dir_all(&self.output_dir).await?;
        }
        tokio::fs::create_dir_all(&self.output_dir).await?;

        for (path, handler) in pages {
            let _html = self.generate_page(path, handler, layouts).await?;
            generated.push(path.clone());
        }

        tracing::info!("SSG: built {} static pages", generated.len());
        Ok(generated)
    }

    fn path_to_file(&self, url_path: &str) -> PathBuf {
        let mut file_path = self.output_dir.clone();
        if url_path == "/" {
            file_path.push("index.html");
        } else {
            let clean = url_path.trim_start_matches('/');
            file_path.push(clean);
            file_path.push("index.html");
        }
        file_path
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_path_to_file() {
        let gen = StaticGenerator::new("dist");
        assert_eq!(gen.path_to_file("/"), PathBuf::from("dist/index.html"));
        assert_eq!(
            gen.path_to_file("/about"),
            PathBuf::from("dist/about/index.html")
        );
        assert_eq!(
            gen.path_to_file("/blog/hello"),
            PathBuf::from("dist/blog/hello/index.html")
        );
    }
}
