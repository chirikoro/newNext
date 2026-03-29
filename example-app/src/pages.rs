use hayabusa_core::prelude::*;
use std::time::Duration;

pub fn build_routes() -> RouteTable {
    RouteTable::new()
        // Root layout
        .layout("/", Box::new(RootAppLayout))
        // Home page (SSR)
        .page(
            "/",
            RenderMode::Ssr,
            Box::new(|_req| {
                Box::pin(async move {
                    RenderResult::new(html! {
                        <main class="container">
                            <section class="hero">
                                <h1>"Hayabusa 隼"</h1>
                                <p class="subtitle">"A blazing-fast full-stack web framework for Rust"</p>
                                <div class="features">
                                    <div class="feature">
                                        <h3>"⚡ SSR"</h3>
                                        <p>"Server-Side Rendering for dynamic pages"</p>
                                    </div>
                                    <div class="feature">
                                        <h3>"📦 SSG"</h3>
                                        <p>"Static Site Generation for maximum speed"</p>
                                    </div>
                                    <div class="feature">
                                        <h3>"🔄 ISR"</h3>
                                        <p>"Incremental Static Regeneration for the best of both"</p>
                                    </div>
                                </div>
                            </section>
                        </main>
                    })
                    .with_head(
                        HeadContext::new()
                            .title("Hayabusa - Rust Full-Stack Framework")
                            .description("A high-performance full-stack web framework for Rust")
                            .og_title("Hayabusa 隼")
                            .og_description("Build blazing-fast web apps with Rust"),
                    )
                })
            }),
        )
        // About page (SSR)
        .page(
            "/about",
            RenderMode::Ssr,
            Box::new(|_req| {
                Box::pin(async move {
                    RenderResult::new(html! {
                        <main class="container">
                            <h1>"About Hayabusa"</h1>
                            <p>"Hayabusa (隼, meaning 'peregrine falcon') is a full-stack web framework for Rust."</p>
                            <h2>"Why Hayabusa?"</h2>
                            <ul>
                                <li>"Rust-powered performance: faster than Node.js-based frameworks"</li>
                                <li>"Next.js-inspired DX: file-based routing, layouts, SSR/SSG/ISR"</li>
                                <li>"Type-safe HTML: compile-time HTML generation with the html! macro"</li>
                                <li>"Zero-cost abstractions: no runtime overhead for templates"</li>
                            </ul>
                        </main>
                    })
                    .with_head(
                        HeadContext::new()
                            .title("About - Hayabusa")
                            .description("Learn about the Hayabusa web framework"),
                    )
                })
            }),
        )
        // Blog list page (SSG - static generation)
        .page(
            "/blog",
            RenderMode::Ssg { revalidate: None },
            Box::new(|_req| {
                Box::pin(async move {
                    let posts = vec![
                        ("hello-world", "Hello World", "Getting started with Hayabusa"),
                        ("ssr-vs-ssg", "SSR vs SSG", "Understanding rendering strategies"),
                        ("rust-performance", "Rust Performance", "Why Rust is fast"),
                    ];

                    let posts_html: String = posts
                        .iter()
                        .map(|(slug, title, desc)| {
                            html! {
                                <article class="post-card">
                                    <h2>
                                        <a href={format!("/blog/{}", slug)}>{*title}</a>
                                    </h2>
                                    <p>{*desc}</p>
                                </article>
                            }
                        })
                        .collect();

                    RenderResult::new(html! {
                        <main class="container">
                            <h1>"Blog"</h1>
                            {posts_html}
                        </main>
                    })
                    .with_head(
                        HeadContext::new()
                            .title("Blog - Hayabusa")
                            .description("Articles about Hayabusa and Rust web development"),
                    )
                })
            }),
        )
        // Blog post page (ISR - revalidate every 60 seconds)
        .page(
            "/blog/:slug",
            RenderMode::Ssg {
                revalidate: Some(Duration::from_secs(60)),
            },
            Box::new(|req| {
                Box::pin(async move {
                    let slug = req.param("slug").unwrap_or("unknown");
                    let (title, content) = match slug {
                        "hello-world" => (
                            "Hello World",
                            "Welcome to Hayabusa! This framework brings Next.js-like developer experience to Rust.",
                        ),
                        "ssr-vs-ssg" => (
                            "SSR vs SSG",
                            "SSR renders pages on every request. SSG pre-renders at build time. ISR gives you both!",
                        ),
                        "rust-performance" => (
                            "Rust Performance",
                            "Rust's zero-cost abstractions and ownership model make it ideal for high-performance web servers.",
                        ),
                        _ => ("Not Found", "This blog post does not exist."),
                    };

                    RenderResult::new(html! {
                        <main class="container">
                            <article>
                                <h1>{title}</h1>
                                <p>{content}</p>
                                <a href="/blog">"← Back to Blog"</a>
                            </article>
                        </main>
                    })
                    .with_head(
                        HeadContext::new()
                            .title(format!("{} - Hayabusa Blog", title))
                            .description(content)
                            .og_title(title),
                    )
                })
            }),
        )
        // API: Health check
        .api(
            ApiMethod::Get,
            "/api/health",
            Box::new(|_req| {
                Box::pin(async move {
                    let body = serde_json::json!({
                        "status": "ok",
                        "framework": "hayabusa",
                        "version": "0.1.0"
                    });
                    hayabusa_core::render::json_response(&body)
                })
            }),
        )
        // API: Get posts
        .api(
            ApiMethod::Get,
            "/api/posts",
            Box::new(|_req| {
                Box::pin(async move {
                    let posts = serde_json::json!([
                        { "slug": "hello-world", "title": "Hello World" },
                        { "slug": "ssr-vs-ssg", "title": "SSR vs SSG" },
                        { "slug": "rust-performance", "title": "Rust Performance" },
                    ]);
                    hayabusa_core::render::json_response(&posts)
                })
            }),
        )
}

/// Custom root layout with navigation
struct RootAppLayout;

impl Layout for RootAppLayout {
    fn render(&self, children: &str, head: &HeadContext) -> String {
        let title = head.title.as_deref().unwrap_or("Hayabusa App");
        let head_meta = head.render();

        format!(
            r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="utf-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1" />
    <title>{title}</title>
    <link rel="stylesheet" href="/style.css" />
{head_meta}</head>
<body>
    <nav>
        <a href="/"><strong>隼 Hayabusa</strong></a>
        <a href="/about">About</a>
        <a href="/blog">Blog</a>
        <a href="/api/health">API</a>
    </nav>
    {children}
</body>
</html>"#,
        )
    }
}
