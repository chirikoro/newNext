//! # Hayabusa Macros (隼)
//!
//! Procedural macros for the Hayabusa full-stack web framework.
//!
//! - `html!` - JSX-like HTML templating with compile-time generation
//! - `#[component]` - Server component attribute

mod component;
mod html;

use proc_macro::TokenStream;
use syn::parse_macro_input;

/// JSX-like HTML macro that generates HTML strings at compile time.
///
/// # Example
/// ```ignore
/// use hayabusa_macros::html;
///
/// let title = "Hello";
/// let output = html! {
///     <div class="container">
///         <h1>{title}</h1>
///         <p>"Welcome to Hayabusa"</p>
///     </div>
/// };
/// ```
///
/// ## Supported syntax:
/// - Elements: `<div>...</div>`
/// - Self-closing: `<br />`
/// - Attributes: `<div class="foo" id={my_id}>...</div>`
/// - Text: `"literal text"`
/// - Expressions: `{rust_expression}`
/// - Fragments: `<>.....</>`
#[proc_macro]
pub fn html(input: TokenStream) -> TokenStream {
    let parsed = parse_macro_input!(input as html::HtmlMacroInput);
    html::generate(parsed).into()
}

/// Mark a function as a Hayabusa server component.
///
/// # Example
/// ```ignore
/// use hayabusa_macros::component;
///
/// #[component]
/// async fn home_page() -> String {
///     html! { <h1>"Home"</h1> }
/// }
///
/// // With render mode:
/// #[component(ssg)]
/// async fn about_page() -> String { ... }
///
/// #[component(isr(60))]
/// async fn blog_page() -> String { ... }
/// ```
///
/// Render modes: `ssr` (default), `ssg`, `isr(seconds)`, `streaming`
#[proc_macro_attribute]
pub fn component(attr: TokenStream, item: TokenStream) -> TokenStream {
    let item_fn = parse_macro_input!(item as syn::ItemFn);
    match component::expand_component(attr.into(), item_fn) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.to_compile_error().into(),
    }
}
