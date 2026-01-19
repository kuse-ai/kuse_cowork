# Backend Architecture

The Kuse Cowork backend is built with Rust and Tauri, providing high performance and native system integration.

## Technology Stack

| Technology | Version | Purpose |
|------------|---------|---------|
| Rust | 2021 Edition | Systems programming |
| Tauri | 2.0.x | Desktop framework |
| Tokio | 1.x | Async runtime |
| Reqwest | 0.12.x | HTTP client |
| SQLite | 3.x | Local database |
| Bollard | 0.18.x | Docker API |

## Project Structure

```
src-tauri/
├── Cargo.toml           # Dependencies
├── tauri.conf.json      # Tauri configuration
├── src/
│   ├── main.rs          # Entry point
│   ├── lib.rs           # Library root
│   ├── commands.rs      # Tauri commands (1800+ lines)
│   ├── database.rs      # Database operations
│   ├── llm_client.rs    # LLM providers (950+ lines)
│   ├── agent/
│   │   ├── mod.rs
│   │   ├── agent_loop.rs    # Agent execution (1000+ lines)
│   │   ├── message_builder.rs
│   │   ├── tool_executor.rs
│   │   └── types.rs
│   ├── tools/
│   │   ├── mod.rs
│   │   ├── bash.rs
│   │   ├── docker.rs
│   │   ├── file_read.rs
│   │   ├── file_write.rs
│   │   ├── file_edit.rs
│   │   ├── glob.rs
│   │   ├── grep.rs
│   │   └── list_dir.rs
│   ├── mcp/
│   │   ├── mod.rs
│   │   ├── client.rs
│   │   ├── http_client.rs
│   │   ├── config.rs
│   │   └── types.rs
│   └── skills/
│       └── mod.rs
```

## Core Modules

### Commands (commands.rs)

Tauri command handlers that bridge frontend and backend:

```rust
#[tauri::command]
pub async fn send_chat_message(
    state: tauri::State<'_, AppState>,
    window: tauri::Window,
    conversation_id: String,
    content: String,
) -> Result<String, CommandError> {
    // Get settings
    let settings = state.db.get_settings()?;

    // Create LLM client
    let client = LLMClient::new(
        settings.api_key,
        settings.base_url,
        settings.model,
    );

    // Stream response
    let response = client.send_message_stream(
        messages,
        |chunk| {
            window.emit("chat-stream", chunk)?;
            Ok(())
        }
    ).await?;

    // Save to database
    state.db.add_message(&response)?;

    Ok(response)
}
```

### Database (database.rs)

SQLite operations for persistent storage:

```rust
pub struct Database {
    conn: Mutex<Connection>,
}

impl Database {
    pub fn new(data_dir: &Path) -> Result<Self, DatabaseError> {
        let db_path = data_dir.join("settings.db");
        let conn = Connection::open(&db_path)?;

        // Initialize schema
        conn.execute_batch(SCHEMA)?;

        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    pub fn get_settings(&self) -> Result<Settings, DatabaseError> {
        let conn = self.conn.lock().unwrap();
        // Query settings...
    }

    pub fn save_settings(&self, settings: &Settings) -> Result<(), DatabaseError> {
        let conn = self.conn.lock().unwrap();
        // Insert/update settings...
    }
}
```

### LLM Client (llm_client.rs)

Multi-provider LLM integration:

```rust
pub struct LLMClient {
    client: Client,
    api_key: String,
    base_url: String,
    model: String,
    provider_config: ProviderConfig,
}

impl LLMClient {
    pub async fn send_message_stream(
        &self,
        messages: Vec<Message>,
        on_chunk: impl Fn(String) -> Result<(), LLMError>,
    ) -> Result<String, LLMError> {
        match self.provider_config.api_format {
            ApiFormat::Anthropic => self.send_anthropic_stream(messages, on_chunk).await,
            ApiFormat::OpenAI => self.send_openai_stream(messages, on_chunk).await,
            ApiFormat::Google => self.send_google_stream(messages, on_chunk).await,
            // ...
        }
    }
}
```

## Agent System

### Agent Loop (agent/agent_loop.rs)

Core autonomous agent execution:

