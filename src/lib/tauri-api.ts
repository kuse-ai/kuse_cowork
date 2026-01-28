import { invoke } from "@tauri-apps/api/core";
import { listen, UnlistenFn } from "@tauri-apps/api/event";
import { open } from "@tauri-apps/plugin-dialog";

// Types matching Rust structs
export interface Settings {
  api_key: string;  // Legacy field, kept for compatibility
  model: string;
  base_url: string;
  max_tokens: number;
  temperature: number;
  provider_keys: Record<string, string>;  // Provider-specific API keys
  openai_organization?: string;  // Optional OpenAI Organization ID
  openai_project?: string;  // Optional OpenAI Project ID
}

export interface Conversation {
  id: string;
  title: string;
  created_at: number;
  updated_at: number;
}

export interface Message {
  id: string;
  conversation_id: string;
  role: "user" | "assistant";
  content: string;
  timestamp: number;
}

interface StreamPayload {
  text: string;
  done: boolean;
}

// Agent types
export interface AgentRequest {
  message: string;
  project_path?: string;
  system_prompt?: string;
  max_turns?: number;
}

export type AgentEvent =
  | { type: "text"; content: string }
  | { type: "plan"; steps: PlanStepInfo[] }
  | { type: "step_start"; step: number }
  | { type: "step_done"; step: number }
  | { type: "tool_start"; tool: string; input: Record<string, unknown> }
  | { type: "tool_end"; tool: string; result: string; success: boolean }
  | { type: "turn_complete"; turn: number }
  | { type: "done"; total_turns: number }
  | { type: "error"; message: string };

export interface PlanStepInfo {
  step: number;
  description: string;
}

// Task types
export interface Task {
  id: string;
  title: string;
  description: string;
  status: "planning" | "running" | "completed" | "failed";
  plan: PlanStep[] | null;
  current_step: number;
  project_path: string | null;
  created_at: number;
  updated_at: number;
}

export interface PlanStep {
  step: number;
  description: string;
  status: "pending" | "running" | "completed" | "failed";
}

export interface TaskAgentRequest {
  task_id: string;
  message: string;
  project_path?: string;
  max_turns?: number;
}

export interface TaskMessage {
  id: string;
  task_id: string;
  role: "user" | "assistant";
  content: string;
  timestamp: number;
}

export interface SkillMetadata {
  name: string;
  description: string;
}

// Trace types
export interface Trace {
  id: string;
  task_id: string | null;
  doc_id: string;
  event_type: string; // "edit", "search", "browse", "approval", "tool_start", "tool_end"
  section_path: string | null;
  delta: number | null;
  payload: Record<string, unknown>;
  created_at: number;
}

export interface TraceInput {
  task_id?: string;
  doc_id: string;
  event_type: string;
  section_path?: string;
  delta?: number;
  payload?: Record<string, unknown>;
}

export interface TraceSettings {
  doc_id: string | null;
  tracing_enabled: boolean;
  include_snippets: boolean;
}

export interface Suggestion {
  id: string;
  suggestion_type: string;
  title: string;
  description: string;
  payload: Record<string, unknown>;
  status: string; // "pending", "approved", "rejected"
  created_at: number;
}

// Document types
export interface Document {
  id: string;
  title: string;
  content: string; // HTML content from TipTap editor
  created_at: number;
  updated_at: number;
}

export interface CreateDocumentInput {
  title: string;
  content?: string;
}

export interface UpdateDocumentInput {
  title?: string;
  content?: string;
}

// Enhanced chat with tools
export interface EnhancedChatRequest {
  conversation_id: string;
  content: string;
  project_path?: string;
  enable_tools: boolean;
}

export type ChatEvent =
  | { type: "text"; content: string }
  | { type: "tool_start"; tool: string; input: Record<string, unknown> }
  | { type: "tool_end"; tool: string; result: string; success: boolean }
  | { type: "done"; final_text: string };

// Check if running in Tauri (Tauri 2.x uses __TAURI_INTERNALS__)
export function isTauri(): boolean {
  return typeof window !== "undefined" &&
    ("__TAURI__" in window || "__TAURI_INTERNALS__" in window);
}

