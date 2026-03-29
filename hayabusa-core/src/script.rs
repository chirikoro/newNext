//! Script loading optimization for Hayabusa.
//!
//! Controls how JavaScript is loaded to minimize render blocking:
//! - `defer`: Parse after HTML, execute in order (best for most scripts)
//! - `async`: Parse in parallel, execute immediately (analytics, etc.)
//! - `module`: ES module with deferred loading by default
//! - `worker`: Offload to Web Worker for non-DOM scripts
//! - `afterInteractive`: Load after page becomes interactive (idle callback)

/// Script loading strategy
#[derive(Debug, Clone)]
pub enum ScriptStrategy {
    /// Block rendering until script loads (avoid unless necessary)
    Blocking,
    /// Download in parallel, execute after HTML parsing (ordered)
    Defer,
    /// Download in parallel, execute immediately when ready (unordered)
    Async,
    /// ES module (deferred by default, supports import/export)
    Module,
    /// Load using requestIdleCallback after page is interactive
    AfterInteractive,
    /// Offload to a Web Worker (for non-DOM computation)
    Worker,
}

/// An optimized script tag.
///
/// # Example
/// ```ignore
/// use hayabusa_core::script::OptimizedScript;
///
/// // Analytics - load async, don't block anything
/// let analytics = OptimizedScript::external("/js/analytics.js")
///     .strategy(ScriptStrategy::Async);
///
/// // Main app bundle - defer for ordered execution
/// let app = OptimizedScript::external("/js/app.js")
///     .strategy(ScriptStrategy::Defer);
///
/// // Heavy computation - offload to worker
/// let calc = OptimizedScript::external("/js/heavy-calc.js")
///     .strategy(ScriptStrategy::Worker);
///
/// // Inline script after page interactive
/// let lazy = OptimizedScript::inline("console.log('loaded')")
///     .strategy(ScriptStrategy::AfterInteractive);
/// ```
#[derive(Debug, Clone)]
pub struct OptimizedScript {
    /// External script URL (None for inline scripts)
    pub src: Option<String>,
    /// Inline script content
    pub content: Option<String>,
    /// Loading strategy
    pub strategy: ScriptStrategy,
    /// Optional id attribute
    pub id: Option<String>,
    /// nonce for CSP
    pub nonce: Option<String>,
}

impl OptimizedScript {
    /// Create an external script reference
    pub fn external(src: impl Into<String>) -> Self {
        Self {
            src: Some(src.into()),
            content: None,
            strategy: ScriptStrategy::Defer,
            id: None,
            nonce: None,
        }
    }

    /// Create an inline script
    pub fn inline(content: impl Into<String>) -> Self {
        Self {
            src: None,
            content: Some(content.into()),
            strategy: ScriptStrategy::Blocking,
            id: None,
            nonce: None,
        }
    }

    pub fn strategy(mut self, strategy: ScriptStrategy) -> Self {
        self.strategy = strategy;
        self
    }

    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.id = Some(id.into());
        self
    }

    pub fn nonce(mut self, nonce: impl Into<String>) -> Self {
        self.nonce = Some(nonce.into());
        self
    }

    /// Render the script tag(s) as HTML.
    pub fn render(&self) -> String {
        let id_attr = self
            .id
            .as_ref()
            .map(|id| format!(" id=\"{id}\""))
            .unwrap_or_default();

        let nonce_attr = self
            .nonce
            .as_ref()
            .map(|n| format!(" nonce=\"{n}\""))
            .unwrap_or_default();

        match &self.strategy {
            ScriptStrategy::Blocking => {
                if let Some(ref src) = self.src {
                    format!("<script src=\"{src}\"{id_attr}{nonce_attr}></script>")
                } else if let Some(ref content) = self.content {
                    format!("<script{id_attr}{nonce_attr}>{content}</script>")
                } else {
                    String::new()
                }
            }

            ScriptStrategy::Defer => {
                if let Some(ref src) = self.src {
                    format!("<script src=\"{src}\" defer{id_attr}{nonce_attr}></script>")
                } else if let Some(ref content) = self.content {
                    // Inline scripts can't defer, use DOMContentLoaded wrapper
                    format!(
                        "<script{id_attr}{nonce_attr}>document.addEventListener('DOMContentLoaded',function(){{{content}}});</script>"
                    )
                } else {
                    String::new()
                }
            }

            ScriptStrategy::Async => {
                if let Some(ref src) = self.src {
                    format!("<script src=\"{src}\" async{id_attr}{nonce_attr}></script>")
                } else if let Some(ref content) = self.content {
                    // Inline async: execute on next microtask
                    format!(
                        "<script{id_attr}{nonce_attr}>Promise.resolve().then(function(){{{content}}});</script>"
                    )
                } else {
                    String::new()
                }
            }

            ScriptStrategy::Module => {
                if let Some(ref src) = self.src {
                    format!("<script type=\"module\" src=\"{src}\"{id_attr}{nonce_attr}></script>")
                } else if let Some(ref content) = self.content {
                    format!("<script type=\"module\"{id_attr}{nonce_attr}>{content}</script>")
                } else {
                    String::new()
                }
            }

            ScriptStrategy::AfterInteractive => {
                let payload = if let Some(ref src) = self.src {
                    format!(
                        "var s=document.createElement('script');s.src='{src}';document.body.appendChild(s);"
                    )
                } else if let Some(ref content) = self.content {
                    content.clone()
                } else {
                    return String::new();
                };

                format!(
                    "<script{id_attr}{nonce_attr}>\
                    'requestIdleCallback' in window\
                    ?requestIdleCallback(function(){{{payload}}})\
                    :setTimeout(function(){{{payload}}},1)\
                    ;</script>"
                )
            }

            ScriptStrategy::Worker => {
                if let Some(ref src) = self.src {
                    format!(
                        "<script{id_attr}{nonce_attr}>new Worker('{src}');</script>"
                    )
                } else if let Some(ref content) = self.content {
                    format!(
                        "<script{id_attr}{nonce_attr}>\
                        new Worker(URL.createObjectURL(new Blob([`{content}`],{{type:'text/javascript'}})));\
                        </script>"
                    )
                } else {
                    String::new()
                }
            }
        }
    }

    /// Generate a preload link for external scripts (Defer/Module strategies).
    pub fn render_preload(&self) -> String {
        match (&self.src, &self.strategy) {
            (Some(src), ScriptStrategy::Defer | ScriptStrategy::Module) => {
                let as_type = match self.strategy {
                    ScriptStrategy::Module => "modulepreload",
                    _ => "preload",
                };
                if as_type == "modulepreload" {
                    format!("<link rel=\"modulepreload\" href=\"{src}\" />")
                } else {
                    format!("<link rel=\"preload\" href=\"{src}\" as=\"script\" />")
                }
            }
            _ => String::new(),
        }
    }
}

