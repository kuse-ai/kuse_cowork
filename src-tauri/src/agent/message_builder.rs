use crate::agent::{AgentConfig, AgentContent, AgentMessage, ToolDefinition};
use crate::mcp::{MCPManager, MCPTool};
use crate::tools;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Debug, Serialize)]
pub struct ClaudeApiRequest {
    pub model: String,
    pub max_tokens: u32,
    pub system: String,
    pub messages: Vec<ApiMessage>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub tools: Vec<ToolDefinition>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    pub stream: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiMessage {
    pub role: String,
    pub content: ApiContent,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ApiContent {
    Text(String),
    Blocks(Vec<serde_json::Value>),
}

pub struct MessageBuilder {
    config: AgentConfig,
    model: String,
    max_tokens: u32,
    temperature: Option<f32>,
    mcp_manager: Option<Arc<MCPManager>>,
}

impl MessageBuilder {
    pub fn new(config: AgentConfig, model: String, max_tokens: u32, temperature: Option<f32>) -> Self {
        Self {
            config,
            model,
            max_tokens,
            temperature,
            mcp_manager: None,
        }
    }

    pub fn with_mcp_manager(mut self, mcp_manager: Arc<MCPManager>) -> Self {
        self.mcp_manager = Some(mcp_manager);
        self
    }

    pub async fn build_request(&self, messages: &[AgentMessage]) -> ClaudeApiRequest {
        let mut tools = tools::get_tools(&self.config.allowed_tools);

        // Add MCP tools if available
        if let Some(mcp_manager) = &self.mcp_manager {
            let mcp_tools = self.get_mcp_tools(mcp_manager).await;
            tools.extend(mcp_tools);
        }

        let api_messages = self.convert_messages(messages);

        ClaudeApiRequest {
            model: self.model.clone(),
            max_tokens: self.max_tokens,
            system: self.config.system_prompt.clone(),
            messages: api_messages,
            tools,
            temperature: self.temperature,
            stream: true,
        }
    }

    async fn get_mcp_tools(&self, mcp_manager: &MCPManager) -> Vec<ToolDefinition> {
        let server_statuses = mcp_manager.get_server_statuses().await;
        let mut mcp_tools = Vec::new();

        for status in server_statuses {
            if matches!(status.status, crate::mcp::types::ConnectionStatus::Connected) {
                for tool in status.tools {
                    mcp_tools.push(Self::convert_mcp_tool_to_definition(&status.id, &status.name, &tool));
                }
            }
        }

        mcp_tools
    }

    fn convert_mcp_tool_to_definition(server_id: &str, server_name: &str, mcp_tool: &MCPTool) -> ToolDefinition {
        let safe_server_id = server_id.replace("-", "_").replace(":", "_");
        let safe_tool_name = mcp_tool.name.replace("-", "_").replace(":", "_");

        ToolDefinition {
            name: format!("mcp_{}_{}", safe_server_id, safe_tool_name),
            description: format!("{} (MCP tool from server '{}')", mcp_tool.description, server_name),
            input_schema: mcp_tool.input_schema.clone(),
        }
    }

    fn convert_messages(&self, messages: &[AgentMessage]) -> Vec<ApiMessage> {
        messages
            .iter()
            .map(|m| {
                let content = match &m.content {
                    AgentContent::Text(text) => ApiContent::Text(text.clone()),
                    AgentContent::Blocks(blocks) => {
                        let json_blocks: Vec<serde_json::Value> = blocks
                            .iter()
                            .map(|b| serde_json::to_value(b).unwrap_or_default())
                            .collect();
                        ApiContent::Blocks(json_blocks)
                    }
                    AgentContent::ToolResults(results) => {
                        let json_results: Vec<serde_json::Value> = results
                            .iter()
                            .map(|r| serde_json::to_value(r).unwrap_or_default())
                            .collect();
                        ApiContent::Blocks(json_results)
                    }
                };

                ApiMessage {
                    role: m.role.clone(),
                    content,
                }
            })
            .collect()
    }
}
