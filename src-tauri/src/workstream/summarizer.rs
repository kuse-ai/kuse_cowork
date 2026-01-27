use crate::llm_client::{LLMClient, Message};
use crate::workstream::types::WorkBlock;

/// AI-powered summarizer for work blocks (optional, on-demand)
pub struct WorkStreamSummarizer {
    client: LLMClient,
    model: String,
}

impl WorkStreamSummarizer {
    pub fn new(api_key: String, model: String, provider: Option<&str>) -> Self {
        let client = LLMClient::new(api_key, None, provider, Some(&model));
        Self { client, model }
    }

    /// Generate an AI-enhanced summary for a work block
    /// Returns a more meaningful summary than the local generation
    pub async fn enhance_summary(&self, block: &WorkBlock) -> Result<String, String> {
        let prompt = build_enhance_prompt(block);

        let messages = vec![
            Message {
                role: "system".to_string(),
                content: SUMMARIZER_SYSTEM_PROMPT.to_string(),
            },
            Message {
                role: "user".to_string(),
                content: prompt,
            },
        ];

        self.client
            .send_message(messages, &self.model, 100, Some(0.3))
            .await
            .map_err(|e| e.to_string())
    }
}

fn build_enhance_prompt(block: &WorkBlock) -> String {
    let mut parts = Vec::new();

    // Context
    if let Some(title) = &block.context_title {
        parts.push(format!("Document/Task: {}", title));
    }

    // Auto-generated summary
    if let Some(auto) = &block.auto_summary {
        parts.push(format!("Activity: {}", auto));
    }

    // Metrics
    parts.push(format!(
        "Metrics: {} edits, {} pages browsed, {} minutes",
        block.edit_count,
        block.browse_count,
        block.duration_secs / 60
    ));

    // Research URLs
    if !block.research_urls.is_empty() {
        let urls = block.research_urls.iter()
            .map(|u| {
                // Extract domain from URL using simple string manipulation
                extract_domain(u)
            })
            .collect::<Vec<_>>()
            .join(", ");
        parts.push(format!("Research: {}", urls));
    }

    // User notes (if any)
    if let Some(notes) = &block.notes {
        parts.push(format!("User notes: {}", notes));
    }

    parts.push("\nProvide a concise 1-sentence summary of what was accomplished.".to_string());

    parts.join("\n")
}

/// Extract domain from URL using simple string manipulation
fn extract_domain(url: &str) -> String {
    // Remove protocol
    let without_protocol = url
        .strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"))
        .unwrap_or(url);

    // Take everything before the first /
    let domain = without_protocol
        .split('/')
        .next()
        .unwrap_or(without_protocol);

    // Remove www. prefix if present
    domain
        .strip_prefix("www.")
        .unwrap_or(domain)
        .to_string()
}

const SUMMARIZER_SYSTEM_PROMPT: &str = r#"You are a work activity summarizer. Create concise, meaningful summaries.

Rules:
- Maximum 1 sentence, under 100 characters
- Focus on WHAT was accomplished, not mechanics
- Use active voice
- If research was done, connect it to the outcome
- Be specific but brief

Good examples:
- "Revised pricing strategy after competitor research"
- "Drafted project introduction with supporting data"
- "Debugged auth flow and added error handling"

Bad examples:
- "Made 5 edits and browsed 3 pages" (too mechanical)
- "Worked on document" (too vague)
- "Did some research and editing" (uninformative)"#;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workstream::types::ContextType;

    #[test]
    fn test_build_enhance_prompt() {
        let block = WorkBlock {
            id: "test".to_string(),
            session_id: None,
            context_type: ContextType::Document,
            context_id: Some("doc-1".to_string()),
            context_title: Some("Q3 Proposal".to_string()),
            started_at: 0,
            ended_at: 600000,
            duration_secs: 600,
            auto_summary: Some("Edited Q3 Proposal with 2 sites researched".to_string()),
            edit_count: 5,
            browse_count: 2,
            research_urls: vec!["https://stripe.com/pricing".to_string()],
            user_summary: None,
            notes: None,
            tags: vec![],
            is_pinned: false,
            is_manual: false,
            created_at: 0,
            updated_at: 0,
        };

        let prompt = build_enhance_prompt(&block);

        assert!(prompt.contains("Q3 Proposal"));
        assert!(prompt.contains("5 edits"));
        assert!(prompt.contains("stripe.com"));
    }
}
