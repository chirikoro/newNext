//! OpenAPI / Swagger Support for Hayabusa.
//!
//! Auto-generate OpenAPI 3.0 spec from route definitions,
//! and serve Swagger UI.
//!
//! ```ignore
//! use hayabusa_core::prelude::*;
//!
//! let spec = OpenApiSpec::new("My API", "1.0.0")
//!     .path("/api/users", HttpMethod::Get, ApiEndpoint::new()
//!         .summary("List users")
//!         .response(200, "User list"));
//! ```

use std::collections::HashMap;

/// OpenAPI 3.0 specification builder
#[derive(Debug, Clone)]
pub struct OpenApiSpec {
    pub title: String,
    pub version: String,
    pub description: Option<String>,
    pub servers: Vec<ApiServer>,
    pub paths: Vec<ApiPath>,
    pub schemas: HashMap<String, ApiSchema>,
    pub security_schemes: HashMap<String, SecurityScheme>,
}

#[derive(Debug, Clone)]
pub struct ApiServer {
    pub url: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ApiPath {
    pub path: String,
    pub method: HttpMethod,
    pub endpoint: ApiEndpoint,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum HttpMethod { Get, Post, Put, Patch, Delete, Options, Head }

impl HttpMethod {
    pub fn as_str(&self) -> &str {
        match self {
            HttpMethod::Get => "get", HttpMethod::Post => "post",
            HttpMethod::Put => "put", HttpMethod::Patch => "patch",
            HttpMethod::Delete => "delete", HttpMethod::Options => "options",
            HttpMethod::Head => "head",
        }
    }
}

#[derive(Debug, Clone)]
pub struct ApiEndpoint {
    pub summary: Option<String>,
    pub description: Option<String>,
    pub operation_id: Option<String>,
    pub tags: Vec<String>,
    pub parameters: Vec<ApiParameter>,
    pub request_body: Option<ApiRequestBody>,
    pub responses: Vec<(u16, ApiResponse)>,
    pub security: Vec<String>,
    pub deprecated: bool,
}

#[derive(Debug, Clone)]
pub struct ApiParameter {
    pub name: String,
    pub location: ParamLocation,
    pub required: bool,
    pub schema_type: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Copy)]
pub enum ParamLocation { Query, Path, Header, Cookie }

impl ParamLocation {
    pub fn as_str(&self) -> &str {
        match self { Self::Query => "query", Self::Path => "path", Self::Header => "header", Self::Cookie => "cookie" }
    }
}

#[derive(Debug, Clone)]
pub struct ApiRequestBody {
    pub content_type: String,
    pub schema_ref: Option<String>,
    pub required: bool,
}

#[derive(Debug, Clone)]
pub struct ApiResponse {
    pub description: String,
    pub content_type: Option<String>,
    pub schema_ref: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ApiSchema {
    pub schema_type: String, // "object", "array", "string", etc.
    pub properties: Vec<(String, String, bool)>, // (name, type, required)
    pub description: Option<String>,
}

#[derive(Debug, Clone)]
pub enum SecurityScheme {
    Bearer,
    ApiKey { name: String, location: ParamLocation },
    OAuth2 { authorize_url: String, token_url: String, scopes: Vec<(String, String)> },
}

impl OpenApiSpec {
    pub fn new(title: &str, version: &str) -> Self {
        Self {
            title: title.into(), version: version.into(), description: None,
            servers: Vec::new(), paths: Vec::new(),
            schemas: HashMap::new(), security_schemes: HashMap::new(),
        }
    }

    pub fn description(mut self, desc: &str) -> Self { self.description = Some(desc.into()); self }

    pub fn server(mut self, url: &str, desc: Option<&str>) -> Self {
        self.servers.push(ApiServer { url: url.into(), description: desc.map(Into::into) });
        self
    }

    pub fn path(mut self, path: &str, method: HttpMethod, endpoint: ApiEndpoint) -> Self {
        self.paths.push(ApiPath { path: path.into(), method, endpoint });
        self
    }

