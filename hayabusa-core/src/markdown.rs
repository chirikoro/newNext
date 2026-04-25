//! Markdown Pages for Hayabusa.
//!
//! Write pages in Markdown with YAML frontmatter — no Rust needed.
//! Perfect for blogs, docs, landing pages.
//!
//! ## Usage
//! Create `app/blog/hello.md`:
//! ```markdown
//! ---
//! title: Hello World
//! description: My first post
//! date: 2024-01-15
//! tags: [rust, web]
//! template: blog-post
//! ---
//!
//! # Hello World
//!
//! This is my **first** blog post written in *Markdown*.
//!
//! ```rust
//! fn main() {
//!     println!("Hello from Hayabusa!");
//! }
//! ```
//! ```

use std::collections::HashMap;

/// A parsed Markdown page with frontmatter
#[derive(Debug, Clone)]
pub struct MarkdownPage {
    /// Frontmatter key-value pairs
    pub frontmatter: HashMap<String, String>,
    /// Raw markdown content (after frontmatter)
    pub markdown: String,
    /// Rendered HTML
    pub html: String,
}

impl MarkdownPage {
    /// Parse a markdown file with optional YAML frontmatter
    pub fn parse(source: &str) -> Self {
        let (frontmatter, markdown) = extract_frontmatter(source);
        let html = markdown_to_html(&markdown);

        Self {
            frontmatter,
            markdown,
            html,
        }
    }

    /// Get a frontmatter value
    pub fn meta(&self, key: &str) -> Option<&str> {
        self.frontmatter.get(key).map(|s| s.as_str())
    }

    /// Get title from frontmatter
    pub fn title(&self) -> Option<&str> {
        self.meta("title")
    }

    /// Get description from frontmatter
    pub fn description(&self) -> Option<&str> {
        self.meta("description")
    }

    /// Render the page wrapped in a layout
    pub fn render_with_layout(&self, layout_fn: impl Fn(&str, &HashMap<String, String>) -> String) -> String {
        layout_fn(&self.html, &self.frontmatter)
    }
}

/// Extract YAML frontmatter from a markdown document
fn extract_frontmatter(source: &str) -> (HashMap<String, String>, String) {
    let trimmed = source.trim_start();

    if !trimmed.starts_with("---") {
        return (HashMap::new(), source.to_string());
    }

    let after_start = &trimmed[3..];
    if let Some(end_pos) = after_start.find("\n---") {
        let yaml_content = &after_start[..end_pos];
        let markdown = &after_start[end_pos + 4..];

        let frontmatter = parse_simple_yaml(yaml_content);
        (frontmatter, markdown.trim_start_matches('\n').to_string())
    } else {
        (HashMap::new(), source.to_string())
    }
}

/// Simple YAML parser (handles key: value pairs, one level deep)
fn parse_simple_yaml(yaml: &str) -> HashMap<String, String> {
    let mut map = HashMap::new();

    for line in yaml.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        if let Some(colon_pos) = line.find(':') {
            let key = line[..colon_pos].trim().to_string();
            let value = line[colon_pos + 1..].trim().to_string();
            // Remove quotes if present
            let value = value
                .trim_matches('"')
                .trim_matches('\'')
                .to_string();
            map.insert(key, value);
        }
    }

    map
}

