//! Privacy-Focused Analytics for Hayabusa.
//!
//! Lightweight page view and event tracking without cookies.
//! GDPR-friendly, no personal data collection.
//!
//! ```ignore
//! use hayabusa_core::prelude::*;
//!
//! let analytics = AnalyticsCollector::new();
//! analytics.track_page_view("/", "Mozilla/5.0...", None);
//! let report = analytics.report();
//! ```

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use dashmap::DashMap;

// ─── Analytics Collector ───────────────────────────────────

/// Privacy-focused analytics collector
#[derive(Debug, Clone)]
pub struct AnalyticsCollector {
    page_views: Arc<DashMap<String, AtomicU64>>,
    referrers: Arc<DashMap<String, AtomicU64>>,
    events: Arc<DashMap<String, AtomicU64>>,
    browsers: Arc<DashMap<String, AtomicU64>>,
    countries: Arc<DashMap<String, AtomicU64>>,
    total_views: Arc<AtomicU64>,
    total_visitors: Arc<AtomicU64>,
}

impl AnalyticsCollector {
    pub fn new() -> Self {
        Self {
            page_views: Arc::new(DashMap::new()),
            referrers: Arc::new(DashMap::new()),
            events: Arc::new(DashMap::new()),
            browsers: Arc::new(DashMap::new()),
            countries: Arc::new(DashMap::new()),
            total_views: Arc::new(AtomicU64::new(0)),
            total_visitors: Arc::new(AtomicU64::new(0)),
        }
    }

    /// Track a page view
    pub fn track_page_view(&self, path: &str, user_agent: &str, referrer: Option<&str>) {
        self.total_views.fetch_add(1, Ordering::Relaxed);

        // Track page
        self.page_views.entry(path.to_string())
            .or_insert_with(|| AtomicU64::new(0))
            .fetch_add(1, Ordering::Relaxed);

        // Track referrer
        if let Some(ref_url) = referrer {
            if !ref_url.is_empty() {
                let domain = extract_domain(ref_url);
                self.referrers.entry(domain)
                    .or_insert_with(|| AtomicU64::new(0))
                    .fetch_add(1, Ordering::Relaxed);
            }
        }

        // Track browser (simplified)
        let browser = detect_browser(user_agent);
        self.browsers.entry(browser)
            .or_insert_with(|| AtomicU64::new(0))
            .fetch_add(1, Ordering::Relaxed);
    }

    /// Track a custom event
    pub fn track_event(&self, name: &str) {
        self.events.entry(name.to_string())
            .or_insert_with(|| AtomicU64::new(0))
            .fetch_add(1, Ordering::Relaxed);
    }

    /// Track a country (from GeoIP or Accept-Language)
    pub fn track_country(&self, country_code: &str) {
        self.countries.entry(country_code.to_uppercase())
            .or_insert_with(|| AtomicU64::new(0))
            .fetch_add(1, Ordering::Relaxed);
    }

    /// Get page view count for a path
    pub fn views_for(&self, path: &str) -> u64 {
        self.page_views.get(path)
            .map(|v| v.load(Ordering::Relaxed))
            .unwrap_or(0)
    }

    /// Get event count
    pub fn event_count(&self, name: &str) -> u64 {
        self.events.get(name)
            .map(|v| v.load(Ordering::Relaxed))
            .unwrap_or(0)
    }

    /// Get total page views
    pub fn total_views(&self) -> u64 {
        self.total_views.load(Ordering::Relaxed)
    }

    /// Get top pages by views
    pub fn top_pages(&self, limit: usize) -> Vec<(String, u64)> {
        let mut pages: Vec<(String, u64)> = self.page_views.iter()
            .map(|e| (e.key().clone(), e.value().load(Ordering::Relaxed)))
            .collect();
        pages.sort_by(|a, b| b.1.cmp(&a.1));
        pages.truncate(limit);
        pages
    }

    /// Get top referrers
    pub fn top_referrers(&self, limit: usize) -> Vec<(String, u64)> {
        let mut refs: Vec<(String, u64)> = self.referrers.iter()
            .map(|e| (e.key().clone(), e.value().load(Ordering::Relaxed)))
            .collect();
        refs.sort_by(|a, b| b.1.cmp(&a.1));
        refs.truncate(limit);
        refs
    }

    /// Get browser breakdown
    pub fn browser_stats(&self) -> Vec<(String, u64)> {
        let mut stats: Vec<(String, u64)> = self.browsers.iter()
            .map(|e| (e.key().clone(), e.value().load(Ordering::Relaxed)))
            .collect();
        stats.sort_by(|a, b| b.1.cmp(&a.1));
        stats
    }

