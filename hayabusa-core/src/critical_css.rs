//! Critical CSS Extraction and Inlining for Hayabusa.
//!
//! Inlines above-the-fold CSS directly into the HTML `<head>` to eliminate
//! render-blocking CSS requests. Remaining CSS is loaded asynchronously.
//!
//! ## Impact on User-Perceived Speed
//! - First paint happens without waiting for external CSS download
//! - Eliminates render-blocking CSS for above-the-fold content
//! - Remaining CSS loads in background (non-blocking)
//!
//! ## Usage
//! ```ignore
//! let optimizer = CssOptimizer::new()
//!     .critical_css("body{margin:0} .hero{display:flex}")
//!     .stylesheet("/style.css");
//! ```

/// CSS optimization configuration for a page
#[derive(Debug, Clone, Default)]
pub struct CssOptimizer {
    /// Critical CSS to inline in <head>
    critical: Option<String>,
    /// External stylesheets to load asynchronously
    stylesheets: Vec<String>,
    /// Whether to enable CSS containment hints
    enable_containment: bool,
}

impl CssOptimizer {
    pub fn new() -> Self {
        Self::default()
    }

    /// Set critical (above-the-fold) CSS to inline in <head>
    pub fn critical_css(mut self, css: impl Into<String>) -> Self {
        self.critical = Some(css.into());
        self
    }

    /// Add an external stylesheet to load asynchronously
    pub fn stylesheet(mut self, url: impl Into<String>) -> Self {
        self.stylesheets.push(url.into());
        self
    }

    /// Enable CSS containment hints for better rendering performance
    pub fn containment(mut self, enable: bool) -> Self {
        self.enable_containment = enable;
        self
    }

    /// Render the optimized CSS loading strategy.
    ///
    /// Returns HTML to insert in `<head>`:
    /// 1. Inlined critical CSS in `<style>`
    /// 2. Async-loaded external stylesheets using the `media` trick
    /// 3. `<noscript>` fallback for non-JS browsers
    pub fn render(&self) -> String {
        let mut html = String::new();

        // 1. Inline critical CSS
        if let Some(ref css) = self.critical {
            let minified = minify_css(css);
            html.push_str(&format!("<style>{}</style>\n", minified));
        }

        // 2. CSS containment
        if self.enable_containment {
            html.push_str("<style>.contain-layout{contain:layout}.contain-paint{contain:paint}.contain-strict{contain:strict}</style>\n");
        }

        // 3. Async external stylesheets
        // Uses media="print" trick: loads without blocking, then swaps to media="all"
        for url in &self.stylesheets {
            html.push_str(&format!(
                "<link rel=\"stylesheet\" href=\"{url}\" media=\"print\" onload=\"this.media='all'\" />\n"
            ));
        }

        // 4. Noscript fallback for external stylesheets
        if !self.stylesheets.is_empty() {
            html.push_str("<noscript>");
            for url in &self.stylesheets {
                html.push_str(&format!(
                    "<link rel=\"stylesheet\" href=\"{url}\" />"
                ));
            }
            html.push_str("</noscript>\n");
        }

        html
    }
}

/// Simple CSS minification: remove comments, excess whitespace, newlines
pub fn minify_css(css: &str) -> String {
    let mut result = String::with_capacity(css.len());
    let mut chars = css.chars().peekable();
    let mut in_string = false;
    let mut string_char = '"';

    while let Some(ch) = chars.next() {
        // Handle string literals
        if in_string {
            result.push(ch);
            if ch == string_char {
                in_string = false;
            }
            continue;
        }

        if ch == '"' || ch == '\'' {
            in_string = true;
            string_char = ch;
            result.push(ch);
            continue;
        }

        // Remove comments /* ... */
        if ch == '/' {
            if chars.peek() == Some(&'*') {
                chars.next(); // consume *
                // Skip until */
                loop {
                    match chars.next() {
                        Some('*') if chars.peek() == Some(&'/') => {
                            chars.next();
                            break;
                        }
                        None => break,
                        _ => continue,
                    }
                }
                continue;
            }
        }

        // Collapse whitespace
        if ch.is_whitespace() {
            // Only emit a single space if needed (not after/before certain chars)
            if let Some(last) = result.chars().last() {
                if !matches!(last, '{' | '}' | ';' | ':' | ',' | '>' | '+' | '~') {
                    if let Some(&next) = chars.peek() {
                        if !next.is_whitespace()
                            && !matches!(next, '{' | '}' | ';' | ':' | ',' | '>' | '+' | '~')
                        {
                            result.push(' ');
                        }
                    }
                }
            }
            // Skip remaining whitespace
            while chars.peek().map_or(false, |c| c.is_whitespace()) {
                chars.next();
            }
            continue;
        }

        result.push(ch);
    }

    result
}

