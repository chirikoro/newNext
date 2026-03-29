use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};

/// Error type for Hayabusa framework operations
#[derive(Debug)]
pub enum HayabusaError {
    /// Route not found
    NotFound(String),
    /// Internal server error
    Internal(String),
    /// Bad request
    BadRequest(String),
    /// IO error
    Io(std::io::Error),
    /// Configuration error
    Config(String),
}

impl std::fmt::Display for HayabusaError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HayabusaError::NotFound(msg) => write!(f, "Not Found: {msg}"),
            HayabusaError::Internal(msg) => write!(f, "Internal Error: {msg}"),
            HayabusaError::BadRequest(msg) => write!(f, "Bad Request: {msg}"),
            HayabusaError::Io(err) => write!(f, "IO Error: {err}"),
            HayabusaError::Config(msg) => write!(f, "Config Error: {msg}"),
        }
    }
}

impl std::error::Error for HayabusaError {}

impl From<std::io::Error> for HayabusaError {
    fn from(err: std::io::Error) -> Self {
        HayabusaError::Io(err)
    }
}

impl IntoResponse for HayabusaError {
    fn into_response(self) -> Response {
        let (status, body) = match &self {
            HayabusaError::NotFound(msg) => (StatusCode::NOT_FOUND, render_error_page(404, msg)),
            HayabusaError::Internal(msg) => {
                (StatusCode::INTERNAL_SERVER_ERROR, render_error_page(500, msg))
            }
            HayabusaError::BadRequest(msg) => {
                (StatusCode::BAD_REQUEST, render_error_page(400, msg))
            }
            HayabusaError::Io(err) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                render_error_page(500, &err.to_string()),
            ),
            HayabusaError::Config(msg) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                render_error_page(500, msg),
            ),
        };

        (status, Html(body)).into_response()
    }
}

/// Default error page renderer
pub fn render_error_page(code: u16, message: &str) -> String {
    let title = match code {
        404 => "Page Not Found",
        400 => "Bad Request",
        500 => "Internal Server Error",
        _ => "Error",
    };

    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="utf-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1" />
    <title>{code} - {title}</title>
    <style>
        body {{
            font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;
            display: flex;
            justify-content: center;
            align-items: center;
            min-height: 100vh;
            margin: 0;
            background: #fafafa;
            color: #333;
        }}
        .error-container {{
            text-align: center;
            padding: 2rem;
        }}
        .error-code {{
            font-size: 6rem;
            font-weight: 700;
            color: #e0e0e0;
            margin: 0;
            line-height: 1;
        }}
        .error-title {{
            font-size: 1.5rem;
            margin: 1rem 0 0.5rem;
        }}
        .error-message {{
            color: #666;
            font-size: 0.9rem;
        }}
        a {{
            color: #0070f3;
            text-decoration: none;
        }}
        a:hover {{
            text-decoration: underline;
        }}
    </style>
</head>
<body>
    <div class="error-container">
        <p class="error-code">{code}</p>
        <h1 class="error-title">{title}</h1>
        <p class="error-message">{message}</p>
        <p><a href="/">Back to Home</a></p>
    </div>
</body>
</html>"#,
        code = code,
        title = title,
        message = html_escape(message),
    )
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// Custom error page handler type
pub type ErrorPageHandler = Box<dyn Fn(u16, &str) -> String + Send + Sync>;
