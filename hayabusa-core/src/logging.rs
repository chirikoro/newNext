//! Structured Logging and Request Tracing for Hayabusa.
//!
//! JSON-based structured logging with request ID tracing,
//! log levels, and performance measurement.
//!
//! ```ignore
//! use hayabusa_core::prelude::*;
//!
//! let logger = Logger::new(LogLevel::Info).json_format(true);
//! logger.info("Server started", &[("port", "3000")]);
//! ```

use std::time::{Instant, SystemTime, UNIX_EPOCH};

// ─── Log Levels ─────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LogLevel {
    Trace = 0,
    Debug = 1,
    Info = 2,
    Warn = 3,
    Error = 4,
    Fatal = 5,
}

impl LogLevel {
    pub fn as_str(&self) -> &str {
        match self {
            LogLevel::Trace => "TRACE",
            LogLevel::Debug => "DEBUG",
            LogLevel::Info => "INFO",
            LogLevel::Warn => "WARN",
            LogLevel::Error => "ERROR",
            LogLevel::Fatal => "FATAL",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s.to_uppercase().as_str() {
            "TRACE" => LogLevel::Trace,
            "DEBUG" => LogLevel::Debug,
            "WARN" | "WARNING" => LogLevel::Warn,
            "ERROR" | "ERR" => LogLevel::Error,
            "FATAL" => LogLevel::Fatal,
            _ => LogLevel::Info,
        }
    }

    /// Get from RUST_LOG or HAYABUSA_LOG env var
    pub fn from_env() -> Self {
        std::env::var("HAYABUSA_LOG")
            .or_else(|_| std::env::var("RUST_LOG"))
            .map(|s| Self::from_str(&s))
            .unwrap_or(LogLevel::Info)
    }
}

// ─── Logger ─────────────────────────────────────────────────

/// Structured logger
#[derive(Debug, Clone)]
pub struct Logger {
    pub level: LogLevel,
    pub json_format: bool,
    pub include_timestamp: bool,
    pub include_location: bool,
    pub prefix: Option<String>,
}

impl Logger {
    pub fn new(level: LogLevel) -> Self {
        Self {
            level,
            json_format: false,
            include_timestamp: true,
            include_location: false,
            prefix: None,
        }
    }

    /// Create from environment variable
    pub fn from_env() -> Self {
        Self::new(LogLevel::from_env())
    }

    pub fn json_format(mut self, enabled: bool) -> Self {
        self.json_format = enabled;
        self
    }

    pub fn with_prefix(mut self, prefix: impl Into<String>) -> Self {
        self.prefix = Some(prefix.into());
        self
    }

    pub fn include_location(mut self, enabled: bool) -> Self {
        self.include_location = enabled;
        self
    }

    fn should_log(&self, level: LogLevel) -> bool {
        level >= self.level
    }

    fn format_log(&self, level: LogLevel, message: &str, fields: &[(&str, &str)]) -> String {
        if self.json_format {
            return self.format_json(level, message, fields);
        }
        self.format_text(level, message, fields)
    }

    fn format_json(&self, level: LogLevel, message: &str, fields: &[(&str, &str)]) -> String {
        let mut json = format!(
            "{{\"level\":\"{}\",\"msg\":\"{}\"",
            level.as_str(),
            json_escape(message)
        );
        if self.include_timestamp {
            json.push_str(&format!(",\"ts\":{}", timestamp_ms()));
        }
        if let Some(ref prefix) = self.prefix {
            json.push_str(&format!(",\"component\":\"{}\"", json_escape(prefix)));
        }
        for (k, v) in fields {
            json.push_str(&format!(",\"{}\":\"{}\"", k, json_escape(v)));
        }
        json.push('}');
        json
    }

    fn format_text(&self, level: LogLevel, message: &str, fields: &[(&str, &str)]) -> String {
        let mut line = String::new();
        if self.include_timestamp {
            line.push_str(&format!("[{}] ", timestamp_iso()));
        }
        line.push_str(&format!("{:<5} ", level.as_str()));
        if let Some(ref prefix) = self.prefix {
            line.push_str(&format!("[{}] ", prefix));
        }
        line.push_str(message);
        if !fields.is_empty() {
            let pairs: Vec<String> = fields.iter().map(|(k, v)| format!("{}={}", k, v)).collect();
            line.push_str(&format!(" {}", pairs.join(" ")));
        }
        line
    }