// Settings API
export async function getSettings(): Promise<Settings> {
  if (!isTauri()) {
    // Fallback for web dev
    const stored = localStorage.getItem("kuse-cowork-settings");
    if (stored) {
      const parsed = JSON.parse(stored);
      return {
        api_key: parsed.apiKey || "",
        model: parsed.model || "claude-sonnet-4-5-20250929",
        base_url: parsed.baseUrl || "https://api.anthropic.com",
        max_tokens: parsed.maxTokens || 4096,
        temperature: parsed.temperature ?? 0.7,
        provider_keys: parsed.providerKeys || {},
      };
    }
    return {
      api_key: "",
      model: "claude-sonnet-4-5-20250929",
      base_url: "https://api.anthropic.com",
      max_tokens: 4096,
      temperature: 0.7,
      provider_keys: {},
    };
  }
  return invoke<Settings>("get_settings");
}

export async function saveSettings(settings: Settings): Promise<void> {
  if (!isTauri()) {
    localStorage.setItem(
      "kuse-cowork-settings",
      JSON.stringify({
        apiKey: settings.api_key,
        model: settings.model,
        baseUrl: settings.base_url,
        maxTokens: settings.max_tokens,
        temperature: settings.temperature,
        providerKeys: settings.provider_keys,
      })
    );
    return;
  }
  return invoke("save_settings", { settings });
}

export async function testConnection(): Promise<string> {
  console.log("testConnection called, isTauri:", isTauri());
  if (!isTauri()) {
    // Web fallback - use unified AI client
    const settings = await getSettings();
    if (!settings.api_key) return "No API key configured";

    const { testConnection: testAIConnection } = await import("./ai-client");
    const convertedSettings = {
      apiKey: settings.api_key,
      model: settings.model,
      baseUrl: settings.base_url,
      maxTokens: settings.max_tokens,
      temperature: settings.temperature,
      providerKeys: settings.provider_keys || {},
    };

    return testAIConnection(convertedSettings);
  }
  console.log("Calling Tauri invoke test_connection...");
  const result = await invoke<string>("test_connection");
  console.log("Tauri invoke result:", result);
  return result;
}

// Conversations API
export async function listConversations(): Promise<Conversation[]> {
  if (!isTauri()) {
    const stored = localStorage.getItem("kuse-cowork-conversations");
    return stored ? JSON.parse(stored) : [];
  }
  return invoke<Conversation[]>("list_conversations");
}

export async function createConversation(title: string): Promise<Conversation> {
  if (!isTauri()) {
    const conv: Conversation = {
      id: crypto.randomUUID(),
      title,
      created_at: Date.now(),
      updated_at: Date.now(),
    };
    const conversations = await listConversations();
    conversations.unshift(conv);
    localStorage.setItem("kuse-cowork-conversations", JSON.stringify(conversations));
    return conv;
  }
  return invoke<Conversation>("create_conversation", { title });
}

export async function updateConversationTitle(
  id: string,
  title: string
): Promise<void> {
  if (!isTauri()) {
    const conversations = await listConversations();
    const idx = conversations.findIndex((c) => c.id === id);
    if (idx >= 0) {
      conversations[idx].title = title;
      conversations[idx].updated_at = Date.now();
      localStorage.setItem("kuse-cowork-conversations", JSON.stringify(conversations));
    }
    return;
  }
  return invoke("update_conversation_title", { id, title });
}

export async function deleteConversation(id: string): Promise<void> {
  if (!isTauri()) {
    const conversations = await listConversations();
    const filtered = conversations.filter((c) => c.id !== id);
    localStorage.setItem("kuse-cowork-conversations", JSON.stringify(filtered));
    localStorage.removeItem(`kuse-cowork-messages-${id}`);
    return;
  }
  return invoke("delete_conversation", { id });
}

// Messages API
export async function getMessages(conversationId: string): Promise<Message[]> {
  if (!isTauri()) {
    const stored = localStorage.getItem(`kuse-cowork-messages-${conversationId}`);
    return stored ? JSON.parse(stored) : [];
  }
  return invoke<Message[]>("get_messages", { conversationId });
}

function saveMessagesLocal(conversationId: string, messages: Message[]) {
  localStorage.setItem(
    `kuse-cowork-messages-${conversationId}`,
    JSON.stringify(messages)
  );
}

