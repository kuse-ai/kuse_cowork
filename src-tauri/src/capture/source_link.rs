use std::sync::{Arc, Mutex};
use crate::capture::types::*;
use crate::database::Database;

/// Decay rate per second (relevance decays over time)
const DECAY_RATE_PER_SEC: f32 = 0.01;
/// Minimum relevance threshold for linking
const MIN_RELEVANCE_THRESHOLD: f32 = 0.3;
/// Maximum number of active sources to track
const MAX_ACTIVE_SOURCES: usize = 30;

/// Tracks active sources with relevance decay
pub struct ActiveSourceTracker {
    sources: Arc<Mutex<Vec<ActiveSourceEntry>>>,
}

impl ActiveSourceTracker {
    pub fn new() -> Self {
        Self {
            sources: Arc::new(Mutex::new(Vec::with_capacity(MAX_ACTIVE_SOURCES))),
        }
    }

    /// Activate or refresh a source (called when user views it)
    pub fn activate_source(&self, source_type: SourceType, source_id: String, title: Option<String>) {
        let mut sources = self.sources.lock().unwrap();
        let now = chrono::Utc::now().timestamp_millis();

        // Check if source already exists
        if let Some(existing) = sources.iter_mut().find(|s| s.source_id == source_id) {
            // Refresh: reset relevance to 1.0 and update timestamp
            existing.relevance = 1.0;
            existing.activated_at = now;
            existing.title = title;
            return;
        }

        // Add new source
        let entry = ActiveSourceEntry {
            source_type,
            source_id,
            title,
            activated_at: now,
            relevance: 1.0,
        };

        // If at capacity, remove the least relevant source
        if sources.len() >= MAX_ACTIVE_SOURCES {
            // Apply decay before finding minimum
            Self::apply_decay_internal(&mut sources, now);

            // Find index of least relevant source
            if let Some((min_idx, _)) = sources
                .iter()
                .enumerate()
                .min_by(|(_, a), (_, b)| a.relevance.partial_cmp(&b.relevance).unwrap())
            {
                sources.remove(min_idx);
            }
        }

        sources.push(entry);
    }

    /// Deactivate a source (e.g., when leaving a page)
    pub fn deactivate_source(&self, source_id: &str) {
        let mut sources = self.sources.lock().unwrap();
        sources.retain(|s| s.source_id != source_id);
    }

    /// Get active sources with relevance >= threshold
    pub fn get_active_sources(&self, min_relevance: Option<f32>) -> Vec<ActiveSourceEntry> {
        let mut sources = self.sources.lock().unwrap();
        let now = chrono::Utc::now().timestamp_millis();
        let threshold = min_relevance.unwrap_or(MIN_RELEVANCE_THRESHOLD);

        // Apply decay
        Self::apply_decay_internal(&mut sources, now);

        // Filter by threshold and return cloned results
        sources
            .iter()
            .filter(|s| s.relevance >= threshold)
            .cloned()
            .collect()
    }

    /// Apply decay to all sources and remove those below threshold
    pub fn apply_decay(&self) {
        let mut sources = self.sources.lock().unwrap();
        let now = chrono::Utc::now().timestamp_millis();

        Self::apply_decay_internal(&mut sources, now);

        // Remove sources below minimum threshold
        sources.retain(|s| s.relevance >= MIN_RELEVANCE_THRESHOLD);
    }

    /// Internal decay application
    fn apply_decay_internal(sources: &mut Vec<ActiveSourceEntry>, now: i64) {
        for source in sources.iter_mut() {
            let elapsed_secs = ((now - source.activated_at) / 1000) as f32;
            source.relevance = (1.0 - DECAY_RATE_PER_SEC * elapsed_secs).max(0.0);
        }
    }

    /// Get the average relevance of active sources
    pub fn average_relevance(&self) -> f32 {
        let sources = self.sources.lock().unwrap();
        if sources.is_empty() {
            return 0.0;
        }

        let sum: f32 = sources.iter().map(|s| s.relevance).sum();
        sum / sources.len() as f32
    }

    /// Clear all active sources
    pub fn clear(&self) {
        let mut sources = self.sources.lock().unwrap();
        sources.clear();
    }

    /// Get the count of active sources
    pub fn count(&self) -> usize {
        self.sources.lock().unwrap().len()
    }
}

impl Default for ActiveSourceTracker {
    fn default() -> Self {
        Self::new()
    }
}

/// Source linking service that connects written content to sources
pub struct SourceLinker {
    tracker: Arc<ActiveSourceTracker>,
    db: Arc<Database>,
}

impl SourceLinker {
    pub fn new(tracker: Arc<ActiveSourceTracker>, db: Arc<Database>) -> Self {
        Self { tracker, db }
    }

