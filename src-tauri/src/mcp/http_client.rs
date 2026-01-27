use serde_json::{json, Value};
use std::time::Duration;
use reqwest;

#[derive(Debug)]
pub struct HttpMcpClient {
    client: reqwest::Client,
    base_url: String,
    session_id: Option<String>,
    oauth_token: Option<String>,
    message_id: std::sync::atomic::AtomicU64,
}

impl HttpMcpClient {
    pub fn new(server_url: String, oauth_token: Option<String>) -> Self {
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
            oauth_token,
            message_id: std::sync::atomic::AtomicU64::new(1),
        }
    }

    fn next_message_id(&self) -> u64 {
        self.message_id.fetch_add(1, std::sync::atomic::Ordering::SeqCst)
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

        let mut request = self.client.post(&self.base_url)
            .header("Content-Type", "application/json")
            .header("Accept", "application/json")
            .json(&request_body);

        // Add OAuth token if configured
        if let Some(ref token) = self.oauth_token {
            request = request.header("Authorization", format!("Bearer {}", token));
        }

        let response = request.send().await?;

        // Extract session ID if present
        if let Some(session_id) = response.headers().get("Mcp-Session-Id") {
            self.session_id = Some(session_id.to_str()?.to_string());
        }

        let response_body: Value = response.json().await?;

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

        let mut request = self.client.post(&self.base_url)
            .header("Content-Type", "application/json");

        // Add session ID if we have one
        if let Some(ref session_id) = self.session_id {
            request = request.header("Mcp-Session-Id", session_id);
        }

        // Add OAuth token if configured
        if let Some(ref token) = self.oauth_token {
            request = request.header("Authorization", format!("Bearer {}", token));
        }

        let response = request.json(&request_body).send().await?;

        if !response.status().is_success() {
            return Err(format!("Failed to send initialized notification: {}", response.status()).into());
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

        let mut request = self.client.post(&self.base_url)
            .header("Content-Type", "application/json")
            .header("Accept", "application/json");

        // Add session ID if we have one
        if let Some(ref session_id) = self.session_id {
            request = request.header("Mcp-Session-Id", session_id);
        }

        // Add OAuth token if configured
        if let Some(ref token) = self.oauth_token {
            request = request.header("Authorization", format!("Bearer {}", token));
        }

        let response = request.json(&request_body).send().await?;
        let response_body: Value = response.json().await?;

        Ok(response_body)
    }

    pub async fn call_tool(&self, tool_name: &str, arguments: Option<Value>) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
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

        let mut request = self.client.post(&self.base_url)
            .header("Content-Type", "application/json")
            .header("Accept", "application/json");

        // Add session ID if we have one
        if let Some(ref session_id) = self.session_id {
            request = request.header("Mcp-Session-Id", session_id);
        }

        // Add OAuth token if configured
        if let Some(ref token) = self.oauth_token {
            request = request.header("Authorization", format!("Bearer {}", token));
        }

        let response = request.json(&request_body).send().await?;
        let response_body: Value = response.json().await?;

        Ok(response_body)
    }

    /// Fetch a resource from the MCP server (used for ui:// resources in MCP Apps)
    pub async fn read_resource(&self, resource_uri: &str) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
        let id = self.next_message_id();

        let request_body = json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": "resources/read",
            "params": {
                "uri": resource_uri
            }
        });

        let mut request = self.client.post(&self.base_url)
            .header("Content-Type", "application/json")
            .header("Accept", "application/json");

        // Add session ID if we have one
        if let Some(ref session_id) = self.session_id {
            request = request.header("Mcp-Session-Id", session_id);
        }

        // Add OAuth token if configured
        if let Some(ref token) = self.oauth_token {
            request = request.header("Authorization", format!("Bearer {}", token));
        }

        let response = request.json(&request_body).send().await?;
        let response_body: Value = response.json().await?;

        Ok(response_body)
    }

    /// List available resources from the MCP server
    pub async fn list_resources(&self) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
        let id = self.next_message_id();

        let request_body = json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": "resources/list",
            "params": {}
        });

        let mut request = self.client.post(&self.base_url)
            .header("Content-Type", "application/json")
            .header("Accept", "application/json");

        // Add session ID if we have one
        if let Some(ref session_id) = self.session_id {
            request = request.header("Mcp-Session-Id", session_id);
        }

        // Add OAuth token if configured
        if let Some(ref token) = self.oauth_token {
            request = request.header("Authorization", format!("Bearer {}", token));
        }

        let response = request.json(&request_body).send().await?;
        let response_body: Value = response.json().await?;

        Ok(response_body)
    }
}