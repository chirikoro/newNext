use axum::response::{Html, IntoResponse, Response};
use bytes::Bytes;
use futures::stream::{self, Stream, StreamExt};
use http::StatusCode;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::pin::Pin;
use crate::component::{HeadContext, RenderMode, RenderResult};
use crate::layout::{self, Layout, RootLayout};

/// Render a page with its layouts into a full HTML response,
/// with appropriate Cache-Control and ETag headers based on render mode.
pub fn render_page(
    content: &RenderResult,
    layouts: &[&dyn Layout],
    render_mode: &RenderMode,
) -> Response {
    let html = if layouts.is_empty() {
        let root = RootLayout::default();
        root.render(&content.html, &content.head)
    } else {
        layout::compose_layouts(layouts, &content.html, &content.head)
    };

    let minified = minify_html(&html);
    let etag = generate_etag(&minified);

    let mut builder = Response::builder()
        .status(StatusCode::OK)
        .header("content-type", "text/html; charset=utf-8")
        .header("etag", &etag)
        .header("vary", "Accept-Encoding");

    // Set Cache-Control based on render mode
    builder = match render_mode {
        RenderMode::Ssr => builder
            .header("cache-control", "private, no-cache, must-revalidate"),
        RenderMode::Ssg { revalidate: None } => builder
            .header("cache-control", "public, max-age=31536000, immutable"),
        RenderMode::Ssg { revalidate: Some(dur) } => builder
            .header("cache-control", format!(
                "public, s-maxage={}, stale-while-revalidate={}",
                dur.as_secs(),
                dur.as_secs() * 2
            )),
        RenderMode::Streaming => builder
            .header("cache-control", "private, no-cache"),
    };

    // Add preload link headers for critical resources
    let link_headers = build_preload_headers(&content.head);
    if !link_headers.is_empty() {
        builder = builder.header("link", link_headers);
    }

    builder
        .body(axum::body::Body::from(minified))
        .unwrap()
}

/// Render a page as a streaming response with Suspense-like pattern.
///
/// Strategy: Send the HTML shell immediately (fast TTFB), then stream
/// content chunks. Each chunk is flushed immediately to the client.
/// Supports out-of-order streaming with placeholder replacement via
/// inline JavaScript.
pub fn render_streaming(
    head: &HeadContext,
    content_stream: Pin<Box<dyn Stream<Item = String> + Send>>,
) -> Response {
    let title = head.title.as_deref().unwrap_or("Hayabusa App");
    let head_meta = head.render();
    let preload_tags = build_preload_tags(head);

    // Shell includes the suspense replacement script
    let shell_start = format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8" />
<meta name="viewport" content="width=device-width, initial-scale=1" />
<title>{title}</title>
{preload_tags}{head_meta}<script>
// Hayabusa streaming: replace suspense fallbacks with resolved content
window.__hayabusa_resolve=function(id,html){{
var el=document.getElementById('H:'+id);
if(el){{var t=document.createElement('template');t.innerHTML=html;el.replaceWith(t.content)}}
}};
</script>
</head>
<body>
"#
    );

    let shell_end = "</body>\n</html>".to_string();

    let start_stream = stream::once(async move {
        Ok::<Bytes, std::io::Error>(Bytes::from(shell_start))
    });

    let content_stream = content_stream.map(|chunk| {
        Ok::<Bytes, std::io::Error>(Bytes::from(chunk))
    });

    let end_stream = stream::once(async move {
        Ok::<Bytes, std::io::Error>(Bytes::from(shell_end))
    });

    let body_stream = start_stream.chain(content_stream).chain(end_stream);

    let body = axum::body::Body::from_stream(body_stream);

    Response::builder()
        .status(StatusCode::OK)
        .header("content-type", "text/html; charset=utf-8")
        .header("transfer-encoding", "chunked")
        .header("cache-control", "private, no-cache")
        .header("x-content-type-options", "nosniff")
        .body(body)
        .unwrap()
}