// Chat API with streaming
export async function sendChatMessage(
  conversationId: string,
  content: string,
  onStream: (text: string) => void
): Promise<string> {
  if (!isTauri()) {
    // Web fallback - direct API call
    const settings = await getSettings();
    const messages = await getMessages(conversationId);

    // Add user message
    const userMsg: Message = {
      id: crypto.randomUUID(),
      conversation_id: conversationId,
      role: "user",
      content,
      timestamp: Date.now(),
    };
    messages.push(userMsg);
    saveMessagesLocal(conversationId, messages);

    // Use unified AI client
    const { sendMessage: sendAIMessage } = await import("./ai-client");
    const convertedSettings = {
      apiKey: settings.api_key,
      model: settings.model,
      baseUrl: settings.base_url,
      maxTokens: settings.max_tokens,
      temperature: settings.temperature,
      providerKeys: settings.provider_keys || {},
    };

    const fullText = await sendAIMessage(messages, convertedSettings, onStream);

    // Save assistant message
    const assistantMsg: Message = {
      id: crypto.randomUUID(),
      conversation_id: conversationId,
      role: "assistant",
      content: fullText,
      timestamp: Date.now(),
    };
    messages.push(assistantMsg);
    saveMessagesLocal(conversationId, messages);

    // Update title if first message
    if (messages.length === 2) {
      const title = content.length > 30 ? content.slice(0, 30) + "..." : content;
      await updateConversationTitle(conversationId, title);
    }

    return fullText;
  }

  // Tauri mode - use Rust backend
  let unlisten: UnlistenFn | undefined;

  try {
    // Listen for stream events
    unlisten = await listen<StreamPayload>("chat-stream", (event) => {
      onStream(event.payload.text);
    });

    // Send message via Rust
    const response = await invoke<string>("send_chat_message", {
      conversationId,
      content,
    });

    return response;
  } finally {
    if (unlisten) {
      unlisten();
    }
  }
}

// Agent API
export async function runAgent(
  request: AgentRequest,
  onEvent: (event: AgentEvent) => void
): Promise<string> {
  if (!isTauri()) {
    // Web fallback - agent requires Tauri backend
    throw new Error("Agent mode requires the desktop app");
  }

  let unlisten: UnlistenFn | undefined;

  try {
    // Listen for agent events
    unlisten = await listen<AgentEvent>("agent-event", (event) => {
      onEvent(event.payload);
    });

    // Run agent via Rust
    const response = await invoke<string>("run_agent", { request });
    return response;
  } finally {
    if (unlisten) {
      unlisten();
    }
  }
}

// Enhanced Chat API with tool support
export async function sendChatWithTools(
  request: EnhancedChatRequest,
  onEvent: (event: ChatEvent) => void
): Promise<string> {
  if (!isTauri()) {
    // Web fallback - tools require Tauri backend
    throw new Error("Tool-enabled chat requires the desktop app");
  }

  let unlisten: UnlistenFn | undefined;

  try {
    // Listen for chat events
    unlisten = await listen<ChatEvent>("chat-event", (event) => {
      onEvent(event.payload);
    });

    // Send chat with tools via Rust
    const response = await invoke<string>("send_chat_with_tools", { request });
    return response;
  } finally {
    if (unlisten) {
      unlisten();
    }
  }
}

// Task API
export async function listTasks(): Promise<Task[]> {
  if (!isTauri()) {
    const stored = localStorage.getItem("kuse-cowork-tasks");
    return stored ? JSON.parse(stored) : [];
  }
  return invoke<Task[]>("list_tasks");
}

export async function getTask(id: string): Promise<Task | null> {
  if (!isTauri()) {
    const tasks = await listTasks();
    return tasks.find((t) => t.id === id) || null;
  }
  return invoke<Task | null>("get_task", { id });
}

export async function createTask(
  title: string,
  description: string,
  projectPath?: string
): Promise<Task> {
  if (!isTauri()) {
    const task: Task = {
      id: crypto.randomUUID(),
      title,
      description,
      status: "planning",
      plan: null,
      current_step: 0,
      project_path: projectPath || null,
      created_at: Date.now(),
      updated_at: Date.now(),
    };
    const tasks = await listTasks();
    tasks.unshift(task);
    localStorage.setItem("kuse-cowork-tasks", JSON.stringify(tasks));
    return task;
  }
  return invoke<Task>("create_task", { title, description, projectPath });
}

export async function deleteTask(id: string): Promise<void> {
  if (!isTauri()) {
    const tasks = await listTasks();
    const filtered = tasks.filter((t) => t.id !== id);
    localStorage.setItem("kuse-cowork-tasks", JSON.stringify(filtered));
    return;
  }
  return invoke("delete_task", { id });
}

