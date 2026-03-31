//! GraphQL Support for Hayabusa.
//!
//! Lightweight GraphQL server (schema definition, query execution,
//! resolver registration) and client (query builder).
//!
//! ```ignore
//! use hayabusa_core::prelude::*;
//!
//! let schema = GqlSchema::new()
//!     .query("user", GqlField::new("User")
//!         .arg("id", "ID!")
//!         .field("name", "String!")
//!         .field("email", "String"));
//! ```

use std::collections::HashMap;

// ─── Schema Definition ──────────────────────────────────────

/// GraphQL schema builder
#[derive(Debug, Clone)]
pub struct GqlSchema {
    pub queries: Vec<GqlField>,
    pub mutations: Vec<GqlField>,
    pub types: HashMap<String, GqlType>,
    pub subscriptions: Vec<GqlField>,
}

/// A GraphQL type definition
#[derive(Debug, Clone)]
pub struct GqlType {
    pub name: String,
    pub fields: Vec<(String, String)>, // (name, type)
    pub description: Option<String>,
}

/// A GraphQL field (query/mutation)
#[derive(Debug, Clone)]
pub struct GqlField {
    pub name: String,
    pub return_type: String,
    pub args: Vec<(String, String)>,    // (name, type)
    pub fields: Vec<(String, String)>,  // sub-fields
    pub description: Option<String>,
}

impl GqlSchema {
    pub fn new() -> Self {
        Self {
            queries: Vec::new(),
            mutations: Vec::new(),
            types: HashMap::new(),
            subscriptions: Vec::new(),
        }
    }

    pub fn query(mut self, name: &str, field: GqlField) -> Self {
        let mut f = field;
        f.name = name.to_string();
        self.queries.push(f);
        self
    }

    pub fn mutation(mut self, name: &str, field: GqlField) -> Self {
        let mut f = field;
        f.name = name.to_string();
        self.mutations.push(f);
        self
    }

    pub fn subscription(mut self, name: &str, field: GqlField) -> Self {
        let mut f = field;
        f.name = name.to_string();
        self.subscriptions.push(f);
        self
    }

    pub fn define_type(mut self, gql_type: GqlType) -> Self {
        self.types.insert(gql_type.name.clone(), gql_type);
        self
    }

    /// Generate SDL (Schema Definition Language) string
    pub fn to_sdl(&self) -> String {
        let mut sdl = String::new();

        // Types
        for (_, t) in &self.types {
            if let Some(ref desc) = t.description {
                sdl.push_str(&format!("\"\"\"{}\"\"\"\n", desc));
            }
            sdl.push_str(&format!("type {} {{\n", t.name));
            for (fname, ftype) in &t.fields {
                sdl.push_str(&format!("  {}: {}\n", fname, ftype));
            }
            sdl.push_str("}\n\n");
        }

        // Queries
        if !self.queries.is_empty() {
            sdl.push_str("type Query {\n");
            for q in &self.queries {
                sdl.push_str(&format!("  {}", q.to_sdl_field()));
            }
            sdl.push_str("}\n\n");
        }

        // Mutations
        if !self.mutations.is_empty() {
            sdl.push_str("type Mutation {\n");
            for m in &self.mutations {
                sdl.push_str(&format!("  {}", m.to_sdl_field()));
            }
            sdl.push_str("}\n\n");
        }

        // Subscriptions
        if !self.subscriptions.is_empty() {
            sdl.push_str("type Subscription {\n");
            for s in &self.subscriptions {
                sdl.push_str(&format!("  {}", s.to_sdl_field()));
            }
            sdl.push_str("}\n");
        }

        sdl
    }

    /// Generate the introspection response for the schema
    pub fn introspection_json(&self) -> String {
        let types: Vec<String> = self.types.iter().map(|(_, t)| {
            let fields: Vec<String> = t.fields.iter().map(|(n, ty)| {
                format!("{{\"name\":\"{}\",\"type\":\"{}\"}}", n, ty)
            }).collect();
            format!("{{\"name\":\"{}\",\"fields\":[{}]}}", t.name, fields.join(","))
        }).collect();
        format!("{{\"data\":{{\"__schema\":{{\"types\":[{}]}}}}}}", types.join(","))
    }
}

impl Default for GqlSchema {
    fn default() -> Self {
        Self::new()
    }
}

impl GqlField {
    pub fn new(return_type: impl Into<String>) -> Self {
        Self {
            name: String::new(),
            return_type: return_type.into(),
            args: Vec::new(),
            fields: Vec::new(),
            description: None,
        }
    }

    pub fn arg(mut self, name: &str, gql_type: &str) -> Self {
        self.args.push((name.to_string(), gql_type.to_string()));
        self
    }

    pub fn field(mut self, name: &str, gql_type: &str) -> Self {
        self.fields.push((name.to_string(), gql_type.to_string()));
        self
    }

