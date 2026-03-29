//! Image optimization component for Hayabusa.
//!
//! Provides Next.js `<Image>`-like functionality:
//! - Automatic `srcset` generation for responsive images
//! - Lazy loading with `loading="lazy"`
//! - WebP/AVIF format hints via `<picture>` element
//! - Low-Quality Image Placeholder (LQIP) blur-up effect
//! - Width/height enforcement to prevent Cumulative Layout Shift (CLS)
//! - Priority loading for above-the-fold images

/// Image loading priority
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ImagePriority {
    /// High priority: above-the-fold hero images. Adds `fetchpriority="high"` and `loading="eager"`.
    High,
    /// Normal: default lazy loading. Adds `loading="lazy"`.
    Lazy,
}

impl Default for ImagePriority {
    fn default() -> Self {
        ImagePriority::Lazy
    }
}

/// Image format for `<source>` elements in `<picture>`
#[derive(Debug, Clone, Copy)]
pub enum ImageFormat {
    Avif,
    WebP,
    Original,
}

impl ImageFormat {
    /// Get the MIME type for this format (for <source type="...">)
    pub fn mime_type(&self) -> &'static str {
        match self {
            ImageFormat::Avif => "image/avif",
            ImageFormat::WebP => "image/webp",
            ImageFormat::Original => "",
        }
    }

    fn extension(&self) -> &'static str {
        match self {
            ImageFormat::Avif => "avif",
            ImageFormat::WebP => "webp",
            ImageFormat::Original => "",
        }
    }
}

/// Configuration for an optimized image.
///
/// # Example
/// ```ignore
/// use hayabusa_core::image::OptimizedImage;
///
/// let img = OptimizedImage::new("/images/hero.jpg", 1200, 600)
///     .alt("Hero banner")
///     .priority(ImagePriority::High)
///     .sizes("(max-width: 768px) 100vw, 1200px")
///     .placeholder("/images/hero-blur.jpg");
///
/// let html = img.render();
/// ```
#[derive(Debug, Clone)]
pub struct OptimizedImage {
    pub src: String,
    pub width: u32,
    pub height: u32,
    pub alt: String,
    pub priority: ImagePriority,
    pub sizes: Option<String>,
    pub class: Option<String>,
    pub placeholder: Option<String>,
    /// Custom srcset breakpoints (default: 640, 750, 828, 1080, 1200, 1920, 2048)
    pub breakpoints: Vec<u32>,
    /// Whether to generate modern format sources (<picture> with WebP/AVIF)
    pub modern_formats: bool,
}

/// Default responsive breakpoints (same as Next.js)
const DEFAULT_BREAKPOINTS: &[u32] = &[640, 750, 828, 1080, 1200, 1920, 2048];

impl OptimizedImage {
    pub fn new(src: impl Into<String>, width: u32, height: u32) -> Self {
        Self {
            src: src.into(),
            width,
            height,
            alt: String::new(),
            priority: ImagePriority::default(),
            sizes: None,
            class: None,
            placeholder: None,
            breakpoints: DEFAULT_BREAKPOINTS.to_vec(),
            modern_formats: true,
        }
    }

    pub fn alt(mut self, alt: impl Into<String>) -> Self {
        self.alt = alt.into();
        self
    }

    pub fn priority(mut self, priority: ImagePriority) -> Self {
        self.priority = priority;
        self
    }

    pub fn sizes(mut self, sizes: impl Into<String>) -> Self {
        self.sizes = Some(sizes.into());
        self
    }

    pub fn class(mut self, class: impl Into<String>) -> Self {
        self.class = Some(class.into());
        self
    }

    pub fn placeholder(mut self, placeholder_src: impl Into<String>) -> Self {
        self.placeholder = Some(placeholder_src.into());
        self
    }

    pub fn breakpoints(mut self, bp: Vec<u32>) -> Self {
        self.breakpoints = bp;
        self
    }

    pub fn modern_formats(mut self, enabled: bool) -> Self {
        self.modern_formats = enabled;
        self
    }

