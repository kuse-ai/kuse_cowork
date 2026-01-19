use crate::agent::{AgentConfig, AgentContent, AgentEvent, AgentLoop, AgentMessage};
use crate::claude::{ClaudeClient, Message as ClaudeMessage};
use crate::database::{Conversation, Database, Message, PlanStep, Settings, Task, TaskMessage};
use crate::mcp::{MCPManager, MCPServerConfig, MCPServerStatus, MCPToolCall, MCPToolResult};
use crate::skills::{SkillMetadata, get_available_skills};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::{command, Emitter, State, Window};
use tokio::sync::Mutex;

pub struct AppState {
    pub db: Arc<Database>,
    pub claude_client: Mutex<Option<ClaudeClient>>,
    pub mcp_manager: Arc<MCPManager>,
}

#[derive(Debug, Serialize)]
pub struct CommandError {
    message: String,
}

impl From<crate::database::DbError> for CommandError {
    fn from(e: crate::database::DbError) -> Self {
        CommandError {
            message: e.to_string(),
        }
    }
}

impl From<crate::claude::ClaudeError> for CommandError {
    fn from(e: crate::claude::ClaudeError) -> Self {
        CommandError {
            message: e.to_string(),
        }
    }
}

// Platform command
#[command]
pub fn get_platform() -> String {
    #[cfg(target_os = "macos")]
    return "darwin".to_string();

    #[cfg(target_os = "windows")]
    return "windows".to_string();

    #[cfg(target_os = "linux")]
    return "linux".to_string();

    #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
    return "unknown".to_string();
}

// Settings commands
#[command]
pub fn get_settings(state: State<'_, Arc<AppState>>) -> Result<Settings, CommandError> {
    let settings = state.db.get_settings()?;
    println!("[get_settings] api_key length from db: {}", settings.api_key.len());
    Ok(settings)
}

#[command]
pub async fn save_settings(
    state: State<'_, Arc<AppState>>,
    settings: Settings,
) -> Result<(), CommandError> {
    println!("[save_settings] model: {}", settings.model);
    println!("[save_settings] base_url: {}", settings.base_url);
    println!("[save_settings] api_key length: {}", settings.api_key.len());
    // Show first and last 10 chars for debugging
    if settings.api_key.len() > 20 {
        println!("[save_settings] api_key preview: {}...{}",
            &settings.api_key[..10],
            &settings.api_key[settings.api_key.len()-10..]);
    }

    state.db.save_settings(&settings)?;

    // Update Claude client with new settings
    let mut client = state.claude_client.lock().await;
    if !settings.api_key.is_empty() {
        *client = Some(ClaudeClient::new(
            settings.api_key.clone(),
            Some(settings.base_url.clone()),
        ));
    } else {
        *client = None;
    }

    Ok(())
}

#[command]
pub async fn test_connection(state: State<'_, Arc<AppState>>) -> Result<String, CommandError> {
    use crate::llm_client::LLMClient;

    let settings = state.db.get_settings()?;

    // Debug logging
    println!("[test_connection] model: {}", settings.model);
    println!("[test_connection] base_url: {}", settings.base_url);
    println!("[test_connection] api_key length: {}", settings.api_key.len());
    println!("[test_connection] provider: {}", settings.get_provider());
    println!("[test_connection] is_local_provider: {}", settings.is_local_provider());

    if settings.api_key.is_empty() && !settings.is_local_provider() {
        return Ok("No API key configured".to_string());
    }

    // Choose test method based on provider type
    if settings.is_local_provider() {
        // Local service - use LLMClient to check connection
        let llm_client = LLMClient::new(
            String::new(), // Local services don't need API key
            Some(settings.base_url.clone()),
            None,
            Some(&settings.model),
        );

        match llm_client.check_connection().await {
            Ok(true) => Ok("success".to_string()),
            Ok(false) => Ok("Error: Cannot connect to local service, please ensure it is running".to_string()),
            Err(e) => Ok(format!("Error: {}", e)),
        }
    } else {
        // Cloud service - check provider type
        let provider = settings.get_provider();

        match provider.as_str() {
            "anthropic" => {
                // Anthropic - use ClaudeClient
                let client = ClaudeClient::new(settings.api_key, Some(settings.base_url));
                let messages = vec![ClaudeMessage {
                    role: "user".to_string(),
                    content: "Hi".to_string(),
                }];

                match client.send_message(messages, &settings.model, 10, None).await {
                    Ok(_) => Ok("success".to_string()),
                    Err(e) => Ok(format!("Error: {}", e)),
                }
            }
            _ => {
                // Other cloud services - use LLMClient
                let llm_client = LLMClient::new(
                    settings.api_key.clone(),
                    Some(settings.base_url.clone()),
                    None,
                    Some(&settings.model),
                );

                match llm_client.check_connection().await {
                    Ok(true) => Ok("success".to_string()),
                    Ok(false) => Ok("Error: Cannot connect to service".to_string()),
                    Err(e) => Ok(format!("Error: {}", e)),
                }
            }
        }
    }
}

