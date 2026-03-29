//! Server-Sent Events (SSE) for Hayabusa.
//!
//! Provides real-time streaming from server to client without WebSocket complexity.
//! Ideal for: live updates, notifications, progress bars, chat, stock tickers.
//!
//! ## Usage
//! ```ignore
//! use hayabusa_core::sse::*;
//!
//! // Create an SSE stream
//! let stream = SseStream::new()
//!     .event("message", "Hello, world!")
//!     .event("update", r#"{"count": 42}"#)
//!     .keep_alive(Duration::from_secs(15));
//! ```

use std::time::Duration;
use bytes::Bytes;
use futures::stream::{self, Stream, StreamExt};

/// A single SSE event
#[derive(Debug, Clone)]
pub struct SseEvent {
    /// Event type (e.g., "message", "update", "error")
    pub event: Option<String>,
    /// Event data (can be multi-line, each line prefixed with "data: ")
    pub data: String,
    /// Optional event ID for reconnection
    pub id: Option<String>,
    /// Optional retry interval in milliseconds
    pub retry: Option<u64>,
}

impl SseEvent {
    /// Create a new SSE event with just data
    pub fn data(data: impl Into<String>) -> Self {
        Self {
            event: None,
            data: data.into(),
            id: None,
            retry: None,
        }
    }

    /// Create a named event
    pub fn named(event: impl Into<String>, data: impl Into<String>) -> Self {
        Self {
            event: Some(event.into()),
            data: data.into(),
            id: None,
            retry: None,
        }
    }

    /// Set the event ID
    pub fn with_id(mut self, id: impl Into<String>) -> Self {
        self.id = Some(id.into());
        self
    }

    /// Set retry interval
    pub fn with_retry(mut self, ms: u64) -> Self {
        self.retry = Some(ms);
        self
    }

    /// Serialize to SSE wire format
    pub fn to_string(&self) -> String {
        let mut output = String::new();

        if let Some(ref event) = self.event {
            output.push_str(&format!("event: {}\n", event));
        }

        if let Some(ref id) = self.id {
            output.push_str(&format!("id: {}\n", id));
        }

        if let Some(retry) = self.retry {
            output.push_str(&format!("retry: {}\n", retry));
        }

        // Data can be multi-line: each line gets "data: " prefix
        for line in self.data.lines() {
            output.push_str(&format!("data: {}\n", line));
        }

        output.push('\n'); // Empty line terminates the event
        output
    }
}

/// Builder for SSE responses
pub struct SseStream {
    events: Vec<SseEvent>,
    keep_alive_interval: Option<Duration>,
}

impl SseStream {
    pub fn new() -> Self {
        Self {
            events: Vec::new(),
            keep_alive_interval: None,
        }
    }

    /// Add a data-only event
    pub fn data(mut self, data: impl Into<String>) -> Self {
        self.events.push(SseEvent::data(data));
        self
    }

    /// Add a named event
    pub fn event(mut self, event_type: impl Into<String>, data: impl Into<String>) -> Self {
        self.events.push(SseEvent::named(event_type, data));
        self
    }

    /// Set keep-alive interval (sends comment lines to keep connection open)
    pub fn keep_alive(mut self, interval: Duration) -> Self {
        self.keep_alive_interval = Some(interval);
        self
    }

    /// Convert to an axum Response with proper SSE headers
    pub fn into_response(self) -> axum::response::Response {
        let event_stream = stream::iter(self.events.into_iter().map(|e| {
            Ok::<Bytes, std::io::Error>(Bytes::from(e.to_string()))
        }));

        let body = axum::body::Body::from_stream(event_stream);

        axum::response::Response::builder()
            .status(200)
            .header("content-type", "text/event-stream")
            .header("cache-control", "no-cache")
            .header("connection", "keep-alive")
            .header("x-accel-buffering", "no") // Disable nginx buffering
            .body(body)
            .unwrap()
    }
}

impl Default for SseStream {
    fn default() -> Self {
        Self::new()
    }
}

/// Create an SSE response from an async stream of events.
///
/// This is the most powerful form: you provide a stream that yields events
/// over time, and the response streams them to the client.
///
/// # Example
/// ```ignore
/// let response = sse_from_stream(async_stream::stream! {
///     for i in 0..10 {
///         yield SseEvent::data(format!("Count: {}", i));
///         tokio::time::sleep(Duration::from_secs(1)).await;
///     }
/// });
/// ```
pub fn sse_from_stream<S>(stream: S) -> axum::response::Response
where
    S: Stream<Item = SseEvent> + Send + 'static,
{
    let byte_stream = stream.map(|event| {
        Ok::<Bytes, std::io::Error>(Bytes::from(event.to_string()))
    });

    let body = axum::body::Body::from_stream(byte_stream);

    axum::response::Response::builder()
        .status(200)
        .header("content-type", "text/event-stream")
        .header("cache-control", "no-cache")
        .header("connection", "keep-alive")
        .header("x-accel-buffering", "no")
        .body(body)
        .unwrap()
}

/// Generate client-side JavaScript for consuming SSE events.
///
/// Returns a `<script>` tag that connects to the SSE endpoint
/// and dispatches custom DOM events for each SSE event type.
pub fn sse_client_script(endpoint: &str, event_types: &[&str]) -> String {
    let handlers: String = event_types.iter().map(|et| {
        format!(
            "es.addEventListener('{et}',function(e){{document.dispatchEvent(new CustomEvent('sse:{et}',{{detail:JSON.parse(e.data)}}))}}); "
        )
    }).collect();

    format!(
        "<script>(function(){{var es=new EventSource('{endpoint}');{handlers}es.onerror=function(){{setTimeout(function(){{es=new EventSource('{endpoint}')}},3000)}}}})()</script>"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sse_event_data_only() {
        let event = SseEvent::data("hello");
        assert_eq!(event.to_string(), "data: hello\n\n");
    }

    #[test]
    fn test_sse_event_named() {
        let event = SseEvent::named("update", r#"{"count":1}"#);
        assert_eq!(event.to_string(), "event: update\ndata: {\"count\":1}\n\n");
    }

    #[test]
    fn test_sse_event_with_id_and_retry() {
        let event = SseEvent::data("hello")
            .with_id("42")
            .with_retry(3000);
        let s = event.to_string();
        assert!(s.contains("id: 42\n"));
        assert!(s.contains("retry: 3000\n"));
        assert!(s.contains("data: hello\n"));
    }

    #[test]
    fn test_sse_multiline_data() {
        let event = SseEvent::data("line1\nline2\nline3");
        let s = event.to_string();
        assert!(s.contains("data: line1\n"));
        assert!(s.contains("data: line2\n"));
        assert!(s.contains("data: line3\n"));
    }

    #[test]
    fn test_sse_client_script() {
        let script = sse_client_script("/api/events", &["message", "update"]);
        assert!(script.contains("EventSource"));
        assert!(script.contains("/api/events"));
        assert!(script.contains("sse:message"));
        assert!(script.contains("sse:update"));
    }

    #[test]
    fn test_sse_stream_builder() {
        let stream = SseStream::new()
            .event("init", r#"{"ready":true}"#)
            .data("ping");
        assert_eq!(stream.events.len(), 2);
    }
}
