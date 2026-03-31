//! Email Sending for Hayabusa.
//!
//! Supports SMTP and HTTP API providers (Resend, SendGrid).
//! Template-based email rendering with the built-in template engine.
//!
//! ```ignore
//! use hayabusa_core::prelude::*;
//!
//! let mailer = EmailClient::resend("re_api_key");
//! let email = Email::new()
//!     .to("user@example.com")
//!     .subject("Welcome!")
//!     .html("<h1>Hello</h1>");
//! ```

use std::collections::HashMap;

// ─── Email ──────────────────────────────────────────────────

/// An email message
#[derive(Debug, Clone)]
pub struct Email {
    pub from: Option<String>,
    pub to: Vec<String>,
    pub cc: Vec<String>,
    pub bcc: Vec<String>,
    pub reply_to: Option<String>,
    pub subject: String,
    pub html_body: Option<String>,
    pub text_body: Option<String>,
    pub headers: HashMap<String, String>,
    pub tags: Vec<String>,
}

impl Email {
    pub fn new() -> Self {
        Self {
            from: None,
            to: Vec::new(),
            cc: Vec::new(),
            bcc: Vec::new(),
            reply_to: None,
            subject: String::new(),
            html_body: None,
            text_body: None,
            headers: HashMap::new(),
            tags: Vec::new(),
        }
    }

    pub fn from(mut self, addr: impl Into<String>) -> Self { self.from = Some(addr.into()); self }
    pub fn to(mut self, addr: impl Into<String>) -> Self { self.to.push(addr.into()); self }
    pub fn cc(mut self, addr: impl Into<String>) -> Self { self.cc.push(addr.into()); self }
    pub fn bcc(mut self, addr: impl Into<String>) -> Self { self.bcc.push(addr.into()); self }
    pub fn reply_to(mut self, addr: impl Into<String>) -> Self { self.reply_to = Some(addr.into()); self }
    pub fn subject(mut self, s: impl Into<String>) -> Self { self.subject = s.into(); self }
    pub fn html(mut self, html: impl Into<String>) -> Self { self.html_body = Some(html.into()); self }
    pub fn text(mut self, text: impl Into<String>) -> Self { self.text_body = Some(text.into()); self }
    pub fn tag(mut self, tag: impl Into<String>) -> Self { self.tags.push(tag.into()); self }

    pub fn header(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.insert(key.into(), value.into());
        self
    }

    /// Validate the email has minimum required fields
    pub fn is_valid(&self) -> bool {
        !self.to.is_empty() && !self.subject.is_empty() && (self.html_body.is_some() || self.text_body.is_some())
    }

    /// Convert to JSON body for API providers
    pub fn to_json(&self, default_from: &str) -> String {
        let from = self.from.as_deref().unwrap_or(default_from);
        let to_arr: Vec<String> = self.to.iter().map(|t| format!("\"{}\"", t)).collect();

        let mut json = format!(
            "{{\"from\":\"{}\",\"to\":[{}],\"subject\":\"{}\"",
            json_escape(from),
            to_arr.join(","),
            json_escape(&self.subject)
        );

        if let Some(ref html) = self.html_body {
            json.push_str(&format!(",\"html\":\"{}\"", json_escape(html)));
        }
        if let Some(ref text) = self.text_body {
            json.push_str(&format!(",\"text\":\"{}\"", json_escape(text)));
        }
        if !self.cc.is_empty() {
            let cc: Vec<String> = self.cc.iter().map(|c| format!("\"{}\"", c)).collect();
            json.push_str(&format!(",\"cc\":[{}]", cc.join(",")));
        }
        if !self.bcc.is_empty() {
            let bcc: Vec<String> = self.bcc.iter().map(|c| format!("\"{}\"", c)).collect();
            json.push_str(&format!(",\"bcc\":[{}]", bcc.join(",")));
        }
        if let Some(ref reply) = self.reply_to {
            json.push_str(&format!(",\"reply_to\":\"{}\"", reply));
        }

        json.push('}');
        json
    }

