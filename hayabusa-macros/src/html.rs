use proc_macro2::TokenStream;
use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::{braced, Expr, Ident, LitStr, Result, Token};

/// Represents a parsed HTML node in the macro
#[derive(Debug)]
enum HtmlNode {
    /// An HTML element: <tag attrs...>children</tag>
    Element(HtmlElement),
    /// A text literal: "hello"
    Text(LitStr),
    /// A Rust expression in braces: {variable}
    Expression(Expr),
    /// A fragment: <>children</>
    Fragment(Vec<HtmlNode>),
}

#[derive(Debug)]
struct HtmlElement {
    tag: String,
    attributes: Vec<HtmlAttribute>,
    children: Vec<HtmlNode>,
    self_closing: bool,
}

#[derive(Debug)]
struct HtmlAttribute {
    name: String,
    value: AttrValue,
}

#[derive(Debug)]
enum AttrValue {
    Literal(LitStr),
    Expression(Expr),
}

/// Top-level parser for the html! macro
pub struct HtmlMacroInput {
    nodes: Vec<HtmlNode>,
}

impl Parse for HtmlMacroInput {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut nodes = Vec::new();
        while !input.is_empty() {
            nodes.push(parse_node(input)?);
        }
        Ok(HtmlMacroInput { nodes })
    }
}

fn parse_node(input: ParseStream) -> Result<HtmlNode> {
    if input.peek(Token![<]) {
        parse_element_or_fragment(input)
    } else if input.peek(LitStr) {
        let lit: LitStr = input.parse()?;
        Ok(HtmlNode::Text(lit))
    } else if input.peek(syn::token::Brace) {
        let content;
        braced!(content in input);
        let expr: Expr = content.parse()?;
        Ok(HtmlNode::Expression(expr))
    } else {
        Err(input.error("expected an HTML element, string literal, or {expression}"))
    }
}

fn parse_element_or_fragment(input: ParseStream) -> Result<HtmlNode> {
    input.parse::<Token![<]>()?;

    // Check for fragment: <>...</>
    if input.peek(Token![>]) {
        input.parse::<Token![>]>()?;
        let children = parse_children(input)?;
        // Parse closing </>
        input.parse::<Token![<]>()?;
        input.parse::<Token![/]>()?;
        input.parse::<Token![>]>()?;
        return Ok(HtmlNode::Fragment(children));
    }

    // Parse tag name (supports hyphenated tags like "my-component")
    let tag = parse_tag_name(input)?;

    // Parse attributes
    let attributes = parse_attributes(input)?;

    // Self-closing tag: <br />
    if input.peek(Token![/]) {
        input.parse::<Token![/]>()?;
        input.parse::<Token![>]>()?;
        return Ok(HtmlNode::Element(HtmlElement {
            tag,
            attributes,
            children: Vec::new(),
            self_closing: true,
        }));
    }

    input.parse::<Token![>]>()?;

    // Parse children
    let children = parse_children(input)?;

    // Parse closing tag: </tag>
    input.parse::<Token![<]>()?;
    input.parse::<Token![/]>()?;
    let closing_tag = parse_tag_name(input)?;
    input.parse::<Token![>]>()?;

    if closing_tag != tag {
        return Err(input.error(format!(
            "mismatched closing tag: expected </{tag}>, found </{closing_tag}>"
        )));
    }

    Ok(HtmlNode::Element(HtmlElement {
        tag,
        attributes,
        children,
        self_closing: false,
    }))
}

fn parse_tag_name(input: ParseStream) -> Result<String> {
    let mut name = String::new();
    let ident: Ident = input.parse()?;
    name.push_str(&ident.to_string());

    // Support hyphenated names like "my-component"
    while input.peek(Token![-]) {
        input.parse::<Token![-]>()?;
        name.push('-');
        let part: Ident = input.parse()?;
        name.push_str(&part.to_string());
    }

    Ok(name)
}

