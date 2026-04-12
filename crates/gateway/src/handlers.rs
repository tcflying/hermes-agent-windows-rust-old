use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{Json, sse::{Event, Sse}, IntoResponse},
    routing::{get, post},
    Router,
};
use anyhow::Result;
use futures_util::TryStreamExt;
use hermes_agent::chat::{self, Message};
use hermes_config::{ConfigLoader, ConfigUpdate};
use hermes_session::{SessionDb, SessionInfo, SessionMessage};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio_stream::wrappers::UnboundedReceiverStream;
use std::path::PathBuf;
use std::fs;
use tower_http::cors::{CorsLayer, Any};

const DEFAULT_API_URL: &str = "https://api.minimaxi.com/v1";

#[derive(Clone)]
pub struct AppState {
    pub session_db: Arc<Mutex<SessionDb>>,
    pub config: Arc<Mutex<ConfigLoader>>,
}

#[derive(Debug, Deserialize)]
pub struct ChatRequest {
    pub model: String,
    pub messages: Vec<ChatMessage>,
    pub api_url: Option<String>,
    pub api_key: Option<String>,
    pub session_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Serialize)]
pub struct ChatResponse {
    pub content: String,
    pub session_id: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: String,
}

pub fn create_router(state: AppState) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    Router::new()
        .route("/health", get(health))
        .route("/api/chat", post(chat))
        .route("/api/chat/stream", post(chat_stream))
        .route("/api/sessions", get(list_sessions).post(create_session))
        .route("/api/sessions/{id}", get(get_session).delete(delete_session))
        .route("/api/sessions/{id}/messages", get(get_session_messages))
        .route("/api/tools", get(list_tools))
        .route("/api/files/list", post(list_dir))
        .route("/api/files/read", post(read_file))
        .route("/api/files/write", post(write_file))
        .route("/api/terminal", post(exec_terminal))
        .route("/api/config", get(get_config).put(update_config))
        .layer(cors)
        .with_state(state)
}

pub async fn start_server(port: u16) -> Result<()> {
    let db_path = std::env::var("HERMES_DB")
        .map(|p| std::path::PathBuf::from(p))
        .unwrap_or_else(|_| {
            dirs::home_dir()
                .unwrap_or_else(|| std::path::PathBuf::from("."))
                .join(".hermes")
                .join("sessions.db")
        });

    let session_db = SessionDb::new(db_path)
        .map_err(|e| anyhow::anyhow!("Failed to open session DB: {}", e))?;

    let config_path = std::env::var("HERMES_CONFIG")
        .map(|p| std::path::PathBuf::from(p))
        .unwrap_or_else(|_| {
            dirs::home_dir()
                .unwrap_or_else(|| std::path::PathBuf::from("."))
                .join(".hermes")
                .join("config.yaml")
        });

    let mut config_loader = ConfigLoader::new();
    if let Err(e) = config_loader.load(config_path) {
        eprintln!("Warning: Failed to load config: {}", e);
    }

    let state = AppState {
        session_db: Arc::new(Mutex::new(session_db)),
        config: Arc::new(Mutex::new(config_loader)),
    };

    let app = create_router(state);

    let addr = format!("0.0.0.0:{}", port);
    println!("Gateway server starting on http://{}", addr);
    
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

async fn health() -> &'static str {
    "OK"
}

