//! Core Web Vitals Measurement for Hayabusa.
//!
//! Automatically measures and reports Google's Core Web Vitals:
//! - **LCP** (Largest Contentful Paint) — loading performance
//! - **FID** (First Input Delay) / **INP** (Interaction to Next Paint) — interactivity
//! - **CLS** (Cumulative Layout Shift) — visual stability
//! - **TTFB** (Time to First Byte) — server response time
//! - **FCP** (First Contentful Paint) — initial render
//!
//! ## Why This Matters
//! You can't improve what you don't measure. This module lets you
//! track real-user performance and verify that Hayabusa is actually
//! delivering the speed improvements.
//!
//! ## Usage
//! ```ignore
//! // Include in your layout
//! let vitals_script = web_vitals_script("/api/vitals");
//! ```

/// Generate a client-side script that measures Core Web Vitals
/// and reports them to the specified endpoint.
///
/// The script uses the PerformanceObserver API (no dependencies).
pub fn web_vitals_script(report_endpoint: &str) -> String {
    format!(
        r#"<script>
(function(){{
var q=[];
function send(m){{q.push(m);if(q.length>=5||m.name==='LCP')flush()}}
function flush(){{
if(!q.length)return;
var d=JSON.stringify(q);q=[];
if(navigator.sendBeacon)navigator.sendBeacon('{endpoint}',d);
else fetch('{endpoint}',{{method:'POST',body:d,keepalive:true}});
}}

// TTFB
try{{
var nav=performance.getEntriesByType('navigation')[0];
if(nav)send({{name:'TTFB',value:nav.responseStart,path:location.pathname}});
}}catch(e){{}}

// FCP
try{{
new PerformanceObserver(function(l){{
l.getEntries().forEach(function(e){{
if(e.name==='first-contentful-paint')send({{name:'FCP',value:e.startTime,path:location.pathname}})
}});
}}).observe({{type:'paint',buffered:true}});
}}catch(e){{}}

// LCP
try{{
var lcpValue=0;
new PerformanceObserver(function(l){{
l.getEntries().forEach(function(e){{lcpValue=e.startTime}});
}}).observe({{type:'largest-contentful-paint',buffered:true}});
addEventListener('visibilitychange',function(){{
if(document.visibilityState==='hidden'&&lcpValue)send({{name:'LCP',value:lcpValue,path:location.pathname}})
}},{{once:true}});
}}catch(e){{}}

// CLS
try{{
var clsValue=0;var clsEntries=[];
new PerformanceObserver(function(l){{
l.getEntries().forEach(function(e){{
if(!e.hadRecentInput){{clsEntries.push(e);clsValue+=e.value}}
}});
}}).observe({{type:'layout-shift',buffered:true}});
addEventListener('visibilitychange',function(){{
if(document.visibilityState==='hidden')send({{name:'CLS',value:clsValue,path:location.pathname}})
}},{{once:true}});
}}catch(e){{}}

// INP (Interaction to Next Paint)
try{{
var inpValue=0;
new PerformanceObserver(function(l){{
l.getEntries().forEach(function(e){{
var d=e.processingStart?e.duration:0;
if(d>inpValue)inpValue=d;
}});
}}).observe({{type:'event',buffered:true,durationThreshold:16}});
addEventListener('visibilitychange',function(){{
if(document.visibilityState==='hidden'&&inpValue)send({{name:'INP',value:inpValue,path:location.pathname}})
}},{{once:true}});
}}catch(e){{}}

// Flush remaining on page hide
addEventListener('visibilitychange',function(){{if(document.visibilityState==='hidden')flush()}});
}})();
</script>"#,
        endpoint = report_endpoint
    )
}

/// Generate a minimal analytics endpoint handler that logs vitals.
///
/// Returns the response HTML/JSON for acknowledging the report.
pub fn vitals_response() -> String {
    r#"{"ok":true}"#.to_string()
}

/// Performance budget configuration.
///
/// Define thresholds for each metric. If exceeded, a warning is logged.
#[derive(Debug, Clone)]
pub struct PerformanceBudget {
    /// LCP threshold in milliseconds (good: <2500ms)
    pub lcp_ms: u64,
    /// FCP threshold in milliseconds (good: <1800ms)
    pub fcp_ms: u64,
    /// CLS threshold (good: <0.1)
    pub cls: f64,
    /// INP threshold in milliseconds (good: <200ms)
    pub inp_ms: u64,
    /// TTFB threshold in milliseconds (good: <800ms)
    pub ttfb_ms: u64,
}