export async function runTaskAgent(
  request: TaskAgentRequest,
  onEvent: (event: AgentEvent) => void
): Promise<string> {
  if (!isTauri()) {
    throw new Error("Task agent requires the desktop app");
  }

  let unlisten: UnlistenFn | undefined;

  try {
    unlisten = await listen<AgentEvent>("agent-event", (event) => {
      onEvent(event.payload);
    });

    const response = await invoke<string>("run_task_agent", { request });
    return response;
  } finally {
    if (unlisten) {
      unlisten();
    }
  }
}

export async function getTaskMessages(taskId: string): Promise<TaskMessage[]> {
  if (!isTauri()) {
    // Web fallback
    const stored = localStorage.getItem(`kuse-cowork-task-messages-${taskId}`);
    return stored ? JSON.parse(stored) : [];
  }
  return invoke<TaskMessage[]>("get_task_messages", { taskId });
}

// File/Folder picker API
export async function openFolderDialog(): Promise<string | null> {
  if (!isTauri()) {
    // Web fallback - not supported
    return null;
  }
  const selected = await open({
    directory: true,
    multiple: false,
    title: "Select folder to mount",
  });
  return selected as string | null;
}

export async function openMultipleFoldersDialog(): Promise<string[]> {
  if (!isTauri()) {
    // Web fallback - not supported
    return [];
  }
  const selected = await open({
    directory: true,
    multiple: true,
    title: "Select folders to mount",
  });
  if (!selected) return [];
  return Array.isArray(selected) ? selected : [selected];
}

// Skills API
export async function getSkillsList(): Promise<SkillMetadata[]> {
  if (!isTauri()) {
    // Web fallback - return empty list
    return [];
  }
  return invoke<SkillMetadata[]>("get_skills_list");
}

// ==================== MCP Apps API ====================

import type {
  MCPAppInstance,
  MCPResourceResponse,
  MCPTool,
} from "./mcp-apps";

/**
 * Get all MCP tools that have MCP Apps UI support
 */
export async function getMCPAppTools(): Promise<MCPTool[]> {
  if (!isTauri()) {
    return [];
  }
  return invoke<MCPTool[]>("get_mcp_app_tools");
}

/**
 * Fetch a UI resource from an MCP server
 */
export async function fetchMCPAppResource(
  serverId: string,
  resourceUri: string
): Promise<MCPResourceResponse> {
  if (!isTauri()) {
    throw new Error("MCP Apps require the desktop app");
  }
  return invoke<MCPResourceResponse>("fetch_mcp_app_resource", {
    serverId,
    resourceUri,
  });
}

/**
 * Create an MCP App instance for rendering
 */
export async function createMCPAppInstance(
  serverId: string,
  toolName: string,
  toolResult: unknown
): Promise<MCPAppInstance> {
  if (!isTauri()) {
    throw new Error("MCP Apps require the desktop app");
  }
  return invoke<MCPAppInstance>("create_mcp_app_instance", {
    serverId,
    toolName,
    toolResult,
  });
}

/**
 * Execute an MCP tool (reusing existing execute_mcp_tool command)
 */
export async function executeMCPTool(
  serverId: string,
  toolName: string,
  parameters: Record<string, unknown>
): Promise<{ success: boolean; result: unknown; error?: string; ui_resource_uri?: string }> {
  if (!isTauri()) {
    throw new Error("MCP tools require the desktop app");
  }
  return invoke("execute_mcp_tool", {
    call: {
      server_id: serverId,
      tool_name: toolName,
      parameters,
    },
  });
}

// ==================== Trace API ====================

/**
 * Log a new trace event
 */
export async function logTrace(input: TraceInput): Promise<Trace> {
  if (!isTauri()) {
    // Web fallback - store in localStorage
    const trace: Trace = {
      id: crypto.randomUUID(),
      task_id: input.task_id || null,
      doc_id: input.doc_id,
      event_type: input.event_type,
      section_path: input.section_path || null,
      delta: input.delta || null,
      payload: input.payload || {},
      created_at: Date.now(),
    };
    const traces = JSON.parse(localStorage.getItem(`traces-${input.doc_id}`) || "[]");
    traces.unshift(trace);
    localStorage.setItem(`traces-${input.doc_id}`, JSON.stringify(traces.slice(0, 1000)));
    return trace;
  }
  return invoke<Trace>("log_trace", { input });
}

