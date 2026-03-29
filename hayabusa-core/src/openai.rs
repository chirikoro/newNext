//! OpenAI API Client for Hayabusa.
//!
//! Unofficial Rust client for the OpenAI REST API.
//! Supports Chat Completions, Embeddings, and Streaming.
//!
//! ## Supported Features
//! - **Chat Completions**: GPT-4o, GPT-4, GPT-3.5 with tools/functions
//! - **Streaming**: Server-Sent Events streaming for chat
//! - **Embeddings**: text-embedding-3-small/large, ada-002
//! - **Images**: DALL-E generation
//! - **Audio**: Whisper transcription, TTS
//!
//! ## Usage
//! ```ignore
//! let ai = OpenAiClient::new("sk-...");
//!
//! // Chat completion
//! let response = ai.chat()
//!     .model("gpt-4o")
//!     .system("You are a helpful assistant.")
//!     .user("What is Rust?")
//!     .temperature(0.7)
//!     .send()
//!     .await?;
//! println!("{}", response.content());
//!
//! // Streaming
//! let stream = ai.chat()
//!     .model("gpt-4o")
//!     .user("Tell me a story.")
//!     .stream()
//!     .await?;
//!
//! // Embeddings
//! let embeddings = ai.embeddings("text-embedding-3-small")
//!     .input("Hello, world!")
//!     .send()
//!     .await?;
//! ```

use reqwest::Client;
use serde_json::Value;

/// OpenAI client errors
#[derive(Debug, Clone)]
pub enum OpenAiError {
    RequestFailed(String),
    RateLimited(String),
    InvalidApiKey(String),
    ParseError(String),
}

impl std::fmt::Display for OpenAiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OpenAiError::RequestFailed(m) => write!(f, "OpenAI request failed: {}", m),
            OpenAiError::RateLimited(m) => write!(f, "OpenAI rate limited: {}", m),
            OpenAiError::InvalidApiKey(m) => write!(f, "OpenAI invalid API key: {}", m),
            OpenAiError::ParseError(m) => write!(f, "OpenAI parse error: {}", m),
        }
    }
}

/// OpenAI API client
#[derive(Debug, Clone)]
pub struct OpenAiClient {
    api_key: String,
    base_url: String,
    organization: Option<String>,
    client: Client,
}