```rust
pub struct AgentLoop {
    client: Client,
    api_key: String,
    base_url: String,
    config: AgentConfig,
    tool_executor: ToolExecutor,
    message_builder: MessageBuilder,
    provider_config: ProviderConfig,
}

impl AgentLoop {
    pub async fn run_with_history(
        &mut self,
        messages: Vec<AgentMessage>,
        event_tx: mpsc::Sender<AgentEvent>,
    ) -> Result<String, String> {
        let max_turns = self.config.max_turns;

        for turn in 1..=max_turns {
            // Build API request
            let request = self.message_builder.build_request(&messages).await;

            // Send to LLM
            let response = self.send_request(&request, &event_tx).await?;

            // Parse response
            let (text, tool_uses) = self.parse_response(&response)?;

            // Execute tools if needed
            if !tool_uses.is_empty() {
                for tool_use in &tool_uses {
                    let result = self.tool_executor.execute(tool_use).await;
                    // Add result to messages...
                }
            } else {
                // No more tools, we're done
                return Ok(text);
            }
        }

        Err("Max turns exceeded".to_string())
    }
}
```

### Message Builder (agent/message_builder.rs)

Constructs API requests:

```rust
pub struct MessageBuilder {
    config: AgentConfig,
    model: String,
    max_tokens: u32,
    mcp_manager: Option<Arc<MCPManager>>,
}

impl MessageBuilder {
    pub async fn build_request(
        &self,
        messages: &[AgentMessage],
    ) -> ClaudeApiRequest {
        let mut tools = tools::get_tools(&self.config.allowed_tools);

        // Add MCP tools
        if let Some(mcp) = &self.mcp_manager {
            tools.extend(self.get_mcp_tools(mcp).await);
        }

        ClaudeApiRequest {
            model: self.model.clone(),
            max_tokens: self.max_tokens,
            system: self.config.system_prompt.clone(),
            messages: self.convert_messages(messages),
            tools,
            stream: true,
        }
    }
}
```

### Tool Executor (agent/tool_executor.rs)

Executes agent tools:

```rust
pub struct ToolExecutor {
    project_path: Option<String>,
    mcp_manager: Option<Arc<MCPManager>>,
}

impl ToolExecutor {
    pub async fn execute(&self, tool_use: &ToolUse) -> ToolResult {
        // Check for MCP tools
        if tool_use.name.starts_with("mcp_") {
            return self.execute_mcp_tool(tool_use).await;
        }

        // Execute built-in tools
        match tool_use.name.as_str() {
            "read_file" => tools::read_file(tool_use),
            "write_file" => tools::write_file(tool_use),
            "edit_file" => tools::edit_file(tool_use),
            "bash" => tools::bash(tool_use, &self.project_path).await,
            "glob" => tools::glob(tool_use, &self.project_path),
            "grep" => tools::grep(tool_use, &self.project_path),
            "list_dir" => tools::list_dir(tool_use),
            "docker_run" => tools::docker_run(tool_use, &self.project_path),
            _ => ToolResult::error(tool_use.id.clone(), "Unknown tool"),
        }
    }
}
```

## Tool Implementations

### File Operations

```rust
// tools/file_read.rs
pub fn read_file(tool_use: &ToolUse) -> ToolResult {
    let path = tool_use.input.get("path")
        .and_then(|v| v.as_str())
        .ok_or("Missing path")?;

    // Expand ~ to home directory
    let expanded = expand_tilde(path);

    let content = std::fs::read_to_string(&expanded)?;

    ToolResult::success(tool_use.id.clone(), content)
}
```

### Docker Integration

```rust
// tools/docker.rs
pub async fn docker_run(
    tool_use: &ToolUse,
    project_path: &Option<String>,
) -> ToolResult {
    let docker = Docker::connect_with_local_defaults()?;

    let image = tool_use.input.get("image")
        .and_then(|v| v.as_str())
        .unwrap_or("python:3.11-alpine");

    let command = tool_use.input.get("command")
        .and_then(|v| v.as_str())
        .ok_or("Missing command")?;

    // Create container config
    let config = Config {
        image: Some(image),
        cmd: Some(vec!["/bin/sh", "-c", command]),
        host_config: Some(HostConfig {
            binds: Some(vec![
                format!("{}:/workspace", project_path),
                format!("{}:/skills:ro", skills_path),
            ]),
            ..Default::default()
        }),
        ..Default::default()
    };

    // Create and run container
    let container = docker.create_container(None, config).await?;
    docker.start_container(&container.id, None).await?;

    // Wait and get output
    let output = docker.wait_container(&container.id).await?;
    let logs = docker.logs(&container.id, options).await?;

    // Cleanup
    docker.remove_container(&container.id, None).await?;

    ToolResult::success(tool_use.id.clone(), logs)
}
```

## MCP Integration

### MCP Manager (mcp/client.rs)

