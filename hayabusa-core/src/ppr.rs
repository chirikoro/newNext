//! Partial Prerendering (PPR) for Hayabusa.
//!
//! The most advanced rendering strategy, inspired by Next.js 14+ PPR.
//!
//! ## How it works
//! 1. At build time (or first request), render the **static shell** of the page
//!    (layout, navigation, static content)
//! 2. Mark **dynamic holes** with `<slot>` placeholders
//! 3. On each request:
//!    - Immediately send the cached static shell (near-zero TTFB)
//!    - Stream dynamic content into the holes as it resolves
//!
//! This gives SSG-level speed with SSR-level freshness.
//!
//! ## Usage
//! ```ignore
//! use hayabusa_core::ppr::*;
//!
//! let page = PartialPage::new()
//!     .static_shell(html! {
//!         <main>
//!             <h1>"Dashboard"</h1>
//!             {dynamic_slot("user-info", "<p>Loading user...</p>")}
//!             {dynamic_slot("feed", "<p>Loading feed...</p>")}
//!         </main>
//!     })
//!     .dynamic("user-info", |req| Box::pin(async move {
//!         let user = fetch_user(req).await;
//!         html! { <div class="user">{user.name}</div> }
//!     }))
//!     .dynamic("feed", |req| Box::pin(async move {
//!         let posts = fetch_posts().await;
//!         render_posts(&posts)
//!     }));
//! ```

use std::collections::HashMap;
use std::pin::Pin;
use std::future::Future;
use std::sync::Arc;
use std::time::Instant;

use bytes::Bytes;
use futures::stream::{self, StreamExt};

use crate::component::PageRequest;

/// A handler that produces dynamic content for a slot
pub type DynamicSlotHandler = Arc<
    dyn Fn(PageRequest) -> Pin<Box<dyn Future<Output = String> + Send>>
        + Send
        + Sync,
>;

/// A partially prerendered page with static shell and dynamic holes
pub struct PartialPage {
    /// The static HTML shell (prerendered, cached)
    shell: String,
    /// Dynamic slot handlers keyed by slot ID
    slots: HashMap<String, DynamicSlotHandler>,
    /// Cached shell (after first render)
    cached_shell: Option<CachedShell>,
}

struct CachedShell {
    /// Parts of the shell split at slot boundaries
    /// [before_slot1, before_slot2, ..., after_last_slot]
    parts: Vec<String>,
    /// Slot IDs in order of appearance
    slot_order: Vec<String>,
    /// Fallback HTML for each slot
    fallbacks: HashMap<String, String>,
    #[allow(dead_code)]
    generated_at: Instant,
}

impl PartialPage {
    pub fn new() -> Self {
        Self {
            shell: String::new(),
            slots: HashMap::new(),
            cached_shell: None,
        }
    }

    /// Set the static shell HTML.
    /// Use `dynamic_slot("id", "fallback html")` to mark dynamic holes.
    pub fn static_shell(mut self, html: String) -> Self {
        self.shell = html;
        self
    }

    /// Register a dynamic slot handler
    pub fn dynamic(
        mut self,
        id: impl Into<String>,
        handler: impl Fn(PageRequest) -> Pin<Box<dyn Future<Output = String> + Send>>
            + Send
            + Sync
            + 'static,
    ) -> Self {
        self.slots.insert(id.into(), Arc::new(handler));
        self
    }

    /// Parse the shell into parts and slot locations for efficient streaming.
    fn prepare_shell(&mut self) {
        let mut parts = Vec::new();
        let mut slot_order = Vec::new();
        let mut fallbacks = HashMap::new();
        let mut remaining = self.shell.as_str();

        // Find all PPR slot markers: <!--PPR:id-->fallback<!--/PPR:id-->
        while let Some(start_pos) = remaining.find("<!--PPR:") {
            // Add the part before this slot
            parts.push(remaining[..start_pos].to_string());

            // Find the slot ID
            let after_marker = &remaining[start_pos + 8..];
            if let Some(id_end) = after_marker.find("-->") {
                let slot_id = after_marker[..id_end].to_string();
                let after_id = &after_marker[id_end + 3..];

                // Find the closing marker
                let close_marker = format!("<!--/PPR:{}-->", slot_id);
                if let Some(close_pos) = after_id.find(&close_marker) {
                    let fallback = after_id[..close_pos].to_string();
                    fallbacks.insert(slot_id.clone(), fallback);
                    slot_order.push(slot_id);
                    remaining = &after_id[close_pos + close_marker.len()..];
                } else {
                    // Malformed: no closing marker, treat as static content
                    parts.last_mut().unwrap().push_str(&remaining[start_pos..start_pos + 8]);
                    remaining = &remaining[start_pos + 8..];
                }
            } else {
                break;
            }
        }

        // Add the remaining content after all slots
        parts.push(remaining.to_string());

        self.cached_shell = Some(CachedShell {
            parts,
            slot_order,
            fallbacks,
            generated_at: Instant::now(),
        });
    }

