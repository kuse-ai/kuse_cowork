use serde::{Deserialize, Serialize};

/// Represents a single trace event in the system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Trace {
    pub id: String,
    pub task_id: Option<String>,
    pub doc_id: String,
    /// Event type: "edit", "search", "browse", "approval", "tool_start", "tool_end"
    pub event_type: String,
    /// Path to the section being edited (for edit events)
    pub section_path: Option<String>,
    /// Character delta (positive for additions, negative for deletions)
    pub delta: Option<i32>,
    /// Additional event data (query, url, snippet, etc.)
    pub payload: serde_json::Value,
    pub created_at: i64,
}

/// Settings for trace collection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceSettings {
    pub doc_id: Option<String>,
    pub tracing_enabled: bool,
    pub include_snippets: bool,
}

impl Default for TraceSettings {
    fn default() -> Self {
        Self {
            doc_id: None,
            tracing_enabled: true,
            include_snippets: true,
        }
    }
}

/// Input for creating a new trace
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceInput {
    pub task_id: Option<String>,
    pub doc_id: String,
    pub event_type: String,
    pub section_path: Option<String>,
    pub delta: Option<i32>,
    pub payload: Option<serde_json::Value>,
}

/// AI-generated suggestion based on trace history
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Suggestion {
    pub id: String,
    /// Type of suggestion: "edit", "add_section", "search", "refactor"
    pub suggestion_type: String,
    pub title: String,
    pub description: String,
    /// Proposed changes or parameters for the suggestion
    pub payload: serde_json::Value,
    /// Status: "pending", "approved", "rejected"
    pub status: String,
    pub created_at: i64,
}
