use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use crate::workstream::types::*;

/// Maximum events to buffer before forcing a flush
const MAX_BUFFER_SIZE: usize = 100;

/// Minimum events needed to create a work block
const MIN_EVENTS_FOR_BLOCK: usize = 2;

/// In-memory event buffer - NEVER persisted to disk
pub struct EventBuffer {
    events: Arc<Mutex<VecDeque<BufferedEvent>>>,
    current_context: Arc<Mutex<Option<(ContextType, Option<String>)>>>,
}

impl EventBuffer {
    pub fn new() -> Self {
        Self {
            events: Arc::new(Mutex::new(VecDeque::with_capacity(MAX_BUFFER_SIZE))),
            current_context: Arc::new(Mutex::new(None)),
        }
    }

    /// Add an event to the buffer
    pub fn push(&self, event: BufferedEvent) -> BufferPushResult {
        let mut events = self.events.lock().unwrap();
        let mut current_ctx = self.current_context.lock().unwrap();

        // Check for context switch
        let context_switched = if let Some((ctx_type, ctx_id)) = current_ctx.as_ref() {
            event.context_type != *ctx_type || event.context_id != *ctx_id
        } else {
            false
        };

        // Update current context
        *current_ctx = Some((event.context_type.clone(), event.context_id.clone()));

        // Add event
        events.push_back(event);

        // Determine if we should trigger a flush
        let should_flush = events.len() >= MAX_BUFFER_SIZE || context_switched;

        BufferPushResult {
            should_flush,
            context_switched,
            event_count: events.len(),
        }
    }

    /// Get current buffer status
    pub fn status(&self) -> BufferStatus {
        let events = self.events.lock().unwrap();
        let current_ctx = self.current_context.lock().unwrap();

        BufferStatus {
            event_count: events.len(),
            oldest_event_at: events.front().map(|e| e.timestamp),
            newest_event_at: events.back().map(|e| e.timestamp),
            current_context: current_ctx.as_ref().and_then(|(_, id)| id.clone()),
        }
    }

    /// Check if buffer has enough events for a block
    pub fn can_create_block(&self) -> bool {
        let events = self.events.lock().unwrap();
        events.len() >= MIN_EVENTS_FOR_BLOCK
    }

    /// Flush buffer and return data for creating a work block
    /// Returns None if not enough events
    pub fn flush(&self) -> Option<FlushResult> {
        let mut events = self.events.lock().unwrap();
        let mut current_ctx = self.current_context.lock().unwrap();

        if events.len() < MIN_EVENTS_FOR_BLOCK {
            return None;
        }

        // Collect all events
        let flushed: Vec<BufferedEvent> = events.drain(..).collect();
        *current_ctx = None;

        // Calculate metrics
        let started_at = flushed.first().map(|e| e.timestamp).unwrap_or(0);
        let ended_at = flushed.last().map(|e| e.timestamp).unwrap_or(started_at);

        let edit_count = flushed.iter().filter(|e| e.event_type == EventType::Edit).count() as i32;
        let browse_count = flushed.iter().filter(|e| e.event_type == EventType::Browse).count() as i32;

        // Extract unique research URLs
        let research_urls: Vec<String> = flushed
            .iter()
            .filter(|e| e.event_type == EventType::Browse)
            .filter_map(|e| e.url.clone())
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .take(5) // Max 5 URLs
            .collect();

        // Determine context from events
        let context_types: Vec<_> = flushed.iter().map(|e| &e.context_type).collect();
        let context_type = if context_types.iter().all(|t| *t == context_types[0]) {
            context_types[0].clone()
        } else {
            ContextType::Mixed
        };

        let context_id = flushed.first().and_then(|e| e.context_id.clone());
        let context_title = flushed.iter().find_map(|e| e.context_title.clone());

        // Generate local summary
        let auto_summary = generate_local_summary(&flushed, &context_title);

        Some(FlushResult {
            events: flushed,
            started_at,
            ended_at,
            context_type,
            context_id,
            context_title,
            edit_count,
            browse_count,
            research_urls,
            auto_summary,
        })
    }

    /// Clear buffer without creating a block (e.g., on discard)
    pub fn clear(&self) {
        let mut events = self.events.lock().unwrap();
        let mut current_ctx = self.current_context.lock().unwrap();
        events.clear();
        *current_ctx = None;
    }

    /// Get time since last event (for inactivity detection)
    pub fn time_since_last_event(&self) -> Option<i64> {
        let events = self.events.lock().unwrap();
        events.back().map(|e| chrono::Utc::now().timestamp_millis() - e.timestamp)
    }
}

