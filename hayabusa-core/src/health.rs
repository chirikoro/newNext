//! Health Checks & Metrics for Hayabusa.
//!
//! Readiness/liveness probes and Prometheus-compatible metrics.
//!
//! ```ignore
//! use hayabusa_core::prelude::*;
//!
//! let health = HealthChecker::new()
//!     .check("database", || true)
//!     .check("redis", || true);
//! let status = health.run_checks();
//! ```

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use dashmap::DashMap;

// ─── Health Check ──────────────────────────────────────────

/// Health status
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum HealthStatus {
    Healthy,
    Degraded,
    Unhealthy,
}

impl HealthStatus {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Healthy => "healthy",
            Self::Degraded => "degraded",
            Self::Unhealthy => "unhealthy",
        }
    }

    pub fn http_status(&self) -> u16 {
        match self {
            Self::Healthy => 200,
            Self::Degraded => 200,
            Self::Unhealthy => 503,
        }
    }
}

/// Result of a single health check
#[derive(Debug, Clone)]
pub struct CheckResult {
    pub name: String,
    pub status: HealthStatus,
    pub message: Option<String>,
    pub duration_ms: u64,
}

/// Overall health report
#[derive(Debug, Clone)]
pub struct HealthReport {
    pub status: HealthStatus,
    pub checks: Vec<CheckResult>,
    pub version: Option<String>,
    pub uptime_secs: u64,
}

impl HealthReport {
    pub fn to_json(&self) -> String {
        let checks_json: Vec<String> = self.checks.iter().map(|c| {
            let msg = match &c.message {
                Some(m) => format!(",\"message\":\"{}\"", m.replace('"', "\\\"")),
                None => String::new(),
            };
            format!(
                "{{\"name\":\"{}\",\"status\":\"{}\",\"duration_ms\":{}{}}}",
                c.name, c.status.as_str(), c.duration_ms, msg
            )
        }).collect();

        let version = match &self.version {
            Some(v) => format!(",\"version\":\"{}\"", v),
            None => String::new(),
        };

        format!(
            "{{\"status\":\"{}\",\"uptime_secs\":{}{},\"checks\":[{}]}}",
            self.status.as_str(), self.uptime_secs, version, checks_json.join(",")
        )
    }

    pub fn http_status(&self) -> u16 {
        self.status.http_status()
    }
}

/// Health check definition
#[derive(Clone)]
pub struct HealthCheck {
    pub name: String,
    pub check_fn: Arc<dyn Fn() -> (HealthStatus, Option<String>) + Send + Sync>,
    pub critical: bool,
}

impl std::fmt::Debug for HealthCheck {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HealthCheck")
            .field("name", &self.name)
            .field("critical", &self.critical)
            .finish()
    }
}

/// Health checker with registered checks
pub struct HealthChecker {
    checks: Vec<HealthCheck>,
    version: Option<String>,
    start_time: std::time::Instant,
}

impl HealthChecker {
    pub fn new() -> Self {
        Self {
            checks: Vec::new(),
            version: None,
            start_time: std::time::Instant::now(),
        }
    }

    pub fn version(mut self, v: &str) -> Self {
        self.version = Some(v.to_string());
        self
    }

    /// Add a simple boolean health check
    pub fn check(mut self, name: &str, f: impl Fn() -> bool + Send + Sync + 'static) -> Self {
        let name_owned = name.to_string();
        self.checks.push(HealthCheck {
            name: name_owned,
            check_fn: Arc::new(move || {
                if f() {
                    (HealthStatus::Healthy, None)
                } else {
                    (HealthStatus::Unhealthy, Some("Check failed".to_string()))
                }
            }),
            critical: true,
        });
        self
    }

    /// Add a detailed health check
    pub fn check_detailed(
        mut self,
        name: &str,
        critical: bool,
        f: impl Fn() -> (HealthStatus, Option<String>) + Send + Sync + 'static,
    ) -> Self {
        self.checks.push(HealthCheck {
            name: name.to_string(),
            check_fn: Arc::new(f),
            critical,
        });
        self
    }

    /// Run all health checks
    pub fn run_checks(&self) -> HealthReport {
        let mut results = Vec::new();
        let mut overall = HealthStatus::Healthy;

        for check in &self.checks {
            let start = std::time::Instant::now();
            let (status, message) = (check.check_fn)();
            let duration_ms = start.elapsed().as_millis() as u64;

            if status != HealthStatus::Healthy {
                if check.critical {
                    overall = HealthStatus::Unhealthy;
                } else if overall == HealthStatus::Healthy {
                    overall = HealthStatus::Degraded;
                }
            }

            results.push(CheckResult {
                name: check.name.clone(),
                status,
                message,
                duration_ms,
            });
        }

        HealthReport {
            status: overall,
            checks: results,
            version: self.version.clone(),
            uptime_secs: self.start_time.elapsed().as_secs(),
        }
    }