    pub fn description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    fn to_sdl_field(&self) -> String {
        let args = if self.args.is_empty() {
            String::new()
        } else {
            let a: Vec<String> = self.args.iter().map(|(n, t)| format!("{}: {}", n, t)).collect();
            format!("({})", a.join(", "))
        };
        format!("{}{}: {}\n", self.name, args, self.return_type)
    }
}

impl GqlType {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            fields: Vec::new(),
            description: None,
        }
    }

    pub fn field(mut self, name: &str, gql_type: &str) -> Self {
        self.fields.push((name.to_string(), gql_type.to_string()));
        self
    }

    pub fn description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }
}

// ─── Query Parser ───────────────────────────────────────────

/// Parsed GraphQL query
#[derive(Debug, Clone)]
pub struct GqlQuery {
    pub operation: GqlOperation,
    pub name: Option<String>,
    pub selections: Vec<GqlSelection>,
    pub variables: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum GqlOperation {
    Query,
    Mutation,
    Subscription,
}

/// A field selection in a query
#[derive(Debug, Clone)]
pub struct GqlSelection {
    pub field: String,
    pub alias: Option<String>,
    pub args: HashMap<String, serde_json::Value>,
    pub children: Vec<GqlSelection>,
}

/// Simple GraphQL query parser
pub fn parse_query(input: &str) -> Option<GqlQuery> {
    let input = input.trim();

    // Determine operation type
    let (operation, rest) = if input.starts_with("mutation") {
        (GqlOperation::Mutation, input.trim_start_matches("mutation").trim())
    } else if input.starts_with("subscription") {
        (GqlOperation::Subscription, input.trim_start_matches("subscription").trim())
    } else if input.starts_with("query") {
        (GqlOperation::Query, input.trim_start_matches("query").trim())
    } else if input.starts_with('{') {
        (GqlOperation::Query, input)
    } else {
        return None;
    };

    // Extract operation name (if any)
    let (name, body) = if rest.starts_with('{') {
        (None, rest)
    } else {
        let name_end = rest.find(|c: char| c == '{' || c == '(')?;
        let name = rest[..name_end].trim().to_string();
        (Some(name).filter(|n| !n.is_empty()), &rest[name_end..])
    };

    // Parse selections from the body
    let selections = parse_selections(body);

    Some(GqlQuery {
        operation,
        name,
        selections,
        variables: HashMap::new(),
    })
}

fn parse_selections(input: &str) -> Vec<GqlSelection> {
    let input = input.trim();
    if !input.starts_with('{') || !input.ends_with('}') {
        return Vec::new();
    }
    let inner = &input[1..input.len() - 1].trim();

    let mut selections = Vec::new();
    for field in inner.split_whitespace() {
        let field = field.trim();
        if field.is_empty() || field == "{" || field == "}" {
            continue;
        }
        selections.push(GqlSelection {
            field: field.to_string(),
            alias: None,
            args: HashMap::new(),
            children: Vec::new(),
        });
    }
    selections
}

// ─── GraphQL Client ─────────────────────────────────────────

/// GraphQL HTTP client for making queries to external APIs
#[derive(Debug, Clone)]
pub struct GqlClient {
    pub endpoint: String,
    pub headers: HashMap<String, String>,
}

impl GqlClient {
    pub fn new(endpoint: impl Into<String>) -> Self {
        Self {
            endpoint: endpoint.into(),
            headers: HashMap::new(),
        }
    }

    pub fn header(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.insert(key.into(), value.into());
        self
    }

    pub fn bearer_token(self, token: &str) -> Self {
        self.header("Authorization", format!("Bearer {}", token))
    }

    /// Build a query request body as JSON
    pub fn query_body(query: &str, variables: Option<&str>) -> String {
        let vars = variables.unwrap_or("{}");
        format!(
            "{{\"query\":\"{}\",\"variables\":{}}}",
            json_escape(query),
            vars
        )
    }

    /// Build a named query request body
    pub fn named_query_body(query: &str, operation: &str, variables: Option<&str>) -> String {
        let vars = variables.unwrap_or("{}");
        format!(
            "{{\"query\":\"{}\",\"operationName\":\"{}\",\"variables\":{}}}",
            json_escape(query),
            operation,
            vars
        )
    }
}

// ─── GraphQL Response ───────────────────────────────────────

/// Parse a GraphQL response
#[derive(Debug, Clone)]
pub struct GqlResponse {
    pub data: Option<serde_json::Value>,
    pub errors: Vec<GqlError>,
}

#[derive(Debug, Clone)]
pub struct GqlError {
    pub message: String,
    pub path: Option<Vec<String>>,
}

impl GqlResponse {
    pub fn from_json(json: &str) -> Option<Self> {
        let val: serde_json::Value = serde_json::from_str(json).ok()?;
        let data = val.get("data").cloned();
        let errors = val
            .get("errors")
            .and_then(|e| e.as_array())
            .map(|arr| {
                arr.iter()
                    .map(|e| GqlError {
                        message: e
                            .get("message")
                            .and_then(|m| m.as_str())
                            .unwrap_or("")
                            .to_string(),
                        path: e.get("path").and_then(|p| p.as_array()).map(|arr| {
                            arr.iter()
                                .filter_map(|v| v.as_str().map(String::from))
                                .collect()
                        }),
                    })
                    .collect()
            })
            .unwrap_or_default();
        Some(Self { data, errors })
    }

    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }

    pub fn is_ok(&self) -> bool {
        self.errors.is_empty() && self.data.is_some()
    }
}

// ─── Helpers ────────────────────────────────────────────────

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
    fn test_schema_to_sdl() {
        let schema = GqlSchema::new()
            .define_type(
                GqlType::new("User")
                    .field("id", "ID!")
                    .field("name", "String!")
                    .field("email", "String"),
            )
            .query("user", GqlField::new("User").arg("id", "ID!"))
            .query("users", GqlField::new("[User!]!"))
            .mutation("createUser", GqlField::new("User!").arg("name", "String!").arg("email", "String!"));

        let sdl = schema.to_sdl();
        assert!(sdl.contains("type User {"));
        assert!(sdl.contains("name: String!"));
        assert!(sdl.contains("type Query {"));
        assert!(sdl.contains("user(id: ID!): User"));
        assert!(sdl.contains("type Mutation {"));
        assert!(sdl.contains("createUser(name: String!, email: String!): User!"));
    }

    #[test]
    fn test_schema_with_description() {
        let schema = GqlSchema::new()
            .define_type(GqlType::new("Post").description("A blog post").field("title", "String!"));
        let sdl = schema.to_sdl();
        assert!(sdl.contains("\"\"\"A blog post\"\"\""));
    }

    #[test]
    fn test_parse_query() {
        let q = parse_query("query GetUser { user name email }").unwrap();
        assert_eq!(q.operation, GqlOperation::Query);
        assert_eq!(q.name, Some("GetUser".to_string()));
    }

    #[test]
    fn test_parse_mutation() {
        let q = parse_query("mutation { createUser }").unwrap();
        assert_eq!(q.operation, GqlOperation::Mutation);
    }

    #[test]
    fn test_parse_anonymous() {
        let q = parse_query("{ user posts }").unwrap();
        assert_eq!(q.operation, GqlOperation::Query);
        assert_eq!(q.name, None);
        assert_eq!(q.selections.len(), 2);
    }

    #[test]
    fn test_gql_client() {
        let client = GqlClient::new("https://api.example.com/graphql")
            .bearer_token("token123");
        assert!(client.headers.get("Authorization").unwrap().contains("token123"));
    }

    #[test]
    fn test_query_body() {
        let body = GqlClient::query_body("{ users { id name } }", None);
        assert!(body.contains("\"query\""));
        assert!(body.contains("users"));
    }

    #[test]
    fn test_named_query_body() {
        let body = GqlClient::named_query_body("query GetUser($id: ID!) { user(id: $id) { name } }", "GetUser", Some("{\"id\":\"1\"}"));
        assert!(body.contains("\"operationName\":\"GetUser\""));
        assert!(body.contains("\"id\":\"1\""));
    }

    #[test]
    fn test_gql_response_ok() {
        let resp = GqlResponse::from_json("{\"data\":{\"user\":{\"name\":\"Alice\"}}}").unwrap();
        assert!(resp.is_ok());
        assert!(!resp.has_errors());
    }

    #[test]
    fn test_gql_response_error() {
        let resp = GqlResponse::from_json("{\"data\":null,\"errors\":[{\"message\":\"Not found\"}]}").unwrap();
        assert!(resp.has_errors());
        assert_eq!(resp.errors[0].message, "Not found");
    }

    #[test]
    fn test_introspection() {
        let schema = GqlSchema::new()
            .define_type(GqlType::new("User").field("id", "ID!"));
        let json = schema.introspection_json();
        assert!(json.contains("__schema"));
        assert!(json.contains("User"));
    }

    #[test]
    fn test_gql_field_builder() {
        let field = GqlField::new("User!")
            .arg("id", "ID!")
            .description("Get a user by ID");
        assert_eq!(field.return_type, "User!");
        assert_eq!(field.args.len(), 1);
    }

    #[test]
    fn test_subscriptions() {
        let schema = GqlSchema::new()
            .subscription("messageAdded", GqlField::new("Message!"));
        let sdl = schema.to_sdl();
        assert!(sdl.contains("type Subscription {"));
        assert!(sdl.contains("messageAdded: Message!"));
    }
}
