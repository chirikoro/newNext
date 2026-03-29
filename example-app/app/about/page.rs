// About page (/about) - SSR
use hayabusa_core::prelude::*;

pub async fn render(_req: &PageRequest) -> RenderResult {
    RenderResult::new(html! {
        <main class="container">
            <h1>"About Hayabusa"</h1>
            <p>"A full-stack web framework for Rust, inspired by Next.js."</p>
        </main>
    })
    .with_head(HeadContext::new().title("About - Hayabusa"))
}
