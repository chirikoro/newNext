//! View Transitions API Support for Hayabusa.
//!
//! Enables smooth, animated page transitions using the browser's
//! View Transitions API. This creates native-feeling navigation
//! without full-page reloads.
//!
//! ## Impact on User-Perceived Speed
//! - Pages feel instant because transitions mask loading time
//! - Smooth crossfade/morph between old and new content
//! - Works with MPA (multi-page app) via Navigation API
//!
//! ## Usage
//! ```ignore
//! let vt = ViewTransitionConfig::new()
//!     .enable_cross_document()
//!     .transition("fade", Duration::from_millis(200))
//!     .transition("slide-left", Duration::from_millis(300))
//!     .morph_element(".hero-image");
//! ```

use std::time::Duration;

/// View Transition configuration
#[derive(Debug, Clone)]
pub struct ViewTransitionConfig {
    /// Enable cross-document view transitions (MPA)
    pub cross_document: bool,
    /// Named transitions with durations
    pub transitions: Vec<TransitionDef>,
    /// Elements that should morph between pages (via view-transition-name)
    pub morph_elements: Vec<String>,
    /// Default transition duration
    pub default_duration: Duration,
    /// Reduce motion preference support
    pub respect_reduced_motion: bool,
}

/// A named transition definition
#[derive(Debug, Clone)]
pub struct TransitionDef {
    pub name: String,
    pub duration: Duration,
    pub easing: String,
}

impl Default for ViewTransitionConfig {
    fn default() -> Self {
        Self {
            cross_document: false,
            transitions: Vec::new(),
            morph_elements: Vec::new(),
            default_duration: Duration::from_millis(250),
            respect_reduced_motion: true,
        }
    }
}

impl ViewTransitionConfig {
    pub fn new() -> Self {
        Self::default()
    }

    /// Enable cross-document view transitions (for MPA navigation)
    pub fn enable_cross_document(mut self) -> Self {
        self.cross_document = true;
        self
    }

    /// Add a named transition
    pub fn transition(mut self, name: impl Into<String>, duration: Duration) -> Self {
        self.transitions.push(TransitionDef {
            name: name.into(),
            duration,
            easing: "ease-in-out".to_string(),
        });
        self
    }

    /// Add a named transition with custom easing
    pub fn transition_with_easing(
        mut self,
        name: impl Into<String>,
        duration: Duration,
        easing: impl Into<String>,
    ) -> Self {
        self.transitions.push(TransitionDef {
            name: name.into(),
            duration,
            easing: easing.into(),
        });
        self
    }

    /// Mark an element for morphing between pages (assigns view-transition-name)
    pub fn morph_element(mut self, selector: impl Into<String>) -> Self {
        self.morph_elements.push(selector.into());
        self
    }

    /// Set default transition duration
    pub fn default_duration(mut self, duration: Duration) -> Self {
        self.default_duration = duration;
        self
    }

    /// Generate the CSS for view transitions
    pub fn render_css(&self) -> String {
        let mut css = String::new();

        // Default crossfade transition
        let default_ms = self.default_duration.as_millis();
        css.push_str(&format!(
            "::view-transition-old(root),::view-transition-new(root){{animation-duration:{}ms}}\n",
            default_ms
        ));

        // Named transitions
        for t in &self.transitions {
            let ms = t.duration.as_millis();
            match t.name.as_str() {
                "fade" => {
                    css.push_str(&format!(
                        "@keyframes fade-in{{from{{opacity:0}}to{{opacity:1}}}}\
                        @keyframes fade-out{{from{{opacity:1}}to{{opacity:0}}}}\
                        ::view-transition-old(root){{animation:fade-out {}ms {}}}\
                        ::view-transition-new(root){{animation:fade-in {}ms {}}}\n",
                        ms, t.easing, ms, t.easing
                    ));
                }
                "slide-left" => {
                    css.push_str(&format!(
                        "@keyframes slide-to-left{{from{{transform:translateX(0)}}to{{transform:translateX(-100%)}}}}\
                        @keyframes slide-from-right{{from{{transform:translateX(100%)}}to{{transform:translateX(0)}}}}\
                        ::view-transition-old(root){{animation:slide-to-left {}ms {}}}\
                        ::view-transition-new(root){{animation:slide-from-right {}ms {}}}\n",
                        ms, t.easing, ms, t.easing
                    ));
                }
                "slide-up" => {
                    css.push_str(&format!(
                        "@keyframes slide-to-top{{from{{transform:translateY(0)}}to{{transform:translateY(-100%)}}}}\
                        @keyframes slide-from-bottom{{from{{transform:translateY(100%)}}to{{transform:translateY(0)}}}}\
                        ::view-transition-old(root){{animation:slide-to-top {}ms {}}}\
                        ::view-transition-new(root){{animation:slide-from-bottom {}ms {}}}\n",
                        ms, t.easing, ms, t.easing
                    ));
                }
                name => {
                    // Custom named transition group
                    css.push_str(&format!(
                        "::view-transition-old({name}),::view-transition-new({name}){{animation-duration:{}ms;animation-timing-function:{}}}\n",
                        ms, t.easing
                    ));
                }
            }
        }

        // Morph elements: assign view-transition-name
        for (i, selector) in self.morph_elements.iter().enumerate() {
            let vt_name = format!("morph-{}", i);
            css.push_str(&format!(
                "{}{{view-transition-name:{}}}\n",
                selector, vt_name
            ));
        }

        // Respect reduced motion
        if self.respect_reduced_motion {
            css.push_str(
                "@media(prefers-reduced-motion:reduce){::view-transition-group(*),::view-transition-old(*),::view-transition-new(*){animation-duration:0.01ms!important}}\n"
            );
        }

        css
    }