async fn chat(
    State(state): State<AppState>,
    Json(req): Json<ChatRequest>,
) -> Result<Json<ChatResponse>, (StatusCode, Json<ErrorResponse>)> {
    let config = state.config.lock().await.get().clone();
    let api_url = req.api_url
        .or_else(|| std::env::var("MINIMAX_API_URL").ok())
        .unwrap_or_else(|| config.api_url.clone());
    let api_key = req.api_key
        .or_else(|| std::env::var("MINIMAX_API_KEY").ok())
        .unwrap_or_else(|| config.api_key.clone());
    let model = req.model.clone();

    let session_id = if let Some(ref sid) = req.session_id {
        sid.clone()
    } else {
        let new_session = state.session_db.lock().await.create_session(Some(model.clone())).await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse { error: e.to_string() })))?;
        new_session.id
    };

    for msg in &req.messages {
        let role = if msg.role == "user" { "user" } else if msg.role == "assistant" { "assistant" } else { &msg.role };
        state.session_db.lock().await.save_message(&session_id, role, &msg.content)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse { error: e.to_string() })))?;
    }

    let messages: Vec<Message> = req
        .messages
        .into_iter()
        .map(|m| Message {
            role: m.role,
            content: Some(m.content),
            tool_calls: None,
            tool_call_id: None,
            name: None,
        })
        .collect();

    match chat::run_conversation(&model, &api_url, &api_key, messages).await {
        Ok(response) => {
            state.session_db.lock().await.save_message(&session_id, "assistant", &response.content)
                .await
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse { error: e.to_string() })))?;
            Ok(Json(ChatResponse {
                content: response.content,
                session_id: Some(session_id),
            }))
        }
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse {
            error: e.to_string(),
        }))),
    }
}

async fn chat_stream(
    State(state): State<AppState>,
    Json(req): Json<ChatRequest>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    let config = state.config.lock().await.get().clone();
    let api_url = req.api_url
        .or_else(|| std::env::var("MINIMAX_API_URL").ok())
        .unwrap_or_else(|| config.api_url.clone());
    let api_key = req.api_key
        .or_else(|| std::env::var("MINIMAX_API_KEY").ok())
        .unwrap_or_else(|| config.api_key.clone());
    let model = req.model.clone();

    let session_id = match req.session_id {
        Some(ref sid) => sid.clone(),
        None => {
            let new_session = state.session_db.lock().await.create_session(Some(model.clone()))
                .await
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse { error: e.to_string() })))?;
            new_session.id
        }
    };

    for msg in &req.messages {
        let role = if msg.role == "user" { "user" } else if msg.role == "assistant" { "assistant" } else { &msg.role };
        state.session_db.lock().await.save_message(&session_id, role, &msg.content)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse { error: e.to_string() })))?;
    }

    let messages: Vec<Message> = req
        .messages
        .into_iter()
        .map(|m| Message {
            role: m.role,
            content: Some(m.content),
            tool_calls: None,
            tool_call_id: None,
            name: None,
        })
        .collect();

    let (tx, rx) = tokio::sync::mpsc::unbounded_channel();

    let session_id_clone = session_id.clone();
    let state_clone = state.clone();

    tokio::spawn(async move {
        eprintln!("[stream] Starting run_conversation for session {}", session_id_clone);
        let result = chat::run_conversation(&model, &api_url, &api_key, messages).await;

        match result {
            Ok(response) => {
                let full_content = response.content;
                let total_len = full_content.len();
                eprintln!("[stream] Got response, content_len={}, session={}", total_len, session_id_clone);

                tx.send(Ok(Event::default().data(format!(r#"{{"session_id":"{}"}}"#, session_id_clone)))).ok();

                let chars: Vec<char> = full_content.chars().collect();
                let total_chars = chars.len();
                let chars_per_chunk = 8.max(total_chars / 40);
                let mut pos = 0;
                while pos < total_chars {
                    let end = (pos + chars_per_chunk).min(total_chars);
                    let chunk: String = chars[pos..end].iter().collect();
                    let event_data = serde_json::json!({
                        "content": chunk,
                        "done": false
                    });
                    if let Ok(event) = serde_json::to_string(&event_data) {
                        tx.send(Ok(Event::default().data(event))).ok();
                    }
                    pos = end;
                }

                if !full_content.is_empty() {
                    state_clone.session_db.lock().await
                        .save_message(&session_id_clone, "assistant", &full_content)
                        .await
                        .ok();
                }

                tx.send(Ok(Event::default().data(r#"{"done":true}"#))).ok();
            }
            Err(e) => {
                eprintln!("[stream] ERROR in run_conversation for session {}: {}", session_id_clone, e);
                let error_data = serde_json::json!({
                    "error": e.to_string()
                });
                if let Ok(event) = serde_json::to_string(&error_data) {
                    tx.send(Ok(Event::default().data(event))).ok();
                }
                tx.send(Ok(Event::default().data(r#"{"done":true}"#))).ok();
            }
        }
    });

    let stream = UnboundedReceiverStream::new(rx).map_err(|e: anyhow::Error| anyhow::anyhow!("Channel error: {}", e));
    Ok(Sse::new(stream))
}

async fn list_sessions(
    State(state): State<AppState>,
) -> Result<Json<Vec<SessionInfo>>, (StatusCode, Json<ErrorResponse>)> {
    match state.session_db.lock().await.list_sessions().await {
        Ok(sessions) => Ok(Json(sessions)),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse {
            error: e.to_string(),
        }))),
    }
}

async fn create_session(
    State(state): State<AppState>,
    Json(req): Json<CreateSessionRequest>,
) -> Result<Json<SessionInfo>, (StatusCode, Json<ErrorResponse>)> {
    match state.session_db.lock().await.create_session(req.model).await {
        Ok(session) => Ok(Json(session)),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse {
            error: e.to_string(),
        }))),
    }
}

async fn get_session(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<SessionInfo>, (StatusCode, Json<ErrorResponse>)> {
    match state.session_db.lock().await.get_session(&id).await {
        Ok(Some(s)) => Ok(Json(SessionInfo {
            id: s.id,
            created_at: s.created_at,
            updated_at: s.updated_at,
            model: s.model,
        })),
        Ok(None) => Err((StatusCode::NOT_FOUND, Json(ErrorResponse {
            error: "Session not found".to_string(),
        }))),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse {
            error: e.to_string(),
        }))),
    }
}

async fn get_session_messages(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Vec<SessionMessage>>, (StatusCode, Json<ErrorResponse>)> {
    match state.session_db.lock().await.get_messages(&id).await {
        Ok(messages) => Ok(Json(messages)),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse {
            error: e.to_string(),
        }))),
    }
}

