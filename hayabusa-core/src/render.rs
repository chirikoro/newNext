use axum::response::{Html, IntoResponse, Response};
use bytes::Bytes;
use futures::stream::{self, Stream, StreamExt};
use http::StatusCode;
use std::pin::Pin;

use crate::component::{HeadContext, RenderResult};
use crate::layout::{self, Layout, RootLayout};

/// Render a page with its layouts into a full HTML response.
pub fn render_page(
    content: &RenderResult,
    layouts: &[&dyn Layout],
) -> Response {
    let html = if layouts.is_empty() {
        let root = RootLayout::default();
        root.render(&content.html, &content.head)
    } else {
        layout::compose_layouts(layouts, &content.html, &content.head)
    };

    Html(html).into_response()
}

/// Render a page as a streaming response.
///
/// Sends the HTML shell (head + opening body) immediately, then streams
/// the page content. This provides a fast Time-To-First-Byte (TTFB).
pub fn render_streaming(
    head: &HeadContext,
    content_stream: Pin<Box<dyn Stream<Item = String> + Send>>,
) -> Response {
    let title = head.title.as_deref().unwrap_or("Hayabusa App");
    let head_meta = head.render();

    let shell_start = format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="utf-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1" />
    <title>{title}</title>
{head_meta}</head>
<body>
"#
    );

    let shell_end = "\n</body>\n</html>".to_string();

    let start_stream = stream::once(async move { Ok::<Bytes, std::io::Error>(Bytes::from(shell_start)) });

    let content_stream = content_stream.map(|chunk| Ok::<Bytes, std::io::Error>(Bytes::from(chunk)));

    let end_stream = stream::once(async move { Ok::<Bytes, std::io::Error>(Bytes::from(shell_end)) });

    let body_stream = start_stream.chain(content_stream).chain(end_stream);

    let body = axum::body::Body::from_stream(body_stream);

    Response::builder()
        .status(StatusCode::OK)
        .header("content-type", "text/html; charset=utf-8")
        .header("transfer-encoding", "chunked")
        .body(body)
        .unwrap()
}

/// Render a simple HTML string response
pub fn html_response(html: String) -> Response {
    Html(html).into_response()
}

/// Render a JSON response
pub fn json_response<T: serde::Serialize>(data: &T) -> Response {
    match serde_json::to_string(data) {
        Ok(json) => Response::builder()
            .status(StatusCode::OK)
            .header("content-type", "application/json")
            .body(axum::body::Body::from(json))
            .unwrap(),
        Err(err) => Response::builder()
            .status(StatusCode::INTERNAL_SERVER_ERROR)
            .body(axum::body::Body::from(format!("Serialization error: {err}")))
            .unwrap(),
    }
}
