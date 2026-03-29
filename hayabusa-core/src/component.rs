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

/// Metadata for the HTML <head> section (SEO optimization)
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

    /// Render the head context into HTML meta tags
    pub fn render(&self) -> String {
        let mut html = String::new();

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
        for href in &self.extra_links {
            html.push_str(&format!(
                "<link rel=\"stylesheet\" href=\"{}\" />\n",
                html_escape(href)
            ));
        }
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
