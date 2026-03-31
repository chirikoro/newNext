//! WebSocket Support for Hayabusa.
//!
//! Bidirectional real-time communication using WebSocket protocol.
//! Provides room management, broadcasting, and client-side integration scripts.
//!
//! ```ignore
//! use hayabusa_core::prelude::*;
//!
//! let ws = WsRoom::new("chat")
//!     .max_clients(100)
//!     .heartbeat_interval(30);
//! ```

use std::collections::HashMap;
use std::sync::Arc;
use dashmap::DashMap;

// ─── WebSocket Message ──────────────────────────────────────

/// WebSocket message types
#[derive(Debug, Clone)]
pub enum WsMessage {
    Text(String),
    Binary(Vec<u8>),
    Ping,
    Pong,
    Close(Option<String>),
}

impl WsMessage {
    pub fn text(msg: impl Into<String>) -> Self {
        WsMessage::Text(msg.into())
    }

    pub fn json(data: &serde_json::Value) -> Self {
        WsMessage::Text(data.to_string())
    }

    pub fn as_text(&self) -> Option<&str> {
        match self {
            WsMessage::Text(s) => Some(s),
            _ => None,
        }
    }

    pub fn is_text(&self) -> bool {
        matches!(self, WsMessage::Text(_))
    }
}

// ─── WebSocket Room ─────────────────────────────────────────

/// A WebSocket room for managing connected clients
#[derive(Debug, Clone)]
pub struct WsRoom {
    pub name: String,
    pub max_clients: usize,
    pub heartbeat_interval: u64,
    clients: Arc<DashMap<String, WsClient>>,
}

/// A connected WebSocket client
#[derive(Debug, Clone)]
pub struct WsClient {
    pub id: String,
    pub metadata: HashMap<String, String>,
}

impl WsRoom {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            max_clients: 1000,
            heartbeat_interval: 30,
            clients: Arc::new(DashMap::new()),
        }
    }

    pub fn max_clients(mut self, max: usize) -> Self {
        self.max_clients = max;
        self
    }

    pub fn heartbeat_interval(mut self, secs: u64) -> Self {
        self.heartbeat_interval = secs;
        self
    }

    /// Add a client to the room
    pub fn add_client(&self, id: impl Into<String>) -> bool {
        let id = id.into();
        if self.clients.len() >= self.max_clients {
            return false;
        }
        self.clients.insert(
            id.clone(),
            WsClient {
                id,
                metadata: HashMap::new(),
            },
        );
        true
    }

    /// Add a client with metadata
    pub fn add_client_with_meta(&self, id: impl Into<String>, meta: HashMap<String, String>) -> bool {
        let id = id.into();
        if self.clients.len() >= self.max_clients {
            return false;
        }
        self.clients.insert(
            id.clone(),
            WsClient { id, metadata: meta },
        );
        true
    }

    /// Remove a client from the room
    pub fn remove_client(&self, id: &str) -> bool {
        self.clients.remove(id).is_some()
    }

    /// Get the number of connected clients
    pub fn client_count(&self) -> usize {
        self.clients.len()
    }

    /// Check if a client is in the room
    pub fn has_client(&self, id: &str) -> bool {
        self.clients.contains_key(id)
    }

    /// Get all client IDs
    pub fn client_ids(&self) -> Vec<String> {
        self.clients.iter().map(|e| e.key().clone()).collect()
    }

    /// Prepare a broadcast message (returns serialized JSON event)
    pub fn broadcast_event(&self, event: &str, data: &str) -> String {
        format!(
            "{{\"type\":\"broadcast\",\"room\":\"{}\",\"event\":\"{}\",\"data\":{},\"clients\":{}}}",
            self.name, event, data, self.client_count()
        )
    }

    /// Prepare a direct message to a specific client
    pub fn direct_message(&self, client_id: &str, event: &str, data: &str) -> Option<String> {
        if self.has_client(client_id) {
            Some(format!(
                "{{\"type\":\"direct\",\"to\":\"{}\",\"event\":\"{}\",\"data\":{}}}",
                client_id, event, data
            ))
        } else {
            None
        }
    }
}

// ─── WebSocket Hub ──────────────────────────────────────────

/// Central hub managing multiple WebSocket rooms
#[derive(Debug, Clone)]
pub struct WsHub {
    rooms: Arc<DashMap<String, WsRoom>>,
}

impl WsHub {
    pub fn new() -> Self {
        Self {
            rooms: Arc::new(DashMap::new()),
        }
    }

