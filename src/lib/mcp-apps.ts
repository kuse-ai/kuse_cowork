/**
 * MCP Apps TypeScript types and utilities
 * Based on @modelcontextprotocol/ext-apps specification
 */

// Types from Rust backend
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
  input_schema: Record<string, unknown>;
  _meta?: MCPToolMeta;
}

export interface MCPResourceContent {
  uri: string;
  mimeType: string;
  text?: string;
  blob?: string;
}

export interface MCPResourceResponse {
  contents: MCPResourceContent[];
}

export interface MCPAppInstance {
  id: string;
  server_id: string;
  tool_name: string;
  html_content: string;
  tool_result: unknown;
  permissions: string[];
  csp?: string;
}

// MCP Apps JSON-RPC message types
export interface MCPAppMessage {
  jsonrpc: "2.0";
  id?: string | number;
  method?: string;
  params?: Record<string, unknown>;
  result?: unknown;
  error?: {
    code: number;
    message: string;
    data?: unknown;
  };
}

// App-to-Host request types
export type AppToHostMethod =
  | "ui/initialize"
  | "ui/ready"
  | "tools/call"
  | "ui/openUrl"
  | "ui/log"
  | "ui/contextUpdate";

// Host-to-App notification types
export type HostToAppMethod =
  | "ui/toolResult"
  | "ui/toolInput"
  | "ui/resize";

/**
 * Message handler for MCP App communication
 */
export class MCPAppBridge {
  private iframe: HTMLIFrameElement;
  private origin: string;
  private pendingRequests = new Map<
    string | number,
    {
      resolve: (value: unknown) => void;
      reject: (error: Error) => void;
    }
  >();
  private onToolCallRequest?: (
    toolName: string,
    args: Record<string, unknown>
  ) => Promise<unknown>;

  constructor(
    iframe: HTMLIFrameElement,
    _serverId: string,  // Reserved for future use (e.g., multi-server routing)
    options?: {
      onToolCallRequest?: (
        toolName: string,
        args: Record<string, unknown>
      ) => Promise<unknown>;
    }
  ) {
    this.iframe = iframe;
    this.origin = "*"; // We use srcdoc so origin is null
    this.onToolCallRequest = options?.onToolCallRequest;

    window.addEventListener("message", this.handleMessage);
  }

  destroy() {
    window.removeEventListener("message", this.handleMessage);
    this.pendingRequests.clear();
  }

  private handleMessage = async (event: MessageEvent) => {
    // Verify the message is from our iframe
    if (event.source !== this.iframe.contentWindow) {
      return;
    }

    const message = event.data as MCPAppMessage;

    if (!message.jsonrpc || message.jsonrpc !== "2.0") {
      return;
    }

    // Handle response to our request
    if (message.id !== undefined && !message.method) {
      const pending = this.pendingRequests.get(message.id);
      if (pending) {
        this.pendingRequests.delete(message.id);
        if (message.error) {
          pending.reject(new Error(message.error.message));
        } else {
          pending.resolve(message.result);
        }
      }
      return;
    }

    // Handle request from app
    if (message.method) {
      await this.handleAppRequest(message);
    }
  };

  private async handleAppRequest(message: MCPAppMessage) {
    const { id, method, params } = message;

    try {
      let result: unknown;

      switch (method) {
        case "ui/initialize":
          result = {
            protocolVersion: "2024-11-05",
            capabilities: {
              tools: true,
              contextUpdate: true,
            },
            hostInfo: {
              name: "kuse-cowork",
              version: "0.1.0",
            },
          };
          break;

        case "ui/ready":
          // App is ready, nothing to return
          result = {};
          break;

        case "tools/call":
          if (this.onToolCallRequest && params) {
            const { name, arguments: args } = params as {
              name: string;
              arguments: Record<string, unknown>;
            };
            result = await this.onToolCallRequest(name, args || {});
          } else {
            throw new Error("Tool call handler not configured");
          }
          break;

        case "ui/openUrl":
          // Open URL in external browser
          if (params?.url) {
            // Use Tauri shell.open
            const { open } = await import("@tauri-apps/plugin-shell");
            await open(params.url as string);
            result = { success: true };
          }
          break;

        case "ui/log":
          // Log message from app
          console.log("[MCP App]", params?.message, params?.data);
          result = {};
          break;

        case "ui/contextUpdate":
          // App is updating its context
          // We could store this or emit an event
          console.log("[MCP App Context Update]", params);
          result = {};
          break;

        default:
          throw new Error(`Unknown method: ${method}`);
      }

      // Send response if request had an id
      if (id !== undefined) {
        this.sendResponse(id, result);
      }
    } catch (error) {
      if (id !== undefined) {
        this.sendError(
          id,
          -32000,
          error instanceof Error ? error.message : "Unknown error"
        );
      }
    }
  }

