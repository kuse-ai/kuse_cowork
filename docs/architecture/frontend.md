# Frontend Architecture

The Kuse Cowork frontend is built with SolidJS, providing a reactive and performant user interface.

## Technology Stack

| Technology | Version | Purpose |
|------------|---------|---------|
| SolidJS | 1.8.x | Reactive UI framework |
| TypeScript | 5.3.x | Type safety |
| Vite | 5.4.x | Build tool |
| Tauri API | 2.0.x | Backend communication |

## Project Structure

```
src/
├── App.tsx              # Main application component
├── index.tsx            # Entry point
├── index.css            # Global styles
├── components/          # UI components
│   ├── Chat.tsx         # Chat interface
│   ├── Chat.css
│   ├── AgentMain.tsx    # Agent view
│   ├── AgentMain.css
│   ├── Settings.tsx     # Settings panel
│   ├── Settings.css
│   ├── ModelSelector.tsx
│   ├── TaskPanel.tsx
│   ├── TaskSidebar.tsx
│   ├── MCPSettings.tsx
│   └── ...
├── stores/              # State management
│   ├── settings.ts      # Settings store
│   └── chat.ts          # Chat store
└── lib/                 # Utilities
    ├── tauri-api.ts     # Tauri bridge
    ├── ai-client.ts     # AI provider clients
    ├── mcp-api.ts       # MCP client
    └── claude.ts        # Claude utilities
```

## Component Architecture

### App Component

The root component that handles routing and layout:

```tsx
const App: Component = () => {
  const [view, setView] = createSignal<'chat' | 'tasks'>('chat');

  return (
    <div class="app">
      <Sidebar view={view()} onViewChange={setView} />
      <main>
        <Switch>
          <Match when={view() === 'chat'}>
            <Chat />
          </Match>
          <Match when={view() === 'tasks'}>
            <AgentMain />
          </Match>
        </Switch>
      </main>
      <Show when={showSettings()}>
        <Settings />
      </Show>
    </div>
  );
};
```

### Chat Component

Handles conversational interactions:

```tsx
const Chat: Component = () => {
  const [messages, setMessages] = createSignal<Message[]>([]);
  const [input, setInput] = createSignal('');
  const [streaming, setStreaming] = createSignal(false);

  const sendMessage = async () => {
    const content = input();
    setInput('');

    await sendChatMessage(conversationId, content, (text) => {
      // Update streaming display
    });
  };

  return (
    <div class="chat">
      <MessageList messages={messages()} />
      <InputArea
        value={input()}
        onChange={setInput}
        onSubmit={sendMessage}
        disabled={streaming()}
      />
    </div>
  );
};
```

### AgentMain Component

Manages task execution and progress display:

```tsx
const AgentMain: Component = () => {
  const [task, setTask] = createSignal<Task | null>(null);
  const [events, setEvents] = createSignal<AgentEvent[]>([]);

  const runTask = async () => {
    await runTaskAgent(request, (event) => {
      setEvents(e => [...e, event]);
      // Handle different event types
    });
  };

  return (
    <div class="agent-main">
      <TaskSidebar onSelectTask={setTask} />
      <TaskPanel task={task()} events={events()} />
    </div>
  );
};
```

## State Management

### Settings Store

Global settings management:

```typescript
// stores/settings.ts
import { createSignal } from "solid-js";

export interface Settings {
  apiKey: string;
  model: string;
  baseUrl: string;
  maxTokens: number;
  temperature?: number;
  providerKeys: Record<string, string>;
}

const [settings, setSettings] = createSignal<Settings>(DEFAULT_SETTINGS);

export function useSettings() {
  return {
    settings,
    updateSetting: async <K extends keyof Settings>(
      key: K,
      value: Settings[K]
    ) => {
      const newSettings = { ...settings(), [key]: value };
      setSettings(newSettings);
      await persistSettings(newSettings);
    },
  };
}
```

### Chat Store

Message history and conversation state:

```typescript
// stores/chat.ts
import { createStore } from "solid-js/store";

export interface Message {
  id: string;
  role: "user" | "assistant";
  content: string;
  timestamp: number;
}

const [messages, setMessages] = createStore<Message[]>([]);

export function useChat() {
  return {
    messages,
    addMessage: (msg: Message) => {
      setMessages(m => [...m, msg]);
    },
    clearMessages: () => {
      setMessages([]);
    },
  };
}
```

## Tauri Integration

### API Bridge

Communication with the Rust backend:

