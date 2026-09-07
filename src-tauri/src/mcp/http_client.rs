use serde_json::{json, Value};
use std::collections::HashMap;
use std::time::Duration;
use reqwest;

#[derive(Debug)]
pub struct HttpMcpClient {
    client: reqwest::Client,
    base_url: String,
    session_id: Option<String>,
    oauth_token: Option<String>,
    custom_headers: HashMap<String, String>,
    message_id: std::sync::atomic::AtomicU64,
}

impl HttpMcpClient {
    pub fn new(server_url: String, oauth_token: Option<String>, custom_headers: HashMap<String, String>) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .expect("Failed to create HTTP client");

        // Ensure URL path ends with /mcp, preserving any query parameters
        let base_url = if let Ok(mut parsed) = reqwest::Url::parse(&server_url) {
            if parsed.path().ends_with("/mcp") {
                server_url
            } else {
                let path = parsed.path().trim_end_matches('/');
                parsed.set_path(&format!("{}/mcp", path));
                parsed.to_string()
            }
        } else {
            server_url
        };

        Self {
            client,
            base_url,
            session_id: None,
            oauth_token,
            custom_headers,
            message_id: std::sync::atomic::AtomicU64::new(1),
        }
    }

    fn next_message_id(&self) -> u64 {
        self.message_id.fetch_add(1, std::sync::atomic::Ordering::SeqCst)
    }

    /// Parse response body, handling both JSON and SSE (text/event-stream) content types.
    /// For SSE, extracts the last JSON-RPC message from `data:` lines.
    async fn parse_response(&self, response: reqwest::Response) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
        let is_sse = response.headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .is_some_and(|ct| ct.contains("text/event-stream"));

        if is_sse {
            let body = response.text().await?;
            // Parse SSE: find the last `data:` line containing a JSON-RPC response (has "id" field)
            let mut last_message: Option<Value> = None;
            for line in body.lines() {
                if let Some(data) = line.strip_prefix("data:") {
                    let data = data.trim();
                    if data.is_empty() {
                        continue;
                    }
                    if let Ok(parsed) = serde_json::from_str::<Value>(data) {
                        // Prefer messages with an "id" field (JSON-RPC responses over notifications)
                        if parsed.get("id").is_some() {
                            last_message = Some(parsed);
                        } else if last_message.is_none() {
                            last_message = Some(parsed);
                        }
                    }
                }
            }
            last_message.ok_or_else(|| "No JSON-RPC message found in SSE stream".into())
        } else {
            Ok(response.json().await?)
        }
    }

    fn apply_headers(&self, mut request: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        if let Some(ref session_id) = self.session_id {
            request = request.header("Mcp-Session-Id", session_id);
        }
        if let Some(ref token) = self.oauth_token {
            request = request.header("Authorization", format!("Bearer {}", token));
        }
        for (key, value) in &self.custom_headers {
            request = request.header(key, value);
        }
        request
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

        let request = self.client.post(&self.base_url)
            .header("Content-Type", "application/json")
            .header("Accept", "application/json, text/event-stream")
            .json(&request_body);

        let request = self.apply_headers(request);

        let response = request.send().await?;

        // Extract session ID if present
        if let Some(session_id) = response.headers().get("Mcp-Session-Id") {
            self.session_id = Some(session_id.to_str()?.to_string());
        }

        let response_body = self.parse_response(response).await?;

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

        let request = self.client.post(&self.base_url)
            .header("Content-Type", "application/json")
            .header("Accept", "application/json, text/event-stream");

        let request = self.apply_headers(request);

        let response = request.json(&request_body).send().await?;

        // Notifications may return 202 Accepted or 204 No Content
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

        let request = self.client.post(&self.base_url)
            .header("Content-Type", "application/json")
            .header("Accept", "application/json, text/event-stream");

        let request = self.apply_headers(request);

        let response = request.json(&request_body).send().await?;
        let response_body = self.parse_response(response).await?;

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

        let request = self.client.post(&self.base_url)
            .header("Content-Type", "application/json")
            .header("Accept", "application/json, text/event-stream");

        let request = self.apply_headers(request);

        let response = request.json(&request_body).send().await?;
        let response_body = self.parse_response(response).await?;

        Ok(response_body)
    }
}