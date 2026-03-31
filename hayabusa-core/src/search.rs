//! Full-Text Search for Hayabusa.
//!
//! In-memory inverted index with tokenization, stemming, and ranking.
//!
//! ```ignore
//! use hayabusa_core::prelude::*;
//!
//! let mut idx = SearchIndex::new();
//! idx.add("1", "Rust programming language");
//! idx.add("2", "JavaScript web development");
//! let results = idx.search("rust");
//! ```

use std::collections::HashMap;

// ─── Tokenizer ─────────────────────────────────────────────

/// Tokenize text into lowercase words
pub fn tokenize(text: &str) -> Vec<String> {
    text.to_lowercase()
        .split(|c: char| !c.is_alphanumeric() && c != '_')
        .filter(|s| !s.is_empty() && s.len() > 1)
        .map(|s| s.to_string())
        .collect()
}

/// Simple English stop words
const STOP_WORDS: &[&str] = &[
    "a", "an", "the", "is", "it", "in", "on", "at", "to", "of",
    "and", "or", "but", "for", "with", "as", "by", "from", "be",
    "was", "were", "been", "are", "am", "do", "does", "did",
    "has", "have", "had", "not", "no", "if", "so", "than", "that",
    "this", "these", "those", "he", "she", "we", "they", "you",
];

/// Remove stop words
pub fn remove_stop_words(tokens: &[String]) -> Vec<String> {
    tokens.iter()
        .filter(|t| !STOP_WORDS.contains(&t.as_str()))
        .cloned()
        .collect()
}

/// Very simple English stemmer (suffix stripping)
pub fn simple_stem(word: &str) -> String {
    let w = word.to_lowercase();
    if w.len() <= 3 { return w; }
    // Remove common suffixes
    for suffix in &["ying", "ting", "ning", "ring", "ling", "ing", "ment", "ness", "tion", "sion", "able", "ible", "ful", "less", "ous", "ive", "ity", "ers", "ies", "ied", "est", "ely", "ble", "ual", "ly", "ed", "er", "es", "al", "en"] {
        if w.len() > suffix.len() + 2 && w.ends_with(suffix) {
            return w[..w.len() - suffix.len()].to_string();
        }
    }
    // Plural s
    if w.ends_with('s') && !w.ends_with("ss") && w.len() > 3 {
        return w[..w.len() - 1].to_string();
    }
    w
}

// ─── Search Index ──────────────────────────────────────────

/// Inverted index entry
#[derive(Debug, Clone)]
struct PostingEntry {
    doc_id: String,
    frequency: u32,
    positions: Vec<usize>,
}

/// In-memory full-text search index
#[derive(Debug, Clone)]
pub struct SearchIndex {
    index: HashMap<String, Vec<PostingEntry>>,
    documents: HashMap<String, DocumentEntry>,
    use_stemming: bool,
    use_stop_words: bool,
}

#[derive(Debug, Clone)]
struct DocumentEntry {
    title: Option<String>,
    token_count: usize,
}

impl SearchIndex {
    pub fn new() -> Self {
        Self {
            index: HashMap::new(),
            documents: HashMap::new(),
            use_stemming: true,
            use_stop_words: true,
        }
    }

    pub fn with_stemming(mut self, enabled: bool) -> Self {
        self.use_stemming = enabled;
        self
    }

    pub fn with_stop_words(mut self, enabled: bool) -> Self {
        self.use_stop_words = enabled;
        self
    }

    /// Add a document to the index
    pub fn add(&mut self, id: &str, content: &str) {
        self.add_with_title(id, None, content);
    }

    /// Add a document with a title (title gets higher weight)
    pub fn add_with_title(&mut self, id: &str, title: Option<&str>, content: &str) {
        // Tokenize content (and title with boost)
        let mut full_text = String::new();
        if let Some(t) = title {
            // Repeat title tokens for boost
            full_text.push_str(t);
            full_text.push(' ');
            full_text.push_str(t);
            full_text.push(' ');
        }
        full_text.push_str(content);

        let mut tokens = tokenize(&full_text);
        if self.use_stop_words {
            tokens = remove_stop_words(&tokens);
        }

        self.documents.insert(id.to_string(), DocumentEntry {
            title: title.map(|t| t.to_string()),
            token_count: tokens.len(),
        });

        for (pos, token) in tokens.iter().enumerate() {
            let key = if self.use_stemming { simple_stem(token) } else { token.clone() };

            let entries = self.index.entry(key).or_default();
            if let Some(entry) = entries.iter_mut().find(|e| e.doc_id == id) {
                entry.frequency += 1;
                entry.positions.push(pos);
            } else {
                entries.push(PostingEntry {
                    doc_id: id.to_string(),
                    frequency: 1,
                    positions: vec![pos],
                });
            }
        }
    }