    pub fn schema(mut self, name: &str, schema: ApiSchema) -> Self {
        self.schemas.insert(name.into(), schema);
        self
    }

    pub fn security_scheme(mut self, name: &str, scheme: SecurityScheme) -> Self {
        self.security_schemes.insert(name.into(), scheme);
        self
    }

    /// Generate OpenAPI 3.0 JSON
    pub fn to_json(&self) -> String {
        let mut json = String::from("{\n  \"openapi\": \"3.0.3\",\n  \"info\": {\n");
        json.push_str(&format!("    \"title\": \"{}\",\n", json_escape(&self.title)));
        json.push_str(&format!("    \"version\": \"{}\"", self.version));
        if let Some(ref desc) = self.description {
            json.push_str(&format!(",\n    \"description\": \"{}\"", json_escape(desc)));
        }
        json.push_str("\n  }");

        // Servers
        if !self.servers.is_empty() {
            let svrs: Vec<String> = self.servers.iter().map(|s| {
                let mut obj = format!("{{\"url\":\"{}\"", s.url);
                if let Some(ref d) = s.description { obj.push_str(&format!(",\"description\":\"{}\"", json_escape(d))); }
                obj.push('}');
                obj
            }).collect();
            json.push_str(&format!(",\n  \"servers\": [{}]", svrs.join(",")));
        }

        // Paths
        json.push_str(",\n  \"paths\": {");
        let mut path_groups: HashMap<&str, Vec<&ApiPath>> = HashMap::new();
        for p in &self.paths {
            path_groups.entry(&p.path).or_default().push(p);
        }
        let path_entries: Vec<String> = path_groups.iter().map(|(path, methods)| {
            let method_entries: Vec<String> = methods.iter().map(|p| {
                format!("\"{}\":{}", p.method.as_str(), p.endpoint.to_json())
            }).collect();
            format!("\"{}\":{{{}}}", path, method_entries.join(","))
        }).collect();
        json.push_str(&path_entries.join(","));
        json.push('}');

        // Schemas
        if !self.schemas.is_empty() {
            json.push_str(",\n  \"components\": {\"schemas\": {");
            let schema_entries: Vec<String> = self.schemas.iter().map(|(name, s)| {
                format!("\"{}\":{}", name, s.to_json())
            }).collect();
            json.push_str(&schema_entries.join(","));
            json.push_str("}}");
        }

        json.push_str("\n}");
        json
    }

    /// Generate Swagger UI HTML page
    pub fn swagger_ui_html(&self, spec_url: &str) -> String {
        let dom_id = "#swagger-ui";
        format!(
r#"<!DOCTYPE html>
<html>
<head>
<title>{title} - API Docs</title>
<link rel="stylesheet" href="https://unpkg.com/swagger-ui-dist@5/swagger-ui.css">
</head>
<body>
<div id="swagger-ui"></div>
<script src="https://unpkg.com/swagger-ui-dist@5/swagger-ui-bundle.js"></script>
<script>
SwaggerUIBundle({{url:"{spec_url}",dom_id:"{dom_id}",presets:[SwaggerUIBundle.presets.apis],layout:"StandaloneLayout"}});
</script>
</body>
</html>"#, title = self.title, spec_url = spec_url, dom_id = dom_id)
    }
}

impl ApiEndpoint {
    pub fn new() -> Self {
        Self {
            summary: None, description: None, operation_id: None, tags: Vec::new(),
            parameters: Vec::new(), request_body: None, responses: Vec::new(),
            security: Vec::new(), deprecated: false,
        }
    }

    pub fn summary(mut self, s: &str) -> Self { self.summary = Some(s.into()); self }
    pub fn description(mut self, d: &str) -> Self { self.description = Some(d.into()); self }
    pub fn operation_id(mut self, id: &str) -> Self { self.operation_id = Some(id.into()); self }
    pub fn tag(mut self, tag: &str) -> Self { self.tags.push(tag.into()); self }