/**
 * List traces for a document
 */
export async function listTraces(
  docId: string,
  limit?: number,
  beforeTimestamp?: number
): Promise<Trace[]> {
  if (!isTauri()) {
    const traces: Trace[] = JSON.parse(localStorage.getItem(`traces-${docId}`) || "[]");
    let filtered = traces;
    if (beforeTimestamp) {
      filtered = traces.filter((t) => t.created_at < beforeTimestamp);
    }
    return filtered.slice(0, limit || 100);
  }
  return invoke<Trace[]>("list_traces", { docId, limit, beforeTimestamp });
}

/**
 * Delete a specific trace
 */
export async function deleteTrace(id: string): Promise<void> {
  if (!isTauri()) {
    // Would need doc_id to properly delete from localStorage
    return;
  }
  return invoke("delete_trace", { id });
}

/**
 * Clear all traces for a document
 */
export async function clearTraces(docId: string): Promise<number> {
  if (!isTauri()) {
    localStorage.removeItem(`traces-${docId}`);
    return 0;
  }
  return invoke<number>("clear_traces", { docId });
}

/**
 * Get trace settings for a document
 */
export async function getTraceSettings(docId: string): Promise<TraceSettings> {
  if (!isTauri()) {
    const stored = localStorage.getItem(`trace-settings-${docId}`);
    if (stored) return JSON.parse(stored);
    return {
      doc_id: docId,
      tracing_enabled: true,
      include_snippets: true,
    };
  }
  return invoke<TraceSettings>("get_trace_settings", { docId });
}

/**
 * Save trace settings for a document
 */
export async function saveTraceSettings(docId: string, settings: TraceSettings): Promise<void> {
  if (!isTauri()) {
    localStorage.setItem(`trace-settings-${docId}`, JSON.stringify(settings));
    return;
  }
  return invoke("save_trace_settings", { docId, settings });
}

// ==================== Suggestion API ====================

/**
 * List suggestions for a document
 */
export async function listSuggestions(docId: string, status?: string): Promise<Suggestion[]> {
  if (!isTauri()) {
    return [];
  }
  return invoke<Suggestion[]>("list_suggestions", { docId, status });
}

/**
 * Update suggestion status (approve/reject)
 */
export async function updateSuggestionStatus(id: string, status: string): Promise<void> {
  if (!isTauri()) {
    return;
  }
  return invoke("update_suggestion_status", { id, status });
}

/**
 * Delete a suggestion
 */
export async function deleteSuggestion(id: string): Promise<void> {
  if (!isTauri()) {
    return;
  }
  return invoke("delete_suggestion", { id });
}

/**
 * Generate AI suggestions based on recent trace history
 */
export async function generateSuggestions(docId: string): Promise<Suggestion[]> {
  if (!isTauri()) {
    return [];
  }
  return invoke<Suggestion[]>("generate_suggestions", { docId });
}

/**
 * Apply an approved suggestion
 */
export async function applySuggestion(suggestionId: string): Promise<{
  applied: boolean;
  suggestion_type: string;
  payload: Record<string, unknown>;
}> {
  if (!isTauri()) {
    throw new Error("Suggestions require the desktop app");
  }
  return invoke("apply_suggestion", { suggestionId });
}

/**
 * Open a URL in a new native webview window (bypasses X-Frame-Options)
 */
export async function openBrowserWindow(url: string, title?: string): Promise<void> {
  if (!isTauri()) {
    window.open(url, "_blank");
    return;
  }
  return invoke("open_browser_window", { url, title });
}

/**
 * Create an embedded browser webview in the main window
 */
export async function createEmbeddedBrowser(
  url: string,
  x: number,
  y: number,
  width: number,
  height: number
): Promise<string> {
  if (!isTauri()) {
    throw new Error("Embedded browser requires the desktop app");
  }
  return invoke<string>("create_embedded_browser", { url, x, y, width, height });
}

/**
 * Update the embedded browser position and size
 */
export async function updateEmbeddedBrowserBounds(
  x: number,
  y: number,
  width: number,
  height: number
): Promise<void> {
  if (!isTauri()) {
    return;
  }
  return invoke("update_embedded_browser_bounds", { x, y, width, height });
}

