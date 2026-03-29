//! Internationalization (i18n) routing for Hayabusa.
//!
//! Provides Next.js-like i18n support:
//! - Locale detection from Accept-Language header
//! - Path-based locale routing (`/en/about`, `/ja/about`)
//! - Default locale (no prefix needed)
//! - Locale-specific metadata and head context
//! - Automatic redirect to user's preferred locale

use std::collections::HashMap;

/// i18n configuration
#[derive(Debug, Clone)]
pub struct I18nConfig {
    /// List of supported locales (e.g., ["en", "ja", "ko"])
    pub locales: Vec<String>,
    /// Default locale (e.g., "en")
    pub default_locale: String,
    /// Whether the default locale should have a path prefix
    /// false: `/about` (default locale), `/ja/about` (other locales)
    /// true: `/en/about`, `/ja/about` (all locales have prefix)
    pub prefix_default: bool,
}

impl Default for I18nConfig {
    fn default() -> Self {
        Self {
            locales: vec!["en".to_string()],
            default_locale: "en".to_string(),
            prefix_default: false,
        }
    }
}

impl I18nConfig {
    pub fn new(locales: Vec<&str>, default: &str) -> Self {
        Self {
            locales: locales.iter().map(|s| s.to_string()).collect(),
            default_locale: default.to_string(),
            prefix_default: false,
        }
    }

    pub fn prefix_default(mut self, prefix: bool) -> Self {
        self.prefix_default = prefix;
        self
    }

    /// Check if a locale is supported
    pub fn is_supported(&self, locale: &str) -> bool {
        self.locales.iter().any(|l| l == locale)
    }

    /// Extract locale from a URL path.
    /// Returns (locale, path_without_locale).
    ///
    /// Examples:
    /// - `/ja/about` → `("ja", "/about")`
    /// - `/about` → `("en", "/about")` (default locale)
    /// - `/en/about` with prefix_default=false → `("en", "/about")`
    pub fn extract_locale<'a>(&self, path: &'a str) -> (String, String) {
        // Try to extract locale from first path segment
        let trimmed = path.trim_start_matches('/');
        if let Some(slash_pos) = trimmed.find('/') {
            let first_segment = &trimmed[..slash_pos];
            if self.is_supported(first_segment) {
                let rest = &trimmed[slash_pos..];
                return (first_segment.to_string(), rest.to_string());
            }
        } else if !trimmed.is_empty() && self.is_supported(trimmed) {
            // Path is just the locale (e.g., "/ja")
            return (trimmed.to_string(), "/".to_string());
        }

        // No locale prefix found: use default
        (self.default_locale.clone(), path.to_string())
    }

    /// Build a localized URL path
    pub fn localize_path(&self, locale: &str, path: &str) -> String {
        if locale == self.default_locale && !self.prefix_default {
            path.to_string()
        } else {
            if path.starts_with('/') {
                format!("/{}{}", locale, path)
            } else {
                format!("/{}/{}", locale, path)
            }
        }
    }

    /// Detect the preferred locale from the Accept-Language header.
    ///
    /// Parses quality values (q-values) and returns the best matching locale.
    /// Example header: `en-US,en;q=0.9,ja;q=0.8`
    pub fn detect_locale(&self, accept_language: &str) -> String {
        let mut preferences = parse_accept_language(accept_language);
        preferences.sort_by(|a, b| b.quality.partial_cmp(&a.quality).unwrap());

        for pref in &preferences {
            // Exact match
            if self.is_supported(&pref.language) {
                return pref.language.clone();
            }

            // Match by language prefix (e.g., "en-US" matches "en")
            if let Some(lang_prefix) = pref.language.split('-').next() {
                if self.is_supported(lang_prefix) {
                    return lang_prefix.to_string();
                }
            }
        }

        self.default_locale.clone()
    }

    /// Generate <link rel="alternate" hreflang="..."> tags for SEO.
    ///
    /// Search engines use these to understand page translations.
    pub fn render_hreflang_tags(&self, base_url: &str, path: &str) -> String {
        let mut tags = String::new();

        for locale in &self.locales {
            let localized = self.localize_path(locale, path);
            tags.push_str(&format!(
                "<link rel=\"alternate\" hreflang=\"{locale}\" href=\"{base_url}{localized}\" />\n"
            ));
        }

        // x-default for the default locale
        let default_path = self.localize_path(&self.default_locale, path);
        tags.push_str(&format!(
            "<link rel=\"alternate\" hreflang=\"x-default\" href=\"{base_url}{default_path}\" />\n"
        ));

        tags
    }
}

/// A parsed language preference from Accept-Language header
#[derive(Debug, Clone)]
struct LanguagePreference {
    language: String,
    quality: f64,
}

/// Parse the Accept-Language header into ordered preferences.
///
/// Example: `en-US,en;q=0.9,ja;q=0.8` →
/// [("en-US", 1.0), ("en", 0.9), ("ja", 0.8)]
fn parse_accept_language(header: &str) -> Vec<LanguagePreference> {
    header
        .split(',')
        .filter_map(|entry| {
            let entry = entry.trim();
            if entry.is_empty() {
                return None;
            }

            let mut parts = entry.split(';');
            let language = parts.next()?.trim().to_lowercase();

            let quality = parts
                .find_map(|p| {
                    let p = p.trim();
                    if p.starts_with("q=") {
                        p[2..].parse::<f64>().ok()
                    } else {
                        None
                    }
                })
                .unwrap_or(1.0);

            Some(LanguagePreference { language, quality })
        })
        .collect()
}