async fn list_tools() -> Json<Vec<ToolInfo>> {
    Json(vec![
        ToolInfo {
            name: "terminal".to_string(),
            description: "Execute terminal commands".to_string(),
        },
        ToolInfo {
            name: "file_read".to_string(),
            description: "Read files from the filesystem".to_string(),
        },
        ToolInfo {
            name: "file_write".to_string(),
            description: "Write content to files".to_string(),
        },
        ToolInfo {
            name: "web_search".to_string(),
            description: "Search the web for information".to_string(),
        },
        ToolInfo {
            name: "browser_navigate".to_string(),
            description: "Navigate and interact with a browser".to_string(),
        },
        ToolInfo {
            name: "code_execute".to_string(),
            description: "Execute code in a sandboxed environment".to_string(),
        },
    ])
}

async fn delete_session(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<()>, (StatusCode, Json<ErrorResponse>)> {
    match state.session_db.lock().await.delete_session(&id).await {
        Ok(_) => Ok(Json(())),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse {
            error: e.to_string(),
        }))),
    }
}

#[derive(Debug, Deserialize)]
struct ListDirRequest {
    path: String,
}

#[derive(Debug, Serialize)]
struct FileEntry {
    name: String,
    path: String,
    is_dir: bool,
    size: u64,
}

async fn list_dir(
    Json(req): Json<ListDirRequest>,
) -> Result<Json<Vec<FileEntry>>, (StatusCode, Json<ErrorResponse>)> {
    let path = PathBuf::from(&req.path);
    
    match fs::read_dir(&path) {
        Ok(entries) => {
            let mut files = Vec::new();
            for entry in entries.flatten() {
                if let Ok(metadata) = entry.metadata() {
                    files.push(FileEntry {
                        name: entry.file_name().to_string_lossy().to_string(),
                        path: entry.path().to_string_lossy().to_string(),
                        is_dir: metadata.is_dir(),
                        size: metadata.len(),
                    });
                }
            }
            files.sort_by(|a, b| {
                match (a.is_dir, b.is_dir) {
                    (true, false) => std::cmp::Ordering::Less,
                    (false, true) => std::cmp::Ordering::Greater,
                    _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
                }
            });
            Ok(Json(files))
        }
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse {
            error: format!("Failed to read directory: {}", e),
        }))),
    }
}

