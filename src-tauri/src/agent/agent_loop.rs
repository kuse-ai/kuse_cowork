use crate::agent::{
    AgentConfig, AgentContent, AgentEvent, AgentMessage, ContentBlock, MessageBuilder,
    PlanStepInfo, ToolExecutor, ToolUse,
};
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
    #[allow(dead_code)]
    model: String,
    #[allow(dead_code)]
    max_tokens: u32,
    #[allow(dead_code)]
    temperature: Option<f32>,
    tool_executor: ToolExecutor,
    message_builder: MessageBuilder,
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
        let tool_executor = ToolExecutor::new(config.project_path.clone())
            .with_mcp_manager(mcp_manager.clone());
        let message_builder = MessageBuilder::new(
            config.clone(),
            model.clone(),
            max_tokens,
            temperature,
        ).with_mcp_manager(mcp_manager);

        Self {
            client: Client::new(),
            api_key,
            base_url,
            config,
            model,
            max_tokens,
            temperature,
            tool_executor,
            message_builder,
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
        let response = self
            .client
            .post(format!("{}/v1/messages", self.base_url))
            .header("Content-Type", "application/json")
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .json(request)
            .send()
            .await
            .map_err(|e| format!("HTTP error: {}", e))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(format!("API error: {}", error_text));
        }

        // Handle streaming response

        self.handle_stream_response(response, event_tx).await
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
