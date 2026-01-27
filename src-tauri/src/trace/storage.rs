use crate::database::{Database, DbError};
use crate::trace::types::{Trace, TraceInput, TraceSettings, Suggestion};

impl Database {
    /// Create the trace-related tables
    pub fn create_trace_tables(&self) -> Result<(), DbError> {
        let conn = self.conn.lock().map_err(|_| DbError::Lock)?;

        // Traces table
        conn.execute(
            "CREATE TABLE IF NOT EXISTS traces (
                id TEXT PRIMARY KEY,
                task_id TEXT,
                doc_id TEXT NOT NULL,
                event_type TEXT NOT NULL,
                section_path TEXT,
                delta INTEGER,
                payload TEXT NOT NULL DEFAULT '{}',
                created_at INTEGER NOT NULL,
                FOREIGN KEY (task_id) REFERENCES tasks(id) ON DELETE CASCADE
            )",
            [],
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_traces_doc_id ON traces(doc_id)",
            [],
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_traces_task_id ON traces(task_id)",
            [],
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_traces_created_at ON traces(created_at DESC)",
            [],
        )?;

        // Trace settings table
        conn.execute(
            "CREATE TABLE IF NOT EXISTS trace_settings (
                doc_id TEXT PRIMARY KEY,
                tracing_enabled INTEGER NOT NULL DEFAULT 1,
                include_snippets INTEGER NOT NULL DEFAULT 1
            )",
            [],
        )?;

        // Suggestions table
        conn.execute(
            "CREATE TABLE IF NOT EXISTS suggestions (
                id TEXT PRIMARY KEY,
                doc_id TEXT NOT NULL,
                suggestion_type TEXT NOT NULL,
                title TEXT NOT NULL,
                description TEXT NOT NULL,
                payload TEXT NOT NULL DEFAULT '{}',
                status TEXT NOT NULL DEFAULT 'pending',
                created_at INTEGER NOT NULL
            )",
            [],
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_suggestions_doc_id ON suggestions(doc_id)",
            [],
        )?;

        Ok(())
    }

    /// Log a new trace event
    pub fn log_trace(&self, input: &TraceInput) -> Result<Trace, DbError> {
        let conn = self.conn.lock().map_err(|_| DbError::Lock)?;
        let id = uuid::Uuid::new_v4().to_string();
        let now = chrono::Utc::now().timestamp_millis();
        let payload_str = serde_json::to_string(&input.payload.clone().unwrap_or(serde_json::json!({})))
            .unwrap_or_else(|_| "{}".to_string());

        conn.execute(
            "INSERT INTO traces (id, task_id, doc_id, event_type, section_path, delta, payload, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            rusqlite::params![
                id,
                input.task_id,
                input.doc_id,
                input.event_type,
                input.section_path,
                input.delta,
                payload_str,
                now
            ],
        )?;

        Ok(Trace {
            id,
            task_id: input.task_id.clone(),
            doc_id: input.doc_id.clone(),
            event_type: input.event_type.clone(),
            section_path: input.section_path.clone(),
            delta: input.delta,
            payload: input.payload.clone().unwrap_or(serde_json::json!({})),
            created_at: now,
        })
    }

    /// List traces for a document, optionally limited and with cursor-based pagination
    pub fn list_traces(
        &self,
        doc_id: &str,
        limit: Option<u32>,
        before_timestamp: Option<i64>,
    ) -> Result<Vec<Trace>, DbError> {
        let conn = self.conn.lock().map_err(|_| DbError::Lock)?;

        let limit_val = limit.unwrap_or(100);
        let before_ts = before_timestamp.unwrap_or(i64::MAX);

        let mut stmt = conn.prepare(
            "SELECT id, task_id, doc_id, event_type, section_path, delta, payload, created_at
             FROM traces
             WHERE doc_id = ?1 AND created_at < ?2
             ORDER BY created_at DESC
             LIMIT ?3",
        )?;

        let rows = stmt.query_map(rusqlite::params![doc_id, before_ts, limit_val], |row| {
            let payload_str: String = row.get(6)?;
            let payload: serde_json::Value =
                serde_json::from_str(&payload_str).unwrap_or(serde_json::json!({}));

            Ok(Trace {
                id: row.get(0)?,
                task_id: row.get(1)?,
                doc_id: row.get(2)?,
                event_type: row.get(3)?,
                section_path: row.get(4)?,
                delta: row.get(5)?,
                payload,
                created_at: row.get(7)?,
            })
        })?;

        let mut traces = Vec::new();
        for row in rows {
            traces.push(row?);
        }

        Ok(traces)
    }

    /// Delete a specific trace
    pub fn delete_trace(&self, id: &str) -> Result<(), DbError> {
        let conn = self.conn.lock().map_err(|_| DbError::Lock)?;
        conn.execute("DELETE FROM traces WHERE id = ?1", [id])?;
        Ok(())
    }

    /// Clear all traces for a document
    pub fn clear_traces(&self, doc_id: &str) -> Result<u64, DbError> {
        let conn = self.conn.lock().map_err(|_| DbError::Lock)?;
        let deleted = conn.execute("DELETE FROM traces WHERE doc_id = ?1", [doc_id])?;
        Ok(deleted as u64)
    }

    /// Get trace settings for a document
    pub fn get_trace_settings(&self, doc_id: &str) -> Result<TraceSettings, DbError> {
        let conn = self.conn.lock().map_err(|_| DbError::Lock)?;

        let mut stmt = conn.prepare(
            "SELECT doc_id, tracing_enabled, include_snippets FROM trace_settings WHERE doc_id = ?1",
        )?;

        let mut rows = stmt.query([doc_id])?;

        if let Some(row) = rows.next()? {
            Ok(TraceSettings {
                doc_id: row.get(0)?,
                tracing_enabled: row.get::<_, i32>(1)? != 0,
                include_snippets: row.get::<_, i32>(2)? != 0,
            })
        } else {
            // Return default settings
            Ok(TraceSettings {
                doc_id: Some(doc_id.to_string()),
                ..Default::default()
            })
        }
    }

    /// Save trace settings for a document
    pub fn save_trace_settings(&self, doc_id: &str, settings: &TraceSettings) -> Result<(), DbError> {
        let conn = self.conn.lock().map_err(|_| DbError::Lock)?;

        conn.execute(
            "INSERT OR REPLACE INTO trace_settings (doc_id, tracing_enabled, include_snippets)
             VALUES (?1, ?2, ?3)",
            rusqlite::params![
                doc_id,
                settings.tracing_enabled as i32,
                settings.include_snippets as i32
            ],
        )?;

        Ok(())
    }

    /// Save a suggestion
    pub fn save_suggestion(&self, doc_id: &str, suggestion: &Suggestion) -> Result<Suggestion, DbError> {
        let conn = self.conn.lock().map_err(|_| DbError::Lock)?;
        let id = if suggestion.id.is_empty() {
            uuid::Uuid::new_v4().to_string()
        } else {
            suggestion.id.clone()
        };
        let now = chrono::Utc::now().timestamp_millis();
        let payload_str = serde_json::to_string(&suggestion.payload)
            .unwrap_or_else(|_| "{}".to_string());

        conn.execute(
            "INSERT OR REPLACE INTO suggestions (id, doc_id, suggestion_type, title, description, payload, status, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            rusqlite::params![
                id,
                doc_id,
                suggestion.suggestion_type,
                suggestion.title,
                suggestion.description,
                payload_str,
                suggestion.status,
                now
            ],
        )?;

        Ok(Suggestion {
            id,
            suggestion_type: suggestion.suggestion_type.clone(),
            title: suggestion.title.clone(),
            description: suggestion.description.clone(),
            payload: suggestion.payload.clone(),
            status: suggestion.status.clone(),
            created_at: now,
        })
    }

    /// List suggestions for a document
    pub fn list_suggestions(&self, doc_id: &str, status: Option<&str>) -> Result<Vec<Suggestion>, DbError> {
        let conn = self.conn.lock().map_err(|_| DbError::Lock)?;

        let query = if let Some(status_filter) = status {
            format!(
                "SELECT id, suggestion_type, title, description, payload, status, created_at
                 FROM suggestions
                 WHERE doc_id = ?1 AND status = '{}'
                 ORDER BY created_at DESC",
                status_filter
            )
        } else {
            "SELECT id, suggestion_type, title, description, payload, status, created_at
             FROM suggestions
             WHERE doc_id = ?1
             ORDER BY created_at DESC".to_string()
        };

        let mut stmt = conn.prepare(&query)?;
        let rows = stmt.query_map([doc_id], |row| {
            let payload_str: String = row.get(4)?;
            let payload: serde_json::Value =
                serde_json::from_str(&payload_str).unwrap_or(serde_json::json!({}));

            Ok(Suggestion {
                id: row.get(0)?,
                suggestion_type: row.get(1)?,
                title: row.get(2)?,
                description: row.get(3)?,
                payload,
                status: row.get(5)?,
                created_at: row.get(6)?,
            })
        })?;

        let mut suggestions = Vec::new();
        for row in rows {
            suggestions.push(row?);
        }

        Ok(suggestions)
    }

    /// Update suggestion status
    pub fn update_suggestion_status(&self, id: &str, status: &str) -> Result<(), DbError> {
        let conn = self.conn.lock().map_err(|_| DbError::Lock)?;
        conn.execute(
            "UPDATE suggestions SET status = ?1 WHERE id = ?2",
            [status, id],
        )?;
        Ok(())
    }

    /// Delete a suggestion
    pub fn delete_suggestion(&self, id: &str) -> Result<(), DbError> {
        let conn = self.conn.lock().map_err(|_| DbError::Lock)?;
        conn.execute("DELETE FROM suggestions WHERE id = ?1", [id])?;
        Ok(())
    }
}
