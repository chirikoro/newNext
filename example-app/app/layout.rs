// Root layout for the example app
// This file follows the file-based routing convention.
// Layouts wrap all pages within their directory and subdirectories.

use hayabusa_core::prelude::*;

pub fn render(children: &str, head: &HeadContext) -> String {
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
    </nav>
    {children}
</body>
</html>"#,
    )
}