/**
 * Navigate the embedded browser to a URL
 */
export async function navigateEmbeddedBrowser(url: string): Promise<void> {
  if (!isTauri()) {
    return;
  }
  return invoke("navigate_embedded_browser", { url });
}

/**
 * Close the embedded browser
 */
export async function closeEmbeddedBrowser(): Promise<void> {
  if (!isTauri()) {
    return;
  }
  return invoke("close_embedded_browser");
}

// ==================== Document API ====================

/**
 * Create a new document
 */
export async function createDocument(input: CreateDocumentInput): Promise<Document> {
  if (!isTauri()) {
    // Web fallback - store in localStorage
    const doc: Document = {
      id: crypto.randomUUID(),
      title: input.title,
      content: input.content || "",
      created_at: Date.now(),
      updated_at: Date.now(),
    };
    const docs = JSON.parse(localStorage.getItem("kuse-cowork-documents") || "[]");
    docs.unshift(doc);
    localStorage.setItem("kuse-cowork-documents", JSON.stringify(docs));
    return doc;
  }
  return invoke<Document>("create_document", { input });
}

/**
 * Get a document by ID
 */
export async function getDocument(id: string): Promise<Document | null> {
  if (!isTauri()) {
    const docs: Document[] = JSON.parse(localStorage.getItem("kuse-cowork-documents") || "[]");
    return docs.find((d) => d.id === id) || null;
  }
  return invoke<Document | null>("get_document", { id });
}

/**
 * Update a document
 */
export async function updateDocument(id: string, input: UpdateDocumentInput): Promise<Document | null> {
  if (!isTauri()) {
    const docs: Document[] = JSON.parse(localStorage.getItem("kuse-cowork-documents") || "[]");
    const idx = docs.findIndex((d) => d.id === id);
    if (idx < 0) return null;

    if (input.title !== undefined) docs[idx].title = input.title;
    if (input.content !== undefined) docs[idx].content = input.content;
    docs[idx].updated_at = Date.now();

    localStorage.setItem("kuse-cowork-documents", JSON.stringify(docs));
    return docs[idx];
  }
  return invoke<Document | null>("update_document", { id, input });
}

/**
 * List all documents
 */
export async function listDocuments(): Promise<Document[]> {
  if (!isTauri()) {
    const docs: Document[] = JSON.parse(localStorage.getItem("kuse-cowork-documents") || "[]");
    return docs.sort((a, b) => b.updated_at - a.updated_at);
  }
  return invoke<Document[]>("list_documents");
}

/**
 * Delete a document
 */
export async function deleteDocument(id: string): Promise<boolean> {
  if (!isTauri()) {
    const docs: Document[] = JSON.parse(localStorage.getItem("kuse-cowork-documents") || "[]");
    const filtered = docs.filter((d) => d.id !== id);
    localStorage.setItem("kuse-cowork-documents", JSON.stringify(filtered));
    return docs.length !== filtered.length;
  }
  return invoke<boolean>("delete_document", { id });
}

// ==================== Activity API ====================

// Activity types matching Rust structs
export type ContextType = "document" | "task" | "browser" | "mixed" | "unknown";
export type EventType = "edit" | "browse" | "search" | "tool" | "focus" | "blur" | "save" | "export" | "import";

export interface Session {
  id: string;
  started_at: number;
  ended_at: number | null;
  summary: string | null;
  total_work_blocks: number;
  total_duration_mins: number;
}

export interface WorkBlock {
  id: string;
  session_id: string;
  started_at: number;
  ended_at: number | null;
  context_type: ContextType;
  context_id: string | null;
  context_title: string | null;
  summary: string | null;
  content_snapshot: string | null;
  edit_count: number;
  browse_count: number;
  total_chars_changed: number;
  research_links: ResearchLink[];
}

export interface ResearchLink {
  id: string;
  work_block_id: string;
  url: string;
  title: string | null;
  extracted_content: string | null;
  timestamp: number;
}

export interface ActivityEvent {
  id: string;
  work_block_id: string | null;
  session_id: string;
  timestamp: number;
  event_type: EventType;
  context_type: ContextType | null;
  context_id: string | null;
  payload: Record<string, unknown>;
  content_snippet: string | null;
  archived: boolean;
}

export interface ActivityEventInput {
  event_type: string;
  context_type?: string;
  context_id?: string;
  context_title?: string;
  payload?: Record<string, unknown>;
  content_snippet?: string;
}