    /// Remove a document from the index
    pub fn remove(&mut self, id: &str) {
        self.documents.remove(id);
        for entries in self.index.values_mut() {
            entries.retain(|e| e.doc_id != id);
        }
        self.index.retain(|_, v| !v.is_empty());
    }

    /// Search the index, returns results sorted by relevance (TF-IDF)
    pub fn search(&self, query: &str) -> Vec<SearchResult> {
        let mut tokens = tokenize(query);
        if self.use_stop_words {
            tokens = remove_stop_words(&tokens);
        }

        let num_docs = self.documents.len() as f64;
        if num_docs == 0.0 || tokens.is_empty() {
            return Vec::new();
        }

        let mut scores: HashMap<String, f64> = HashMap::new();

        for token in &tokens {
            let key = if self.use_stemming { simple_stem(token) } else { token.clone() };

            if let Some(entries) = self.index.get(&key) {
                let idf = (num_docs / entries.len() as f64).ln() + 1.0;
                for entry in entries {
                    let doc = match self.documents.get(&entry.doc_id) {
                        Some(d) => d,
                        None => continue,
                    };
                    let tf = entry.frequency as f64 / doc.token_count.max(1) as f64;
                    *scores.entry(entry.doc_id.clone()).or_insert(0.0) += tf * idf;
                }
            }
        }

        let mut results: Vec<SearchResult> = scores.into_iter().map(|(id, score)| {
            let title = self.documents.get(&id).and_then(|d| d.title.clone());
            SearchResult { id, score, title }
        }).collect();

        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        results
    }

    /// Search with limit
    pub fn search_limit(&self, query: &str, limit: usize) -> Vec<SearchResult> {
        let mut results = self.search(query);
        results.truncate(limit);
        results
    }

    /// Get the number of indexed documents
    pub fn doc_count(&self) -> usize {
        self.documents.len()
    }

    /// Get the number of unique terms
    pub fn term_count(&self) -> usize {
        self.index.len()
    }

    /// Clear the entire index
    pub fn clear(&mut self) {
        self.index.clear();
        self.documents.clear();
    }
}

impl Default for SearchIndex {
    fn default() -> Self {
        Self::new()
    }
}

/// Search result
#[derive(Debug, Clone)]
pub struct SearchResult {
    pub id: String,
    pub score: f64,
    pub title: Option<String>,
}

impl SearchResult {
    pub fn to_json(&self) -> String {
        let title = match &self.title {
            Some(t) => format!(",\"title\":\"{}\"", t.replace('"', "\\\"")),
            None => String::new(),
        };
        format!("{{\"id\":\"{}\",\"score\":{:.4}{}}}", self.id, self.score, title)
    }
}

/// Generate a simple search form HTML
pub fn search_form(action: &str, placeholder: &str) -> String {
    format!(
        r#"<form action="{}" method="GET" class="search-form">
<input type="search" name="q" placeholder="{}" autocomplete="off" aria-label="Search">
<button type="submit">Search</button>
</form>"#,
        action, placeholder
    )
}

/// Highlight search terms in text
pub fn highlight(text: &str, query: &str, tag: &str) -> String {
    let terms = tokenize(query);
    let mut result = text.to_string();
    for term in &terms {
        // Case-insensitive replace with highlight
        let lower = result.to_lowercase();
        if let Some(pos) = lower.find(&term.to_lowercase()) {
            let original = &result[pos..pos + term.len()];
            let highlighted = format!("<{tag}>{}</{tag}>", original, tag = tag);
            result = format!("{}{}{}", &result[..pos], highlighted, &result[pos + term.len()..]);
        }
    }
    result
}

