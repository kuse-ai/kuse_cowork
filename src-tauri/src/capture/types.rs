use serde::{Deserialize, Serialize};

/// Maximum content preview length for display
pub const PREVIEW_LENGTH: usize = 200;
/// Maximum answer preview length (slightly longer for AI responses)
pub const ANSWER_PREVIEW_LENGTH: usize = 300;
/// Maximum clipboard content size in bytes (50KB)
pub const MAX_CLIPBOARD_SIZE: usize = 50 * 1024;

// ==================== Source Types ====================

/// Type of source for source linking
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum SourceType {
    Webpage,
    Clipboard,
    AIExchange,
    Search,
    Document,
}

impl SourceType {
    pub fn as_str(&self) -> &'static str {
        match self {
            SourceType::Webpage => "webpage",
            SourceType::Clipboard => "clipboard",
            SourceType::AIExchange => "ai_exchange",
            SourceType::Search => "search",
            SourceType::Document => "document",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "webpage" => SourceType::Webpage,
            "clipboard" => SourceType::Clipboard,
            "ai_exchange" => SourceType::AIExchange,
            "search" => SourceType::Search,
            "document" => SourceType::Document,
            _ => SourceType::Webpage,
        }
    }
}

/// How the source contributed to the content
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ContributionType {
    DirectCopy,
    Referenced,
    Inspired,
    AIAssisted,
}

impl ContributionType {
    pub fn as_str(&self) -> &'static str {
        match self {
            ContributionType::DirectCopy => "direct_copy",
            ContributionType::Referenced => "referenced",
            ContributionType::Inspired => "inspired",
            ContributionType::AIAssisted => "ai_assisted",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "direct_copy" => ContributionType::DirectCopy,
            "referenced" => ContributionType::Referenced,
            "inspired" => ContributionType::Inspired,
            "ai_assisted" => ContributionType::AIAssisted,
            _ => ContributionType::Referenced,
        }
    }
}

// ==================== Capture Types ====================

/// Clipboard copy event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClipboardCapture {
    pub id: String,
    pub content_hash: String,        // Reference to deduplicated content_store
    pub content_preview: String,     // First 200 chars for display
    pub source_url: Option<String>,  // If copied from browser
    pub source_title: Option<String>,
    pub captured_at: i64,
}

/// Browse event with full context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowseCapture {
    pub id: String,
    pub url: String,
    pub page_title: Option<String>,
    pub entered_at: i64,
    pub left_at: Option<i64>,        // For time-on-page
    pub scroll_depth_percent: Option<u8>,
}

/// Search with query text
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchCapture {
    pub id: String,
    pub query: String,               // The actual query!
    pub search_engine: String,
    pub result_clicked: Option<String>,
    pub timestamp: i64,
}

/// AI question -> answer pair
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AIExchangeCapture {
    pub id: String,
    pub question_hash: String,
    pub question_preview: String,    // First 200 chars
    pub answer_hash: String,
    pub answer_preview: String,      // First 300 chars
    pub model: String,
    pub context_doc_id: Option<String>,
    pub timestamp: i64,
}

/// Document edit event - captures writing activity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocEditCapture {
    pub id: String,
    pub doc_id: String,
    pub doc_title: String,
    pub edit_preview: String,        // Preview of what was written
    pub char_delta: i32,             // +/- characters changed
    pub started_at: i64,
    pub ended_at: i64,
}

// ==================== Input Types ====================

/// Input for reporting page context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageContextInput {
    pub url: String,
    pub title: Option<String>,
    pub entered_at: i64,
}

/// Input for updating page context when leaving
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageContextUpdate {
    pub browse_id: String,
    pub left_at: i64,
    pub scroll_depth_percent: Option<u8>,
}

/// Input for capturing a search query
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchInput {
    pub query: String,
    pub search_engine: String,
    pub timestamp: i64,
}

/// Input for capturing AI exchange
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AIExchangeInput {
    pub question: String,
    pub answer: String,
    pub model: String,
    pub context_doc_id: Option<String>,
}

// ==================== Source Linking Types ====================

/// Active source entry for tracking recent sources
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActiveSourceEntry {
    pub source_type: SourceType,
    pub source_id: String,
    pub title: Option<String>,
    pub activated_at: i64,
    pub relevance: f32,  // Decays over time, 1.0 = just activated
}

/// Source link connecting document content to sources
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceLink {
    pub id: String,
    pub doc_id: String,
    pub section_path: Option<String>,  // Where in doc this was written
    pub content_hash: String,          // What was written
    pub content_preview: Option<String>,
    pub created_at: i64,
    pub confidence_score: f32,         // Based on source relevance
}

/// Individual source within a source link
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinkedSource {
    pub id: String,
    pub link_id: String,               // FK to source_links
    pub source_type: SourceType,
    pub source_id: String,             // FK to respective capture table
    pub contribution_type: ContributionType,
    pub timestamp: i64,
}

/// Input for creating a source link
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateSourceLinkInput {
    pub doc_id: String,
    pub section_path: Option<String>,
    pub content: String,
}

/// Response with source link and its linked sources
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceLinkWithSources {
    pub link: SourceLink,
    pub sources: Vec<LinkedSource>,
}

// ==================== Config Types ====================

/// Capture configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaptureConfig {
    pub clipboard_enabled: bool,
    pub browse_enabled: bool,
    pub search_enabled: bool,
    pub ai_exchange_enabled: bool,
    pub source_linking_enabled: bool,
    pub flush_interval_secs: u32,
    pub clipboard_poll_ms: u32,
}

impl Default for CaptureConfig {
    fn default() -> Self {
        Self {
            clipboard_enabled: true,
            browse_enabled: true,
            search_enabled: true,
            ai_exchange_enabled: true,
            source_linking_enabled: true,
            flush_interval_secs: 30,
            clipboard_poll_ms: 500,
        }
    }
}

/// Result of batch insert operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchInsertResult {
    pub clipboard_inserted: usize,
    pub browse_inserted: usize,
    pub search_inserted: usize,
    pub ai_exchange_inserted: usize,
    pub doc_edit_inserted: usize,
}

// ==================== Content Store Types ====================

/// Content store entry for deduplication
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentEntry {
    pub hash: String,
    pub content: String,
    pub content_type: String,  // "text", "clipboard", "question", "answer"
    pub byte_size: i64,
    pub created_at: i64,
    pub last_accessed_at: i64,
    pub access_count: i32,
}

// ==================== Helper Functions ====================

/// Generate a preview of content (first N chars)
pub fn generate_preview(content: &str, max_len: usize) -> String {
    if content.len() <= max_len {
        content.to_string()
    } else {
        let mut preview = content.chars().take(max_len).collect::<String>();
        preview.push_str("...");
        preview
    }
}

/// Calculate SHA-256 hash of content
pub fn hash_content(content: &str) -> String {
    use sha2::{Sha256, Digest};
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    format!("{:x}", hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_preview() {
        let short = "Hello world";
        assert_eq!(generate_preview(short, 200), short);

        let long = "A".repeat(300);
        let preview = generate_preview(&long, 200);
        assert!(preview.ends_with("..."));
        assert!(preview.len() <= 203); // 200 + "..."
    }

    #[test]
    fn test_hash_content() {
        let hash = hash_content("test content");
        assert_eq!(hash.len(), 64); // SHA-256 hex string
    }
}
