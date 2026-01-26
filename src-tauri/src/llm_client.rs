use reqwest::Client;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::sync::mpsc;

#[derive(Error, Debug)]
#[allow(dead_code)]
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

/// API format type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "kebab-case")]
pub enum ApiFormat {
    #[default]
    Anthropic,
    OpenAI,
    OpenAICompatible,
    OpenAIResponses,  // For GPT-5 series using /v1/responses endpoint
    Google,
    Minimax,
}

/// Authentication type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "kebab-case")]
pub enum AuthType {
    None,
    #[default]
    Bearer,
    ApiKey,
    QueryParam,
}

/// Provider configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    pub id: String,
    pub name: String,
    pub base_url: String,
    pub api_format: ApiFormat,
    pub auth_type: AuthType,
}

impl ProviderConfig {
    /// Get preset configuration by provider ID
    pub fn from_preset(provider_id: &str) -> Self {
        match provider_id {
            // Official APIs
            "anthropic" => Self {
                id: "anthropic".to_string(),
                name: "Anthropic".to_string(),
                base_url: "https://api.anthropic.com".to_string(),
                api_format: ApiFormat::Anthropic,
                auth_type: AuthType::ApiKey,
            },
            "openai" => Self {
                id: "openai".to_string(),
                name: "OpenAI".to_string(),
                base_url: "https://api.openai.com".to_string(),
                api_format: ApiFormat::OpenAI,
                auth_type: AuthType::Bearer,
            },
            "google" => Self {
                id: "google".to_string(),
                name: "Google".to_string(),
                base_url: "https://generativelanguage.googleapis.com".to_string(),
                api_format: ApiFormat::Google,
                auth_type: AuthType::QueryParam,
            },
            "minimax" => Self {
                id: "minimax".to_string(),
                name: "Minimax".to_string(),
                base_url: "https://api.minimax.chat".to_string(),
                api_format: ApiFormat::Minimax,
                auth_type: AuthType::Bearer,
            },

            // Local inference services
            "ollama" => Self {
                id: "ollama".to_string(),
                name: "Ollama".to_string(),
                base_url: "http://localhost:11434".to_string(),
                api_format: ApiFormat::OpenAICompatible,
                auth_type: AuthType::None,
            },
            "lm-studio" => Self {
                id: "lm-studio".to_string(),
                name: "LM Studio".to_string(),
                base_url: "http://localhost:1234".to_string(),
                api_format: ApiFormat::OpenAICompatible,
                auth_type: AuthType::None,
            },
            "localai" => Self {
                id: "localai".to_string(),
                name: "LocalAI".to_string(),
                base_url: "http://localhost:8080".to_string(),
                api_format: ApiFormat::OpenAICompatible,
                auth_type: AuthType::None,
            },

            // Cloud GPU inference
            "vllm" => Self {
                id: "vllm".to_string(),
                name: "vLLM".to_string(),
                base_url: "http://localhost:8000".to_string(),
                api_format: ApiFormat::OpenAICompatible,
                auth_type: AuthType::None,
            },
            "tgi" => Self {
                id: "tgi".to_string(),
                name: "TGI".to_string(),
                base_url: "http://localhost:8080".to_string(),
                api_format: ApiFormat::OpenAICompatible,
                auth_type: AuthType::None,
            },
            "sglang" => Self {
                id: "sglang".to_string(),
                name: "SGLang".to_string(),
                base_url: "http://localhost:30000".to_string(),
                api_format: ApiFormat::OpenAICompatible,
                auth_type: AuthType::None,
            },

            // API aggregation services
            "openrouter" => Self {
                id: "openrouter".to_string(),
                name: "OpenRouter".to_string(),
                base_url: "https://openrouter.ai/api/v1".to_string(),
                api_format: ApiFormat::OpenAICompatible,
                auth_type: AuthType::Bearer,
            },
            "together" => Self {
                id: "together".to_string(),
                name: "Together AI".to_string(),
                base_url: "https://api.together.xyz/v1".to_string(),
                api_format: ApiFormat::OpenAICompatible,
                auth_type: AuthType::Bearer,
            },
            "groq" => Self {
                id: "groq".to_string(),
                name: "Groq".to_string(),
                base_url: "https://api.groq.com/openai/v1".to_string(),
                api_format: ApiFormat::OpenAICompatible,
                auth_type: AuthType::Bearer,
            },
            "deepseek" => Self {
                id: "deepseek".to_string(),
                name: "DeepSeek".to_string(),
                base_url: "https://api.deepseek.com".to_string(),
                api_format: ApiFormat::OpenAICompatible,
                auth_type: AuthType::Bearer,
            },
            "siliconflow" => Self {
                id: "siliconflow".to_string(),
                name: "SiliconFlow".to_string(),
                base_url: "https://api.siliconflow.cn/v1".to_string(),
                api_format: ApiFormat::OpenAICompatible,
                auth_type: AuthType::Bearer,
            },

            // Default/Custom - assume OpenAI compatible
            _ => Self {
                id: provider_id.to_string(),
                name: "Custom".to_string(),
                base_url: "http://localhost:8000".to_string(),
                api_format: ApiFormat::OpenAICompatible,
                auth_type: AuthType::Bearer,
            },
        }
    }