/// Generate a suspense placeholder that will be replaced when content resolves.
pub fn suspense_placeholder(id: &str, fallback_html: &str) -> String {
    format!(r#"<div id="H:{id}">{fallback_html}</div>"#)
}

/// Generate the script tag that resolves a suspense placeholder.
pub fn suspense_resolve(id: &str, content_html: &str) -> String {
    let escaped = content_html
        .replace('\\', "\\\\")
        .replace('\'', "\\'")
        .replace('\n', "\\n");
    format!(r#"<script>__hayabusa_resolve('{id}','{escaped}')</script>"#)
}

/// Render a simple HTML string response
pub fn html_response(html: String) -> Response {
    Html(html).into_response()
}

/// Render a JSON response with appropriate caching headers
pub fn json_response<T: serde::Serialize>(data: &T) -> Response {
    match serde_json::to_string(data) {
        Ok(json) => {
            let etag = generate_etag(&json);
            Response::builder()
                .status(StatusCode::OK)
                .header("content-type", "application/json; charset=utf-8")
                .header("etag", &etag)
                .header("cache-control", "private, no-cache")
                .body(axum::body::Body::from(json))
                .unwrap()
        }
        Err(err) => Response::builder()
            .status(StatusCode::INTERNAL_SERVER_ERROR)
            .body(axum::body::Body::from(format!("Serialization error: {err}")))
            .unwrap(),
    }
}

/// Check if a request's If-None-Match header matches the given ETag.
/// Returns a 304 Not Modified response if it does.
pub fn check_etag(if_none_match: Option<&str>, etag: &str) -> Option<Response> {
    if let Some(client_etag) = if_none_match {
        if client_etag == etag || client_etag == format!("W/{etag}") {
            return Some(
                Response::builder()
                    .status(StatusCode::NOT_MODIFIED)
                    .header("etag", etag)
                    .body(axum::body::Body::empty())
                    .unwrap(),
            );
        }
    }
    None
}

/// Generate a weak ETag from content using a fast hash
pub fn generate_etag(content: &str) -> String {
    let mut hasher = DefaultHasher::new();
    content.hash(&mut hasher);
    format!("W/\"{:x}\"", hasher.finish())
}

/// Minify HTML by removing unnecessary whitespace.
/// Preserves whitespace inside <pre>, <code>, <script>, <style> tags.
pub fn minify_html(html: &str) -> String {
    let mut result = String::with_capacity(html.len());
    let mut in_pre = false;
    let mut last_was_whitespace = false;
    let mut chars = html.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '<' {
            // Check for pre/code/script/style tags
            let remaining: String = chars.clone().take(10).collect();
            let lower = remaining.to_lowercase();

            if lower.starts_with("pre") || lower.starts_with("code")
                || lower.starts_with("script") || lower.starts_with("style")
            {
                in_pre = true;
            } else if lower.starts_with("/pre") || lower.starts_with("/code")
                || lower.starts_with("/script") || lower.starts_with("/style")
            {
                in_pre = false;
            }
            result.push(ch);
            last_was_whitespace = false;
        } else if in_pre {
            result.push(ch);
            last_was_whitespace = false;
        } else if ch.is_whitespace() {
            if !last_was_whitespace {
                result.push(' ');
                last_was_whitespace = true;
            }
        } else {
            result.push(ch);
            last_was_whitespace = false;
        }
    }

    result
}

/// Build HTTP Link headers for resource preloading (CSS, fonts, scripts).
/// These allow the browser to start fetching resources before parsing HTML.
fn build_preload_headers(head: &HeadContext) -> String {
    let mut links = Vec::new();

    for href in &head.extra_links {
        links.push(format!("<{href}>; rel=preload; as=style"));
    }

    for src in &head.scripts {
        links.push(format!("<{src}>; rel=preload; as=script"));
    }

    links.join(", ")
}

/// Build HTML preload tags for the <head> section.
fn build_preload_tags(head: &HeadContext) -> String {
    let mut tags = String::new();

    for href in &head.extra_links {
        tags.push_str(&format!(
            "<link rel=\"preload\" href=\"{href}\" as=\"style\" />\n"
        ));
    }

    for src in &head.scripts {
        tags.push_str(&format!(
            "<link rel=\"preload\" href=\"{src}\" as=\"script\" />\n"
        ));
    }

    tags
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_minify_html() {
        let input = "<div>\n    <h1>  Hello  </h1>\n    <p>  World  </p>\n</div>";
        let output = minify_html(input);
        assert_eq!(output, "<div> <h1> Hello </h1> <p> World </p> </div>");
    }

    #[test]
    fn test_minify_preserves_pre() {
        let input = "<pre>\n  code here\n  indented\n</pre>";
        let output = minify_html(input);
        assert!(output.contains("\n  code here\n  indented\n"));
    }

    #[test]
    fn test_etag_generation() {
        let etag1 = generate_etag("hello");
        let etag2 = generate_etag("hello");
        let etag3 = generate_etag("world");
        assert_eq!(etag1, etag2);
        assert_ne!(etag1, etag3);
        assert!(etag1.starts_with("W/\""));
    }

    #[test]
    fn test_check_etag_match() {
        let etag = generate_etag("test");
        assert!(check_etag(Some(&etag), &etag).is_some());
        assert!(check_etag(Some("wrong"), &etag).is_none());
        assert!(check_etag(None, &etag).is_none());
    }

    #[test]
    fn test_suspense_placeholder_and_resolve() {
        let placeholder = suspense_placeholder("1", "<p>Loading...</p>");
        assert!(placeholder.contains("H:1"));
        assert!(placeholder.contains("Loading..."));

        let resolve = suspense_resolve("1", "<p>Done!</p>");
        assert!(resolve.contains("__hayabusa_resolve"));
        assert!(resolve.contains("Done!"));
    }

    #[test]
    fn test_preload_headers() {
        let head = HeadContext::new()
            .link("/style.css")
            .link("/fonts.css");
        let headers = build_preload_headers(&head);
        assert!(headers.contains("</style.css>; rel=preload; as=style"));
        assert!(headers.contains("</fonts.css>; rel=preload; as=style"));
    }
}
