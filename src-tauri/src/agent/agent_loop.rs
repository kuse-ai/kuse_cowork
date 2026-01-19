use crate::agent::{
    AgentConfig, AgentContent, AgentEvent, AgentMessage, ContentBlock, MessageBuilder,
    PlanStepInfo, ToolExecutor, ToolUse,
};
use crate::llm_client::{ApiFormat, ProviderConfig};
use crate::mcp::MCPManager;
use regex::Regex;
use reqwest::Client;
use std::sync::Arc;
use tokio::sync::mpsc;

pub struct AgentLoop {
    client: Client,
    api_key: String,
    base_url: String,
    config: AgentConfig,
    model: String,
    max_tokens: u32,
    temperature: Option<f32>,
    tool_executor: ToolExecutor,
    message_builder: MessageBuilder,
    /// Provider configuration for determining API format
    provider_config: ProviderConfig,
}

impl AgentLoop {
    pub fn new(
        api_key: String,
        base_url: String,
        config: AgentConfig,
        model: String,
        max_tokens: u32,
        temperature: Option<f32>,
        mcp_manager: Arc<MCPManager>,
    ) -> Self {
        Self::new_with_provider(api_key, base_url, config, model, max_tokens, temperature, mcp_manager, None)
    }

    pub fn new_with_provider(
        api_key: String,
        base_url: String,
        config: AgentConfig,
        model: String,
        max_tokens: u32,
        temperature: Option<f32>,
        mcp_manager: Arc<MCPManager>,
        provider_id: Option<&str>,
    ) -> Self {
        let tool_executor = ToolExecutor::new(config.project_path.clone())
            .with_mcp_manager(mcp_manager.clone());
        let message_builder = MessageBuilder::new(
            config.clone(),
            model.clone(),
            max_tokens,
            temperature,
        ).with_mcp_manager(mcp_manager);

        // Infer config from provider_id or model
        let mut provider_config = if let Some(pid) = provider_id {
            ProviderConfig::from_preset(pid)
        } else {
            ProviderConfig::from_model(&model)
        };

        // Use custom base_url
        if !base_url.is_empty() {
            provider_config.base_url = base_url.clone();
        }

        Self {
            client: Client::new(),
            api_key,
            base_url: provider_config.base_url.clone(),
            config,
            model,
            max_tokens,
            temperature,
            tool_executor,
            message_builder,
            provider_config,
        }
    }

    pub async fn run(
        &self,
        initial_message: String,
        event_tx: mpsc::Sender<AgentEvent>,
    ) -> Result<Vec<AgentMessage>, String> {
        let messages: Vec<AgentMessage> = vec![AgentMessage {
            role: "user".to_string(),
            content: AgentContent::Text(initial_message),
        }];

        self.run_with_history(messages, event_tx).await
    }