    /// Render the page as a streaming response.
    ///
    /// 1. Sends the static shell immediately (with fallback content in slots)
    /// 2. Resolves dynamic slots concurrently
    /// 3. Streams replacement scripts as each slot resolves
    pub fn render_streaming(
        &mut self,
        request: PageRequest,
    ) -> axum::response::Response {
        // Prepare the shell if not already done
        if self.cached_shell.is_none() {
            self.prepare_shell();
        }

        let cached = self.cached_shell.as_ref().unwrap();

        // Build the initial HTML with fallbacks in slot positions
        let mut initial_html = String::new();
        for (i, part) in cached.parts.iter().enumerate() {
            initial_html.push_str(part);
            if i < cached.slot_order.len() {
                let slot_id = &cached.slot_order[i];
                let fallback = cached.fallbacks.get(slot_id).cloned().unwrap_or_default();
                initial_html.push_str(&format!(
                    "<div id=\"__ppr_{slot_id}\">{fallback}</div>"
                ));
            }
        }

        // Add the PPR resolution script
        initial_html.push_str(
            "<script>\
            window.__ppr_resolve=function(id,html){\
            var el=document.getElementById('__ppr_'+id);\
            if(el){var t=document.createElement('template');t.innerHTML=html;el.replaceWith(t.content)}\
            };\
            </script>"
        );

        // Start stream with the full shell
        let initial_chunk = stream::once(async move {
            Ok::<Bytes, std::io::Error>(Bytes::from(initial_html))
        });

        // Create futures for each dynamic slot
        let mut slot_futures = Vec::new();
        for slot_id in &cached.slot_order {
            if let Some(handler) = self.slots.get(slot_id) {
                let handler = handler.clone();
                let req = request.clone();
                let id = slot_id.clone();

                slot_futures.push(async move {
                    let content = handler(req).await;
                    let escaped = content
                        .replace('\\', "\\\\")
                        .replace('\'', "\\'")
                        .replace('\n', "\\n");
                    let script = format!(
                        "<script>__ppr_resolve('{id}','{escaped}')</script>"
                    );
                    Ok::<Bytes, std::io::Error>(Bytes::from(script))
                });
            }
        }

        // Stream slot resolutions as they complete
        let slot_stream = stream::iter(slot_futures)
            .buffer_unordered(10)  // Resolve up to 10 slots concurrently
            .map(|result| result);

        let body_stream = initial_chunk.chain(slot_stream);
        let body = axum::body::Body::from_stream(body_stream);

        axum::response::Response::builder()
            .status(200)
            .header("content-type", "text/html; charset=utf-8")
            .header("transfer-encoding", "chunked")
            .header("cache-control", "private, no-cache")
            .body(body)
            .unwrap()
    }
}

impl Default for PartialPage {
    fn default() -> Self {
        Self::new()
    }
}

/// Create a PPR slot marker in the static shell HTML.
///
/// The fallback is shown immediately while the dynamic content loads.
pub fn dynamic_slot(id: &str, fallback_html: &str) -> String {
    format!("<!--PPR:{id}-->{fallback_html}<!--/PPR:{id}-->")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dynamic_slot_marker() {
        let slot = dynamic_slot("user", "<p>Loading...</p>");
        assert_eq!(slot, "<!--PPR:user--><p>Loading...</p><!--/PPR:user-->");
    }

    #[test]
    fn test_shell_parsing() {
        let shell = format!(
            "<main><h1>Title</h1>{}{}</main>",
            dynamic_slot("a", "<p>Loading A</p>"),
            dynamic_slot("b", "<p>Loading B</p>"),
        );

        let mut page = PartialPage::new().static_shell(shell);
        page.prepare_shell();

        let cached = page.cached_shell.as_ref().unwrap();
        assert_eq!(cached.parts.len(), 3); // before_a, between_a_b, after_b
        assert_eq!(cached.slot_order, vec!["a", "b"]);
        assert_eq!(cached.fallbacks.get("a").unwrap(), "<p>Loading A</p>");
        assert_eq!(cached.fallbacks.get("b").unwrap(), "<p>Loading B</p>");
    }

    #[test]
    fn test_shell_with_no_slots() {
        let shell = "<main><h1>Static Page</h1></main>".to_string();
        let mut page = PartialPage::new().static_shell(shell);
        page.prepare_shell();

        let cached = page.cached_shell.as_ref().unwrap();
        assert_eq!(cached.parts.len(), 1);
        assert!(cached.slot_order.is_empty());
    }
}