    /// Generate the JavaScript for MPA view transitions
    pub fn render_script(&self) -> String {
        if !self.cross_document {
            return String::new();
        }

        r#"<script>
if(document.startViewTransition){
document.addEventListener('click',function(e){
var a=e.target.closest('a');
if(!a||a.origin!==location.origin||a.hasAttribute('data-no-transition'))return;
e.preventDefault();
var href=a.href;
document.startViewTransition(function(){
return fetch(href).then(function(r){return r.text()}).then(function(html){
var d=new DOMParser().parseFromString(html,'text/html');
document.title=d.title;
document.querySelector('main').innerHTML=d.querySelector('main').innerHTML;
history.pushState(null,'',href);
})});
});
window.addEventListener('popstate',function(){
document.startViewTransition(function(){
return fetch(location.href).then(function(r){return r.text()}).then(function(html){
var d=new DOMParser().parseFromString(html,'text/html');
document.title=d.title;
document.querySelector('main').innerHTML=d.querySelector('main').innerHTML;
})});
});
}
</script>"#.to_string()
    }

    /// Generate the <meta> tag for cross-document view transitions
    pub fn render_meta(&self) -> String {
        if self.cross_document {
            "<meta name=\"view-transition\" content=\"same-origin\" />\n".to_string()
        } else {
            String::new()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_css() {
        let vt = ViewTransitionConfig::new();
        let css = vt.render_css();
        assert!(css.contains("::view-transition-old(root)"));
        assert!(css.contains("250ms"));
        assert!(css.contains("prefers-reduced-motion"));
    }

    #[test]
    fn test_fade_transition() {
        let vt = ViewTransitionConfig::new()
            .transition("fade", Duration::from_millis(200));
        let css = vt.render_css();
        assert!(css.contains("fade-in"));
        assert!(css.contains("fade-out"));
        assert!(css.contains("200ms"));
    }

    #[test]
    fn test_slide_transition() {
        let vt = ViewTransitionConfig::new()
            .transition("slide-left", Duration::from_millis(300));
        let css = vt.render_css();
        assert!(css.contains("slide-to-left"));
        assert!(css.contains("slide-from-right"));
        assert!(css.contains("300ms"));
    }

    #[test]
    fn test_morph_elements() {
        let vt = ViewTransitionConfig::new()
            .morph_element(".hero-image")
            .morph_element(".logo");
        let css = vt.render_css();
        assert!(css.contains(".hero-image{view-transition-name:morph-0}"));
        assert!(css.contains(".logo{view-transition-name:morph-1}"));
    }

    #[test]
    fn test_cross_document_meta() {
        let vt = ViewTransitionConfig::new().enable_cross_document();
        let meta = vt.render_meta();
        assert!(meta.contains("view-transition"));
        assert!(meta.contains("same-origin"));
    }

    #[test]
    fn test_cross_document_script() {
        let vt = ViewTransitionConfig::new().enable_cross_document();
        let script = vt.render_script();
        assert!(script.contains("startViewTransition"));
        assert!(script.contains("popstate"));
    }

    #[test]
    fn test_no_script_without_cross_document() {
        let vt = ViewTransitionConfig::new();
        assert!(vt.render_script().is_empty());
        assert!(vt.render_meta().is_empty());
    }
}