// Conversation commands
#[command]
pub fn list_conversations(
    state: State<'_, Arc<AppState>>,
) -> Result<Vec<Conversation>, CommandError> {
    state.db.list_conversations().map_err(Into::into)
}

#[command]
pub fn create_conversation(
    state: State<'_, Arc<AppState>>,
    title: String,
) -> Result<Conversation, CommandError> {
    let id = uuid::Uuid::new_v4().to_string();
    state.db.create_conversation(&id, &title).map_err(Into::into)
}

#[command]
pub fn update_conversation_title(
    state: State<'_, Arc<AppState>>,
    id: String,
    title: String,
) -> Result<(), CommandError> {
    state.db.update_conversation_title(&id, &title).map_err(Into::into)
}

#[command]
pub fn delete_conversation(
    state: State<'_, Arc<AppState>>,
    id: String,
) -> Result<(), CommandError> {
    state.db.delete_conversation(&id).map_err(Into::into)
}

// Message commands
#[command]
pub fn get_messages(
    state: State<'_, Arc<AppState>>,
    conversation_id: String,
) -> Result<Vec<Message>, CommandError> {
    state.db.get_messages(&conversation_id).map_err(Into::into)
}

#[command]
pub fn add_message(
    state: State<'_, Arc<AppState>>,
    conversation_id: String,
    role: String,
    content: String,
) -> Result<Message, CommandError> {
    let id = uuid::Uuid::new_v4().to_string();
    state
        .db
        .add_message(&id, &conversation_id, &role, &content)
        .map_err(Into::into)
}

// Chat command with streaming
#[derive(Clone, Serialize)]
struct StreamPayload {
    text: String,
    done: bool,
}

#[command]
pub async fn send_chat_message(
    window: Window,
    state: State<'_, Arc<AppState>>,
    conversation_id: String,
    content: String,
) -> Result<String, CommandError> {
    let settings = state.db.get_settings()?;

    if settings.api_key.is_empty() && !settings.is_local_provider() {
        return Err(CommandError {
            message: "API key not configured".to_string(),
        });
    }

    // Add user message to database
    let user_msg_id = uuid::Uuid::new_v4().to_string();
    state
        .db
        .add_message(&user_msg_id, &conversation_id, "user", &content)?;

    // Get conversation history
    let db_messages = state.db.get_messages(&conversation_id)?;
    let claude_messages: Vec<ClaudeMessage> = db_messages
        .iter()
        .map(|m| ClaudeMessage {
            role: m.role.clone(),
            content: m.content.clone(),
        })
        .collect();

    // Create Claude client
    let client = ClaudeClient::new(settings.api_key, Some(settings.base_url));

    // Create channel for streaming
    let (tx, mut rx) = tokio::sync::mpsc::channel::<String>(100);

    // Spawn task to emit events
    let window_clone = window.clone();
    let emit_task = tokio::spawn(async move {
        while let Some(text) = rx.recv().await {
            let _ = window_clone.emit("chat-stream", StreamPayload { text, done: false });
        }
    });

    // Send message
    let response = client
        .send_message_stream(
            claude_messages,
            &settings.model,
            settings.max_tokens,
            Some(settings.temperature),
            tx,
        )
        .await?;

    // Wait for emit task to finish
    let _ = emit_task.await;

    // Emit done event
    let _ = window.emit(
        "chat-stream",
        StreamPayload {
            text: response.clone(),
            done: true,
        },
    );

    // Save assistant response to database
    let assistant_msg_id = uuid::Uuid::new_v4().to_string();
    state
        .db
        .add_message(&assistant_msg_id, &conversation_id, "assistant", &response)?;

    // Update conversation title if this is the first message
    if db_messages.len() == 1 {
        let title = if content.len() > 30 {
            format!("{}...", &content[..30])
        } else {
            content.clone()
        };
        state.db.update_conversation_title(&conversation_id, &title)?;
    }

    Ok(response)
}

