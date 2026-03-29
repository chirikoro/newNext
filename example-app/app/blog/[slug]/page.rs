// Blog post page (/blog/:slug) - ISR with 60s revalidation
use hayabusa_core::prelude::*;

pub async fn render(req: &PageRequest) -> RenderResult {
    let slug = req.param("slug").unwrap_or("unknown");

    RenderResult::new(html! {
        <main class="container">
            <article>
                <h1>{format!("Blog Post: {}", slug)}</h1>
                <p>"This is a dynamically rendered blog post."</p>
                <a href="/blog">"Back to Blog"</a>
            </article>
        </main>
    })
    .with_head(HeadContext::new().title(format!("{} - Blog", slug)))
}
