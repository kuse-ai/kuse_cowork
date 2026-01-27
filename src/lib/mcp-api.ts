import { invoke } from "@tauri-apps/api/core";

export interface MCPServerConfig {
  id: string;
  name: string;
  server_url: string;
  oauth_client_id?: string;
  oauth_client_secret?: string;
  enabled: boolean;
  created_at: string;
  updated_at: string;
}

export interface MCPToolUI {
  resourceUri: string;
  permissions?: string[];
  csp?: string;
}

export interface MCPToolMeta {
  ui?: MCPToolUI;
}

export interface MCPTool {
  server_id: string;
  name: string;
  description: string;
  input_schema: any;
  _meta?: MCPToolMeta;
}

export interface MCPAppInstance {
  id: string;
  server_id: string;
  tool_name: string;
  html_content: string;
  tool_result: any;
  permissions: string[];
  csp?: string;
}

export interface MCPServerStatus {
  id: string;
  name: string;
  status: "Connected" | "Disconnected" | "Connecting" | "Error";
  tools: MCPTool[];
  last_error?: string;
}

export interface MCPToolCall {
  server_id: string;
  tool_name: string;
  parameters: any;
}

export interface MCPToolResult {
  success: boolean;
  result: any;
  error?: string;
  ui_resource_uri?: string;
}

export async function listMCPServers(): Promise<MCPServerConfig[]> {
  return invoke("list_mcp_servers");
}

export async function saveMCPServer(config: MCPServerConfig): Promise<void> {
  return invoke("save_mcp_server", { config });
}

export async function deleteMCPServer(id: string): Promise<void> {
  return invoke("delete_mcp_server", { id });
}

export async function connectMCPServer(id: string): Promise<void> {
  return invoke("connect_mcp_server", { id });
}

export async function disconnectMCPServer(id: string): Promise<void> {
  return invoke("disconnect_mcp_server", { id });
}

export async function getMCPServerStatuses(): Promise<MCPServerStatus[]> {
  return invoke("get_mcp_server_statuses");
}

export async function executeMCPTool(call: MCPToolCall): Promise<MCPToolResult> {
  return invoke("execute_mcp_tool", { call });
}

// MCP Apps functions

export async function getMCPAppTools(): Promise<MCPTool[]> {
  return invoke("get_mcp_app_tools");
}

export async function createMCPAppInstance(
  serverId: string,
  toolName: string,
  toolResult: any
): Promise<MCPAppInstance> {
  return invoke("create_mcp_app_instance", { serverId, toolName, toolResult });
}

/** Check if a tool has MCP Apps UI support */
export function isAppTool(tool: MCPTool): boolean {
  return !!tool._meta?.ui?.resourceUri;
}