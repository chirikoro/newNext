//! File Upload Handling for Hayabusa.
//!
//! Multipart form data parsing, file validation, storage backends,
//! and streaming upload support.
//!
//! ```ignore
//! use hayabusa_core::prelude::*;
//!
//! let config = UploadConfig::new()
//!     .max_size(10 * 1024 * 1024) // 10MB
//!     .allowed_types(&["image/jpeg", "image/png", "application/pdf"])
//!     .upload_dir("uploads/");
//! ```

use std::collections::HashMap;
use std::path::{Path, PathBuf};

// ─── Upload Config ──────────────────────────────────────────

/// Upload configuration and validation rules
#[derive(Debug, Clone)]
pub struct UploadConfig {
    /// Maximum file size in bytes
    pub max_size: usize,
    /// Maximum total upload size (all files combined)
    pub max_total_size: usize,
    /// Maximum number of files per request
    pub max_files: usize,
    /// Allowed MIME types (empty = allow all)
    pub allowed_types: Vec<String>,
    /// Allowed file extensions (empty = allow all)
    pub allowed_extensions: Vec<String>,
    /// Upload destination directory
    pub upload_dir: PathBuf,
    /// Whether to generate unique filenames
    pub unique_names: bool,
    /// Whether to preserve original filenames
    pub preserve_names: bool,
}

impl Default for UploadConfig {
    fn default() -> Self {
        Self {
            max_size: 5 * 1024 * 1024,      // 5MB
            max_total_size: 50 * 1024 * 1024, // 50MB
            max_files: 10,
            allowed_types: Vec::new(),
            allowed_extensions: Vec::new(),
            upload_dir: PathBuf::from("uploads"),
            unique_names: true,
            preserve_names: false,
        }
    }
}

impl UploadConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn max_size(mut self, bytes: usize) -> Self {
        self.max_size = bytes;
        self
    }

    pub fn max_total_size(mut self, bytes: usize) -> Self {
        self.max_total_size = bytes;
        self
    }

    pub fn max_files(mut self, n: usize) -> Self {
        self.max_files = n;
        self
    }

    pub fn allowed_types(mut self, types: &[&str]) -> Self {
        self.allowed_types = types.iter().map(|s| s.to_string()).collect();
        self
    }

    pub fn allowed_extensions(mut self, exts: &[&str]) -> Self {
        self.allowed_extensions = exts.iter().map(|s| s.to_string()).collect();
        self
    }

    pub fn upload_dir(mut self, dir: impl Into<PathBuf>) -> Self {
        self.upload_dir = dir.into();
        self
    }

    pub fn unique_names(mut self, enabled: bool) -> Self {
        self.unique_names = enabled;
        self
    }

    /// Images only preset
    pub fn images_only() -> Self {
        Self::new()
            .allowed_types(&["image/jpeg", "image/png", "image/gif", "image/webp", "image/avif"])
            .allowed_extensions(&["jpg", "jpeg", "png", "gif", "webp", "avif"])
    }

    /// Documents preset
    pub fn documents() -> Self {
        Self::new()
            .allowed_types(&["application/pdf", "text/plain", "application/msword",
                "application/vnd.openxmlformats-officedocument.wordprocessingml.document"])
            .allowed_extensions(&["pdf", "txt", "doc", "docx"])
    }
}

// ─── Uploaded File ──────────────────────────────────────────

/// Represents an uploaded file
#[derive(Debug, Clone)]
pub struct UploadedFile {
    /// Original filename from the client
    pub original_name: String,
    /// Saved filename (may be different if unique_names is enabled)
    pub saved_name: String,
    /// MIME content type
    pub content_type: String,
    /// File size in bytes
    pub size: usize,
    /// Full path where the file is saved
    pub path: PathBuf,
    /// Form field name
    pub field_name: String,
    /// File extension
    pub extension: String,
}

impl UploadedFile {
    /// Human-readable file size
    pub fn size_human(&self) -> String {
        format_file_size(self.size)
    }

    /// Check if the file is an image
    pub fn is_image(&self) -> bool {
        self.content_type.starts_with("image/")
    }

    /// Check if the file is a video
    pub fn is_video(&self) -> bool {
        self.content_type.starts_with("video/")
    }

    /// Check if the file is a PDF
    pub fn is_pdf(&self) -> bool {
        self.content_type == "application/pdf"
    }

