use crate::agent::{ToolResult, ToolUse};
use crate::mcp::{MCPManager, MCPToolCall};
use crate::tools;
use std::sync::Arc;

/// Execute an Excel query tool
fn execute_excel_tool(input: &serde_json::Value, tool_type: &str) -> Result<String, String> {
    // Get required file_path from input
    let file_path = input
        .get("file_path")
        .and_then(|v| v.as_str())
        .ok_or("Missing 'file_path' parameter. The Excel file path must be provided.")?;

    let sheet = input.get("sheet").and_then(|v| v.as_str());

    match tool_type {
        "filter" => tools::excel::execute_filter(input, file_path, sheet),
        "search" => tools::excel::execute_search(input, file_path, sheet),
        "aggregate" => tools::excel::execute_aggregate(input, file_path, sheet),
        _ => Err(format!("Unknown Excel tool type: {}", tool_type)),
    }
}

pub struct ToolExecutor {
    project_path: Option<String>,
    mcp_manager: Option<Arc<MCPManager>>,
}

impl ToolExecutor {
    pub fn new(project_path: Option<String>) -> Self {
        Self {
            project_path,
            mcp_manager: None,
        }
    }

    pub fn with_mcp_manager(mut self, mcp_manager: Arc<MCPManager>) -> Self {
        self.mcp_manager = Some(mcp_manager);
        self
    }

    pub async fn execute(&self, tool_use: &ToolUse) -> ToolResult {
        let project_path = self.project_path.as_deref();

        // Check if this is an MCP tool (format: mcp_server_id_tool_name)
        if tool_use.name.starts_with("mcp_") {
            if let Some(mcp_manager) = &self.mcp_manager {
                // Get all available tools to find the correct mapping
                let all_tools = mcp_manager.get_all_tools().await;
                let mut matching_tool = None;

                // Find the tool that matches the current tool_use name
                for tool in &all_tools {
                    let safe_server_id = tool.server_id.replace("-", "_").replace(":", "_");
                    let safe_tool_name = tool.name.replace("-", "_").replace(":", "_");
                    let expected_name = format!("mcp_{}_{}", safe_server_id, safe_tool_name);

                    if expected_name == tool_use.name {
                        matching_tool = Some(tool);
                        break;
                    }
                }

                if let Some(tool) = matching_tool {
                    let mcp_call = MCPToolCall {
                        server_id: tool.server_id.clone(),
                        tool_name: tool.name.clone(),
                        parameters: tool_use.input.clone(),
                    };

                    let mcp_result = mcp_manager.execute_tool(&mcp_call).await;

                    return if mcp_result.success {
                        ToolResult::success(tool_use.id.clone(), mcp_result.result.to_string())
                    } else {
                        ToolResult::error(
                            tool_use.id.clone(),
                            mcp_result.error.unwrap_or("MCP tool execution failed".to_string())
                        )
                    };
                } else {
                    return ToolResult::error(
                        tool_use.id.clone(),
                        format!("MCP tool '{}' not found", tool_use.name)
                    );
                }
            } else {
                return ToolResult::error(
                    tool_use.id.clone(),
                    "MCP manager not available".to_string()
                );
            }
        }

        // Docker tools have their own result handling
        if tool_use.name.starts_with("docker_") {
            return tools::docker::execute_docker_tool(tool_use, &self.project_path);
        }

        let result = match tool_use.name.as_str() {
            "read_file" => tools::file_read::execute(&tool_use.input, project_path),
            "write_file" => tools::file_write::execute(&tool_use.input, project_path),
            "edit_file" => tools::file_edit::execute(&tool_use.input, project_path),
            "bash" => tools::bash::execute(&tool_use.input, project_path),
            "glob" => tools::glob::execute(&tool_use.input, project_path),
            "grep" => tools::grep::execute(&tool_use.input, project_path),
            "list_dir" => tools::list_dir::execute(&tool_use.input, project_path),
            "excel_filter" => execute_excel_tool(&tool_use.input, "filter"),
            "excel_search" => execute_excel_tool(&tool_use.input, "search"),
            "excel_aggregate" => execute_excel_tool(&tool_use.input, "aggregate"),
            _ => Err(format!("Unknown tool: {}", tool_use.name)),
        };

        match result {
            Ok(content) => ToolResult::success(tool_use.id.clone(), content),
            Err(error) => ToolResult::error(tool_use.id.clone(), error),
        }
    }
}
