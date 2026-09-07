use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Mutex;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum DbError {
    #[error("SQLite error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Lock error")]
    Lock,
}

use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub api_key: String,
    pub model: String,
    pub base_url: String,
    pub max_tokens: u32,
    pub temperature: f32,
    /// Provider ID (e.g., "anthropic", "ollama", "openrouter")
    /// If empty, will be inferred automatically from model
    #[serde(default)]
    pub provider: String,
    /// Provider-specific API keys
    #[serde(default)]
    pub provider_keys: HashMap<String, String>,
    /// Optional OpenAI Organization ID
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub openai_organization: Option<String>,
    /// Optional OpenAI Project ID
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub openai_project: Option<String>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            api_key: String::new(),
            model: "claude-sonnet-4-20250514".to_string(),
            base_url: "https://api.anthropic.com".to_string(),
            max_tokens: 4096,
            temperature: 0.7,
            provider: "anthropic".to_string(),
            provider_keys: HashMap::new(),
            openai_organization: None,
            openai_project: None,
        }
    }
}

impl Settings {
    /// Automatically infer provider from model (if not set)
    pub fn get_provider(&self) -> String {
        if !self.provider.is_empty() {
            return self.provider.clone();
        }

        // Infer from model name
        let model_lower = self.model.to_lowercase();

        if model_lower.contains("claude") {
            "anthropic".to_string()
        } else if (model_lower.contains("gpt") || model_lower.starts_with("o1") || model_lower.starts_with("o3") || model_lower.starts_with("gpt-")) && !model_lower.contains("/") {
            // OpenAI models: gpt-*, o1-*, o3-*
            "openai".to_string()
        } else if model_lower.contains("gemini") {
            "google".to_string()
        } else if model_lower.contains("minimax") {
            "minimax".to_string()
        } else if model_lower.starts_with("anthropic/") || model_lower.starts_with("openai/") || model_lower.starts_with("meta-llama/") || model_lower.starts_with("deepseek/") 
            || model_lower.starts_with("z-ai/") || model_lower.starts_with("minimax/") || model_lower.starts_with("moonshot/"){
            "openrouter".to_string()
        } else if model_lower.contains(":") {
            // Ollama format (e.g., llama3.3:latest)
            "ollama".to_string()
        } else {
            // Default to anthropic
            "anthropic".to_string()
        }
    }

    /// Check if it's a local service that doesn't require API Key
    /// Only returns true for known local inference services with authType === "none"
    pub fn is_local_provider(&self) -> bool {
        let provider = self.get_provider();
        // Only these providers truly don't need API keys
        matches!(provider.as_str(), "ollama" | "localai" | "vllm" | "tgi" | "sglang" | "lm-studio")
    }

