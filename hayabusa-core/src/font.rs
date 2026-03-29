//! Font optimization for Hayabusa.
//!
//! Provides automatic font optimization to prevent layout shift (CLS):
//! - `font-display: swap` for instant text rendering
//! - Preload hints for critical fonts
//! - `size-adjust` CSS for fallback font metric matching
//! - Self-hosting helper (avoid third-party DNS lookups)

/// Supported font formats
#[derive(Debug, Clone, Copy)]
pub enum FontFormat {
    Woff2,
    Woff,
    TrueType,
    OpenType,
}

impl FontFormat {
    fn extension(&self) -> &'static str {
        match self {
            FontFormat::Woff2 => "woff2",
            FontFormat::Woff => "woff",
            FontFormat::TrueType => "ttf",
            FontFormat::OpenType => "otf",
        }
    }

    fn mime_type(&self) -> &'static str {
        match self {
            FontFormat::Woff2 => "font/woff2",
            FontFormat::Woff => "font/woff",
            FontFormat::TrueType => "font/ttf",
            FontFormat::OpenType => "font/otf",
        }
    }
}

/// Font weight specification
#[derive(Debug, Clone)]
pub enum FontWeight {
    Normal,
    Bold,
    Specific(u32),
    Range(u32, u32),
}

impl FontWeight {
    fn to_css(&self) -> String {
        match self {
            FontWeight::Normal => "400".to_string(),
            FontWeight::Bold => "700".to_string(),
            FontWeight::Specific(w) => w.to_string(),
            FontWeight::Range(min, max) => format!("{min} {max}"),
        }
    }
}

/// Configuration for an optimized font.
///
/// # Example
/// ```ignore
/// use hayabusa_core::font::OptimizedFont;
///
/// let font = OptimizedFont::new("Inter", "/fonts/inter-var.woff2")
///     .weight(FontWeight::Range(100, 900))
///     .fallback("system-ui, -apple-system, sans-serif")
///     .size_adjust(107.0) // Match fallback metrics
///     .preload(true);
///
/// // Add to <head>
/// let css = font.render_css();
/// let preload_tag = font.render_preload();
/// ```
#[derive(Debug, Clone)]
pub struct OptimizedFont {
    /// Font family name
    pub family: String,
    /// Primary font file URL
    pub src: String,
    /// Font format
    pub format: FontFormat,
    /// Font weight
    pub weight: FontWeight,
    /// Font style (normal, italic)
    pub style: String,
    /// Fallback font stack
    pub fallback: String,
    /// size-adjust percentage for fallback font CLS prevention
    pub size_adjust: Option<f64>,
    /// ascent-override for precise metric matching
    pub ascent_override: Option<f64>,
    /// descent-override for precise metric matching
    pub descent_override: Option<f64>,
    /// line-gap-override for precise metric matching
    pub line_gap_override: Option<f64>,
    /// Whether to add a preload link
    pub preload: bool,
    /// Unicode range to subset (e.g., "U+0000-00FF" for Latin)
    pub unicode_range: Option<String>,
}

impl OptimizedFont {
    pub fn new(family: impl Into<String>, src: impl Into<String>) -> Self {
        Self {
            family: family.into(),
            src: src.into(),
            format: FontFormat::Woff2,
            weight: FontWeight::Normal,
            style: "normal".to_string(),
            fallback: "system-ui, -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif"
                .to_string(),
            size_adjust: None,
            ascent_override: None,
            descent_override: None,
            line_gap_override: None,
            preload: true,
            unicode_range: None,
        }
    }

    pub fn format(mut self, format: FontFormat) -> Self {
        self.format = format;
        self
    }

    pub fn weight(mut self, weight: FontWeight) -> Self {
        self.weight = weight;
        self
    }

    pub fn style(mut self, style: impl Into<String>) -> Self {
        self.style = style.into();
        self
    }

    pub fn fallback(mut self, fallback: impl Into<String>) -> Self {
        self.fallback = fallback.into();
        self
    }

    pub fn size_adjust(mut self, percent: f64) -> Self {
        self.size_adjust = Some(percent);
        self
    }

    pub fn ascent_override(mut self, percent: f64) -> Self {
        self.ascent_override = Some(percent);
        self
    }

    pub fn descent_override(mut self, percent: f64) -> Self {
        self.descent_override = Some(percent);
        self
    }

    pub fn line_gap_override(mut self, percent: f64) -> Self {
        self.line_gap_override = Some(percent);
        self
    }

    pub fn preload(mut self, preload: bool) -> Self {
        self.preload = preload;
        self
    }

    pub fn unicode_range(mut self, range: impl Into<String>) -> Self {
        self.unicode_range = Some(range.into());
        self
    }

