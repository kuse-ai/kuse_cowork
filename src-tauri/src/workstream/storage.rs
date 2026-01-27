use crate::database::{Database, DbError};
use crate::workstream::types::*;

/// TTL for work blocks (7 days)
const WORK_BLOCK_TTL_MS: i64 = 7 * 24 * 60 * 60 * 1000;

/// TTL for sessions (30 days)
const SESSION_TTL_MS: i64 = 30 * 24 * 60 * 60 * 1000;

impl Database {
    /// Create the workstream tables
    pub fn create_workstream_tables(&self) -> Result<(), DbError> {
        let conn = self.conn.lock().map_err(|_| DbError::Lock)?;

        // Work blocks table
        conn.execute(
            "CREATE TABLE IF NOT EXISTS work_blocks (
                id TEXT PRIMARY KEY,
                session_id TEXT,
                context_type TEXT NOT NULL,
                context_id TEXT,
                context_title TEXT,
                started_at INTEGER NOT NULL,
                ended_at INTEGER NOT NULL,
                duration_secs INTEGER NOT NULL,

                auto_summary TEXT,
                edit_count INTEGER DEFAULT 0,
                browse_count INTEGER DEFAULT 0,
                research_urls TEXT,

                user_summary TEXT,
                notes TEXT,
                tags TEXT,
                is_pinned INTEGER DEFAULT 0,
                is_manual INTEGER DEFAULT 0,

                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL
            )",
            [],
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_work_blocks_time ON work_blocks(started_at DESC)",
            [],
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_work_blocks_context ON work_blocks(context_type, context_id)",
            [],
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_work_blocks_pinned ON work_blocks(is_pinned)",
            [],
        )?;