    /// Create or get a room
    pub fn room(&self, name: &str) -> WsRoom {
        if let Some(room) = self.rooms.get(name) {
            return room.clone();
        }
        let room = WsRoom::new(name);
        self.rooms.insert(name.to_string(), room.clone());
        room
    }

    /// Create a room with configuration
    pub fn create_room(&self, room: WsRoom) {
        self.rooms.insert(room.name.clone(), room);
    }

    /// Remove a room
    pub fn remove_room(&self, name: &str) -> bool {
        self.rooms.remove(name).is_some()
    }

    /// Get all room names
    pub fn room_names(&self) -> Vec<String> {
        self.rooms.iter().map(|e| e.key().clone()).collect()
    }

    /// Total connected clients across all rooms
    pub fn total_clients(&self) -> usize {
        self.rooms.iter().map(|r| r.client_count()).sum()
    }
}

impl Default for WsHub {
    fn default() -> Self {
        Self::new()
    }
}

// ─── WebSocket Route Config ─────────────────────────────────

/// Configuration for a WebSocket endpoint
#[derive(Debug, Clone)]
pub struct WsEndpoint {
    pub path: String,
    pub room: String,
    pub max_message_size: usize,
    pub require_auth: bool,
    pub allowed_origins: Vec<String>,
}

impl WsEndpoint {
    pub fn new(path: impl Into<String>, room: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            room: room.into(),
            max_message_size: 64 * 1024, // 64KB
            require_auth: false,
            allowed_origins: Vec::new(),
        }
    }

    pub fn max_message_size(mut self, bytes: usize) -> Self {
        self.max_message_size = bytes;
        self
    }

    pub fn require_auth(mut self) -> Self {
        self.require_auth = true;
        self
    }

    pub fn allowed_origin(mut self, origin: impl Into<String>) -> Self {
        self.allowed_origins.push(origin.into());
        self
    }

    /// Check if an origin is allowed
    pub fn is_origin_allowed(&self, origin: &str) -> bool {
        self.allowed_origins.is_empty() || self.allowed_origins.iter().any(|o| o == origin || o == "*")
    }
}

// ─── Client-Side WebSocket Script ───────────────────────────

/// Generate client-side WebSocket connection script
pub fn ws_client_script(path: &str, options: &WsClientOptions) -> String {
    format!(
        r#"<script>
(function() {{
  const WS_PATH = '{path}';
  const RECONNECT = {reconnect};
  const MAX_RETRIES = {max_retries};
  const HEARTBEAT = {heartbeat};

  let ws, retries = 0, heartbeatTimer;

  function connect() {{
    const proto = location.protocol === 'https:' ? 'wss:' : 'ws:';
    ws = new WebSocket(proto + '//' + location.host + WS_PATH);

    ws.onopen = function() {{
      retries = 0;
      console.log('[WS] Connected to', WS_PATH);
      if (HEARTBEAT > 0) {{
        heartbeatTimer = setInterval(() => ws.send(JSON.stringify({{type:'ping'}})), HEARTBEAT * 1000);
      }}
      document.dispatchEvent(new CustomEvent('ws:open'));
    }};

    ws.onmessage = function(e) {{
      try {{
        const msg = JSON.parse(e.data);
        document.dispatchEvent(new CustomEvent('ws:message', {{detail: msg}}));
        if (msg.event) {{
          document.dispatchEvent(new CustomEvent('ws:' + msg.event, {{detail: msg.data}}));
        }}
      }} catch(err) {{
        document.dispatchEvent(new CustomEvent('ws:raw', {{detail: e.data}}));
      }}
    }};

    ws.onclose = function() {{
      clearInterval(heartbeatTimer);
      document.dispatchEvent(new CustomEvent('ws:close'));
      if (RECONNECT && retries < MAX_RETRIES) {{
        retries++;
        const delay = Math.min(1000 * Math.pow(2, retries), 30000);
        console.log('[WS] Reconnecting in', delay, 'ms (attempt', retries + '/' + MAX_RETRIES + ')');
        setTimeout(connect, delay);
      }}
    }};

    ws.onerror = function(e) {{
      console.error('[WS] Error:', e);
    }};
  }}

  window.wsSend = function(event, data) {{
    if (ws && ws.readyState === WebSocket.OPEN) {{
      ws.send(JSON.stringify({{event: event, data: data}}));
    }}
  }};

  window.wsClose = function() {{
    if (ws) ws.close();
  }};

  connect();
}})();
</script>"#,
        path = path,
        reconnect = options.auto_reconnect,
        max_retries = options.max_retries,
        heartbeat = options.heartbeat_interval,
    )
}

