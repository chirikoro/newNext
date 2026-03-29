//! Hot Reload for Hayabusa Development Server.
//!
//! WebSocket-based live reload that instantly refreshes the browser
//! when files change. No manual refresh needed.
//!
//! ## How It Works
//! 1. Dev server injects a tiny WebSocket client into every HTML page
//! 2. File watcher detects changes to templates, markdown, CSS, etc.
//! 3. Server sends "reload" message over WebSocket
//! 4. Browser refreshes instantly (or applies CSS without full reload)
//!
//! ## Key Feature: NO recompilation for non-Rust files
//! - `.html` templates → instant reload
//! - `.md` markdown → instant reload
//! - `.css` styles → hot CSS swap (no flash)
//! - `.toml` config → reload routes
//! - `.rs` files → triggers `cargo build` then reload

/// Client-side hot reload script to inject into HTML during development.
///
/// Connects via WebSocket and reloads on file changes.
/// CSS changes are applied without a full page refresh.
pub fn hot_reload_script(ws_port: u16) -> String {
    format!(
        r#"<script>
(function(){{
var ws;var retry=0;
function connect(){{
ws=new WebSocket('ws://localhost:{port}/__hayabusa_hmr');
ws.onopen=function(){{retry=0;console.log('[Hayabusa] Hot reload connected')}};
ws.onmessage=function(e){{
var msg=JSON.parse(e.data);
if(msg.type==='css'){{
document.querySelectorAll('link[rel=stylesheet]').forEach(function(l){{
var href=l.href.split('?')[0];
l.href=href+'?t='+Date.now();
}});
console.log('[Hayabusa] CSS updated');
}}else if(msg.type==='reload'){{
console.log('[Hayabusa] Reloading...');
location.reload();
}}else if(msg.type==='error'){{
showError(msg.message);
}}
}};
ws.onclose=function(){{
retry++;
var delay=Math.min(1000*Math.pow(2,retry),30000);
console.log('[Hayabusa] Reconnecting in '+delay+'ms...');
setTimeout(connect,delay);
}};
}}
function showError(msg){{
var el=document.getElementById('__hayabusa_error');
if(!el){{
el=document.createElement('div');
el.id='__hayabusa_error';
el.style.cssText='position:fixed;top:0;left:0;right:0;z-index:99999;padding:16px 20px;background:#1a1a2e;color:#ff6b6b;font-family:monospace;font-size:14px;white-space:pre-wrap;border-bottom:3px solid #ff6b6b;cursor:pointer;max-height:40vh;overflow:auto';
el.onclick=function(){{el.remove()}};
document.body.prepend(el);
}}
el.textContent=msg;
}}
connect();
}})();
</script>"#,
        port = ws_port
    )
}

/// Determine the reload type based on file extension
pub fn reload_type_for_file(path: &str) -> ReloadType {
    if path.ends_with(".css") {
        ReloadType::CssOnly
    } else if path.ends_with(".rs") {
        ReloadType::RustRecompile
    } else if path.ends_with(".html")
        || path.ends_with(".md")
        || path.ends_with(".toml")
        || path.ends_with(".json")
    {
        ReloadType::FullReload
    } else if path.ends_with(".js") || path.ends_with(".ts") {
        ReloadType::FullReload
    } else {
        ReloadType::Ignore
    }
}

/// Type of reload action
#[derive(Debug, Clone, PartialEq)]
pub enum ReloadType {
    /// Only swap CSS stylesheets (no flash)
    CssOnly,
    /// Full page reload (templates, markdown, config)
    FullReload,
    /// Needs Rust recompilation first, then reload
    RustRecompile,
    /// File type that doesn't need reload
    Ignore,
}

/// Generate the WebSocket message JSON for a reload event
pub fn reload_message(reload_type: &ReloadType, _file_path: Option<&str>) -> String {
    match reload_type {
        ReloadType::CssOnly => r#"{"type":"css"}"#.to_string(),
        ReloadType::FullReload => r#"{"type":"reload"}"#.to_string(),
        ReloadType::RustRecompile => r#"{"type":"reload"}"#.to_string(),
        ReloadType::Ignore => String::new(),
    }
}