// ─── Tests ──────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tokenize() {
        let tokens = tokenize("Hello, World! Rust programming.");
        assert_eq!(tokens, vec!["hello", "world", "rust", "programming"]);
    }

    #[test]
    fn test_tokenize_short_words() {
        let tokens = tokenize("I am a cat");
        // Single char words filtered out
        assert!(!tokens.contains(&"i".to_string()));
        assert!(!tokens.contains(&"a".to_string()));
    }

    #[test]
    fn test_stop_words() {
        let tokens = tokenize("the cat is on the mat");
        let filtered = remove_stop_words(&tokens);
        assert!(filtered.contains(&"cat".to_string()));
        assert!(filtered.contains(&"mat".to_string()));
        assert!(!filtered.contains(&"the".to_string()));
        assert!(!filtered.contains(&"is".to_string()));
    }

    #[test]
    fn test_simple_stem() {
        assert_eq!(simple_stem("running"), "run");
        assert_eq!(simple_stem("cats"), "cat");
        assert_eq!(simple_stem("happiness"), "happi");
        assert_eq!(simple_stem("go"), "go"); // too short
    }

    #[test]
    fn test_search_index_basic() {
        let mut idx = SearchIndex::new();
        idx.add("1", "Rust programming language");
        idx.add("2", "JavaScript web development");
        idx.add("3", "Rust web framework");

        let results = idx.search("rust");
        assert!(!results.is_empty());
        assert!(results.iter().any(|r| r.id == "1"));
        assert!(results.iter().any(|r| r.id == "3"));
    }

    #[test]
    fn test_search_ranking() {
        let mut idx = SearchIndex::new();
        idx.add("1", "rust rust rust programming");
        idx.add("2", "rust programming");

        let results = idx.search("rust");
        assert!(results.len() >= 2);
        // Doc 1 has more occurrences of "rust"
        assert_eq!(results[0].id, "1");
    }

    #[test]
    fn test_search_no_results() {
        let mut idx = SearchIndex::new();
        idx.add("1", "hello world");
        let results = idx.search("nonexistent");
        assert!(results.is_empty());
    }

    #[test]
    fn test_search_with_title() {
        let mut idx = SearchIndex::new();
        idx.add_with_title("1", Some("Rust Guide"), "A comprehensive guide");
        let results = idx.search("rust");
        assert!(!results.is_empty());
        assert_eq!(results[0].title, Some("Rust Guide".to_string()));
    }

    #[test]
    fn test_search_remove() {
        let mut idx = SearchIndex::new();
        idx.add("1", "hello world");
        idx.add("2", "hello rust");
        idx.remove("1");
        assert_eq!(idx.doc_count(), 1);
        let results = idx.search("hello");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, "2");
    }

    #[test]
    fn test_search_limit() {
        let mut idx = SearchIndex::new();
        for i in 0..10 {
            idx.add(&i.to_string(), &format!("document about rust {}", i));
        }
        let results = idx.search_limit("rust", 3);
        assert_eq!(results.len(), 3);
    }

    #[test]
    fn test_search_clear() {
        let mut idx = SearchIndex::new();
        idx.add("1", "test");
        idx.clear();
        assert_eq!(idx.doc_count(), 0);
        assert_eq!(idx.term_count(), 0);
    }

    #[test]
    fn test_search_result_json() {
        let r = SearchResult { id: "1".into(), score: 2.5, title: Some("Test".into()) };
        let json = r.to_json();
        assert!(json.contains("\"id\":\"1\""));
        assert!(json.contains("\"title\":\"Test\""));
    }

    #[test]
    fn test_search_form() {
        let html = search_form("/search", "Search...");
        assert!(html.contains("/search"));
        assert!(html.contains("Search..."));
        assert!(html.contains("name=\"q\""));
    }

    #[test]
    fn test_highlight() {
        let result = highlight("Rust programming is great", "rust", "mark");
        assert!(result.contains("<mark>Rust</mark>"));
    }

    #[test]
    fn test_no_stemming() {
        let mut idx = SearchIndex::new().with_stemming(false);
        idx.add("1", "running runners");
        let results = idx.search("running");
        assert!(!results.is_empty());
        let results2 = idx.search("run");
        assert!(results2.is_empty()); // No stemming, so "run" won't match
    }

    #[test]
    fn test_empty_search() {
        let idx = SearchIndex::new();
        let results = idx.search("");
        assert!(results.is_empty());
    }
}
