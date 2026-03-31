//! SEO Tools for Hayabusa.
//!
//! Sitemap generation, RSS feeds, Open Graph meta tags,
//! JSON-LD structured data, and robots.txt generation.
//!
//! ```ignore
//! use hayabusa_core::prelude::*;
//!
//! let sitemap = Sitemap::new("https://example.com")
//!     .add("/", Priority::High, ChangeFreq::Daily)
//!     .add("/blog", Priority::Medium, ChangeFreq::Weekly)
//!     .build();
//! ```

// ─── Sitemap ────────────────────────────────────────────────

/// XML Sitemap generator
#[derive(Debug, Clone)]
pub struct Sitemap {
    pub base_url: String,
    pub entries: Vec<SitemapEntry>,
}

#[derive(Debug, Clone)]
pub struct SitemapEntry {
    pub path: String,
    pub priority: Priority,
    pub changefreq: ChangeFreq,
    pub lastmod: Option<String>,
}

#[derive(Debug, Clone, Copy)]
pub enum Priority {
    Highest,
    High,
    Medium,
    Low,
    Lowest,
}

impl Priority {
    pub fn value(&self) -> f32 {
        match self {
            Priority::Highest => 1.0,
            Priority::High => 0.8,
            Priority::Medium => 0.5,
            Priority::Low => 0.3,
            Priority::Lowest => 0.1,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum ChangeFreq {
    Always,
    Hourly,
    Daily,
    Weekly,
    Monthly,
    Yearly,
    Never,
}

impl ChangeFreq {
    pub fn as_str(&self) -> &str {
        match self {
            ChangeFreq::Always => "always",
            ChangeFreq::Hourly => "hourly",
            ChangeFreq::Daily => "daily",
            ChangeFreq::Weekly => "weekly",
            ChangeFreq::Monthly => "monthly",
            ChangeFreq::Yearly => "yearly",
            ChangeFreq::Never => "never",
        }
    }
}

impl Sitemap {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into().trim_end_matches('/').to_string(),
            entries: Vec::new(),
        }
    }

    pub fn add(mut self, path: &str, priority: Priority, changefreq: ChangeFreq) -> Self {
        self.entries.push(SitemapEntry {
            path: path.to_string(),
            priority,
            changefreq,
            lastmod: None,
        });
        self
    }

    pub fn add_with_date(mut self, path: &str, priority: Priority, changefreq: ChangeFreq, lastmod: &str) -> Self {
        self.entries.push(SitemapEntry {
            path: path.to_string(),
            priority,
            changefreq,
            lastmod: Some(lastmod.to_string()),
        });
        self
    }

    /// Build XML sitemap string
    pub fn build(&self) -> String {
        let mut xml = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
        xml.push_str("<urlset xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\">\n");

        for entry in &self.entries {
            xml.push_str("  <url>\n");
            xml.push_str(&format!("    <loc>{}{}</loc>\n", self.base_url, entry.path));
            if let Some(ref lastmod) = entry.lastmod {
                xml.push_str(&format!("    <lastmod>{}</lastmod>\n", lastmod));
            }
            xml.push_str(&format!("    <changefreq>{}</changefreq>\n", entry.changefreq.as_str()));
            xml.push_str(&format!("    <priority>{:.1}</priority>\n", entry.priority.value()));
            xml.push_str("  </url>\n");
        }

        xml.push_str("</urlset>");
        xml
    }

    /// Build sitemap index (for large sites with multiple sitemaps)
    pub fn build_index(base_url: &str, sitemap_paths: &[&str]) -> String {
        let base = base_url.trim_end_matches('/');
        let mut xml = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
        xml.push_str("<sitemapindex xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\">\n");
        for path in sitemap_paths {
            xml.push_str("  <sitemap>\n");
            xml.push_str(&format!("    <loc>{}{}</loc>\n", base, path));
            xml.push_str("  </sitemap>\n");
        }
        xml.push_str("</sitemapindex>");
        xml
    }
}

// ─── RSS Feed ───────────────────────────────────────────────

/// RSS 2.0 feed generator
#[derive(Debug, Clone)]
pub struct RssFeed {
    pub title: String,
    pub link: String,
    pub description: String,
    pub language: Option<String>,
    pub items: Vec<RssItem>,
}

#[derive(Debug, Clone)]
pub struct RssItem {
    pub title: String,
    pub link: String,
    pub description: String,
    pub pub_date: Option<String>,
    pub guid: Option<String>,
    pub author: Option<String>,
    pub categories: Vec<String>,
}

impl RssFeed {
    pub fn new(title: impl Into<String>, link: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            link: link.into(),
            description: description.into(),
            language: None,
            items: Vec::new(),
        }
    }