```rust
pub struct MCPManager {
    servers: RwLock<HashMap<String, MCPServer>>,
    http_client: Client,
}

impl MCPManager {
    pub async fn add_server(&self, config: MCPServerConfig) -> Result<(), MCPError> {
        let server = MCPServer::connect(config).await?;
        self.servers.write().await.insert(server.id.clone(), server);
        Ok(())
    }

    pub async fn get_all_tools(&self) -> Vec<MCPTool> {
        let servers = self.servers.read().await;
        let mut tools = Vec::new();

        for server in servers.values() {
            if server.is_connected() {
                tools.extend(server.tools.clone());
            }
        }

        tools
    }

    pub async fn execute_tool(&self, call: &MCPToolCall) -> Result<String, MCPError> {
        let servers = self.servers.read().await;
        let server = servers.get(&call.server_id)
            .ok_or(MCPError::ServerNotFound)?;

        server.call_tool(&call.tool_name, &call.parameters).await
    }
}
```

## Error Handling

### Error Types

```rust
#[derive(Error, Debug)]
pub enum LLMError {
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("API error: {0}")]
    Api(String),

    #[error("Parse error: {0}")]
    Parse(String),

    #[error("Unsupported provider: {0}")]
    UnsupportedProvider(String),
}

#[derive(Debug, Serialize)]
pub struct CommandError {
    pub message: String,
}

impl From<LLMError> for CommandError {
    fn from(err: LLMError) -> Self {
        CommandError {
            message: err.to_string(),
        }
    }
}
```

## Async Architecture

### Tokio Runtime

```rust
// main.rs
#[tokio::main]
async fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_fs::init())
        .invoke_handler(tauri::generate_handler![
            commands::get_settings,
            commands::save_settings,
            commands::send_chat_message,
            commands::run_task_agent,
            // ...
        ])
        .run(tauri::generate_context!())
        .expect("error running application");
}
```

### Streaming with Channels

```rust
use tokio::sync::mpsc;

pub async fn run_agent_with_events(
    request: AgentRequest,
    window: Window,
) -> Result<String, CommandError> {
    let (tx, mut rx) = mpsc::channel(100);

    // Spawn event forwarder
    let event_window = window.clone();
    tokio::spawn(async move {
        while let Some(event) = rx.recv().await {
            let _ = event_window.emit("agent-event", event);
        }
    });

    // Run agent
    let result = agent_loop.run_with_history(messages, tx).await;

    Ok(result?)
}
```

## Testing

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_detection() {
        assert_eq!(
            ProviderConfig::from_model("claude-3-sonnet").api_format,
            ApiFormat::Anthropic
        );
        assert_eq!(
            ProviderConfig::from_model("gpt-4o").api_format,
            ApiFormat::OpenAI
        );
    }

    #[tokio::test]
    async fn test_tool_execution() {
        let executor = ToolExecutor::new(None);
        let tool_use = ToolUse {
            id: "test".to_string(),
            name: "list_dir".to_string(),
            input: json!({"path": "."}),
        };

        let result = executor.execute(&tool_use).await;
        assert!(result.is_error.is_none());
    }
}
```

### Integration Tests

```rust
#[cfg(test)]
mod integration_tests {
    #[tokio::test]
    async fn test_full_agent_flow() {
        // Create agent with mock LLM
        // Run a simple task
        // Verify tools are called
        // Check final output
    }
}
```

## Performance Optimizations

### Connection Pooling

```rust
let client = reqwest::Client::builder()
    .pool_max_idle_per_host(10)
    .timeout(Duration::from_secs(300))
    .build()?;
```

### Parallel Tool Execution

```rust
// Future: Execute independent tools in parallel
let results = futures::future::join_all(
    tool_uses.iter().map(|tu| self.tool_executor.execute(tu))
).await;
```

## Security Considerations

### Input Validation

```rust
fn validate_path(path: &str) -> Result<PathBuf, ToolError> {
    let expanded = expand_tilde(path);
    let canonical = expanded.canonicalize()?;

    // Prevent path traversal
    if !canonical.starts_with(&project_root) {
        return Err(ToolError::AccessDenied);
    }

    Ok(canonical)
}
```

### Command Sanitization

```rust
// Commands run in Docker are isolated
// But we still validate inputs
fn sanitize_command(cmd: &str) -> Result<String, ToolError> {
    if cmd.contains("rm -rf /") || cmd.contains(":(){ :|:& };:") {
        return Err(ToolError::DangerousCommand);
    }
    Ok(cmd.to_string())
}
```

## Next Steps

- [Architecture Overview](overview.md)
- [Development Setup](../development/setup.md)
- [Contributing](../development/contributing.md)
