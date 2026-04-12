use anyhow::Result;
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct SessionDb {
    conn: Arc<Mutex<Connection>>,
}

impl SessionDb {
    pub fn new(path: PathBuf) -> Result<Self> {
        let conn = Connection::open(path)?;
        conn.execute_batch(
            "PRAGMA journal_mode = WAL;
             CREATE TABLE IF NOT EXISTS sessions (
                 id TEXT PRIMARY KEY,
                 created_at TEXT NOT NULL,
                 updated_at TEXT NOT NULL,
                 model TEXT,
                 messages TEXT NOT NULL
             );
             CREATE TABLE IF NOT EXISTS messages (
                 id INTEGER PRIMARY KEY AUTOINCREMENT,
                 session_id TEXT NOT NULL,
                 role TEXT NOT NULL,
                 content TEXT NOT NULL,
                 created_at TEXT NOT NULL
             );",
        )?;
        Ok(Self { conn: Arc::new(Mutex::new(conn)) })
    }

    pub async fn save_message(&self, session_id: &str, role: &str, content: &str) -> Result<()> {
        let conn = self.conn.lock().await;
        conn.execute(
            "INSERT INTO messages (session_id, role, content, created_at) VALUES (?1, ?2, ?3, datetime('now'))",
            params![session_id, role, content],
        )?;
        Ok(())
    }

    pub async fn get_session(&self, session_id: &str) -> Result<Option<Session>> {
        let conn = self.conn.lock().await;
        let mut stmt = conn.prepare(
            "SELECT id, created_at, updated_at, model FROM sessions WHERE id = ?1",
        )?;
        let session = stmt
            .query_row(params![session_id], |row| {
                Ok(Session {
                    id: row.get(0)?,
                    created_at: row.get(1)?,
                    updated_at: row.get(2)?,
                    model: row.get(3)?,
                })
            })
            .optional()?;
        Ok(session)
    }

    pub async fn list_sessions(&self) -> Result<Vec<SessionInfo>> {
        let conn = self.conn.lock().await;
        let mut stmt = conn.prepare(
            "SELECT id, created_at, updated_at, model FROM sessions ORDER BY updated_at DESC",
        )?;
        let sessions = stmt.query_map([], |row| {
            Ok(SessionInfo {
                id: row.get(0)?,
                created_at: row.get(1)?,
                updated_at: row.get(2)?,
                model: row.get::<_, Option<String>>(3)?,
            })
        })?;
        let mut result = Vec::new();
        for session in sessions {
            result.push(session?);
        }
        Ok(result)
    }

    pub async fn create_session(&self, model: Option<String>) -> Result<SessionInfo> {
        let id = uuid::Uuid::new_v4().to_string();
        let now = chrono::Utc::now().to_rfc3339();
        let conn = self.conn.lock().await;
        conn.execute(
            "INSERT INTO sessions (id, created_at, updated_at, model, messages) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![id, now, now, model, "[]"],
        )?;
        Ok(SessionInfo {
            id,
            created_at: now.clone(),
            updated_at: now,
            model,
        })
    }

    pub async fn get_messages(&self, session_id: &str) -> Result<Vec<SessionMessage>> {
        let conn = self.conn.lock().await;
        let mut stmt = conn.prepare(
            "SELECT role, content FROM messages WHERE session_id = ?1 ORDER BY id ASC",
        )?;
        let rows = stmt.query_map(params![session_id], |row| {
            Ok(SessionMessage {
                role: row.get(0)?,
                content: row.get(1)?,
            })
        })?;
        let mut result = Vec::new();
        for row in rows {
            result.push(row?);
        }
        Ok(result)
    }

    pub async fn delete_session(&self, session_id: &str) -> Result<()> {
        let conn = self.conn.lock().await;
        conn.execute("DELETE FROM messages WHERE session_id = ?1", params![session_id])?;
        conn.execute("DELETE FROM sessions WHERE id = ?1", params![session_id])?;
        Ok(())
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Session {
    pub id: String,
    pub created_at: String,
    pub updated_at: String,
    pub model: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionInfo {
    pub id: String,
    pub created_at: String,
    pub updated_at: String,
    pub model: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionMessage {
    pub role: String,
    pub content: String,
}
