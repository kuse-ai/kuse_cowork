import { Component, createSignal, onMount, Show } from "solid-js";
import {
  MCPServerConfig,
  MCPServerStatus,
  MCPTool,
  listMCPServers,
  getMCPServerStatuses,
  getMCPAppTools,
} from "../lib/mcp-api";
import { MCPServersTab, MCPAppsTab } from "./MCP";
import "./MCPSettings.css";

interface MCPSettingsProps {
  onClose: () => void;
}

type TabType = "servers" | "apps";

const MCPSettings: Component<MCPSettingsProps> = (props) => {
  const [activeTab, setActiveTab] = createSignal<TabType>("servers");
  const [servers, setServers] = createSignal<MCPServerConfig[]>([]);
  const [statuses, setStatuses] = createSignal<MCPServerStatus[]>([]);
  const [appTools, setAppTools] = createSignal<MCPTool[]>([]);
  const [loading, setLoading] = createSignal(false);

  onMount(async () => {
    await refreshData();
  });

  const refreshData = async () => {
    try {
      setLoading(true);
      const [serverList, statusList, appToolsList] = await Promise.all([
        listMCPServers(),
        getMCPServerStatuses(),
        getMCPAppTools().catch(() => [] as MCPTool[]),
      ]);
      setServers(serverList);
      setStatuses(statusList);
      setAppTools(appToolsList);
    } catch (err) {
      console.error("Failed to load MCP data:", err);
    } finally {
      setLoading(false);
    }
  };

  return (
    <div class="mcp-settings">
      <div class="mcp-settings-header">
        <h2>MCP Settings</h2>
        <div class="header-actions">
          <button class="refresh-btn" onClick={refreshData} disabled={loading()}>
            {loading() ? "Loading..." : "Refresh"}
          </button>
          <button class="close-btn" onClick={props.onClose}>
            Close
          </button>
        </div>
      </div>

      <div class="mcp-tabs">
        <button
          class={`mcp-tab ${activeTab() === "servers" ? "active" : ""}`}
          onClick={() => setActiveTab("servers")}
        >
          Servers
        </button>
        <button
          class={`mcp-tab ${activeTab() === "apps" ? "active" : ""}`}
          onClick={() => setActiveTab("apps")}
        >
          Apps {appTools().length > 0 && `(${appTools().length})`}
        </button>
      </div>

      <div class="mcp-settings-content">
        <Show when={activeTab() === "servers"}>
          <MCPServersTab
            servers={servers()}
            statuses={statuses()}
            onRefresh={refreshData}
          />
        </Show>

        <Show when={activeTab() === "apps"}>
          <MCPAppsTab appTools={appTools()} />
        </Show>
      </div>
    </div>
  );
};

export default MCPSettings;
