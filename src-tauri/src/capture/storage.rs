use crate::database::{Database, DbError};
use crate::capture::types::{
    ClipboardCapture, BrowseCapture, SearchCapture, AIExchangeCapture, DocEditCapture,
    SourceLink, LinkedSource, SourceLinkWithSources, CaptureConfig,
    SourceType, ContributionType,
};
use serde::{Deserialize, Serialize};

/// TTL for content store entries (30 days) if access_count < 3
const CONTENT_TTL_MS: i64 = 30 * 24 * 60 * 60 * 1000;
/// Minimum access count to keep content beyond TTL
const MIN_ACCESS_COUNT_TO_KEEP: i32 = 3;

impl Database {
    /// Create the capture tables
    pub fn create_capture_tables(&self) -> Result<(), DbError> {
        let conn = self.conn.lock().map_err(|_| DbError::Lock)?;

        // Content store for deduplication (hash -> full content)
        conn.execute(
            "CREATE TABLE IF NOT EXISTS content_store (
                hash TEXT PRIMARY KEY,
                content TEXT NOT NULL,
                content_type TEXT NOT NULL,
                byte_size INTEGER NOT NULL,
                created_at INTEGER NOT NULL,
                last_accessed_at INTEGER NOT NULL,
                access_count INTEGER DEFAULT 1
            )",
            [],
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_content_store_accessed
             ON content_store(last_accessed_at)",
            [],
        )?;