    /// Generate analytics report as JSON
    pub fn report_json(&self) -> String {
        let pages: Vec<String> = self.top_pages(10).iter()
            .map(|(p, v)| format!("{{\"path\":\"{}\",\"views\":{}}}", p, v))
            .collect();
        let refs: Vec<String> = self.top_referrers(10).iter()
            .map(|(r, v)| format!("{{\"domain\":\"{}\",\"visits\":{}}}", r, v))
            .collect();
        let browsers: Vec<String> = self.browser_stats().iter()
            .map(|(b, v)| format!("{{\"browser\":\"{}\",\"count\":{}}}", b, v))
            .collect();

        format!(
            "{{\"total_views\":{},\"pages\":[{}],\"referrers\":[{}],\"browsers\":[{}]}}",
            self.total_views(),
            pages.join(","),
            refs.join(","),
            browsers.join(","),
        )
    }

    /// Clear all analytics data
    pub fn clear(&self) {
        self.page_views.clear();
        self.referrers.clear();
        self.events.clear();
        self.browsers.clear();
        self.countries.clear();
        self.total_views.store(0, Ordering::Relaxed);
        self.total_visitors.store(0, Ordering::Relaxed);
    }
}

impl Default for AnalyticsCollector {
    fn default() -> Self {
        Self::new()
    }
}

// ─── Client-Side Tracking Script ───────────────────────────

/// Generate a privacy-focused analytics tracking script
pub fn analytics_script(endpoint: &str) -> String {
    format!(
        r#"<script>
(function(){{
  var ep='{endpoint}';
  function send(t,d){{
    var x=new XMLHttpRequest();
    x.open('POST',ep,true);
    x.setRequestHeader('Content-Type','application/json');
    x.send(JSON.stringify({{type:t,data:d,ts:Date.now()}}));
  }}
  send('pageview',{{
    path:location.pathname,
    referrer:document.referrer||null,
    screen:screen.width+'x'+screen.height,
    lang:navigator.language
  }});
  window.trackEvent=function(name,props){{
    send('event',{{name:name,props:props||{{}}}});
  }};
}})();
</script>"#,
        endpoint = endpoint
    )
}

