use std::time::Duration;

/// Defines how a page component should be rendered.
#[derive(Debug, Clone)]
pub enum RenderMode {
    /// Server-Side Rendering: render on every request
    Ssr,
    /// Static Site Generation with optional ISR revalidation
    Ssg { revalidate: Option<Duration> },
    /// Streaming SSR: send HTML in chunks
    Streaming,
}

impl Default for RenderMode {
    fn default() -> Self {
        RenderMode::Ssr
    }
}

/// The result of rendering a component
#[derive(Debug, Clone)]
pub struct RenderResult {
    /// The rendered HTML body
    pub html: String,
    /// Head context (title, meta tags) to inject into <head>
    pub head: HeadContext,
}

impl RenderResult {
    pub fn new(html: String) -> Self {
        Self {
            html,
            head: HeadContext::default(),
        }
    }

    pub fn with_head(mut self, head: HeadContext) -> Self {
        self.head = head;
        self
    }
}

/// Link relation types for resource hints
#[derive(Debug, Clone)]
pub enum LinkRel {
    /// Standard stylesheet link
    Stylesheet,
    /// Preload: fetch resource early with high priority
    Preload { as_type: String },
    /// Prefetch: fetch resource for future navigation (low priority)
    Prefetch,
    /// DNS Prefetch: resolve DNS for external domain early
    DnsPrefetch,
    /// Preconnect: establish connection to external domain early
    Preconnect,
    /// Module preload for ES modules
    ModulePreload,
}

/// A link element for the <head>
#[derive(Debug, Clone)]
pub struct LinkEntry {
    pub rel: LinkRel,
    pub href: String,
    pub crossorigin: bool,
}

/// Metadata for the HTML <head> section (SEO + performance optimization)
#[derive(Debug, Clone, Default)]
pub struct HeadContext {
    pub title: Option<String>,
    pub description: Option<String>,
    pub og_title: Option<String>,
    pub og_description: Option<String>,
    pub og_image: Option<String>,
    pub canonical: Option<String>,
    pub extra_meta: Vec<(String, String)>,
    pub extra_links: Vec<String>,
    pub scripts: Vec<String>,
    /// Resource hints (preload, prefetch, dns-prefetch, preconnect)
    pub resource_hints: Vec<LinkEntry>,
}

impl HeadContext {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    pub fn description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    pub fn og_title(mut self, title: impl Into<String>) -> Self {
        self.og_title = Some(title.into());
        self
    }

    pub fn og_description(mut self, desc: impl Into<String>) -> Self {
        self.og_description = Some(desc.into());
        self
    }

    pub fn og_image(mut self, url: impl Into<String>) -> Self {
        self.og_image = Some(url.into());
        self
    }

    pub fn canonical(mut self, url: impl Into<String>) -> Self {
        self.canonical = Some(url.into());
        self
    }

    pub fn meta(mut self, name: impl Into<String>, content: impl Into<String>) -> Self {
        self.extra_meta.push((name.into(), content.into()));
        self
    }

    pub fn link(mut self, href: impl Into<String>) -> Self {
        self.extra_links.push(href.into());
        self
    }

    /// Add a preload hint for a critical resource (CSS, font, script, image)
    pub fn preload(mut self, href: impl Into<String>, as_type: impl Into<String>) -> Self {
        self.resource_hints.push(LinkEntry {
            rel: LinkRel::Preload { as_type: as_type.into() },
            href: href.into(),
            crossorigin: false,
        });
        self
    }

    /// Add a prefetch hint for a resource needed on future navigation
    pub fn prefetch(mut self, href: impl Into<String>) -> Self {
        self.resource_hints.push(LinkEntry {
            rel: LinkRel::Prefetch,
            href: href.into(),
            crossorigin: false,
        });
        self
    }

    /// Add a dns-prefetch hint for an external domain
    pub fn dns_prefetch(mut self, domain: impl Into<String>) -> Self {
        self.resource_hints.push(LinkEntry {
            rel: LinkRel::DnsPrefetch,
            href: domain.into(),
            crossorigin: false,
        });
        self
    }

    /// Add a preconnect hint for an external domain
    pub fn preconnect(mut self, domain: impl Into<String>) -> Self {
        self.resource_hints.push(LinkEntry {
            rel: LinkRel::Preconnect,
            href: domain.into(),
            crossorigin: true,
        });
        self
    }