export interface WorkBlockInput {
  context_type: string;
  context_id?: string;
  context_title?: string;
  content_snapshot?: string;
  event_ids: string[];
}

export interface TimelineQuery {
  session_id?: string;
  context_type?: string;
  context_id?: string;
  from_timestamp?: number;
  to_timestamp?: number;
  limit?: number;
  include_events?: boolean;
}

export interface Timeline {
  session: Session | null;
  work_blocks: WorkBlock[];
  total_count: number;
}

/**
 * Get or create the current active session
 */
export async function getActiveSession(): Promise<Session> {
  if (!isTauri()) {
    // Web fallback
    const stored = localStorage.getItem("kuse-activity-session");
    if (stored) {
      const session = JSON.parse(stored);
      if (!session.ended_at) return session;
    }
    const newSession: Session = {
      id: crypto.randomUUID(),
      started_at: Date.now(),
      ended_at: null,
      summary: null,
      total_work_blocks: 0,
      total_duration_mins: 0,
    };
    localStorage.setItem("kuse-activity-session", JSON.stringify(newSession));
    return newSession;
  }
  return invoke<Session>("get_active_session");
}

/**
 * End the current session
 */
export async function endSession(sessionId: string, summary?: string): Promise<void> {
  if (!isTauri()) {
    const stored = localStorage.getItem("kuse-activity-session");
    if (stored) {
      const session = JSON.parse(stored);
      session.ended_at = Date.now();
      session.summary = summary || null;
      localStorage.setItem("kuse-activity-session", JSON.stringify(session));
    }
    return;
  }
  return invoke("end_session", { sessionId, summary });
}

/**
 * Log an activity event
 */
export async function logActivityEvent(input: ActivityEventInput): Promise<ActivityEvent> {
  if (!isTauri()) {
    const session = await getActiveSession();
    const event: ActivityEvent = {
      id: crypto.randomUUID(),
      work_block_id: null,
      session_id: session.id,
      timestamp: Date.now(),
      event_type: input.event_type as EventType,
      context_type: (input.context_type as ContextType) || null,
      context_id: input.context_id || null,
      payload: input.payload || {},
      content_snippet: input.content_snippet || null,
      archived: false,
    };
    const events = JSON.parse(localStorage.getItem("kuse-activity-events") || "[]");
    events.push(event);
    localStorage.setItem("kuse-activity-events", JSON.stringify(events.slice(-500)));
    return event;
  }
  return invoke<ActivityEvent>("log_activity_event", { input });
}

/**
 * Get unbatched events (not yet in a work block)
 */
export async function getUnbatchedEvents(): Promise<ActivityEvent[]> {
  if (!isTauri()) {
    const events: ActivityEvent[] = JSON.parse(localStorage.getItem("kuse-activity-events") || "[]");
    return events.filter((e) => !e.work_block_id);
  }
  return invoke<ActivityEvent[]>("get_unbatched_events");
}

/**
 * Create a work block from batched events
 */
export async function createWorkBlock(input: WorkBlockInput): Promise<WorkBlock> {
  if (!isTauri()) {
    const session = await getActiveSession();
    const block: WorkBlock = {
      id: crypto.randomUUID(),
      session_id: session.id,
      started_at: Date.now(),
      ended_at: null,
      context_type: input.context_type as ContextType,
      context_id: input.context_id || null,
      context_title: input.context_title || null,
      summary: null,
      content_snapshot: input.content_snapshot || null,
      edit_count: 0,
      browse_count: 0,
      total_chars_changed: 0,
      research_links: [],
    };
    const blocks = JSON.parse(localStorage.getItem("kuse-activity-blocks") || "[]");
    blocks.unshift(block);
    localStorage.setItem("kuse-activity-blocks", JSON.stringify(blocks.slice(0, 100)));
    // Mark events as batched
    const events: ActivityEvent[] = JSON.parse(localStorage.getItem("kuse-activity-events") || "[]");
    for (const e of events) {
      if (input.event_ids.includes(e.id)) {
        e.work_block_id = block.id;
      }
    }
    localStorage.setItem("kuse-activity-events", JSON.stringify(events));
    return block;
  }
  return invoke<WorkBlock>("create_work_block", { input });
}

/**
 * Finalize a work block with optional summary
 */