        // Clipboard captures
        conn.execute(
            "CREATE TABLE IF NOT EXISTS clipboard_captures (
                id TEXT PRIMARY KEY,
                content_hash TEXT NOT NULL,
                content_preview TEXT NOT NULL,
                source_url TEXT,
                source_title TEXT,
                captured_at INTEGER NOT NULL
            )",
            [],
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_clipboard_time
             ON clipboard_captures(captured_at DESC)",
            [],
        )?;

        // Browse captures
        conn.execute(
            "CREATE TABLE IF NOT EXISTS browse_captures (
                id TEXT PRIMARY KEY,
                url TEXT NOT NULL,
                page_title TEXT,
                entered_at INTEGER NOT NULL,
                left_at INTEGER,
                scroll_depth_percent INTEGER
            )",
            [],
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_browse_time
             ON browse_captures(entered_at DESC)",
            [],
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_browse_url
             ON browse_captures(url)",
            [],
        )?;

        // Search captures
        conn.execute(
            "CREATE TABLE IF NOT EXISTS search_captures (
                id TEXT PRIMARY KEY,
                query TEXT NOT NULL,
                search_engine TEXT NOT NULL,
                result_clicked TEXT,
                timestamp INTEGER NOT NULL
            )",
            [],
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_search_time
             ON search_captures(timestamp DESC)",
            [],
        )?;

        // AI exchange captures
        conn.execute(
            "CREATE TABLE IF NOT EXISTS ai_exchange_captures (
                id TEXT PRIMARY KEY,
                question_hash TEXT NOT NULL,
                question_preview TEXT NOT NULL,
                answer_hash TEXT NOT NULL,
                answer_preview TEXT NOT NULL,
                model TEXT NOT NULL,
                context_doc_id TEXT,
                timestamp INTEGER NOT NULL
            )",
            [],
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_ai_exchange_time
             ON ai_exchange_captures(timestamp DESC)",
            [],
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_ai_exchange_doc
             ON ai_exchange_captures(context_doc_id)",
            [],
        )?;

        // Document edit captures
        conn.execute(
            "CREATE TABLE IF NOT EXISTS doc_edit_captures (
                id TEXT PRIMARY KEY,
                doc_id TEXT NOT NULL,
                doc_title TEXT NOT NULL,
                edit_preview TEXT NOT NULL,
                char_delta INTEGER NOT NULL,
                started_at INTEGER NOT NULL,
                ended_at INTEGER NOT NULL
            )",
            [],
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_doc_edit_time
             ON doc_edit_captures(ended_at DESC)",
            [],
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_doc_edit_doc
             ON doc_edit_captures(doc_id)",
            [],
        )?;

        // Source links
        conn.execute(
            "CREATE TABLE IF NOT EXISTS source_links (
                id TEXT PRIMARY KEY,
                doc_id TEXT NOT NULL,
                section_path TEXT,
                content_hash TEXT NOT NULL,
                content_preview TEXT,
                created_at INTEGER NOT NULL,
                confidence_score REAL NOT NULL
            )",
            [],
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_source_links_doc
             ON source_links(doc_id)",
            [],
        )?;

        // Linked sources (many-to-one with source_links)
        conn.execute(
            "CREATE TABLE IF NOT EXISTS linked_sources (
                id TEXT PRIMARY KEY,
                link_id TEXT NOT NULL,
                source_type TEXT NOT NULL,
                source_id TEXT NOT NULL,
                contribution_type TEXT NOT NULL,
                timestamp INTEGER NOT NULL,
                FOREIGN KEY (link_id) REFERENCES source_links(id) ON DELETE CASCADE
            )",
            [],
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_linked_sources_link
             ON linked_sources(link_id)",
            [],
        )?;

        // Capture config
        conn.execute(
            "CREATE TABLE IF NOT EXISTS capture_config (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            )",
            [],
        )?;

        Ok(())
    }

    // ==================== Content Store Methods ====================

    /// Store content if not exists, or update access info
    pub fn store_content(&self, hash: &str, content: &str, content_type: &str) -> Result<(), DbError> {
        let conn = self.conn.lock().map_err(|_| DbError::Lock)?;
        let now = chrono::Utc::now().timestamp_millis();
        let byte_size = content.len() as i64;

        // Try to update existing, otherwise insert
        let updated = conn.execute(
            "UPDATE content_store SET last_accessed_at = ?1, access_count = access_count + 1
             WHERE hash = ?2",
            rusqlite::params![now, hash],
        )?;

        if updated == 0 {
            conn.execute(
                "INSERT INTO content_store (hash, content, content_type, byte_size, created_at, last_accessed_at, access_count)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?5, 1)",
                rusqlite::params![hash, content, content_type, byte_size, now],
            )?;
        }

        Ok(())
    }

    /// Get content by hash
    pub fn get_content(&self, hash: &str) -> Result<Option<String>, DbError> {
        let conn = self.conn.lock().map_err(|_| DbError::Lock)?;
        let now = chrono::Utc::now().timestamp_millis();

        // Update access time and count
        conn.execute(
            "UPDATE content_store SET last_accessed_at = ?1, access_count = access_count + 1
             WHERE hash = ?2",
            rusqlite::params![now, hash],
        )?;

        let mut stmt = conn.prepare("SELECT content FROM content_store WHERE hash = ?1")?;
        let mut rows = stmt.query([hash])?;

        if let Some(row) = rows.next()? {
            Ok(Some(row.get(0)?))
        } else {
            Ok(None)
        }
    }

    // ==================== Clipboard Capture Methods ====================

    /// Insert clipboard captures in batch
    pub fn insert_clipboard_captures(&self, captures: &[ClipboardCapture]) -> Result<usize, DbError> {
        if captures.is_empty() {
            return Ok(0);
        }

        let conn = self.conn.lock().map_err(|_| DbError::Lock)?;
        let mut count = 0;

        for capture in captures {
            conn.execute(
                "INSERT OR IGNORE INTO clipboard_captures
                 (id, content_hash, content_preview, source_url, source_title, captured_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                rusqlite::params![
                    capture.id,
                    capture.content_hash,
                    capture.content_preview,
                    capture.source_url,
                    capture.source_title,
                    capture.captured_at,
                ],
            )?;
            count += 1;
        }

        Ok(count)
    }

    /// Get recent clipboard captures
    pub fn get_recent_clipboard(&self, limit: i32) -> Result<Vec<ClipboardCapture>, DbError> {
        let conn = self.conn.lock().map_err(|_| DbError::Lock)?;

        let mut stmt = conn.prepare(
            "SELECT id, content_hash, content_preview, source_url, source_title, captured_at
             FROM clipboard_captures
             ORDER BY captured_at DESC
             LIMIT ?1"
        )?;

        let rows = stmt.query_map([limit], |row| {
            Ok(ClipboardCapture {
                id: row.get(0)?,
                content_hash: row.get(1)?,
                content_preview: row.get(2)?,
                source_url: row.get(3)?,
                source_title: row.get(4)?,
                captured_at: row.get(5)?,
            })
        })?;

        let mut captures = Vec::new();
        for row in rows {
            captures.push(row?);
        }

        Ok(captures)
    }

    // ==================== Browse Capture Methods ====================

    /// Insert browse captures in batch
    pub fn insert_browse_captures(&self, captures: &[BrowseCapture]) -> Result<usize, DbError> {
        if captures.is_empty() {
            return Ok(0);
        }

        let conn = self.conn.lock().map_err(|_| DbError::Lock)?;
        let mut count = 0;

        for capture in captures {
            conn.execute(
                "INSERT OR REPLACE INTO browse_captures
                 (id, url, page_title, entered_at, left_at, scroll_depth_percent)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                rusqlite::params![
                    capture.id,
                    capture.url,
                    capture.page_title,
                    capture.entered_at,
                    capture.left_at,
                    capture.scroll_depth_percent.map(|v| v as i32),
                ],
            )?;
            count += 1;
        }

        Ok(count)
    }

    /// Get recent browse captures
    pub fn get_recent_browse(&self, limit: i32) -> Result<Vec<BrowseCapture>, DbError> {
        let conn = self.conn.lock().map_err(|_| DbError::Lock)?;

        let mut stmt = conn.prepare(
            "SELECT id, url, page_title, entered_at, left_at, scroll_depth_percent
             FROM browse_captures
             ORDER BY entered_at DESC
             LIMIT ?1"
        )?;

        let rows = stmt.query_map([limit], |row| {
            Ok(BrowseCapture {
                id: row.get(0)?,
                url: row.get(1)?,
                page_title: row.get(2)?,
                entered_at: row.get(3)?,
                left_at: row.get(4)?,
                scroll_depth_percent: row.get::<_, Option<i32>>(5)?.map(|v| v as u8),
            })
        })?;

        let mut captures = Vec::new();
        for row in rows {
            captures.push(row?);
        }

        Ok(captures)
    }

    // ==================== Search Capture Methods ====================

    /// Insert search captures in batch
    pub fn insert_search_captures(&self, captures: &[SearchCapture]) -> Result<usize, DbError> {
        if captures.is_empty() {
            return Ok(0);
        }

        let conn = self.conn.lock().map_err(|_| DbError::Lock)?;
        let mut count = 0;

        for capture in captures {
            conn.execute(
                "INSERT OR REPLACE INTO search_captures
                 (id, query, search_engine, result_clicked, timestamp)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                rusqlite::params![
                    capture.id,
                    capture.query,
                    capture.search_engine,
                    capture.result_clicked,
                    capture.timestamp,
                ],
            )?;
            count += 1;
        }

        Ok(count)
    }

    /// Get recent search captures
    pub fn get_recent_search(&self, limit: i32) -> Result<Vec<SearchCapture>, DbError> {
        let conn = self.conn.lock().map_err(|_| DbError::Lock)?;

        let mut stmt = conn.prepare(
            "SELECT id, query, search_engine, result_clicked, timestamp
             FROM search_captures
             ORDER BY timestamp DESC
             LIMIT ?1"
        )?;

        let rows = stmt.query_map([limit], |row| {
            Ok(SearchCapture {
                id: row.get(0)?,
                query: row.get(1)?,
                search_engine: row.get(2)?,
                result_clicked: row.get(3)?,
                timestamp: row.get(4)?,
            })
        })?;

        let mut captures = Vec::new();
        for row in rows {
            captures.push(row?);
        }

        Ok(captures)
    }

    // ==================== AI Exchange Capture Methods ====================

    /// Insert AI exchange captures in batch
    pub fn insert_ai_exchange_captures(&self, captures: &[AIExchangeCapture]) -> Result<usize, DbError> {
        if captures.is_empty() {
            return Ok(0);
        }

        let conn = self.conn.lock().map_err(|_| DbError::Lock)?;
        let mut count = 0;

        for capture in captures {
            conn.execute(
                "INSERT OR IGNORE INTO ai_exchange_captures
                 (id, question_hash, question_preview, answer_hash, answer_preview, model, context_doc_id, timestamp)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                rusqlite::params![
                    capture.id,
                    capture.question_hash,
                    capture.question_preview,
                    capture.answer_hash,
                    capture.answer_preview,
                    capture.model,
                    capture.context_doc_id,
                    capture.timestamp,
                ],
            )?;
            count += 1;
        }

        Ok(count)
    }

    /// Get recent AI exchange captures
    pub fn get_recent_ai_exchange(&self, limit: i32) -> Result<Vec<AIExchangeCapture>, DbError> {
        let conn = self.conn.lock().map_err(|_| DbError::Lock)?;

        let mut stmt = conn.prepare(
            "SELECT id, question_hash, question_preview, answer_hash, answer_preview, model, context_doc_id, timestamp
             FROM ai_exchange_captures
             ORDER BY timestamp DESC
             LIMIT ?1"
        )?;

        let rows = stmt.query_map([limit], |row| {
            Ok(AIExchangeCapture {
                id: row.get(0)?,
                question_hash: row.get(1)?,
                question_preview: row.get(2)?,
                answer_hash: row.get(3)?,
                answer_preview: row.get(4)?,
                model: row.get(5)?,
                context_doc_id: row.get(6)?,
                timestamp: row.get(7)?,
            })
        })?;

        let mut captures = Vec::new();
        for row in rows {
            captures.push(row?);
        }

        Ok(captures)
    }

    // ==================== Document Edit Capture Methods ====================

    /// Insert document edit captures in batch
    pub fn insert_doc_edit_captures(&self, captures: &[DocEditCapture]) -> Result<usize, DbError> {
        if captures.is_empty() {
            return Ok(0);
        }

        let conn = self.conn.lock().map_err(|_| DbError::Lock)?;
        let mut count = 0;

        for capture in captures {
            conn.execute(
                "INSERT OR IGNORE INTO doc_edit_captures
                 (id, doc_id, doc_title, edit_preview, char_delta, started_at, ended_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                rusqlite::params![
                    capture.id,
                    capture.doc_id,
                    capture.doc_title,
                    capture.edit_preview,
                    capture.char_delta,
                    capture.started_at,
                    capture.ended_at,
                ],
            )?;
            count += 1;
        }

        Ok(count)
    }

    /// Get recent document edit captures
    pub fn get_recent_doc_edit(&self, limit: i32) -> Result<Vec<DocEditCapture>, DbError> {
        let conn = self.conn.lock().map_err(|_| DbError::Lock)?;

        let mut stmt = conn.prepare(
            "SELECT id, doc_id, doc_title, edit_preview, char_delta, started_at, ended_at
             FROM doc_edit_captures
             ORDER BY ended_at DESC
             LIMIT ?1"
        )?;

        let rows = stmt.query_map([limit], |row| {
            Ok(DocEditCapture {
                id: row.get(0)?,
                doc_id: row.get(1)?,
                doc_title: row.get(2)?,
                edit_preview: row.get(3)?,
                char_delta: row.get(4)?,
                started_at: row.get(5)?,
                ended_at: row.get(6)?,
            })
        })?;

        let mut captures = Vec::new();
        for row in rows {
            captures.push(row?);
        }

        Ok(captures)
    }

    // ==================== Source Link Methods ====================

    /// Create a source link
    pub fn create_source_link(&self, link: &SourceLink) -> Result<(), DbError> {
        let conn = self.conn.lock().map_err(|_| DbError::Lock)?;

        conn.execute(
            "INSERT INTO source_links (id, doc_id, section_path, content_hash, content_preview, created_at, confidence_score)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            rusqlite::params![
                link.id,
                link.doc_id,
                link.section_path,
                link.content_hash,
                link.content_preview,
                link.created_at,
                link.confidence_score,
            ],
        )?;

        Ok(())
    }

    /// Add linked sources to a source link
    pub fn add_linked_sources(&self, sources: &[LinkedSource]) -> Result<(), DbError> {
        if sources.is_empty() {
            return Ok(());
        }

        let conn = self.conn.lock().map_err(|_| DbError::Lock)?;

        for source in sources {
            conn.execute(
                "INSERT INTO linked_sources (id, link_id, source_type, source_id, contribution_type, timestamp)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                rusqlite::params![
                    source.id,
                    source.link_id,
                    source.source_type.as_str(),
                    source.source_id,
                    source.contribution_type.as_str(),
                    source.timestamp,
                ],
            )?;
        }

        Ok(())
    }

    /// Get source links for a document
    pub fn get_document_source_links(&self, doc_id: &str) -> Result<Vec<SourceLinkWithSources>, DbError> {
        let conn = self.conn.lock().map_err(|_| DbError::Lock)?;

        // Get source links
        let mut stmt = conn.prepare(
            "SELECT id, doc_id, section_path, content_hash, content_preview, created_at, confidence_score
             FROM source_links
             WHERE doc_id = ?1
             ORDER BY created_at DESC"
        )?;

        let link_rows = stmt.query_map([doc_id], |row| {
            Ok(SourceLink {
                id: row.get(0)?,
                doc_id: row.get(1)?,
                section_path: row.get(2)?,
                content_hash: row.get(3)?,
                content_preview: row.get(4)?,
                created_at: row.get(5)?,
                confidence_score: row.get(6)?,
            })
        })?;

        let mut links: Vec<SourceLink> = Vec::new();
        for row in link_rows {
            links.push(row?);
        }

        // Get linked sources for each link
        let mut result = Vec::new();
        for link in links {
            let mut source_stmt = conn.prepare(
                "SELECT id, link_id, source_type, source_id, contribution_type, timestamp
                 FROM linked_sources
                 WHERE link_id = ?1"
            )?;

            let source_rows = source_stmt.query_map([&link.id], |row| {
                Ok(LinkedSource {
                    id: row.get(0)?,
                    link_id: row.get(1)?,
                    source_type: SourceType::from_str(&row.get::<_, String>(2)?),
                    source_id: row.get(3)?,
                    contribution_type: ContributionType::from_str(&row.get::<_, String>(4)?),
                    timestamp: row.get(5)?,
                })
            })?;

            let mut sources = Vec::new();
            for row in source_rows {
                sources.push(row?);
            }

            result.push(SourceLinkWithSources { link, sources });
        }

        Ok(result)
    }

    // ==================== Config Methods ====================

    /// Get capture config
    pub fn get_capture_config(&self) -> Result<CaptureConfig, DbError> {
        let conn = self.conn.lock().map_err(|_| DbError::Lock)?;
        let mut config = CaptureConfig::default();

        let mut stmt = conn.prepare("SELECT key, value FROM capture_config")?;
        let rows = stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?;

        for row in rows {
            let (key, value) = row?;
            match key.as_str() {
                "clipboard_enabled" => config.clipboard_enabled = value == "true",
                "browse_enabled" => config.browse_enabled = value == "true",
                "search_enabled" => config.search_enabled = value == "true",
                "ai_exchange_enabled" => config.ai_exchange_enabled = value == "true",
                "source_linking_enabled" => config.source_linking_enabled = value == "true",
                "flush_interval_secs" => config.flush_interval_secs = value.parse().unwrap_or(30),
                "clipboard_poll_ms" => config.clipboard_poll_ms = value.parse().unwrap_or(500),
                _ => {}
            }
        }

        Ok(config)
    }

    /// Save capture config
    pub fn save_capture_config(&self, config: &CaptureConfig) -> Result<(), DbError> {
        let conn = self.conn.lock().map_err(|_| DbError::Lock)?;

        let pairs = [
            ("clipboard_enabled", config.clipboard_enabled.to_string()),
            ("browse_enabled", config.browse_enabled.to_string()),
            ("search_enabled", config.search_enabled.to_string()),
            ("ai_exchange_enabled", config.ai_exchange_enabled.to_string()),
            ("source_linking_enabled", config.source_linking_enabled.to_string()),
            ("flush_interval_secs", config.flush_interval_secs.to_string()),
            ("clipboard_poll_ms", config.clipboard_poll_ms.to_string()),
        ];

        for (key, value) in pairs {
            conn.execute(
                "INSERT OR REPLACE INTO capture_config (key, value) VALUES (?1, ?2)",
                [key, &value],
            )?;
        }

        Ok(())
    }

    // ==================== Cleanup Methods ====================

    /// Clear all capture data
    pub fn clear_all_captures(&self) -> Result<CaptureCleanupResult, DbError> {
        let conn = self.conn.lock().map_err(|_| DbError::Lock)?;

        let clipboard_deleted = conn.execute("DELETE FROM clipboard_captures", [])?;
        let browse_deleted = conn.execute("DELETE FROM browse_captures", [])?;
        let search_deleted = conn.execute("DELETE FROM search_captures", [])?;
        let ai_deleted = conn.execute("DELETE FROM ai_exchange_captures", [])?;
        let doc_edit_deleted = conn.execute("DELETE FROM doc_edit_captures", [])?;

        // Also clear content store
        conn.execute("DELETE FROM content_store", [])?;

        Ok(CaptureCleanupResult {
            content_entries_deleted: (clipboard_deleted + browse_deleted + search_deleted + ai_deleted + doc_edit_deleted) as u32,
        })
    }

    /// Cleanup old content store entries
    pub fn cleanup_capture_content(&self) -> Result<CaptureCleanupResult, DbError> {
        let conn = self.conn.lock().map_err(|_| DbError::Lock)?;
        let now = chrono::Utc::now().timestamp_millis();
        let cutoff = now - CONTENT_TTL_MS;

        // Delete old content with low access count
        let content_deleted = conn.execute(
            "DELETE FROM content_store WHERE last_accessed_at < ?1 AND access_count < ?2",
            rusqlite::params![cutoff, MIN_ACCESS_COUNT_TO_KEEP],
        )?;

        Ok(CaptureCleanupResult {
            content_entries_deleted: content_deleted as u32,
        })
    }
}

/// Result of capture cleanup operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaptureCleanupResult {
    pub content_entries_deleted: u32,
}