// Chat event for tool-enabled chat
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type")]
pub enum ChatEvent {
    #[serde(rename = "text")]
    Text { content: String },
    #[serde(rename = "tool_start")]
    ToolStart { tool: String, input: serde_json::Value },
    #[serde(rename = "tool_end")]
    ToolEnd { tool: String, result: String, success: bool },
    #[serde(rename = "done")]
    Done { final_text: String },
}

// Agent command
#[derive(Debug, Deserialize)]
pub struct AgentRequest {
    pub message: String,
    pub project_path: Option<String>,
    pub system_prompt: Option<String>,
    pub max_turns: Option<u32>,
}

#[command]
pub async fn run_agent(
    window: Window,
    state: State<'_, Arc<AppState>>,
    request: AgentRequest,
) -> Result<String, CommandError> {
    let settings = state.db.get_settings()?;

    // Check if API Key is needed (local services don't need it)
    if settings.api_key.is_empty() && !settings.is_local_provider() {
        return Err(CommandError {
            message: "API key not configured".to_string(),
        });
    }

    // Build agent config
    let mut config = AgentConfig::default();
    if let Some(prompt) = request.system_prompt {
        config.system_prompt = prompt;
    } else {
        // Add MCP servers info to default system prompt
        let mcp_servers = state.mcp_manager.get_server_statuses().await;
        let mut mcp_info = String::new();
        if !mcp_servers.is_empty() {
            mcp_info.push_str("\nMCP (Model Context Protocol) Tools:\n");
            for server in mcp_servers {
                if matches!(server.status, crate::mcp::types::ConnectionStatus::Connected) {
                    mcp_info.push_str(&format!("Server '{}' is connected with tools:\n", server.id));
                    for tool in server.tools {
                        mcp_info.push_str(&format!("  - {}: {} (use format: {}:{})\n",
                            tool.name, tool.description, server.id, tool.name));
                    }
                }
            }
        }
        if !mcp_info.is_empty() {
            config.system_prompt.push_str(&mcp_info);
        }
    }
    if let Some(turns) = request.max_turns {
        config.max_turns = turns;
    }
    config.project_path = request.project_path;

    // Get provider info
    let provider_id = settings.get_provider();

    // Create agent loop with provider
    let agent = AgentLoop::new_with_provider(
        settings.api_key,
        settings.base_url,
        config,
        settings.model,
        settings.max_tokens,
        Some(settings.temperature),
        state.mcp_manager.clone(),
        Some(&provider_id),
    );

    // Create channel for events
    let (tx, mut rx) = tokio::sync::mpsc::channel::<AgentEvent>(100);

    // Spawn event emitter
    let window_clone = window.clone();
    let emit_task = tokio::spawn(async move {
        while let Some(event) = rx.recv().await {
            let _ = window_clone.emit("agent-event", &event);
        }
    });

    // Run agent
    let result = agent.run(request.message, tx).await;

    // Wait for emitter to finish
    let _ = emit_task.await;

    match result {
        Ok(_messages) => Ok("Agent completed successfully".to_string()),
        Err(e) => Err(CommandError { message: e }),
    }
}

