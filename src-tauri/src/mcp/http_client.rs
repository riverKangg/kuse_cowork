use reqwest::{self, Response};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::time::Duration;

const MCP_ACCEPT_JSON_ONLY: &str = "application/json";
const MCP_ACCEPT_WITH_SSE: &str = "application/json, text/event-stream";

#[derive(Debug)]
pub struct HttpMcpClient {
    client: reqwest::Client,
    base_url: String,
    session_id: Option<String>,
    auth_token: Option<String>,
    custom_headers: HashMap<String, String>,
    message_id: std::sync::atomic::AtomicU64,
}

impl HttpMcpClient {
    pub fn new(
        server_url: String,
        auth_token: Option<String>,
        custom_headers: HashMap<String, String>,
    ) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .expect("Failed to create HTTP client");

        // Ensure URL ends with /mcp
        let base_url = if server_url.ends_with("/mcp") {
            server_url
        } else if server_url.ends_with("/") {
            format!("{}mcp", server_url)
        } else {
            format!("{}/mcp", server_url)
        };

        Self {
            client,
            base_url,
            session_id: None,
            auth_token,
            custom_headers,
            message_id: std::sync::atomic::AtomicU64::new(1),
        }
    }

    fn next_message_id(&self) -> u64 {
        self.message_id
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst)
    }

    pub async fn initialize(&mut self) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
        let id = self.next_message_id();

        let request_body = json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": "initialize",
            "params": {
                "protocolVersion": "2024-11-05",
                "capabilities": {},
                "clientInfo": {
                    "name": "kuse-cowork",
                    "title": "Kuse Cowork Desktop",
                    "version": "0.1.0"
                }
            }
        });

        let response = self.send_with_accept_fallback(&request_body).await?;

        // Extract session ID if present
        if let Some(session_id) = response.headers().get("Mcp-Session-Id") {
            self.session_id = Some(session_id.to_str()?.to_string());
        }

        let response_body = self.parse_response_body(response).await?;

        // Send initialized notification
        self.send_initialized().await?;

        Ok(response_body)
    }

    async fn send_initialized(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let request_body = json!({
            "jsonrpc": "2.0",
            "method": "notifications/initialized",
            "params": {}
        });

        let response = self.send_with_accept_fallback(&request_body).await?;

        if !response.status().is_success() {
            return Err(format!(
                "Failed to send initialized notification: {}",
                response.status()
            )
            .into());
        }

        Ok(())
    }

    pub async fn list_tools(&self) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
        let id = self.next_message_id();

        let request_body = json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": "tools/list",
            "params": {}
        });

        let response = self.send_with_accept_fallback(&request_body).await?;
        let response_body = self.parse_response_body(response).await?;

        Ok(response_body)
    }

    pub async fn call_tool(
        &self,
        tool_name: &str,
        arguments: Option<Value>,
    ) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
        let id = self.next_message_id();

        let request_body = json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": "tools/call",
            "params": {
                "name": tool_name,
                "arguments": arguments.unwrap_or(json!({}))
            }
        });

        let response = self.send_with_accept_fallback(&request_body).await?;
        let response_body = self.parse_response_body(response).await?;

        Ok(response_body)
    }

    fn apply_auth(&self, mut request: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        if let Some(ref token) = self.auth_token {
            request = request.header("Authorization", format!("Bearer {}", token));
        }
        request
    }

    fn apply_custom_headers(
        &self,
        mut request: reqwest::RequestBuilder,
    ) -> reqwest::RequestBuilder {
        for (key, value) in &self.custom_headers {
            request = request.header(key, value);
        }
        request
    }

    fn build_json_request(
        &self,
        request_body: &Value,
        accept_header: &str,
    ) -> reqwest::RequestBuilder {
        let mut request = self
            .client
            .post(&self.base_url)
            .header("Content-Type", "application/json")
            .header("Accept", accept_header);

        if let Some(ref session_id) = self.session_id {
            request = request.header("Mcp-Session-Id", session_id);
        }

        request = self.apply_auth(request);
        request = self.apply_custom_headers(request);
        request.json(request_body)
    }

    async fn send_with_accept_fallback(
        &self,
        request_body: &Value,
    ) -> Result<Response, Box<dyn std::error::Error + Send + Sync>> {
        let first_response = self
            .build_json_request(request_body, MCP_ACCEPT_JSON_ONLY)
            .send()
            .await?;

        if first_response.status() != reqwest::StatusCode::NOT_ACCEPTABLE {
            return Ok(first_response);
        }

        let response_text = first_response.text().await?;
        if !response_text.contains("text/event-stream") {
            return Err(format!("MCP request failed with 406: {}", response_text).into());
        }

        let retry_request = self.build_json_request(request_body, MCP_ACCEPT_WITH_SSE);
        Ok(retry_request.send().await?)
    }

    async fn parse_response_body(
        &self,
        response: Response,
    ) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
        let status = response.status();
        let content_type = response
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .unwrap_or("")
            .to_string();

        if content_type.contains("text/event-stream") {
            let body = response.text().await?;
            if !status.is_success() {
                return Err(format!("HTTP {}: {}", status, body).into());
            }
            return Self::parse_sse_json(&body);
        }

        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(format!("HTTP {}: {}", status, body).into());
        }

        Ok(response.json().await?)
    }

    fn parse_sse_json(body: &str) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
        let mut data_lines = Vec::new();

        for line in body.lines() {
            if let Some(rest) = line.strip_prefix("data:") {
                data_lines.push(rest.trim());
            }
        }

        if data_lines.is_empty() {
            return Err(format!("No JSON data found in SSE response: {}", body).into());
        }

        let payload = data_lines.join("\n");
        Ok(serde_json::from_str(&payload)?)
    }
}
