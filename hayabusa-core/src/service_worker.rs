//! Service Worker / PWA Support for Hayabusa.
//!
//! Generates service worker scripts for offline support and caching strategies.
//! Enables Progressive Web App (PWA) features.
//!
//! ## Caching Strategies
//! - **CacheFirst**: Serve from cache, fall back to network (static assets)
//! - **NetworkFirst**: Try network first, fall back to cache (API/pages)
//! - **StaleWhileRevalidate**: Serve from cache, update in background
//! - **NetworkOnly**: Always fetch from network
//! - **CacheOnly**: Only serve from cache
//!
//! ## Usage
//! ```ignore
//! let sw = ServiceWorkerConfig::new("v1")
//!     .cache_first(&["/fonts/*", "/images/*", "*.css", "*.js"])
//!     .network_first(&["/api/*", "/blog/*"])
//!     .stale_while_revalidate(&["/"])
//!     .offline_page("/offline.html");
//! ```

/// Service Worker configuration
#[derive(Debug, Clone)]
pub struct ServiceWorkerConfig {
    /// Cache version (change to bust cache)
    pub cache_version: String,
    /// Precache URLs (downloaded during SW install)
    pub precache: Vec<String>,
    /// Cache-first patterns (static assets)
    pub cache_first_patterns: Vec<String>,
    /// Network-first patterns (dynamic content)
    pub network_first_patterns: Vec<String>,
    /// Stale-while-revalidate patterns
    pub swr_patterns: Vec<String>,
    /// Offline fallback page
    pub offline_page: Option<String>,
    /// Max cache age in seconds
    pub max_age: u64,
    /// Max cache entries
    pub max_entries: usize,
}

impl ServiceWorkerConfig {
    pub fn new(version: impl Into<String>) -> Self {
        Self {
            cache_version: version.into(),
            precache: Vec::new(),
            cache_first_patterns: Vec::new(),
            network_first_patterns: Vec::new(),
            swr_patterns: Vec::new(),
            offline_page: None,
            max_age: 30 * 24 * 3600, // 30 days
            max_entries: 100,
        }
    }

    /// Add URLs to precache (downloaded during SW install)
    pub fn precache(mut self, urls: &[&str]) -> Self {
        self.precache.extend(urls.iter().map(|s| s.to_string()));
        self
    }

    /// Add patterns for cache-first strategy (best for static assets)
    pub fn cache_first(mut self, patterns: &[&str]) -> Self {
        self.cache_first_patterns
            .extend(patterns.iter().map(|s| s.to_string()));
        self
    }

    /// Add patterns for network-first strategy (best for dynamic content)
    pub fn network_first(mut self, patterns: &[&str]) -> Self {
        self.network_first_patterns
            .extend(patterns.iter().map(|s| s.to_string()));
        self
    }

    /// Add patterns for stale-while-revalidate strategy
    pub fn stale_while_revalidate(mut self, patterns: &[&str]) -> Self {
        self.swr_patterns
            .extend(patterns.iter().map(|s| s.to_string()));
        self
    }

    /// Set the offline fallback page
    pub fn offline_page(mut self, url: impl Into<String>) -> Self {
        self.offline_page = Some(url.into());
        self
    }

    /// Set maximum cache age
    pub fn max_age_secs(mut self, secs: u64) -> Self {
        self.max_age = secs;
        self
    }

