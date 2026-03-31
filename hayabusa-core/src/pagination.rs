//! Pagination for Hayabusa.
//!
//! Cursor-based and offset-based pagination with HTML UI helpers.
//!
//! ```ignore
//! use hayabusa_core::prelude::*;
//!
//! let page = OffsetPagination::new(100, 10, 1);
//! let html = page.to_html("/items");
//! ```

// ─── Offset-Based Pagination ──────────────────────────────

/// Offset-based pagination (page number + page size)
#[derive(Debug, Clone)]
pub struct OffsetPagination {
    pub total_items: usize,
    pub page_size: usize,
    pub current_page: usize,
}

impl OffsetPagination {
    pub fn new(total_items: usize, page_size: usize, current_page: usize) -> Self {
        Self {
            total_items,
            page_size: page_size.max(1),
            current_page: current_page.max(1),
        }
    }

    /// Parse from query parameters (e.g., page=2&per_page=20)
    pub fn from_params(total_items: usize, params: &[(&str, &str)]) -> Self {
        let page = params.iter()
            .find(|(k, _)| *k == "page")
            .and_then(|(_, v)| v.parse::<usize>().ok())
            .unwrap_or(1);
        let per_page = params.iter()
            .find(|(k, _)| *k == "per_page" || *k == "page_size" || *k == "limit")
            .and_then(|(_, v)| v.parse::<usize>().ok())
            .unwrap_or(20);
        Self::new(total_items, per_page, page)
    }

    pub fn total_pages(&self) -> usize {
        (self.total_items + self.page_size - 1) / self.page_size
    }

    pub fn offset(&self) -> usize {
        (self.current_page - 1) * self.page_size
    }

    pub fn has_prev(&self) -> bool {
        self.current_page > 1
    }

    pub fn has_next(&self) -> bool {
        self.current_page < self.total_pages()
    }

    pub fn prev_page(&self) -> Option<usize> {
        if self.has_prev() { Some(self.current_page - 1) } else { None }
    }

    pub fn next_page(&self) -> Option<usize> {
        if self.has_next() { Some(self.current_page + 1) } else { None }
    }

    /// Generate visible page numbers (with ellipsis represented as 0)
    pub fn page_numbers(&self, window: usize) -> Vec<usize> {
        let total = self.total_pages();
        if total <= window * 2 + 3 {
            return (1..=total).collect();
        }
        let mut pages = Vec::new();
        pages.push(1);
        let start = self.current_page.saturating_sub(window).max(2);
        let end = (self.current_page + window).min(total - 1);
        if start > 2 { pages.push(0); } // ellipsis
        for i in start..=end { pages.push(i); }
        if end < total - 1 { pages.push(0); } // ellipsis
        if total > 1 { pages.push(total); }
        pages
    }

    /// SQL LIMIT/OFFSET clause
    pub fn sql_clause(&self) -> String {
        format!("LIMIT {} OFFSET {}", self.page_size, self.offset())
    }

    /// Convert to JSON
    pub fn to_json(&self) -> String {
        format!(
            "{{\"total_items\":{},\"page_size\":{},\"current_page\":{},\"total_pages\":{},\"has_prev\":{},\"has_next\":{}}}",
            self.total_items, self.page_size, self.current_page, self.total_pages(), self.has_prev(), self.has_next()
        )
    }

    /// Generate pagination HTML
    pub fn to_html(&self, base_path: &str) -> String {
        let mut html = String::from("<nav class=\"pagination\" aria-label=\"Pagination\">\n");

        // Previous button
        if self.has_prev() {
            html.push_str(&format!(
                "  <a href=\"{}?page={}\" class=\"pagination-prev\" rel=\"prev\">&laquo; Prev</a>\n",
                base_path, self.current_page - 1
            ));
        } else {
            html.push_str("  <span class=\"pagination-prev disabled\">&laquo; Prev</span>\n");
        }

        // Page numbers
        html.push_str("  <span class=\"pagination-pages\">\n");
        for &p in &self.page_numbers(2) {
            if p == 0 {
                html.push_str("    <span class=\"pagination-ellipsis\">&hellip;</span>\n");
            } else if p == self.current_page {
                html.push_str(&format!(
                    "    <span class=\"pagination-current\" aria-current=\"page\">{}</span>\n", p
                ));
            } else {
                html.push_str(&format!(
                    "    <a href=\"{}?page={}\" class=\"pagination-page\">{}</a>\n",
                    base_path, p, p
                ));
            }
        }
        html.push_str("  </span>\n");

        // Next button
        if self.has_next() {
            html.push_str(&format!(
                "  <a href=\"{}?page={}\" class=\"pagination-next\" rel=\"next\">Next &raquo;</a>\n",
                base_path, self.current_page + 1
            ));
        } else {
            html.push_str("  <span class=\"pagination-next disabled\">Next &raquo;</span>\n");
        }

        html.push_str("</nav>");
        html
    }