    pub fn log(&self, level: LogLevel, message: &str, fields: &[(&str, &str)]) {
        if self.should_log(level) {
            let formatted = self.format_log(level, message, fields);
            if level >= LogLevel::Error {
                eprintln!("{}", formatted);
            } else {
                println!("{}", formatted);
            }
        }
    }

    pub fn trace(&self, msg: &str, fields: &[(&str, &str)]) { self.log(LogLevel::Trace, msg, fields); }
    pub fn debug(&self, msg: &str, fields: &[(&str, &str)]) { self.log(LogLevel::Debug, msg, fields); }
    pub fn info(&self, msg: &str, fields: &[(&str, &str)]) { self.log(LogLevel::Info, msg, fields); }
    pub fn warn(&self, msg: &str, fields: &[(&str, &str)]) { self.log(LogLevel::Warn, msg, fields); }
    pub fn error(&self, msg: &str, fields: &[(&str, &str)]) { self.log(LogLevel::Error, msg, fields); }
}

impl Default for Logger {
    fn default() -> Self {
        Self::new(LogLevel::Info)
    }
}

// ─── Request Logger ─────────────────────────────────────────

/// HTTP request logging middleware data
#[derive(Debug, Clone)]
pub struct RequestLog {
    pub request_id: String,
    pub method: String,
    pub path: String,
    pub status: u16,
    pub duration_ms: f64,
    pub user_agent: Option<String>,
    pub ip: Option<String>,
    pub bytes_sent: Option<u64>,
}

impl RequestLog {
    pub fn start(method: &str, path: &str) -> RequestLogTimer {
        RequestLogTimer {
            method: method.to_string(),
            path: path.to_string(),
            start: Instant::now(),
            request_id: generate_request_id(),
        }
    }

    /// Format as JSON
    pub fn to_json(&self) -> String {
        let mut json = format!(
            "{{\"request_id\":\"{}\",\"method\":\"{}\",\"path\":\"{}\",\"status\":{},\"duration_ms\":{:.2}",
            self.request_id, self.method, json_escape(&self.path), self.status, self.duration_ms
        );
        if let Some(ref ua) = self.user_agent {
            json.push_str(&format!(",\"user_agent\":\"{}\"", json_escape(ua)));
        }
        if let Some(ref ip) = self.ip {
            json.push_str(&format!(",\"ip\":\"{}\"", ip));
        }
        if let Some(bytes) = self.bytes_sent {
            json.push_str(&format!(",\"bytes\":{}", bytes));
        }
        json.push('}');
        json
    }

    /// Format as combined log format (Apache/nginx style)
    pub fn to_combined(&self) -> String {
        format!(
            "{} - - \"{}\" {} {} - {:.0}ms \"{}\"",
            self.ip.as_deref().unwrap_or("-"),
            format!("{} {}", self.method, self.path),
            self.status,
            self.bytes_sent.unwrap_or(0),
            self.duration_ms,
            self.user_agent.as_deref().unwrap_or("-"),
        )
    }
}

/// Timer for measuring request duration
pub struct RequestLogTimer {
    pub method: String,
    pub path: String,
    pub start: Instant,
    pub request_id: String,
}

impl RequestLogTimer {
    /// Complete the request log
    pub fn finish(self, status: u16) -> RequestLog {
        RequestLog {
            request_id: self.request_id,
            method: self.method,
            path: self.path,
            status,
            duration_ms: self.start.elapsed().as_secs_f64() * 1000.0,
            user_agent: None,
            ip: None,
            bytes_sent: None,
        }
    }
}

// ─── Performance Span ───────────────────────────────────────

/// Named performance measurement span
pub struct Span {
    pub name: String,
    start: Instant,
}

impl Span {
    pub fn start(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            start: Instant::now(),
        }
    }

    pub fn elapsed_ms(&self) -> f64 {
        self.start.elapsed().as_secs_f64() * 1000.0
    }

    pub fn finish(self) -> SpanResult {
        SpanResult {
            name: self.name,
            duration_ms: self.start.elapsed().as_secs_f64() * 1000.0,
        }
    }
}

/// Completed span result
#[derive(Debug, Clone)]
pub struct SpanResult {
    pub name: String,
    pub duration_ms: f64,
}

impl std::fmt::Display for SpanResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {:.2}ms", self.name, self.duration_ms)
    }
}

