//! Lightweight Client-Side Interactivity for Hayabusa.
//!
//! Integrates with Alpine.js, htmx, and Petite-Vue to provide
//! client-side interactivity without heavy frameworks like React.
//!
//! ## Philosophy
//! Hayabusa serves fast HTML. For interactivity, we use the **lightest
//! possible tools** instead of shipping 200KB+ of React:
//!
//! | Library    | Size  | Approach                        |
//! |-----------|-------|---------------------------------|
//! | htmx      | 14KB  | Server returns HTML fragments   |
//! | Alpine.js | 15KB  | Declarative x-data attributes   |
//! | Petite-Vue| 6KB   | Vue-like reactivity, minimal    |
//!
//! These are **10-50x smaller than React** and require zero build step.

/// Client framework integration choice
#[derive(Debug, Clone, PartialEq)]
pub enum ClientFramework {
    /// htmx: Server-centric, returns HTML fragments
    Htmx,
    /// Alpine.js: Declarative, x-data driven
    Alpine,
    /// Petite-Vue: Vue-like reactivity, smallest
    PetiteVue,
}

/// Configuration for client-side interactivity
#[derive(Debug, Clone)]
pub struct InteractivityConfig {
    pub framework: ClientFramework,
    /// Use CDN or self-hosted
    pub cdn: bool,
    /// Custom CDN URL override
    pub custom_url: Option<String>,
    /// Enable htmx extensions
    pub htmx_extensions: Vec<String>,
}

impl InteractivityConfig {
    pub fn htmx() -> Self {
        Self {
            framework: ClientFramework::Htmx,
            cdn: true,
            custom_url: None,
            htmx_extensions: Vec::new(),
        }
    }

    pub fn alpine() -> Self {
        Self {
            framework: ClientFramework::Alpine,
            cdn: true,
            custom_url: None,
            htmx_extensions: Vec::new(),
        }
    }

    pub fn petite_vue() -> Self {
        Self {
            framework: ClientFramework::PetiteVue,
            cdn: true,
            custom_url: None,
            htmx_extensions: Vec::new(),
        }
    }

    /// Use a self-hosted URL instead of CDN
    pub fn self_hosted(mut self, url: impl Into<String>) -> Self {
        self.cdn = false;
        self.custom_url = Some(url.into());
        self
    }

    /// Add an htmx extension (e.g., "sse", "ws", "json-enc")
    pub fn htmx_ext(mut self, ext: impl Into<String>) -> Self {
        self.htmx_extensions.push(ext.into());
        self
    }

    /// Render the <script> tag to load the framework
    pub fn render_script(&self) -> String {
        let url = self.get_url();

        let mut script = match self.framework {
            ClientFramework::Htmx => {
                format!("<script src=\"{}\" defer></script>\n", url)
            }
            ClientFramework::Alpine => {
                format!("<script src=\"{}\" defer></script>\n", url)
            }
            ClientFramework::PetiteVue => {
                format!("<script src=\"{}\" defer init></script>\n", url)
            }
        };

        // htmx extensions
        for ext in &self.htmx_extensions {
            let ext_url = format!(
                "https://unpkg.com/htmx-ext-{ext}@latest/{ext}.js"
            );
            script.push_str(&format!(
                "<script src=\"{}\" defer></script>\n",
                ext_url
            ));
        }

        script
    }

    fn get_url(&self) -> String {
        if let Some(ref url) = self.custom_url {
            return url.clone();
        }

        match self.framework {
            ClientFramework::Htmx => {
                "https://unpkg.com/htmx.org@2/dist/htmx.min.js".to_string()
            }
            ClientFramework::Alpine => {
                "https://unpkg.com/alpinejs@3/dist/cdn.min.js".to_string()
            }
            ClientFramework::PetiteVue => {
                "https://unpkg.com/petite-vue@0.4/dist/petite-vue.iife.js".to_string()
            }
        }
    }
}

// ─────────────────────────────────────────────
//  htmx Helpers
// ─────────────────────────────────────────────

