use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::sync::Mutex;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LogEntry {
    pub timestamp: String,
    pub level: String,
    pub target: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
}

pub struct LogBuffer {
    entries: Vec<LogEntry>,
    max_entries: usize,
    file_path: Option<PathBuf>,
}

const MAX_FILE_SIZE: u64 = 10 * 1024 * 1024;
const MAX_ROTATED: usize = 5;

impl LogBuffer {
    pub fn new(max_entries: usize) -> Self {
        let file_path =
            dirs::home_dir().map(|h| h.join(".hermes").join("logs").join("agent.jsonl"));
        if let Some(ref path) = file_path {
            if let Some(parent) = path.parent() {
                let _ = fs::create_dir_all(parent);
            }
        }
        Self {
            entries: Vec::new(),
            max_entries,
            file_path,
        }
    }

    pub fn push(&mut self, entry: LogEntry) {
        if self.entries.len() >= self.max_entries {
            self.entries.remove(0);
        }
        self.entries.push(entry);
    }

    pub fn query(
        &self,
        level: Option<&str>,
        target: Option<&str>,
        limit: Option<usize>,
        since: Option<&str>,
    ) -> Vec<&LogEntry> {
        let mut results: Vec<&LogEntry> = self.entries.iter().collect();
        if let Some(lvl) = level {
            results.retain(|e| e.level == lvl);
        }
        if let Some(tgt) = target {
            results.retain(|e| e.target == tgt);
        }
        if let Some(s) = since {
            results.retain(|e| e.timestamp.as_str() >= s);
        }
        if let Some(lim) = limit {
            let start = results.len().saturating_sub(lim);
            results = results[start..].to_vec();
        }
        results
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn write_to_file(&self) -> std::io::Result<()> {
        let path = match &self.file_path {
            Some(p) => p,
            None => return Ok(()),
        };
        let mut f = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)?;
        for entry in &self.entries {
            let line = serde_json::to_string(entry).unwrap_or_default();
            writeln!(f, "{}", line)?;
        }
        Ok(())
    }

    pub fn rotate_if_needed(&mut self) -> std::io::Result<()> {
        let path = match &self.file_path.clone() {
            Some(p) => p.clone(),
            None => return Ok(()),
        };
        let metadata = match fs::metadata(&path) {
            Ok(m) => m,
            Err(_) => return Ok(()),
        };
        if metadata.len() < MAX_FILE_SIZE {
            return Ok(());
        }
        for i in (1..MAX_ROTATED).rev() {
            let old = path.with_extension(format!("jsonl.{}", i));
            let new = path.with_extension(format!("jsonl.{}", i + 1));
            if old.exists() {
                let _ = fs::rename(&old, &new);
            }
        }
        let _ = fs::rename(&path, path.with_extension("jsonl.1"));
        Ok(())
    }
}

pub fn log_agent(
    buffer: &Mutex<LogBuffer>,
    level: &str,
    target: &str,
    message: &str,
    model: Option<String>,
    session_id: Option<String>,
) {
    let entry = LogEntry {
        timestamp: chrono::Utc::now().to_rfc3339(),
        level: level.to_string(),
        target: target.to_string(),
        message: message.to_string(),
        model,
        session_id,
        metadata: None,
    };
    if let Ok(mut buf) = buffer.lock() {
        buf.push(entry);
    }
    eprintln!("[{}] [{}] {}", level, target, message);
}