    /// Get the URL path for serving this file
    pub fn url_path(&self, base: &str) -> String {
        format!("{}/{}", base.trim_end_matches('/'), self.saved_name)
    }
}

// ─── Multipart Parser ───────────────────────────────────────

/// Parse multipart form data boundary from Content-Type header
pub fn parse_boundary(content_type: &str) -> Option<String> {
    content_type
        .split(';')
        .find_map(|part| {
            let part = part.trim();
            if part.starts_with("boundary=") {
                Some(part[9..].trim_matches('"').to_string())
            } else {
                None
            }
        })
}

/// Parse a multipart form body into parts
pub fn parse_multipart(body: &[u8], boundary: &str) -> Vec<MultipartPart> {
    let delimiter = format!("--{}", boundary);
    let end_delimiter = format!("--{}--", boundary);

    let body_str = String::from_utf8_lossy(body);
    let mut parts = Vec::new();

    for section in body_str.split(&delimiter) {
        let section = section.trim();
        if section.is_empty() || section.starts_with("--") {
            continue;
        }
        if section == end_delimiter.trim_start_matches(&delimiter) {
            break;
        }

        // Split headers from body
        if let Some(header_end) = section.find("\r\n\r\n") {
            let headers = &section[..header_end];
            let content = &section[header_end + 4..];

            let mut name = None;
            let mut filename = None;
            let mut content_type = "application/octet-stream".to_string();

            for line in headers.lines() {
                let line = line.trim();
                if line.to_lowercase().starts_with("content-disposition:") {
                    name = extract_header_param(line, "name");
                    filename = extract_header_param(line, "filename");
                } else if line.to_lowercase().starts_with("content-type:") {
                    content_type = line.split(':').nth(1).unwrap_or("").trim().to_string();
                }
            }

            parts.push(MultipartPart {
                name: name.unwrap_or_default(),
                filename,
                content_type,
                data: content.trim_end_matches("\r\n").as_bytes().to_vec(),
            });
        }
    }

    parts
}

/// A single part of a multipart form
#[derive(Debug, Clone)]
pub struct MultipartPart {
    pub name: String,
    pub filename: Option<String>,
    pub content_type: String,
    pub data: Vec<u8>,
}

impl MultipartPart {
    pub fn is_file(&self) -> bool {
        self.filename.is_some()
    }

    pub fn as_str(&self) -> Option<&str> {
        std::str::from_utf8(&self.data).ok()
    }

    pub fn extension(&self) -> Option<String> {
        self.filename.as_ref().and_then(|f| {
            f.rsplit('.').next().map(|e| e.to_lowercase())
        })
    }
}

fn extract_header_param(header: &str, param: &str) -> Option<String> {
    let pattern = format!("{}=\"", param);
    let start = header.find(&pattern)? + pattern.len();
    let end = header[start..].find('"')? + start;
    Some(header[start..end].to_string())
}

// ─── File Validation ────────────────────────────────────────

/// Validate an uploaded file against config rules
pub fn validate_upload(file: &MultipartPart, config: &UploadConfig) -> Result<(), UploadError> {
    // Check file size
    if file.data.len() > config.max_size {
        return Err(UploadError::TooLarge {
            size: file.data.len(),
            max: config.max_size,
        });
    }

    // Check MIME type
    if !config.allowed_types.is_empty() {
        if !config.allowed_types.iter().any(|t| t == &file.content_type) {
            return Err(UploadError::InvalidType {
                got: file.content_type.clone(),
                allowed: config.allowed_types.clone(),
            });
        }
    }

    // Check extension
    if !config.allowed_extensions.is_empty() {
        if let Some(ext) = file.extension() {
            if !config.allowed_extensions.iter().any(|e| e == &ext) {
                return Err(UploadError::InvalidExtension {
                    got: ext,
                    allowed: config.allowed_extensions.clone(),
                });
            }
        }
    }

    Ok(())
}

/// Validate all files in a multipart upload
pub fn validate_uploads(parts: &[MultipartPart], config: &UploadConfig) -> Result<(), UploadError> {
    let files: Vec<&MultipartPart> = parts.iter().filter(|p| p.is_file()).collect();

    if files.len() > config.max_files {
        return Err(UploadError::TooManyFiles {
            count: files.len(),
            max: config.max_files,
        });
    }

    let total_size: usize = files.iter().map(|f| f.data.len()).sum();
    if total_size > config.max_total_size {
        return Err(UploadError::TotalTooLarge {
            size: total_size,
            max: config.max_total_size,
        });
    }

    for file in &files {
        validate_upload(file, config)?;
    }

    Ok(())
}