    pub fn param(mut self, p: ApiParameter) -> Self { self.parameters.push(p); self }
    pub fn query_param(mut self, name: &str, schema_type: &str, required: bool) -> Self {
        self.parameters.push(ApiParameter { name: name.into(), location: ParamLocation::Query, required, schema_type: schema_type.into(), description: None });
        self
    }
    pub fn path_param(mut self, name: &str, schema_type: &str) -> Self {
        self.parameters.push(ApiParameter { name: name.into(), location: ParamLocation::Path, required: true, schema_type: schema_type.into(), description: None });
        self
    }

    pub fn body(mut self, content_type: &str, schema_ref: Option<&str>) -> Self {
        self.request_body = Some(ApiRequestBody { content_type: content_type.into(), schema_ref: schema_ref.map(Into::into), required: true });
        self
    }

    pub fn response(mut self, status: u16, description: &str) -> Self {
        self.responses.push((status, ApiResponse { description: description.into(), content_type: None, schema_ref: None }));
        self
    }

    pub fn response_with_schema(mut self, status: u16, desc: &str, schema_ref: &str) -> Self {
        self.responses.push((status, ApiResponse { description: desc.into(), content_type: Some("application/json".into()), schema_ref: Some(schema_ref.into()) }));
        self
    }

    pub fn security(mut self, name: &str) -> Self { self.security.push(name.into()); self }
    pub fn deprecated(mut self) -> Self { self.deprecated = true; self }

    fn to_json(&self) -> String {
        let mut json = String::from("{");
        let mut parts = Vec::new();

        if let Some(ref s) = self.summary { parts.push(format!("\"summary\":\"{}\"", json_escape(s))); }
        if let Some(ref d) = self.description { parts.push(format!("\"description\":\"{}\"", json_escape(d))); }
        if let Some(ref id) = self.operation_id { parts.push(format!("\"operationId\":\"{}\"", id)); }
        if !self.tags.is_empty() {
            let tags: Vec<String> = self.tags.iter().map(|t| format!("\"{}\"", t)).collect();
            parts.push(format!("\"tags\":[{}]", tags.join(",")));
        }
        if self.deprecated { parts.push("\"deprecated\":true".into()); }
        if !self.parameters.is_empty() {
            let params: Vec<String> = self.parameters.iter().map(|p| p.to_json()).collect();
            parts.push(format!("\"parameters\":[{}]", params.join(",")));
        }
        if !self.responses.is_empty() {
            let resps: Vec<String> = self.responses.iter().map(|(code, r)| {
                format!("\"{}\":{}", code, r.to_json())
            }).collect();
            parts.push(format!("\"responses\":{{{}}}", resps.join(",")));
        }

        json.push_str(&parts.join(","));
        json.push('}');
        json
    }
}

impl Default for ApiEndpoint {
    fn default() -> Self { Self::new() }
}

impl ApiParameter {
    fn to_json(&self) -> String {
        let mut json = format!(
            "{{\"name\":\"{}\",\"in\":\"{}\",\"required\":{},\"schema\":{{\"type\":\"{}\"}}",
            self.name, self.location.as_str(), self.required, self.schema_type
        );
        if let Some(ref d) = self.description {
            json.push_str(&format!(",\"description\":\"{}\"", json_escape(d)));
        }
        json.push('}');
        json
    }
}

impl ApiResponse {
    fn to_json(&self) -> String {
        let mut json = format!("{{\"description\":\"{}\"", json_escape(&self.description));
        if let Some(ref ct) = self.content_type {
            json.push_str(&format!(",\"content\":{{\"{}\":{{", ct));
            if let Some(ref sr) = self.schema_ref {
                json.push_str(&format!("\"schema\":{{\"$ref\":\"#/components/schemas/{}\"}}", sr));
            }
            json.push_str("}}");
        }
        json.push('}');
        json
    }
}

impl ApiSchema {
    pub fn object() -> Self {
        Self { schema_type: "object".into(), properties: Vec::new(), description: None }
    }