/// Generate the navigation prefetch script.
///
/// Injects a small inline script that:
/// 1. On link hover (mouseenter), creates a `<link rel="prefetch">` for the href
/// 2. Uses IntersectionObserver to prefetch visible links
/// 3. Deduplicates prefetch requests
///
/// This mimics Next.js `<Link>` prefetch behavior without requiring client-side JS framework.
pub fn navigation_prefetch_script() -> String {
    r#"<script>
(function(){
var p=new Set();
function pf(u){
if(p.has(u)||u===location.pathname)return;
p.add(u);
var l=document.createElement('link');
l.rel='prefetch';l.href=u;
document.head.appendChild(l)
}
document.addEventListener('mouseover',function(e){
var a=e.target.closest('a[href]');
if(!a)return;
var h=a.getAttribute('href');
if(h&&h.startsWith('/')&&!h.startsWith('//'))pf(h)
},true);
if('IntersectionObserver' in window){
var o=new IntersectionObserver(function(es){
es.forEach(function(e){
if(!e.isIntersecting)return;
var a=e.target;var h=a.getAttribute('href');
if(h&&h.startsWith('/')&&!h.startsWith('//'))pf(h);
o.unobserve(a)
})
},{rootMargin:'200px'});
document.querySelectorAll('a[href^="/"]').forEach(function(a){o.observe(a)})
}
})();
</script>"#
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_defer_script() {
        let s = OptimizedScript::external("/js/app.js").strategy(ScriptStrategy::Defer);
        let html = s.render();
        assert!(html.contains("defer"));
        assert!(html.contains("src=\"/js/app.js\""));
    }

    #[test]
    fn test_async_script() {
        let s = OptimizedScript::external("/js/analytics.js").strategy(ScriptStrategy::Async);
        let html = s.render();
        assert!(html.contains("async"));
    }

    #[test]
    fn test_module_script() {
        let s = OptimizedScript::external("/js/app.mjs").strategy(ScriptStrategy::Module);
        let html = s.render();
        assert!(html.contains("type=\"module\""));
    }

    #[test]
    fn test_after_interactive() {
        let s = OptimizedScript::external("/js/lazy.js").strategy(ScriptStrategy::AfterInteractive);
        let html = s.render();
        assert!(html.contains("requestIdleCallback"));
    }

    #[test]
    fn test_worker_script() {
        let s = OptimizedScript::external("/js/calc.js").strategy(ScriptStrategy::Worker);
        let html = s.render();
        assert!(html.contains("new Worker"));
    }

    #[test]
    fn test_inline_defer() {
        let s = OptimizedScript::inline("console.log('hi')").strategy(ScriptStrategy::Defer);
        let html = s.render();
        assert!(html.contains("DOMContentLoaded"));
    }

    #[test]
    fn test_navigation_prefetch() {
        let html = navigation_prefetch_script();
        assert!(html.contains("prefetch"));
        assert!(html.contains("IntersectionObserver"));
        assert!(html.contains("mouseover"));
    }

    #[test]
    fn test_script_preload() {
        let s = OptimizedScript::external("/js/app.js").strategy(ScriptStrategy::Defer);
        let preload = s.render_preload();
        assert!(preload.contains("rel=\"preload\""));
        assert!(preload.contains("as=\"script\""));
    }

    #[test]
    fn test_module_preload() {
        let s = OptimizedScript::external("/js/app.mjs").strategy(ScriptStrategy::Module);
        let preload = s.render_preload();
        assert!(preload.contains("modulepreload"));
    }
}