    pub fn language(mut self, lang: impl Into<String>) -> Self {
        self.language = Some(lang.into());
        self
    }

    pub fn add_item(mut self, item: RssItem) -> Self {
        self.items.push(item);
        self
    }

    /// Build RSS 2.0 XML
    pub fn build(&self) -> String {
        let mut xml = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
        xml.push_str("<rss version=\"2.0\" xmlns:atom=\"http://www.w3.org/2005/Atom\">\n");
        xml.push_str("<channel>\n");
        xml.push_str(&format!("  <title>{}</title>\n", xml_escape(&self.title)));
        xml.push_str(&format!("  <link>{}</link>\n", xml_escape(&self.link)));
        xml.push_str(&format!("  <description>{}</description>\n", xml_escape(&self.description)));
        xml.push_str(&format!(
            "  <atom:link href=\"{}/rss.xml\" rel=\"self\" type=\"application/rss+xml\"/>\n",
            self.link.trim_end_matches('/')
        ));

        if let Some(ref lang) = self.language {
            xml.push_str(&format!("  <language>{}</language>\n", lang));
        }

        for item in &self.items {
            xml.push_str("  <item>\n");
            xml.push_str(&format!("    <title>{}</title>\n", xml_escape(&item.title)));
            xml.push_str(&format!("    <link>{}</link>\n", xml_escape(&item.link)));
            xml.push_str(&format!("    <description>{}</description>\n", xml_escape(&item.description)));
            if let Some(ref date) = item.pub_date {
                xml.push_str(&format!("    <pubDate>{}</pubDate>\n", date));
            }
            if let Some(ref guid) = item.guid {
                xml.push_str(&format!("    <guid>{}</guid>\n", xml_escape(guid)));
            } else {
                xml.push_str(&format!("    <guid>{}</guid>\n", xml_escape(&item.link)));
            }
            if let Some(ref author) = item.author {
                xml.push_str(&format!("    <author>{}</author>\n", xml_escape(author)));
            }
            for cat in &item.categories {
                xml.push_str(&format!("    <category>{}</category>\n", xml_escape(cat)));
            }
            xml.push_str("  </item>\n");
        }

        xml.push_str("</channel>\n</rss>");
        xml
    }
}

impl RssItem {
    pub fn new(title: impl Into<String>, link: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            link: link.into(),
            description: description.into(),
            pub_date: None,
            guid: None,
            author: None,
            categories: Vec::new(),
        }
    }

    pub fn pub_date(mut self, date: impl Into<String>) -> Self {
        self.pub_date = Some(date.into());
        self
    }

    pub fn author(mut self, author: impl Into<String>) -> Self {
        self.author = Some(author.into());
        self
    }

    pub fn category(mut self, cat: impl Into<String>) -> Self {
        self.categories.push(cat.into());
        self
    }
}

// ─── Open Graph Meta Tags ───────────────────────────────────

/// Open Graph and Twitter Card meta tag generator
#[derive(Debug, Clone)]
pub struct OpenGraph {
    pub title: String,
    pub description: String,
    pub url: Option<String>,
    pub image: Option<String>,
    pub image_alt: Option<String>,
    pub og_type: OgType,
    pub site_name: Option<String>,
    pub locale: Option<String>,
    pub twitter_card: TwitterCard,
    pub twitter_site: Option<String>,
    pub twitter_creator: Option<String>,
}

#[derive(Debug, Clone, Copy)]
pub enum OgType {
    Website,
    Article,
    Product,
    Profile,
    Video,
    Music,
}

