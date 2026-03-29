use crate::component::HeadContext;

/// A layout wraps page content with shared HTML structure.
///
/// Layouts can be nested: root layout → segment layout → page.
pub trait Layout: Send + Sync {
    /// Render the layout with the given children HTML and head context.
    fn render(&self, children: &str, head: &HeadContext) -> String;
}

/// The default root layout providing a full HTML document shell.
pub struct RootLayout {
    pub lang: String,
    pub default_title: String,
    pub stylesheets: Vec<String>,
}

impl Default for RootLayout {
    fn default() -> Self {
        Self {
            lang: "en".to_string(),
            default_title: "Hayabusa App".to_string(),
            stylesheets: vec!["/style.css".to_string()],
        }
    }
}

impl Layout for RootLayout {
    fn render(&self, children: &str, head: &HeadContext) -> String {
        let title = head.title.as_deref().unwrap_or(&self.default_title);

        let stylesheets: String = self
            .stylesheets
            .iter()
            .map(|href| format!("    <link rel=\"stylesheet\" href=\"{href}\" />\n"))
            .collect();

        let head_meta = head.render();

        format!(
            r#"<!DOCTYPE html>
<html lang="{lang}">
<head>
    <meta charset="utf-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1" />
    <title>{title}</title>
{stylesheets}{head_meta}</head>
<body>
{children}
</body>
</html>"#,
            lang = self.lang,
            title = title,
            stylesheets = stylesheets,
            head_meta = head_meta,
            children = children,
        )
    }
}

/// A layout defined by a closure for convenience
pub struct FnLayout<F>
where
    F: Fn(&str, &HeadContext) -> String + Send + Sync,
{
    f: F,
}

impl<F> FnLayout<F>
where
    F: Fn(&str, &HeadContext) -> String + Send + Sync,
{
    pub fn new(f: F) -> Self {
        Self { f }
    }
}

impl<F> Layout for FnLayout<F>
where
    F: Fn(&str, &HeadContext) -> String + Send + Sync,
{
    fn render(&self, children: &str, head: &HeadContext) -> String {
        (self.f)(children, head)
    }
}

/// Compose multiple layouts by nesting them (outer first, inner last).
pub fn compose_layouts(layouts: &[&dyn Layout], content: &str, head: &HeadContext) -> String {
    let mut result = content.to_string();
    for layout in layouts.iter().rev() {
        result = layout.render(&result, head);
    }
    result
}