    /// Generate pagination CSS
    pub fn css() -> &'static str {
        r#".pagination{display:flex;align-items:center;gap:.5rem;font-family:system-ui}
.pagination a,.pagination span{padding:.5rem .75rem;border:1px solid #ddd;border-radius:4px;text-decoration:none;color:#333}
.pagination a:hover{background:#f0f0f0}
.pagination-current{background:#0070f3;color:#fff!important;border-color:#0070f3!important}
.pagination .disabled{opacity:.4;pointer-events:none}
.pagination-ellipsis{border:none!important}"#
    }
}

// ─── Cursor-Based Pagination ──────────────────────────────

/// Cursor-based pagination (for infinite scroll, real-time feeds)
#[derive(Debug, Clone)]
pub struct CursorPagination {
    pub cursor: Option<String>,
    pub limit: usize,
    pub has_more: bool,
    pub next_cursor: Option<String>,
}

impl CursorPagination {
    pub fn new(limit: usize) -> Self {
        Self {
            cursor: None,
            limit,
            has_more: false,
            next_cursor: None,
        }
    }

    /// Parse cursor from query params
    pub fn from_params(params: &[(&str, &str)]) -> Self {
        let cursor = params.iter()
            .find(|(k, _)| *k == "cursor" || *k == "after")
            .map(|(_, v)| v.to_string());
        let limit = params.iter()
            .find(|(k, _)| *k == "limit" || *k == "first")
            .and_then(|(_, v)| v.parse::<usize>().ok())
            .unwrap_or(20);
        Self {
            cursor,
            limit,
            has_more: false,
            next_cursor: None,
        }
    }

    /// Set the result: next cursor and whether more items exist
    pub fn with_result(mut self, next_cursor: Option<String>, has_more: bool) -> Self {
        self.next_cursor = next_cursor;
        self.has_more = has_more;
        self
    }

    /// SQL WHERE clause for cursor-based pagination (assumes cursor is an ID)
    pub fn sql_where(&self, column: &str) -> String {
        if let Some(ref cursor) = self.cursor {
            format!("WHERE {} > '{}' ORDER BY {} ASC LIMIT {}", column, cursor, column, self.limit + 1)
        } else {
            format!("ORDER BY {} ASC LIMIT {}", column, self.limit + 1)
        }
    }

    /// Convert to JSON
    pub fn to_json(&self) -> String {
        let cursor_str = match &self.next_cursor {
            Some(c) => format!("\"{}\"", c),
            None => "null".to_string(),
        };
        format!(
            "{{\"cursor\":{},\"limit\":{},\"has_more\":{}}}",
            cursor_str, self.limit, self.has_more
        )
    }
}

// ─── Link Header ──────────────────────────────────────────

/// Generate RFC 8288 Link header for pagination
pub fn link_header(base_url: &str, page: &OffsetPagination) -> String {
    let mut links = Vec::new();
    links.push(format!("<{}?page=1>; rel=\"first\"", base_url));
    if let Some(prev) = page.prev_page() {
        links.push(format!("<{}?page={}>; rel=\"prev\"", base_url, prev));
    }
    if let Some(next) = page.next_page() {
        links.push(format!("<{}?page={}>; rel=\"next\"", base_url, next));
    }
    links.push(format!("<{}?page={}>; rel=\"last\"", base_url, page.total_pages()));
    links.join(", ")
}

// ─── Tests ──────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_offset_basic() {
        let p = OffsetPagination::new(100, 10, 1);
        assert_eq!(p.total_pages(), 10);
        assert_eq!(p.offset(), 0);
        assert!(!p.has_prev());
        assert!(p.has_next());
    }

    #[test]
    fn test_offset_middle() {
        let p = OffsetPagination::new(100, 10, 5);
        assert_eq!(p.offset(), 40);
        assert!(p.has_prev());
        assert!(p.has_next());
        assert_eq!(p.prev_page(), Some(4));
        assert_eq!(p.next_page(), Some(6));
    }