fn parse_attributes(input: ParseStream) -> Result<Vec<HtmlAttribute>> {
    let mut attrs = Vec::new();

    // Parse attributes until we hit > or />
    while !input.peek(Token![>]) && !input.peek(Token![/]) {
        let name = parse_attr_name(input)?;
        input.parse::<Token![=]>()?;

        let value = if input.peek(LitStr) {
            AttrValue::Literal(input.parse()?)
        } else if input.peek(syn::token::Brace) {
            let content;
            braced!(content in input);
            AttrValue::Expression(content.parse()?)
        } else {
            return Err(input.error("expected string literal or {expression} for attribute value"));
        };

        attrs.push(HtmlAttribute { name, value });
    }

    Ok(attrs)
}

fn parse_attr_name(input: ParseStream) -> Result<String> {
    let mut name = String::new();
    let ident: Ident = input.parse()?;
    name.push_str(&ident.to_string());

    // Support hyphenated attribute names
    while input.peek(Token![-]) {
        input.parse::<Token![-]>()?;
        name.push('-');
        let part: Ident = input.parse()?;
        name.push_str(&part.to_string());
    }

    Ok(name)
}

fn parse_children(input: ParseStream) -> Result<Vec<HtmlNode>> {
    let mut children = Vec::new();

    // Parse children until we see a closing tag </
    while !is_closing_tag(input) && !input.is_empty() {
        children.push(parse_node(input)?);
    }

    Ok(children)
}

fn is_closing_tag(input: ParseStream) -> bool {
    let fork = input.fork();
    if fork.parse::<Token![<]>().is_ok() {
        fork.peek(Token![/])
    } else {
        false
    }
}

/// Generate the output TokenStream from parsed HTML nodes
pub fn generate(input: HtmlMacroInput) -> TokenStream {
    let parts: Vec<TokenStream> = input.nodes.iter().map(generate_node).collect();

    if parts.len() == 1 {
        let part = &parts[0];
        quote! { { #part } }
    } else {
        quote! {
            {
                let mut __hayabusa_html = String::new();
                #( __hayabusa_html.push_str(&(#parts)); )*
                __hayabusa_html
            }
        }
    }
}

fn generate_node(node: &HtmlNode) -> TokenStream {
    match node {
        HtmlNode::Element(el) => generate_element(el),
        HtmlNode::Text(lit) => {
            quote! { #lit.to_string() }
        }
        HtmlNode::Expression(expr) => {
            quote! { ::std::string::ToString::to_string(&(#expr)) }
        }
        HtmlNode::Fragment(children) => {
            let parts: Vec<TokenStream> = children.iter().map(generate_node).collect();
            quote! {
                {
                    let mut __hayabusa_frag = String::new();
                    #( __hayabusa_frag.push_str(&(#parts)); )*
                    __hayabusa_frag
                }
            }
        }
    }
}

fn generate_element(el: &HtmlElement) -> TokenStream {
    let tag = &el.tag;

    let attr_parts: Vec<TokenStream> = el.attributes.iter().map(|attr| {
        let name = &attr.name;
        match &attr.value {
            AttrValue::Literal(lit) => {
                quote! {
                    __hayabusa_el.push_str(&format!(" {}=\"{}\"", #name, #lit));
                }
            }
            AttrValue::Expression(expr) => {
                quote! {
                    __hayabusa_el.push_str(&format!(" {}=\"{}\"", #name, #expr));
                }
            }
        }
    }).collect();

    if el.self_closing {
        quote! {
            {
                let mut __hayabusa_el = format!("<{}", #tag);
                #( #attr_parts )*
                __hayabusa_el.push_str(" />");
                __hayabusa_el
            }
        }
    } else {
        let child_parts: Vec<TokenStream> = el.children.iter().map(generate_node).collect();
        quote! {
            {
                let mut __hayabusa_el = format!("<{}", #tag);
                #( #attr_parts )*
                __hayabusa_el.push('>');
                #( __hayabusa_el.push_str(&(#child_parts)); )*
                __hayabusa_el.push_str(&format!("</{}>", #tag));
                __hayabusa_el
            }
        }
    }
}