    /// Convert to SMTP MIME format
    pub fn to_mime(&self, default_from: &str) -> String {
        let from = self.from.as_deref().unwrap_or(default_from);
        let mut mime = String::new();

        mime.push_str(&format!("From: {}\r\n", from));
        mime.push_str(&format!("To: {}\r\n", self.to.join(", ")));
        if !self.cc.is_empty() {
            mime.push_str(&format!("Cc: {}\r\n", self.cc.join(", ")));
        }
        mime.push_str(&format!("Subject: {}\r\n", self.subject));
        mime.push_str("MIME-Version: 1.0\r\n");

        for (k, v) in &self.headers {
            mime.push_str(&format!("{}: {}\r\n", k, v));
        }

        if self.html_body.is_some() && self.text_body.is_some() {
            let boundary = "hayabusa_boundary_001";
            mime.push_str(&format!("Content-Type: multipart/alternative; boundary={}\r\n\r\n", boundary));

            if let Some(ref text) = self.text_body {
                mime.push_str(&format!("--{}\r\n", boundary));
                mime.push_str("Content-Type: text/plain; charset=UTF-8\r\n\r\n");
                mime.push_str(text);
                mime.push_str("\r\n");
            }
            if let Some(ref html) = self.html_body {
                mime.push_str(&format!("--{}\r\n", boundary));
                mime.push_str("Content-Type: text/html; charset=UTF-8\r\n\r\n");
                mime.push_str(html);
                mime.push_str("\r\n");
            }
            mime.push_str(&format!("--{}--\r\n", boundary));
        } else if let Some(ref html) = self.html_body {
            mime.push_str("Content-Type: text/html; charset=UTF-8\r\n\r\n");
            mime.push_str(html);
        } else if let Some(ref text) = self.text_body {
            mime.push_str("Content-Type: text/plain; charset=UTF-8\r\n\r\n");
            mime.push_str(text);
        }

        mime
    }
}

impl Default for Email {
    fn default() -> Self {
        Self::new()
    }
}

// ─── Email Client ───────────────────────────────────────────

/// Email provider configuration
#[derive(Debug, Clone)]
pub struct EmailClient {
    pub provider: EmailProvider,
    pub default_from: String,
    pub api_key: Option<String>,
    pub smtp_host: Option<String>,
    pub smtp_port: u16,
    pub smtp_username: Option<String>,
    pub smtp_password: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum EmailProvider {
    Resend,
    SendGrid,
    Smtp,
}

impl EmailClient {
    /// Create a Resend API client
    pub fn resend(api_key: impl Into<String>) -> Self {
        Self {
            provider: EmailProvider::Resend,
            default_from: "noreply@example.com".to_string(),
            api_key: Some(api_key.into()),
            smtp_host: None,
            smtp_port: 587,
            smtp_username: None,
            smtp_password: None,
        }
    }

    /// Create a SendGrid API client
    pub fn sendgrid(api_key: impl Into<String>) -> Self {
        Self {
            provider: EmailProvider::SendGrid,
            default_from: "noreply@example.com".to_string(),
            api_key: Some(api_key.into()),
            smtp_host: None,
            smtp_port: 587,
            smtp_username: None,
            smtp_password: None,
        }
    }

    /// Create an SMTP client
    pub fn smtp(host: impl Into<String>, port: u16) -> Self {
        Self {
            provider: EmailProvider::Smtp,
            default_from: "noreply@example.com".to_string(),
            api_key: None,
            smtp_host: Some(host.into()),
            smtp_port: port,
            smtp_username: None,
            smtp_password: None,
        }
    }

    pub fn from_address(mut self, from: impl Into<String>) -> Self {
        self.default_from = from.into();
        self
    }

    pub fn smtp_credentials(mut self, username: impl Into<String>, password: impl Into<String>) -> Self {
        self.smtp_username = Some(username.into());
        self.smtp_password = Some(password.into());
        self
    }

    /// Get the API endpoint URL for the provider
    pub fn api_url(&self) -> &str {
        match self.provider {
            EmailProvider::Resend => "https://api.resend.com/emails",
            EmailProvider::SendGrid => "https://api.sendgrid.com/v3/mail/send",
            EmailProvider::Smtp => "",
        }
    }

