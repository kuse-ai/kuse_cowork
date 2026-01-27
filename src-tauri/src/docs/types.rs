use serde::{Deserialize, Serialize};

/// A document stored in the database
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Document {
    pub id: String,
    pub title: String,
    pub content: String, // HTML content from TipTap editor
    pub created_at: i64,
    pub updated_at: i64,
}

/// Input for creating a new document
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateDocumentInput {
    pub title: String,
    pub content: Option<String>,
}

/// Input for updating an existing document
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateDocumentInput {
    pub title: Option<String>,
    pub content: Option<String>,
}