    /// Generate the srcset string for responsive images.
    /// Only includes breakpoints <= 2x the specified width.
    fn generate_srcset(&self, format: Option<ImageFormat>) -> String {
        let max_width = self.width * 2;
        let relevant: Vec<u32> = self
            .breakpoints
            .iter()
            .copied()
            .filter(|&bp| bp <= max_width)
            .collect();

        if relevant.is_empty() {
            return String::new();
        }

        relevant
            .iter()
            .map(|&w| {
                let url = self.format_url(w, format);
                format!("{url} {w}w")
            })
            .collect::<Vec<_>>()
            .join(", ")
    }

    /// Format the image URL with width and optional format parameters.
    /// Uses query parameters that can be handled by an image optimization proxy.
    fn format_url(&self, width: u32, format: Option<ImageFormat>) -> String {
        let mut url = self.src.clone();
        let sep = if url.contains('?') { '&' } else { '?' };

        url.push_str(&format!("{sep}w={width}"));

        if let Some(fmt) = format {
            match fmt {
                ImageFormat::Original => {}
                _ => url.push_str(&format!("&f={}", fmt.extension())),
            }
        }

        url
    }

    /// Render the optimized image as HTML.
    ///
    /// High-priority images get:
    /// - `fetchpriority="high"` for browser resource prioritization
    /// - `loading="eager"` (no lazy loading)
    /// - A `<link rel="preload">` hint should be added to HeadContext
    ///
    /// Lazy images get:
    /// - `loading="lazy"` for native lazy loading
    /// - `decoding="async"` for non-blocking decode
    ///
    /// When `modern_formats` is true, renders a `<picture>` element with
    /// AVIF and WebP `<source>` elements for modern browsers.
    pub fn render(&self) -> String {
        let sizes = self.sizes.as_deref().unwrap_or("100vw");
        let class_attr = self
            .class
            .as_ref()
            .map(|c| format!(" class=\"{c}\""))
            .unwrap_or_default();
        let aspect_ratio = format!("{}/{}", self.width, self.height);

        let (loading, fetchpriority, decoding) = match self.priority {
            ImagePriority::High => ("eager", " fetchpriority=\"high\"", "auto"),
            ImagePriority::Lazy => ("lazy", "", "async"),
        };

        let placeholder_style = self
            .placeholder
            .as_ref()
            .map(|p| {
                format!(
                    " style=\"background-image:url('{p}');background-size:cover;background-repeat:no-repeat\"",
                )
            })
            .unwrap_or_default();

        if self.modern_formats {
            let avif_srcset = self.generate_srcset(Some(ImageFormat::Avif));
            let webp_srcset = self.generate_srcset(Some(ImageFormat::WebP));
            let orig_srcset = self.generate_srcset(None);

            let mut html = format!("<picture{class_attr}{placeholder_style}>");

            if !avif_srcset.is_empty() {
                html.push_str(&format!(
                    "<source type=\"image/avif\" srcset=\"{avif_srcset}\" sizes=\"{sizes}\" />"
                ));
            }
            if !webp_srcset.is_empty() {
                html.push_str(&format!(
                    "<source type=\"image/webp\" srcset=\"{webp_srcset}\" sizes=\"{sizes}\" />"
                ));
            }

            html.push_str(&format!(
                "<img src=\"{src}\" width=\"{w}\" height=\"{h}\" alt=\"{alt}\" \
                loading=\"{loading}\"{fetchpriority} decoding=\"{decoding}\" \
                sizes=\"{sizes}\" srcset=\"{orig_srcset}\" \
                style=\"aspect-ratio:{aspect_ratio};max-width:100%;height:auto\" /></picture>",
                src = html_escape(&self.src),
                w = self.width,
                h = self.height,
                alt = html_escape(&self.alt),
                loading = loading,
                fetchpriority = fetchpriority,
                decoding = decoding,
                sizes = sizes,
                orig_srcset = orig_srcset,
                aspect_ratio = aspect_ratio,
            ));

            html
        } else {
            let srcset = self.generate_srcset(None);

            format!(
                "<img src=\"{src}\" width=\"{w}\" height=\"{h}\" alt=\"{alt}\" \
                loading=\"{loading}\"{fetchpriority} decoding=\"{decoding}\" \
                sizes=\"{sizes}\" srcset=\"{srcset}\" \
                style=\"aspect-ratio:{aspect_ratio};max-width:100%;height:auto\"{class_attr}{placeholder_style} />",
                src = html_escape(&self.src),
                w = self.width,
                h = self.height,
                alt = html_escape(&self.alt),
                loading = loading,
                fetchpriority = fetchpriority,
                decoding = decoding,
                sizes = sizes,
                srcset = srcset,
                aspect_ratio = aspect_ratio,
                class_attr = class_attr,
                placeholder_style = placeholder_style,
            )
        }
    }