/// Convert Markdown to HTML.
///
/// Supports:
/// - Headings (# to ######)
/// - Paragraphs
/// - **bold**, *italic*, ~~strikethrough~~, `code`
/// - Code blocks (``` with optional language)
/// - Unordered lists (- or *)
/// - Ordered lists (1.)
/// - Links [text](url)
/// - Images ![alt](url)
/// - Horizontal rules (---, ***, ___)
/// - Blockquotes (>)
/// - Tables (GFM-style with alignment)
pub fn markdown_to_html(md: &str) -> String {
    let mut html = String::new();
    let mut in_code_block = false;
    let mut code_lang = String::new();
    let mut code_content = String::new();
    let mut in_list = false;
    let mut list_type = "ul"; // "ul" or "ol"
    let mut in_paragraph = false;
    let mut in_blockquote = false;
    let mut in_table = false;
    let mut table_alignments: Vec<Alignment> = Vec::new();

    let lines: Vec<&str> = md.lines().collect();
    let mut i = 0;

    while i < lines.len() {
        let line = lines[i];

        // Code blocks
        if line.trim().starts_with("```") {
            if in_code_block {
                html.push_str(&format!(
                    "<pre><code class=\"language-{}\">{}</code></pre>\n",
                    code_lang,
                    html_escape_code(&code_content)
                ));
                code_content.clear();
                code_lang.clear();
                in_code_block = false;
            } else {
                close_open_blocks(&mut html, &mut in_paragraph, &mut in_list, list_type, &mut in_blockquote);
                if in_table {
                    html.push_str("</tbody></table>\n");
                    in_table = false;
                }
                code_lang = line.trim().trim_start_matches('`').to_string();
                in_code_block = true;
            }
            i += 1;
            continue;
        }

        if in_code_block {
            if !code_content.is_empty() {
                code_content.push('\n');
            }
            code_content.push_str(line);
            i += 1;
            continue;
        }

        let trimmed = line.trim();

        // Table detection: current line looks like a table row and next line
        // is a separator row
        if !in_table && is_table_row(trimmed) && i + 1 < lines.len() && is_separator_row(lines[i + 1].trim()) {
            close_open_blocks(&mut html, &mut in_paragraph, &mut in_list, list_type, &mut in_blockquote);

            // Parse header and separator
            let headers = parse_table_cells(trimmed);
            table_alignments = parse_alignments(lines[i + 1].trim());

            html.push_str("<table>\n<thead>\n<tr>\n");
            for (j, header) in headers.iter().enumerate() {
                let align = table_alignments.get(j).copied().unwrap_or(Alignment::None);
                let attr = align.attr();
                html.push_str(&format!("<th{}>{}</th>\n", attr, inline_markdown(header.trim())));
            }
            html.push_str("</tr>\n</thead>\n<tbody>\n");
            in_table = true;
            i += 2; // skip header + separator
            continue;
        }

        // Table data row
        if in_table {
            if is_table_row(trimmed) {
                let cells = parse_table_cells(trimmed);
                html.push_str("<tr>\n");
                for (j, cell) in cells.iter().enumerate() {
                    let align = table_alignments.get(j).copied().unwrap_or(Alignment::None);
                    let attr = align.attr();
                    html.push_str(&format!("<td{}>{}</td>\n", attr, inline_markdown(cell.trim())));
                }
                html.push_str("</tr>\n");
                i += 1;
                continue;
            } else {
                // End of table
                html.push_str("</tbody>\n</table>\n");
                in_table = false;
                table_alignments.clear();
                // fall through to process this line normally
            }
        }

        // Empty line
        if trimmed.is_empty() {
            if in_paragraph {
                html.push_str("</p>\n");
                in_paragraph = false;
            }
            if in_blockquote {
                html.push_str("</blockquote>\n");
                in_blockquote = false;
            }
            i += 1;
            continue;
        }

        // Horizontal rule
        if trimmed == "---" || trimmed == "***" || trimmed == "___" {
            close_open_blocks(&mut html, &mut in_paragraph, &mut in_list, list_type, &mut in_blockquote);
            html.push_str("<hr />\n");
            i += 1;
            continue;
        }

        // Headings
        if trimmed.starts_with('#') {
            close_open_blocks(&mut html, &mut in_paragraph, &mut in_list, list_type, &mut in_blockquote);
            let level = trimmed.chars().take_while(|c| *c == '#').count().min(6);
            let content = trimmed[level..].trim().trim_end_matches('#').trim();
            let id = slugify(content);
            html.push_str(&format!(
                "<h{level} id=\"{id}\">{content}</h{level}>\n",
                level = level,
                id = id,
                content = inline_markdown(content)
            ));
            i += 1;
            continue;
        }

        // Blockquote
        if trimmed.starts_with('>') {
            if in_paragraph {
                html.push_str("</p>\n");
                in_paragraph = false;
            }
            if !in_blockquote {
                html.push_str("<blockquote>\n");
                in_blockquote = true;
            }
            let content = trimmed[1..].trim();
            html.push_str(&format!("<p>{}</p>\n", inline_markdown(content)));
            i += 1;
            continue;
        }

        // Unordered list
        if trimmed.starts_with("- ") || trimmed.starts_with("* ") {
            if in_paragraph {
                html.push_str("</p>\n");
                in_paragraph = false;
            }
            if !in_list {
                html.push_str("<ul>\n");
                in_list = true;
                list_type = "ul";
            }
            let content = trimmed[2..].trim();
            html.push_str(&format!("<li>{}</li>\n", inline_markdown(content)));
            i += 1;
            continue;
        }

        // Ordered list
        if trimmed.len() > 2 && trimmed.chars().next().unwrap().is_ascii_digit() {
            if let Some(dot_pos) = trimmed.find(". ") {
                if trimmed[..dot_pos].chars().all(|c| c.is_ascii_digit()) {
                    if in_paragraph {
                        html.push_str("</p>\n");
                        in_paragraph = false;
                    }
                    if !in_list {
                        html.push_str("<ol>\n");
                        in_list = true;
                        list_type = "ol";
                    }
                    let content = trimmed[dot_pos + 2..].trim();
                    html.push_str(&format!("<li>{}</li>\n", inline_markdown(content)));
                    i += 1;
                    continue;
                }
            }
        }

        // Close list if we're no longer in list items
        if in_list {
            html.push_str(&format!("</{}>\n", list_type));
            in_list = false;
        }

        // Paragraph
        if !in_paragraph {
            html.push_str("<p>");
            in_paragraph = true;
        } else {
            html.push('\n');
        }
        html.push_str(&inline_markdown(trimmed));

        i += 1;
    }

    // Close any open blocks
    if in_paragraph {
        html.push_str("</p>\n");
    }
    if in_list {
        html.push_str(&format!("</{}>\n", list_type));
    }
    if in_blockquote {
        html.push_str("</blockquote>\n");
    }
    if in_table {
        html.push_str("</tbody>\n</table>\n");
    }

    html
}

