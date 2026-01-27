import {
  Component,
  createSignal,
  onMount,
  onCleanup,
  Show,
  createEffect,
} from "solid-js";
import {
  MCPAppInstance,
  MCPAppBridge,
  buildSandboxAttribute,
  prepareAppHTML,
  MCPTool,
} from "../lib/mcp-apps";
import "./MCPAppRenderer.css";

interface MCPAppRendererProps {
  /** The MCP App instance to render */
  instance: MCPAppInstance;
  /** Callback when the app requests a tool call */
  onToolCall?: (
    serverId: string,
    toolName: string,
    args: Record<string, unknown>
  ) => Promise<unknown>;
  /** Callback when the app is closed */
  onClose?: () => void;
  /** Optional initial height */
  initialHeight?: number;
}

/**
 * Renders an MCP App in a sandboxed iframe with postMessage communication
 */
export const MCPAppRenderer: Component<MCPAppRendererProps> = (props) => {
  let iframeRef: HTMLIFrameElement | undefined;
  let bridgeRef: MCPAppBridge | undefined;

  const [isLoading, setIsLoading] = createSignal(true);
  const [error, setError] = createSignal<string | null>(null);
  const [height, setHeight] = createSignal(props.initialHeight || 400);

  onMount(() => {
    if (!iframeRef) return;

    // Prepare the HTML content with CSP
    const preparedHTML = prepareAppHTML(
      props.instance.html_content,
      props.instance.csp
    );

    // Set iframe content
    iframeRef.srcdoc = preparedHTML;

    // Create bridge for communication
    bridgeRef = new MCPAppBridge(iframeRef, props.instance.server_id, {
      onToolCallRequest: async (toolName, args) => {
        if (props.onToolCall) {
          return props.onToolCall(
            props.instance.server_id,
            toolName,
            args
          );
        }
        throw new Error("Tool call handler not configured");
      },
    });

    // Handle iframe load
    iframeRef.onload = () => {
      setIsLoading(false);

      // Send the initial tool result to the app
      setTimeout(() => {
        bridgeRef?.sendToolResult(props.instance.tool_result);
      }, 100);
    };

    iframeRef.onerror = () => {
      setError("Failed to load MCP App");
      setIsLoading(false);
    };
  });

  onCleanup(() => {
    bridgeRef?.destroy();
  });

  // Watch for tool result updates
  createEffect(() => {
    if (bridgeRef && !isLoading()) {
      bridgeRef.sendToolResult(props.instance.tool_result);
    }
  });

  const sandboxAttr = buildSandboxAttribute(props.instance.permissions);

  return (
    <div class="mcp-app-renderer">
      <div class="mcp-app-header">
        <span class="mcp-app-title">
          {props.instance.tool_name}
        </span>
        <span class="mcp-app-server">
          from {props.instance.server_id}
        </span>
        <Show when={props.onClose}>
          <button
            class="mcp-app-close"
            onClick={props.onClose}
            title="Close"
          >
            x
          </button>
        </Show>
      </div>

      <div class="mcp-app-content" style={{ height: `${height()}px` }}>
        <Show when={isLoading()}>
          <div class="mcp-app-loading">
            <span>Loading MCP App...</span>
          </div>
        </Show>

        <Show when={error()}>
          <div class="mcp-app-error">
            <span>{error()}</span>
          </div>
        </Show>

        <iframe
          ref={iframeRef}
          class="mcp-app-iframe"
          sandbox={sandboxAttr}
          title={`MCP App: ${props.instance.tool_name}`}
          style={{
            display: isLoading() || error() ? "none" : "block",
          }}
        />
      </div>

      <div class="mcp-app-resize-handle"
        onMouseDown={(e) => {
          e.preventDefault();
          const startY = e.clientY;
          const startHeight = height();

          const onMouseMove = (moveEvent: MouseEvent) => {
            const delta = moveEvent.clientY - startY;
            setHeight(Math.max(200, startHeight + delta));
          };

          const onMouseUp = () => {
            document.removeEventListener("mousemove", onMouseMove);
            document.removeEventListener("mouseup", onMouseUp);
          };

          document.addEventListener("mousemove", onMouseMove);
          document.addEventListener("mouseup", onMouseUp);
        }}
      />
    </div>
  );
};

/**
 * Button to trigger an MCP App tool
 */
interface MCPAppToolButtonProps {
  tool: MCPTool;
  onActivate: (tool: MCPTool) => void;
}

export const MCPAppToolButton: Component<MCPAppToolButtonProps> = (props) => {
  return (
    <button
      class="mcp-app-tool-button"
      onClick={() => props.onActivate(props.tool)}
      title={props.tool.description}
    >
      <span class="tool-icon">UI</span>
      <span class="tool-name">{props.tool.name}</span>
    </button>
  );
};

/**
 * Panel showing available MCP Apps
 */
interface MCPAppsListProps {
  tools: MCPTool[];
  onSelectTool: (tool: MCPTool) => void;
}

export const MCPAppsList: Component<MCPAppsListProps> = (props) => {
  return (
    <div class="mcp-apps-list">
      <div class="mcp-apps-list-header">
        <span>Available MCP Apps</span>
      </div>
      <div class="mcp-apps-list-content">
        <Show
          when={props.tools.length > 0}
          fallback={
            <div class="mcp-apps-empty">
              No MCP Apps available. Connect an MCP server with app-enabled
              tools.
            </div>
          }
        >
          {props.tools.map((tool) => (
            <MCPAppToolButton
              tool={tool}
              onActivate={props.onSelectTool}
            />
          ))}
        </Show>
      </div>
    </div>
  );
};

export default MCPAppRenderer;
