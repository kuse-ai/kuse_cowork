use crate::agent::{AgentConfig, AgentContent, AgentEvent, AgentLoop, AgentMessage};
use crate::claude::{ClaudeClient, Message as ClaudeMessage};
use crate::database::{Conversation, DataPanel, Database, Message, PlanStep, Settings, Task, TaskMessage};
use crate::trace::{Trace, TraceInput, TraceSettings, Suggestion};
use crate::excel::{
    self, ApplyResult, CellEdit, ExcelError, ExcelReadOptions, ExcelReadResult,
    ExcelSchema, ExcelWatcher, FileChangeEvent, ValidationResult, create_event_channel,
};
use crate::mcp::{MCPManager, MCPServerConfig, MCPServerStatus, MCPToolCall, MCPToolResult, MCPAppInstance, MCPResourceResponse, MCPTool};
use crate::skills::{SkillMetadata, get_available_skills};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::{command, Emitter, State, Window};
use tokio::sync::Mutex;

pub struct AppState {
    pub db: Arc<Database>,
    pub claude_client: Mutex<Option<ClaudeClient>>,
    pub mcp_manager: Arc<MCPManager>,
    pub excel_watcher: Mutex<Option<ExcelWatcher>>,
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

impl From<ExcelError> for CommandError {
    fn from(e: ExcelError) -> Self {
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
            "google" => {
                // Google Gemini - test with actual API request
                let llm_client = LLMClient::new(
                    settings.api_key.clone(),
                    Some(settings.base_url.clone()),
                    Some("google"),
                    Some(&settings.model),
                );

                let test_messages = vec![Message {
                    role: "user".to_string(),
                    content: "Hi".to_string(),
                }];

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

    // Determine API format
    let use_openai_format = matches!(
        provider_config.api_format,
        crate::llm_client::ApiFormat::OpenAI | crate::llm_client::ApiFormat::OpenAICompatible
    );
    let use_google_format = matches!(
        provider_config.api_format,
        crate::llm_client::ApiFormat::Google
    );

    // For Google: track thoughtSignature per function call across iterations (required for Gemini 3)
    let mut google_thought_signatures: std::collections::HashMap<String, String> = std::collections::HashMap::new();

    loop {
        turn += 1;
        if turn > max_turns {
            break;
        }

        // Build and send request
        let api_request = message_builder.build_request(&agent_messages).await;

        let response = if use_google_format {
            // Google Gemini format request (pass thought signatures for Gemini 3 function calling)
            let google_request = convert_to_google_format(&api_request, &settings.model, settings.max_tokens, &google_thought_signatures);
            let base = provider_config.base_url.trim_end_matches('/');
            let url = format!("{}/v1beta/models/{}:streamGenerateContent?alt=sse", base, settings.model);

            client.post(&url)
                .header("Content-Type", "application/json")
                .header("x-goog-api-key", &settings.api_key)
                .json(&google_request)
                .send()
                .await
                .map_err(|e| CommandError { message: format!("HTTP error: {}", e) })?
        } else if use_openai_format {
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

        if use_google_format {
            // Google Gemini streaming format (SSE with alt=sse)
            while let Some(chunk) = stream.next().await {
                let chunk = chunk.map_err(|e| CommandError { message: format!("Stream error: {}", e) })?;
                buffer.push_str(&String::from_utf8_lossy(&chunk));

                while let Some(pos) = buffer.find('\n') {
                    let line = buffer[..pos].trim().to_string();
                    buffer = buffer[pos + 1..].to_string();

                    if line.is_empty() {
                        continue;
                    }

                    // Parse SSE data: prefix
                    let json_str = if let Some(data) = line.strip_prefix("data: ") {
                        data
                    } else {
                        continue;
                    };

                    if let Ok(event) = serde_json::from_str::<serde_json::Value>(json_str) {
                        // Extract text and function calls from candidates
                        if let Some(candidates) = event.get("candidates").and_then(|v| v.as_array()) {
                            for candidate in candidates {
                                if let Some(parts) = candidate.get("content")
                                    .and_then(|c| c.get("parts"))
                                    .and_then(|p| p.as_array())
                                {
                                    for part in parts {
                                        // Handle text
                                        if let Some(text) = part.get("text").and_then(|v| v.as_str()) {
                                            if !text.is_empty() {
                                                accumulated_text.push_str(text);
                                                let _ = window.emit("chat-event", ChatEvent::Text {
                                                    content: accumulated_text.clone(),
                                                });
                                            }
                                        }
                                        // Handle function calls (with thoughtSignature for Gemini 3)
                                        if let Some(fc) = part.get("functionCall") {
                                            let name = fc.get("name").and_then(|v| v.as_str()).unwrap_or("").to_string();
                                            let args = fc.get("args").cloned().unwrap_or(serde_json::json!({}));
                                            let id = format!("fc_{}", uuid::Uuid::new_v4());

                                            // Capture thoughtSignature from the same part (required for Gemini 3)
                                            let thought_signature = part.get("thoughtSignature")
                                                .and_then(|v| v.as_str())
                                                .map(|s| s.to_string());

                                            // Also store in map for lookup when building functionResponse
                                            if let Some(ref sig) = thought_signature {
                                                google_thought_signatures.insert(id.clone(), sig.clone());
                                            }

                                            tool_uses.push(ToolUse {
                                                id: id.clone(),
                                                name: name.clone(),
                                                input: args.clone(),
                                                thought_signature,
                                            });

                                            let _ = window.emit("chat-event", ChatEvent::ToolStart {
                                                tool: name,
                                                input: args,
                                            });
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        } else if use_openai_format {
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
                                        for (id, name, args) in current_tool_calls.values() {
                                            if !id.is_empty() && !name.is_empty() {
                                                let input: serde_json::Value = serde_json::from_str(args)
                                                    .unwrap_or(serde_json::json!({}));

                                                tool_uses.push(ToolUse {
                                                    id: id.clone(),
                                                    name: name.clone(),
                                                    input: input.clone(),
                                                    thought_signature: None, // OpenAI doesn't use thought signatures
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
                                            thought_signature: None, // Anthropic doesn't use thought signatures
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
                    thought_signature: tu.thought_signature.clone(),
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

/// Convert Claude API request format to Google Gemini format
fn convert_to_google_format(
    request: &crate::agent::message_builder::ClaudeApiRequest,
    _model: &str,
    max_tokens: u32,
    thought_signatures: &std::collections::HashMap<String, String>,
) -> serde_json::Value {
    use crate::agent::message_builder::ApiContent;

    // Build contents array
    let mut contents: Vec<serde_json::Value> = Vec::new();

    // Convert messages to Google format
    for msg in &request.messages {
        // Google uses "user" and "model" instead of "user" and "assistant"
        let role = if msg.role == "assistant" { "model" } else { &msg.role };

        let parts = match &msg.content {
            ApiContent::Text(text) => {
                vec![serde_json::json!({"text": text})]
            }
            ApiContent::Blocks(blocks) => {
                let mut parts_list = Vec::new();
                for block in blocks {
                    let block_type = block.get("type").and_then(|v| v.as_str()).unwrap_or("");

                    match block_type {
                        "text" => {
                            if let Some(text) = block.get("text").and_then(|v| v.as_str()) {
                                parts_list.push(serde_json::json!({"text": text}));
                            }
                        }
                        "tool_use" => {
                            // Convert to functionCall format with thoughtSignature if present (for Gemini 3)
                            let tool_id = block.get("id").and_then(|v| v.as_str()).unwrap_or("");
                            let mut fc_part = serde_json::json!({
                                "functionCall": {
                                    "name": block.get("name"),
                                    "args": block.get("input")
                                }
                            });
                            // Include thoughtSignature if we have it for this tool
                            if let Some(sig) = thought_signatures.get(tool_id) {
                                fc_part["thoughtSignature"] = serde_json::json!(sig);
                            }
                            parts_list.push(fc_part);
                        }
                        "tool_result" => {
                            // Convert to functionResponse format with thoughtSignature (required for Gemini 3)
                            let tool_use_id = block.get("tool_use_id").and_then(|v| v.as_str()).unwrap_or("unknown");
                            let mut fr_part = serde_json::json!({
                                "functionResponse": {
                                    "name": tool_use_id,
                                    "response": {
                                        "content": block.get("content")
                                    }
                                }
                            });
                            // Include thoughtSignature from matching tool_use (required for Gemini 3)
                            if let Some(sig) = thought_signatures.get(tool_use_id) {
                                fr_part["thoughtSignature"] = serde_json::json!(sig);
                            }
                            parts_list.push(fr_part);
                        }
                        _ => {}
                    }
                }
                parts_list
            }
        };

        if !parts.is_empty() {
            contents.push(serde_json::json!({
                "role": role,
                "parts": parts
            }));
        }
    }

    // Convert tools to Google functionDeclarations format
    let function_declarations: Vec<serde_json::Value> = request.tools.iter().map(|tool| {
        serde_json::json!({
            "name": tool.name,
            "description": tool.description,
            "parameters": tool.input_schema
        })
    }).collect();

    let mut google_request = serde_json::json!({
        "contents": contents,
        "generationConfig": {
            "maxOutputTokens": max_tokens
        }
    });

    // Add system instruction if present
    if !request.system.is_empty() {
        google_request["systemInstruction"] = serde_json::json!({
            "parts": [{"text": request.system}]
        });
    }

    // Add tools if present
    if !function_declarations.is_empty() {
        google_request["tools"] = serde_json::json!([{
            "functionDeclarations": function_declarations
        }]);
    }

    google_request
}

// ==================== Excel Commands ====================

/// Read an Excel file with pagination support
#[command]
pub async fn excel_read(
    _state: State<'_, Arc<AppState>>,
    path: String,
    sheet: Option<String>,
    range: Option<String>,
    offset: Option<u32>,
    max_rows: Option<u32>,
) -> Result<ExcelReadResult, CommandError> {
    let options = ExcelReadOptions {
        path,
        sheet,
        range,
        offset,
        max_rows,
    };

    let result = tokio::task::spawn_blocking(move || {
        excel::read_excel(&options)
    })
    .await
    .map_err(|e| CommandError { message: format!("Task join error: {}", e) })??;

    Ok(result)
}

/// Validate Excel data against a schema
#[command]
pub async fn excel_validate(
    _state: State<'_, Arc<AppState>>,
    path: String,
    sheet: Option<String>,
    range: Option<String>,
    schema: ExcelSchema,
    offset: Option<u32>,
    max_rows: Option<u32>,
) -> Result<ValidationResult, CommandError> {
    // First read the data
    let read_options = ExcelReadOptions {
        path,
        sheet,
        range,
        offset,
        max_rows,
    };

    let read_result = tokio::task::spawn_blocking(move || {
        excel::read_excel(&read_options)
    })
    .await
    .map_err(|e| CommandError { message: format!("Task join error: {}", e) })??;

    // Then validate
    let offset_val = read_result.offset;
    let columns = read_result.columns;
    let rows = read_result.rows;

    let validation_result = excel::validate_schema(&rows, &columns, &schema, offset_val);

    Ok(validation_result)
}

/// Apply edits to an Excel file
#[command]
pub async fn excel_apply(
    window: Window,
    state: State<'_, Arc<AppState>>,
    path: String,
    sheet: String,
    edits: Vec<CellEdit>,
    validate_checksum: Option<String>,
) -> Result<ApplyResult, CommandError> {
    let path_clone = path.clone();
    let sheet_clone = sheet.clone();
    let checksum = validate_checksum.clone();

    let result = tokio::task::spawn_blocking(move || {
        excel::apply_edits(&path_clone, &sheet_clone, &edits, checksum.as_deref())
    })
    .await
    .map_err(|e| CommandError { message: format!("Task join error: {}", e) })??;

    // Emit event for UI update
    let _ = window.emit("excel-edits-applied", &serde_json::json!({
        "path": path,
        "sheet": sheet,
        "edits_applied": result.edits_applied,
        "new_checksum": result.new_checksum,
    }));

    Ok(result)
}

/// Start or stop watching an Excel file for changes
#[command]
pub async fn excel_watch(
    window: Window,
    state: State<'_, Arc<AppState>>,
    path: String,
    enable: bool,
) -> Result<(), CommandError> {
    let mut watcher_guard = state.excel_watcher.lock().await;

    if enable {
        // Create watcher if it doesn't exist
        if watcher_guard.is_none() {
            let (tx, rx) = create_event_channel();
            *watcher_guard = Some(ExcelWatcher::new(tx));

            // Spawn a task to forward events to the frontend
            let window_clone = window.clone();
            tokio::spawn(async move {
                while let Ok(event) = rx.recv() {
                    let _ = window_clone.emit("excel-file-changed", &event);
                }
            });
        }

        if let Some(ref watcher) = *watcher_guard {
            watcher.watch_file(&path)?;
        }
    } else {
        if let Some(ref watcher) = *watcher_guard {
            watcher.unwatch_file(&path)?;
        }
    }

    Ok(())
}

/// Get the list of sheets in an Excel file
#[command]
pub async fn excel_get_sheets(
    _state: State<'_, Arc<AppState>>,
    path: String,
) -> Result<Vec<excel::SheetInfo>, CommandError> {
    let result = tokio::task::spawn_blocking(move || {
        excel::get_sheets(&path)
    })
    .await
    .map_err(|e| CommandError { message: format!("Task join error: {}", e) })??;

    Ok(result)
}

/// Compute checksum of an Excel file
#[command]
pub async fn excel_checksum(
    _state: State<'_, Arc<AppState>>,
    path: String,
) -> Result<String, CommandError> {
    let result = tokio::task::spawn_blocking(move || {
        excel::compute_checksum(&path)
    })
    .await
    .map_err(|e| CommandError { message: format!("Task join error: {}", e) })??;

    Ok(result)
}

/// Create a backup of an Excel file
#[command]
pub async fn excel_backup(
    _state: State<'_, Arc<AppState>>,
    path: String,
) -> Result<String, CommandError> {
    let result = tokio::task::spawn_blocking(move || {
        excel::create_backup(&path)
    })
    .await
    .map_err(|e| CommandError { message: format!("Task join error: {}", e) })??;

    Ok(result)
}

// ==================== Data Panel Commands ====================

/// Get a data panel by provider (singleton pattern)
#[command]
pub async fn get_data_panel(
    state: State<'_, Arc<AppState>>,
    provider: String,
) -> Result<Option<DataPanel>, CommandError> {
    let panel = state.db.get_data_panel_by_provider(&provider)?;
    Ok(panel)
}

/// Save a data panel (upsert - create or update)
#[command]
pub async fn save_data_panel(
    state: State<'_, Arc<AppState>>,
    provider: String,
    config: String,
) -> Result<DataPanel, CommandError> {
    let panel = state.db.upsert_data_panel(&provider, &config)?;
    Ok(panel)
}

/// Delete a data panel
#[command]
pub async fn delete_data_panel(
    state: State<'_, Arc<AppState>>,
    id: String,
) -> Result<(), CommandError> {
    state.db.delete_data_panel(&id)?;
    Ok(())
}

/// List all data panels
#[command]
pub async fn list_data_panels(
    state: State<'_, Arc<AppState>>,
) -> Result<Vec<DataPanel>, CommandError> {
    let panels = state.db.list_data_panels()?;
    Ok(panels)
}

// ==================== MCP Apps Commands ====================

/// Fetch a UI resource from an MCP server (for rendering MCP Apps)
#[command]
pub async fn fetch_mcp_app_resource(
    state: State<'_, Arc<AppState>>,
    server_id: String,
    resource_uri: String,
) -> Result<MCPResourceResponse, CommandError> {
    state.mcp_manager
        .fetch_ui_resource(&server_id, &resource_uri)
        .await
        .map_err(|e| CommandError {
            message: format!("Failed to fetch MCP App resource: {}", e),
        })
}

/// Get all MCP tools that have MCP Apps UI support
#[command]
pub async fn get_mcp_app_tools(
    state: State<'_, Arc<AppState>>,
) -> Result<Vec<MCPTool>, CommandError> {
    Ok(state.mcp_manager.get_app_tools().await)
}

/// Create an MCP App instance for rendering
#[command]
pub async fn create_mcp_app_instance(
    state: State<'_, Arc<AppState>>,
    server_id: String,
    tool_name: String,
    tool_result: serde_json::Value,
) -> Result<MCPAppInstance, CommandError> {
    // Get the tool to find its UI configuration
    let tool = state.mcp_manager
        .get_tool(&server_id, &tool_name)
        .await
        .ok_or_else(|| CommandError {
            message: format!("Tool '{}' not found on server '{}'", tool_name, server_id),
        })?;

    // Check if tool has UI configuration
    let ui_config = tool.meta
        .as_ref()
        .and_then(|m| m.ui.as_ref())
        .ok_or_else(|| CommandError {
            message: format!("Tool '{}' does not support MCP Apps", tool_name),
        })?;

    // Fetch the UI resource
    let resource = state.mcp_manager
        .fetch_ui_resource(&server_id, &ui_config.resource_uri)
        .await
        .map_err(|e| CommandError {
            message: format!("Failed to fetch UI resource: {}", e),
        })?;

    // Get the HTML content
    let html_content = resource.contents
        .first()
        .and_then(|c| c.text.clone())
        .ok_or_else(|| CommandError {
            message: "UI resource did not contain HTML content".to_string(),
        })?;

    // Create the app instance
    let instance = MCPAppInstance {
        id: uuid::Uuid::new_v4().to_string(),
        server_id,
        tool_name,
        html_content,
        tool_result,
        permissions: ui_config.permissions.clone().unwrap_or_default(),
        csp: ui_config.csp.clone(),
    };

    Ok(instance)
}

// ==================== Trace Commands ====================

/// Log a new trace event
#[command]
pub async fn log_trace(
    state: State<'_, Arc<AppState>>,
    input: TraceInput,
) -> Result<Trace, CommandError> {
    state.db.log_trace(&input).map_err(Into::into)
}

/// List traces for a document
#[command]
pub async fn list_traces(
    state: State<'_, Arc<AppState>>,
    doc_id: String,
    limit: Option<u32>,
    before_timestamp: Option<i64>,
) -> Result<Vec<Trace>, CommandError> {
    state.db.list_traces(&doc_id, limit, before_timestamp).map_err(Into::into)
}

/// Delete a specific trace
#[command]
pub async fn delete_trace(
    state: State<'_, Arc<AppState>>,
    id: String,
) -> Result<(), CommandError> {
    state.db.delete_trace(&id).map_err(Into::into)
}

/// Clear all traces for a document
#[command]
pub async fn clear_traces(
    state: State<'_, Arc<AppState>>,
    doc_id: String,
) -> Result<u64, CommandError> {
    state.db.clear_traces(&doc_id).map_err(Into::into)
}

/// Get trace settings for a document
#[command]
pub async fn get_trace_settings(
    state: State<'_, Arc<AppState>>,
    doc_id: String,
) -> Result<TraceSettings, CommandError> {
    state.db.get_trace_settings(&doc_id).map_err(Into::into)
}

/// Save trace settings for a document
#[command]
pub async fn save_trace_settings(
    state: State<'_, Arc<AppState>>,
    doc_id: String,
    settings: TraceSettings,
) -> Result<(), CommandError> {
    state.db.save_trace_settings(&doc_id, &settings).map_err(Into::into)
}

// ==================== Suggestion Commands ====================

/// List suggestions for a document
#[command]
pub async fn list_suggestions(
    state: State<'_, Arc<AppState>>,
    doc_id: String,
    status: Option<String>,
) -> Result<Vec<Suggestion>, CommandError> {
    state.db.list_suggestions(&doc_id, status.as_deref()).map_err(Into::into)
}

/// Update suggestion status (approve/reject)
#[command]
pub async fn update_suggestion_status(
    state: State<'_, Arc<AppState>>,
    id: String,
    status: String,
) -> Result<(), CommandError> {
    state.db.update_suggestion_status(&id, &status).map_err(Into::into)
}

/// Delete a suggestion
#[command]
pub async fn delete_suggestion(
    state: State<'_, Arc<AppState>>,
    id: String,
) -> Result<(), CommandError> {
    state.db.delete_suggestion(&id).map_err(Into::into)
}

/// Generate AI suggestions based on recent trace history
#[command]
pub async fn generate_suggestions(
    state: State<'_, Arc<AppState>>,
    doc_id: String,
) -> Result<Vec<Suggestion>, CommandError> {
    use crate::llm_client::{LLMClient, Message};

    let settings = state.db.get_settings()?;

    if settings.api_key.is_empty() && !settings.allows_empty_api_key() {
        return Err(CommandError {
            message: "API key not configured".to_string(),
        });
    }

    // Get recent traces
    let traces = state.db.list_traces(&doc_id, Some(50), None)?;

    if traces.is_empty() {
        return Ok(Vec::new());
    }

    // Build context from traces
    let trace_summary: Vec<String> = traces
        .iter()
        .map(|t| {
            let payload_str = serde_json::to_string(&t.payload).unwrap_or_default();
            format!(
                "- {} at {}: {}",
                t.event_type,
                t.created_at,
                payload_str.chars().take(100).collect::<String>()
            )
        })
        .collect();

    let prompt = format!(
        r#"Based on the following user activity trace, suggest helpful actions the user might want to take.

Recent activity:
{}

Generate 1-3 suggestions in JSON format. Each suggestion should have:
- suggestion_type: one of "edit", "add_section", "search", "refactor"
- title: short title (under 50 chars)
- description: helpful description (under 150 chars)
- payload: relevant parameters for the action

Respond ONLY with a JSON array of suggestions. Example:
[{{"suggestion_type": "add_section", "title": "Add error handling", "description": "Based on your code edits, you might want to add error handling for edge cases", "payload": {{"section": "error_handling"}}}}]"#,
        trace_summary.join("\n")
    );

    // Create LLM client
    let provider_id = settings.get_provider();
    let llm_client = LLMClient::new(
        settings.api_key.clone(),
        Some(settings.base_url.clone()),
        Some(&provider_id),
        Some(&settings.model),
    );

    let messages = vec![Message {
        role: "user".to_string(),
        content: prompt,
    }];

    // Call LLM
    let response = llm_client
        .send_message(messages, &settings.model, settings.max_tokens, Some(settings.temperature))
        .await
        .map_err(|e| CommandError {
            message: format!("LLM error: {}", e),
        })?;

    // Parse suggestions from response
    let suggestions = parse_suggestions_from_response(&response, &doc_id);

    // Save suggestions to database
    for suggestion in &suggestions {
        let _ = state.db.save_suggestion(&doc_id, suggestion);
    }

    Ok(suggestions)
}

fn parse_suggestions_from_response(response: &str, _doc_id: &str) -> Vec<Suggestion> {
    // Try to find JSON array in response
    let json_start = response.find('[');
    let json_end = response.rfind(']');

    if let (Some(start), Some(end)) = (json_start, json_end) {
        let json_str = &response[start..=end];
        if let Ok(parsed) = serde_json::from_str::<Vec<serde_json::Value>>(json_str) {
            return parsed
                .into_iter()
                .filter_map(|v| {
                    let suggestion_type = v.get("suggestion_type")?.as_str()?.to_string();
                    let title = v.get("title")?.as_str()?.to_string();
                    let description = v.get("description")?.as_str()?.to_string();
                    let payload = v.get("payload").cloned().unwrap_or(serde_json::json!({}));

                    Some(Suggestion {
                        id: uuid::Uuid::new_v4().to_string(),
                        suggestion_type,
                        title,
                        description,
                        payload,
                        status: "pending".to_string(),
                        created_at: chrono::Utc::now().timestamp_millis(),
                    })
                })
                .collect();
        }
    }

    Vec::new()
}

/// Open a URL in a new webview window (bypasses X-Frame-Options)
#[command]
pub async fn open_browser_window(
    app: tauri::AppHandle,
    url: String,
    title: Option<String>,
) -> Result<(), CommandError> {
    use tauri::WebviewWindowBuilder;
    use tauri::WebviewUrl;

    let window_title = title.unwrap_or_else(|| "Browser".to_string());
    let window_label = format!("browser-{}", uuid::Uuid::new_v4().to_string().split('-').next().unwrap_or("window"));

    WebviewWindowBuilder::new(
        &app,
        &window_label,
        WebviewUrl::External(url.parse().map_err(|e| CommandError {
            message: format!("Invalid URL: {}", e),
        })?),
    )
    .title(&window_title)
    .inner_size(1024.0, 768.0)
    .min_inner_size(400.0, 300.0)
    .center()
    .build()
    .map_err(|e| CommandError {
        message: format!("Failed to create browser window: {}", e),
    })?;

    Ok(())
}

/// Create an embedded webview as a child of the main window (true side panel)
#[command]
pub async fn create_embedded_browser(
    app: tauri::AppHandle,
    url: String,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
) -> Result<String, CommandError> {
    use tauri::{Manager, WebviewUrl, WebviewBuilder};

    let webview_label = "embedded-browser";

    println!("create_embedded_browser called: url={}, x={}, y={}, w={}, h={}", url, x, y, width, height);

    // Check if embedded browser already exists
    if let Some(existing) = app.get_webview(webview_label) {
        println!("Embedded browser already exists, navigating...");
        // Navigate existing webview to new URL
        existing.navigate(url.parse().map_err(|e| CommandError {
            message: format!("Invalid URL: {}", e),
        })?).map_err(|e| CommandError {
            message: format!("Failed to navigate: {}", e),
        })?;
        return Ok(webview_label.to_string());
    }

    // Try to get the main window
    // First try get_window, if that fails, try to get from webview_window
    let main_window = if let Some(window) = app.get_window("main") {
        println!("Got main window via get_window");
        window
    } else {
        // List available windows for debugging
        let windows: Vec<_> = app.windows().keys().cloned().collect();
        println!("Available windows: {:?}", windows);

        // Try to find any window
        let first_window = app.windows().values().next().cloned();
        first_window.ok_or_else(|| {
            println!("ERROR: No windows found!");
            CommandError {
                message: "No windows found".to_string(),
            }
        })?
    };

    println!("Got window: {:?}, creating child webview...", main_window.label());

    // Create a child webview embedded in the main window
    let webview_builder = WebviewBuilder::new(
        webview_label,
        WebviewUrl::External(url.parse().map_err(|e| CommandError {
            message: format!("Invalid URL: {}", e),
        })?),
    );

    let webview = main_window.add_child(
        webview_builder,
        tauri::LogicalPosition::new(x, y),
        tauri::LogicalSize::new(width, height),
    ).map_err(|e| {
        println!("ERROR creating child webview: {}", e);
        CommandError {
            message: format!("Failed to create embedded browser: {}", e),
        }
    })?;

    println!("Child webview created successfully: {:?}", webview.label());

    Ok(webview_label.to_string())
}

/// Update the position and size of the embedded browser
#[command]
pub async fn update_embedded_browser_bounds(
    app: tauri::AppHandle,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
) -> Result<(), CommandError> {
    use tauri::Manager;

    let webview = app.get_webview("embedded-browser").ok_or_else(|| CommandError {
        message: "Embedded browser not found".to_string(),
    })?;

    webview.set_position(tauri::LogicalPosition::new(x, y)).map_err(|e| CommandError {
        message: format!("Failed to set position: {}", e),
    })?;

    webview.set_size(tauri::LogicalSize::new(width, height)).map_err(|e| CommandError {
        message: format!("Failed to set size: {}", e),
    })?;

    Ok(())
}

/// Navigate the embedded browser to a new URL
#[command]
pub async fn navigate_embedded_browser(
    app: tauri::AppHandle,
    url: String,
) -> Result<(), CommandError> {
    use tauri::Manager;

    let webview = app.get_webview("embedded-browser").ok_or_else(|| CommandError {
        message: "Embedded browser not found".to_string(),
    })?;

    webview.navigate(url.parse().map_err(|e| CommandError {
        message: format!("Invalid URL: {}", e),
    })?).map_err(|e| CommandError {
        message: format!("Failed to navigate: {}", e),
    })?;

    Ok(())
}

/// Close the embedded browser
#[command]
pub async fn close_embedded_browser(
    app: tauri::AppHandle,
) -> Result<(), CommandError> {
    use tauri::Manager;

    if let Some(webview) = app.get_webview("embedded-browser") {
        webview.close().map_err(|e| CommandError {
            message: format!("Failed to close browser: {}", e),
        })?;
    }

    Ok(())
}

/// Apply an approved suggestion
#[command]
pub async fn apply_suggestion(
    state: State<'_, Arc<AppState>>,
    suggestion_id: String,
) -> Result<serde_json::Value, CommandError> {
    // Get the suggestion
    let suggestions = state.db.list_suggestions("", None)?;
    let suggestion = suggestions
        .into_iter()
        .find(|s| s.id == suggestion_id)
        .ok_or_else(|| CommandError {
            message: "Suggestion not found".to_string(),
        })?;

    // Update status to approved
    state.db.update_suggestion_status(&suggestion_id, "approved")?;

    // Return the suggestion payload for the frontend to handle
    // The actual application logic depends on the suggestion type and will be
    // handled by the frontend or additional backend logic
    Ok(serde_json::json!({
        "applied": true,
        "suggestion_type": suggestion.suggestion_type,
        "payload": suggestion.payload,
    }))
}