impl Default for EventBuffer {
    fn default() -> Self {
        Self::new()
    }
}

/// Result of pushing an event
pub struct BufferPushResult {
    pub should_flush: bool,
    pub context_switched: bool,
    pub event_count: usize,
}

/// Result of flushing the buffer
pub struct FlushResult {
    pub events: Vec<BufferedEvent>,
    pub started_at: i64,
    pub ended_at: i64,
    pub context_type: ContextType,
    pub context_id: Option<String>,
    pub context_title: Option<String>,
    pub edit_count: i32,
    pub browse_count: i32,
    pub research_urls: Vec<String>,
    pub auto_summary: String,
}

/// Generate a simple local summary without AI
fn generate_local_summary(events: &[BufferedEvent], context_title: &Option<String>) -> String {
    let edit_count = events.iter().filter(|e| e.event_type == EventType::Edit).count();
    let browse_count = events.iter().filter(|e| e.event_type == EventType::Browse).count();
    let search_count = events.iter().filter(|e| e.event_type == EventType::Search).count();
    let tool_count = events.iter().filter(|e| e.event_type == EventType::Tool).count();
    let save_count = events.iter().filter(|e| e.event_type == EventType::Save).count();

    let title = context_title.as_deref().unwrap_or("document");

    let mut parts = Vec::new();

    // Primary action
    if edit_count > 0 {
        if save_count > 0 {
            parts.push(format!("Edited and saved {}", title));
        } else {
            parts.push(format!("Edited {}", title));
        }
    }

    // Research component
    if browse_count > 0 {
        if edit_count > 0 {
            parts.push(format!("with {} site{} researched", browse_count, if browse_count > 1 { "s" } else { "" }));
        } else {
            parts.push(format!("Browsed {} site{}", browse_count, if browse_count > 1 { "s" } else { "" }));
        }
    }

    // Search component
    if search_count > 0 && parts.is_empty() {
        parts.push(format!("Searched {} time{}", search_count, if search_count > 1 { "s" } else { "" }));
    }

    // Tool component
    if tool_count > 0 {
        if parts.is_empty() {
            parts.push(format!("Used {} tool{}", tool_count, if tool_count > 1 { "s" } else { "" }));
        } else {
            parts.push(format!("using {} tool{}", tool_count, if tool_count > 1 { "s" } else { "" }));
        }
    }

    if parts.is_empty() {
        return format!("Brief activity on {}", title);
    }

    parts.join(", ")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_event(event_type: EventType, context_id: Option<&str>) -> BufferedEvent {
        BufferedEvent {
            id: uuid::Uuid::new_v4().to_string(),
            timestamp: chrono::Utc::now().timestamp_millis(),
            event_type,
            context_type: ContextType::Document,
            context_id: context_id.map(String::from),
            context_title: Some("Test Doc".to_string()),
            url: None,
            delta: None,
        }
    }

    #[test]
    fn test_buffer_push_and_status() {
        let buffer = EventBuffer::new();

        buffer.push(make_event(EventType::Edit, Some("doc1")));
        buffer.push(make_event(EventType::Edit, Some("doc1")));

        let status = buffer.status();
        assert_eq!(status.event_count, 2);
    }

    #[test]
    fn test_context_switch_detection() {
        let buffer = EventBuffer::new();

        buffer.push(make_event(EventType::Edit, Some("doc1")));
        let result = buffer.push(make_event(EventType::Edit, Some("doc2")));

        assert!(result.context_switched);
    }

    #[test]
    fn test_flush() {
        let buffer = EventBuffer::new();

        buffer.push(make_event(EventType::Edit, Some("doc1")));
        buffer.push(make_event(EventType::Edit, Some("doc1")));
        buffer.push(make_event(EventType::Browse, Some("doc1")));

        let result = buffer.flush();
        assert!(result.is_some());

        let flush = result.unwrap();
        assert_eq!(flush.edit_count, 2);
        assert_eq!(flush.browse_count, 1);
    }

    #[test]
    fn test_local_summary_generation() {
        let events = vec![
            make_event(EventType::Edit, Some("doc1")),
            make_event(EventType::Edit, Some("doc1")),
            make_event(EventType::Save, Some("doc1")),
        ];

        let summary = generate_local_summary(&events, &Some("Q3 Proposal".to_string()));
        assert!(summary.contains("Edited and saved"));
        assert!(summary.contains("Q3 Proposal"));
    }
}