/// htmx attribute builder for server-driven interactivity.
///
/// Instead of client-side React state management, the server returns
/// HTML fragments and htmx swaps them into the DOM.
///
/// ## Example
/// ```ignore
/// // Rust server handler returns HTML fragment:
/// fn search_results(query: &str) -> String {
///     html! { <ul>{% for item in results %}<li>{{ item }}</li>{% endfor %}</ul> }
/// }
///
/// // Template uses htmx to call it:
/// // <input hx-get="/api/search" hx-trigger="keyup changed delay:300ms"
/// //        hx-target="#results" name="q" />
/// // <div id="results"></div>
/// ```
#[derive(Debug, Clone, Default)]
pub struct HtmxAttrs {
    attrs: Vec<(String, String)>,
}

impl HtmxAttrs {
    pub fn new() -> Self {
        Self::default()
    }

    /// GET request
    pub fn get(mut self, url: impl Into<String>) -> Self {
        self.attrs.push(("hx-get".into(), url.into()));
        self
    }

    /// POST request
    pub fn post(mut self, url: impl Into<String>) -> Self {
        self.attrs.push(("hx-post".into(), url.into()));
        self
    }

    /// PUT request
    pub fn put(mut self, url: impl Into<String>) -> Self {
        self.attrs.push(("hx-put".into(), url.into()));
        self
    }

    /// DELETE request
    pub fn delete(mut self, url: impl Into<String>) -> Self {
        self.attrs.push(("hx-delete".into(), url.into()));
        self
    }

    /// Target element selector
    pub fn target(mut self, selector: impl Into<String>) -> Self {
        self.attrs.push(("hx-target".into(), selector.into()));
        self
    }

    /// Swap strategy (innerHTML, outerHTML, beforeend, afterend, etc.)
    pub fn swap(mut self, strategy: impl Into<String>) -> Self {
        self.attrs.push(("hx-swap".into(), strategy.into()));
        self
    }

    /// Trigger event
    pub fn trigger(mut self, event: impl Into<String>) -> Self {
        self.attrs.push(("hx-trigger".into(), event.into()));
        self
    }

    /// Include additional values
    pub fn vals(mut self, json: impl Into<String>) -> Self {
        self.attrs.push(("hx-vals".into(), json.into()));
        self
    }

    /// Confirm before sending
    pub fn confirm(mut self, message: impl Into<String>) -> Self {
        self.attrs.push(("hx-confirm".into(), message.into()));
        self
    }

    /// Loading indicator
    pub fn indicator(mut self, selector: impl Into<String>) -> Self {
        self.attrs.push(("hx-indicator".into(), selector.into()));
        self
    }

    /// Push URL to history
    pub fn push_url(mut self) -> Self {
        self.attrs.push(("hx-push-url".into(), "true".into()));
        self
    }

    /// SSE connection (htmx-ext-sse)
    pub fn sse_connect(mut self, url: impl Into<String>) -> Self {
        self.attrs.push(("sse-connect".into(), url.into()));
        self
    }

    /// SSE swap on event
    pub fn sse_swap(mut self, event: impl Into<String>) -> Self {
        self.attrs.push(("sse-swap".into(), event.into()));
        self
    }

    /// Custom attribute
    pub fn attr(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.attrs.push((key.into(), value.into()));
        self
    }

    /// Render as HTML attribute string
    pub fn render(&self) -> String {
        self.attrs
            .iter()
            .map(|(k, v)| format!("{}=\"{}\"", k, v))
            .collect::<Vec<_>>()
            .join(" ")
    }
}

// ─────────────────────────────────────────────
//  Alpine.js Helpers
// ─────────────────────────────────────────────

/// Alpine.js component builder for declarative interactivity.
///
/// ## Example
/// ```ignore
/// // Counter component
/// let counter = AlpineComponent::new()
///     .data("count", "0")
///     .html(r#"
///         <span x-text="count"></span>
///         <button @click="count++">+</button>
///     "#);
/// ```
#[derive(Debug, Clone, Default)]
pub struct AlpineComponent {
    data: Vec<(String, String)>,
    init: Option<String>,
    html: String,
}