// ─── Filename Generation ────────────────────────────────────

/// Generate a unique filename
pub fn unique_filename(original: &str) -> String {
    let ext = Path::new(original)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("");
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    if ext.is_empty() {
        format!("{:x}_{:x}", ts.as_secs(), ts.subsec_nanos())
    } else {
        format!("{:x}_{:x}.{}", ts.as_secs(), ts.subsec_nanos(), ext)
    }
}

/// Sanitize a filename (remove path traversal, special chars)
pub fn sanitize_filename(name: &str) -> String {
    let name = name
        .replace("..", "")
        .replace('/', "")
        .replace('\\', "");
    let name: String = name
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '.' || *c == '-' || *c == '_')
        .collect();
    if name.is_empty() {
        "unnamed".to_string()
    } else {
        name
    }
}

// ─── Upload Errors ──────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum UploadError {
    TooLarge { size: usize, max: usize },
    TotalTooLarge { size: usize, max: usize },
    TooManyFiles { count: usize, max: usize },
    InvalidType { got: String, allowed: Vec<String> },
    InvalidExtension { got: String, allowed: Vec<String> },
    IoError(String),
}

impl std::fmt::Display for UploadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UploadError::TooLarge { size, max } => write!(f, "File too large: {} > {}", format_file_size(*size), format_file_size(*max)),
            UploadError::TotalTooLarge { size, max } => write!(f, "Total upload too large: {} > {}", format_file_size(*size), format_file_size(*max)),
            UploadError::TooManyFiles { count, max } => write!(f, "Too many files: {} > {}", count, max),
            UploadError::InvalidType { got, allowed } => write!(f, "Invalid type '{}'. Allowed: {}", got, allowed.join(", ")),
            UploadError::InvalidExtension { got, allowed } => write!(f, "Invalid extension '{}'. Allowed: {}", got, allowed.join(", ")),
            UploadError::IoError(msg) => write!(f, "IO error: {}", msg),
        }
    }
}

/// Upload HTML form helper
pub fn upload_form(action: &str, field_name: &str, multiple: bool, accept: Option<&str>) -> String {
    let multiple_attr = if multiple { " multiple" } else { "" };
    let accept_attr = accept.map_or(String::new(), |a| format!(" accept=\"{}\"", a));
    format!(
        r#"<form method="POST" action="{}" enctype="multipart/form-data">
  <input type="file" name="{}"{}{}>
  <button type="submit">Upload</button>
</form>"#,
        action, field_name, multiple_attr, accept_attr
    )
}

// ─── Helpers ────────────────────────────────────────────────

fn format_file_size(bytes: usize) -> String {
    if bytes < 1024 {
        format!("{}B", bytes)
    } else if bytes < 1024 * 1024 {
        format!("{:.1}KB", bytes as f64 / 1024.0)
    } else if bytes < 1024 * 1024 * 1024 {
        format!("{:.1}MB", bytes as f64 / (1024.0 * 1024.0))
    } else {
        format!("{:.1}GB", bytes as f64 / (1024.0 * 1024.0 * 1024.0))
    }
}