    /// Render the @font-face CSS declaration.
    ///
    /// Always uses `font-display: swap` to ensure text is immediately
    /// visible with the fallback font while the web font loads.
    pub fn render_css(&self) -> String {
        let mut css = String::with_capacity(512);

        // Primary @font-face with swap
        css.push_str(&format!(
            "@font-face{{\
            font-family:'{family}';\
            src:url('{src}') format('{format}');\
            font-weight:{weight};\
            font-style:{style};\
            font-display:swap;",
            family = self.family,
            src = self.src,
            format = self.format.extension(),
            weight = self.weight.to_css(),
            style = self.style,
        ));

        if let Some(ref range) = self.unicode_range {
            css.push_str(&format!("unicode-range:{range};"));
        }

        css.push('}');

        // Fallback @font-face with size-adjust for CLS prevention
        if self.size_adjust.is_some()
            || self.ascent_override.is_some()
            || self.descent_override.is_some()
        {
            css.push_str(&format!(
                "@font-face{{font-family:'{} Fallback';src:local('{}');",
                self.family,
                self.fallback.split(',').next().unwrap_or("Arial").trim(),
            ));

            if let Some(sa) = self.size_adjust {
                css.push_str(&format!("size-adjust:{sa:.1}%;"));
            }
            if let Some(ao) = self.ascent_override {
                css.push_str(&format!("ascent-override:{ao:.1}%;"));
            }
            if let Some(d_o) = self.descent_override {
                css.push_str(&format!("descent-override:{d_o:.1}%;"));
            }
            if let Some(lg) = self.line_gap_override {
                css.push_str(&format!("line-gap-override:{lg:.1}%;"));
            }

            css.push('}');
        }

        css
    }

    /// Render the <link rel="preload"> tag for this font.
    pub fn render_preload(&self) -> String {
        if self.preload {
            format!(
                "<link rel=\"preload\" href=\"{src}\" as=\"font\" type=\"{mime}\" crossorigin />",
                src = self.src,
                mime = self.format.mime_type(),
            )
        } else {
            String::new()
        }
    }

    /// Get the CSS font-family value including the fallback stack.
    pub fn font_family_css(&self) -> String {
        if self.size_adjust.is_some() {
            format!("'{f}', '{f} Fallback', {fb}", f = self.family, fb = self.fallback)
        } else {
            format!("'{f}', {fb}", f = self.family, fb = self.fallback)
        }
    }
}

/// Helper to create common Google Font configurations with
/// pre-calculated size-adjust values for popular fonts.
pub fn google_font(name: &str, src: &str) -> OptimizedFont {
    let mut font = OptimizedFont::new(name, src);

    // Pre-calculated size-adjust values for popular Google Fonts
    // These match the system-ui fallback metrics to prevent CLS
    match name {
        "Inter" => {
            font.size_adjust = Some(107.0);
            font.ascent_override = Some(90.0);
            font.descent_override = Some(22.0);
            font.line_gap_override = Some(0.0);
        }
        "Roboto" => {
            font.size_adjust = Some(100.3);
            font.ascent_override = Some(92.7);
            font.descent_override = Some(24.4);
            font.line_gap_override = Some(0.0);
        }
        "Open Sans" => {
            font.size_adjust = Some(105.0);
            font.ascent_override = Some(101.0);
            font.descent_override = Some(27.0);
            font.line_gap_override = Some(0.0);
        }
        "Noto Sans JP" => {
            font.size_adjust = Some(113.0);
            font.ascent_override = Some(98.0);
            font.descent_override = Some(33.0);
            font.line_gap_override = Some(0.0);
        }
        _ => {}
    }

    font
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_font_css_with_swap() {
        let font = OptimizedFont::new("Inter", "/fonts/inter.woff2");
        let css = font.render_css();

        assert!(css.contains("font-display:swap"));
        assert!(css.contains("font-family:'Inter'"));
        assert!(css.contains("url('/fonts/inter.woff2')"));
    }

    #[test]
    fn test_font_size_adjust_fallback() {
        let font = OptimizedFont::new("Inter", "/fonts/inter.woff2")
            .size_adjust(107.0)
            .ascent_override(90.0);
        let css = font.render_css();

        assert!(css.contains("'Inter Fallback'"));
        assert!(css.contains("size-adjust:107.0%"));
        assert!(css.contains("ascent-override:90.0%"));
    }

    #[test]
    fn test_font_preload() {
        let font = OptimizedFont::new("Inter", "/fonts/inter.woff2")
            .preload(true);
        let preload = font.render_preload();

        assert!(preload.contains("rel=\"preload\""));
        assert!(preload.contains("as=\"font\""));
        assert!(preload.contains("crossorigin"));
    }

    #[test]
    fn test_font_family_css() {
        let font = OptimizedFont::new("Inter", "/fonts/inter.woff2")
            .size_adjust(107.0);
        let family = font.font_family_css();

        assert!(family.contains("'Inter'"));
        assert!(family.contains("'Inter Fallback'"));
        assert!(family.contains("system-ui"));
    }

    #[test]
    fn test_google_font_helper() {
        let font = google_font("Inter", "/fonts/inter.woff2");
        assert!(font.size_adjust.is_some());
        assert!(font.ascent_override.is_some());
    }
}