```typescript
// lib/tauri-api.ts
import { invoke } from "@tauri-apps/api/core";
import { listen, UnlistenFn } from "@tauri-apps/api/event";

export async function sendChatMessage(
  conversationId: string,
  content: string,
  onStream: (text: string) => void
): Promise<string> {
  let unlisten: UnlistenFn | undefined;

  try {
    unlisten = await listen<StreamPayload>("chat-stream", (event) => {
      onStream(event.payload.text);
    });

    return await invoke<string>("send_chat_message", {
      conversationId,
      content,
    });
  } finally {
    unlisten?.();
  }
}
```

### Event Handling

Real-time updates from backend:

```typescript
// Listening for agent events
const unlisten = await listen<AgentEvent>("agent-event", (event) => {
  switch (event.payload.type) {
    case "text":
      updateContent(event.payload.content);
      break;
    case "tool_start":
      showToolExecution(event.payload.tool);
      break;
    case "tool_end":
      hideToolExecution();
      break;
    case "done":
      completeTask();
      break;
  }
});
```

## Styling

### CSS Organization

Each component has its own CSS file:

```css
/* Chat.css */
.chat {
  display: flex;
  flex-direction: column;
  height: 100%;
}

.chat-messages {
  flex: 1;
  overflow-y: auto;
  padding: 1rem;
}

.chat-input {
  border-top: 1px solid var(--border-color);
  padding: 1rem;
}
```

### Theme Variables

Global CSS variables for theming:

```css
:root {
  --primary-color: #6366f1;
  --background-color: #ffffff;
  --text-color: #1f2937;
  --border-color: #e5e7eb;
}

@media (prefers-color-scheme: dark) {
  :root {
    --background-color: #1f2937;
    --text-color: #f9fafb;
    --border-color: #374151;
  }
}
```

## Web Fallback

For development without Tauri:

```typescript
// lib/tauri-api.ts
export function isTauri(): boolean {
  return typeof window !== "undefined" &&
    ("__TAURI__" in window || "__TAURI_INTERNALS__" in window);
}

export async function getSettings(): Promise<Settings> {
  if (!isTauri()) {
    // Fallback to localStorage
    const stored = localStorage.getItem("kuse-cowork-settings");
    return stored ? JSON.parse(stored) : DEFAULT_SETTINGS;
  }
  return invoke<Settings>("get_settings");
}
```

## AI Client (Web Mode)

Direct AI provider access for web development:

```typescript
// lib/ai-client.ts
class AnthropicProvider implements AIProvider {
  async sendMessage(
    messages: AIMessage[],
    settings: Settings,
    onStream?: (text: string) => void
  ): Promise<string> {
    const response = await fetch(`${settings.baseUrl}/v1/messages`, {
      method: "POST",
      headers: {
        "x-api-key": settings.apiKey,
        "anthropic-version": "2023-06-01",
      },
      body: JSON.stringify({
        model: settings.model,
        messages,
        stream: !!onStream,
      }),
    });

    if (onStream) {
      return this.handleStreamResponse(response, onStream);
    }
    return this.handleResponse(response);
  }
}
```

## Performance Optimizations

### Reactive Updates

SolidJS fine-grained reactivity:

```tsx
// Only updates when specific signal changes
<Show when={loading()}>
  <Spinner />
</Show>

// Efficient list rendering
<For each={messages()}>
  {(message) => <MessageItem message={message} />}
</For>
```

### Memoization

```tsx
const filteredModels = createMemo(() => {
  return AVAILABLE_MODELS.filter(m =>
    m.provider === selectedProvider()
  );
});
```

### Lazy Loading

```tsx
const Settings = lazy(() => import("./components/Settings"));
```

## Build Configuration

### Vite Config

```typescript
// vite.config.ts
export default defineConfig({
  plugins: [solidPlugin()],
  server: {
    port: 1420,
    strictPort: true,
  },
  build: {
    target: "esnext",
    minify: "esbuild",
  },
});
```

### TypeScript Config

```json
{
  "compilerOptions": {
    "target": "ESNext",
    "module": "ESNext",
    "jsx": "preserve",
    "jsxImportSource": "solid-js",
    "strict": true
  }
}
```

## Testing

### Component Testing

```typescript
import { render, screen } from "@solidjs/testing-library";
import { Chat } from "./Chat";

test("renders chat input", () => {
  render(() => <Chat />);
  expect(screen.getByRole("textbox")).toBeInTheDocument();
});
```

### Integration Testing

```typescript
test("sends message to backend", async () => {
  const mockInvoke = vi.fn().mockResolvedValue("response");
  vi.mock("@tauri-apps/api/core", () => ({ invoke: mockInvoke }));

  // Test message sending
});
```

## Next Steps

- [Backend Architecture](backend.md)
- [Development Setup](../development/setup.md)
- [Contributing](../development/contributing.md)