#[derive(Debug, Deserialize)]
struct ReadFileRequest {
    path: String,
}

#[derive(Debug, Serialize)]
struct ReadFileResponse {
    content: String,
    encoding: String,
}

async fn read_file(
    Json(req): Json<ReadFileRequest>,
) -> Result<Json<ReadFileResponse>, (StatusCode, Json<ErrorResponse>)> {
    let path = PathBuf::from(&req.path);
    
    match fs::read_to_string(&path) {
        Ok(content) => Ok(Json(ReadFileResponse {
            content,
            encoding: "utf-8".to_string(),
        })),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse {
            error: format!("Failed to read file: {}", e),
        }))),
    }
}

#[derive(Debug, Deserialize)]
struct WriteFileRequest {
    path: String,
    content: String,
}

async fn write_file(
    Json(req): Json<WriteFileRequest>,
) -> Result<Json<()>, (StatusCode, Json<ErrorResponse>)> {
    let path = PathBuf::from(&req.path);
    
    if let Some(parent) = path.parent() {
        if !parent.exists() {
            if let Err(e) = fs::create_dir_all(parent) {
                return Err((StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse {
                    error: format!("Failed to create directory: {}", e),
                })));
            }
        }
    }
    
    match fs::write(&path, &req.content) {
        Ok(_) => Ok(Json(())),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse {
            error: format!("Failed to write file: {}", e),
        }))),
    }
}

#[derive(Debug, Deserialize)]
struct TerminalRequest {
    command: String,
    cwd: Option<String>,
}

#[derive(Debug, Serialize)]
struct TerminalResponse {
    output: String,
    exit_code: i32,
}

async fn exec_terminal(
    Json(req): Json<TerminalRequest>,
) -> Result<Json<TerminalResponse>, (StatusCode, Json<ErrorResponse>)> {
    let cwd = req.cwd.as_ref().map(PathBuf::from);
    
    let output = if cfg!(windows) {
        let mut cmd = std::process::Command::new("cmd");
        cmd.args(["/C", &req.command]);
        if let Some(ref dir) = cwd {
            cmd.current_dir(dir);
        }
        cmd.output()
    } else {
        let mut cmd = std::process::Command::new("sh");
        cmd.args(["-c", &req.command]);
        if let Some(ref dir) = cwd {
            cmd.current_dir(dir);
        }
        cmd.output()
    };
    
    match output {
        Ok(out) => {
            let stdout = String::from_utf8_lossy(&out.stdout).to_string();
            let stderr = String::from_utf8_lossy(&out.stderr).to_string();
            let exit_code = out.status.code().unwrap_or(-1);
            Ok(Json(TerminalResponse {
                output: if stderr.is_empty() { stdout } else { format!("{}\n{}", stdout, stderr) },
                exit_code,
            }))
        }
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse {
            error: format!("Failed to execute command: {}", e),
        }))),
    }
}

async fn get_config(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let config = state.config.lock().await.get().clone();
    Ok(Json(serde_json::json!({
        "model": config.model,
        "provider": config.provider,
        "api_url": config.api_url,
        "api_key": config.api_key,
        "skin": config.display.skin,
    })))
}

async fn update_config(
    State(state): State<AppState>,
    Json(update): Json<ConfigUpdate>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    if let Err(e) = state.config.lock().await.update(update) {
        return Err((StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse {
            error: e.to_string(),
        })));
    }
    let config = state.config.lock().await.get().clone();
    Ok(Json(serde_json::json!({
        "model": config.model,
        "provider": config.provider,
        "api_url": config.api_url,
        "api_key": config.api_key,
        "skin": config.display.skin,
    })))
}

#[derive(Debug, Deserialize)]
struct CreateSessionRequest {
    model: Option<String>,
}

#[derive(Debug, Serialize)]
struct ToolInfo {
    name: String,
    description: String,
}