// ─── Tests ──────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_upload_config_default() {
        let config = UploadConfig::new();
        assert_eq!(config.max_size, 5 * 1024 * 1024);
        assert_eq!(config.max_files, 10);
    }

    #[test]
    fn test_upload_config_images() {
        let config = UploadConfig::images_only();
        assert!(config.allowed_types.contains(&"image/jpeg".to_string()));
        assert!(config.allowed_types.contains(&"image/png".to_string()));
    }

    #[test]
    fn test_upload_config_documents() {
        let config = UploadConfig::documents();
        assert!(config.allowed_types.contains(&"application/pdf".to_string()));
    }

    #[test]
    fn test_parse_boundary() {
        let ct = "multipart/form-data; boundary=----WebKitFormBoundary123";
        assert_eq!(parse_boundary(ct), Some("----WebKitFormBoundary123".to_string()));
    }

    #[test]
    fn test_parse_boundary_quoted() {
        let ct = "multipart/form-data; boundary=\"abc123\"";
        assert_eq!(parse_boundary(ct), Some("abc123".to_string()));
    }

    #[test]
    fn test_parse_boundary_none() {
        assert_eq!(parse_boundary("text/plain"), None);
    }

    #[test]
    fn test_multipart_parse() {
        let boundary = "boundary123";
        let body = format!(
            "--{b}\r\nContent-Disposition: form-data; name=\"field1\"\r\n\r\nvalue1\r\n--{b}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"test.txt\"\r\nContent-Type: text/plain\r\n\r\nfile content\r\n--{b}--",
            b = boundary
        );
        let parts = parse_multipart(body.as_bytes(), boundary);
        assert_eq!(parts.len(), 2);
        assert_eq!(parts[0].name, "field1");
        assert!(!parts[0].is_file());
        assert_eq!(parts[1].name, "file");
        assert!(parts[1].is_file());
        assert_eq!(parts[1].filename, Some("test.txt".to_string()));
    }

    #[test]
    fn test_validate_size() {
        let config = UploadConfig::new().max_size(100);
        let part = MultipartPart {
            name: "file".into(),
            filename: Some("big.txt".into()),
            content_type: "text/plain".into(),
            data: vec![0u8; 200],
        };
        assert!(validate_upload(&part, &config).is_err());
    }

    #[test]
    fn test_validate_type() {
        let config = UploadConfig::images_only();
        let part = MultipartPart {
            name: "file".into(),
            filename: Some("doc.pdf".into()),
            content_type: "application/pdf".into(),
            data: vec![0u8; 10],
        };
        assert!(validate_upload(&part, &config).is_err());
    }

    #[test]
    fn test_validate_ok() {
        let config = UploadConfig::images_only();
        let part = MultipartPart {
            name: "file".into(),
            filename: Some("photo.jpg".into()),
            content_type: "image/jpeg".into(),
            data: vec![0u8; 10],
        };
        assert!(validate_upload(&part, &config).is_ok());
    }

    #[test]
    fn test_validate_too_many_files() {
        let config = UploadConfig::new().max_files(1);
        let parts = vec![
            MultipartPart { name: "f".into(), filename: Some("a.txt".into()), content_type: "text/plain".into(), data: vec![0] },
            MultipartPart { name: "f".into(), filename: Some("b.txt".into()), content_type: "text/plain".into(), data: vec![0] },
        ];
        assert!(validate_uploads(&parts, &config).is_err());
    }

    #[test]
    fn test_unique_filename() {
        let name = unique_filename("photo.jpg");
        assert!(name.ends_with(".jpg"));
        assert!(name.len() > 5);
    }

    #[test]
    fn test_sanitize_filename() {
        assert_eq!(sanitize_filename("../../../etc/passwd"), "etcpasswd");
        assert_eq!(sanitize_filename("my file (1).jpg"), "myfile1.jpg");
        assert_eq!(sanitize_filename("normal-file_2.png"), "normal-file_2.png");
        assert_eq!(sanitize_filename(""), "unnamed");
    }

    #[test]
    fn test_upload_form() {
        let html = upload_form("/upload", "file", true, Some("image/*"));
        assert!(html.contains("enctype=\"multipart/form-data\""));
        assert!(html.contains("multiple"));
        assert!(html.contains("accept=\"image/*\""));
    }

    #[test]
    fn test_format_file_size() {
        assert_eq!(format_file_size(500), "500B");
        assert_eq!(format_file_size(1536), "1.5KB");
        assert_eq!(format_file_size(5 * 1024 * 1024), "5.0MB");
    }

    #[test]
    fn test_uploaded_file_helpers() {
        let file = UploadedFile {
            original_name: "photo.jpg".into(),
            saved_name: "abc123.jpg".into(),
            content_type: "image/jpeg".into(),
            size: 1024,
            path: PathBuf::from("/uploads/abc123.jpg"),
            field_name: "avatar".into(),
            extension: "jpg".into(),
        };
        assert!(file.is_image());
        assert!(!file.is_pdf());
        assert_eq!(file.size_human(), "1.0KB");
        assert_eq!(file.url_path("/uploads"), "/uploads/abc123.jpg");
    }

    #[test]
    fn test_multipart_part_extension() {
        let part = MultipartPart {
            name: "file".into(),
            filename: Some("Document.PDF".into()),
            content_type: "application/pdf".into(),
            data: vec![],
        };
        assert_eq!(part.extension(), Some("pdf".to_string()));
    }
}