  /**
   * Send a notification to the app (no response expected)
   */
  sendNotification(method: HostToAppMethod, params?: Record<string, unknown>) {
    const message: MCPAppMessage = {
      jsonrpc: "2.0",
      method,
      params,
    };
    this.iframe.contentWindow?.postMessage(message, this.origin);
  }

  /**
   * Send the tool result to the app
   */
  sendToolResult(result: unknown) {
    this.sendNotification("ui/toolResult", {
      content: result,
    });
  }

  /**
   * Send tool input to the app (streaming)
   */
  sendToolInput(input: Record<string, unknown>) {
    this.sendNotification("ui/toolInput", {
      arguments: input,
    });
  }

  private sendResponse(id: string | number, result: unknown) {
    const message: MCPAppMessage = {
      jsonrpc: "2.0",
      id,
      result,
    };
    this.iframe.contentWindow?.postMessage(message, this.origin);
  }

  private sendError(id: string | number, code: number, message: string) {
    const msg: MCPAppMessage = {
      jsonrpc: "2.0",
      id,
      error: { code, message },
    };
    this.iframe.contentWindow?.postMessage(msg, this.origin);
  }
}

/**
 * Build sandbox attribute for iframe
 */
export function buildSandboxAttribute(permissions: string[]): string {
  // Base sandbox permissions for MCP Apps
  const basePermissions = [
    "allow-scripts",
    "allow-forms",
  ];

  // Map MCP permissions to sandbox permissions
  const permissionMap: Record<string, string> = {
    camera: "allow-camera",
    microphone: "allow-microphone",
    geolocation: "allow-geolocation",
  };

  const sandboxPermissions = [...basePermissions];

  for (const perm of permissions) {
    const mapped = permissionMap[perm];
    if (mapped) {
      sandboxPermissions.push(mapped);
    }
  }

  return sandboxPermissions.join(" ");
}

/**
 * Build CSP meta tag for the iframe content
 */
export function buildCSPMetaTag(csp?: string): string {
  // Default restrictive CSP
  const defaultCSP = [
    "default-src 'self'",
    "script-src 'self' 'unsafe-inline'",
    "style-src 'self' 'unsafe-inline'",
    "img-src 'self' data: blob:",
    "font-src 'self' data:",
    "connect-src 'self'",
    "frame-ancestors 'none'",
  ].join("; ");

  const finalCSP = csp || defaultCSP;
  return `<meta http-equiv="Content-Security-Policy" content="${finalCSP}">`;
}

/**
 * Inject the CSP and host connection script into the HTML content
 */
export function prepareAppHTML(
  htmlContent: string,
  csp?: string
): string {
  // Inject CSP meta tag
  const cspTag = buildCSPMetaTag(csp);

  // Check if there's a <head> tag
  if (htmlContent.includes("<head>")) {
    return htmlContent.replace("<head>", `<head>\n    ${cspTag}`);
  }

  // Check if there's an <html> tag
  if (htmlContent.includes("<html")) {
    return htmlContent.replace(
      /<html[^>]*>/,
      `$&\n<head>\n    ${cspTag}\n</head>`
    );
  }

  // Prepend to the content
  return `<!DOCTYPE html>
<html>
<head>
    ${cspTag}
</head>
<body>
${htmlContent}
</body>
</html>`;
}