    /// Generate a preload link entry for high-priority images.
    /// Should be added to HeadContext for above-the-fold images.
    pub fn preload_link(&self) -> Option<String> {
        if self.priority == ImagePriority::High {
            Some(format!(
                "<link rel=\"preload\" as=\"image\" href=\"{}\" imagesrcset=\"{}\" imagesizes=\"{}\" />",
                html_escape(&self.src),
                self.generate_srcset(Some(ImageFormat::WebP)),
                self.sizes.as_deref().unwrap_or("100vw"),
            ))
        } else {
            None
        }
    }
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_image_render() {
        let img = OptimizedImage::new("/photo.jpg", 800, 600)
            .alt("A photo")
            .modern_formats(false);
        let html = img.render();

        assert!(html.contains("loading=\"lazy\""));
        assert!(html.contains("decoding=\"async\""));
        assert!(html.contains("width=\"800\""));
        assert!(html.contains("height=\"600\""));
        assert!(html.contains("alt=\"A photo\""));
        assert!(html.contains("aspect-ratio:800/600"));
        assert!(!html.contains("fetchpriority"));
    }

    #[test]
    fn test_high_priority_image() {
        let img = OptimizedImage::new("/hero.jpg", 1200, 600)
            .alt("Hero")
            .priority(ImagePriority::High)
            .modern_formats(false);
        let html = img.render();

        assert!(html.contains("loading=\"eager\""));
        assert!(html.contains("fetchpriority=\"high\""));
        assert!(!html.contains("decoding=\"async\""));
    }

    #[test]
    fn test_picture_element_with_modern_formats() {
        let img = OptimizedImage::new("/photo.jpg", 800, 600)
            .alt("Photo")
            .modern_formats(true);
        let html = img.render();

        assert!(html.contains("<picture"));
        assert!(html.contains("type=\"image/avif\""));
        assert!(html.contains("type=\"image/webp\""));
        assert!(html.contains("</picture>"));
    }

    #[test]
    fn test_srcset_generation() {
        let img = OptimizedImage::new("/photo.jpg", 800, 600)
            .breakpoints(vec![400, 800, 1200, 1600]);
        let srcset = img.generate_srcset(None);

        assert!(srcset.contains("400w"));
        assert!(srcset.contains("800w"));
        assert!(srcset.contains("1200w"));
        assert!(srcset.contains("1600w"));
    }

    #[test]
    fn test_srcset_filters_by_width() {
        let img = OptimizedImage::new("/photo.jpg", 400, 300)
            .breakpoints(vec![400, 800, 1200, 1600]);
        let srcset = img.generate_srcset(None);

        // max_width = 400 * 2 = 800
        assert!(srcset.contains("400w"));
        assert!(srcset.contains("800w"));
        assert!(!srcset.contains("1200w"));
        assert!(!srcset.contains("1600w"));
    }

    #[test]
    fn test_preload_link_high_priority() {
        let img = OptimizedImage::new("/hero.jpg", 1200, 600)
            .priority(ImagePriority::High);
        assert!(img.preload_link().is_some());

        let img2 = OptimizedImage::new("/thumb.jpg", 200, 200);
        assert!(img2.preload_link().is_none());
    }

    #[test]
    fn test_placeholder_blur() {
        let img = OptimizedImage::new("/photo.jpg", 800, 600)
            .placeholder("/photo-blur.jpg")
            .modern_formats(false);
        let html = img.render();

        assert!(html.contains("background-image:url('/photo-blur.jpg')"));
        assert!(html.contains("background-size:cover"));
    }
}