    /// Simple liveness check (always returns 200 if the process is running)
    pub fn liveness_json() -> String {
        "{\"status\":\"alive\"}".to_string()
    }

    /// Readiness check (runs all checks)
    pub fn readiness_json(&self) -> String {
        self.run_checks().to_json()
    }
}

impl Default for HealthChecker {
    fn default() -> Self {
        Self::new()
    }
}

// ─── Prometheus Metrics ────────────────────────────────────

/// Simple Prometheus-compatible metrics collector
#[derive(Debug, Clone)]
pub struct MetricsCollector {
    counters: Arc<DashMap<String, AtomicU64>>,
    gauges: Arc<DashMap<String, AtomicU64>>,
    histograms: Arc<DashMap<String, Vec<f64>>>,
    labels: Arc<DashMap<String, String>>,
}

impl MetricsCollector {
    pub fn new() -> Self {
        Self {
            counters: Arc::new(DashMap::new()),
            gauges: Arc::new(DashMap::new()),
            histograms: Arc::new(DashMap::new()),
            labels: Arc::new(DashMap::new()),
        }
    }

    /// Add a global label
    pub fn label(self, name: &str, value: &str) -> Self {
        self.labels.insert(name.to_string(), value.to_string());
        self
    }

    /// Increment a counter
    pub fn inc(&self, name: &str) {
        self.counters.entry(name.to_string())
            .or_insert_with(|| AtomicU64::new(0))
            .fetch_add(1, Ordering::Relaxed);
    }

    /// Increment a counter by a value
    pub fn inc_by(&self, name: &str, n: u64) {
        self.counters.entry(name.to_string())
            .or_insert_with(|| AtomicU64::new(0))
            .fetch_add(n, Ordering::Relaxed);
    }

    /// Set a gauge value
    pub fn gauge_set(&self, name: &str, value: u64) {
        self.gauges.entry(name.to_string())
            .or_insert_with(|| AtomicU64::new(0))
            .store(value, Ordering::Relaxed);
    }

    /// Get a counter value
    pub fn counter_value(&self, name: &str) -> u64 {
        self.counters.get(name)
            .map(|c| c.load(Ordering::Relaxed))
            .unwrap_or(0)
    }

    /// Get a gauge value
    pub fn gauge_value(&self, name: &str) -> u64 {
        self.gauges.get(name)
            .map(|g| g.load(Ordering::Relaxed))
            .unwrap_or(0)
    }

    /// Export as Prometheus text format
    pub fn to_prometheus(&self) -> String {
        let mut out = String::new();

        for entry in self.counters.iter() {
            let labels = self.format_labels();
            out.push_str(&format!(
                "# TYPE {} counter\n{}{} {}\n",
                entry.key(), entry.key(), labels, entry.value().load(Ordering::Relaxed)
            ));
        }

        for entry in self.gauges.iter() {
            let labels = self.format_labels();
            out.push_str(&format!(
                "# TYPE {} gauge\n{}{} {}\n",
                entry.key(), entry.key(), labels, entry.value().load(Ordering::Relaxed)
            ));
        }

        out
    }

    fn format_labels(&self) -> String {
        if self.labels.is_empty() {
            return String::new();
        }
        let pairs: Vec<String> = self.labels.iter()
            .map(|e| format!("{}=\"{}\"", e.key(), e.value()))
            .collect();
        format!("{{{}}}", pairs.join(","))
    }

    /// Export as JSON
    pub fn to_json(&self) -> String {
        let mut entries = Vec::new();
        for entry in self.counters.iter() {
            entries.push(format!("\"{}\":{}", entry.key(), entry.value().load(Ordering::Relaxed)));
        }
        for entry in self.gauges.iter() {
            entries.push(format!("\"{}\":{}", entry.key(), entry.value().load(Ordering::Relaxed)));
        }
        format!("{{{}}}", entries.join(","))
    }
}

impl Default for MetricsCollector {
    fn default() -> Self {
        Self::new()
    }
}

// ─── Request Metrics Middleware Helper ─────────────────────