/// Client-side WebSocket options
#[derive(Debug, Clone)]
pub struct WsClientOptions {
    pub auto_reconnect: bool,
    pub max_retries: u32,
    pub heartbeat_interval: u64,
}

impl Default for WsClientOptions {
    fn default() -> Self {
        Self {
            auto_reconnect: true,
            max_retries: 10,
            heartbeat_interval: 30,
        }
    }
}

// ─── Tests ──────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ws_message_text() {
        let msg = WsMessage::text("hello");
        assert!(msg.is_text());
        assert_eq!(msg.as_text(), Some("hello"));
    }

    #[test]
    fn test_ws_message_binary() {
        let msg = WsMessage::Binary(vec![1, 2, 3]);
        assert!(!msg.is_text());
        assert_eq!(msg.as_text(), None);
    }

    #[test]
    fn test_ws_room() {
        let room = WsRoom::new("chat").max_clients(2);
        assert!(room.add_client("user1"));
        assert!(room.add_client("user2"));
        assert!(!room.add_client("user3")); // full
        assert_eq!(room.client_count(), 2);
    }

    #[test]
    fn test_ws_room_remove() {
        let room = WsRoom::new("test");
        room.add_client("u1");
        assert!(room.has_client("u1"));
        room.remove_client("u1");
        assert!(!room.has_client("u1"));
    }

    #[test]
    fn test_ws_room_broadcast() {
        let room = WsRoom::new("chat");
        room.add_client("u1");
        let msg = room.broadcast_event("message", "\"hello\"");
        assert!(msg.contains("\"broadcast\""));
        assert!(msg.contains("\"chat\""));
        assert!(msg.contains("\"clients\":1"));
    }

    #[test]
    fn test_ws_room_direct() {
        let room = WsRoom::new("chat");
        room.add_client("u1");
        assert!(room.direct_message("u1", "msg", "\"hi\"").is_some());
        assert!(room.direct_message("u2", "msg", "\"hi\"").is_none());
    }

    #[test]
    fn test_ws_hub() {
        let hub = WsHub::new();
        let room = hub.room("lobby");
        room.add_client("u1");
        assert_eq!(hub.total_clients(), 1);
        assert!(hub.room_names().contains(&"lobby".to_string()));
    }

    #[test]
    fn test_ws_hub_remove() {
        let hub = WsHub::new();
        hub.room("test");
        assert!(hub.remove_room("test"));
        assert!(!hub.remove_room("nonexistent"));
    }

    #[test]
    fn test_ws_endpoint() {
        let ep = WsEndpoint::new("/ws/chat", "chat")
            .max_message_size(1024)
            .require_auth()
            .allowed_origin("https://example.com");
        assert!(ep.require_auth);
        assert!(ep.is_origin_allowed("https://example.com"));
        assert!(!ep.is_origin_allowed("https://evil.com"));
    }

    #[test]
    fn test_ws_endpoint_any_origin() {
        let ep = WsEndpoint::new("/ws", "room");
        // Empty allowed_origins means allow all
        assert!(ep.is_origin_allowed("anything"));
    }

    #[test]
    fn test_ws_endpoint_wildcard() {
        let ep = WsEndpoint::new("/ws", "room").allowed_origin("*");
        assert!(ep.is_origin_allowed("anything"));
    }

    #[test]
    fn test_client_script() {
        let script = ws_client_script("/ws/chat", &WsClientOptions::default());
        assert!(script.contains("/ws/chat"));
        assert!(script.contains("WebSocket"));
        assert!(script.contains("RECONNECT"));
        assert!(script.contains("wsSend"));
    }

    #[test]
    fn test_client_with_meta() {
        let room = WsRoom::new("r");
        let mut meta = HashMap::new();
        meta.insert("role".to_string(), "admin".to_string());
        room.add_client_with_meta("u1", meta);
        assert!(room.has_client("u1"));
    }

    #[test]
    fn test_room_client_ids() {
        let room = WsRoom::new("r");
        room.add_client("a");
        room.add_client("b");
        let ids = room.client_ids();
        assert_eq!(ids.len(), 2);
        assert!(ids.contains(&"a".to_string()));
    }
}
