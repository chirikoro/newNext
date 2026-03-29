// Blog list page (/blog) - SSG
use hayabusa_core::prelude::*;

pub async fn render(_req: &PageRequest) -> RenderResult {
    RenderResult::new(html! {
        <main class="container">
            <h1>"Blog"</h1>
            <p>"Articles about Hayabusa and Rust web development."</p>
        </main>
    })
    .with_head(HeadContext::new().title("Blog - Hayabusa"))
}