// ─── Table Helpers ──────────────────────────────────────────

/// Column alignment parsed from the separator row
#[derive(Debug, Clone, Copy, PartialEq)]
enum Alignment {
    None,
    Left,
    Center,
    Right,
}

impl Alignment {
    fn attr(&self) -> &'static str {
        match self {
            Self::None => "",
            Self::Left => " style=\"text-align:left\"",
            Self::Center => " style=\"text-align:center\"",
            Self::Right => " style=\"text-align:right\"",
        }
    }
}

/// Check if a line looks like a table row (`| ... | ... |`)
fn is_table_row(line: &str) -> bool {
    let trimmed = line.trim();
    trimmed.starts_with('|') && trimmed.ends_with('|') && trimmed.matches('|').count() >= 2
}

/// Check if a line is a table separator row (`| --- | --- |`)
fn is_separator_row(line: &str) -> bool {
    let trimmed = line.trim();
    if !trimmed.starts_with('|') || !trimmed.ends_with('|') {
        return false;
    }
    let inner = &trimmed[1..trimmed.len() - 1];
    inner.split('|').all(|cell| {
        let c = cell.trim();
        !c.is_empty()
            && c.chars().all(|ch| ch == '-' || ch == ':' || ch == ' ')
            && c.contains('-')
    })
}

/// Parse cells from a table row
fn parse_table_cells(line: &str) -> Vec<String> {
    let trimmed = line.trim();
    // Strip leading/trailing pipes and split on |
    let inner = if trimmed.starts_with('|') && trimmed.ends_with('|') {
        &trimmed[1..trimmed.len() - 1]
    } else {
        trimmed
    };
    inner.split('|').map(|c| c.to_string()).collect()
}