    #[test]
    fn test_offset_last_page() {
        let p = OffsetPagination::new(100, 10, 10);
        assert!(!p.has_next());
        assert!(p.has_prev());
        assert_eq!(p.next_page(), None);
    }

    #[test]
    fn test_offset_from_params() {
        let p = OffsetPagination::from_params(50, &[("page", "3"), ("per_page", "15")]);
        assert_eq!(p.current_page, 3);
        assert_eq!(p.page_size, 15);
        assert_eq!(p.total_pages(), 4);
    }

    #[test]
    fn test_offset_page_numbers() {
        let p = OffsetPagination::new(200, 10, 10);
        let pages = p.page_numbers(2);
        assert!(pages.contains(&1));
        assert!(pages.contains(&20));
        assert!(pages.contains(&10));
    }

    #[test]
    fn test_offset_small_total() {
        let p = OffsetPagination::new(30, 10, 2);
        let pages = p.page_numbers(2);
        assert_eq!(pages, vec![1, 2, 3]); // No ellipsis needed
    }

    #[test]
    fn test_offset_sql() {
        let p = OffsetPagination::new(100, 20, 3);
        assert_eq!(p.sql_clause(), "LIMIT 20 OFFSET 40");
    }

    #[test]
    fn test_offset_json() {
        let p = OffsetPagination::new(50, 10, 2);
        let json = p.to_json();
        assert!(json.contains("\"total_items\":50"));
        assert!(json.contains("\"current_page\":2"));
        assert!(json.contains("\"total_pages\":5"));
    }

    #[test]
    fn test_offset_html() {
        let p = OffsetPagination::new(100, 10, 5);
        let html = p.to_html("/items");
        assert!(html.contains("pagination"));
        assert!(html.contains("page=4"));
        assert!(html.contains("page=6"));
        assert!(html.contains("aria-current"));
    }

    #[test]
    fn test_offset_html_first_page() {
        let p = OffsetPagination::new(30, 10, 1);
        let html = p.to_html("/items");
        assert!(html.contains("disabled"));
        assert!(html.contains("page=2"));
    }

    #[test]
    fn test_offset_css() {
        let css = OffsetPagination::css();
        assert!(css.contains(".pagination"));
        assert!(css.contains(".pagination-current"));
    }

    #[test]
    fn test_cursor_basic() {
        let c = CursorPagination::new(20);
        assert_eq!(c.limit, 20);
        assert!(c.cursor.is_none());
        assert!(!c.has_more);
    }

    #[test]
    fn test_cursor_from_params() {
        let c = CursorPagination::from_params(&[("cursor", "abc123"), ("limit", "10")]);
        assert_eq!(c.cursor, Some("abc123".to_string()));
        assert_eq!(c.limit, 10);
    }

    #[test]
    fn test_cursor_with_result() {
        let c = CursorPagination::new(10)
            .with_result(Some("next_id".into()), true);
        assert!(c.has_more);
        assert_eq!(c.next_cursor, Some("next_id".to_string()));
    }

    #[test]
    fn test_cursor_sql() {
        let c = CursorPagination::new(10);
        let sql = c.sql_where("id");
        assert!(sql.contains("LIMIT 11"));
        assert!(sql.contains("ORDER BY id ASC"));

        let c2 = CursorPagination::from_params(&[("cursor", "50")]);
        let sql2 = c2.sql_where("id");
        assert!(sql2.contains("WHERE id > '50'"));
    }

    #[test]
    fn test_cursor_json() {
        let c = CursorPagination::new(10)
            .with_result(Some("abc".into()), true);
        let json = c.to_json();
        assert!(json.contains("\"has_more\":true"));
        assert!(json.contains("\"abc\""));
    }

    #[test]
    fn test_link_header() {
        let p = OffsetPagination::new(100, 10, 5);
        let link = link_header("/api/items", &p);
        assert!(link.contains("rel=\"first\""));
        assert!(link.contains("rel=\"prev\""));
        assert!(link.contains("rel=\"next\""));
        assert!(link.contains("rel=\"last\""));
        assert!(link.contains("page=4"));
        assert!(link.contains("page=6"));
    }

    #[test]
    fn test_zero_items() {
        let p = OffsetPagination::new(0, 10, 1);
        assert_eq!(p.total_pages(), 0);
        assert!(!p.has_prev());
        assert!(!p.has_next());
    }
}
