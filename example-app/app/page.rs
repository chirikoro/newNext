// Home page (/) - SSR
use hayabusa_core::prelude::*;

pub async fn render(_req: &PageRequest) -> RenderResult {
    RenderResult::new(html! {
        <main class="container">
            <section class="hero">
                <h1>"Hayabusa 隼"</h1>
                <p class="subtitle">"A blazing-fast full-stack web framework for Rust"</p>
            </section>
        </main>
    })
    .with_head(HeadContext::new().title("Home - Hayabusa"))
}