/// Parse alignment from separator row
fn parse_alignments(line: &str) -> Vec<Alignment> {
    let trimmed = line.trim();
    let inner = if trimmed.starts_with('|') && trimmed.ends_with('|') {
        &trimmed[1..trimmed.len() - 1]
    } else {
        trimmed
    };
    inner.split('|').map(|cell| {
        let c = cell.trim();
        let left = c.starts_with(':');
        let right = c.ends_with(':');
        match (left, right) {
            (true, true) => Alignment::Center,
            (true, false) => Alignment::Left,
            (false, true) => Alignment::Right,
            (false, false) => Alignment::None,
        }
    }).collect()
}

fn close_open_blocks(
    html: &mut String,
    in_paragraph: &mut bool,
    in_list: &mut bool,
    list_type: &str,
    in_blockquote: &mut bool,
) {
    if *in_paragraph {
        html.push_str("</p>\n");
        *in_paragraph = false;
    }
    if *in_list {
        html.push_str(&format!("</{}>\n", list_type));
        *in_list = false;
    }
    if *in_blockquote {
        html.push_str("</blockquote>\n");
        *in_blockquote = false;
    }
}

/// Process inline markdown: **bold**, *italic*, `code`, [links], ![images]
fn inline_markdown(text: &str) -> String {
    let mut result = text.to_string();

    // Images ![alt](url) — must be before links
    while let Some(start) = result.find("![") {
        if let Some(alt_end) = result[start + 2..].find("](") {
            let alt_end = start + 2 + alt_end;
            if let Some(url_end) = result[alt_end + 2..].find(')') {
                let alt = &result[start + 2..alt_end];
                let url = &result[alt_end + 2..alt_end + 2 + url_end];
                let img = format!("<img src=\"{}\" alt=\"{}\" loading=\"lazy\" />", url, alt);
                result = format!("{}{}{}", &result[..start], img, &result[alt_end + 2 + url_end + 1..]);
                continue;
            }
        }
        break;
    }

    // Links [text](url)
    while let Some(start) = result.find('[') {
        // Skip if it's preceded by ! (image)
        if start > 0 && result.as_bytes()[start - 1] == b'!' {
            break;
        }
        if let Some(text_end) = result[start + 1..].find("](") {
            let text_end = start + 1 + text_end;
            if let Some(url_end) = result[text_end + 2..].find(')') {
                let link_text = &result[start + 1..text_end];
                let url = &result[text_end + 2..text_end + 2 + url_end];
                let link = format!("<a href=\"{}\">{}</a>", url, link_text);
                result = format!("{}{}{}", &result[..start], link, &result[text_end + 2 + url_end + 1..]);
                continue;
            }
        }
        break;
    }

    // Inline code `code`
    while let Some(start) = result.find('`') {
        if let Some(end) = result[start + 1..].find('`') {
            let code = &result[start + 1..start + 1 + end];
            let replacement = format!("<code>{}</code>", html_escape_code(code));
            result = format!("{}{}{}", &result[..start], replacement, &result[start + 1 + end + 1..]);
        } else {
            break;
        }
    }

    // Strikethrough ~~text~~
    while let Some(start) = result.find("~~") {
        if let Some(end) = result[start + 2..].find("~~") {
            let struck = &result[start + 2..start + 2 + end];
            result = format!(
                "{}<del>{}</del>{}",
                &result[..start],
                struck,
                &result[start + 2 + end + 2..]
            );
        } else {
            break;
        }
    }

    // Bold **text**
    while let Some(start) = result.find("**") {
        if let Some(end) = result[start + 2..].find("**") {
            let bold_text = &result[start + 2..start + 2 + end];
            result = format!(
                "{}<strong>{}</strong>{}",
                &result[..start],
                bold_text,
                &result[start + 2 + end + 2..]
            );
        } else {
            break;
        }
    }

    // Italic *text*
    while let Some(start) = result.find('*') {
        if let Some(end) = result[start + 1..].find('*') {
            let italic_text = &result[start + 1..start + 1 + end];
            result = format!(
                "{}<em>{}</em>{}",
                &result[..start],
                italic_text,
                &result[start + 1 + end + 1..]
            );
        } else {
            break;
        }
    }

    result
}

