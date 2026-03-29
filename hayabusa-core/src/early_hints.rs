//! HTTP 103 Early Hints for Hayabusa.
//!
//! Sends preload hints to the browser BEFORE the full response is ready.
//! This allows the browser to start fetching critical resources (CSS, fonts, JS)
//! while the server is still rendering the page.
//!
//! ## Impact on User-Perceived Speed
//! - Browser begins downloading CSS/fonts 50-300ms earlier
//! - Eliminates the "blank screen" waiting period
//! - Works with CDNs that support 103 Early Hints (Cloudflare, Fastly)
//!
//! ## Usage
//! ```ignore
//! let hints = EarlyHints::new()
//!     .preload("/style.css", "style")
//!     .preload("/fonts/inter.woff2", "font")
//!     .preconnect("https://api.example.com");
//! ```

use std::fmt;

/// A collection of Early Hints to send as HTTP 103 response
#[derive(Debug, Clone, Default)]
pub struct EarlyHints {
    links: Vec<EarlyHintLink>,
}

/// A single Link header entry for Early Hints
#[derive(Debug, Clone)]
pub struct EarlyHintLink {
    pub url: String,
    pub rel: EarlyHintRel,
    pub as_type: Option<String>,
    pub crossorigin: bool,
}

/// Relationship types for Early Hints
#[derive(Debug, Clone)]
pub enum EarlyHintRel {
    Preload,
    Preconnect,
    ModulePreload,
}

impl fmt::Display for EarlyHintRel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EarlyHintRel::Preload => write!(f, "preload"),
            EarlyHintRel::Preconnect => write!(f, "preconnect"),
            EarlyHintRel::ModulePreload => write!(f, "modulepreload"),
        }
    }
}

impl EarlyHints {
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a preload hint for a resource
    pub fn preload(mut self, url: impl Into<String>, as_type: impl Into<String>) -> Self {
        self.links.push(EarlyHintLink {
            url: url.into(),
            rel: EarlyHintRel::Preload,
            as_type: Some(as_type.into()),
            crossorigin: false,
        });
        self
    }

    /// Add a preload hint for a font (automatically sets crossorigin)
    pub fn preload_font(mut self, url: impl Into<String>) -> Self {
        self.links.push(EarlyHintLink {
            url: url.into(),
            rel: EarlyHintRel::Preload,
            as_type: Some("font".to_string()),
            crossorigin: true,
        });
        self
    }

    /// Add a preconnect hint
    pub fn preconnect(mut self, origin: impl Into<String>) -> Self {
        self.links.push(EarlyHintLink {
            url: origin.into(),
            rel: EarlyHintRel::Preconnect,
            as_type: None,
            crossorigin: false,
        });
        self
    }

    /// Add a module preload hint
    pub fn module_preload(mut self, url: impl Into<String>) -> Self {
        self.links.push(EarlyHintLink {
            url: url.into(),
            rel: EarlyHintRel::ModulePreload,
            as_type: None,
            crossorigin: false,
        });
        self
    }

    /// Render all hints as Link header values for HTTP 103 response
    pub fn render_link_headers(&self) -> Vec<String> {
        self.links.iter().map(|link| {
            let mut parts = vec![format!("<{}>; rel={}", link.url, link.rel)];
            if let Some(ref as_type) = link.as_type {
                parts.push(format!("as={}", as_type));
            }
            if link.crossorigin {
                parts.push("crossorigin".to_string());
            }
            parts.join("; ")
        }).collect()
    }

    /// Render as a single combined Link header value
    pub fn render_combined_link_header(&self) -> String {
        self.render_link_headers().join(", ")
    }

    /// Check if there are any hints to send
    pub fn is_empty(&self) -> bool {
        self.links.is_empty()
    }

    /// Get the number of hints
    pub fn len(&self) -> usize {
        self.links.len()
    }
}

/// Auto-detect Early Hints from HeadContext resource hints.
///
/// Extracts preload/preconnect entries that should be sent as 103 Early Hints.
pub fn extract_early_hints_from_head(head_html: &str) -> EarlyHints {
    let mut hints = EarlyHints::new();

    // Parse <link rel="preload" ...> tags
    for line in head_html.lines() {
        let line = line.trim();
        if !line.starts_with("<link ") {
            continue;
        }

        let rel = extract_attr(line, "rel");
        let href = extract_attr(line, "href");

        if let (Some(rel), Some(href)) = (rel, href) {
            match rel.as_str() {
                "preload" => {
                    let as_type = extract_attr(line, "as").unwrap_or_default();
                    if as_type == "font" {
                        hints = hints.preload_font(href);
                    } else {
                        hints = hints.preload(href, as_type);
                    }
                }
                "preconnect" => {
                    hints = hints.preconnect(href);
                }
                "modulepreload" => {
                    hints = hints.module_preload(href);
                }
                _ => {}
            }
        }
    }

    hints
}

/// Simple attribute extractor from an HTML tag string
fn extract_attr(tag: &str, attr_name: &str) -> Option<String> {
    let search = format!("{}=\"", attr_name);
    if let Some(start) = tag.find(&search) {
        let after = &tag[start + search.len()..];
        if let Some(end) = after.find('"') {
            return Some(after[..end].to_string());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_early_hints_link_header() {
        let hints = EarlyHints::new()
            .preload("/style.css", "style")
            .preload_font("/fonts/inter.woff2")
            .preconnect("https://api.example.com");

        let headers = hints.render_link_headers();
        assert_eq!(headers.len(), 3);
        assert_eq!(headers[0], "</style.css>; rel=preload; as=style");
        assert_eq!(headers[1], "</fonts/inter.woff2>; rel=preload; as=font; crossorigin");
        assert_eq!(headers[2], "<https://api.example.com>; rel=preconnect");
    }

    #[test]
    fn test_combined_link_header() {
        let hints = EarlyHints::new()
            .preload("/style.css", "style")
            .preconnect("https://cdn.example.com");

        let combined = hints.render_combined_link_header();
        assert!(combined.contains("</style.css>; rel=preload; as=style"));
        assert!(combined.contains("<https://cdn.example.com>; rel=preconnect"));
    }

    #[test]
    fn test_extract_from_head_html() {
        let head = r#"
<link rel="preload" href="/fonts/inter.woff2" as="font" crossorigin />
<link rel="preload" href="/style.css" as="style" />
<link rel="preconnect" href="https://api.example.com" />
<link rel="modulepreload" href="/app.js" />
"#;
        let hints = extract_early_hints_from_head(head);
        assert_eq!(hints.len(), 4);

        let headers = hints.render_link_headers();
        assert!(headers[0].contains("as=font"));
        assert!(headers[1].contains("as=style"));
        assert!(headers[2].contains("rel=preconnect"));
        assert!(headers[3].contains("rel=modulepreload"));
    }

    #[test]
    fn test_module_preload() {
        let hints = EarlyHints::new()
            .module_preload("/app.mjs");

        let headers = hints.render_link_headers();
        assert_eq!(headers[0], "</app.mjs>; rel=modulepreload");
    }

    #[test]
    fn test_empty_hints() {
        let hints = EarlyHints::new();
        assert!(hints.is_empty());
        assert_eq!(hints.len(), 0);
    }
}