    /// Run agent with existing conversation history
    pub async fn run_with_history(
        &self,
        mut messages: Vec<AgentMessage>,
        event_tx: mpsc::Sender<AgentEvent>,
    ) -> Result<Vec<AgentMessage>, String> {
        let mut turn = 0;

        loop {
            turn += 1;

            if turn > self.config.max_turns {
                let _ = event_tx
                    .send(AgentEvent::Error {
                        message: format!("Reached maximum turns ({})", self.config.max_turns),
                    })
                    .await;
                break;
            }

            // Build and send request
            let request = self.message_builder.build_request(&messages).await;

            let response = self.send_request(&request, &event_tx).await?;

            // Parse response
            let (text_content, tool_uses) = self.parse_response(&response)?;

            // Parse and emit plan if present
            if let Some(plan_steps) = self.parse_plan(&text_content) {
                let _ = event_tx
                    .send(AgentEvent::Plan { steps: plan_steps })
                    .await;
            }

            // Parse and emit step markers
            self.emit_step_markers(&text_content, &event_tx).await;

            // Emit text content
            if !text_content.is_empty() {
                let _ = event_tx
                    .send(AgentEvent::Text {
                        content: text_content.clone(),
                    })
                    .await;
            }

            // Add assistant message
            let assistant_content = if tool_uses.is_empty() {
                AgentContent::Text(text_content)
            } else {
                let mut blocks = Vec::new();
                if !text_content.is_empty() {
                    blocks.push(ContentBlock::Text { text: text_content });
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

            messages.push(AgentMessage {
                role: "assistant".to_string(),
                content: assistant_content,
            });

            // If no tool uses, we're done
            if tool_uses.is_empty() {
                let _ = event_tx
                    .send(AgentEvent::Done { total_turns: turn })
                    .await;
                break;
            }

            // Execute tools
            let mut tool_results = Vec::new();

            for tool_use in &tool_uses {
                // Emit tool start
                let _ = event_tx
                    .send(AgentEvent::ToolStart {
                        tool: tool_use.name.clone(),
                        input: tool_use.input.clone(),
                    })
                    .await;

                // Execute tool
                let result = self.tool_executor.execute(tool_use).await;

                // Emit tool end
                let _ = event_tx
                    .send(AgentEvent::ToolEnd {
                        tool: tool_use.name.clone(),
                        result: result.content.clone(),
                        success: result.is_error.is_none(),
                    })
                    .await;

                tool_results.push(result);
            }

            // Add tool results as user message
            messages.push(AgentMessage {
                role: "user".to_string(),
                content: AgentContent::ToolResults(tool_results),
            });

            // Emit turn complete
            let _ = event_tx.send(AgentEvent::TurnComplete { turn }).await;
        }

        Ok(messages)
    }

    async fn send_request(
        &self,
        request: &crate::agent::message_builder::ClaudeApiRequest,
        event_tx: &mpsc::Sender<AgentEvent>,
    ) -> Result<serde_json::Value, String> {
        match self.provider_config.api_format {
            ApiFormat::Anthropic => self.send_anthropic_request(request, event_tx).await,
            ApiFormat::OpenAI | ApiFormat::OpenAICompatible => {
                self.send_openai_request(request, event_tx).await
            }
            _ => Err(format!("Unsupported API format: {:?}", self.provider_config.api_format)),
        }
    }

    /// Send Anthropic format request
    async fn send_anthropic_request(
        &self,
        request: &crate::agent::message_builder::ClaudeApiRequest,
        event_tx: &mpsc::Sender<AgentEvent>,
    ) -> Result<serde_json::Value, String> {
        let url = format!("{}/v1/messages", self.base_url.trim_end_matches('/'));

        let mut req = self.client.post(&url)
            .header("Content-Type", "application/json")
            .header("anthropic-version", "2023-06-01");

        // Add authentication
        if !self.api_key.is_empty() {
            req = req.header("x-api-key", &self.api_key);
        }

        let response = req
            .json(request)
            .send()
            .await
            .map_err(|e| format!("HTTP error: {}", e))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(format!("API error: {}", error_text));
        }

        self.handle_stream_response(response, event_tx).await
    }

    /// Send OpenAI compatible format request
    async fn send_openai_request(
        &self,
        request: &crate::agent::message_builder::ClaudeApiRequest,
        event_tx: &mpsc::Sender<AgentEvent>,
    ) -> Result<serde_json::Value, String> {
        let base = self.base_url.trim_end_matches('/');
        let url = if base.ends_with("/v1") {
            format!("{}/chat/completions", base)
        } else {
            format!("{}/v1/chat/completions", base)
        };

        // Convert request format to OpenAI format
        let openai_request = self.convert_to_openai_format(request);

        let mut req = self.client.post(&url)
            .header("Content-Type", "application/json");

        // Add authentication (if needed)
        if !self.api_key.is_empty() {
            req = req.header("Authorization", format!("Bearer {}", self.api_key));
        }

        let response = req
            .json(&openai_request)
            .send()
            .await
            .map_err(|e| format!("HTTP error: {}", e))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(format!("API error: {}", error_text));
        }

        self.handle_openai_stream_response(response, event_tx).await
    }