    /// Check if API key can be empty (local providers or custom with empty key)
    pub fn allows_empty_api_key(&self) -> bool {
        // Known local providers don't need API key
        if self.is_local_provider() {
            return true;
        }
        // Custom provider with localhost URL - API key is optional
        let provider = self.get_provider();
        if provider == "custom" {
            return true;
        }
        false
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Conversation {
    pub id: String,
    pub title: String,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: String,
    pub conversation_id: String,
    pub role: String,
    pub content: String,
    pub timestamp: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: String,
    pub title: String,
    pub description: String,
    pub status: String, // "planning", "running", "completed", "failed"
    pub plan: Option<Vec<PlanStep>>,
    pub current_step: i32,
    pub project_path: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanStep {
    pub step: i32,
    pub description: String,
    pub status: String, // "pending", "running", "completed", "failed"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskMessage {
    pub id: String,
    pub task_id: String,
    pub role: String, // "user", "assistant"
    pub content: String,
    pub timestamp: i64,
}

pub struct Database {
    pub(crate) conn: Mutex<Connection>,
}

impl Database {
    pub fn new() -> Result<Self, DbError> {
        let db_path = Self::get_db_path()?;

        // Ensure parent directory exists
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let conn = Connection::open(&db_path)?;
        let db = Self {
            conn: Mutex::new(conn),
        };
        db.init_tables()?;
        Ok(db)
    }

    fn get_db_path() -> Result<PathBuf, DbError> {
        let data_dir = dirs::data_dir()
            .ok_or_else(|| DbError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "Could not find data directory",
            )))?;
        Ok(data_dir.join("kuse-cowork").join("kuse-cowork.db"))
    }

    fn init_tables(&self) -> Result<(), DbError> {
        let conn = self.conn.lock().map_err(|_| DbError::Lock)?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS settings (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            )",
            [],
        )?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS conversations (
                id TEXT PRIMARY KEY,
                title TEXT NOT NULL,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL
            )",
            [],
        )?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS messages (
                id TEXT PRIMARY KEY,
                conversation_id TEXT NOT NULL,
                role TEXT NOT NULL,
                content TEXT NOT NULL,
                timestamp INTEGER NOT NULL,
                FOREIGN KEY (conversation_id) REFERENCES conversations(id) ON DELETE CASCADE
            )",
            [],
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_messages_conversation
             ON messages(conversation_id)",
            [],
        )?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS tasks (
                id TEXT PRIMARY KEY,
                title TEXT NOT NULL,
                description TEXT NOT NULL,
                status TEXT NOT NULL DEFAULT 'planning',
                plan TEXT,
                current_step INTEGER NOT NULL DEFAULT 0,
                project_path TEXT,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL
            )",
            [],
        )?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS task_messages (
                id TEXT PRIMARY KEY,
                task_id TEXT NOT NULL,
                role TEXT NOT NULL,
                content TEXT NOT NULL,
                timestamp INTEGER NOT NULL,
                FOREIGN KEY (task_id) REFERENCES tasks(id) ON DELETE CASCADE
            )",
            [],
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_task_messages_task
             ON task_messages(task_id)",
            [],
        )?;

        Ok(())
    }

    // Settings methods
    pub fn get_settings(&self) -> Result<Settings, DbError> {
        let conn = self.conn.lock().map_err(|_| DbError::Lock)?;
        let mut settings = Settings::default();

        let mut stmt = conn.prepare("SELECT key, value FROM settings")?;
        let rows = stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?;

        for row in rows {
            let (key, value) = row?;
            match key.as_str() {
                "api_key" => settings.api_key = value,
                "model" => settings.model = value,
                "base_url" => settings.base_url = value,
                "max_tokens" => settings.max_tokens = value.parse().unwrap_or(4096),
                "temperature" => settings.temperature = value.parse().unwrap_or(0.7),
                "provider" => settings.provider = value,
                "provider_keys" => {
                    // Parse JSON to HashMap
                    if let Ok(keys) = serde_json::from_str::<HashMap<String, String>>(&value) {
                        settings.provider_keys = keys;
                    }
                }
                _ => {}
            }
        }

        // If provider is empty, infer from model
        if settings.provider.is_empty() {
            settings.provider = settings.get_provider();
        }

        // If api_key is empty but we have a provider_key for current provider, use it
        if settings.api_key.is_empty() {
            if let Some(key) = settings.provider_keys.get(&settings.provider) {
                settings.api_key = key.clone();
            }
        }

        Ok(settings)
    }

    pub fn save_settings(&self, settings: &Settings) -> Result<(), DbError> {
        let conn = self.conn.lock().map_err(|_| DbError::Lock)?;

        // If provider is empty, infer automatically
        let provider = if settings.provider.is_empty() {
            settings.get_provider()
        } else {
            settings.provider.clone()
        };

        // Serialize provider_keys to JSON
        let provider_keys_json = serde_json::to_string(&settings.provider_keys)
            .unwrap_or_else(|_| "{}".to_string());

        let pairs = [
            ("api_key", settings.api_key.clone()),
            ("model", settings.model.clone()),
            ("base_url", settings.base_url.clone()),
            ("max_tokens", settings.max_tokens.to_string()),
            ("temperature", settings.temperature.to_string()),
            ("provider", provider),
            ("provider_keys", provider_keys_json),
        ];

        for (key, value) in pairs {
            conn.execute(
                "INSERT OR REPLACE INTO settings (key, value) VALUES (?1, ?2)",
                [key, &value],
            )?;
        }

        Ok(())
    }

    // Conversation methods
    pub fn list_conversations(&self) -> Result<Vec<Conversation>, DbError> {
        let conn = self.conn.lock().map_err(|_| DbError::Lock)?;

        let mut stmt = conn.prepare(
            "SELECT id, title, created_at, updated_at
             FROM conversations
             ORDER BY updated_at DESC"
        )?;

        let rows = stmt.query_map([], |row| {
            Ok(Conversation {
                id: row.get(0)?,
                title: row.get(1)?,
                created_at: row.get(2)?,
                updated_at: row.get(3)?,
            })
        })?;

        let mut conversations = Vec::new();
        for row in rows {
            conversations.push(row?);
        }

        Ok(conversations)
    }

    pub fn create_conversation(&self, id: &str, title: &str) -> Result<Conversation, DbError> {
        let conn = self.conn.lock().map_err(|_| DbError::Lock)?;
        let now = chrono::Utc::now().timestamp_millis();

        conn.execute(
            "INSERT INTO conversations (id, title, created_at, updated_at) VALUES (?1, ?2, ?3, ?4)",
            [id, title, &now.to_string(), &now.to_string()],
        )?;

        Ok(Conversation {
            id: id.to_string(),
            title: title.to_string(),
            created_at: now,
            updated_at: now,
        })
    }

    pub fn update_conversation_title(&self, id: &str, title: &str) -> Result<(), DbError> {
        let conn = self.conn.lock().map_err(|_| DbError::Lock)?;
        let now = chrono::Utc::now().timestamp_millis();

        conn.execute(
            "UPDATE conversations SET title = ?1, updated_at = ?2 WHERE id = ?3",
            [title, &now.to_string(), id],
        )?;

        Ok(())
    }

    pub fn delete_conversation(&self, id: &str) -> Result<(), DbError> {
        let conn = self.conn.lock().map_err(|_| DbError::Lock)?;

        // Delete messages first (cascade)
        conn.execute("DELETE FROM messages WHERE conversation_id = ?1", [id])?;
        conn.execute("DELETE FROM conversations WHERE id = ?1", [id])?;

        Ok(())
    }

    // Message methods
    pub fn get_messages(&self, conversation_id: &str) -> Result<Vec<Message>, DbError> {
        let conn = self.conn.lock().map_err(|_| DbError::Lock)?;

        let mut stmt = conn.prepare(
            "SELECT id, conversation_id, role, content, timestamp
             FROM messages
             WHERE conversation_id = ?1
             ORDER BY timestamp ASC"
        )?;

        let rows = stmt.query_map([conversation_id], |row| {
            Ok(Message {
                id: row.get(0)?,
                conversation_id: row.get(1)?,
                role: row.get(2)?,
                content: row.get(3)?,
                timestamp: row.get(4)?,
            })
        })?;

        let mut messages = Vec::new();
        for row in rows {
            messages.push(row?);
        }

        Ok(messages)
    }

    pub fn add_message(
        &self,
        id: &str,
        conversation_id: &str,
        role: &str,
        content: &str,
    ) -> Result<Message, DbError> {
        let conn = self.conn.lock().map_err(|_| DbError::Lock)?;
        let now = chrono::Utc::now().timestamp_millis();

        conn.execute(
            "INSERT INTO messages (id, conversation_id, role, content, timestamp)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            [id, conversation_id, role, content, &now.to_string()],
        )?;

        // Update conversation's updated_at
        conn.execute(
            "UPDATE conversations SET updated_at = ?1 WHERE id = ?2",
            [&now.to_string(), conversation_id],
        )?;

        Ok(Message {
            id: id.to_string(),
            conversation_id: conversation_id.to_string(),
            role: role.to_string(),
            content: content.to_string(),
            timestamp: now,
        })
    }

    #[allow(dead_code)]
    pub fn update_message_content(&self, id: &str, content: &str) -> Result<(), DbError> {
        let conn = self.conn.lock().map_err(|_| DbError::Lock)?;

        conn.execute(
            "UPDATE messages SET content = ?1 WHERE id = ?2",
            [content, id],
        )?;

        Ok(())
    }

    // Task methods
    pub fn list_tasks(&self) -> Result<Vec<Task>, DbError> {
        let conn = self.conn.lock().map_err(|_| DbError::Lock)?;

        let mut stmt = conn.prepare(
            "SELECT id, title, description, status, plan, current_step, project_path, created_at, updated_at
             FROM tasks
             ORDER BY updated_at DESC"
        )?;

        let rows = stmt.query_map([], |row| {
            let plan_json: Option<String> = row.get(4)?;
            let plan: Option<Vec<PlanStep>> = plan_json
                .and_then(|json| serde_json::from_str(&json).ok());

            Ok(Task {
                id: row.get(0)?,
                title: row.get(1)?,
                description: row.get(2)?,
                status: row.get(3)?,
                plan,
                current_step: row.get(5)?,
                project_path: row.get(6)?,
                created_at: row.get(7)?,
                updated_at: row.get(8)?,
            })
        })?;

        let mut tasks = Vec::new();
        for row in rows {
            tasks.push(row?);
        }

        Ok(tasks)
    }

    pub fn get_task(&self, id: &str) -> Result<Option<Task>, DbError> {
        let conn = self.conn.lock().map_err(|_| DbError::Lock)?;

        let mut stmt = conn.prepare(
            "SELECT id, title, description, status, plan, current_step, project_path, created_at, updated_at
             FROM tasks WHERE id = ?1"
        )?;

        let mut rows = stmt.query([id])?;

        if let Some(row) = rows.next()? {
            let plan_json: Option<String> = row.get(4)?;
            let plan: Option<Vec<PlanStep>> = plan_json
                .and_then(|json| serde_json::from_str(&json).ok());

            Ok(Some(Task {
                id: row.get(0)?,
                title: row.get(1)?,
                description: row.get(2)?,
                status: row.get(3)?,
                plan,
                current_step: row.get(5)?,
                project_path: row.get(6)?,
                created_at: row.get(7)?,
                updated_at: row.get(8)?,
            }))
        } else {
            Ok(None)
        }
    }

    pub fn create_task(&self, id: &str, title: &str, description: &str, project_path: Option<&str>) -> Result<Task, DbError> {
        let conn = self.conn.lock().map_err(|_| DbError::Lock)?;
        let now = chrono::Utc::now().timestamp_millis();

        conn.execute(
            "INSERT INTO tasks (id, title, description, status, current_step, project_path, created_at, updated_at)
             VALUES (?1, ?2, ?3, 'planning', 0, ?4, ?5, ?6)",
            rusqlite::params![id, title, description, project_path, now, now],
        )?;

        Ok(Task {
            id: id.to_string(),
            title: title.to_string(),
            description: description.to_string(),
            status: "planning".to_string(),
            plan: None,
            current_step: 0,
            project_path: project_path.map(|s| s.to_string()),
            created_at: now,
            updated_at: now,
        })
    }

    pub fn update_task_plan(&self, id: &str, plan: &[PlanStep]) -> Result<(), DbError> {
        let conn = self.conn.lock().map_err(|_| DbError::Lock)?;
        let now = chrono::Utc::now().timestamp_millis();
        let plan_json = serde_json::to_string(plan).unwrap_or_default();

        conn.execute(
            "UPDATE tasks SET plan = ?1, status = 'running', updated_at = ?2 WHERE id = ?3",
            rusqlite::params![plan_json, now, id],
        )?;

        Ok(())
    }

    pub fn update_task_step(&self, id: &str, current_step: i32, step_status: &str) -> Result<(), DbError> {
        let conn = self.conn.lock().map_err(|_| DbError::Lock)?;
        let now = chrono::Utc::now().timestamp_millis();

        // Get current plan and update the step status
        let mut stmt = conn.prepare("SELECT plan FROM tasks WHERE id = ?1")?;
        let plan_json: Option<String> = stmt.query_row([id], |row| row.get(0)).ok().flatten();

        if let Some(json) = plan_json {
            if let Ok(mut plan) = serde_json::from_str::<Vec<PlanStep>>(&json) {
                if let Some(step) = plan.iter_mut().find(|s| s.step == current_step) {
                    step.status = step_status.to_string();
                }
                let updated_json = serde_json::to_string(&plan).unwrap_or_default();
                conn.execute(
                    "UPDATE tasks SET plan = ?1, current_step = ?2, updated_at = ?3 WHERE id = ?4",
                    rusqlite::params![updated_json, current_step, now, id],
                )?;
            }
        }

        Ok(())
    }

    pub fn update_task_status(&self, id: &str, status: &str) -> Result<(), DbError> {
        let conn = self.conn.lock().map_err(|_| DbError::Lock)?;
        let now = chrono::Utc::now().timestamp_millis();

        conn.execute(
            "UPDATE tasks SET status = ?1, updated_at = ?2 WHERE id = ?3",
            rusqlite::params![status, now, id],
        )?;

        Ok(())
    }

    pub fn delete_task(&self, id: &str) -> Result<(), DbError> {
        let conn = self.conn.lock().map_err(|_| DbError::Lock)?;
        // Delete messages first
        conn.execute("DELETE FROM task_messages WHERE task_id = ?1", [id])?;
        conn.execute("DELETE FROM tasks WHERE id = ?1", [id])?;
        Ok(())
    }

    // Task message methods
    pub fn get_task_messages(&self, task_id: &str) -> Result<Vec<TaskMessage>, DbError> {
        let conn = self.conn.lock().map_err(|_| DbError::Lock)?;

        let mut stmt = conn.prepare(
            "SELECT id, task_id, role, content, timestamp
             FROM task_messages
             WHERE task_id = ?1
             ORDER BY timestamp ASC"
        )?;

        let rows = stmt.query_map([task_id], |row| {
            Ok(TaskMessage {
                id: row.get(0)?,
                task_id: row.get(1)?,
                role: row.get(2)?,
                content: row.get(3)?,
                timestamp: row.get(4)?,
            })
        })?;

        let mut messages = Vec::new();
        for row in rows {
            messages.push(row?);
        }

        Ok(messages)
    }

    pub fn add_task_message(
        &self,
        id: &str,
        task_id: &str,
        role: &str,
        content: &str,
    ) -> Result<TaskMessage, DbError> {
        let conn = self.conn.lock().map_err(|_| DbError::Lock)?;
        let now = chrono::Utc::now().timestamp_millis();

        conn.execute(
            "INSERT INTO task_messages (id, task_id, role, content, timestamp)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![id, task_id, role, content, now],
        )?;

        // Update task's updated_at
        conn.execute(
            "UPDATE tasks SET updated_at = ?1 WHERE id = ?2",
            rusqlite::params![now, task_id],
        )?;

        Ok(TaskMessage {
            id: id.to_string(),
            task_id: task_id.to_string(),
            role: role.to_string(),
            content: content.to_string(),
            timestamp: now,
        })
    }

    #[allow(dead_code)]
    pub fn update_task_message_content(&self, id: &str, content: &str) -> Result<(), DbError> {
        let conn = self.conn.lock().map_err(|_| DbError::Lock)?;

        conn.execute(
            "UPDATE task_messages SET content = ?1 WHERE id = ?2",
            [content, id],
        )?;

        Ok(())
    }
}