// ─── Helpers ────────────────────────────────────────────────

fn timestamp_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

fn timestamp_iso() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    // Simple ISO-ish format: epoch seconds (proper ISO requires chrono crate)
    format!("{}", secs)
}

fn json_escape(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"").replace('\n', "\\n")
}

fn generate_request_id() -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    format!("req_{:x}_{:x}", now.as_secs(), now.subsec_nanos())
}

// ─── Tests ──────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_log_level_ordering() {
        assert!(LogLevel::Error > LogLevel::Info);
        assert!(LogLevel::Debug < LogLevel::Warn);
        assert!(LogLevel::Trace < LogLevel::Fatal);
    }

    #[test]
    fn test_log_level_from_str() {
        assert_eq!(LogLevel::from_str("debug"), LogLevel::Debug);
        assert_eq!(LogLevel::from_str("ERROR"), LogLevel::Error);
        assert_eq!(LogLevel::from_str("warn"), LogLevel::Warn);
        assert_eq!(LogLevel::from_str("unknown"), LogLevel::Info);
    }

    #[test]
    fn test_logger_json_format() {
        let logger = Logger::new(LogLevel::Debug).json_format(true);
        let output = logger.format_log(LogLevel::Info, "test message", &[("key", "value")]);
        assert!(output.contains("\"level\":\"INFO\""));
        assert!(output.contains("\"msg\":\"test message\""));
        assert!(output.contains("\"key\":\"value\""));
    }

    #[test]
    fn test_logger_text_format() {
        let logger = Logger::new(LogLevel::Debug)
            .json_format(false)
            .with_prefix("app");
        let output = logger.format_log(LogLevel::Warn, "something wrong", &[("code", "500")]);
        assert!(output.contains("WARN"));
        assert!(output.contains("[app]"));
        assert!(output.contains("something wrong"));
        assert!(output.contains("code=500"));
    }

    #[test]
    fn test_logger_level_filter() {
        let logger = Logger::new(LogLevel::Warn);
        assert!(!logger.should_log(LogLevel::Debug));
        assert!(!logger.should_log(LogLevel::Info));
        assert!(logger.should_log(LogLevel::Warn));
        assert!(logger.should_log(LogLevel::Error));
    }

    #[test]
    fn test_request_log_json() {
        let log = RequestLog {
            request_id: "req_1".to_string(),
            method: "GET".to_string(),
            path: "/api/users".to_string(),
            status: 200,
            duration_ms: 12.5,
            user_agent: Some("Mozilla/5.0".to_string()),
            ip: Some("127.0.0.1".to_string()),
            bytes_sent: Some(1024),
        };
        let json = log.to_json();
        assert!(json.contains("\"status\":200"));
        assert!(json.contains("\"duration_ms\":12.50"));
        assert!(json.contains("\"ip\":\"127.0.0.1\""));
    }

    #[test]
    fn test_request_log_combined() {
        let log = RequestLog {
            request_id: "req_1".to_string(),
            method: "GET".to_string(),
            path: "/".to_string(),
            status: 200,
            duration_ms: 5.0,
            user_agent: Some("curl".to_string()),
            ip: Some("10.0.0.1".to_string()),
            bytes_sent: Some(512),
        };
        let line = log.to_combined();
        assert!(line.contains("10.0.0.1"));
        assert!(line.contains("GET /"));
        assert!(line.contains("200"));
    }

    #[test]
    fn test_request_log_timer() {
        let timer = RequestLog::start("POST", "/api/submit");
        assert_eq!(timer.method, "POST");
        let log = timer.finish(201);
        assert_eq!(log.status, 201);
        assert!(log.duration_ms >= 0.0);
        assert!(log.request_id.starts_with("req_"));
    }

    #[test]
    fn test_span() {
        let span = Span::start("render");
        assert!(span.elapsed_ms() >= 0.0);
        let result = span.finish();
        assert_eq!(result.name, "render");
        assert!(result.duration_ms >= 0.0);
    }

    #[test]
    fn test_span_display() {
        let result = SpanResult { name: "test".into(), duration_ms: 1.23 };
        let s = format!("{}", result);
        assert!(s.contains("test"));
        assert!(s.contains("1.23"));
    }

    #[test]
    fn test_generate_request_id() {
        let id = generate_request_id();
        assert!(id.starts_with("req_"));
        assert!(id.len() > 5);
    }
}