    /// Generate the service worker JavaScript
    pub fn generate_sw_js(&self) -> String {
        let cache_name = format!("hayabusa-{}", self.cache_version);
        let precache_json: Vec<String> = self.precache.iter().map(|u| format!("'{}'", u)).collect();
        let offline = self.offline_page.as_deref().unwrap_or("/offline.html");

        let cache_first = self.patterns_to_regex(&self.cache_first_patterns);
        let network_first = self.patterns_to_regex(&self.network_first_patterns);
        let swr = self.patterns_to_regex(&self.swr_patterns);

        format!(
            r#"// Hayabusa Service Worker {version}
const CACHE='{cache_name}';
const PRECACHE=[{precache}];
const OFFLINE='{offline}';

self.addEventListener('install',e=>{{
  e.waitUntil(caches.open(CACHE).then(c=>c.addAll([...PRECACHE,OFFLINE])));
  self.skipWaiting();
}});

self.addEventListener('activate',e=>{{
  e.waitUntil(caches.keys().then(ks=>Promise.all(ks.filter(k=>k!==CACHE).map(k=>caches.delete(k)))));
  self.clients.claim();
}});

const CF=[{cache_first}];
const NF=[{network_first}];
const SWR=[{swr}];

function matchPatterns(url,patterns){{return patterns.some(p=>p.test(url))}}

self.addEventListener('fetch',e=>{{
  const u=new URL(e.request.url);
  if(e.request.method!=='GET')return;

  // Cache-first
  if(matchPatterns(u.pathname,CF)){{
    e.respondWith(caches.match(e.request).then(r=>r||fetch(e.request).then(nr=>{{
      const c=nr.clone();caches.open(CACHE).then(ca=>ca.put(e.request,c));return nr;
    }})));
    return;
  }}

  // Stale-while-revalidate
  if(matchPatterns(u.pathname,SWR)){{
    e.respondWith(caches.match(e.request).then(r=>{{
      const f=fetch(e.request).then(nr=>{{
        const c=nr.clone();caches.open(CACHE).then(ca=>ca.put(e.request,c));return nr;
      }});
      return r||f;
    }}));
    return;
  }}

  // Network-first
  if(matchPatterns(u.pathname,NF)){{
    e.respondWith(fetch(e.request).then(r=>{{
      const c=r.clone();caches.open(CACHE).then(ca=>ca.put(e.request,c));return r;
    }}).catch(()=>caches.match(e.request).then(r=>r||caches.match(OFFLINE))));
    return;
  }}

  // Default: network with cache fallback
  e.respondWith(fetch(e.request).catch(()=>caches.match(e.request).then(r=>r||caches.match(OFFLINE))));
}});"#,
            version = self.cache_version,
            cache_name = cache_name,
            precache = precache_json.join(","),
            offline = offline,
            cache_first = cache_first,
            network_first = network_first,
            swr = swr,
        )
    }

    /// Convert glob patterns to JS regex strings
    fn patterns_to_regex(&self, patterns: &[String]) -> String {
        patterns
            .iter()
            .map(|p| {
                let regex = glob_to_regex(p);
                format!("new RegExp('{}')", regex)
            })
            .collect::<Vec<_>>()
            .join(",")
    }

    /// Generate the registration script to include in HTML
    pub fn render_registration_script(&self) -> String {
        "<script>if('serviceWorker' in navigator){navigator.serviceWorker.register('/sw.js')}</script>".to_string()
    }
}

impl Default for ServiceWorkerConfig {
    fn default() -> Self {
        Self::new("v1")
    }
}

/// PWA manifest configuration
#[derive(Debug, Clone)]
pub struct PwaManifest {
    pub name: String,
    pub short_name: String,
    pub start_url: String,
    pub display: PwaDisplay,
    pub background_color: String,
    pub theme_color: String,
    pub icons: Vec<PwaIcon>,
}

#[derive(Debug, Clone)]
pub enum PwaDisplay {
    Standalone,
    Fullscreen,
    MinimalUi,
    Browser,
}

impl PwaDisplay {
    fn as_str(&self) -> &str {
        match self {
            PwaDisplay::Standalone => "standalone",
            PwaDisplay::Fullscreen => "fullscreen",
            PwaDisplay::MinimalUi => "minimal-ui",
            PwaDisplay::Browser => "browser",
        }
    }
}

#[derive(Debug, Clone)]
pub struct PwaIcon {
    pub src: String,
    pub sizes: String,
    pub icon_type: String,
}

impl PwaManifest {
    pub fn new(name: impl Into<String>) -> Self {
        let name = name.into();
        let short_name = name.clone();
        Self {
            name,
            short_name,
            start_url: "/".to_string(),
            display: PwaDisplay::Standalone,
            background_color: "#ffffff".to_string(),
            theme_color: "#000000".to_string(),
            icons: Vec::new(),
        }
    }

    pub fn short_name(mut self, name: impl Into<String>) -> Self {
        self.short_name = name.into();
        self
    }

