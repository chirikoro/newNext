use std::fs;
use std::path::Path;

/// Scaffold a new Hayabusa project
pub async fn run(name: &str) {
    let project_dir = Path::new(name);

    if project_dir.exists() {
        tracing::error!("Directory '{}' already exists", name);
        return;
    }

    tracing::info!("Creating new Hayabusa project: {}", name);

    // Create directory structure
    let dirs = [
        "",
        "src",
        "app",
        "app/api",
        "public",
    ];

    for dir in &dirs {
        let path = project_dir.join(dir);
        fs::create_dir_all(&path).expect("Failed to create directory");
    }

    // Cargo.toml
    let cargo_toml = format!(
        r#"[package]
name = "{name}"
version = "0.1.0"
edition = "2021"

[dependencies]
hayabusa-core = {{ git = "https://github.com/chirikoro/newnext" }}
hayabusa-macros = {{ git = "https://github.com/chirikoro/newnext" }}
tokio = {{ version = "1", features = ["full"] }}
serde = {{ version = "1", features = ["derive"] }}
serde_json = "1"
async-trait = "0.1"
"#
    );
    fs::write(project_dir.join("Cargo.toml"), cargo_toml).expect("Failed to write Cargo.toml");

    // src/main.rs
    let main_rs = r#"use hayabusa_core::prelude::*;

mod pages;

#[tokio::main]
async fn main() {
    let routes = pages::build_routes();

    HayabusaApp::new()
        .routes(routes)
        .port(3000)
        .serve()
        .await
        .unwrap();
}
"#;
    fs::write(project_dir.join("src/main.rs"), main_rs).expect("Failed to write main.rs");

    // src/pages.rs (route registration)
    let pages_rs = r#"use hayabusa_core::prelude::*;

pub fn build_routes() -> RouteTable {
    RouteTable::new()
        .page("/", RenderMode::Ssr, Box::new(|req| {
            Box::pin(async move {
                RenderResult::new(html! {
                    <div class="container">
                        <h1>"Welcome to Hayabusa"</h1>
                        <p>"Edit app/page.rs to get started"</p>
                    </div>
                })
                .with_head(HeadContext::new().title("Home - My App"))
            })
        }))
}
"#;
    fs::write(project_dir.join("src/pages.rs"), pages_rs).expect("Failed to write pages.rs");

    // app/page.rs (placeholder)
    let page_rs = r#"// Home page component
// This file follows the file-based routing convention.
// Edit this file and restart the dev server to see changes.

use hayabusa_core::prelude::*;

pub async fn render(req: &PageRequest) -> RenderResult {
    RenderResult::new(html! {
        <div class="container">
            <h1>"Welcome to Hayabusa 隼"</h1>
            <p>"A blazing-fast full-stack web framework for Rust"</p>
        </div>
    })
    .with_head(HeadContext::new().title("Home"))
}
"#;
    fs::write(project_dir.join("app/page.rs"), page_rs).expect("Failed to write page.rs");

    // public/style.css
    let style_css = r#"* {
    margin: 0;
    padding: 0;
    box-sizing: border-box;
}

body {
    font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;
    line-height: 1.6;
    color: #333;
    background: #fafafa;
}

.container {
    max-width: 800px;
    margin: 0 auto;
    padding: 2rem;
}

h1 {
    font-size: 2.5rem;
    margin-bottom: 1rem;
}

p {
    font-size: 1.1rem;
    color: #666;
}

a {
    color: #0070f3;
    text-decoration: none;
}

a:hover {
    text-decoration: underline;
}

nav {
    background: #fff;
    border-bottom: 1px solid #eaeaea;
    padding: 1rem 2rem;
}

nav a {
    margin-right: 1.5rem;
    font-weight: 500;
}
"#;
    fs::write(project_dir.join("public/style.css"), style_css).expect("Failed to write style.css");

    // .gitignore
    let gitignore = r#"/target
/dist
*.swp
*.swo
.DS_Store
"#;
    fs::write(project_dir.join(".gitignore"), gitignore).expect("Failed to write .gitignore");

    tracing::info!("✅ Project '{}' created successfully!", name);
    tracing::info!("   cd {}", name);
    tracing::info!("   cargo run");
}
