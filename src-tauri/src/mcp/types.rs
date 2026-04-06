use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MCPServerConfig {
    pub id: String,
    pub name: String,
    pub server_url: String,
    #[serde(default = "default_auth_type")]
    pub auth_type: String,
    #[serde(default)]
    pub bearer_token: Option<String>,
    #[serde(default)]
    pub oauth_client_id: Option<String>,
    #[serde(default)]
    pub oauth_client_secret: Option<String>,
    #[serde(default)]
    pub custom_headers: HashMap<String, String>,
    #[serde(default)]
    pub custom_headers_updated: bool,
    #[serde(default)]
    pub has_bearer_token: bool,
    #[serde(default)]
    pub has_oauth_client_secret: bool,
    #[serde(default)]
    pub custom_header_keys: Vec<String>,
    pub enabled: bool,
    pub created_at: String,
    pub updated_at: String,
}

fn default_auth_type() -> String {
    "none".to_string()
}

impl MCPServerConfig {
    pub fn sanitized(&self) -> Self {
        let mut sanitized = self.clone();
        sanitized.has_bearer_token = self
            .bearer_token
            .as_ref()
            .is_some_and(|value| !value.trim().is_empty());
        sanitized.has_oauth_client_secret = self
            .oauth_client_secret
            .as_ref()
            .is_some_and(|value| !value.trim().is_empty());
        sanitized.custom_header_keys = self.custom_headers.keys().cloned().collect();
        sanitized.custom_header_keys.sort();
        sanitized.bearer_token = None;
        sanitized.oauth_client_secret = None;
        sanitized.custom_headers = HashMap::new();
        sanitized.custom_headers_updated = false;
        sanitized
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MCPTool {
    pub server_id: String,
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MCPServerStatus {
    pub id: String,
    pub name: String,
    pub status: ConnectionStatus,
    pub tools: Vec<MCPTool>,
    pub last_error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConnectionStatus {
    Connected,
    Disconnected,
    Connecting,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MCPToolCall {
    pub server_id: String,
    pub tool_name: String,
    pub parameters: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MCPToolResult {
    pub success: bool,
    pub result: serde_json::Value,
    pub error: Option<String>,
}
