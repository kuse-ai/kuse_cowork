use super::http_client::HttpMcpClient;
use super::types::*;
use std::collections::HashMap;
use tokio::sync::RwLock;
use std::sync::Arc;

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

    pub async fn connect_server(&self, config: &MCPServerConfig) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if !config.enabled {
            return Err("Server is not enabled".into());
        }

        // Update status to connecting
        {
            let mut status_map = self.server_status.write().await;
            status_map.insert(config.id.clone(), MCPServerStatus {
                id: config.id.clone(),
                name: config.name.clone(),
                status: ConnectionStatus::Connecting,
                tools: vec![],
                last_error: None,
            });
        }

        // Create OAuth token if needed
        let oauth_token = if let (Some(client_id), Some(client_secret)) =
            (&config.oauth_client_id, &config.oauth_client_secret) {
            match self.perform_oauth_flow(client_id, client_secret, &config.server_url).await {
                Ok(token) => Some(token),
                Err(e) => {
                    self.update_status_error(&config.id, format!("OAuth failed: {}", e)).await;
                    return Err(e);
                }
            }
        } else {
            None
        };

        // Create HTTP MCP client
        let mut http_client = HttpMcpClient::new(config.server_url.clone(), oauth_token);

        // Initialize the connection
        match http_client.initialize().await {
            Ok(_) => {
                // Initialization successful
            }
            Err(e) => {
                let error_msg = format!("Connection failed: {}", e);
                self.update_status_error(&config.id, error_msg.clone()).await;
                return Err(error_msg.into());
            }
        }

        // Discover tools
        let tools = match self.discover_tools_http(&http_client, &config.id).await {
            Ok(tools) => tools,
            Err(e) => {
                let error_msg = format!("Tool discovery failed: {}", e);
                self.update_status_error(&config.id, error_msg.clone()).await;
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
            status_map.insert(config.id.clone(), MCPServerStatus {
                id: config.id.clone(),
                name: config.name.clone(),
                status: ConnectionStatus::Connected,
                tools,
                last_error: None,
            });
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
                    client.http_client.call_tool(&call.tool_name, Some(call.parameters.clone()))
                ).await {
                    Ok(Ok(response)) => {
                        // Parse the JSON-RPC response
                        // Look up the tool to check if it has UI metadata
                        let ui_resource_uri = {
                            let status_map = self.server_status.read().await;
                            status_map.get(&call.server_id)
                                .and_then(|status| {
                                    status.tools.iter()
                                        .find(|t| t.name == call.tool_name)
                                        .and_then(|t| t.meta.as_ref())
                                        .and_then(|m| m.ui.as_ref())
                                        .map(|ui| ui.resource_uri.clone())
                                })
                        };

                        if let Some(error) = response.get("error") {
                            MCPToolResult {
                                success: false,
                                result: serde_json::Value::Null,
                                error: Some(format!("Tool execution error: {}", error)),
                                ui_resource_uri: None,
                            }
                        } else if let Some(result) = response.get("result") {
                            MCPToolResult {
                                success: true,
                                result: result.clone(),
                                error: None,
                                ui_resource_uri,
                            }
                        } else {
                            MCPToolResult {
                                success: false,
                                result: serde_json::Value::Null,
                                error: Some("Invalid response format".to_string()),
                                ui_resource_uri: None,
                            }
                        }
                    },
                    Ok(Err(e)) => {
                        MCPToolResult {
                            success: false,
                            result: serde_json::Value::Null,
                            error: Some(format!("Tool execution failed: {}", e)),
                            ui_resource_uri: None,
                        }
                    },
                    Err(_) => {
                        MCPToolResult {
                            success: false,
                            result: serde_json::Value::Null,
                            error: Some("Tool execution timed out after 60 seconds".to_string()),
                            ui_resource_uri: None,
                        }
                    }
                }
            }
            None => MCPToolResult {
                success: false,
                result: serde_json::Value::Null,
                error: Some(format!("Server {} not connected", call.server_id)),
                ui_resource_uri: None,
            }
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

    async fn discover_tools_http(&self, client: &HttpMcpClient, server_id: &str) -> Result<Vec<MCPTool>, Box<dyn std::error::Error + Send + Sync>> {
        let tools_response = client.list_tools().await?;

        let mut mcp_tools = Vec::new();

        if let Some(result) = tools_response.get("result") {
            if let Some(tools_array) = result.get("tools") {
                if let Some(tools) = tools_array.as_array() {
                    for tool in tools {
                        let name = tool.get("name")
                            .and_then(|n| n.as_str())
                            .unwrap_or("")
                            .to_string();

                        let description = tool.get("description")
                            .and_then(|d| d.as_str())
                            .unwrap_or("")
                            .to_string();

                        let input_schema = tool.get("inputSchema")
                            .cloned()
                            .unwrap_or(serde_json::Value::Object(serde_json::Map::new()));

                        // Parse MCP Apps UI metadata if present
                        let meta = tool.get("_meta").and_then(|meta_val| {
                            let ui = meta_val.get("ui").and_then(|ui_val| {
                                let resource_uri = ui_val.get("resourceUri")
                                    .and_then(|v| v.as_str())
                                    .map(|s| s.to_string())?;

                                let permissions = ui_val.get("permissions")
                                    .and_then(|v| v.as_array())
                                    .map(|arr| {
                                        arr.iter()
                                            .filter_map(|v| v.as_str().map(|s| s.to_string()))
                                            .collect()
                                    });

                                let csp = ui_val.get("csp")
                                    .and_then(|v| v.as_str())
                                    .map(|s| s.to_string());

                                Some(MCPToolUI {
                                    resource_uri,
                                    permissions,
                                    csp,
                                })
                            });

                            if ui.is_some() {
                                Some(MCPToolMeta { ui })
                            } else {
                                None
                            }
                        });

                        mcp_tools.push(MCPTool {
                            server_id: server_id.to_string(),
                            name,
                            description,
                            input_schema,
                            meta,
                        });
                    }
                }
            }
        }

        Ok(mcp_tools)
    }

    /// Fetch a UI resource from an MCP server (for MCP Apps)
    pub async fn fetch_ui_resource(&self, server_id: &str, resource_uri: &str) -> Result<MCPResourceResponse, Box<dyn std::error::Error + Send + Sync>> {
        let clients = self.clients.read().await;

        let client = clients.get(server_id)
            .ok_or_else(|| format!("Server {} not connected", server_id))?;

        let response = client.http_client.read_resource(resource_uri).await?;

        // Parse the response
        if let Some(error) = response.get("error") {
            return Err(format!("Resource fetch error: {}", error).into());
        }

        let result = response.get("result")
            .ok_or("Invalid response: missing result")?;

        let contents_array = result.get("contents")
            .and_then(|c| c.as_array())
            .ok_or("Invalid response: missing contents array")?;

        let mut contents = Vec::new();
        for content in contents_array {
            let uri = content.get("uri")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            let mime_type = content.get("mimeType")
                .and_then(|v| v.as_str())
                .unwrap_or("text/html")
                .to_string();

            let text = content.get("text")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());

            let blob = content.get("blob")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());

            contents.push(MCPResourceContent {
                uri,
                mime_type,
                text,
                blob,
            });
        }

        Ok(MCPResourceResponse { contents })
    }

    /// Get tool by name from a specific server
    pub async fn get_tool(&self, server_id: &str, tool_name: &str) -> Option<MCPTool> {
        let status_map = self.server_status.read().await;
        status_map.get(server_id)
            .and_then(|status| {
                status.tools.iter()
                    .find(|t| t.name == tool_name)
                    .cloned()
            })
    }

    /// Get all tools that have MCP Apps UI support
    pub async fn get_app_tools(&self) -> Vec<MCPTool> {
        let status_map = self.server_status.read().await;
        let mut tools = Vec::new();

        for status in status_map.values() {
            if matches!(status.status, ConnectionStatus::Connected) {
                for tool in &status.tools {
                    if tool.meta.as_ref().and_then(|m| m.ui.as_ref()).is_some() {
                        tools.push(tool.clone());
                    }
                }
            }
        }

        tools
    }



    async fn perform_oauth_flow(&self, client_id: &str, client_secret: &str, server_url: &str) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        // Create OAuth endpoint URL
        let mut oauth_url = if server_url.ends_with("/mcp") {
            server_url.trim_end_matches("/mcp").to_string()
        } else {
            server_url.to_string()
        };

        if !oauth_url.ends_with('/') {
            oauth_url.push('/');
        }
        oauth_url.push_str("oauth/token");

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
                .send()
        ).await {
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
            Ok(format!("Bearer {}", access_token))
        } else {
            Err("No access_token in OAuth response".into())
        }
    }
}

impl Default for MCPManager {
    fn default() -> Self {
        Self::new()
    }
}