        // Sessions table
        conn.execute(
            "CREATE TABLE IF NOT EXISTS workstream_sessions (
                id TEXT PRIMARY KEY,
                started_at INTEGER NOT NULL,
                ended_at INTEGER,
                summary TEXT,
                block_count INTEGER DEFAULT 0,
                total_duration_secs INTEGER DEFAULT 0
            )",
            [],
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_sessions_time ON workstream_sessions(started_at DESC)",
            [],
        )?;

        // Milestones table (permanent)
        conn.execute(
            "CREATE TABLE IF NOT EXISTS milestones (
                id TEXT PRIMARY KEY,
                context_type TEXT NOT NULL,
                context_id TEXT NOT NULL,
                milestone_type TEXT NOT NULL,
                timestamp INTEGER NOT NULL,
                note TEXT
            )",
            [],
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_milestones_context ON milestones(context_type, context_id)",
            [],
        )?;

        Ok(())
    }

    // ==================== Work Block Methods ====================

    /// Create a work block from buffer flush
    pub fn create_ws_block(&self, input: &CreateBlockInput) -> Result<WorkBlock, DbError> {
        let conn = self.conn.lock().map_err(|_| DbError::Lock)?;
        let id = uuid::Uuid::new_v4().to_string();
        let now = chrono::Utc::now().timestamp_millis();
        let duration_secs = ((input.ended_at - input.started_at) / 1000) as i32;

        let research_urls_json = serde_json::to_string(&input.research_urls).unwrap_or("[]".to_string());

        conn.execute(
            "INSERT INTO work_blocks (
                id, session_id, context_type, context_id, context_title,
                started_at, ended_at, duration_secs,
                auto_summary, edit_count, browse_count, research_urls,
                user_summary, notes, tags, is_pinned, is_manual,
                created_at, updated_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, NULL, NULL, '[]', 0, 0, ?13, ?13)",
            rusqlite::params![
                id,
                Option::<String>::None, // session_id (TODO: implement sessions)
                input.context_type,
                input.context_id,
                input.context_title,
                input.started_at,
                input.ended_at,
                duration_secs,
                input.auto_summary,
                input.edit_count,
                input.browse_count,
                research_urls_json,
                now,
            ],
        )?;

        Ok(WorkBlock {
            id,
            session_id: None,
            context_type: ContextType::from_str(&input.context_type),
            context_id: input.context_id.clone(),
            context_title: input.context_title.clone(),
            started_at: input.started_at,
            ended_at: input.ended_at,
            duration_secs,
            auto_summary: input.auto_summary.clone(),
            edit_count: input.edit_count,
            browse_count: input.browse_count,
            research_urls: input.research_urls.clone(),
            user_summary: None,
            notes: None,
            tags: vec![],
            is_pinned: false,
            is_manual: false,
            created_at: now,
            updated_at: now,
        })
    }

    /// Create a manual work block (user-entered)
    pub fn create_manual_block(&self, input: &ManualBlockInput) -> Result<WorkBlock, DbError> {
        let conn = self.conn.lock().map_err(|_| DbError::Lock)?;
        let id = uuid::Uuid::new_v4().to_string();
        let now = chrono::Utc::now().timestamp_millis();
        let duration_secs = ((input.ended_at - input.started_at) / 1000) as i32;
        let context_type = input.context_type.as_deref().unwrap_or("manual");
        let tags_json = serde_json::to_string(&input.tags.clone().unwrap_or_default()).unwrap_or("[]".to_string());

        conn.execute(
            "INSERT INTO work_blocks (
                id, session_id, context_type, context_id, context_title,
                started_at, ended_at, duration_secs,
                auto_summary, edit_count, browse_count, research_urls,
                user_summary, notes, tags, is_pinned, is_manual,
                created_at, updated_at
            ) VALUES (?1, NULL, ?2, ?3, ?4, ?5, ?6, ?7, NULL, 0, 0, '[]', ?8, ?9, ?10, 0, 1, ?11, ?11)",
            rusqlite::params![
                id,
                context_type,
                input.context_id,
                input.context_title,
                input.started_at,
                input.ended_at,
                duration_secs,
                input.user_summary,
                input.notes,
                tags_json,
                now,
            ],
        )?;

        Ok(WorkBlock {
            id,
            session_id: None,
            context_type: ContextType::from_str(context_type),
            context_id: input.context_id.clone(),
            context_title: input.context_title.clone(),
            started_at: input.started_at,
            ended_at: input.ended_at,
            duration_secs,
            auto_summary: None,
            edit_count: 0,
            browse_count: 0,
            research_urls: vec![],
            user_summary: Some(input.user_summary.clone()),
            notes: input.notes.clone(),
            tags: input.tags.clone().unwrap_or_default(),
            is_pinned: false,
            is_manual: true,
            created_at: now,
            updated_at: now,
        })
    }

    /// Update a work block (user edits)
    pub fn update_work_block(&self, id: &str, input: &UpdateBlockInput) -> Result<Option<WorkBlock>, DbError> {
        let conn = self.conn.lock().map_err(|_| DbError::Lock)?;
        let now = chrono::Utc::now().timestamp_millis();

        // Build update query dynamically
        let mut updates = vec!["updated_at = ?1".to_string()];
        let mut param_idx = 2;

        if input.user_summary.is_some() {
            updates.push(format!("user_summary = ?{}", param_idx));
            param_idx += 1;
        }
        if input.notes.is_some() {
            updates.push(format!("notes = ?{}", param_idx));
            param_idx += 1;
        }
        if input.tags.is_some() {
            updates.push(format!("tags = ?{}", param_idx));
            param_idx += 1;
        }
        if input.is_pinned.is_some() {
            updates.push(format!("is_pinned = ?{}", param_idx));
            // param_idx += 1; // unused after this
        }

        let sql = format!(
            "UPDATE work_blocks SET {} WHERE id = ?{}",
            updates.join(", "),
            param_idx
        );

        // Build params
        let mut params: Vec<Box<dyn rusqlite::ToSql>> = vec![Box::new(now)];

        if let Some(ref summary) = input.user_summary {
            params.push(Box::new(summary.clone()));
        }
        if let Some(ref notes) = input.notes {
            params.push(Box::new(notes.clone()));
        }
        if let Some(ref tags) = input.tags {
            params.push(Box::new(serde_json::to_string(tags).unwrap_or("[]".to_string())));
        }
        if let Some(pinned) = input.is_pinned {
            params.push(Box::new(if pinned { 1 } else { 0 }));
        }
        params.push(Box::new(id.to_string()));

        let param_refs: Vec<&dyn rusqlite::ToSql> = params.iter().map(|p| p.as_ref()).collect();
        conn.execute(&sql, param_refs.as_slice())?;

        drop(conn);
        self.get_work_block_by_id(id)
    }

    /// Get a work block by ID
    pub fn get_work_block_by_id(&self, id: &str) -> Result<Option<WorkBlock>, DbError> {
        let conn = self.conn.lock().map_err(|_| DbError::Lock)?;

        let mut stmt = conn.prepare(
            "SELECT id, session_id, context_type, context_id, context_title,
                    started_at, ended_at, duration_secs,
                    auto_summary, edit_count, browse_count, research_urls,
                    user_summary, notes, tags, is_pinned, is_manual,
                    created_at, updated_at
             FROM work_blocks WHERE id = ?1",
        )?;

        let mut rows = stmt.query([id])?;

        if let Some(row) = rows.next()? {
            Ok(Some(row_to_work_block(row)?))
        } else {
            Ok(None)
        }
    }

    /// List work blocks with filtering
    pub fn list_work_blocks(&self, query: &WorkBlockQuery) -> Result<Vec<WorkBlock>, DbError> {
        let conn = self.conn.lock().map_err(|_| DbError::Lock)?;

        let mut sql = String::from(
            "SELECT id, session_id, context_type, context_id, context_title,
                    started_at, ended_at, duration_secs,
                    auto_summary, edit_count, browse_count, research_urls,
                    user_summary, notes, tags, is_pinned, is_manual,
                    created_at, updated_at
             FROM work_blocks WHERE 1=1"
        );
        let mut params: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

        if let Some(ref ctx_type) = query.context_type {
            sql.push_str(&format!(" AND context_type = ?{}", params.len() + 1));
            params.push(Box::new(ctx_type.clone()));
        }

        if let Some(ref ctx_id) = query.context_id {
            sql.push_str(&format!(" AND context_id = ?{}", params.len() + 1));
            params.push(Box::new(ctx_id.clone()));
        }

        if let Some(from_ts) = query.from_timestamp {
            sql.push_str(&format!(" AND started_at >= ?{}", params.len() + 1));
            params.push(Box::new(from_ts));
        }

        if let Some(to_ts) = query.to_timestamp {
            sql.push_str(&format!(" AND started_at <= ?{}", params.len() + 1));
            params.push(Box::new(to_ts));
        }

        if query.include_pinned_only {
            sql.push_str(" AND is_pinned = 1");
        }

        sql.push_str(" ORDER BY started_at DESC");

        if let Some(limit) = query.limit {
            sql.push_str(&format!(" LIMIT {}", limit));
        }

        let mut stmt = conn.prepare(&sql)?;
        let param_refs: Vec<&dyn rusqlite::ToSql> = params.iter().map(|p| p.as_ref()).collect();
        let rows = stmt.query_map(param_refs.as_slice(), |row| row_to_work_block(row))?;

        let mut blocks = Vec::new();
        for row in rows {
            blocks.push(row?);
        }

        Ok(blocks)
    }

    /// Delete a work block
    pub fn delete_work_block(&self, id: &str) -> Result<bool, DbError> {
        let conn = self.conn.lock().map_err(|_| DbError::Lock)?;
        let count = conn.execute("DELETE FROM work_blocks WHERE id = ?1", [id])?;
        Ok(count > 0)
    }

    /// Get timeline (work blocks for today/recent)
    pub fn get_workstream_timeline(&self, limit: Option<i32>) -> Result<Timeline, DbError> {
        let blocks = self.list_work_blocks(&WorkBlockQuery {
            limit: Some(limit.unwrap_or(50)),
            ..Default::default()
        })?;

        let total_duration_secs = blocks.iter().map(|b| b.duration_secs).sum();

        Ok(Timeline {
            session: None, // TODO: implement current session
            blocks,
            total_duration_secs,
        })
    }

    // ==================== Cleanup Methods ====================

    /// Cleanup old data based on TTL
    /// - Work blocks: 7 days (unless pinned)
    /// - Sessions: 30 days
    pub fn cleanup_workstream(&self) -> Result<CleanupResult, DbError> {
        let conn = self.conn.lock().map_err(|_| DbError::Lock)?;
        let now = chrono::Utc::now().timestamp_millis();

        let work_block_cutoff = now - WORK_BLOCK_TTL_MS;
        let session_cutoff = now - SESSION_TTL_MS;

        // Delete old work blocks (not pinned, not manual)
        let blocks_deleted = conn.execute(
            "DELETE FROM work_blocks WHERE ended_at < ?1 AND is_pinned = 0",
            [work_block_cutoff],
        )?;

        // Delete old sessions
        let sessions_deleted = conn.execute(
            "DELETE FROM workstream_sessions WHERE ended_at < ?1",
            [session_cutoff],
        )?;

        Ok(CleanupResult {
            blocks_deleted: blocks_deleted as u32,
            sessions_deleted: sessions_deleted as u32,
        })
    }

    // ==================== Milestone Methods ====================

    /// Create a milestone
    pub fn create_milestone(&self, input: &MilestoneInput) -> Result<Milestone, DbError> {
        let conn = self.conn.lock().map_err(|_| DbError::Lock)?;
        let id = uuid::Uuid::new_v4().to_string();
        let now = chrono::Utc::now().timestamp_millis();

        conn.execute(
            "INSERT INTO milestones (id, context_type, context_id, milestone_type, timestamp, note)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            rusqlite::params![
                id,
                input.context_type,
                input.context_id,
                input.milestone_type,
                now,
                input.note,
            ],
        )?;

        Ok(Milestone {
            id,
            context_type: ContextType::from_str(&input.context_type),
            context_id: input.context_id.clone(),
            milestone_type: input.milestone_type.clone(),
            timestamp: now,
            note: input.note.clone(),
        })
    }

    /// List milestones for a context
    pub fn list_milestones(&self, context_type: Option<&str>, context_id: Option<&str>) -> Result<Vec<Milestone>, DbError> {
        let conn = self.conn.lock().map_err(|_| DbError::Lock)?;

        let mut sql = String::from(
            "SELECT id, context_type, context_id, milestone_type, timestamp, note
             FROM milestones WHERE 1=1"
        );
        let mut params: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

        if let Some(ctx_type) = context_type {
            sql.push_str(&format!(" AND context_type = ?{}", params.len() + 1));
            params.push(Box::new(ctx_type.to_string()));
        }

        if let Some(ctx_id) = context_id {
            sql.push_str(&format!(" AND context_id = ?{}", params.len() + 1));
            params.push(Box::new(ctx_id.to_string()));
        }

        sql.push_str(" ORDER BY timestamp DESC");

        let mut stmt = conn.prepare(&sql)?;
        let param_refs: Vec<&dyn rusqlite::ToSql> = params.iter().map(|p| p.as_ref()).collect();

        let rows = stmt.query_map(param_refs.as_slice(), |row| {
            Ok(Milestone {
                id: row.get(0)?,
                context_type: ContextType::from_str(&row.get::<_, String>(1)?),
                context_id: row.get(2)?,
                milestone_type: row.get(3)?,
                timestamp: row.get(4)?,
                note: row.get(5)?,
            })
        })?;

        let mut milestones = Vec::new();
        for row in rows {
            milestones.push(row?);
        }

        Ok(milestones)
    }
}