export async function finalizeWorkBlock(workBlockId: string, summary?: string): Promise<void> {
  if (!isTauri()) {
    const blocks: WorkBlock[] = JSON.parse(localStorage.getItem("kuse-activity-blocks") || "[]");
    const idx = blocks.findIndex((b) => b.id === workBlockId);
    if (idx >= 0) {
      blocks[idx].ended_at = Date.now();
      blocks[idx].summary = summary || null;
      localStorage.setItem("kuse-activity-blocks", JSON.stringify(blocks));
    }
    return;
  }
  return invoke("finalize_work_block", { workBlockId, summary });
}

/**
 * Get work block by ID
 */
export async function getWorkBlock(workBlockId: string): Promise<WorkBlock | null> {
  if (!isTauri()) {
    const blocks: WorkBlock[] = JSON.parse(localStorage.getItem("kuse-activity-blocks") || "[]");
    return blocks.find((b) => b.id === workBlockId) || null;
  }
  return invoke<WorkBlock | null>("get_work_block", { workBlockId });
}

/**
 * Get events for a work block
 */
export async function getWorkBlockEvents(workBlockId: string): Promise<ActivityEvent[]> {
  if (!isTauri()) {
    const events: ActivityEvent[] = JSON.parse(localStorage.getItem("kuse-activity-events") || "[]");
    return events.filter((e) => e.work_block_id === workBlockId);
  }
  return invoke<ActivityEvent[]>("get_work_block_events", { workBlockId });
}

/**
 * Add a research link to a work block
 */
export async function addResearchLink(
  workBlockId: string,
  url: string,
  title?: string,
  extractedContent?: string
): Promise<ResearchLink> {
  if (!isTauri()) {
    const link: ResearchLink = {
      id: crypto.randomUUID(),
      work_block_id: workBlockId,
      url,
      title: title || null,
      extracted_content: extractedContent || null,
      timestamp: Date.now(),
    };
    // Add to work block
    const blocks: WorkBlock[] = JSON.parse(localStorage.getItem("kuse-activity-blocks") || "[]");
    const idx = blocks.findIndex((b) => b.id === workBlockId);
    if (idx >= 0) {
      blocks[idx].research_links.push(link);
      localStorage.setItem("kuse-activity-blocks", JSON.stringify(blocks));
    }
    return link;
  }
  return invoke<ResearchLink>("add_research_link", { workBlockId, url, title, extractedContent });
}

/**
 * Get activity timeline with filtering
 */
export async function getActivityTimeline(query: TimelineQuery): Promise<Timeline> {
  if (!isTauri()) {
    const blocks: WorkBlock[] = JSON.parse(localStorage.getItem("kuse-activity-blocks") || "[]");
    let filtered = blocks;
    if (query.context_type) {
      filtered = filtered.filter((b) => b.context_type === query.context_type);
    }
    if (query.context_id) {
      filtered = filtered.filter((b) => b.context_id === query.context_id);
    }
    if (query.from_timestamp) {
      filtered = filtered.filter((b) => b.started_at >= query.from_timestamp!);
    }
    if (query.to_timestamp) {
      filtered = filtered.filter((b) => b.started_at <= query.to_timestamp!);
    }
    const limited = filtered.slice(0, query.limit || 50);
    return {
      session: null,
      work_blocks: limited,
      total_count: limited.length,
    };
  }
  // Ensure required fields are present for Rust serde
  const safeQuery = {
    ...query,
    include_events: query.include_events ?? false
  };
  return invoke<Timeline>("get_activity_timeline", { query: safeQuery });
}

/**
 * Generate AI summary for a work block
 */
export async function summarizeWorkBlock(workBlockId: string): Promise<string> {
  if (!isTauri()) {
    throw new Error("AI summarization requires the desktop app");
  }
  return invoke<string>("summarize_work_block", { workBlockId });
}

/**
 * Archive old events
 */
export async function archiveOldEvents(beforeTimestamp: number): Promise<number> {
  if (!isTauri()) {
    return 0;
  }
  return invoke<number>("archive_old_events", { beforeTimestamp });
}

/**
 * Compress old work blocks
 */
export async function compressOldWorkBlocks(beforeTimestamp: number): Promise<number> {
  if (!isTauri()) {
    return 0;
  }
  return invoke<number>("compress_old_work_blocks", { beforeTimestamp });
}