    /// Render the head context into HTML meta tags
    pub fn render(&self) -> String {
        let mut html = String::with_capacity(512);

        if let Some(ref title) = self.title {
            html.push_str(&format!("<title>{}</title>\n", html_escape(title)));
        }
        if let Some(ref desc) = self.description {
            html.push_str(&format!(
                "<meta name=\"description\" content=\"{}\" />\n",
                html_escape(desc)
            ));
        }
        if let Some(ref og_title) = self.og_title {
            html.push_str(&format!(
                "<meta property=\"og:title\" content=\"{}\" />\n",
                html_escape(og_title)
            ));
        }
        if let Some(ref og_desc) = self.og_description {
            html.push_str(&format!(
                "<meta property=\"og:description\" content=\"{}\" />\n",
                html_escape(og_desc)
            ));
        }
        if let Some(ref og_image) = self.og_image {
            html.push_str(&format!(
                "<meta property=\"og:image\" content=\"{}\" />\n",
                html_escape(og_image)
            ));
        }
        if let Some(ref canonical) = self.canonical {
            html.push_str(&format!(
                "<link rel=\"canonical\" href=\"{}\" />\n",
                html_escape(canonical)
            ));
        }
        for (name, content) in &self.extra_meta {
            html.push_str(&format!(
                "<meta name=\"{}\" content=\"{}\" />\n",
                html_escape(name),
                html_escape(content)
            ));
        }

        // Resource hints (preload, prefetch, dns-prefetch, preconnect)
        // These go BEFORE stylesheets and scripts for maximum effectiveness
        for link in &self.resource_hints {
            let crossorigin = if link.crossorigin { " crossorigin" } else { "" };
            match &link.rel {
                LinkRel::Preload { as_type } => {
                    html.push_str(&format!(
                        "<link rel=\"preload\" href=\"{}\" as=\"{}\"{} />\n",
                        html_escape(&link.href),
                        html_escape(as_type),
                        crossorigin
                    ));
                }
                LinkRel::Prefetch => {
                    html.push_str(&format!(
                        "<link rel=\"prefetch\" href=\"{}\" />\n",
                        html_escape(&link.href)
                    ));
                }
                LinkRel::DnsPrefetch => {
                    html.push_str(&format!(
                        "<link rel=\"dns-prefetch\" href=\"{}\" />\n",
                        html_escape(&link.href)
                    ));
                }
                LinkRel::Preconnect => {
                    html.push_str(&format!(
                        "<link rel=\"preconnect\" href=\"{}\"{} />\n",
                        html_escape(&link.href),
                        crossorigin
                    ));
                }
                LinkRel::ModulePreload => {
                    html.push_str(&format!(
                        "<link rel=\"modulepreload\" href=\"{}\" />\n",
                        html_escape(&link.href)
                    ));
                }
                LinkRel::Stylesheet => {
                    html.push_str(&format!(
                        "<link rel=\"stylesheet\" href=\"{}\" />\n",
                        html_escape(&link.href)
                    ));
                }
            }
        }

        // Stylesheets
        for href in &self.extra_links {
            html.push_str(&format!(
                "<link rel=\"stylesheet\" href=\"{}\" />\n",
                html_escape(href)
            ));
        }

        // Scripts
        for src in &self.scripts {
            html.push_str(&format!(
                "<script src=\"{}\"></script>\n",
                html_escape(src)
            ));
        }

        html
    }
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#x27;")
}

/// Type alias for page handler functions
pub type PageHandler =
    Box<dyn Fn(PageRequest) -> std::pin::Pin<Box<dyn std::future::Future<Output = RenderResult> + Send>> + Send + Sync>;

/// Request context passed to page components
#[derive(Debug, Clone)]
pub struct PageRequest {
    /// URL path parameters (e.g., slug from /blog/[slug])
    pub params: std::collections::HashMap<String, String>,
    /// Query string parameters
    pub query: std::collections::HashMap<String, String>,
    /// Request path
    pub path: String,
}

impl PageRequest {
    pub fn new(path: String) -> Self {
        Self {
            params: std::collections::HashMap::new(),
            query: std::collections::HashMap::new(),
            path,
        }
    }

    pub fn param(&self, key: &str) -> Option<&str> {
        self.params.get(key).map(|s| s.as_str())
    }

    pub fn query_param(&self, key: &str) -> Option<&str> {
        self.query.get(key).map(|s| s.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_head_context_render_with_resource_hints() {
        let head = HeadContext::new()
            .title("Test")
            .preload("/font.woff2", "font")
            .prefetch("/next-page.js")
            .dns_prefetch("https://cdn.example.com")
            .preconnect("https://api.example.com")
            .link("/style.css");

        let rendered = head.render();
        assert!(rendered.contains("<link rel=\"preload\" href=\"/font.woff2\" as=\"font\" />"));
        assert!(rendered.contains("<link rel=\"prefetch\" href=\"/next-page.js\" />"));
        assert!(rendered.contains("<link rel=\"dns-prefetch\" href=\"https://cdn.example.com\" />"));
        assert!(rendered.contains("<link rel=\"preconnect\" href=\"https://api.example.com\" crossorigin />"));
        assert!(rendered.contains("<link rel=\"stylesheet\" href=\"/style.css\" />"));
    }

    #[test]
    fn test_resource_hints_before_stylesheets() {
        let head = HeadContext::new()
            .link("/style.css")
            .preload("/critical.css", "style");

        let rendered = head.render();
        let preload_pos = rendered.find("rel=\"preload\"").unwrap();
        let stylesheet_pos = rendered.find("rel=\"stylesheet\"").unwrap();
        assert!(preload_pos < stylesheet_pos, "Preload hints should come before stylesheets");
    }
}