    /// Generate the HTTP request headers for API-based providers
    pub fn api_headers(&self) -> Vec<(String, String)> {
        let mut headers = vec![
            ("Content-Type".to_string(), "application/json".to_string()),
        ];
        if let Some(ref key) = self.api_key {
            match self.provider {
                EmailProvider::Resend => {
                    headers.push(("Authorization".to_string(), format!("Bearer {}", key)));
                }
                EmailProvider::SendGrid => {
                    headers.push(("Authorization".to_string(), format!("Bearer {}", key)));
                }
                EmailProvider::Smtp => {}
            }
        }
        headers
    }
}

// ─── Email Templates ────────────────────────────────────────

/// Pre-built email template
pub fn email_template(title: &str, body_html: &str, footer: Option<&str>) -> String {
    let footer_html = footer.unwrap_or("&copy; Hayabusa App");
    format!(
        r#"<!DOCTYPE html>
<html>
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<title>{title}</title>
<style>
body {{ font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; margin: 0; padding: 0; background: #f5f5f5; }}
.container {{ max-width: 600px; margin: 0 auto; background: #ffffff; }}
.header {{ background: #1a1a2e; color: #ffffff; padding: 24px; text-align: center; }}
.content {{ padding: 32px 24px; line-height: 1.6; color: #333333; }}
.footer {{ padding: 16px 24px; text-align: center; color: #999999; font-size: 12px; border-top: 1px solid #eeeeee; }}
a {{ color: #0070f3; }}
.btn {{ display: inline-block; padding: 12px 24px; background: #0070f3; color: #ffffff; text-decoration: none; border-radius: 6px; }}
</style>
</head>
<body>
<div class="container">
  <div class="header"><h1>{title}</h1></div>
  <div class="content">{body_html}</div>
  <div class="footer">{footer_html}</div>
</div>
</body>
</html>"#,
    )
}

// ─── Helpers ────────────────────────────────────────────────

fn json_escape(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
}

// ─── Tests ──────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_email_builder() {
        let email = Email::new()
            .from("sender@example.com")
            .to("user@example.com")
            .subject("Hello")
            .html("<h1>Hi</h1>");
        assert!(email.is_valid());
        assert_eq!(email.to, vec!["user@example.com"]);
    }

    #[test]
    fn test_email_invalid() {
        let email = Email::new().subject("No body");
        assert!(!email.is_valid());
    }

    #[test]
    fn test_email_to_json() {
        let email = Email::new()
            .to("a@b.com")
            .cc("c@d.com")
            .subject("Test")
            .html("<p>Hi</p>");
        let json = email.to_json("default@example.com");
        assert!(json.contains("\"to\":[\"a@b.com\"]"));
        assert!(json.contains("\"cc\":[\"c@d.com\"]"));
        assert!(json.contains("\"subject\":\"Test\""));
    }

    #[test]
    fn test_email_to_mime() {
        let email = Email::new()
            .to("user@example.com")
            .subject("Hello")
            .text("Plain text")
            .html("<p>HTML</p>");
        let mime = email.to_mime("noreply@example.com");
        assert!(mime.contains("From: noreply@example.com"));
        assert!(mime.contains("multipart/alternative"));
        assert!(mime.contains("text/plain"));
        assert!(mime.contains("text/html"));
    }

    #[test]
    fn test_email_mime_text_only() {
        let email = Email::new()
            .to("user@example.com")
            .subject("Hi")
            .text("Just text");
        let mime = email.to_mime("from@example.com");
        assert!(mime.contains("text/plain"));
        assert!(!mime.contains("multipart"));
    }

    #[test]
    fn test_resend_client() {
        let client = EmailClient::resend("re_key123")
            .from_address("hello@myapp.com");
        assert_eq!(client.provider, EmailProvider::Resend);
        assert_eq!(client.default_from, "hello@myapp.com");
        assert!(client.api_url().contains("resend.com"));
    }

    #[test]
    fn test_sendgrid_client() {
        let client = EmailClient::sendgrid("SG.key");
        assert_eq!(client.provider, EmailProvider::SendGrid);
        assert!(client.api_url().contains("sendgrid.com"));
    }

    #[test]
    fn test_smtp_client() {
        let client = EmailClient::smtp("smtp.gmail.com", 587)
            .smtp_credentials("user", "pass");
        assert_eq!(client.provider, EmailProvider::Smtp);
        assert_eq!(client.smtp_host, Some("smtp.gmail.com".to_string()));
    }

    #[test]
    fn test_api_headers() {
        let client = EmailClient::resend("mykey");
        let headers = client.api_headers();
        assert!(headers.iter().any(|(k, v)| k == "Authorization" && v.contains("mykey")));
    }

    #[test]
    fn test_email_template() {
        let html = email_template("Welcome", "<p>Hello World</p>", Some("My Company"));
        assert!(html.contains("<title>Welcome</title>"));
        assert!(html.contains("Hello World"));
        assert!(html.contains("My Company"));
    }

    #[test]
    fn test_email_multiple_recipients() {
        let email = Email::new()
            .to("a@b.com")
            .to("c@d.com")
            .subject("Batch")
            .text("Hi all");
        assert_eq!(email.to.len(), 2);
    }

    #[test]
    fn test_email_tags() {
        let email = Email::new()
            .to("a@b.com")
            .subject("Tagged")
            .text("Content")
            .tag("welcome")
            .tag("onboarding");
        assert_eq!(email.tags.len(), 2);
    }
}