// Enhanced chat with tools - integrates agent capabilities into chat
#[derive(Debug, Deserialize)]
pub struct EnhancedChatRequest {
    pub conversation_id: String,
    pub content: String,
    pub project_path: Option<String>,
    pub enable_tools: bool,
}

#[command]
pub async fn send_chat_with_tools(
    window: Window,
    state: State<'_, Arc<AppState>>,
    request: EnhancedChatRequest,
) -> Result<String, CommandError> {
    use crate::agent::{
        AgentConfig, AgentContent, AgentMessage, ContentBlock, MessageBuilder, ToolExecutor, ToolUse,
    };
    use futures::StreamExt;

    let settings = state.db.get_settings()?;

    if settings.api_key.is_empty() && !settings.is_local_provider() {
        return Err(CommandError {
            message: "API key not configured".to_string(),
        });
    }

    // Add user message to database
    let user_msg_id = uuid::Uuid::new_v4().to_string();
    state
        .db
        .add_message(&user_msg_id, &request.conversation_id, "user", &request.content)?;

    // Get conversation history
    let db_messages = state.db.get_messages(&request.conversation_id)?;

    // If tools are not enabled, fall back to simple chat
    if !request.enable_tools {
        // Simple streaming chat
        let claude_messages: Vec<ClaudeMessage> = db_messages
            .iter()
            .map(|m| ClaudeMessage {
                role: m.role.clone(),
                content: m.content.clone(),
            })
            .collect();

        let client = ClaudeClient::new(settings.api_key, Some(settings.base_url));
        let (tx, mut rx) = tokio::sync::mpsc::channel::<String>(100);

        let window_clone = window.clone();
        let emit_task = tokio::spawn(async move {
            while let Some(text) = rx.recv().await {
                let _ = window_clone.emit("chat-event", ChatEvent::Text { content: text });
            }
        });

        let response = client
            .send_message_stream(
                claude_messages,
                &settings.model,
                settings.max_tokens,
                Some(settings.temperature),
                tx,
            )
            .await?;

        let _ = emit_task.await;
        let _ = window.emit("chat-event", ChatEvent::Done { final_text: response.clone() });

        // Save assistant response
        let assistant_msg_id = uuid::Uuid::new_v4().to_string();
        state
            .db
            .add_message(&assistant_msg_id, &request.conversation_id, "assistant", &response)?;

        return Ok(response);
    }

    // Enhanced chat with tools
    let tool_executor = ToolExecutor::new(request.project_path.clone())
        .with_mcp_manager(state.mcp_manager.clone());

    // Build agent-style config for tools
    let mut config = AgentConfig {
        project_path: request.project_path,
        max_turns: 10, // Limit turns in chat mode
        ..Default::default()
    };

    // System prompt for chat with tools - include MCP servers info
    let mcp_servers = state.mcp_manager.get_server_statuses().await;
    let mut mcp_info = String::new();
    if !mcp_servers.is_empty() {
        mcp_info.push_str("\nMCP (Model Context Protocol) Tools:\n");
        for server in mcp_servers {
            if matches!(server.status, crate::mcp::types::ConnectionStatus::Connected) {
                mcp_info.push_str(&format!("Server '{}' is connected with tools:\n", server.id));
                for tool in server.tools {
                    mcp_info.push_str(&format!("  - {}: {} (use format: {}:{})\n",
                        tool.name, tool.description, server.id, tool.name));
                }
            }
        }
    }

    config.system_prompt = format!(r#"You are Kuse Cowork, an AI assistant that helps users for non dev work.

You have access to tools that allow you to read and write files, execute commands, and search through codebases.

When the user asks you to do something that requires accessing files or running commands, use the appropriate tools.
For simple questions or conversations, respond directly without using tools.

Be concise and helpful. Explain what you're doing when using tools.{}"#, mcp_info);

    let message_builder = MessageBuilder::new(
        config.clone(),
        settings.model.clone(),
        settings.max_tokens,
        Some(settings.temperature),
    );

    // Convert DB messages to agent messages
    let mut agent_messages: Vec<AgentMessage> = db_messages
        .iter()
        .map(|m| AgentMessage {
            role: m.role.clone(),
            content: AgentContent::Text(m.content.clone()),
        })
        .collect();

    let client = reqwest::Client::new();
    let mut final_text = String::new();
    let mut turn = 0;
    let max_turns = config.max_turns;

    loop {
        turn += 1;
        if turn > max_turns {
            break;
        }

        // Build and send request
        let api_request = message_builder.build_request(&agent_messages).await;

        let response = client
            .post(format!("{}/v1/messages", settings.base_url))
            .header("Content-Type", "application/json")
            .header("x-api-key", &settings.api_key)
            .header("anthropic-version", "2023-06-01")
            .json(&api_request)
            .send()
            .await
            .map_err(|e| CommandError { message: format!("HTTP error: {}", e) })?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(CommandError { message: format!("API error: {}", error_text) });
        }

        // Handle streaming response
        let mut stream = response.bytes_stream();
        let mut buffer = String::new();
        let mut accumulated_text = String::new();
        let mut tool_uses: Vec<ToolUse> = Vec::new();
        let mut current_tool_input = String::new();
        let mut current_tool_id = String::new();
        let mut current_tool_name = String::new();

        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(|e| CommandError { message: format!("Stream error: {}", e) })?;
            buffer.push_str(&String::from_utf8_lossy(&chunk));

            while let Some(pos) = buffer.find('\n') {
                let line = buffer[..pos].to_string();
                buffer = buffer[pos + 1..].to_string();

                if let Some(data) = line.strip_prefix("data: ") {
                    if data == "[DONE]" {
                        continue;
                    }

                    if let Ok(event) = serde_json::from_str::<serde_json::Value>(data) {
                        let event_type = event.get("type").and_then(|v| v.as_str()).unwrap_or("");

                        match event_type {
                            "content_block_start" => {
                                if let Some(block) = event.get("content_block") {
                                    if block.get("type").and_then(|v| v.as_str()) == Some("tool_use") {
                                        current_tool_id = block
                                            .get("id")
                                            .and_then(|v| v.as_str())
                                            .unwrap_or("")
                                            .to_string();
                                        current_tool_name = block
                                            .get("name")
                                            .and_then(|v| v.as_str())
                                            .unwrap_or("")
                                            .to_string();
                                        current_tool_input.clear();
                                    }
                                }
                            }
                            "content_block_delta" => {
                                if let Some(delta) = event.get("delta") {
                                    let delta_type = delta.get("type").and_then(|v| v.as_str()).unwrap_or("");

                                    if delta_type == "text_delta" {
                                        if let Some(text) = delta.get("text").and_then(|v| v.as_str()) {
                                            accumulated_text.push_str(text);
                                            let _ = window.emit("chat-event", ChatEvent::Text {
                                                content: accumulated_text.clone(),
                                            });
                                        }
                                    } else if delta_type == "input_json_delta" {
                                        if let Some(partial) = delta.get("partial_json").and_then(|v| v.as_str()) {
                                            current_tool_input.push_str(partial);
                                        }
                                    }
                                }
                            }
                            "content_block_stop" => {
                                if !current_tool_id.is_empty() {
                                    let input: serde_json::Value = serde_json::from_str(&current_tool_input)
                                        .unwrap_or(serde_json::json!({}));

                                    tool_uses.push(ToolUse {
                                        id: current_tool_id.clone(),
                                        name: current_tool_name.clone(),
                                        input: input.clone(),
                                    });

                                    // Emit tool start
                                    let _ = window.emit("chat-event", ChatEvent::ToolStart {
                                        tool: current_tool_name.clone(),
                                        input,
                                    });

                                    current_tool_id.clear();
                                    current_tool_name.clear();
                                    current_tool_input.clear();
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
        }

        // Update final text
        if !accumulated_text.is_empty() {
            final_text = accumulated_text.clone();
        }

        // Add assistant message to history
        let assistant_content = if tool_uses.is_empty() {
            AgentContent::Text(accumulated_text)
        } else {
            let mut blocks = Vec::new();
            if !accumulated_text.is_empty() {
                blocks.push(ContentBlock::Text { text: accumulated_text });
            }
            for tu in &tool_uses {
                blocks.push(ContentBlock::ToolUse {
                    id: tu.id.clone(),
                    name: tu.name.clone(),
                    input: tu.input.clone(),
                });
            }
            AgentContent::Blocks(blocks)
        };

        agent_messages.push(AgentMessage {
            role: "assistant".to_string(),
            content: assistant_content,
        });

        // If no tool uses, we're done
        if tool_uses.is_empty() {
            break;
        }

        // Execute tools
        let mut tool_results = Vec::new();

        for tool_use in &tool_uses {
            let result = tool_executor.execute(tool_use).await;

            // Emit tool end
            let _ = window.emit("chat-event", ChatEvent::ToolEnd {
                tool: tool_use.name.clone(),
                result: result.content.clone(),
                success: result.is_error.is_none(),
            });

            tool_results.push(result);
        }

        // Add tool results as user message
        agent_messages.push(AgentMessage {
            role: "user".to_string(),
            content: AgentContent::ToolResults(tool_results),
        });
    }

    // Emit done
    let _ = window.emit("chat-event", ChatEvent::Done { final_text: final_text.clone() });

    // Save final assistant response to database
    let assistant_msg_id = uuid::Uuid::new_v4().to_string();
    state
        .db
        .add_message(&assistant_msg_id, &request.conversation_id, "assistant", &final_text)?;

    // Update conversation title if this is the first exchange
    if db_messages.len() == 1 {
        let title = if request.content.len() > 30 {
            format!("{}...", &request.content[..30])
        } else {
            request.content.clone()
        };
        state.db.update_conversation_title(&request.conversation_id, &title)?;
    }

    Ok(final_text)
}

// Task commands
#[command]
pub fn list_tasks(state: State<'_, Arc<AppState>>) -> Result<Vec<Task>, CommandError> {
    state.db.list_tasks().map_err(Into::into)
}

#[command]
pub fn get_task(state: State<'_, Arc<AppState>>, id: String) -> Result<Option<Task>, CommandError> {
    state.db.get_task(&id).map_err(Into::into)
}

#[command]
pub fn create_task(
    state: State<'_, Arc<AppState>>,
    title: String,
    description: String,
    project_path: Option<String>,
) -> Result<Task, CommandError> {
    let id = uuid::Uuid::new_v4().to_string();
    state.db.create_task(&id, &title, &description, project_path.as_deref()).map_err(Into::into)
}

#[command]
pub fn delete_task(state: State<'_, Arc<AppState>>, id: String) -> Result<(), CommandError> {
    state.db.delete_task(&id).map_err(Into::into)
}

// Run agent with task tracking
#[derive(Debug, Deserialize)]
pub struct TaskAgentRequest {
    pub task_id: String,
    pub message: String,
    pub project_path: Option<String>,
    pub max_turns: Option<u32>,
}

#[command]
pub async fn run_task_agent(
    window: Window,
    state: State<'_, Arc<AppState>>,
    request: TaskAgentRequest,
) -> Result<String, CommandError> {
    let settings = state.db.get_settings()?;

    // Check if API Key is needed (local services don't need it)
    if settings.api_key.is_empty() && !settings.is_local_provider() {
        return Err(CommandError {
            message: "API key not configured".to_string(),
        });
    }

    // Load existing conversation history
    let existing_messages = state.db.get_task_messages(&request.task_id)?;

    // Save new user message
    let user_msg_id = uuid::Uuid::new_v4().to_string();
    state.db.add_task_message(&user_msg_id, &request.task_id, "user", &request.message)?;

    // Update task status to running
    state.db.update_task_status(&request.task_id, "running")?;

    // Build agent config with MCP servers info
    let mut config = AgentConfig::default();

    // Add MCP servers info to system prompt
    let mcp_servers = state.mcp_manager.get_server_statuses().await;
    let mut mcp_info = String::new();
    if !mcp_servers.is_empty() {
        mcp_info.push_str("\nMCP (Model Context Protocol) Tools:\n");
        for server in mcp_servers {
            if matches!(server.status, crate::mcp::types::ConnectionStatus::Connected) {
                mcp_info.push_str(&format!("Server '{}' is connected with tools:\n", server.id));
                for tool in server.tools {
                    mcp_info.push_str(&format!("  - {}: {} (use format: {}:{})\n",
                        tool.name, tool.description, server.id, tool.name));
                }
            }
        }
    }
    if !mcp_info.is_empty() {
        config.system_prompt.push_str(&mcp_info);
    }

    if let Some(turns) = request.max_turns {
        config.max_turns = turns;
    }
    config.project_path = request.project_path;

    // Get provider info
    let provider_id = settings.get_provider();

    // Create agent loop with provider
    let agent = AgentLoop::new_with_provider(
        settings.api_key,
        settings.base_url,
        config,
        settings.model,
        settings.max_tokens,
        Some(settings.temperature),
        state.mcp_manager.clone(),
        Some(&provider_id),
    );

    // Build conversation history from existing messages
    let mut agent_messages: Vec<AgentMessage> = existing_messages
        .iter()
        .map(|m| AgentMessage {
            role: m.role.clone(),
            content: AgentContent::Text(m.content.clone()),
        })
        .collect();

    // Add the new user message
    agent_messages.push(AgentMessage {
        role: "user".to_string(),
        content: AgentContent::Text(request.message.clone()),
    });

    // Create channel for events
    let (tx, mut rx) = tokio::sync::mpsc::channel::<AgentEvent>(100);

    // Clone state for event handler
    let task_id = request.task_id.clone();
    let task_id_for_msg = request.task_id.clone();
    let db = state.db.clone();
    let db_for_msg = state.db.clone();

    // Track accumulated text for saving
    let accumulated_text = std::sync::Arc::new(std::sync::Mutex::new(String::new()));
    let accumulated_text_clone = accumulated_text.clone();

    // Spawn event emitter with task tracking
    let window_clone = window.clone();
    let emit_task = tokio::spawn(async move {
        while let Some(event) = rx.recv().await {
            // Track plan and step updates in database
            match &event {
                AgentEvent::Text { content } => {
                    // Update accumulated text
                    if let Ok(mut text) = accumulated_text_clone.lock() {
                        *text = content.clone();
                    }
                }
                AgentEvent::Plan { steps } => {
                    let plan_steps: Vec<PlanStep> = steps.iter().map(|s| PlanStep {
                        step: s.step,
                        description: s.description.clone(),
                        status: "pending".to_string(),
                    }).collect();
                    let _ = db.update_task_plan(&task_id, &plan_steps);
                }
                AgentEvent::StepStart { step } => {
                    let _ = db.update_task_step(&task_id, *step, "running");
                }
                AgentEvent::StepDone { step } => {
                    let _ = db.update_task_step(&task_id, *step, "completed");
                }
                AgentEvent::Done { .. } => {
                    let _ = db.update_task_status(&task_id, "completed");
                }
                AgentEvent::Error { .. } => {
                    let _ = db.update_task_status(&task_id, "failed");
                }
                _ => {}
            }

            // Emit to frontend
            let _ = window_clone.emit("agent-event", &event);
        }
    });

    // Run agent with conversation history
    let result = agent.run_with_history(agent_messages, tx).await;

    // Wait for emitter to finish
    let _ = emit_task.await;

    // Save assistant message with accumulated text
    let final_text = accumulated_text.lock().map(|t| t.clone()).unwrap_or_default();
    if !final_text.is_empty() {
        let assistant_msg_id = uuid::Uuid::new_v4().to_string();
        let _ = db_for_msg.add_task_message(&assistant_msg_id, &task_id_for_msg, "assistant", &final_text);
    }

    // Always ensure task status is updated at the end
    match result {
        Ok(_messages) => {
            // Explicitly update to completed (in case event was missed)
            let _ = state.db.update_task_status(&request.task_id, "completed");
            Ok("Task completed successfully".to_string())
        }
        Err(e) => {
            state.db.update_task_status(&request.task_id, "failed")?;
            Err(CommandError { message: e })
        }
    }
}

// Get task messages command
#[command]
pub fn get_task_messages(
    state: State<'_, Arc<AppState>>,
    task_id: String,
) -> Result<Vec<TaskMessage>, CommandError> {
    state.db.get_task_messages(&task_id).map_err(Into::into)
}

// Skills commands
#[command]
pub fn get_skills_list() -> Vec<SkillMetadata> {
    get_available_skills()
}

// MCP commands
#[command]
pub fn list_mcp_servers(state: State<'_, Arc<AppState>>) -> Result<Vec<MCPServerConfig>, CommandError> {
    state.db.get_mcp_servers().map_err(|e| CommandError {
        message: format!("Failed to get MCP servers: {}", e)
    })
}

#[command]
pub fn save_mcp_server(
    state: State<'_, Arc<AppState>>,
    config: MCPServerConfig,
) -> Result<(), CommandError> {
    state.db.save_mcp_server(&config).map_err(|e| CommandError {
        message: format!("Failed to save MCP server: {}", e)
    })
}

#[command]
pub fn delete_mcp_server(
    state: State<'_, Arc<AppState>>,
    id: String,
) -> Result<(), CommandError> {
    state.db.delete_mcp_server(&id).map_err(|e| CommandError {
        message: format!("Failed to delete MCP server: {}", e)
    })
}

#[command]
pub async fn connect_mcp_server(
    state: State<'_, Arc<AppState>>,
    id: String,
) -> Result<(), CommandError> {
    // Get server config from database
    let config = match state.db.get_mcp_server(&id).map_err(|e| CommandError {
        message: format!("Failed to get server config: {}", e)
    })? {
        Some(config) => config,
        None => return Err(CommandError {
            message: "MCP server not found".to_string()
        }),
    };

    // Connect using MCP manager
    state.mcp_manager.connect_server(&config).await.map_err(|e| CommandError {
        message: format!("Failed to connect to MCP server: {}", e)
    })?;

    // Update enabled status in database
    state.db.update_mcp_server_enabled(&id, true).map_err(|e| CommandError {
        message: format!("Failed to update server status: {}", e)
    })
}

#[command]
pub async fn disconnect_mcp_server(
    state: State<'_, Arc<AppState>>,
    id: String,
) -> Result<(), CommandError> {
    // Disconnect using MCP manager
    state.mcp_manager.disconnect_server(&id).await;

    // Update enabled status in database
    state.db.update_mcp_server_enabled(&id, false).map_err(|e| CommandError {
        message: format!("Failed to update server status: {}", e)
    })
}

#[command]
pub async fn get_mcp_server_statuses(
    state: State<'_, Arc<AppState>>,
) -> Result<Vec<MCPServerStatus>, CommandError> {
    Ok(state.mcp_manager.get_server_statuses().await)
}

#[command]
pub async fn execute_mcp_tool(
    state: State<'_, Arc<AppState>>,
    call: MCPToolCall,
) -> Result<MCPToolResult, CommandError> {
    Ok(state.mcp_manager.execute_tool(&call).await)
}
