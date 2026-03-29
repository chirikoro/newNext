use hayabusa_core::prelude::*;

mod pages;

#[tokio::main]
async fn main() {
    let routes = pages::build_routes();

    HayabusaApp::new()
        .routes(routes)
        .port(3000)
        .with_static_gen("dist")
        .serve()
        .await
        .unwrap();
}