    pub fn theme_color(mut self, color: impl Into<String>) -> Self {
        self.theme_color = color.into();
        self
    }

    pub fn background_color(mut self, color: impl Into<String>) -> Self {
        self.background_color = color.into();
        self
    }

    pub fn icon(mut self, src: impl Into<String>, sizes: impl Into<String>, icon_type: impl Into<String>) -> Self {
        self.icons.push(PwaIcon {
            src: src.into(),
            sizes: sizes.into(),
            icon_type: icon_type.into(),
        });
        self
    }

    /// Generate the manifest.json content
    pub fn to_json(&self) -> String {
        let icons: Vec<String> = self.icons.iter().map(|i| {
            format!(r#"{{"src":"{}","sizes":"{}","type":"{}"}}"#, i.src, i.sizes, i.icon_type)
        }).collect();

        format!(
            r#"{{"name":"{}","short_name":"{}","start_url":"{}","display":"{}","background_color":"{}","theme_color":"{}","icons":[{}]}}"#,
            self.name, self.short_name, self.start_url,
            self.display.as_str(), self.background_color, self.theme_color,
            icons.join(",")
        )
    }

    /// Generate the <link> tag for the manifest
    pub fn render_link_tag(&self) -> String {
        format!(
            "<link rel=\"manifest\" href=\"/manifest.json\" />\n<meta name=\"theme-color\" content=\"{}\" />",
            self.theme_color
        )
    }
}

/// Convert a simple glob pattern to a JavaScript regex string
fn glob_to_regex(glob: &str) -> String {
    let mut regex = String::new();
    regex.push('^');

    for ch in glob.chars() {
        match ch {
            '*' => regex.push_str(".*"),
            '.' => regex.push_str("\\."),
            '/' => regex.push_str("\\/"),
            '?' => regex.push('.'),
            _ => regex.push(ch),
        }
    }

    regex.push('$');
    regex
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_glob_to_regex() {
        assert_eq!(glob_to_regex("/fonts/*"), "^\\/fonts\\/.*$");
        assert_eq!(glob_to_regex("*.css"), "^.*\\.css$");
        assert_eq!(glob_to_regex("/api/*"), "^\\/api\\/.*$");
    }

    #[test]
    fn test_sw_generation() {
        let sw = ServiceWorkerConfig::new("v1")
            .precache(&["/", "/style.css"])
            .cache_first(&["/fonts/*", "*.css", "*.js"])
            .network_first(&["/api/*"])
            .stale_while_revalidate(&["/"]);

        let js = sw.generate_sw_js();
        assert!(js.contains("hayabusa-v1"));
        assert!(js.contains("PRECACHE"));
        assert!(js.contains("skipWaiting"));
        assert!(js.contains("Cache-first"));
        assert!(js.contains("Network-first"));
        assert!(js.contains("Stale-while-revalidate"));
    }

    #[test]
    fn test_sw_registration_script() {
        let sw = ServiceWorkerConfig::new("v1");
        let script = sw.render_registration_script();
        assert!(script.contains("serviceWorker"));
        assert!(script.contains("register"));
        assert!(script.contains("/sw.js"));
    }

    #[test]
    fn test_pwa_manifest() {
        let manifest = PwaManifest::new("My App")
            .short_name("App")
            .theme_color("#ff0000")
            .icon("/icon-192.png", "192x192", "image/png")
            .icon("/icon-512.png", "512x512", "image/png");

        let json = manifest.to_json();
        assert!(json.contains("\"My App\""));
        assert!(json.contains("\"App\""));
        assert!(json.contains("#ff0000"));
        assert!(json.contains("192x192"));
        assert!(json.contains("512x512"));
    }

    #[test]
    fn test_pwa_link_tag() {
        let manifest = PwaManifest::new("Test").theme_color("#333");
        let tag = manifest.render_link_tag();
        assert!(tag.contains("rel=\"manifest\""));
        assert!(tag.contains("#333"));
    }

    #[test]
    fn test_offline_page() {
        let sw = ServiceWorkerConfig::new("v2")
            .offline_page("/custom-offline.html");
        let js = sw.generate_sw_js();
        assert!(js.contains("/custom-offline.html"));
    }
}
