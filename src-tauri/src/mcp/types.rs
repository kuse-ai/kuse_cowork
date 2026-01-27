use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MCPServerConfig {
    pub id: String,
    pub name: String,
    pub server_url: String,
    pub oauth_client_id: Option<String>,
    pub oauth_client_secret: Option<String>,
    pub enabled: bool,
    pub created_at: String,
    pub updated_at: String,
}

/// MCP Apps UI configuration for tools that render interactive UIs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MCPToolUI {
    /// The resource URI for the UI (ui:// scheme)
    #[serde(rename = "resourceUri")]
    pub resource_uri: String,
    /// Optional permissions requested by the app (e.g., ["microphone", "camera"])
    pub permissions: Option<Vec<String>>,
    /// Content Security Policy for loading external resources
    pub csp: Option<String>,
}

/// Tool metadata including UI configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MCPToolMeta {
    /// UI configuration for MCP Apps
    pub ui: Option<MCPToolUI>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MCPTool {
    pub server_id: String,
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
    /// Tool metadata including MCP Apps UI configuration
    #[serde(rename = "_meta")]
    pub meta: Option<MCPToolMeta>,
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
    /// UI resource URI if the tool supports MCP Apps
    pub ui_resource_uri: Option<String>,
}

/// MCP Resource content for ui:// resources
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MCPResourceContent {
    /// The resource URI
    pub uri: String,
    /// MIME type (typically "text/html; mcp-ui-root=true" for MCP Apps)
    #[serde(rename = "mimeType")]
    pub mime_type: String,
    /// The HTML content
    pub text: Option<String>,
    /// Binary content (base64 encoded)
    pub blob: Option<String>,
}

/// MCP Resource response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MCPResourceResponse {
    pub contents: Vec<MCPResourceContent>,
}

/// Request to fetch an MCP App resource
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MCPResourceRequest {
    pub server_id: String,
    pub resource_uri: String,
}

/// MCP App instance for rendering
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MCPAppInstance {
    /// Unique instance ID
    pub id: String,
    /// Server ID that provided this app
    pub server_id: String,
    /// Tool name that triggered this app
    pub tool_name: String,
    /// The HTML content to render
    pub html_content: String,
    /// Tool result data to pass to the app
    pub tool_result: serde_json::Value,
    /// Permissions granted to this app
    pub permissions: Vec<String>,
    /// CSP policy for the app
    pub csp: Option<String>,
}