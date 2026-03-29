// Posts API endpoint (/api/posts)
// Returns a list of blog posts as JSON

use serde_json::json;

pub async fn handler() -> serde_json::Value {
    json!([
        { "slug": "hello-world", "title": "Hello World" },
        { "slug": "ssr-vs-ssg", "title": "SSR vs SSG" },
        { "slug": "rust-performance", "title": "Rust Performance" }
    ])
}