impl OgType {
    pub fn as_str(&self) -> &str {
        match self {
            OgType::Website => "website",
            OgType::Article => "article",
            OgType::Product => "product",
            OgType::Profile => "profile",
            OgType::Video => "video.other",
            OgType::Music => "music.song",
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum TwitterCard {
    Summary,
    SummaryLargeImage,
    Player,
}

impl TwitterCard {
    pub fn as_str(&self) -> &str {
        match self {
            TwitterCard::Summary => "summary",
            TwitterCard::SummaryLargeImage => "summary_large_image",
            TwitterCard::Player => "player",
        }
    }
}

impl OpenGraph {
    pub fn new(title: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            description: description.into(),
            url: None,
            image: None,
            image_alt: None,
            og_type: OgType::Website,
            site_name: None,
            locale: None,
            twitter_card: TwitterCard::SummaryLargeImage,
            twitter_site: None,
            twitter_creator: None,
        }
    }

    pub fn url(mut self, url: impl Into<String>) -> Self { self.url = Some(url.into()); self }
    pub fn image(mut self, url: impl Into<String>) -> Self { self.image = Some(url.into()); self }
    pub fn image_alt(mut self, alt: impl Into<String>) -> Self { self.image_alt = Some(alt.into()); self }
    pub fn og_type(mut self, t: OgType) -> Self { self.og_type = t; self }
    pub fn site_name(mut self, name: impl Into<String>) -> Self { self.site_name = Some(name.into()); self }
    pub fn locale(mut self, locale: impl Into<String>) -> Self { self.locale = Some(locale.into()); self }
    pub fn twitter_card(mut self, card: TwitterCard) -> Self { self.twitter_card = card; self }
    pub fn twitter_site(mut self, handle: impl Into<String>) -> Self { self.twitter_site = Some(handle.into()); self }
    pub fn twitter_creator(mut self, handle: impl Into<String>) -> Self { self.twitter_creator = Some(handle.into()); self }

    /// Generate HTML meta tags
    pub fn to_html(&self) -> String {
        let mut html = String::new();

        // Open Graph
        html.push_str(&format!("<meta property=\"og:title\" content=\"{}\">\n", html_escape(&self.title)));
        html.push_str(&format!("<meta property=\"og:description\" content=\"{}\">\n", html_escape(&self.description)));
        html.push_str(&format!("<meta property=\"og:type\" content=\"{}\">\n", self.og_type.as_str()));

        if let Some(ref url) = self.url {
            html.push_str(&format!("<meta property=\"og:url\" content=\"{}\">\n", url));
        }
        if let Some(ref image) = self.image {
            html.push_str(&format!("<meta property=\"og:image\" content=\"{}\">\n", image));
        }
        if let Some(ref alt) = self.image_alt {
            html.push_str(&format!("<meta property=\"og:image:alt\" content=\"{}\">\n", html_escape(alt)));
        }
        if let Some(ref name) = self.site_name {
            html.push_str(&format!("<meta property=\"og:site_name\" content=\"{}\">\n", html_escape(name)));
        }
        if let Some(ref locale) = self.locale {
            html.push_str(&format!("<meta property=\"og:locale\" content=\"{}\">\n", locale));
        }

        // Twitter Card
        html.push_str(&format!("<meta name=\"twitter:card\" content=\"{}\">\n", self.twitter_card.as_str()));
        html.push_str(&format!("<meta name=\"twitter:title\" content=\"{}\">\n", html_escape(&self.title)));
        html.push_str(&format!("<meta name=\"twitter:description\" content=\"{}\">\n", html_escape(&self.description)));

        if let Some(ref image) = self.image {
            html.push_str(&format!("<meta name=\"twitter:image\" content=\"{}\">\n", image));
        }
        if let Some(ref site) = self.twitter_site {
            html.push_str(&format!("<meta name=\"twitter:site\" content=\"{}\">\n", site));
        }
        if let Some(ref creator) = self.twitter_creator {
            html.push_str(&format!("<meta name=\"twitter:creator\" content=\"{}\">\n", creator));
        }

        html
    }
}

// ─── JSON-LD Structured Data ────────────────────────────────

/// JSON-LD structured data generator for SEO
#[derive(Debug, Clone)]
pub struct JsonLd {
    entries: Vec<String>,
}

impl JsonLd {
    pub fn new() -> Self {
        Self { entries: Vec::new() }
    }

    /// Website schema
    pub fn website(name: &str, url: &str) -> String {
        format!(
            "{{\"@context\":\"https://schema.org\",\"@type\":\"WebSite\",\"name\":\"{}\",\"url\":\"{}\"}}",
            json_escape(name), json_escape(url)
        )
    }

    /// Article schema
    pub fn article(title: &str, author: &str, date: &str, image: Option<&str>) -> String {
        let mut json = format!(
            "{{\"@context\":\"https://schema.org\",\"@type\":\"Article\",\"headline\":\"{}\",\"author\":{{\"@type\":\"Person\",\"name\":\"{}\"}},\"datePublished\":\"{}\"",
            json_escape(title), json_escape(author), date
        );
        if let Some(img) = image {
            json.push_str(&format!(",\"image\":\"{}\"", json_escape(img)));
        }
        json.push('}');
        json
    }

    /// Breadcrumb schema
    pub fn breadcrumbs(items: &[(&str, &str)]) -> String {
        let list: Vec<String> = items.iter().enumerate().map(|(i, (name, url))| {
            format!(
                "{{\"@type\":\"ListItem\",\"position\":{},\"name\":\"{}\",\"item\":\"{}\"}}",
                i + 1, json_escape(name), json_escape(url)
            )
        }).collect();
        format!(
            "{{\"@context\":\"https://schema.org\",\"@type\":\"BreadcrumbList\",\"itemListElement\":[{}]}}",
            list.join(",")
        )
    }

    /// Organization schema
    pub fn organization(name: &str, url: &str, logo: Option<&str>) -> String {
        let mut json = format!(
            "{{\"@context\":\"https://schema.org\",\"@type\":\"Organization\",\"name\":\"{}\",\"url\":\"{}\"",
            json_escape(name), json_escape(url)
        );
        if let Some(logo_url) = logo {
            json.push_str(&format!(",\"logo\":\"{}\"", json_escape(logo_url)));
        }
        json.push('}');
        json
    }

    /// Product schema
    pub fn product(name: &str, price: f64, currency: &str, description: Option<&str>) -> String {
        let mut json = format!(
            "{{\"@context\":\"https://schema.org\",\"@type\":\"Product\",\"name\":\"{}\",\"offers\":{{\"@type\":\"Offer\",\"price\":\"{:.2}\",\"priceCurrency\":\"{}\"}}",
            json_escape(name), price, currency
        );
        if let Some(desc) = description {
            json.push_str(&format!(",\"description\":\"{}\"", json_escape(desc)));
        }
        json.push('}');
        json
    }

    /// FAQ schema
    pub fn faq(questions: &[(&str, &str)]) -> String {
        let items: Vec<String> = questions.iter().map(|(q, a)| {
            format!(
                "{{\"@type\":\"Question\",\"name\":\"{}\",\"acceptedAnswer\":{{\"@type\":\"Answer\",\"text\":\"{}\"}}}}",
                json_escape(q), json_escape(a)
            )
        }).collect();
        format!(
            "{{\"@context\":\"https://schema.org\",\"@type\":\"FAQPage\",\"mainEntity\":[{}]}}",
            items.join(",")
        )
    }

    /// Add a raw JSON-LD entry
    pub fn add(mut self, json: String) -> Self {
        self.entries.push(json);
        self
    }

    /// Render all JSON-LD entries as <script> tags
    pub fn to_html(&self) -> String {
        self.entries
            .iter()
            .map(|e| format!("<script type=\"application/ld+json\">{}</script>", e))
            .collect::<Vec<_>>()
            .join("\n")
    }
}

impl Default for JsonLd {
    fn default() -> Self {
        Self::new()
    }
}

// ─── Robots.txt ─────────────────────────────────────────────

/// robots.txt generator
#[derive(Debug, Clone)]
pub struct RobotsTxt {
    pub rules: Vec<RobotsRule>,
    pub sitemaps: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct RobotsRule {
    pub user_agent: String,
    pub allow: Vec<String>,
    pub disallow: Vec<String>,
    pub crawl_delay: Option<u32>,
}

impl RobotsTxt {
    pub fn new() -> Self {
        Self {
            rules: Vec::new(),
            sitemaps: Vec::new(),
        }
    }

    pub fn allow_all() -> Self {
        Self {
            rules: vec![RobotsRule {
                user_agent: "*".to_string(),
                allow: vec!["/".to_string()],
                disallow: Vec::new(),
                crawl_delay: None,
            }],
            sitemaps: Vec::new(),
        }
    }

    pub fn add_rule(mut self, rule: RobotsRule) -> Self {
        self.rules.push(rule);
        self
    }

    pub fn sitemap(mut self, url: impl Into<String>) -> Self {
        self.sitemaps.push(url.into());
        self
    }

    pub fn build(&self) -> String {
        let mut txt = String::new();
        for rule in &self.rules {
            txt.push_str(&format!("User-agent: {}\n", rule.user_agent));
            for path in &rule.allow {
                txt.push_str(&format!("Allow: {}\n", path));
            }
            for path in &rule.disallow {
                txt.push_str(&format!("Disallow: {}\n", path));
            }
            if let Some(delay) = rule.crawl_delay {
                txt.push_str(&format!("Crawl-delay: {}\n", delay));
            }
            txt.push('\n');
        }
        for sitemap in &self.sitemaps {
            txt.push_str(&format!("Sitemap: {}\n", sitemap));
        }
        txt
    }
}

impl Default for RobotsTxt {
    fn default() -> Self {
        Self::new()
    }
}

impl RobotsRule {
    pub fn new(user_agent: impl Into<String>) -> Self {
        Self {
            user_agent: user_agent.into(),
            allow: Vec::new(),
            disallow: Vec::new(),
            crawl_delay: None,
        }
    }

    pub fn allow(mut self, path: impl Into<String>) -> Self {
        self.allow.push(path.into());
        self
    }

    pub fn disallow(mut self, path: impl Into<String>) -> Self {
        self.disallow.push(path.into());
        self
    }

    pub fn crawl_delay(mut self, seconds: u32) -> Self {
        self.crawl_delay = Some(seconds);
        self
    }
}

// ─── Canonical URL ──────────────────────────────────────────

/// Generate canonical link tag
pub fn canonical_tag(url: &str) -> String {
    format!("<link rel=\"canonical\" href=\"{}\">", url)
}

/// Generate alternate hreflang tags for i18n
pub fn hreflang_tags(urls: &[(&str, &str)]) -> String {
    urls.iter()
        .map(|(lang, url)| {
            format!("<link rel=\"alternate\" hreflang=\"{}\" href=\"{}\">", lang, url)
        })
        .collect::<Vec<_>>()
        .join("\n")
}

// ─── Helpers ────────────────────────────────────────────────

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn json_escape(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
}

// ─── Tests ──────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sitemap_basic() {
        let sitemap = Sitemap::new("https://example.com")
            .add("/", Priority::Highest, ChangeFreq::Daily)
            .add("/about", Priority::Medium, ChangeFreq::Monthly);
        let xml = sitemap.build();
        assert!(xml.contains("<loc>https://example.com/</loc>"));
        assert!(xml.contains("<loc>https://example.com/about</loc>"));
        assert!(xml.contains("<priority>1.0</priority>"));
        assert!(xml.contains("<changefreq>daily</changefreq>"));
    }

    #[test]
    fn test_sitemap_with_date() {
        let sitemap = Sitemap::new("https://example.com")
            .add_with_date("/blog/post1", Priority::High, ChangeFreq::Weekly, "2025-01-15");
        let xml = sitemap.build();
        assert!(xml.contains("<lastmod>2025-01-15</lastmod>"));
    }

    #[test]
    fn test_sitemap_index() {
        let xml = Sitemap::build_index("https://example.com", &["/sitemap-1.xml", "/sitemap-2.xml"]);
        assert!(xml.contains("<sitemapindex"));
        assert!(xml.contains("sitemap-1.xml"));
        assert!(xml.contains("sitemap-2.xml"));
    }

    #[test]
    fn test_rss_feed() {
        let feed = RssFeed::new("My Blog", "https://example.com", "Blog description")
            .language("en")
            .add_item(
                RssItem::new("Post 1", "https://example.com/post1", "First post")
                    .pub_date("Mon, 01 Jan 2025 00:00:00 GMT")
                    .author("author@example.com")
                    .category("tech"),
            );
        let xml = feed.build();
        assert!(xml.contains("<title>My Blog</title>"));
        assert!(xml.contains("<title>Post 1</title>"));
        assert!(xml.contains("<language>en</language>"));
        assert!(xml.contains("<category>tech</category>"));
    }

    #[test]
    fn test_rss_escaping() {
        let feed = RssFeed::new("Blog & More", "https://example.com", "A <b>bold</b> blog")
            .add_item(RssItem::new("Title & Stuff", "https://example.com/1", "Desc"));
        let xml = feed.build();
        assert!(xml.contains("Blog &amp; More"));
        assert!(xml.contains("Title &amp; Stuff"));
    }

    #[test]
    fn test_open_graph() {
        let og = OpenGraph::new("My Page", "A great page")
            .url("https://example.com")
            .image("https://example.com/og.jpg")
            .twitter_site("@mysite");
        let html = og.to_html();
        assert!(html.contains("og:title"));
        assert!(html.contains("My Page"));
        assert!(html.contains("og:image"));
        assert!(html.contains("twitter:card"));
        assert!(html.contains("@mysite"));
    }

    #[test]
    fn test_og_article() {
        let og = OpenGraph::new("Article Title", "Desc")
            .og_type(OgType::Article)
            .locale("ja_JP");
        let html = og.to_html();
        assert!(html.contains("article"));
        assert!(html.contains("ja_JP"));
    }

    #[test]
    fn test_json_ld_website() {
        let json = JsonLd::website("My Site", "https://example.com");
        assert!(json.contains("\"WebSite\""));
        assert!(json.contains("My Site"));
    }

    #[test]
    fn test_json_ld_article() {
        let json = JsonLd::article("Title", "Author", "2025-01-01", Some("https://img.jpg"));
        assert!(json.contains("\"Article\""));
        assert!(json.contains("Author"));
        assert!(json.contains("2025-01-01"));
    }

    #[test]
    fn test_json_ld_breadcrumbs() {
        let json = JsonLd::breadcrumbs(&[("Home", "https://example.com"), ("Blog", "https://example.com/blog")]);
        assert!(json.contains("BreadcrumbList"));
        assert!(json.contains("\"position\":1"));
        assert!(json.contains("\"position\":2"));
    }

    #[test]
    fn test_json_ld_faq() {
        let json = JsonLd::faq(&[("What is Rust?", "A systems language")]);
        assert!(json.contains("FAQPage"));
        assert!(json.contains("What is Rust?"));
    }

    #[test]
    fn test_json_ld_product() {
        let json = JsonLd::product("Widget", 29.99, "USD", Some("A cool widget"));
        assert!(json.contains("\"Product\""));
        assert!(json.contains("29.99"));
        assert!(json.contains("USD"));
    }

    #[test]
    fn test_json_ld_to_html() {
        let ld = JsonLd::new()
            .add(JsonLd::website("Site", "https://example.com"))
            .add(JsonLd::organization("Org", "https://org.com", None));
        let html = ld.to_html();
        assert_eq!(html.matches("application/ld+json").count(), 2);
    }

    #[test]
    fn test_robots_txt() {
        let robots = RobotsTxt::new()
            .add_rule(
                RobotsRule::new("*")
                    .allow("/")
                    .disallow("/admin/")
                    .crawl_delay(10),
            )
            .sitemap("https://example.com/sitemap.xml");
        let txt = robots.build();
        assert!(txt.contains("User-agent: *"));
        assert!(txt.contains("Allow: /"));
        assert!(txt.contains("Disallow: /admin/"));
        assert!(txt.contains("Crawl-delay: 10"));
        assert!(txt.contains("Sitemap: https://example.com/sitemap.xml"));
    }

    #[test]
    fn test_robots_allow_all() {
        let robots = RobotsTxt::allow_all();
        let txt = robots.build();
        assert!(txt.contains("Allow: /"));
    }

    #[test]
    fn test_canonical_tag() {
        let tag = canonical_tag("https://example.com/page");
        assert!(tag.contains("rel=\"canonical\""));
        assert!(tag.contains("https://example.com/page"));
    }

    #[test]
    fn test_hreflang_tags() {
        let tags = hreflang_tags(&[("en", "https://example.com/en"), ("ja", "https://example.com/ja")]);
        assert!(tags.contains("hreflang=\"en\""));
        assert!(tags.contains("hreflang=\"ja\""));
    }
}
