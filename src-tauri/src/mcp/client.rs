use super::http_client::HttpMcpClient;
use super::types::*;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct MCPClient {
    http_client: HttpMcpClient,
    #[allow(dead_code)]
    url: String,
}

pub struct MCPManager {
    clients: Arc<RwLock<HashMap<String, MCPClient>>>,
    server_status: Arc<RwLock<HashMap<String, MCPServerStatus>>>,
}

impl MCPManager {
    pub fn new() -> Self {
        Self {
            clients: Arc::new(RwLock::new(HashMap::new())),
            server_status: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn connect_server(
        &self,
        config: &MCPServerConfig,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if !config.enabled {
            return Err("Server is not enabled".into());
        }

        // Update status to connecting
        {
            let mut status_map = self.server_status.write().await;
            status_map.insert(
                config.id.clone(),
                MCPServerStatus {
                    id: config.id.clone(),
                    name: config.name.clone(),
                    status: ConnectionStatus::Connecting,
                    tools: vec![],
                    last_error: None,
                },
            );
        }

        // Create OAuth token if needed
        let auth_token = match config.auth_type.as_str() {
            "bearer" => {
                let token = config
                    .bearer_token
                    .clone()
                    .filter(|value| !value.trim().is_empty());
                if token.is_none() {
                    let error = "Bearer token is required for bearer auth".to_string();
                    self.update_status_error(&config.id, error.clone()).await;
                    return Err(error.into());
                }
                token
            }
            "oauth_client_credentials" => {
                if let (Some(client_id), Some(client_secret)) =
                    (&config.oauth_client_id, &config.oauth_client_secret)
                {
                    match self
                        .perform_oauth_flow(client_id, client_secret, &config.server_url)
                        .await
                    {
                        Ok(token) => Some(token),
                        Err(e) => {
                            self.update_status_error(&config.id, format!("OAuth failed: {}", e))
                                .await;
                            return Err(e);
                        }
                    }
                } else {
                    let error =
                        "OAuth client ID and secret are required for OAuth auth".to_string();
                    self.update_status_error(&config.id, error.clone()).await;
                    return Err(error.into());
                }
            }
            _ => None,
        };

        // Create HTTP MCP client
        let mut http_client = HttpMcpClient::new(
            config.server_url.clone(),
            auth_token,
            config.custom_headers.clone(),
        );

        // Initialize the connection
        match http_client.initialize().await {
            Ok(_) => {
                // Initialization successful
            }
            Err(e) => {
                let error_msg = format!("Connection failed: {}", e);
                self.update_status_error(&config.id, error_msg.clone())
                    .await;
                return Err(error_msg.into());
            }
        }

        // Discover tools
        let tools = match self.discover_tools_http(&http_client, &config.id).await {
            Ok(tools) => tools,
            Err(e) => {
                let error_msg = format!("Tool discovery failed: {}", e);
                self.update_status_error(&config.id, error_msg.clone())
                    .await;
                return Err(error_msg.into());
            }
        };

        // Store client and update status to connected
        let mcp_client = MCPClient {
            http_client,
            url: config.server_url.clone(),
        };

        {
            let mut clients = self.clients.write().await;
            clients.insert(config.id.clone(), mcp_client);
        }

        {
            let mut status_map = self.server_status.write().await;
            status_map.insert(
                config.id.clone(),
                MCPServerStatus {
                    id: config.id.clone(),
                    name: config.name.clone(),
                    status: ConnectionStatus::Connected,
                    tools,
                    last_error: None,
                },
            );
        }

        Ok(())
    }

    pub async fn disconnect_server(&self, server_id: &str) {
        // Remove client
        {
            let mut clients = self.clients.write().await;
            clients.remove(server_id);
        }

        // Update status to disconnected
        {
            let mut status_map = self.server_status.write().await;
            if let Some(status) = status_map.get_mut(server_id) {
                status.status = ConnectionStatus::Disconnected;
                status.tools.clear();
                status.last_error = None;
            }
        }
    }

    pub async fn execute_tool(&self, call: &MCPToolCall) -> MCPToolResult {
        let clients = self.clients.read().await;

        match clients.get(&call.server_id) {
            Some(client) => {
                // Execute tool with timeout
                match tokio::time::timeout(
                    std::time::Duration::from_secs(60),
                    client
                        .http_client
                        .call_tool(&call.tool_name, Some(call.parameters.clone())),
                )
                .await
                {
                    Ok(Ok(response)) => {
                        // Parse the JSON-RPC response
                        if let Some(error) = response.get("error") {
                            MCPToolResult {
                                success: false,
                                result: serde_json::Value::Null,
                                error: Some(format!("Tool execution error: {}", error)),
                            }
                        } else if let Some(result) = response.get("result") {
                            MCPToolResult {
                                success: true,
                                result: result.clone(),
                                error: None,
                            }
                        } else {
                            MCPToolResult {
                                success: false,
                                result: serde_json::Value::Null,
                                error: Some("Invalid response format".to_string()),
                            }
                        }
                    }
                    Ok(Err(e)) => MCPToolResult {
                        success: false,
                        result: serde_json::Value::Null,
                        error: Some(format!("Tool execution failed: {}", e)),
                    },
                    Err(_) => MCPToolResult {
                        success: false,
                        result: serde_json::Value::Null,
                        error: Some("Tool execution timed out after 60 seconds".to_string()),
                    },
                }
            }
            None => MCPToolResult {
                success: false,
                result: serde_json::Value::Null,
                error: Some(format!("Server {} not connected", call.server_id)),
            },
        }
    }

    pub async fn get_all_tools(&self) -> Vec<MCPTool> {
        let status_map = self.server_status.read().await;
        let mut tools = Vec::new();

        for status in status_map.values() {
            if matches!(status.status, ConnectionStatus::Connected) {
                tools.extend(status.tools.clone());
            }
        }

        tools
    }

    pub async fn get_server_statuses(&self) -> Vec<MCPServerStatus> {
        let status_map = self.server_status.read().await;
        status_map.values().cloned().collect()
    }

    async fn update_status_error(&self, server_id: &str, error: String) {
        let mut status_map = self.server_status.write().await;
        if let Some(status) = status_map.get_mut(server_id) {
            status.status = ConnectionStatus::Error;
            status.last_error = Some(error);
        }
    }

    async fn discover_tools_http(
        &self,
        client: &HttpMcpClient,
        server_id: &str,
    ) -> Result<Vec<MCPTool>, Box<dyn std::error::Error + Send + Sync>> {
        let tools_response = client.list_tools().await?;

        let mut mcp_tools = Vec::new();

        if let Some(result) = tools_response.get("result") {
            if let Some(tools_array) = result.get("tools") {
                if let Some(tools) = tools_array.as_array() {
                    for tool in tools {
                        let name = tool
                            .get("name")
                            .and_then(|n| n.as_str())
                            .unwrap_or("")
                            .to_string();

                        let description = tool
                            .get("description")
                            .and_then(|d| d.as_str())
                            .unwrap_or("")
                            .to_string();

                        let input_schema = tool
                            .get("inputSchema")
                            .cloned()
                            .unwrap_or(serde_json::Value::Object(serde_json::Map::new()));

                        mcp_tools.push(MCPTool {
                            server_id: server_id.to_string(),
                            name,
                            description,
                            input_schema,
                        });
                    }
                }
            }
        }

        Ok(mcp_tools)
    }

    async fn perform_oauth_flow(
        &self,
        client_id: &str,
        client_secret: &str,
        server_url: &str,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let oauth_url = self.discover_oauth_token_url(server_url).await?;

        // Create HTTP client for OAuth request
        let client = reqwest::Client::new();

        // Prepare OAuth request body (Client Credentials Grant)
        let params = [
            ("grant_type", "client_credentials"),
            ("client_id", client_id),
            ("client_secret", client_secret),
        ];

        // Send OAuth token request with timeout
        let response = match tokio::time::timeout(
            std::time::Duration::from_secs(15),
            client
                .post(&oauth_url)
                .form(&params)
                .header("Content-Type", "application/x-www-form-urlencoded")
                .send(),
        )
        .await
        {
            Ok(Ok(response)) => response,
            Ok(Err(e)) => {
                return Err(format!("OAuth request failed: {}", e).into());
            }
            Err(_) => {
                return Err("OAuth request timed out after 15 seconds".into());
            }
        };

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(format!("OAuth request failed: {} - {}", status, error_text).into());
        }

        // Parse OAuth response
        let oauth_response: serde_json::Value = response.json().await?;

        // Extract access token
        if let Some(access_token) = oauth_response.get("access_token").and_then(|v| v.as_str()) {
            Ok(access_token.to_string())
        } else {
            Err("No access_token in OAuth response".into())
        }
    }

    async fn discover_oauth_token_url(
        &self,
        server_url: &str,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let resource_url = server_url.trim_end_matches('/').to_string();
        let base_url = if resource_url.ends_with("/mcp") {
            resource_url.trim_end_matches("/mcp").to_string()
        } else {
            resource_url.clone()
        };

        let authorization_metadata_urls = [
            format!("{}/.well-known/oauth-authorization-server", base_url),
            format!("{}/.well-known/oauth-authorization-server/mcp", base_url),
            format!("{}/.well-known/openid-configuration", base_url),
            format!("{}/.well-known/openid-configuration/mcp", base_url),
        ];
        let protected_resource_urls = [
            format!("{}/.well-known/oauth-protected-resource", base_url),
            format!("{}/.well-known/oauth-protected-resource/mcp", base_url),
        ];

        let client = reqwest::Client::new();

        for metadata_url in authorization_metadata_urls {
            if let Some(token_endpoint) = self
                .fetch_token_endpoint_from_metadata(&client, &metadata_url)
                .await?
            {
                return Ok(token_endpoint);
            }
        }

        for protected_resource_url in protected_resource_urls {
            if let Some(authorization_servers) = self
                .fetch_authorization_servers(&client, &protected_resource_url)
                .await?
            {
                for authorization_server in authorization_servers {
                    for metadata_url in [
                        format!(
                            "{}/.well-known/oauth-authorization-server",
                            authorization_server.trim_end_matches('/')
                        ),
                        authorization_server,
                    ] {
                        if let Some(token_endpoint) = self
                            .fetch_token_endpoint_from_metadata(&client, &metadata_url)
                            .await?
                        {
                            return Ok(token_endpoint);
                        }
                    }
                }
            }
        }

        Ok(format!("{}/oauth/token", base_url))
    }

    async fn fetch_token_endpoint_from_metadata(
        &self,
        client: &reqwest::Client,
        metadata_url: &str,
    ) -> Result<Option<String>, Box<dyn std::error::Error + Send + Sync>> {
        let response = client
            .get(metadata_url)
            .header("Accept", "application/json")
            .send()
            .await;

        if let Ok(response) = response {
            if !response.status().is_success() {
                return Ok(None);
            }

            let body: serde_json::Value = response.json().await?;
            if let Some(token_endpoint) = body.get("token_endpoint").and_then(|v| v.as_str()) {
                return Ok(Some(token_endpoint.to_string()));
            }
        }

        Ok(None)
    }

    async fn fetch_authorization_servers(
        &self,
        client: &reqwest::Client,
        metadata_url: &str,
    ) -> Result<Option<Vec<String>>, Box<dyn std::error::Error + Send + Sync>> {
        let response = client
            .get(metadata_url)
            .header("Accept", "application/json")
            .send()
            .await;

        if let Ok(response) = response {
            if !response.status().is_success() {
                return Ok(None);
            }

            let body: serde_json::Value = response.json().await?;
            if let Some(servers) = body
                .get("authorization_servers")
                .and_then(|value| value.as_array())
            {
                let parsed = servers
                    .iter()
                    .filter_map(|value| value.as_str().map(str::to_string))
                    .collect::<Vec<_>>();
                if !parsed.is_empty() {
                    return Ok(Some(parsed));
                }
            }
        }

        Ok(None)
    }
}

impl Default for MCPManager {
    fn default() -> Self {
        Self::new()
    }
}