impl Default for PerformanceBudget {
    fn default() -> Self {
        Self {
            lcp_ms: 2500,
            fcp_ms: 1800,
            cls: 0.1,
            inp_ms: 200,
            ttfb_ms: 800,
        }
    }
}

impl PerformanceBudget {
    pub fn new() -> Self {
        Self::default()
    }

    /// Strict budget for high-performance sites
    pub fn strict() -> Self {
        Self {
            lcp_ms: 1200,
            fcp_ms: 1000,
            cls: 0.05,
            inp_ms: 100,
            ttfb_ms: 200,
        }
    }

    /// Check if a metric value is within budget
    pub fn check(&self, metric: &str, value: f64) -> BudgetResult {
        let (threshold, unit) = match metric {
            "LCP" => (self.lcp_ms as f64, "ms"),
            "FCP" => (self.fcp_ms as f64, "ms"),
            "CLS" => (self.cls, ""),
            "INP" => (self.inp_ms as f64, "ms"),
            "TTFB" => (self.ttfb_ms as f64, "ms"),
            _ => return BudgetResult::Unknown,
        };

        if value <= threshold {
            BudgetResult::Good {
                metric: metric.to_string(),
                value,
                threshold,
                unit: unit.to_string(),
            }
        } else if value <= threshold * 1.5 {
            BudgetResult::NeedsImprovement {
                metric: metric.to_string(),
                value,
                threshold,
                unit: unit.to_string(),
            }
        } else {
            BudgetResult::Poor {
                metric: metric.to_string(),
                value,
                threshold,
                unit: unit.to_string(),
            }
        }
    }
}

/// Result of a performance budget check
#[derive(Debug, Clone)]
pub enum BudgetResult {
    Good {
        metric: String,
        value: f64,
        threshold: f64,
        unit: String,
    },
    NeedsImprovement {
        metric: String,
        value: f64,
        threshold: f64,
        unit: String,
    },
    Poor {
        metric: String,
        value: f64,
        threshold: f64,
        unit: String,
    },
    Unknown,
}

impl BudgetResult {
    pub fn is_good(&self) -> bool {
        matches!(self, BudgetResult::Good { .. })
    }

    pub fn is_poor(&self) -> bool {
        matches!(self, BudgetResult::Poor { .. })
    }
}

/// Generate a <meta> tag for server timing (shows in DevTools)
pub fn server_timing_header(timings: &[(&str, f64)]) -> String {
    timings
        .iter()
        .map(|(name, dur)| format!("{};dur={:.1}", name, dur))
        .collect::<Vec<_>>()
        .join(", ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_web_vitals_script() {
        let script = web_vitals_script("/api/vitals");
        assert!(script.contains("/api/vitals"));
        assert!(script.contains("PerformanceObserver"));
        assert!(script.contains("largest-contentful-paint"));
        assert!(script.contains("layout-shift"));
        assert!(script.contains("TTFB"));
        assert!(script.contains("sendBeacon"));
    }

    #[test]
    fn test_performance_budget_good() {
        let budget = PerformanceBudget::new();
        assert!(budget.check("LCP", 2000.0).is_good());
        assert!(budget.check("CLS", 0.05).is_good());
        assert!(budget.check("TTFB", 500.0).is_good());
    }

    #[test]
    fn test_performance_budget_poor() {
        let budget = PerformanceBudget::new();
        assert!(budget.check("LCP", 5000.0).is_poor());
        assert!(budget.check("CLS", 0.5).is_poor());
    }

    #[test]
    fn test_strict_budget() {
        let budget = PerformanceBudget::strict();
        assert_eq!(budget.lcp_ms, 1200);
        assert_eq!(budget.ttfb_ms, 200);
        // 2000ms LCP would be poor under strict budget
        assert!(budget.check("LCP", 2000.0).is_poor());
    }

    #[test]
    fn test_server_timing_header() {
        let header = server_timing_header(&[
            ("render", 12.5),
            ("db", 3.2),
            ("cache", 0.1),
        ]);
        assert_eq!(header, "render;dur=12.5, db;dur=3.2, cache;dur=0.1");
    }

    #[test]
    fn test_needs_improvement() {
        let budget = PerformanceBudget::new();
        let result = budget.check("LCP", 3000.0); // Between 2500 and 3750
        assert!(matches!(result, BudgetResult::NeedsImprovement { .. }));
    }
}