/// Extract critical CSS rules that match a set of selectors.
///
/// Given the full CSS and a list of "above-the-fold" selectors,
/// returns only the matching rules.
pub fn extract_critical_rules(full_css: &str, critical_selectors: &[&str]) -> String {
    let mut critical = String::new();
    let mut remaining = full_css;

    while let Some(brace_pos) = remaining.find('{') {
        let selector = remaining[..brace_pos].trim();

        // Find the matching closing brace
        if let Some(close_pos) = find_matching_brace(&remaining[brace_pos..]) {
            let rule = &remaining[..brace_pos + close_pos + 1];

            // Check if this selector matches any critical selector
            let is_critical = critical_selectors.iter().any(|cs| {
                selector.contains(cs) || selector == "*" || selector.starts_with('@')
            });

            if is_critical {
                critical.push_str(rule);
                critical.push('\n');
            }

            remaining = &remaining[brace_pos + close_pos + 1..];
        } else {
            break;
        }
    }

    critical
}

/// Find the matching closing brace, handling nesting
fn find_matching_brace(s: &str) -> Option<usize> {
    let mut depth = 0;
    for (i, ch) in s.char_indices() {
        match ch {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(i);
                }
            }
            _ => {}
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_css_optimizer_render() {
        let opt = CssOptimizer::new()
            .critical_css("body { margin: 0; } .hero { display: flex; }")
            .stylesheet("/style.css");

        let html = opt.render();
        // Critical CSS is inlined
        assert!(html.contains("<style>"));
        assert!(html.contains("margin:0"));
        // External CSS uses async loading trick
        assert!(html.contains("media=\"print\""));
        assert!(html.contains("onload=\"this.media='all'\""));
        // Noscript fallback
        assert!(html.contains("<noscript>"));
    }

    #[test]
    fn test_minify_css() {
        let css = r#"
            body {
                margin: 0;
                padding: 0;
            }
            /* This is a comment */
            .hero {
                display: flex;
                align-items: center;
            }
        "#;
        let minified = minify_css(css);
        assert!(!minified.contains("/* This is a comment */"));
        assert!(!minified.contains('\n'));
        assert!(minified.contains("margin:0"));
        assert!(minified.contains("display:flex"));
    }

    #[test]
    fn test_minify_css_preserves_strings() {
        let css = r#"body::after { content: "  spaces  "; }"#;
        let minified = minify_css(css);
        assert!(minified.contains("\"  spaces  \""));
    }

    #[test]
    fn test_extract_critical_rules() {
        let css = "body{margin:0} .hero{display:flex} .footer{margin-top:20px} @media(max-width:768px){.hero{flex-direction:column}}";
        let critical = extract_critical_rules(css, &["body", ".hero", "@media"]);
        assert!(critical.contains("body{margin:0}"));
        assert!(critical.contains(".hero{display:flex}"));
        assert!(critical.contains("@media"));
        assert!(!critical.contains(".footer"));
    }

    #[test]
    fn test_containment_css() {
        let opt = CssOptimizer::new().containment(true);
        let html = opt.render();
        assert!(html.contains("contain:layout"));
        assert!(html.contains("contain:paint"));
    }

    #[test]
    fn test_no_noscript_without_stylesheets() {
        let opt = CssOptimizer::new().critical_css("body{margin:0}");
        let html = opt.render();
        assert!(!html.contains("<noscript>"));
    }
}
