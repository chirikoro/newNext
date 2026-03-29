use proc_macro2::TokenStream;
use quote::quote;
use syn::{ItemFn, Result};

/// Transform a function into a Hayabusa server component.
///
/// The #[component] attribute generates a companion struct with a
/// `render_mode()` method alongside the original function.
///
/// ```ignore
/// #[component]
/// async fn my_page(props: PageProps) -> String {
///     html! { <h1>"Hello"</h1> }
/// }
/// ```
pub fn expand_component(attr: TokenStream, item: ItemFn) -> Result<TokenStream> {
    let fn_name = &item.sig.ident;
    let fn_vis = &item.vis;

    // Generate a struct name from the function name (PascalCase)
    let struct_name = to_pascal_case(&fn_name.to_string());
    let struct_ident = syn::Ident::new(&struct_name, fn_name.span());

    // Determine render mode from attribute arguments
    let render_mode = parse_render_mode(attr);

    // Emit the original function unchanged, plus a companion struct
    Ok(quote! {
        #item

        #fn_vis struct #struct_ident;

        impl #struct_ident {
            pub fn render_mode() -> hayabusa_core::component::RenderMode {
                #render_mode
            }
        }
    })
}

fn to_pascal_case(s: &str) -> String {
    s.split('_')
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                Some(c) => c.to_uppercase().to_string() + &chars.collect::<String>(),
                None => String::new(),
            }
        })
        .collect()
}

fn parse_render_mode(attr: TokenStream) -> TokenStream {
    let attr_str = attr.to_string();
    let attr_str = attr_str.trim();

    if attr_str.is_empty() || attr_str == "ssr" {
        quote! { hayabusa_core::component::RenderMode::Ssr }
    } else if attr_str == "ssg" {
        quote! { hayabusa_core::component::RenderMode::Ssg { revalidate: None } }
    } else if attr_str.starts_with("isr") {
        if let Some(secs) = attr_str
            .strip_prefix("isr(")
            .and_then(|s| s.strip_suffix(')'))
            .and_then(|s| s.trim().parse::<u64>().ok())
        {
            quote! {
                hayabusa_core::component::RenderMode::Ssg {
                    revalidate: Some(::std::time::Duration::from_secs(#secs))
                }
            }
        } else {
            quote! {
                hayabusa_core::component::RenderMode::Ssg {
                    revalidate: Some(::std::time::Duration::from_secs(60))
                }
            }
        }
    } else if attr_str == "streaming" {
        quote! { hayabusa_core::component::RenderMode::Streaming }
    } else {
        quote! { hayabusa_core::component::RenderMode::Ssr }
    }
}