    /// Infer provider from model name
    pub fn from_model(model: &str) -> Self {
        let model_lower = model.to_lowercase();

        // Check OpenRouter format first (contains slash with known prefix)
        if model_lower.starts_with("anthropic/") || model_lower.starts_with("openai/") || model_lower.starts_with("meta-llama/") || model_lower.starts_with("deepseek/") {
            return Self::from_preset("openrouter");
        }

        // Check Ollama format (contains colon, e.g., llama3.3:latest)
        if model_lower.contains(":") {
            return Self::from_preset("ollama");
        }

        // Direct provider detection by model name
        if model_lower.contains("claude") {
            Self::from_preset("anthropic")
        } else if model_lower.starts_with("gpt-5") || model_lower.contains("gpt-5") {
            // GPT-5 series uses Responses API
            Self::from_preset_with_format("openai", ApiFormat::OpenAIResponses)
        } else if model_lower.contains("gpt") {
            Self::from_preset("openai")
        } else if model_lower.contains("gemini") {
            Self::from_preset("google")
        } else if model_lower.contains("minimax") {
            Self::from_preset("minimax")
        } else {
            // Default to Anthropic
            Self::from_preset("anthropic")
        }
    }

    /// Get preset configuration with custom API format override
    fn from_preset_with_format(provider_id: &str, api_format: ApiFormat) -> Self {
        let mut config = Self::from_preset(provider_id);
        config.api_format = api_format;
        config
    }
}

/// Message structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: String,
    pub content: String,
}

/// General LLM client
pub struct LLMClient {
    client: Client,
    api_key: String,
    base_url: String,
    provider_config: ProviderConfig,
    openai_organization: Option<String>,
    openai_project: Option<String>,
}

impl LLMClient {
    pub fn new(api_key: String, base_url: Option<String>, provider_id: Option<&str>, model: Option<&str>) -> Self {
        Self::new_with_openai_headers(api_key, base_url, provider_id, model, None, None)
    }

    pub fn new_with_openai_headers(
        api_key: String,
        base_url: Option<String>,
        provider_id: Option<&str>,
        model: Option<&str>,
        openai_organization: Option<String>,
        openai_project: Option<String>,
    ) -> Self {
        // Infer config from provider_id or model
        let mut config = if let Some(pid) = provider_id {
            ProviderConfig::from_preset(pid)
        } else if let Some(m) = model {
            ProviderConfig::from_model(m)
        } else {
            ProviderConfig::from_preset("anthropic")
        };

        // Override default with custom base_url if provided
        if let Some(url) = base_url {
            config.base_url = url;
        }

        Self {
            client: Client::new(),
            api_key,
            base_url: config.base_url.clone(),
            provider_config: config,
            openai_organization,
            openai_project,
        }
    }

    /// Get API format
    #[allow(dead_code)]
    pub fn api_format(&self) -> &ApiFormat {
        &self.provider_config.api_format
    }

    /// Get API endpoint
    fn get_api_endpoint(&self) -> String {
        let base = self.base_url.trim_end_matches('/');

        match self.provider_config.api_format {
            ApiFormat::Anthropic => format!("{}/v1/messages", base),
            ApiFormat::OpenAI | ApiFormat::OpenAICompatible => {
                if base.ends_with("/v1") {
                    format!("{}/chat/completions", base)
                } else {
                    format!("{}/v1/chat/completions", base)
                }
            }
            ApiFormat::OpenAIResponses => {
                // GPT-5 series uses Responses API endpoint
                if base.ends_with("/v1") {
                    format!("{}/responses", base)
                } else {
                    format!("{}/v1/responses", base)
                }
            }
            ApiFormat::Google => format!("{}/v1beta/models", base),
            ApiFormat::Minimax => format!("{}/v1/text/chatcompletion_v2", base),
        }
    }