impl AlpineComponent {
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a reactive data property
    pub fn data(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.data.push((key.into(), value.into()));
        self
    }

    /// Add an init function
    pub fn init(mut self, code: impl Into<String>) -> Self {
        self.init = Some(code.into());
        self
    }

    /// Set the inner HTML template
    pub fn html(mut self, html: impl Into<String>) -> Self {
        self.html = html.into();
        self
    }

    /// Render as a <div x-data="..."> element
    pub fn render(&self) -> String {
        let data_obj: Vec<String> = self
            .data
            .iter()
            .map(|(k, v)| format!("{}: {}", k, v))
            .collect();

        let mut xdata = format!("{{ {} }}", data_obj.join(", "));

        if let Some(ref init_code) = self.init {
            // Wrap with init
            xdata = format!(
                "{{ {}, init() {{ {} }} }}",
                data_obj.join(", "),
                init_code
            );
        }

        format!(
            "<div x-data=\"{}\">{}</div>",
            xdata.replace('"', "&quot;"),
            self.html
        )
    }
}

// ─────────────────────────────────────────────
//  Common interactive patterns
// ─────────────────────────────────────────────

/// Generate a search input with live results (htmx pattern)
pub fn htmx_live_search(
    endpoint: &str,
    target: &str,
    placeholder: &str,
) -> String {
    format!(
        r#"<input type="search" name="q" placeholder="{placeholder}"
  hx-get="{endpoint}" hx-trigger="keyup changed delay:300ms"
  hx-target="{target}" hx-indicator=".htmx-indicator"
  autocomplete="off" />
<span class="htmx-indicator" style="display:none">Searching...</span>
<div id="{target_id}"></div>"#,
        placeholder = placeholder,
        endpoint = endpoint,
        target = target,
        target_id = target.trim_start_matches('#'),
    )
}

/// Generate an infinite scroll container (htmx pattern)
pub fn htmx_infinite_scroll(endpoint: &str, page_param: &str) -> String {
    format!(
        r#"<div id="infinite-list" hx-get="{endpoint}?{page_param}=1"
  hx-trigger="revealed" hx-swap="afterend"
  hx-indicator=".load-indicator">
</div>
<div class="load-indicator" style="display:none;text-align:center;padding:1rem">Loading...</div>"#,
        endpoint = endpoint,
        page_param = page_param,
    )
}

/// Generate a toggle/accordion component (Alpine.js pattern)
pub fn alpine_toggle(title: &str, content: &str) -> String {
    format!(
        r#"<div x-data="{{ open: false }}">
  <button @click="open = !open" style="cursor:pointer;width:100%;text-align:left;padding:0.75rem;border:1px solid #ddd;background:#f9f9f9;border-radius:4px">
    <span x-text="open ? '▼' : '▶'"></span> {title}
  </button>
  <div x-show="open" x-transition style="padding:0.75rem;border:1px solid #eee;border-top:none">
    {content}
  </div>
</div>"#,
        title = title,
        content = content,
    )
}

/// Generate a tab component (Alpine.js pattern)
pub fn alpine_tabs(tabs: &[(&str, &str)]) -> String {
    let tab_names: Vec<String> = tabs.iter().map(|(name, _)| format!("'{}'", name)).collect();

    let mut html = format!(
        "<div x-data=\"{{ activeTab: {} }}\">\n<nav style=\"display:flex;gap:0;border-bottom:2px solid #eee\">\n",
        tab_names.first().unwrap_or(&"''".to_string())
    );

    for (name, _) in tabs {
        html.push_str(&format!(
            "  <button @click=\"activeTab = '{name}'\" :style=\"activeTab === '{name}' ? 'border-bottom:2px solid #333;font-weight:600' : 'opacity:0.6'\" style=\"padding:0.75rem 1.5rem;background:none;border:none;cursor:pointer\">{name}</button>\n",
            name = name
        ));
    }
    html.push_str("</nav>\n");

    for (name, content) in tabs {
        html.push_str(&format!(
            "<div x-show=\"activeTab === '{name}'\" x-transition>{content}</div>\n",
            name = name,
            content = content,
        ));
    }

    html.push_str("</div>");
    html
}

