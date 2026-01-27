import { Component, For, createSignal, Show } from "solid-js";
import {
  MCPTool,
  MCPAppInstance,
  executeMCPTool,
  createMCPAppInstance,
} from "../../lib/mcp-api";
import MCPAppRenderer from "../MCPAppRenderer";
import { DEMO_APPS } from "./demoApps";

interface MCPAppsTabProps {
  appTools: MCPTool[];
}

const MCPAppsTab: Component<MCPAppsTabProps> = (props) => {
  const [activeApp, setActiveApp] = createSignal<MCPAppInstance | null>(null);
  const [appLoading, setAppLoading] = createSignal(false);
  const [testInput, setTestInput] = createSignal<string>("{}");
  const [demoMode, setDemoMode] = createSignal(false);

  const handleTestAppTool = async (tool: MCPTool) => {
    try {
      setAppLoading(true);
      let params = {};
      try {
        params = JSON.parse(testInput());
      } catch {
        // Use empty object if JSON is invalid
      }

      const result = await executeMCPTool({
        server_id: tool.server_id,
        tool_name: tool.name,
        parameters: params,
      });

      if (result.success && result.ui_resource_uri) {
        const instance = await createMCPAppInstance(
          tool.server_id,
          tool.name,
          result.result
        );
        setActiveApp(instance);
        setDemoMode(false);
      } else if (!result.success) {
        alert(`Tool error: ${result.error || "Unknown error"}`);
      } else {
        alert("Tool executed but no UI available");
      }
    } catch (err) {
      console.error("Failed to test app tool:", err);
      alert(`Failed to test tool: ${err}`);
    } finally {
      setAppLoading(false);
    }
  };

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

  const handleDemoToolCall = async (
    _serverId: string,
    _toolName: string,
    _args: Record<string, unknown>
  ) => {
    return { success: true, message: "Demo tool call executed!" };
  };

  const loadDemoApp = (app: MCPAppInstance) => {
    setActiveApp(app);
    setDemoMode(true);
  };

  const closeApp = () => {
    setActiveApp(null);
    setDemoMode(false);
  };

  return (
    <div class="apps-section">
      <Show when={activeApp()}>
        <MCPAppRenderer
          instance={activeApp()!}
          onToolCall={demoMode() ? handleDemoToolCall : handleAppToolCall}
          onClose={closeApp}
          initialHeight={400}
        />
      </Show>

      <div class="apps-list">
        <h3>Available MCP Apps</h3>
        <p class="apps-description">
          These tools have interactive UI support. Click "Test" to run the tool and see its UI.
        </p>

        <Show
          when={props.appTools.length > 0}
          fallback={
            <div class="empty-state">
              <p>No MCP Apps available from connected servers.</p>
              <p>Connect an MCP server with app-enabled tools, or try the demo apps below.</p>

              <div class="demo-section">
                <h4>Demo Apps</h4>
                <p class="demo-description">Try these example apps to see how MCP Apps work:</p>
                <div class="demo-apps-grid">
                  <div class="demo-app-card" onClick={() => loadDemoApp(DEMO_APPS.chartViewer)}>
                    <span class="demo-icon">📊</span>
                    <div>
                      <h5>Chart Viewer</h5>
                      <p>Interactive data visualization</p>
                    </div>
                  </div>
                  <div class="demo-app-card" onClick={() => loadDemoApp(DEMO_APPS.dataForm)}>
                    <span class="demo-icon">📝</span>
                    <div>
                      <h5>Data Form</h5>
                      <p>Quick entry form example</p>
                    </div>
                  </div>
                </div>
              </div>
            </div>
          }
        >
          <div class="test-input-section">
            <label>Tool Parameters (JSON)</label>
            <textarea
              value={testInput()}
              onInput={(e) => setTestInput(e.currentTarget.value)}
              placeholder='{"key": "value"}'
              rows={3}
            />
          </div>

          <div class="apps-grid">
            <For each={props.appTools}>
              {(tool) => (
                <div class="app-card">
                  <div class="app-header">
                    <span class="app-icon">UI</span>
                    <div class="app-info">
                      <h4>{tool.name}</h4>
                      <p class="app-server">from {tool.server_id}</p>
                    </div>
                  </div>
                  <p class="app-description">{tool.description}</p>
                  <div class="app-actions">
                    <button
                      class="test-btn"
                      onClick={() => handleTestAppTool(tool)}
                      disabled={appLoading()}
                    >
                      {appLoading() ? "Loading..." : "Test"}
                    </button>
                  </div>
                </div>
              )}
            </For>
          </div>
        </Show>
      </div>
    </div>
  );
};

export default MCPAppsTab;