/// Translation dictionary type
pub type Translations = HashMap<String, HashMap<String, String>>;

/// Simple translation helper.
///
/// # Example
/// ```ignore
/// let mut t = TranslationStore::new();
/// t.add("en", "greeting", "Hello!");
/// t.add("ja", "greeting", "こんにちは！");
///
/// assert_eq!(t.t("en", "greeting"), "Hello!");
/// assert_eq!(t.t("ja", "greeting"), "こんにちは！");
/// ```
pub struct TranslationStore {
    translations: Translations,
}

impl TranslationStore {
    pub fn new() -> Self {
        Self {
            translations: HashMap::new(),
        }
    }

    /// Add a translation
    pub fn add(&mut self, locale: &str, key: &str, value: &str) {
        self.translations
            .entry(locale.to_string())
            .or_default()
            .insert(key.to_string(), value.to_string());
    }

    /// Get a translation, falling back to the key itself
    pub fn t(&self, locale: &str, key: &str) -> String {
        self.translations
            .get(locale)
            .and_then(|map| map.get(key))
            .cloned()
            .unwrap_or_else(|| key.to_string())
    }

    /// Get a translation with named parameters.
    /// Parameters are replaced using `{name}` syntax.
    pub fn t_with(&self, locale: &str, key: &str, params: &[(&str, &str)]) -> String {
        let mut text = self.t(locale, key);
        for (name, value) in params {
            text = text.replace(&format!("{{{name}}}"), value);
        }
        text
    }
}

impl Default for TranslationStore {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_locale() {
        let config = I18nConfig::new(vec!["en", "ja", "ko"], "en");

        let (locale, path) = config.extract_locale("/ja/about");
        assert_eq!(locale, "ja");
        assert_eq!(path, "/about");

        let (locale, path) = config.extract_locale("/about");
        assert_eq!(locale, "en");
        assert_eq!(path, "/about");

        let (locale, path) = config.extract_locale("/ko");
        assert_eq!(locale, "ko");
        assert_eq!(path, "/");
    }

    #[test]
    fn test_localize_path() {
        let config = I18nConfig::new(vec!["en", "ja"], "en");

        assert_eq!(config.localize_path("en", "/about"), "/about");
        assert_eq!(config.localize_path("ja", "/about"), "/ja/about");
    }

    #[test]
    fn test_localize_path_prefix_default() {
        let config = I18nConfig::new(vec!["en", "ja"], "en").prefix_default(true);

        assert_eq!(config.localize_path("en", "/about"), "/en/about");
        assert_eq!(config.localize_path("ja", "/about"), "/ja/about");
    }

    #[test]
    fn test_detect_locale() {
        let config = I18nConfig::new(vec!["en", "ja", "ko"], "en");

        assert_eq!(
            config.detect_locale("ja,en-US;q=0.9,en;q=0.8"),
            "ja"
        );
        assert_eq!(
            config.detect_locale("en-US,en;q=0.9,ja;q=0.8"),
            "en"
        );
        assert_eq!(
            config.detect_locale("fr,de;q=0.9"),
            "en" // fallback to default
        );
        assert_eq!(
            config.detect_locale("ko-KR;q=0.9,en;q=0.8"),
            "ko"
        );
    }

    #[test]
    fn test_hreflang_tags() {
        let config = I18nConfig::new(vec!["en", "ja"], "en");
        let tags = config.render_hreflang_tags("https://example.com", "/about");

        assert!(tags.contains("hreflang=\"en\""));
        assert!(tags.contains("hreflang=\"ja\""));
        assert!(tags.contains("hreflang=\"x-default\""));
        assert!(tags.contains("href=\"https://example.com/about\""));
        assert!(tags.contains("href=\"https://example.com/ja/about\""));
    }

    #[test]
    fn test_translation_store() {
        let mut t = TranslationStore::new();
        t.add("en", "greeting", "Hello!");
        t.add("ja", "greeting", "こんにちは！");
        t.add("en", "welcome", "Welcome, {name}!");

        assert_eq!(t.t("en", "greeting"), "Hello!");
        assert_eq!(t.t("ja", "greeting"), "こんにちは！");
        assert_eq!(t.t("en", "missing_key"), "missing_key"); // fallback
        assert_eq!(
            t.t_with("en", "welcome", &[("name", "World")]),
            "Welcome, World!"
        );
    }

    #[test]
    fn test_parse_accept_language() {
        let prefs = parse_accept_language("en-US,en;q=0.9,ja;q=0.8,*;q=0.1");
        assert_eq!(prefs.len(), 4);
        assert_eq!(prefs[0].language, "en-us");
        assert_eq!(prefs[0].quality, 1.0);
        assert_eq!(prefs[1].language, "en");
        assert_eq!(prefs[1].quality, 0.9);
        assert_eq!(prefs[2].language, "ja");
        assert_eq!(prefs[2].quality, 0.8);
    }
}