    /// Build request headers
    fn build_headers(&self) -> Vec<(String, String)> {
        let mut headers = vec![
            ("Content-Type".to_string(), "application/json".to_string()),
        ];

        match self.provider_config.auth_type {
            AuthType::None => {
                // No auth required
            }
            AuthType::Bearer => {
                if !self.api_key.is_empty() {
                    headers.push(("Authorization".to_string(), format!("Bearer {}", self.api_key)));
                }
            }
            AuthType::ApiKey => {
                if !self.api_key.is_empty() {
                    headers.push(("x-api-key".to_string(), self.api_key.clone()));
                }
                // Anthropic specific
                if self.provider_config.id == "anthropic" {
                    headers.push(("anthropic-version".to_string(), "2023-06-01".to_string()));
                }
            }
            AuthType::QueryParam => {
                // Query param handled in URL
            }
        }

        // Add optional OpenAI organization and project headers
        if let Some(ref org) = self.openai_organization {
            if !org.is_empty() {
                headers.push(("OpenAI-Organization".to_string(), org.clone()));
            }
        }
        if let Some(ref project) = self.openai_project {
            if !project.is_empty() {
                headers.push(("OpenAI-Project".to_string(), project.clone()));
            }
        }

        headers
    }

    /// Send non-streaming message
    pub async fn send_message(
        &self,
        messages: Vec<Message>,
        model: &str,
        max_tokens: u32,
        temperature: Option<f32>,
    ) -> Result<String, LLMError> {
        match self.provider_config.api_format {
            ApiFormat::Anthropic => self.send_anthropic(messages, model, max_tokens, temperature, false, None).await,
            ApiFormat::OpenAI | ApiFormat::OpenAICompatible => self.send_openai_compatible(messages, model, max_tokens, temperature, false, None).await,
            ApiFormat::OpenAIResponses => self.send_openai_responses(messages, model, max_tokens, temperature, false, None).await,
            ApiFormat::Google => self.send_google(messages, model, max_tokens, temperature, false, None).await,
            _ => Err(LLMError::UnsupportedProvider(format!("{:?}", self.provider_config.api_format))),
        }
    }

    /// Send streaming message
    pub async fn send_message_stream(
        &self,
        messages: Vec<Message>,
        model: &str,
        max_tokens: u32,
        temperature: Option<f32>,
        tx: mpsc::Sender<String>,
    ) -> Result<String, LLMError> {
        match self.provider_config.api_format {
            ApiFormat::Anthropic => self.send_anthropic(messages, model, max_tokens, temperature, true, Some(tx)).await,
            ApiFormat::OpenAI | ApiFormat::OpenAICompatible => self.send_openai_compatible(messages, model, max_tokens, temperature, true, Some(tx)).await,
            ApiFormat::OpenAIResponses => self.send_openai_responses(messages, model, max_tokens, temperature, true, Some(tx)).await,
            ApiFormat::Google => self.send_google(messages, model, max_tokens, temperature, true, Some(tx)).await,
            _ => Err(LLMError::UnsupportedProvider(format!("{:?}", self.provider_config.api_format))),
        }
    }

    /// Anthropic API call
    async fn send_anthropic(
        &self,
        messages: Vec<Message>,
        model: &str,
        max_tokens: u32,
        temperature: Option<f32>,
        stream: bool,
        tx: Option<mpsc::Sender<String>>,
    ) -> Result<String, LLMError> {
        let url = self.get_api_endpoint();
        let headers = self.build_headers();

        let payload = serde_json::json!({
            "model": model,
            "max_tokens": max_tokens,
            "messages": messages,
            "stream": stream,
            "temperature": temperature,
        });

        let mut request = self.client.post(&url);
        for (key, value) in headers {
            request = request.header(&key, &value);
        }

        let response = request.json(&payload).send().await?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(LLMError::Api(error_text));
        }