    /// Convert Claude request format to OpenAI format
    fn convert_to_openai_format(
        &self,
        request: &crate::agent::message_builder::ClaudeApiRequest,
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
        let model_lower = request.model.to_lowercase();
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

    /// Handle OpenAI streaming response
    async fn handle_openai_stream_response(
        &self,
        response: reqwest::Response,
        event_tx: &mpsc::Sender<AgentEvent>,
    ) -> Result<serde_json::Value, String> {
        use futures::StreamExt;

        let mut stream = response.bytes_stream();
        let mut buffer = String::new();
        let mut accumulated_text = String::new();
        let mut tool_calls: Vec<serde_json::Value> = Vec::new();
        let mut current_tool_calls: std::collections::HashMap<i64, (String, String, String)> = std::collections::HashMap::new();

        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(|e| format!("Stream error: {}", e))?;
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
                                        let _ = event_tx.send(AgentEvent::Text {
                                            content: accumulated_text.clone(),
                                        }).await;
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
                                    // When finished, convert collected tool_calls to Claude format
                                    for (_idx, (id, name, args)) in &current_tool_calls {
                                        if !id.is_empty() && !name.is_empty() {
                                            let input: serde_json::Value = serde_json::from_str(args)
                                                .unwrap_or(serde_json::json!({}));
                                            tool_calls.push(serde_json::json!({
                                                "type": "tool_use",
                                                "id": id,
                                                "name": name,
                                                "input": input
                                            }));
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // Build Claude format response
        let mut content = Vec::new();
        if !accumulated_text.is_empty() {
            content.push(serde_json::json!({
                "type": "text",
                "text": accumulated_text
            }));
        }
        content.extend(tool_calls);

        Ok(serde_json::json!({
            "content": content
        }))
    }

    async fn handle_stream_response(
        &self,
        response: reqwest::Response,
        event_tx: &mpsc::Sender<AgentEvent>,
    ) -> Result<serde_json::Value, String> {
        use futures::StreamExt;

        let mut stream = response.bytes_stream();
        let mut buffer = String::new();
        let mut full_response: Option<serde_json::Value> = None;
        let mut accumulated_text = String::new();
        let mut tool_uses: Vec<serde_json::Value> = Vec::new();
        let mut current_tool_input = String::new();
        let mut current_tool_id = String::new();
        let mut current_tool_name = String::new();

        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(|e| format!("Stream error: {}", e))?;
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
                                            // Emit streaming text
                                            let _ = event_tx
                                                .send(AgentEvent::Text {
                                                    content: accumulated_text.clone(),
                                                })
                                                .await;
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

                                    tool_uses.push(serde_json::json!({
                                        "type": "tool_use",
                                        "id": current_tool_id,
                                        "name": current_tool_name,
                                        "input": input
                                    }));

                                    current_tool_id.clear();
                                    current_tool_name.clear();
                                    current_tool_input.clear();
                                }
                            }
                            "message_stop" => {
                                // Build final response
                                let mut content = Vec::new();
                                if !accumulated_text.is_empty() {
                                    content.push(serde_json::json!({
                                        "type": "text",
                                        "text": accumulated_text
                                    }));
                                }
                                content.extend(tool_uses.clone());

                                full_response = Some(serde_json::json!({
                                    "content": content
                                }));
                            }
                            _ => {}
                        }
                    }
                }
            }
        }

        full_response.ok_or_else(|| "No response received".to_string())
    }

    fn parse_response(
        &self,
        response: &serde_json::Value,
    ) -> Result<(String, Vec<ToolUse>), String> {
        let content = response
            .get("content")
            .and_then(|v| v.as_array())
            .ok_or("Invalid response: missing content")?;

        let mut text_parts = Vec::new();
        let mut tool_uses = Vec::new();

        for block in content {
            let block_type = block.get("type").and_then(|v| v.as_str()).unwrap_or("");

            match block_type {
                "text" => {
                    if let Some(text) = block.get("text").and_then(|v| v.as_str()) {
                        text_parts.push(text.to_string());
                    }
                }
                "tool_use" => {
                    let id = block
                        .get("id")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    let name = block
                        .get("name")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    let input = block.get("input").cloned().unwrap_or(serde_json::json!({}));

                    tool_uses.push(ToolUse { id, name, input });
                }
                _ => {}
            }
        }

        Ok((text_parts.join(""), tool_uses))
    }

    /// Parse plan from text content
    fn parse_plan(&self, text: &str) -> Option<Vec<PlanStepInfo>> {
        // Look for <plan>...</plan> tags
        let plan_regex = Regex::new(r"(?s)<plan>(.*?)</plan>").ok()?;
        let captures = plan_regex.captures(text)?;
        let plan_content = captures.get(1)?.as_str();

        // Parse numbered steps like "1. Description"
        let step_regex = Regex::new(r"(\d+)\.\s*(.+)").ok()?;
        let mut steps = Vec::new();

        for cap in step_regex.captures_iter(plan_content) {
            if let (Some(num), Some(desc)) = (cap.get(1), cap.get(2)) {
                if let Ok(step_num) = num.as_str().parse::<i32>() {
                    steps.push(PlanStepInfo {
                        step: step_num,
                        description: desc.as_str().trim().to_string(),
                    });
                }
            }
        }

        if steps.is_empty() {
            None
        } else {
            Some(steps)
        }
    }

    /// Emit step start/done markers from text
    async fn emit_step_markers(&self, text: &str, event_tx: &mpsc::Sender<AgentEvent>) {
        // Look for [STEP N START] markers
        let start_regex = Regex::new(r"\[STEP\s*(\d+)\s*START\]").unwrap();
        for cap in start_regex.captures_iter(text) {
            if let Some(num) = cap.get(1) {
                if let Ok(step) = num.as_str().parse::<i32>() {
                    let _ = event_tx.send(AgentEvent::StepStart { step }).await;
                }
            }
        }

        // Look for [STEP N DONE] markers
        let done_regex = Regex::new(r"\[STEP\s*(\d+)\s*DONE\]").unwrap();
        for cap in done_regex.captures_iter(text) {
            if let Some(num) = cap.get(1) {
                if let Ok(step) = num.as_str().parse::<i32>() {
                    let _ = event_tx.send(AgentEvent::StepDone { step }).await;
                }
            }
        }
    }
}

// Make ClaudeApiRequest cloneable for non-stream fallback
impl Clone for crate::agent::message_builder::ClaudeApiRequest {
    fn clone(&self) -> Self {
        Self {
            model: self.model.clone(),
            max_tokens: self.max_tokens,
            system: self.system.clone(),
            messages: self.messages.clone(),
            tools: self.tools.clone(),
            temperature: self.temperature,
            stream: self.stream,
        }
    }
}