/// HTML-escape for code blocks
fn html_escape_code(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

/// Convert a heading to a URL-safe slug
fn slugify(text: &str) -> String {
    text.to_lowercase()
        .chars()
        .map(|c| {
            if c.is_alphanumeric() {
                c
            } else if c == ' ' || c == '-' {
                '-'
            } else {
                '_'
            }
        })
        .collect::<String>()
        .replace("--", "-")
        .trim_matches('-')
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_frontmatter_parsing() {
        let source = "---\ntitle: Hello World\ndate: 2024-01-15\ntags: [rust, web]\n---\n\n# Content";
        let page = MarkdownPage::parse(source);
        assert_eq!(page.title(), Some("Hello World"));
        assert_eq!(page.meta("date"), Some("2024-01-15"));
        assert!(page.html.contains("<h1"));
    }

    #[test]
    fn test_no_frontmatter() {
        let source = "# Just a heading\n\nSome content.";
        let page = MarkdownPage::parse(source);
        assert!(page.frontmatter.is_empty());
        assert!(page.html.contains("<h1"));
    }

    #[test]
    fn test_headings() {
        assert!(markdown_to_html("# H1").contains("<h1"));
        assert!(markdown_to_html("## H2").contains("<h2"));
        assert!(markdown_to_html("### H3").contains("<h3"));
    }

    #[test]
    fn test_heading_ids() {
        let html = markdown_to_html("# Hello World");
        assert!(html.contains("id=\"hello-world\""));
    }

    #[test]
    fn test_paragraphs() {
        let html = markdown_to_html("First paragraph.\n\nSecond paragraph.");
        assert!(html.contains("<p>First paragraph.</p>"));
        assert!(html.contains("<p>Second paragraph.</p>"));
    }

    #[test]
    fn test_bold_and_italic() {
        let html = markdown_to_html("This is **bold** and *italic*.");
        assert!(html.contains("<strong>bold</strong>"));
        assert!(html.contains("<em>italic</em>"));
    }

    #[test]
    fn test_inline_code() {
        let html = markdown_to_html("Use `println!` in Rust.");
        assert!(html.contains("<code>println!</code>"));
    }

    #[test]
    fn test_code_block() {
        let md = "```rust\nfn main() {\n    println!(\"hello\");\n}\n```";
        let html = markdown_to_html(md);
        assert!(html.contains("<pre><code class=\"language-rust\">"));
        assert!(html.contains("fn main()"));
    }

    #[test]
    fn test_unordered_list() {
        let md = "- Apple\n- Banana\n- Cherry";
        let html = markdown_to_html(md);
        assert!(html.contains("<ul>"));
        assert!(html.contains("<li>Apple</li>"));
        assert!(html.contains("<li>Banana</li>"));
    }

    #[test]
    fn test_ordered_list() {
        let md = "1. First\n2. Second\n3. Third";
        let html = markdown_to_html(md);
        assert!(html.contains("<ol>"));
        assert!(html.contains("<li>First</li>"));
    }

    #[test]
    fn test_links() {
        let html = markdown_to_html("Visit [Rust](https://rust-lang.org).");
        assert!(html.contains("<a href=\"https://rust-lang.org\">Rust</a>"));
    }

    #[test]
    fn test_images() {
        let html = markdown_to_html("![Logo](/images/logo.png)");
        assert!(html.contains("<img src=\"/images/logo.png\""));
        assert!(html.contains("alt=\"Logo\""));
        assert!(html.contains("loading=\"lazy\""));
    }

    #[test]
    fn test_blockquote() {
        let html = markdown_to_html("> This is a quote");
        assert!(html.contains("<blockquote>"));
    }

    #[test]
    fn test_horizontal_rule() {
        let html = markdown_to_html("---");
        assert!(html.contains("<hr />"));
    }

    #[test]
    fn test_code_escaping() {
        let md = "```\n<script>alert('xss')</script>\n```";
        let html = markdown_to_html(md);
        assert!(html.contains("&lt;script&gt;"));
    }

    #[test]
    fn test_table_basic() {
        let md = "| Name | Age |\n| --- | --- |\n| Alice | 30 |\n| Bob | 25 |";
        let html = markdown_to_html(md);
        assert!(html.contains("<table>"));
        assert!(html.contains("<thead>"));
        assert!(html.contains("<tbody>"));
        assert!(html.contains("<th>Name</th>"));
        assert!(html.contains("<th>Age</th>"));
        assert!(html.contains("<td>Alice</td>"));
        assert!(html.contains("<td>30</td>"));
        assert!(html.contains("<td>Bob</td>"));
        assert!(html.contains("<td>25</td>"));
        assert!(html.contains("</table>"));
    }

    #[test]
    fn test_table_alignment() {
        let md = "| Left | Center | Right |\n| :--- | :---: | ---: |\n| L | C | R |";
        let html = markdown_to_html(md);
        assert!(html.contains("<th style=\"text-align:left\">"));
        assert!(html.contains("<th style=\"text-align:center\">"));
        assert!(html.contains("<th style=\"text-align:right\">"));
        assert!(html.contains("<td style=\"text-align:left\">L</td>"));
        assert!(html.contains("<td style=\"text-align:center\">C</td>"));
        assert!(html.contains("<td style=\"text-align:right\">R</td>"));
    }

    #[test]
    fn test_table_inline_formatting() {
        let md = "| Header |\n| --- |\n| **bold** and *italic* |";
        let html = markdown_to_html(md);
        assert!(html.contains("<strong>bold</strong>"));
        assert!(html.contains("<em>italic</em>"));
    }

    #[test]
    fn test_table_single_column() {
        let md = "| Solo |\n| --- |\n| data |";
        let html = markdown_to_html(md);
        assert!(html.contains("<th>Solo</th>"));
        assert!(html.contains("<td>data</td>"));
    }

    #[test]
    fn test_table_many_columns() {
        let md = "| A | B | C | D | E |\n| - | - | - | - | - |\n| 1 | 2 | 3 | 4 | 5 |";
        let html = markdown_to_html(md);
        assert!(html.contains("<th>A</th>"));
        assert!(html.contains("<th>E</th>"));
        assert!(html.contains("<td>1</td>"));
        assert!(html.contains("<td>5</td>"));
    }

    #[test]
    fn test_table_followed_by_paragraph() {
        let md = "| H |\n| - |\n| d |\n\nA paragraph after.";
        let html = markdown_to_html(md);
        assert!(html.contains("</table>"));
        assert!(html.contains("<p>A paragraph after.</p>"));
    }

    #[test]
    fn test_table_with_empty_cells() {
        let md = "| A | B |\n| - | - |\n| x |  |";
        let html = markdown_to_html(md);
        assert!(html.contains("<td>x</td>"));
        assert!(html.contains("<td></td>"));
    }

    #[test]
    fn test_is_separator_row() {
        assert!(is_separator_row("| --- | --- |"));
        assert!(is_separator_row("| :--- | :---: | ---: |"));
        assert!(is_separator_row("|---|---|"));
        assert!(!is_separator_row("| abc | def |"));
        assert!(!is_separator_row("not a table"));
    }

    #[test]
    fn test_strikethrough() {
        let html = markdown_to_html("This is ~~deleted~~ text.");
        assert!(html.contains("<del>deleted</del>"));
    }

    #[test]
    fn test_render_with_layout() {
        let page = MarkdownPage::parse("---\ntitle: Test\n---\n# Hello");
        let result = page.render_with_layout(|content, fm| {
            format!("<html><head><title>{}</title></head><body>{}</body></html>",
                fm.get("title").unwrap_or(&String::new()), content)
        });
        assert!(result.contains("<title>Test</title>"));
        assert!(result.contains("<h1"));
    }
}
