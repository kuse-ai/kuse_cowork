use super::types::{Document, CreateDocumentInput, UpdateDocumentInput};
use crate::database::{Database, DbError};

impl Database {
    /// Create the documents table
    pub fn create_docs_table(&self) -> Result<(), DbError> {
        let conn = self.conn.lock().map_err(|_| DbError::Lock)?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS documents (
                id TEXT PRIMARY KEY,
                title TEXT NOT NULL,
                content TEXT NOT NULL DEFAULT '',
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL
            )",
            [],
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_documents_updated_at
             ON documents(updated_at DESC)",
            [],
        )?;

        Ok(())
    }

    /// Create a new document
    pub fn create_document(&self, input: &CreateDocumentInput) -> Result<Document, DbError> {
        let conn = self.conn.lock().map_err(|_| DbError::Lock)?;
        let now = chrono::Utc::now().timestamp_millis();
        let id = uuid::Uuid::new_v4().to_string();
        let content = input.content.clone().unwrap_or_default();

        conn.execute(
            "INSERT INTO documents (id, title, content, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![id, input.title, content, now, now],
        )?;

        Ok(Document {
            id,
            title: input.title.clone(),
            content,
            created_at: now,
            updated_at: now,
        })
    }

    /// Get a document by ID
    pub fn get_document(&self, id: &str) -> Result<Option<Document>, DbError> {
        let conn = self.conn.lock().map_err(|_| DbError::Lock)?;

        let mut stmt = conn.prepare(
            "SELECT id, title, content, created_at, updated_at
             FROM documents WHERE id = ?1"
        )?;

        let mut rows = stmt.query([id])?;

        if let Some(row) = rows.next()? {
            Ok(Some(Document {
                id: row.get(0)?,
                title: row.get(1)?,
                content: row.get(2)?,
                created_at: row.get(3)?,
                updated_at: row.get(4)?,
            }))
        } else {
            Ok(None)
        }
    }

    /// Update a document
    pub fn update_document(&self, id: &str, input: &UpdateDocumentInput) -> Result<Option<Document>, DbError> {
        let conn = self.conn.lock().map_err(|_| DbError::Lock)?;
        let now = chrono::Utc::now().timestamp_millis();

        // Build dynamic update query
        let mut updates = Vec::new();
        let mut params: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();
        let mut param_idx = 1;

        if let Some(title) = &input.title {
            updates.push(format!("title = ?{}", param_idx));
            params.push(Box::new(title.clone()));
            param_idx += 1;
        }

        if let Some(content) = &input.content {
            updates.push(format!("content = ?{}", param_idx));
            params.push(Box::new(content.clone()));
            param_idx += 1;
        }

        if updates.is_empty() {
            // Nothing to update, just return the existing document
            drop(conn);
            return self.get_document(id);
        }

        updates.push(format!("updated_at = ?{}", param_idx));
        params.push(Box::new(now));
        param_idx += 1;

        let sql = format!(
            "UPDATE documents SET {} WHERE id = ?{}",
            updates.join(", "),
            param_idx
        );
        params.push(Box::new(id.to_string()));

        let params_refs: Vec<&dyn rusqlite::ToSql> = params.iter().map(|p| p.as_ref()).collect();
        let affected = conn.execute(&sql, params_refs.as_slice())?;

        if affected == 0 {
            return Ok(None);
        }

        // Fetch and return the updated document
        let mut stmt = conn.prepare(
            "SELECT id, title, content, created_at, updated_at
             FROM documents WHERE id = ?1"
        )?;

        let mut rows = stmt.query([id])?;

        if let Some(row) = rows.next()? {
            Ok(Some(Document {
                id: row.get(0)?,
                title: row.get(1)?,
                content: row.get(2)?,
                created_at: row.get(3)?,
                updated_at: row.get(4)?,
            }))
        } else {
            Ok(None)
        }
    }

    /// List all documents, ordered by updated_at desc
    pub fn list_documents(&self) -> Result<Vec<Document>, DbError> {
        let conn = self.conn.lock().map_err(|_| DbError::Lock)?;

        let mut stmt = conn.prepare(
            "SELECT id, title, content, created_at, updated_at
             FROM documents
             ORDER BY updated_at DESC"
        )?;

        let rows = stmt.query_map([], |row| {
            Ok(Document {
                id: row.get(0)?,
                title: row.get(1)?,
                content: row.get(2)?,
                created_at: row.get(3)?,
                updated_at: row.get(4)?,
            })
        })?;

        let mut documents = Vec::new();
        for row in rows {
            documents.push(row?);
        }

        Ok(documents)
    }

    /// Delete a document by ID
    pub fn delete_document(&self, id: &str) -> Result<bool, DbError> {
        let conn = self.conn.lock().map_err(|_| DbError::Lock)?;

        let affected = conn.execute("DELETE FROM documents WHERE id = ?1", [id])?;

        Ok(affected > 0)
    }
}
