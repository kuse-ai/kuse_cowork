import { Component, Show, For, createSignal } from "solid-js";
import { Task, TaskMessage, openMultipleFoldersDialog } from "../lib/tauri-api";
import { MCPAppInstance, createMCPAppInstance, executeMCPTool } from "../lib/mcp-api";
import { useSettings } from "../stores/settings";
import { hasExcelContext, generateExcelContext } from "../stores/dataPanels";
import MCPAppRenderer from "./MCPAppRenderer";
import "./AgentMain.css";

interface AgentMainProps {
  onNewTask: (title: string, description: string, projectPath?: string) => void;
  onContinueTask: (message: string, projectPath?: string) => void;
  onNewConversation: () => void;
  currentText: string;
  isRunning: boolean;
  activeTask: Task | null;
  messages: TaskMessage[];
  /** Active MCP App instances to display inline */
  activeApps?: MCPAppInstance[];
  /** Callback when an MCP App is closed */
  onCloseApp?: (appId: string) => void;
  /** Callback when a tool result has UI available */
  onToolWithUI?: (serverId: string, toolName: string, result: unknown) => void;
}

const AgentMain: Component<AgentMainProps> = (props) => {
  const { isConfigured, toggleSettings } = useSettings();
  const [input, setInput] = createSignal("");
  const [selectedPaths, setSelectedPaths] = createSignal<string[]>([]);
  const [showPathsPanel, setShowPathsPanel] = createSignal(false);

  // Check if we're in an existing conversation
  const isInConversation = () => props.activeTask !== null && props.messages.length > 0;

  // Handle MCP App tool calls from within the app iframe
  const handleAppToolCall = async (
    serverId: string,
    toolName: string,
    args: Record<string, unknown>
  ) => {
    const result = await executeMCPTool({
      server_id: serverId,
      tool_name: toolName,
      parameters: args,
    });
    if (!result.success) {
      throw new Error(result.error || "Tool call failed");
    }
    return result.result;
  };

  const handleAddFolders = async () => {
    const folders = await openMultipleFoldersDialog();
    if (folders.length > 0) {
      // Add new folders (avoid duplicates)
      const existing = selectedPaths();
      const newPaths = folders.filter(f => !existing.includes(f));
      setSelectedPaths([...existing, ...newPaths]);
      setShowPathsPanel(true);
    }
  };

  const handleRemovePath = (path: string) => {
    setSelectedPaths(selectedPaths().filter(p => p !== path));
  };

  const handleSubmit = (e: Event) => {
    e.preventDefault();
    const userMessage = input().trim();
    if (!userMessage || props.isRunning) return;

    // Join all selected paths with comma for Docker mounting
    const projectPath = selectedPaths().length > 0 ? selectedPaths().join(",") : undefined;

    // Inject Excel context if available
    let messageToSend = userMessage;
    if (hasExcelContext()) {
      const excelContext = generateExcelContext();
      if (excelContext) {
        messageToSend = `${excelContext}\n---\n\n${userMessage}`;
      }
    }

    if (isInConversation()) {
      // Continue existing conversation
      props.onContinueTask(messageToSend, projectPath);
    } else {
      // Create new task
      const firstLine = userMessage.split("\n")[0];
      const title = firstLine.length > 50 ? firstLine.slice(0, 50) + "..." : firstLine;
      props.onNewTask(title, messageToSend, projectPath);
    }
    setInput("");
  };

  return (
    <div class="agent-main">
      <Show
        when={isConfigured()}
        fallback={
          <div class="agent-setup">
            <h2>Welcome to Kuse Cowork</h2>
            <p>Configure your API key to start using the agent</p>
            <button onClick={toggleSettings}>Open Settings</button>
          </div>
        }
      >
        <div class="agent-content">
          {/* Output area */}
          <div class="agent-output">
            <Show
              when={props.activeTask || props.currentText || props.messages.length > 0}
              fallback={
                <div class="empty-state">
                  <h2>Agent Mode</h2>
                  <p>Describe a task and the agent will create a plan and execute it step by step.</p>
                  <div class="capabilities">
                    <div class="capability">
                      <span class="capability-icon">📁</span>
                      <span>Read, write, and edit files</span>
                    </div>
                    <div class="capability">
                      <span class="capability-icon">🔍</span>
                      <span>Search and explore codebases</span>
                    </div>
                    <div class="capability">
                      <span class="capability-icon">⚡</span>
                      <span>Run commands and scripts</span>
                    </div>
                    <div class="capability">
                      <span class="capability-icon">🐳</span>
                      <span>Execute in Docker containers</span>
                    </div>
                  </div>
                </div>
              }
            >
              {/* Show saved message history */}
              <For each={props.messages}>
                {(message) => (
                  <div class={`message ${message.role}`}>
                    <div class="message-label">
                      {message.role === "user" ? "You" : "Agent"}
                    </div>
                    <div class="message-content">{message.content}</div>
                  </div>
                )}
              </For>

              {/* Show current streaming text (when running a new task) */}
              <Show when={props.currentText && props.isRunning}>
                <div class="message assistant streaming">
                  <div class="message-label">Agent</div>
                  <div class="message-content">{props.currentText}</div>
                </div>
              </Show>

              {/* Show active MCP Apps inline */}
              <Show when={props.activeApps && props.activeApps.length > 0}>
                <div class="mcp-apps-inline">
                  <For each={props.activeApps}>
                    {(app) => (
                      <MCPAppRenderer
                        instance={app}
                        onToolCall={handleAppToolCall}
                        onClose={() => props.onCloseApp?.(app.id)}
                        initialHeight={350}
                      />
                    )}
                  </For>
                </div>
              </Show>
            </Show>
          </div>

          {/* Input area */}
          <div class="agent-input-area">
            {/* Selected paths panel */}
            <Show when={showPathsPanel() && selectedPaths().length > 0}>
              <div class="selected-paths">
                <div class="paths-header">
                  <span class="paths-label">Mounted Folders ({selectedPaths().length})</span>
                  <button
                    type="button"
                    class="paths-close"
                    onClick={() => setShowPathsPanel(false)}
                    title="Hide paths"
                  >
                    ×
                  </button>
                </div>
                <div class="paths-list">
                  <For each={selectedPaths()}>
                    {(path) => (
                      <div class="path-item">
                        <span class="path-icon">📁</span>
                        <span class="path-text" title={path}>
                          {path.split("/").pop() || path}
                        </span>
                        <button
                          type="button"
                          class="path-remove"
                          onClick={() => handleRemovePath(path)}
                          disabled={props.isRunning}
                          title={`Remove ${path}`}
                        >
                          ×
                        </button>
                      </div>
                    )}
                  </For>
                </div>
              </div>
            </Show>

            <form class="agent-form" onSubmit={handleSubmit}>
              <div class="input-row">
                <textarea
                  value={input()}
                  onInput={(e) => setInput(e.currentTarget.value)}
                  onKeyDown={(e) => {
                    if (e.key === "Enter" && !e.shiftKey) {
                      e.preventDefault();
                      handleSubmit(e);
                    }
                  }}
                  placeholder={isInConversation()
                    ? "Continue the conversation..."
                    : "Describe a task... (e.g., 'Find and fix the authentication bug in auth.ts')"
                  }
                  disabled={props.isRunning}
                  rows={3}
                />
                <div class="input-actions">
                  <button
                    type="button"
                    class={`path-toggle ${selectedPaths().length > 0 ? "active" : ""}`}
                    onClick={handleAddFolders}
                    disabled={props.isRunning}
                    title="Add folders to mount"
                  >
                    📁
                    <Show when={selectedPaths().length > 0}>
                      <span class="path-count">{selectedPaths().length}</span>
                    </Show>
                  </button>
                  <Show when={isInConversation()}>
                    <button
                      type="button"
                      class="new-chat-btn ghost"
                      onClick={props.onNewConversation}
                      disabled={props.isRunning}
                      title="Start new conversation"
                    >
                      +
                    </button>
                  </Show>
                  <button type="submit" class="submit-btn" disabled={props.isRunning || !input().trim()}>
                    {props.isRunning ? "Running..." : isInConversation() ? "Send" : "Start Task"}
                  </button>
                </div>
              </div>
            </form>
          </div>
        </div>
      </Show>
    </div>
  );
};

export default AgentMain;