/// Generate an error overlay message
pub fn error_message(error: &str) -> String {
    let escaped = error.replace('\\', "\\\\").replace('"', "\\\"").replace('\n', "\\n");
    format!(r#"{{"type":"error","message":"{}"}}"#, escaped)
}

/// Dev overlay HTML for displaying compile errors beautifully.
///
/// Injected only in development mode.
pub fn dev_error_overlay(error: &str) -> String {
    format!(
        r#"<!DOCTYPE html>
<html>
<head>
<meta charset="utf-8" />
<title>Build Error - Hayabusa</title>
<style>
*{{margin:0;padding:0;box-sizing:border-box}}
body{{background:#1a1a2e;color:#e0e0e0;font-family:-apple-system,BlinkMacSystemFont,system-ui,sans-serif;padding:40px}}
.container{{max-width:900px;margin:0 auto}}
.header{{display:flex;align-items:center;gap:12px;margin-bottom:24px}}
.header h1{{font-size:1.5rem;color:#ff6b6b}}
.badge{{background:#ff6b6b;color:#fff;padding:4px 10px;border-radius:4px;font-size:0.75rem;font-weight:600}}
.error-box{{background:#16213e;border:1px solid #2a2a4a;border-left:4px solid #ff6b6b;border-radius:8px;padding:24px;font-family:'Fira Code',monospace;font-size:0.9rem;line-height:1.6;white-space:pre-wrap;overflow-x:auto}}
.hint{{margin-top:20px;padding:16px;background:#16213e;border:1px solid #2a2a4a;border-left:4px solid #4ecdc4;border-radius:8px}}
.hint h3{{color:#4ecdc4;margin-bottom:8px;font-size:0.9rem}}
.footer{{margin-top:32px;text-align:center;opacity:0.5;font-size:0.8rem}}
</style>
</head>
<body>
<div class="container">
<div class="header">
<h1>🚨 Build Error</h1>
<span class="badge">DEVELOPMENT</span>
</div>
<div class="error-box">{error}</div>
<div class="hint">
<h3>💡 Fix and save — Hayabusa will auto-reload</h3>
<p>Template and Markdown changes don't need recompilation.</p>
</div>
<div class="footer">Hayabusa (隼) Dev Server</div>
</div>
</body>
</html>"#,
        error = error.replace('<', "&lt;").replace('>', "&gt;")
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hot_reload_script() {
        let script = hot_reload_script(3001);
        assert!(script.contains("WebSocket"));
        assert!(script.contains("3001"));
        assert!(script.contains("__hayabusa_hmr"));
    }

    #[test]
    fn test_reload_type_css() {
        assert_eq!(reload_type_for_file("style.css"), ReloadType::CssOnly);
        assert_eq!(reload_type_for_file("/public/main.css"), ReloadType::CssOnly);
    }

    #[test]
    fn test_reload_type_templates() {
        assert_eq!(reload_type_for_file("index.html"), ReloadType::FullReload);
        assert_eq!(reload_type_for_file("blog/post.md"), ReloadType::FullReload);
        assert_eq!(reload_type_for_file("config.toml"), ReloadType::FullReload);
    }

    #[test]
    fn test_reload_type_rust() {
        assert_eq!(reload_type_for_file("src/main.rs"), ReloadType::RustRecompile);
    }

    #[test]
    fn test_reload_type_ignore() {
        assert_eq!(reload_type_for_file("image.png"), ReloadType::Ignore);
        assert_eq!(reload_type_for_file("data.bin"), ReloadType::Ignore);
    }

    #[test]
    fn test_reload_messages() {
        assert!(reload_message(&ReloadType::CssOnly, None).contains("css"));
        assert!(reload_message(&ReloadType::FullReload, None).contains("reload"));
    }

    #[test]
    fn test_error_message() {
        let msg = error_message("line 5: unexpected token");
        assert!(msg.contains("error"));
        assert!(msg.contains("unexpected token"));
    }

    #[test]
    fn test_dev_error_overlay() {
        let html = dev_error_overlay("error[E0308]: mismatched types");
        assert!(html.contains("Build Error"));
        assert!(html.contains("mismatched types"));
        assert!(html.contains("auto-reload"));
    }
}