/// Generate a modal dialog (Alpine.js pattern)
pub fn alpine_modal(trigger_text: &str, content: &str) -> String {
    format!(
        r#"<div x-data="{{ showModal: false }}">
  <button @click="showModal = true" style="cursor:pointer">{trigger}</button>
  <template x-if="showModal">
    <div style="position:fixed;inset:0;z-index:9999;display:flex;align-items:center;justify-content:center" @click.self="showModal = false">
      <div style="position:fixed;inset:0;background:rgba(0,0,0,0.5)" @click="showModal = false"></div>
      <div style="position:relative;background:#fff;border-radius:8px;padding:2rem;max-width:500px;width:90%;max-height:80vh;overflow:auto;z-index:1" @click.stop>
        <button @click="showModal = false" style="position:absolute;top:0.5rem;right:0.75rem;background:none;border:none;font-size:1.5rem;cursor:pointer">&times;</button>
        {content}
      </div>
    </div>
  </template>
</div>"#,
        trigger = trigger_text,
        content = content,
    )
}

/// Generate a toast notification system (Alpine.js pattern)
pub fn alpine_toast_system() -> String {
    r#"<div x-data="{
  toasts: [],
  add(msg, type = 'info') {
    const id = Date.now();
    this.toasts.push({ id, msg, type });
    setTimeout(() => this.toasts = this.toasts.filter(t => t.id !== id), 4000);
  }
}" @toast.window="add($event.detail.msg, $event.detail.type)"
  style="position:fixed;top:1rem;right:1rem;z-index:9999;display:flex;flex-direction:column;gap:0.5rem">
  <template x-for="toast in toasts" :key="toast.id">
    <div x-transition
      :style="'padding:0.75rem 1rem;border-radius:6px;color:#fff;min-width:250px;background:' +
        (toast.type === 'error' ? '#ef4444' : toast.type === 'success' ? '#22c55e' : '#3b82f6')"
      x-text="toast.msg"></div>
  </template>
</div>"#
        .to_string()
}