    pub fn prop(mut self, name: &str, prop_type: &str, required: bool) -> Self {
        self.properties.push((name.into(), prop_type.into(), required));
        self
    }

    pub fn description(mut self, d: &str) -> Self { self.description = Some(d.into()); self }

    fn to_json(&self) -> String {
        let mut json = format!("{{\"type\":\"{}\"", self.schema_type);
        if !self.properties.is_empty() {
            let props: Vec<String> = self.properties.iter().map(|(n, t, _)| {
                format!("\"{}\":{{\"type\":\"{}\"}}", n, t)
            }).collect();
            json.push_str(&format!(",\"properties\":{{{}}}", props.join(",")));
            let required: Vec<String> = self.properties.iter().filter(|(_, _, r)| *r).map(|(n, _, _)| format!("\"{}\"", n)).collect();
            if !required.is_empty() {
                json.push_str(&format!(",\"required\":[{}]", required.join(",")));
            }
        }
        json.push('}');
        json
    }
}

fn json_escape(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"").replace('\n', "\\n")
}

// ─── Tests ──────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_spec() {
        let spec = OpenApiSpec::new("Test API", "1.0.0")
            .description("A test API");
        let json = spec.to_json();
        assert!(json.contains("\"openapi\": \"3.0.3\""));
        assert!(json.contains("\"title\": \"Test API\""));
    }

    #[test]
    fn test_spec_with_paths() {
        let spec = OpenApiSpec::new("API", "1.0.0")
            .path("/users", HttpMethod::Get, ApiEndpoint::new().summary("List users").tag("users"))
            .path("/users", HttpMethod::Post, ApiEndpoint::new().summary("Create user"));
        let json = spec.to_json();
        assert!(json.contains("\"/users\""));
        assert!(json.contains("\"get\""));
        assert!(json.contains("\"post\""));
        assert!(json.contains("List users"));
    }

    #[test]
    fn test_endpoint_params() {
        let ep = ApiEndpoint::new()
            .path_param("id", "integer")
            .query_param("page", "integer", false)
            .response(200, "Success");
        let json = ep.to_json();
        assert!(json.contains("\"in\":\"path\""));
        assert!(json.contains("\"in\":\"query\""));
        assert!(json.contains("\"200\""));
    }

    #[test]
    fn test_endpoint_body() {
        let ep = ApiEndpoint::new()
            .body("application/json", Some("CreateUser"))
            .response_with_schema(201, "Created", "User");
        let json = ep.to_json();
        assert!(json.contains("\"201\""));
    }

    #[test]
    fn test_schema() {
        let spec = OpenApiSpec::new("API", "1.0.0")
            .schema("User", ApiSchema::object()
                .prop("id", "integer", true)
                .prop("name", "string", true)
                .prop("email", "string", false));
        let json = spec.to_json();
        assert!(json.contains("\"schemas\""));
        assert!(json.contains("\"User\""));
        assert!(json.contains("\"required\""));
    }

    #[test]
    fn test_swagger_ui() {
        let spec = OpenApiSpec::new("API", "1.0.0");
        let html = spec.swagger_ui_html("/api/openapi.json");
        assert!(html.contains("swagger-ui"));
        assert!(html.contains("/api/openapi.json"));
    }

    #[test]
    fn test_server() {
        let spec = OpenApiSpec::new("API", "1.0.0")
            .server("https://api.example.com", Some("Production"));
        let json = spec.to_json();
        assert!(json.contains("\"servers\""));
        assert!(json.contains("api.example.com"));
    }

    #[test]
    fn test_deprecated() {
        let ep = ApiEndpoint::new().deprecated();
        let json = ep.to_json();
        assert!(json.contains("\"deprecated\":true"));
    }

    #[test]
    fn test_operation_id() {
        let ep = ApiEndpoint::new().operation_id("listUsers");
        let json = ep.to_json();
        assert!(json.contains("\"operationId\":\"listUsers\""));
    }
}