    /// Create a source link connecting written content to active sources
    pub fn create_source_link(&self, input: &CreateSourceLinkInput) -> Result<SourceLinkWithSources, crate::database::DbError> {
        let now = chrono::Utc::now().timestamp_millis();

        // Get active sources
        let active_sources = self.tracker.get_active_sources(Some(MIN_RELEVANCE_THRESHOLD));

        if active_sources.is_empty() {
            // Create a source link with no linked sources (orphan content)
            let link = SourceLink {
                id: uuid::Uuid::new_v4().to_string(),
                doc_id: input.doc_id.clone(),
                section_path: input.section_path.clone(),
                content_hash: hash_content(&input.content),
                content_preview: Some(generate_preview(&input.content, PREVIEW_LENGTH)),
                created_at: now,
                confidence_score: 0.0,
            };

            self.db.create_source_link(&link)?;

            return Ok(SourceLinkWithSources {
                link,
                sources: vec![],
            });
        }

        // Calculate confidence score based on average relevance
        let confidence_score: f32 = active_sources.iter().map(|s| s.relevance).sum::<f32>()
            / active_sources.len() as f32;

        // Create source link
        let link = SourceLink {
            id: uuid::Uuid::new_v4().to_string(),
            doc_id: input.doc_id.clone(),
            section_path: input.section_path.clone(),
            content_hash: hash_content(&input.content),
            content_preview: Some(generate_preview(&input.content, PREVIEW_LENGTH)),
            created_at: now,
            confidence_score,
        };

        self.db.create_source_link(&link)?;

        // Create linked sources
        let linked_sources: Vec<LinkedSource> = active_sources
            .iter()
            .map(|active| {
                LinkedSource {
                    id: uuid::Uuid::new_v4().to_string(),
                    link_id: link.id.clone(),
                    source_type: active.source_type.clone(),
                    source_id: active.source_id.clone(),
                    contribution_type: infer_contribution_type(&active.source_type, active.relevance),
                    timestamp: now,
                }
            })
            .collect();

        self.db.add_linked_sources(&linked_sources)?;

        Ok(SourceLinkWithSources {
            link,
            sources: linked_sources,
        })
    }

    /// Get document provenance (all source links for a document)
    pub fn get_document_provenance(&self, doc_id: &str) -> Result<Vec<SourceLinkWithSources>, crate::database::DbError> {
        self.db.get_document_source_links(doc_id)
    }
}

/// Infer contribution type based on source type and relevance
fn infer_contribution_type(source_type: &SourceType, relevance: f32) -> ContributionType {
    match source_type {
        SourceType::Clipboard => {
            if relevance > 0.8 {
                ContributionType::DirectCopy
            } else {
                ContributionType::Referenced
            }
        }
        SourceType::AIExchange => ContributionType::AIAssisted,
        SourceType::Webpage | SourceType::Search | SourceType::Document => {
            if relevance > 0.7 {
                ContributionType::Referenced
            } else {
                ContributionType::Inspired
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_activate_source() {
        let tracker = ActiveSourceTracker::new();

        tracker.activate_source(
            SourceType::Webpage,
            "page1".to_string(),
            Some("Page Title".to_string()),
        );

        assert_eq!(tracker.count(), 1);

        let sources = tracker.get_active_sources(None);
        assert_eq!(sources.len(), 1);
        assert_eq!(sources[0].source_id, "page1");
        assert!(sources[0].relevance > 0.99); // Should be close to 1.0
    }

    #[test]
    fn test_refresh_source() {
        let tracker = ActiveSourceTracker::new();

        tracker.activate_source(
            SourceType::Webpage,
            "page1".to_string(),
            Some("Page Title".to_string()),
        );

        // Simulate time passing
        std::thread::sleep(std::time::Duration::from_millis(100));

        // Refresh the source
        tracker.activate_source(
            SourceType::Webpage,
            "page1".to_string(),
            Some("Updated Title".to_string()),
        );

        assert_eq!(tracker.count(), 1); // Still just one source

        let sources = tracker.get_active_sources(None);
        assert!(sources[0].relevance > 0.99); // Relevance reset to 1.0
    }

    #[test]
    fn test_deactivate_source() {
        let tracker = ActiveSourceTracker::new();

        tracker.activate_source(
            SourceType::Webpage,
            "page1".to_string(),
            None,
        );

        tracker.deactivate_source("page1");

        assert_eq!(tracker.count(), 0);
    }

    #[test]
    fn test_infer_contribution_type() {
        assert!(matches!(
            infer_contribution_type(&SourceType::Clipboard, 0.9),
            ContributionType::DirectCopy
        ));

        assert!(matches!(
            infer_contribution_type(&SourceType::AIExchange, 0.5),
            ContributionType::AIAssisted
        ));

        assert!(matches!(
            infer_contribution_type(&SourceType::Webpage, 0.5),
            ContributionType::Inspired
        ));
    }
}