/// Generate a dashboard HTML page showing analytics
pub fn analytics_dashboard_html(api_endpoint: &str) -> String {
    format!(
        r#"<!DOCTYPE html>
<html><head><title>Analytics Dashboard</title>
<style>
body{{font-family:system-ui;max-width:960px;margin:0 auto;padding:2rem;background:#f8f9fa}}
h1{{color:#1a1a2e}}
.card{{background:#fff;border-radius:8px;padding:1.5rem;margin:1rem 0;box-shadow:0 2px 4px rgba(0,0,0,.1)}}
.metric{{font-size:2rem;font-weight:700;color:#0070f3}}
table{{width:100%;border-collapse:collapse}}
td,th{{padding:.5rem;text-align:left;border-bottom:1px solid #eee}}
</style></head>
<body>
<h1>Analytics Dashboard</h1>
<div id="dashboard"><p>Loading...</p></div>
<script>
fetch('{api}').then(r=>r.json()).then(d=>{{
  var h='<div class="card"><span class="metric">'+d.total_views+'</span> total views</div>';
  h+='<div class="card"><h3>Top Pages</h3><table><tr><th>Path</th><th>Views</th></tr>';
  (d.pages||[]).forEach(p=>{{h+='<tr><td>'+p.path+'</td><td>'+p.views+'</td></tr>'}});
  h+='</table></div>';
  h+='<div class="card"><h3>Referrers</h3><table><tr><th>Domain</th><th>Visits</th></tr>';
  (d.referrers||[]).forEach(r=>{{h+='<tr><td>'+r.domain+'</td><td>'+r.visits+'</td></tr>'}});
  h+='</table></div>';
  document.getElementById('dashboard').innerHTML=h;
}});
</script></body></html>"#,
        api = api_endpoint
    )
}

// ─── Helpers ───────────────────────────────────────────────

/// Extract domain from a URL
pub fn extract_domain(url: &str) -> String {
    let without_proto = url
        .strip_prefix("https://").or_else(|| url.strip_prefix("http://"))
        .unwrap_or(url);
    without_proto.split('/').next().unwrap_or(url).to_string()
}

/// Detect browser from User-Agent string
pub fn detect_browser(ua: &str) -> String {
    let ua_lower = ua.to_lowercase();
    if ua_lower.contains("firefox") {
        "Firefox".to_string()
    } else if ua_lower.contains("edg/") || ua_lower.contains("edge") {
        "Edge".to_string()
    } else if ua_lower.contains("chrome") && !ua_lower.contains("edg") {
        "Chrome".to_string()
    } else if ua_lower.contains("safari") && !ua_lower.contains("chrome") {
        "Safari".to_string()
    } else if ua_lower.contains("opera") || ua_lower.contains("opr/") {
        "Opera".to_string()
    } else if ua_lower.contains("bot") || ua_lower.contains("crawl") || ua_lower.contains("spider") {
        "Bot".to_string()
    } else {
        "Other".to_string()
    }
}

// ─── Tests ──────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_track_page_view() {
        let a = AnalyticsCollector::new();
        a.track_page_view("/", "Mozilla/5.0 Chrome", None);
        a.track_page_view("/", "Mozilla/5.0 Chrome", None);
        a.track_page_view("/about", "Mozilla/5.0 Firefox", None);
        assert_eq!(a.views_for("/"), 2);
        assert_eq!(a.views_for("/about"), 1);
        assert_eq!(a.total_views(), 3);
    }

    #[test]
    fn test_track_event() {
        let a = AnalyticsCollector::new();
        a.track_event("signup");
        a.track_event("signup");
        a.track_event("purchase");
        assert_eq!(a.event_count("signup"), 2);
        assert_eq!(a.event_count("purchase"), 1);
    }

    #[test]
    fn test_top_pages() {
        let a = AnalyticsCollector::new();
        for _ in 0..5 { a.track_page_view("/popular", "Chrome", None); }
        for _ in 0..2 { a.track_page_view("/less", "Chrome", None); }
        let top = a.top_pages(10);
        assert_eq!(top[0].0, "/popular");
        assert_eq!(top[0].1, 5);
    }

    #[test]
    fn test_referrers() {
        let a = AnalyticsCollector::new();
        a.track_page_view("/", "Chrome", Some("https://google.com/search?q=test"));
        a.track_page_view("/", "Chrome", Some("https://google.com/other"));
        a.track_page_view("/", "Chrome", Some("https://twitter.com"));
        let refs = a.top_referrers(10);
        assert!(refs.iter().any(|(d, c)| d == "google.com" && *c == 2));
    }

    #[test]
    fn test_browser_detection() {
        assert_eq!(detect_browser("Mozilla/5.0 Chrome/120"), "Chrome");
        assert_eq!(detect_browser("Mozilla/5.0 Firefox/120"), "Firefox");
        assert_eq!(detect_browser("Mozilla/5.0 Safari/605"), "Safari");
        assert_eq!(detect_browser("Mozilla/5.0 Edg/120"), "Edge");
        assert_eq!(detect_browser("Googlebot/2.1"), "Bot");
    }

    #[test]
    fn test_browser_stats() {
        let a = AnalyticsCollector::new();
        a.track_page_view("/", "Chrome/120", None);
        a.track_page_view("/", "Chrome/120", None);
        a.track_page_view("/", "Firefox/120", None);
        let stats = a.browser_stats();
        assert!(stats.iter().any(|(b, c)| b == "Chrome" && *c == 2));
    }

    #[test]
    fn test_extract_domain() {
        assert_eq!(extract_domain("https://google.com/search?q=test"), "google.com");
        assert_eq!(extract_domain("http://example.com/path"), "example.com");
        assert_eq!(extract_domain("example.com"), "example.com");
    }

    #[test]
    fn test_report_json() {
        let a = AnalyticsCollector::new();
        a.track_page_view("/", "Chrome", None);
        let json = a.report_json();
        assert!(json.contains("\"total_views\":1"));
        assert!(json.contains("\"pages\":["));
    }

    #[test]
    fn test_clear() {
        let a = AnalyticsCollector::new();
        a.track_page_view("/", "Chrome", None);
        a.track_event("test");
        a.clear();
        assert_eq!(a.total_views(), 0);
        assert_eq!(a.event_count("test"), 0);
    }

    #[test]
    fn test_analytics_script() {
        let script = analytics_script("/api/analytics");
        assert!(script.contains("/api/analytics"));
        assert!(script.contains("trackEvent"));
        assert!(script.contains("pageview"));
    }

    #[test]
    fn test_analytics_dashboard() {
        let html = analytics_dashboard_html("/api/analytics/report");
        assert!(html.contains("Analytics Dashboard"));
        assert!(html.contains("/api/analytics/report"));
    }

    #[test]
    fn test_track_country() {
        let a = AnalyticsCollector::new();
        a.track_country("jp");
        a.track_country("us");
        // Countries are stored uppercase
    }

    #[test]
    fn test_no_empty_referrer() {
        let a = AnalyticsCollector::new();
        a.track_page_view("/", "Chrome", Some(""));
        let refs = a.top_referrers(10);
        assert!(refs.is_empty());
    }

    #[test]
    fn test_views_nonexistent() {
        let a = AnalyticsCollector::new();
        assert_eq!(a.views_for("/nonexistent"), 0);
        assert_eq!(a.event_count("nonexistent"), 0);
    }
}