/// Generate a form with htmx submission (no page reload)
pub fn htmx_form(action: &str, method: &str, content: &str, target: &str) -> String {
    let hx_method = match method.to_lowercase().as_str() {
        "post" => "hx-post",
        "put" => "hx-put",
        "delete" => "hx-delete",
        _ => "hx-get",
    };

    format!(
        r#"<form {hx_method}="{action}" hx-target="{target}" hx-swap="innerHTML" hx-indicator=".form-indicator">
  {content}
  <span class="form-indicator" style="display:none">Submitting...</span>
</form>"#,
        hx_method = hx_method,
        action = action,
        target = target,
        content = content,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_htmx_config() {
        let config = InteractivityConfig::htmx();
        let script = config.render_script();
        assert!(script.contains("htmx"));
        assert!(script.contains("defer"));
    }

    #[test]
    fn test_alpine_config() {
        let config = InteractivityConfig::alpine();
        let script = config.render_script();
        assert!(script.contains("alpinejs"));
    }

    #[test]
    fn test_petite_vue_config() {
        let config = InteractivityConfig::petite_vue();
        let script = config.render_script();
        assert!(script.contains("petite-vue"));
        assert!(script.contains("init"));
    }

    #[test]
    fn test_self_hosted() {
        let config = InteractivityConfig::htmx().self_hosted("/js/htmx.min.js");
        let script = config.render_script();
        assert!(script.contains("/js/htmx.min.js"));
        assert!(!script.contains("unpkg"));
    }

    #[test]
    fn test_htmx_attrs() {
        let attrs = HtmxAttrs::new()
            .get("/api/search")
            .target("#results")
            .trigger("keyup changed delay:300ms")
            .swap("innerHTML");

        let rendered = attrs.render();
        assert!(rendered.contains("hx-get=\"/api/search\""));
        assert!(rendered.contains("hx-target=\"#results\""));
        assert!(rendered.contains("hx-trigger=\"keyup changed delay:300ms\""));
        assert!(rendered.contains("hx-swap=\"innerHTML\""));
    }

    #[test]
    fn test_htmx_post() {
        let attrs = HtmxAttrs::new()
            .post("/api/submit")
            .confirm("Are you sure?")
            .indicator("#spinner");

        let rendered = attrs.render();
        assert!(rendered.contains("hx-post=\"/api/submit\""));
        assert!(rendered.contains("hx-confirm"));
        assert!(rendered.contains("hx-indicator"));
    }

    #[test]
    fn test_alpine_component() {
        let component = AlpineComponent::new()
            .data("count", "0")
            .data("name", "'World'")
            .html("<span x-text=\"count\"></span>");

        let rendered = component.render();
        assert!(rendered.contains("x-data"));
        assert!(rendered.contains("count: 0"));
        assert!(rendered.contains("name: 'World'"));
        assert!(rendered.contains("x-text"));
    }

    #[test]
    fn test_alpine_component_with_init() {
        let component = AlpineComponent::new()
            .data("items", "[]")
            .init("this.items = await (await fetch('/api/items')).json()")
            .html("<ul></ul>");

        let rendered = component.render();
        assert!(rendered.contains("init()"));
        assert!(rendered.contains("fetch"));
    }

    #[test]
    fn test_htmx_live_search() {
        let html = htmx_live_search("/api/search", "#results", "Search...");
        assert!(html.contains("hx-get=\"/api/search\""));
        assert!(html.contains("delay:300ms"));
        assert!(html.contains("hx-target=\"#results\""));
        assert!(html.contains("hx-indicator"));
    }

    #[test]
    fn test_htmx_infinite_scroll() {
        let html = htmx_infinite_scroll("/api/items", "page");
        assert!(html.contains("hx-trigger=\"revealed\""));
        assert!(html.contains("hx-swap=\"afterend\""));
    }

    #[test]
    fn test_alpine_toggle() {
        let html = alpine_toggle("FAQ Question", "Answer here");
        assert!(html.contains("x-data"));
        assert!(html.contains("@click=\"open = !open\""));
        assert!(html.contains("x-show=\"open\""));
        assert!(html.contains("FAQ Question"));
    }

    #[test]
    fn test_alpine_tabs() {
        let html = alpine_tabs(&[
            ("Home", "<p>Home content</p>"),
            ("About", "<p>About content</p>"),
        ]);
        assert!(html.contains("activeTab"));
        assert!(html.contains("Home"));
        assert!(html.contains("About"));
        assert!(html.contains("x-show"));
    }

    #[test]
    fn test_alpine_modal() {
        let html = alpine_modal("Open", "<p>Modal content</p>");
        assert!(html.contains("showModal"));
        assert!(html.contains("@click.self=\"showModal = false\""));
        assert!(html.contains("Modal content"));
    }

    #[test]
    fn test_alpine_toast_system() {
        let html = alpine_toast_system();
        assert!(html.contains("toasts"));
        assert!(html.contains("@toast.window"));
        assert!(html.contains("setTimeout"));
        assert!(html.contains("x-transition"));
    }

    #[test]
    fn test_htmx_form() {
        let html = htmx_form("/api/submit", "post", "<input name=\"email\" />", "#result");
        assert!(html.contains("hx-post=\"/api/submit\""));
        assert!(html.contains("hx-target=\"#result\""));
        assert!(html.contains("hx-swap=\"innerHTML\""));
    }

    #[test]
    fn test_htmx_extensions() {
        let config = InteractivityConfig::htmx()
            .htmx_ext("sse")
            .htmx_ext("json-enc");
        let script = config.render_script();
        assert!(script.contains("htmx-ext-sse"));
        assert!(script.contains("htmx-ext-json-enc"));
    }
}