/// Track HTTP request metrics
pub fn track_request(metrics: &MetricsCollector, method: &str, path: &str, status: u16, duration_ms: u64) {
    metrics.inc("http_requests_total");
    metrics.inc(&format!("http_requests_{}", method.to_lowercase()));
    if status >= 400 && status < 500 {
        metrics.inc("http_errors_4xx");
    } else if status >= 500 {
        metrics.inc("http_errors_5xx");
    }
    // Update active connections gauge (simplified)
    let _ = (path, duration_ms); // used for histogram in full impl
}

// ─── Tests ──────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_health_status() {
        assert_eq!(HealthStatus::Healthy.http_status(), 200);
        assert_eq!(HealthStatus::Unhealthy.http_status(), 503);
        assert_eq!(HealthStatus::Degraded.as_str(), "degraded");
    }

    #[test]
    fn test_health_checker_all_healthy() {
        let checker = HealthChecker::new()
            .version("1.0.0")
            .check("db", || true)
            .check("cache", || true);
        let report = checker.run_checks();
        assert_eq!(report.status, HealthStatus::Healthy);
        assert_eq!(report.checks.len(), 2);
        assert_eq!(report.http_status(), 200);
    }

    #[test]
    fn test_health_checker_unhealthy() {
        let checker = HealthChecker::new()
            .check("db", || false)
            .check("cache", || true);
        let report = checker.run_checks();
        assert_eq!(report.status, HealthStatus::Unhealthy);
        assert_eq!(report.http_status(), 503);
    }

    #[test]
    fn test_health_checker_degraded() {
        let checker = HealthChecker::new()
            .check("db", || true)
            .check_detailed("cache", false, || (HealthStatus::Unhealthy, Some("Down".into())));
        let report = checker.run_checks();
        assert_eq!(report.status, HealthStatus::Degraded);
    }

    #[test]
    fn test_health_report_json() {
        let report = HealthReport {
            status: HealthStatus::Healthy,
            checks: vec![CheckResult {
                name: "db".into(),
                status: HealthStatus::Healthy,
                message: None,
                duration_ms: 5,
            }],
            version: Some("1.0.0".into()),
            uptime_secs: 3600,
        };
        let json = report.to_json();
        assert!(json.contains("\"healthy\""));
        assert!(json.contains("\"version\":\"1.0.0\""));
        assert!(json.contains("\"uptime_secs\":3600"));
    }

    #[test]
    fn test_liveness() {
        let json = HealthChecker::liveness_json();
        assert!(json.contains("alive"));
    }

    #[test]
    fn test_metrics_counter() {
        let m = MetricsCollector::new();
        m.inc("requests");
        m.inc("requests");
        m.inc_by("requests", 3);
        assert_eq!(m.counter_value("requests"), 5);
    }

    #[test]
    fn test_metrics_gauge() {
        let m = MetricsCollector::new();
        m.gauge_set("connections", 42);
        assert_eq!(m.gauge_value("connections"), 42);
        m.gauge_set("connections", 10);
        assert_eq!(m.gauge_value("connections"), 10);
    }

    #[test]
    fn test_metrics_prometheus() {
        let m = MetricsCollector::new().label("app", "hayabusa");
        m.inc("http_requests_total");
        m.gauge_set("active_connections", 5);
        let prom = m.to_prometheus();
        assert!(prom.contains("# TYPE http_requests_total counter"));
        assert!(prom.contains("http_requests_total{app=\"hayabusa\"} 1"));
        assert!(prom.contains("# TYPE active_connections gauge"));
    }

    #[test]
    fn test_metrics_json() {
        let m = MetricsCollector::new();
        m.inc("req");
        let json = m.to_json();
        assert!(json.contains("\"req\":1"));
    }

    #[test]
    fn test_track_request() {
        let m = MetricsCollector::new();
        track_request(&m, "GET", "/api/users", 200, 15);
        track_request(&m, "POST", "/api/users", 500, 100);
        assert_eq!(m.counter_value("http_requests_total"), 2);
        assert_eq!(m.counter_value("http_errors_5xx"), 1);
    }

    #[test]
    fn test_metrics_default_zero() {
        let m = MetricsCollector::new();
        assert_eq!(m.counter_value("nonexistent"), 0);
        assert_eq!(m.gauge_value("nonexistent"), 0);
    }

    #[test]
    fn test_metrics_labels() {
        let m = MetricsCollector::new()
            .label("env", "prod")
            .label("service", "api");
        m.inc("test");
        let prom = m.to_prometheus();
        assert!(prom.contains("env=\"prod\""));
        assert!(prom.contains("service=\"api\""));
    }
}
