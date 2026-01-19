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
    use crate::llm_client::{LLMClient, Message};

    let settings = state.db.get_settings()?;

    // Debug logging
    println!("[test_connection] model: {}", settings.model);
    println!("[test_connection] base_url: {}", settings.base_url);
    println!("[test_connection] api_key length: {}", settings.api_key.len());
    println!("[test_connection] provider: {}", settings.get_provider());
    println!("[test_connection] is_local_provider: {}, allows_empty_api_key: {}",
        settings.is_local_provider(), settings.allows_empty_api_key());

    if settings.api_key.is_empty() && !settings.allows_empty_api_key() {
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
            "openai" => {
                // OpenAI - test with actual API request using LLMClient
                let llm_client = LLMClient::new_with_openai_headers(
                    settings.api_key.clone(),
                    Some(settings.base_url.clone()),
                    Some("openai"),
                    Some(&settings.model),
                    settings.openai_organization.clone(),
                    settings.openai_project.clone(),
                );

                let test_messages = vec![Message {
                    role: "user".to_string(),
                    content: "Hi".to_string(),
                }];

                // Send a minimal test request
                match llm_client.send_message(test_messages, &settings.model, 10, None).await {
                    Ok(_) => Ok("success".to_string()),
                    Err(e) => Ok(format!("Error: {}", e)),
                }
            }
            _ => {
                // Other cloud services - try sending a test message
                let llm_client = LLMClient::new(
                    settings.api_key.clone(),
                    Some(settings.base_url.clone()),
                    None,
                    Some(&settings.model),
                );

                let test_messages = vec![Message {
                    role: "user".to_string(),
                    content: "Hi".to_string(),
                }];

                // Try to send a minimal test request
                match llm_client.send_message(test_messages, &settings.model, 10, None).await {
                    Ok(_) => Ok("success".to_string()),
                    Err(e) => {
                        // If sending fails, try simple connection check (for services that support it)
                        match llm_client.check_connection().await {
                            Ok(true) => Ok("success".to_string()),
                            Ok(false) => Ok(format!("Error: {}", e)),
                            Err(conn_e) => Ok(format!("Error: {}", conn_e)),
                        }
                    }
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
    use crate::llm_client::{LLMClient, Message as LLMMessage};

    let settings = state.db.get_settings()?;

    if settings.api_key.is_empty() && !settings.allows_empty_api_key() {
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

    // Create channel for streaming
    let (tx, mut rx) = tokio::sync::mpsc::channel::<String>(100);

    // Spawn task to emit events
    let window_clone = window.clone();
    let emit_task = tokio::spawn(async move {
        while let Some(text) = rx.recv().await {
            let _ = window_clone.emit("chat-stream", StreamPayload { text, done: false });
        }
    });

    // Choose client based on provider
    let provider = settings.get_provider();
    let response = match provider.as_str() {
        "anthropic" => {
            // Use ClaudeClient for Anthropic
            let claude_messages: Vec<ClaudeMessage> = db_messages
                .iter()
                .map(|m| ClaudeMessage {
                    role: m.role.clone(),
                    content: m.content.clone(),
                })
                .collect();
            let client = ClaudeClient::new(settings.api_key, Some(settings.base_url));
            client
                .send_message_stream(
                    claude_messages,
                    &settings.model,
                    settings.max_tokens,
                    Some(settings.temperature),
                    tx,
                )
                .await?
        }
        _ => {
            // Use LLMClient for OpenAI and other providers
            let llm_messages: Vec<LLMMessage> = db_messages
                .iter()
                .map(|m| LLMMessage {
                    role: m.role.clone(),
                    content: m.content.clone(),
                })
                .collect();
            let llm_client = LLMClient::new_with_openai_headers(
                settings.api_key.clone(),
                Some(settings.base_url.clone()),
                Some(&provider),
                Some(&settings.model),
                settings.openai_organization.clone(),
                settings.openai_project.clone(),
            );
            llm_client
                .send_message_stream(
                    llm_messages,
                    &settings.model,
                    settings.max_tokens,
                    Some(settings.temperature),
                    tx,
                )
                .await
                .map_err(|e| CommandError { message: e.to_string() })?
        }
    };

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
    if settings.api_key.is_empty() && !settings.allows_empty_api_key() {
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

    if settings.api_key.is_empty() && !settings.allows_empty_api_key() {
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
        use crate::llm_client::{LLMClient, Message as LLMMessage};

        let provider = settings.get_provider();
        let (tx, mut rx) = tokio::sync::mpsc::channel::<String>(100);

        let window_clone = window.clone();
        let emit_task = tokio::spawn(async move {
            while let Some(text) = rx.recv().await {
                let _ = window_clone.emit("chat-event", ChatEvent::Text { content: text });
            }
        });

        let response = match provider.as_str() {
            "anthropic" => {
                // Use ClaudeClient for Anthropic
                let claude_messages: Vec<ClaudeMessage> = db_messages
                    .iter()
                    .map(|m| ClaudeMessage {
                        role: m.role.clone(),
                        content: m.content.clone(),
                    })
                    .collect();
                let client = ClaudeClient::new(settings.api_key.clone(), Some(settings.base_url.clone()));
                client
                    .send_message_stream(
                        claude_messages,
                        &settings.model,
                        settings.max_tokens,
                        Some(settings.temperature),
                        tx,
                    )
                    .await?
            }
            _ => {
                // Use LLMClient for OpenAI and other providers
                let llm_messages: Vec<LLMMessage> = db_messages
                    .iter()
                    .map(|m| LLMMessage {
                        role: m.role.clone(),
                        content: m.content.clone(),
                    })
                    .collect();
                let llm_client = LLMClient::new_with_openai_headers(
                    settings.api_key.clone(),
                    Some(settings.base_url.clone()),
                    Some(&provider),
                    Some(&settings.model),
                    settings.openai_organization.clone(),
                    settings.openai_project.clone(),
                );
                llm_client
                    .send_message_stream(
                        llm_messages,
                        &settings.model,
                        settings.max_tokens,
                        Some(settings.temperature),
                        tx,
                    )
                    .await
                    .map_err(|e| CommandError { message: e.to_string() })?
            }
        };

        let _ = emit_task.await;
        let _ = window.emit("chat-event", ChatEvent::Done { final_text: response.clone() });

        // Save assistant response
        let assistant_msg_id = uuid::Uuid::new_v4().to_string();
        state
            .db
            .add_message(&assistant_msg_id, &request.conversation_id, "assistant", &response)?;

        return Ok(response);
    }

    // Enhanced chat with tools - use AgentLoop which supports multiple providers
    use crate::llm_client::ProviderConfig;

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

    // Get provider config for determining API format
    let provider_id = settings.get_provider();
    let mut provider_config = ProviderConfig::from_preset(&provider_id);
    if !settings.base_url.is_empty() {
        provider_config.base_url = settings.base_url.clone();
    }

    // Check if we should use OpenAI format
    let use_openai_format = matches!(
        provider_config.api_format,
        crate::llm_client::ApiFormat::OpenAI | crate::llm_client::ApiFormat::OpenAICompatible
    );

    loop {
        turn += 1;
        if turn > max_turns {
            break;
        }

        // Build and send request
        let api_request = message_builder.build_request(&agent_messages).await;

        let response = if use_openai_format {
            // OpenAI format request
            let openai_request = convert_to_openai_format(&api_request, &settings.model);
            let base = provider_config.base_url.trim_end_matches('/');
            let url = if base.ends_with("/v1") {
                format!("{}/chat/completions", base)
            } else {
                format!("{}/v1/chat/completions", base)
            };

            let mut req = client.post(&url)
                .header("Content-Type", "application/json");

            if !settings.api_key.is_empty() {
                req = req.header("Authorization", format!("Bearer {}", settings.api_key));
            }
            // Add optional OpenAI headers
            if let Some(ref org) = settings.openai_organization {
                if !org.is_empty() {
                    req = req.header("OpenAI-Organization", org);
                }
            }
            if let Some(ref proj) = settings.openai_project {
                if !proj.is_empty() {
                    req = req.header("OpenAI-Project", proj);
                }
            }

            req.json(&openai_request)
                .send()
                .await
                .map_err(|e| CommandError { message: format!("HTTP error: {}", e) })?
        } else {
            // Anthropic format request
            client
                .post(format!("{}/v1/messages", provider_config.base_url.trim_end_matches('/')))
                .header("Content-Type", "application/json")
                .header("x-api-key", &settings.api_key)
                .header("anthropic-version", "2023-06-01")
                .json(&api_request)
                .send()
                .await
                .map_err(|e| CommandError { message: format!("HTTP error: {}", e) })?
        };

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(CommandError { message: format!("API error: {}", error_text) });
        }

        // Handle streaming response based on provider format
        let mut stream = response.bytes_stream();
        let mut buffer = String::new();
        let mut accumulated_text = String::new();
        let mut tool_uses: Vec<ToolUse> = Vec::new();

        if use_openai_format {
            // OpenAI streaming format
            let mut current_tool_calls: std::collections::HashMap<i64, (String, String, String)> = std::collections::HashMap::new();

            while let Some(chunk) = stream.next().await {
                let chunk = chunk.map_err(|e| CommandError { message: format!("Stream error: {}", e) })?;
                buffer.push_str(&String::from_utf8_lossy(&chunk));

                while let Some(pos) = buffer.find('\n') {
                    let line = buffer[..pos].to_string();
                    buffer = buffer[pos + 1..].to_string();

                    if let Some(data) = line.strip_prefix("data: ") {
                        if data.trim() == "[DONE]" {
                            continue;
                        }

                        if let Ok(event) = serde_json::from_str::<serde_json::Value>(data) {
                            if let Some(choices) = event.get("choices").and_then(|v| v.as_array()) {
                                for choice in choices {
                                    if let Some(delta) = choice.get("delta") {
                                        // Handle text content
                                        if let Some(content) = delta.get("content").and_then(|v| v.as_str()) {
                                            accumulated_text.push_str(content);
                                            let _ = window.emit("chat-event", ChatEvent::Text {
                                                content: accumulated_text.clone(),
                                            });
                                        }

                                        // Handle tool_calls
                                        if let Some(tcs) = delta.get("tool_calls").and_then(|v| v.as_array()) {
                                            for tc in tcs {
                                                let index = tc.get("index").and_then(|v| v.as_i64()).unwrap_or(0);

                                                let entry = current_tool_calls.entry(index).or_insert_with(|| {
                                                    (String::new(), String::new(), String::new())
                                                });

                                                if let Some(id) = tc.get("id").and_then(|v| v.as_str()) {
                                                    entry.0 = id.to_string();
                                                }
                                                if let Some(func) = tc.get("function") {
                                                    if let Some(name) = func.get("name").and_then(|v| v.as_str()) {
                                                        entry.1 = name.to_string();
                                                    }
                                                    if let Some(args) = func.get("arguments").and_then(|v| v.as_str()) {
                                                        entry.2.push_str(args);
                                                    }
                                                }
                                            }
                                        }
                                    }

                                    // Check if finished
                                    if choice.get("finish_reason").and_then(|v| v.as_str()).is_some() {
                                        // Convert collected tool_calls to ToolUse
                                        for (_idx, (id, name, args)) in &current_tool_calls {
                                            if !id.is_empty() && !name.is_empty() {
                                                let input: serde_json::Value = serde_json::from_str(args)
                                                    .unwrap_or(serde_json::json!({}));

                                                tool_uses.push(ToolUse {
                                                    id: id.clone(),
                                                    name: name.clone(),
                                                    input: input.clone(),
                                                });

                                                // Emit tool start
                                                let _ = window.emit("chat-event", ChatEvent::ToolStart {
                                                    tool: name.clone(),
                                                    input,
                                                });
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        } else {
            // Anthropic streaming format
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
    if settings.api_key.is_empty() && !settings.allows_empty_api_key() {
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

/// Convert Claude API request format to OpenAI format
fn convert_to_openai_format(
    request: &crate::agent::message_builder::ClaudeApiRequest,
    model: &str,
) -> serde_json::Value {
    use crate::agent::message_builder::ApiContent;

    // Build messages, including system prompt
    let mut messages: Vec<serde_json::Value> = Vec::new();

    // Add system message
    if !request.system.is_empty() {
        messages.push(serde_json::json!({
            "role": "system",
            "content": request.system
        }));
    }

    // Convert conversation messages
    for msg in &request.messages {
        let role = &msg.role;

        match &msg.content {
            ApiContent::Text(text) => {
                messages.push(serde_json::json!({
                    "role": role,
                    "content": text
                }));
            }
            ApiContent::Blocks(blocks) => {
                // Handle content blocks (text, tool_use, tool_result)
                let mut text_parts: Vec<String> = Vec::new();
                let mut tool_calls: Vec<serde_json::Value> = Vec::new();

                for block in blocks {
                    let block_type = block.get("type").and_then(|v| v.as_str()).unwrap_or("");

                    match block_type {
                        "text" => {
                            if let Some(text) = block.get("text").and_then(|v| v.as_str()) {
                                text_parts.push(text.to_string());
                            }
                        }
                        "tool_use" => {
                            tool_calls.push(serde_json::json!({
                                "id": block.get("id"),
                                "type": "function",
                                "function": {
                                    "name": block.get("name"),
                                    "arguments": serde_json::to_string(block.get("input").unwrap_or(&serde_json::json!({}))).unwrap_or_default()
                                }
                            }));
                        }
                        "tool_result" => {
                            // OpenAI uses tool role to represent tool results
                            messages.push(serde_json::json!({
                                "role": "tool",
                                "tool_call_id": block.get("tool_use_id"),
                                "content": block.get("content")
                            }));
                        }
                        _ => {}
                    }
                }

                // If there's text content
                if !text_parts.is_empty() {
                    let mut msg_obj = serde_json::json!({
                        "role": role,
                        "content": text_parts.join("\n")
                    });

                    // If there are tool_calls
                    if !tool_calls.is_empty() {
                        msg_obj["tool_calls"] = serde_json::json!(tool_calls);
                    }

                    messages.push(msg_obj);
                } else if !tool_calls.is_empty() {
                    // Only tool_calls, no text
                    messages.push(serde_json::json!({
                        "role": role,
                        "content": serde_json::Value::Null,
                        "tool_calls": tool_calls
                    }));
                }
            }
        }
    }

    // Convert tools definition
    let tools: Vec<serde_json::Value> = request.tools.iter().map(|tool| {
        serde_json::json!({
            "type": "function",
            "function": {
                "name": tool.name,
                "description": tool.description,
                "parameters": tool.input_schema
            }
        })
    }).collect();

    let mut openai_request = serde_json::json!({
        "model": request.model,
        "stream": request.stream,
        "messages": messages
    });

    // Use correct max tokens parameter based on model
    let model_lower = model.to_lowercase();
    let is_legacy = model_lower.contains("gpt-3.5")
        || (model_lower.contains("gpt-4") && !model_lower.contains("gpt-4o") && !model_lower.contains("gpt-4-turbo"));

    if is_legacy {
        openai_request["max_tokens"] = serde_json::json!(request.max_tokens);
    } else {
        openai_request["max_completion_tokens"] = serde_json::json!(request.max_tokens);
    }

    // Only add temperature for non-reasoning models (o1, o3, gpt-5 don't support custom temperature)
    let is_reasoning = model_lower.starts_with("o1") || model_lower.starts_with("o3") || model_lower.starts_with("gpt-5")
        || model_lower.contains("-o1") || model_lower.contains("-o3")
        || model_lower.contains("o1-") || model_lower.contains("o3-");

    if !is_reasoning {
        if let Some(temp) = request.temperature {
            openai_request["temperature"] = serde_json::json!(temp);
        }
    }

    if !tools.is_empty() {
        openai_request["tools"] = serde_json::json!(tools);
        openai_request["tool_choice"] = serde_json::json!("auto");
    }

    openai_request
}