        if stream {
            self.handle_anthropic_stream(response, tx.unwrap()).await
        } else {
            let data: serde_json::Value = response.json().await?;
            let text = data["content"]
                .as_array()
                .and_then(|arr| arr.first())
                .and_then(|block| block["text"].as_str())
                .unwrap_or("")
                .to_string();
            Ok(text)
        }
    }

    /// Check if model is a reasoning model (o1, o3, gpt-5) that doesn't support custom temperature
    /// These models only support temperature=1 (default)
    fn is_reasoning_model(model: &str) -> bool {
        let lower = model.to_lowercase();
        // o1, o1-mini, o1-preview, o3, o3-mini, gpt-5, gpt-5-mini, gpt-5-nano, etc.
        lower.starts_with("o1") || lower.starts_with("o3") || lower.starts_with("gpt-5")
            || lower.contains("-o1") || lower.contains("-o3")
            || lower.contains("o1-") || lower.contains("o3-")
    }

    /// Check if model supports custom temperature (for OpenAI, some models only support temperature=1)
    fn supports_custom_temperature(model: &str) -> bool {
        !Self::is_reasoning_model(model)
    }

    /// Check if model is a legacy model that uses max_tokens instead of max_completion_tokens
    fn is_legacy_openai_model(model: &str) -> bool {
        let lower = model.to_lowercase();
        lower.contains("gpt-3.5") || (lower.contains("gpt-4") && !lower.contains("gpt-4o") && !lower.contains("gpt-4-turbo"))
    }

    /// OpenAI Compatible API call
    async fn send_openai_compatible(
        &self,
        messages: Vec<Message>,
        model: &str,
        max_tokens: u32,
        temperature: Option<f32>,
        stream: bool,
        tx: Option<mpsc::Sender<String>>,
    ) -> Result<String, LLMError> {
        let url = self.get_api_endpoint();
        let headers = self.build_headers();

        // Build payload based on model type
        let mut payload = serde_json::json!({
            "model": model,
            "messages": messages,
            "stream": stream,
        });

        // Add max tokens with correct parameter name
        if self.provider_config.api_format == ApiFormat::OpenAI {
            if Self::is_legacy_openai_model(model) {
                payload["max_tokens"] = serde_json::json!(max_tokens);
            } else {
                payload["max_completion_tokens"] = serde_json::json!(max_tokens);
            }
            // Only add temperature if explicitly specified AND model supports it
            if let Some(temp) = temperature {
                if Self::supports_custom_temperature(model) {
                    payload["temperature"] = serde_json::json!(temp);
                }
                // For reasoning models, don't send temperature at all (uses default of 1)
            }
        } else {
            // Other OpenAI-compatible APIs use max_tokens and always support temperature
            payload["max_tokens"] = serde_json::json!(max_tokens);
            if let Some(temp) = temperature {
                payload["temperature"] = serde_json::json!(temp);
            }
        };

        let mut request = self.client.post(&url);
        for (key, value) in headers {
            request = request.header(&key, &value);
        }

        let response = request.json(&payload).send().await?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(LLMError::Api(error_text));
        }

        if stream {
            self.handle_openai_stream(response, tx.unwrap()).await
        } else {
            let data: serde_json::Value = response.json().await?;
            let text = data["choices"]
                .as_array()
                .and_then(|arr| arr.first())
                .and_then(|choice| choice["message"]["content"].as_str())
                .unwrap_or("")
                .to_string();
            Ok(text)
        }
    }

    /// Handle Anthropic streaming response
    async fn handle_anthropic_stream(
        &self,
        response: reqwest::Response,
        tx: mpsc::Sender<String>,
    ) -> Result<String, LLMError> {
        use futures::StreamExt;

        let mut stream = response.bytes_stream();
        let mut buffer = String::new();
        let mut full_text = String::new();

        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;
            buffer.push_str(&String::from_utf8_lossy(&chunk));

            while let Some(pos) = buffer.find('\n') {
                let line = buffer[..pos].to_string();
                buffer = buffer[pos + 1..].to_string();

                if let Some(data) = line.strip_prefix("data: ") {
                    if data == "[DONE]" {
                        continue;
                    }

                    if let Ok(event) = serde_json::from_str::<serde_json::Value>(data) {
                        if event["type"].as_str() == Some("content_block_delta") {
                            if let Some(text) = event["delta"]["text"].as_str() {
                                full_text.push_str(text);
                                let _ = tx.send(full_text.clone()).await;
                            }
                        }
                    }
                }
            }
        }

        Ok(full_text)
    }

    /// Handle OpenAI streaming response
    async fn handle_openai_stream(
        &self,
        response: reqwest::Response,
        tx: mpsc::Sender<String>,
    ) -> Result<String, LLMError> {
        use futures::StreamExt;

        let mut stream = response.bytes_stream();
        let mut buffer = String::new();
        let mut full_text = String::new();

        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;
            buffer.push_str(&String::from_utf8_lossy(&chunk));

            while let Some(pos) = buffer.find('\n') {
                let line = buffer[..pos].to_string();
                buffer = buffer[pos + 1..].to_string();

                if let Some(data) = line.strip_prefix("data: ") {
                    if data == "[DONE]" {
                        continue;
                    }

                    if let Ok(event) = serde_json::from_str::<serde_json::Value>(data) {
                        if let Some(delta) = event["choices"]
                            .as_array()
                            .and_then(|arr| arr.first())
                            .and_then(|choice| choice["delta"]["content"].as_str())
                        {
                            full_text.push_str(delta);
                            let _ = tx.send(full_text.clone()).await;
                        }
                    }
                }
            }
        }

        Ok(full_text)
    }

    /// OpenAI Responses API call (for GPT-5 series)
    async fn send_openai_responses(
        &self,
        messages: Vec<Message>,
        model: &str,
        max_tokens: u32,
        temperature: Option<f32>,
        stream: bool,
        tx: Option<mpsc::Sender<String>>,
    ) -> Result<String, LLMError> {
        let url = self.get_api_endpoint();
        let headers = self.build_headers();

        // Extract system message as instructions
        let (instructions, input_messages) = self.extract_instructions(messages);

        // Build request payload for Responses API
        let mut payload = serde_json::json!({
            "model": model,
            "input": input_messages.iter().map(|m| {
                serde_json::json!({"role": m.role, "content": m.content})
            }).collect::<Vec<_>>(),
            "max_output_tokens": max_tokens,
            "temperature": temperature.unwrap_or(1.0),
            "stream": stream
        });

        // Add instructions if present
        if let Some(instr) = instructions {
            payload["instructions"] = serde_json::json!(instr);
        }

        let mut request = self.client.post(&url);
        for (key, value) in headers {
            request = request.header(&key, &value);
        }

        let response = request.json(&payload).send().await?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(LLMError::Api(error_text));
        }

        if stream {
            self.handle_responses_stream(response, tx.unwrap()).await
        } else {
            let data: serde_json::Value = response.json().await?;
            Ok(Self::parse_responses_response(&data).unwrap_or_default())
        }
    }

    /// Extract system message as instructions for Responses API
    fn extract_instructions(&self, messages: Vec<Message>) -> (Option<String>, Vec<Message>) {
        let mut instructions = None;
        let mut input_messages = Vec::new();

        for msg in messages {
            if msg.role == "system" {
                instructions = Some(msg.content);
            } else {
                input_messages.push(msg);
            }
        }

        (instructions, input_messages)
    }

    /// Parse Responses API response
    fn parse_responses_response(data: &serde_json::Value) -> Option<String> {
        // Response format: { output: [{ type: "message", content: [{ type: "output_text", text: "..." }] }] }
        data["output"].as_array()?
            .iter()
            .find(|item| item["type"].as_str() == Some("message"))
            .and_then(|msg| msg["content"].as_array())
            .and_then(|content| content.iter().find(|c| c["type"].as_str() == Some("output_text")))
            .and_then(|c| c["text"].as_str())
            .map(|s| s.to_string())
    }

    /// Handle Responses API streaming response
    async fn handle_responses_stream(
        &self,
        response: reqwest::Response,
        tx: mpsc::Sender<String>,
    ) -> Result<String, LLMError> {
        use futures::StreamExt;

        let mut stream = response.bytes_stream();
        let mut buffer = String::new();
        let mut full_text = String::new();

        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;
            buffer.push_str(&String::from_utf8_lossy(&chunk));

            while let Some(pos) = buffer.find('\n') {
                let line = buffer[..pos].to_string();
                buffer = buffer[pos + 1..].to_string();

                if let Some(data) = line.strip_prefix("data: ") {
                    if data == "[DONE]" {
                        continue;
                    }

                    if let Ok(event) = serde_json::from_str::<serde_json::Value>(data) {
                        // Handle streaming delta: event type "response.output_text.delta"
                        if event["type"].as_str() == Some("response.output_text.delta") {
                            if let Some(delta) = event["delta"].as_str() {
                                full_text.push_str(delta);
                                let _ = tx.send(full_text.clone()).await;
                            }
                        }
                        // Handle response.completed for final text
                        if event["type"].as_str() == Some("response.completed") {
                            if let Some(final_text) = Self::parse_responses_response(&event["response"]) {
                                if !final_text.is_empty() && final_text != full_text {
                                    full_text = final_text;
                                    let _ = tx.send(full_text.clone()).await;
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(full_text)
    }

    /// Google Gemini API call
    async fn send_google(
        &self,
        messages: Vec<Message>,
        model: &str,
        max_tokens: u32,
        _temperature: Option<f32>, // Gemini 3 recommends keeping temperature at default 1.0
        stream: bool,
        tx: Option<mpsc::Sender<String>>,
    ) -> Result<String, LLMError> {
        // Google Gemini API uses a different endpoint format:
        // https://generativelanguage.googleapis.com/v1beta/models/{model}:generateContent
        // or for streaming: :streamGenerateContent?alt=sse
        let base = self.base_url.trim_end_matches('/');
        let action = if stream { "streamGenerateContent" } else { "generateContent" };
        // Use alt=sse for streaming to get Server-Sent Events format
        let url = if stream {
            format!("{}/v1beta/models/{}:{}?alt=sse", base, model, action)
        } else {
            format!("{}/v1beta/models/{}:{}", base, model, action)
        };

        // Convert messages to Google format
        // Google uses "contents" with "parts" structure
        let contents: Vec<serde_json::Value> = messages
            .iter()
            .map(|m| {
                // Google uses "user" and "model" instead of "user" and "assistant"
                let role = if m.role == "assistant" { "model" } else { &m.role };
                serde_json::json!({
                    "role": role,
                    "parts": [{"text": m.content}]
                })
            })
            .collect();

        // Build payload - Gemini 3 recommends NOT setting custom temperature (keep at default 1.0)
        let payload = serde_json::json!({
            "contents": contents,
            "generationConfig": {
                "maxOutputTokens": max_tokens
            }
        });

        // Use x-goog-api-key header for authentication (recommended for Gemini 3)
        let response = self.client
            .post(&url)
            .header("Content-Type", "application/json")
            .header("x-goog-api-key", &self.api_key)
            .json(&payload)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(LLMError::Api(error_text));
        }

        if stream {
            self.handle_google_stream(response, tx.unwrap()).await
        } else {
            let data: serde_json::Value = response.json().await?;
            // Google response: candidates[0].content.parts[0].text
            let text = data["candidates"]
                .as_array()
                .and_then(|arr| arr.first())
                .and_then(|candidate| candidate["content"]["parts"].as_array())
                .and_then(|parts| parts.first())
                .and_then(|part| part["text"].as_str())
                .unwrap_or("")
                .to_string();
            Ok(text)
        }
    }

    /// Handle Google Gemini streaming response (SSE format with alt=sse)
    async fn handle_google_stream(
        &self,
        response: reqwest::Response,
        tx: mpsc::Sender<String>,
    ) -> Result<String, LLMError> {
        use futures::StreamExt;

        let mut stream = response.bytes_stream();
        let mut buffer = String::new();
        let mut full_text = String::new();

        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;
            buffer.push_str(&String::from_utf8_lossy(&chunk));

            // With alt=sse, Google returns SSE format: "data: {...}\n\n"
            while let Some(pos) = buffer.find('\n') {
                let line = buffer[..pos].trim().to_string();
                buffer = buffer[pos + 1..].to_string();

                // Skip empty lines
                if line.is_empty() {
                    continue;
                }

                // Parse SSE data: prefix
                let json_str = if let Some(data) = line.strip_prefix("data: ") {
                    data
                } else {
                    // Also handle raw JSON (fallback for non-SSE responses)
                    line.trim_start_matches('[').trim_end_matches(']').trim_end_matches(',')
                };

                if json_str.is_empty() {
                    continue;
                }

                if let Ok(event) = serde_json::from_str::<serde_json::Value>(json_str) {
                    // Extract text from candidates[0].content.parts[0].text
                    if let Some(parts) = event["candidates"]
                        .as_array()
                        .and_then(|arr| arr.first())
                        .and_then(|candidate| candidate["content"]["parts"].as_array())
                    {
                        for part in parts {
                            if let Some(text) = part["text"].as_str() {
                                if !text.is_empty() {
                                    full_text.push_str(text);
                                    let _ = tx.send(full_text.clone()).await;
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(full_text)
    }

    /// Check if service is reachable (for local services)
    pub async fn check_connection(&self) -> Result<bool, LLMError> {
        let base = self.base_url.trim_end_matches('/');

        // Try OpenAI models endpoint
        let models_url = if base.ends_with("/v1") {
            format!("{}/models", base)
        } else {
            format!("{}/v1/models", base)
        };

        match self.client.get(&models_url).timeout(std::time::Duration::from_secs(5)).send().await {
            Ok(resp) if resp.status().is_success() => return Ok(true),
            _ => {}
        }

        // Try Ollama specific endpoint
        let ollama_url = format!("{}/api/tags", base.replace("/v1", ""));
        match self.client.get(&ollama_url).timeout(std::time::Duration::from_secs(5)).send().await {
            Ok(resp) if resp.status().is_success() => return Ok(true),
            _ => {}
        }

        Ok(false)
    }

    /// Discover available models
    #[allow(dead_code)]
    pub async fn discover_models(&self) -> Result<Vec<String>, LLMError> {
        let base = self.base_url.trim_end_matches('/');

        // Try OpenAI models endpoint
        let models_url = if base.ends_with("/v1") {
            format!("{}/models", base)
        } else {
            format!("{}/v1/models", base)
        };

        // Build request with Authorization header if API key is provided
        let mut request = self.client.get(&models_url);
        if !self.api_key.is_empty() {
            request = request.header("Authorization", format!("Bearer {}", self.api_key));
        }

        if let Ok(resp) = request.send().await {
            if resp.status().is_success() {
                if let Ok(data) = resp.json::<serde_json::Value>().await {
                    if let Some(models) = data["data"].as_array() {
                        return Ok(models
                            .iter()
                            .filter_map(|m| m["id"].as_str().map(|s| s.to_string()))
                            .collect());
                    }
                }
            }
        }

        // Try Ollama endpoint (no auth needed for local Ollama)
        let ollama_url = format!("{}/api/tags", base.replace("/v1", ""));
        if let Ok(resp) = self.client.get(&ollama_url).send().await {
            if resp.status().is_success() {
                if let Ok(data) = resp.json::<serde_json::Value>().await {
                    if let Some(models) = data["models"].as_array() {
                        return Ok(models
                            .iter()
                            .filter_map(|m| m["name"].as_str().map(|s| s.to_string()))
                            .collect());
                    }
                }
            }
        }

        Ok(vec![])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_from_model() {
        // Claude model
        let config = ProviderConfig::from_model("claude-sonnet-4-5-20250929");
        assert_eq!(config.id, "anthropic");
        assert_eq!(config.api_format, ApiFormat::Anthropic);

        // Ollama model
        let config = ProviderConfig::from_model("llama3.3:latest");
        assert_eq!(config.id, "ollama");
        assert_eq!(config.api_format, ApiFormat::OpenAICompatible);

        // OpenRouter model
        let config = ProviderConfig::from_model("anthropic/claude-3.5-sonnet");
        assert_eq!(config.id, "openrouter");
        assert_eq!(config.api_format, ApiFormat::OpenAICompatible);

        // GPT-4 model (uses Chat Completions API)
        let config = ProviderConfig::from_model("gpt-4o");
        assert_eq!(config.id, "openai");
        assert_eq!(config.api_format, ApiFormat::OpenAI);

        // GPT-5 model (uses Responses API)
        let config = ProviderConfig::from_model("gpt-5");
        assert_eq!(config.id, "openai");
        assert_eq!(config.api_format, ApiFormat::OpenAIResponses);

        // GPT-5 mini model (uses Responses API)
        let config = ProviderConfig::from_model("gpt-5-mini");
        assert_eq!(config.id, "openai");
        assert_eq!(config.api_format, ApiFormat::OpenAIResponses);

        // GPT-5 nano model (uses Responses API)
        let config = ProviderConfig::from_model("gpt-5-nano");
        assert_eq!(config.id, "openai");
        assert_eq!(config.api_format, ApiFormat::OpenAIResponses);
    }
}
