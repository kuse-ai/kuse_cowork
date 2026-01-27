use serde::{Deserialize, Serialize};

/// Context type for work blocks
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ContextType {
    Document,
    Task,
    Browser,
    Manual,
    Mixed,
}

impl ContextType {
    pub fn as_str(&self) -> &'static str {
        match self {
            ContextType::Document => "document",
            ContextType::Task => "task",
            ContextType::Browser => "browser",
            ContextType::Manual => "manual",
            ContextType::Mixed => "mixed",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "document" => ContextType::Document,
            "task" => ContextType::Task,
            "browser" => ContextType::Browser,
            "manual" => ContextType::Manual,
            _ => ContextType::Mixed,
        }
    }
}

/// Event type for buffered events (memory only)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum EventType {
    Edit,
    Browse,
    Search,
    Tool,
    Focus,
    Blur,
    Save,
}

impl EventType {
    pub fn as_str(&self) -> &'static str {
        match self {
            EventType::Edit => "edit",
            EventType::Browse => "browse",
            EventType::Search => "search",
            EventType::Tool => "tool",
            EventType::Focus => "focus",
            EventType::Blur => "blur",
            EventType::Save => "save",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "edit" => EventType::Edit,
            "browse" => EventType::Browse,
            "search" => EventType::Search,
            "tool" => EventType::Tool,
            "focus" => EventType::Focus,
            "blur" => EventType::Blur,
            "save" => EventType::Save,
            _ => EventType::Edit,
        }
    }
}

/// Buffered event (memory only, never persisted)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BufferedEvent {
    pub id: String,
    pub timestamp: i64,
    pub event_type: EventType,
    pub context_type: ContextType,
    pub context_id: Option<String>,
    pub context_title: Option<String>,
    pub url: Option<String>,           // For browse events
    pub delta: Option<i32>,            // For edit events (char change)
}

/// Input for emitting an event to the buffer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventInput {
    pub event_type: String,
    pub context_type: String,
    pub context_id: Option<String>,
    pub context_title: Option<String>,
    pub url: Option<String>,
    pub delta: Option<i32>,
}

/// Work block - the persisted unit of work
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkBlock {
    pub id: String,
    pub session_id: Option<String>,
    pub context_type: ContextType,
    pub context_id: Option<String>,
    pub context_title: Option<String>,
    pub started_at: i64,
    pub ended_at: i64,
    pub duration_secs: i32,

    // Auto-generated
    pub auto_summary: Option<String>,
    pub edit_count: i32,
    pub browse_count: i32,
    pub research_urls: Vec<String>,

    // User-editable
    pub user_summary: Option<String>,
    pub notes: Option<String>,
    pub tags: Vec<String>,
    pub is_pinned: bool,
    pub is_manual: bool,

    pub created_at: i64,
    pub updated_at: i64,
}

impl WorkBlock {
    /// Get display summary (user override wins)
    pub fn display_summary(&self) -> Option<&str> {
        self.user_summary.as_deref().or(self.auto_summary.as_deref())
    }
}

/// Input for creating a work block from buffer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateBlockInput {
    pub context_type: String,
    pub context_id: Option<String>,
    pub context_title: Option<String>,
    pub started_at: i64,
    pub ended_at: i64,
    pub auto_summary: Option<String>,
    pub edit_count: i32,
    pub browse_count: i32,
    pub research_urls: Vec<String>,
}

/// Input for creating a manual work block
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManualBlockInput {
    pub context_type: Option<String>,
    pub context_id: Option<String>,
    pub context_title: Option<String>,
    pub started_at: i64,
    pub ended_at: i64,
    pub user_summary: String,
    pub notes: Option<String>,
    pub tags: Option<Vec<String>>,
}

/// Input for updating a work block (user edits)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateBlockInput {
    pub user_summary: Option<String>,
    pub notes: Option<String>,
    pub tags: Option<Vec<String>>,
    pub is_pinned: Option<bool>,
}

/// Session - container for work blocks
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: String,
    pub started_at: i64,
    pub ended_at: Option<i64>,
    pub summary: Option<String>,
    pub block_count: i32,
    pub total_duration_secs: i32,
}

/// Milestone - permanent marker for significant events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Milestone {
    pub id: String,
    pub context_type: ContextType,
    pub context_id: String,
    pub milestone_type: String,  // "created", "major_edit", "exported", "shared"
    pub timestamp: i64,
    pub note: Option<String>,
}

/// Input for creating a milestone
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MilestoneInput {
    pub context_type: String,
    pub context_id: String,
    pub milestone_type: String,
    pub note: Option<String>,
}

/// Query for listing work blocks
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WorkBlockQuery {
    pub context_type: Option<String>,
    pub context_id: Option<String>,
    pub from_timestamp: Option<i64>,
    pub to_timestamp: Option<i64>,
    #[serde(default)]
    pub include_pinned_only: bool,
    pub limit: Option<i32>,
}

/// Timeline response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Timeline {
    pub session: Option<Session>,
    pub blocks: Vec<WorkBlock>,
    pub total_duration_secs: i32,
}

/// Buffer status for UI
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BufferStatus {
    pub event_count: usize,
    pub oldest_event_at: Option<i64>,
    pub newest_event_at: Option<i64>,
    pub current_context: Option<String>,
}

/// Daily summary for quick view
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DailySummary {
    pub date: String,                   // YYYY-MM-DD
    pub block_count: i32,
    pub total_duration_secs: i32,
    pub top_contexts: Vec<String>,      // Most worked on
}