impl OpenAiClient {
    /// Create a new client with API key
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            base_url: "https://api.openai.com/v1".to_string(),
            organization: None,
            client: Client::new(),
        }
    }

    /// Use a custom base URL (for proxies, Azure OpenAI, local models)
    pub fn with_base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = url.into().trim_end_matches('/').to_string();
        self
    }

    /// Set organization ID
    pub fn with_organization(mut self, org: impl Into<String>) -> Self {
        self.organization = Some(org.into());
        self
    }

    /// Create a chat completion builder
    pub fn chat(&self) -> ChatBuilder {
        ChatBuilder {
            client: self.client.clone(),
            url: format!("{}/chat/completions", self.base_url),
            api_key: self.api_key.clone(),
            organization: self.organization.clone(),
            model: "gpt-4o".to_string(),
            messages: Vec::new(),
            temperature: None,
            max_tokens: None,
            top_p: None,
            frequency_penalty: None,
            presence_penalty: None,
            tools: None,
            response_format: None,
            seed: None,
        }
    }

    /// Create an embeddings builder
    pub fn embeddings(&self, model: &str) -> EmbeddingsBuilder {
        EmbeddingsBuilder {
            client: self.client.clone(),
            url: format!("{}/embeddings", self.base_url),
            api_key: self.api_key.clone(),
            organization: self.organization.clone(),
            model: model.to_string(),
            inputs: Vec::new(),
            dimensions: None,
        }
    }

    /// Generate an image with DALL-E
    pub fn image(&self) -> ImageBuilder {
        ImageBuilder {
            client: self.client.clone(),
            url: format!("{}/images/generations", self.base_url),
            api_key: self.api_key.clone(),
            organization: self.organization.clone(),
            model: "dall-e-3".to_string(),
            prompt: String::new(),
            size: "1024x1024".to_string(),
            quality: "standard".to_string(),
            n: 1,
        }
    }

    /// Transcribe audio with Whisper
    pub async fn transcribe(
        &self,
        audio_data: Vec<u8>,
        filename: &str,
    ) -> Result<String, OpenAiError> {
        let part = reqwest::multipart::Part::bytes(audio_data)
            .file_name(filename.to_string())
            .mime_str("audio/mpeg")
            .map_err(|e| OpenAiError::RequestFailed(e.to_string()))?;

        let form = reqwest::multipart::Form::new()
            .text("model", "whisper-1")
            .part("file", part);

        let resp = self
            .client
            .post(format!("{}/audio/transcriptions", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .multipart(form)
            .send()
            .await
            .map_err(|e| OpenAiError::RequestFailed(e.to_string()))?;

        let json = parse_openai_response(resp).await?;
        Ok(json
            .get("text")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string())
    }

    /// Text-to-speech
    pub async fn tts(
        &self,
        input: &str,
        voice: &str,
    ) -> Result<Vec<u8>, OpenAiError> {
        let body = serde_json::json!({
            "model": "tts-1",
            "input": input,
            "voice": voice,
        });

        let resp = self
            .client
            .post(format!("{}/audio/speech", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| OpenAiError::RequestFailed(e.to_string()))?;

        resp.bytes()
            .await
            .map(|b| b.to_vec())
            .map_err(|e| OpenAiError::RequestFailed(e.to_string()))
    }
}

// ─────────────────────────────────────────────
//  Chat Completions
// ─────────────────────────────────────────────

/// Chat completion builder
#[derive(Debug, Clone)]
pub struct ChatBuilder {
    client: Client,
    url: String,
    api_key: String,
    organization: Option<String>,
    model: String,
    messages: Vec<Value>,
    temperature: Option<f64>,
    max_tokens: Option<u64>,
    top_p: Option<f64>,
    frequency_penalty: Option<f64>,
    presence_penalty: Option<f64>,
    tools: Option<Vec<Value>>,
    response_format: Option<Value>,
    seed: Option<u64>,
}

impl ChatBuilder {
    /// Set the model
    pub fn model(mut self, model: impl Into<String>) -> Self {
        self.model = model.into();
        self
    }

    /// Add a system message
    pub fn system(mut self, content: impl Into<String>) -> Self {
        self.messages.push(serde_json::json!({
            "role": "system",
            "content": content.into(),
        }));
        self
    }

    /// Add a user message
    pub fn user(mut self, content: impl Into<String>) -> Self {
        self.messages.push(serde_json::json!({
            "role": "user",
            "content": content.into(),
        }));
        self
    }

    /// Add an assistant message
    pub fn assistant(mut self, content: impl Into<String>) -> Self {
        self.messages.push(serde_json::json!({
            "role": "assistant",
            "content": content.into(),
        }));
        self
    }

    /// Add a user message with images (vision)
    pub fn user_with_images(mut self, text: &str, image_urls: &[&str]) -> Self {
        let mut content = vec![serde_json::json!({
            "type": "text",
            "text": text,
        })];

        for url in image_urls {
            content.push(serde_json::json!({
                "type": "image_url",
                "image_url": {"url": *url},
            }));
        }

        self.messages.push(serde_json::json!({
            "role": "user",
            "content": content,
        }));
        self
    }

    /// Add raw messages
    pub fn messages(mut self, messages: Vec<Value>) -> Self {
        self.messages = messages;
        self
    }

    /// Set temperature (0.0 - 2.0)
    pub fn temperature(mut self, temp: f64) -> Self {
        self.temperature = Some(temp);
        self
    }

    /// Set max tokens
    pub fn max_tokens(mut self, tokens: u64) -> Self {
        self.max_tokens = Some(tokens);
        self
    }

    /// Set top_p
    pub fn top_p(mut self, p: f64) -> Self {
        self.top_p = Some(p);
        self
    }

    /// Set frequency penalty
    pub fn frequency_penalty(mut self, penalty: f64) -> Self {
        self.frequency_penalty = Some(penalty);
        self
    }

    /// Set presence penalty
    pub fn presence_penalty(mut self, penalty: f64) -> Self {
        self.presence_penalty = Some(penalty);
        self
    }

    /// Set seed for deterministic output
    pub fn seed(mut self, seed: u64) -> Self {
        self.seed = Some(seed);
        self
    }

    /// Request JSON output format
    pub fn json_mode(mut self) -> Self {
        self.response_format = Some(serde_json::json!({"type": "json_object"}));
        self
    }

    /// Add tools (function calling)
    pub fn tools(mut self, tools: Vec<Value>) -> Self {
        self.tools = Some(tools);
        self
    }

    /// Add a single tool/function
    pub fn tool(
        mut self,
        name: &str,
        description: &str,
        parameters: Value,
    ) -> Self {
        let tool = serde_json::json!({
            "type": "function",
            "function": {
                "name": name,
                "description": description,
                "parameters": parameters,
            }
        });
        self.tools.get_or_insert_with(Vec::new).push(tool);
        self
    }

    fn build_body(&self) -> Value {
        let mut body = serde_json::json!({
            "model": self.model,
            "messages": self.messages,
        });

        if let Some(temp) = self.temperature {
            body["temperature"] = serde_json::json!(temp);
        }
        if let Some(tokens) = self.max_tokens {
            body["max_tokens"] = serde_json::json!(tokens);
        }
        if let Some(p) = self.top_p {
            body["top_p"] = serde_json::json!(p);
        }
        if let Some(fp) = self.frequency_penalty {
            body["frequency_penalty"] = serde_json::json!(fp);
        }
        if let Some(pp) = self.presence_penalty {
            body["presence_penalty"] = serde_json::json!(pp);
        }
        if let Some(ref tools) = self.tools {
            body["tools"] = serde_json::json!(tools);
        }
        if let Some(ref fmt) = self.response_format {
            body["response_format"] = fmt.clone();
        }
        if let Some(seed) = self.seed {
            body["seed"] = serde_json::json!(seed);
        }

        body
    }

    /// Send the chat completion request
    pub async fn send(&self) -> Result<ChatResponse, OpenAiError> {
        let body = self.build_body();

        let mut req = self
            .client
            .post(&self.url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json");

        if let Some(ref org) = self.organization {
            req = req.header("OpenAI-Organization", org.as_str());
        }

        let resp = req
            .json(&body)
            .send()
            .await
            .map_err(|e| OpenAiError::RequestFailed(e.to_string()))?;

        let json = parse_openai_response(resp).await?;
        Ok(ChatResponse { data: json })
    }

    /// Send as streaming request, returns raw SSE text chunks
    pub async fn stream(&self) -> Result<String, OpenAiError> {
        let mut body = self.build_body();
        body["stream"] = serde_json::json!(true);

        let mut req = self
            .client
            .post(&self.url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json");

        if let Some(ref org) = self.organization {
            req = req.header("OpenAI-Organization", org.as_str());
        }

        let resp = req
            .json(&body)
            .send()
            .await
            .map_err(|e| OpenAiError::RequestFailed(e.to_string()))?;

        // Collect all streamed content
        let text = resp
            .text()
            .await
            .map_err(|e| OpenAiError::RequestFailed(e.to_string()))?;

        // Parse SSE events and extract content deltas
        let mut full_content = String::new();
        for line in text.lines() {
            if let Some(data) = line.strip_prefix("data: ") {
                if data == "[DONE]" {
                    break;
                }
                if let Ok(json) = serde_json::from_str::<Value>(data) {
                    if let Some(delta) = json
                        .get("choices")
                        .and_then(|c| c.get(0))
                        .and_then(|c| c.get("delta"))
                        .and_then(|d| d.get("content"))
                        .and_then(|c| c.as_str())
                    {
                        full_content.push_str(delta);
                    }
                }
            }
        }

        Ok(full_content)
    }
}

/// Chat completion response
#[derive(Debug, Clone)]
pub struct ChatResponse {
    pub data: Value,
}

impl ChatResponse {
    /// Get the text content of the first choice
    pub fn content(&self) -> &str {
        self.data
            .get("choices")
            .and_then(|c| c.get(0))
            .and_then(|c| c.get("message"))
            .and_then(|m| m.get("content"))
            .and_then(|c| c.as_str())
            .unwrap_or("")
    }

    /// Get the role of the first choice
    pub fn role(&self) -> &str {
        self.data
            .get("choices")
            .and_then(|c| c.get(0))
            .and_then(|c| c.get("message"))
            .and_then(|m| m.get("role"))
            .and_then(|r| r.as_str())
            .unwrap_or("assistant")
    }

    /// Get tool calls if any
    pub fn tool_calls(&self) -> Vec<ToolCall> {
        let calls = self
            .data
            .get("choices")
            .and_then(|c| c.get(0))
            .and_then(|c| c.get("message"))
            .and_then(|m| m.get("tool_calls"))
            .and_then(|t| t.as_array());

        match calls {
            Some(arr) => arr
                .iter()
                .filter_map(|tc| {
                    Some(ToolCall {
                        id: tc.get("id")?.as_str()?.to_string(),
                        function_name: tc
                            .get("function")?
                            .get("name")?
                            .as_str()?
                            .to_string(),
                        arguments: tc
                            .get("function")?
                            .get("arguments")?
                            .as_str()?
                            .to_string(),
                    })
                })
                .collect(),
            None => Vec::new(),
        }
    }

    /// Get usage information
    pub fn usage(&self) -> (u64, u64, u64) {
        let usage = self.data.get("usage");
        let prompt = usage
            .and_then(|u| u.get("prompt_tokens"))
            .and_then(|v| v.as_u64())
            .unwrap_or(0);
        let completion = usage
            .and_then(|u| u.get("completion_tokens"))
            .and_then(|v| v.as_u64())
            .unwrap_or(0);
        (prompt, completion, prompt + completion)
    }

    /// Get the finish reason
    pub fn finish_reason(&self) -> &str {
        self.data
            .get("choices")
            .and_then(|c| c.get(0))
            .and_then(|c| c.get("finish_reason"))
            .and_then(|r| r.as_str())
            .unwrap_or("stop")
    }

    /// Get the raw JSON response
    pub fn raw(&self) -> &Value {
        &self.data
    }
}

/// A tool/function call from the model
#[derive(Debug, Clone)]
pub struct ToolCall {
    pub id: String,
    pub function_name: String,
    pub arguments: String,
}

impl ToolCall {
    /// Parse the arguments as JSON
    pub fn parse_arguments(&self) -> Result<Value, OpenAiError> {
        serde_json::from_str(&self.arguments)
            .map_err(|e| OpenAiError::ParseError(e.to_string()))
    }
}

// ─────────────────────────────────────────────
//  Embeddings
// ─────────────────────────────────────────────

/// Embeddings request builder
#[derive(Debug, Clone)]
pub struct EmbeddingsBuilder {
    client: Client,
    url: String,
    api_key: String,
    organization: Option<String>,
    model: String,
    inputs: Vec<String>,
    dimensions: Option<u64>,
}

impl EmbeddingsBuilder {
    /// Add a single input text
    pub fn input(mut self, text: impl Into<String>) -> Self {
        self.inputs.push(text.into());
        self
    }

    /// Add multiple input texts
    pub fn inputs(mut self, texts: &[&str]) -> Self {
        self.inputs.extend(texts.iter().map(|s| s.to_string()));
        self
    }

    /// Set output dimensions (for text-embedding-3 models)
    pub fn dimensions(mut self, dims: u64) -> Self {
        self.dimensions = Some(dims);
        self
    }

    /// Send the embeddings request
    pub async fn send(&self) -> Result<EmbeddingsResponse, OpenAiError> {
        let mut body = serde_json::json!({
            "model": self.model,
            "input": self.inputs,
        });

        if let Some(dims) = self.dimensions {
            body["dimensions"] = serde_json::json!(dims);
        }

        let mut req = self
            .client
            .post(&self.url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json");

        if let Some(ref org) = self.organization {
            req = req.header("OpenAI-Organization", org.as_str());
        }

        let resp = req
            .json(&body)
            .send()
            .await
            .map_err(|e| OpenAiError::RequestFailed(e.to_string()))?;

        let json = parse_openai_response(resp).await?;
        Ok(EmbeddingsResponse { data: json })
    }
}

/// Embeddings response
#[derive(Debug, Clone)]
pub struct EmbeddingsResponse {
    pub data: Value,
}

impl EmbeddingsResponse {
    /// Get the embedding vectors
    pub fn embeddings(&self) -> Vec<Vec<f64>> {
        self.data
            .get("data")
            .and_then(|d| d.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|item| {
                        item.get("embedding").and_then(|e| e.as_array()).map(|vec| {
                            vec.iter()
                                .filter_map(|v| v.as_f64())
                                .collect()
                        })
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get the first embedding
    pub fn first(&self) -> Vec<f64> {
        self.embeddings().into_iter().next().unwrap_or_default()
    }

    /// Get usage
    pub fn usage(&self) -> u64 {
        self.data
            .get("usage")
            .and_then(|u| u.get("total_tokens"))
            .and_then(|v| v.as_u64())
            .unwrap_or(0)
    }
}

// ─────────────────────────────────────────────
//  Image Generation
// ─────────────────────────────────────────────

/// Image generation builder
#[derive(Debug, Clone)]
pub struct ImageBuilder {
    client: Client,
    url: String,
    api_key: String,
    organization: Option<String>,
    model: String,
    prompt: String,
    size: String,
    quality: String,
    n: u32,
}

impl ImageBuilder {
    pub fn model(mut self, model: impl Into<String>) -> Self {
        self.model = model.into();
        self
    }

    pub fn prompt(mut self, prompt: impl Into<String>) -> Self {
        self.prompt = prompt.into();
        self
    }

    pub fn size(mut self, size: &str) -> Self {
        self.size = size.to_string();
        self
    }

    pub fn quality(mut self, quality: &str) -> Self {
        self.quality = quality.to_string();
        self
    }

    pub fn n(mut self, count: u32) -> Self {
        self.n = count;
        self
    }

    pub async fn send(&self) -> Result<Vec<String>, OpenAiError> {
        let body = serde_json::json!({
            "model": self.model,
            "prompt": self.prompt,
            "size": self.size,
            "quality": self.quality,
            "n": self.n,
        });

        let mut req = self
            .client
            .post(&self.url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json");

        if let Some(ref org) = self.organization {
            req = req.header("OpenAI-Organization", org.as_str());
        }

        let resp = req
            .json(&body)
            .send()
            .await
            .map_err(|e| OpenAiError::RequestFailed(e.to_string()))?;

        let json = parse_openai_response(resp).await?;

        let urls = json
            .get("data")
            .and_then(|d| d.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|item| item.get("url").and_then(|u| u.as_str()).map(String::from))
                    .collect()
            })
            .unwrap_or_default();

        Ok(urls)
    }
}

// ─────────────────────────────────────────────
//  Helpers
// ─────────────────────────────────────────────

/// Utility: define a tool/function schema for function calling
pub fn define_tool(
    name: &str,
    description: &str,
    properties: Value,
    required: &[&str],
) -> Value {
    serde_json::json!({
        "type": "function",
        "function": {
            "name": name,
            "description": description,
            "parameters": {
                "type": "object",
                "properties": properties,
                "required": required,
            }
        }
    })
}

/// Cosine similarity between two embedding vectors
pub fn cosine_similarity(a: &[f64], b: &[f64]) -> f64 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }

    let dot: f64 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let mag_a: f64 = a.iter().map(|x| x * x).sum::<f64>().sqrt();
    let mag_b: f64 = b.iter().map(|x| x * x).sum::<f64>().sqrt();

    if mag_a == 0.0 || mag_b == 0.0 {
        0.0
    } else {
        dot / (mag_a * mag_b)
    }
}

async fn parse_openai_response(resp: reqwest::Response) -> Result<Value, OpenAiError> {
    let status = resp.status();
    let text = resp
        .text()
        .await
        .map_err(|e| OpenAiError::ParseError(e.to_string()))?;

    if status.as_u16() == 401 {
        return Err(OpenAiError::InvalidApiKey(text));
    }
    if status.as_u16() == 429 {
        return Err(OpenAiError::RateLimited(text));
    }
    if !status.is_success() {
        return Err(OpenAiError::RequestFailed(format!(
            "HTTP {}: {}",
            status, text
        )));
    }

    serde_json::from_str(&text).map_err(|e| OpenAiError::ParseError(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creation() {
        let client = OpenAiClient::new("sk-test123");
        assert_eq!(client.api_key, "sk-test123");
        assert_eq!(client.base_url, "https://api.openai.com/v1");
    }

    #[test]
    fn test_custom_base_url() {
        let client = OpenAiClient::new("key").with_base_url("https://my-proxy.com/v1");
        assert_eq!(client.base_url, "https://my-proxy.com/v1");
    }

    #[test]
    fn test_chat_builder_body() {
        let client = OpenAiClient::new("key");
        let builder = client
            .chat()
            .model("gpt-4o")
            .system("You are helpful.")
            .user("Hello!")
            .temperature(0.7)
            .max_tokens(100);

        let body = builder.build_body();
        assert_eq!(body["model"], "gpt-4o");
        assert_eq!(body["messages"].as_array().unwrap().len(), 2);
        assert_eq!(body["temperature"], 0.7);
        assert_eq!(body["max_tokens"], 100);
    }

    #[test]
    fn test_chat_builder_json_mode() {
        let client = OpenAiClient::new("key");
        let builder = client.chat().json_mode();
        let body = builder.build_body();
        assert_eq!(body["response_format"]["type"], "json_object");
    }

    #[test]
    fn test_chat_builder_tools() {
        let client = OpenAiClient::new("key");
        let builder = client
            .chat()
            .tool(
                "get_weather",
                "Get the weather",
                serde_json::json!({
                    "type": "object",
                    "properties": {
                        "location": {"type": "string"}
                    }
                }),
            );

        let body = builder.build_body();
        assert!(body.get("tools").is_some());
        assert_eq!(body["tools"][0]["function"]["name"], "get_weather");
    }

    #[test]
    fn test_chat_response_parsing() {
        let json = serde_json::json!({
            "choices": [{
                "message": {
                    "role": "assistant",
                    "content": "Hello! How can I help?"
                },
                "finish_reason": "stop"
            }],
            "usage": {
                "prompt_tokens": 10,
                "completion_tokens": 8,
                "total_tokens": 18
            }
        });

        let resp = ChatResponse { data: json };
        assert_eq!(resp.content(), "Hello! How can I help?");
        assert_eq!(resp.role(), "assistant");
        assert_eq!(resp.finish_reason(), "stop");
        assert_eq!(resp.usage(), (10, 8, 18));
    }

    #[test]
    fn test_tool_calls_parsing() {
        let json = serde_json::json!({
            "choices": [{
                "message": {
                    "role": "assistant",
                    "tool_calls": [{
                        "id": "call_123",
                        "type": "function",
                        "function": {
                            "name": "get_weather",
                            "arguments": "{\"location\":\"Tokyo\"}"
                        }
                    }]
                },
                "finish_reason": "tool_calls"
            }]
        });

        let resp = ChatResponse { data: json };
        let calls = resp.tool_calls();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].function_name, "get_weather");
        assert_eq!(calls[0].id, "call_123");

        let args = calls[0].parse_arguments().unwrap();
        assert_eq!(args["location"], "Tokyo");
    }

    #[test]
    fn test_embeddings_response() {
        let json = serde_json::json!({
            "data": [
                {"embedding": [0.1, 0.2, 0.3], "index": 0},
                {"embedding": [0.4, 0.5, 0.6], "index": 1}
            ],
            "usage": {"total_tokens": 5}
        });

        let resp = EmbeddingsResponse { data: json };
        let embeddings = resp.embeddings();
        assert_eq!(embeddings.len(), 2);
        assert_eq!(embeddings[0], vec![0.1, 0.2, 0.3]);
        assert_eq!(resp.first(), vec![0.1, 0.2, 0.3]);
        assert_eq!(resp.usage(), 5);
    }

    #[test]
    fn test_cosine_similarity() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![1.0, 0.0, 0.0];
        assert!((cosine_similarity(&a, &b) - 1.0).abs() < 1e-10);

        let c = vec![0.0, 1.0, 0.0];
        assert!((cosine_similarity(&a, &c)).abs() < 1e-10);

        let d = vec![-1.0, 0.0, 0.0];
        assert!((cosine_similarity(&a, &d) + 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_cosine_similarity_empty() {
        assert_eq!(cosine_similarity(&[], &[]), 0.0);
        assert_eq!(cosine_similarity(&[1.0], &[1.0, 2.0]), 0.0);
    }

    #[test]
    fn test_define_tool() {
        let tool = define_tool(
            "search",
            "Search the web",
            serde_json::json!({"query": {"type": "string"}}),
            &["query"],
        );
        assert_eq!(tool["function"]["name"], "search");
        assert_eq!(tool["function"]["parameters"]["required"][0], "query");
    }

    #[test]
    fn test_vision_message() {
        let client = OpenAiClient::new("key");
        let builder = client
            .chat()
            .user_with_images("What's in this image?", &["https://example.com/img.png"]);

        let body = builder.build_body();
        let msg = &body["messages"][0];
        assert!(msg["content"].is_array());
        assert_eq!(msg["content"][0]["type"], "text");
        assert_eq!(msg["content"][1]["type"], "image_url");
    }

    #[test]
    fn test_image_builder() {
        let client = OpenAiClient::new("key");
        let builder = client
            .image()
            .prompt("A cute cat")
            .size("1024x1024")
            .quality("hd");

        assert_eq!(builder.prompt, "A cute cat");
        assert_eq!(builder.size, "1024x1024");
        assert_eq!(builder.quality, "hd");
    }
}