/// Helper to convert a row to WorkBlock
fn row_to_work_block(row: &rusqlite::Row) -> rusqlite::Result<WorkBlock> {
    let context_type_str: String = row.get(2)?;
    let research_urls_str: String = row.get(11)?;
    let tags_str: String = row.get(14)?;

    let research_urls: Vec<String> = serde_json::from_str(&research_urls_str).unwrap_or_default();
    let tags: Vec<String> = serde_json::from_str(&tags_str).unwrap_or_default();

    Ok(WorkBlock {
        id: row.get(0)?,
        session_id: row.get(1)?,
        context_type: ContextType::from_str(&context_type_str),
        context_id: row.get(3)?,
        context_title: row.get(4)?,
        started_at: row.get(5)?,
        ended_at: row.get(6)?,
        duration_secs: row.get(7)?,
        auto_summary: row.get(8)?,
        edit_count: row.get(9)?,
        browse_count: row.get(10)?,
        research_urls,
        user_summary: row.get(12)?,
        notes: row.get(13)?,
        tags,
        is_pinned: row.get::<_, i32>(15)? != 0,
        is_manual: row.get::<_, i32>(16)? != 0,
        created_at: row.get(17)?,
        updated_at: row.get(18)?,
    })
}

/// Result of cleanup operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CleanupResult {
    pub blocks_deleted: u32,
    pub sessions_deleted: u32,
}

use serde::{Deserialize, Serialize};
