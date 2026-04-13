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
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio_stream::wrappers::UnboundedReceiverStream;
use std::path::PathBuf;
use std::fs;
use tower_http::cors::{CorsLayer, Any};

const DEFAULT_API_URL: &str = "https://api.minimaxi.com/v1";

#[derive(Clone)]
pub struct AppState {
    pub session_db: Arc<RwLock<SessionDb>>,
    pub config: Arc<RwLock<ConfigLoader>>,
    pub interrupt_flag: Arc<AtomicBool>,
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
        .route("/api/chat/interrupt", post(chat_interrupt))
        .route("/api/config", get(get_config).put(update_config))
        .route("/api/memory/read", get(memory_read))
        .route("/api/memory/action", post(memory_action))
        .layer(cors)
        .with_state(state)
}

pub async fn start_server(port: u16) -> Result<()> {
    print!("\x1b[33m╔══════════════════════════════════════╗\x1b[0m\n");
    print!("\x1b[33m║     Hermes Agent Gateway v0.1.0      ║\x1b[0m\n");
    print!("\x1b[33m╚══════════════════════════════════════╝\x1b[0m\n");
    print!("\x1b[36m  Listening on http://0.0.0.0:{}\x1b[0m\n", port);
    print!("\x1b[36m  Endpoints: /health /api/chat /api/chat/stream\x1b[0m\n");
    print!("\x1b[36m  Sessions:  /api/sessions /api/sessions/{{id}}\x1b[0m\n");
    print!("\x1b[36m  Tools:     /api/terminal /api/files/* /api/tools\x1b[0m\n");
    print!("\x1b[36m  Memory:    /api/memory/read /api/memory/action\x1b[0m\n");
    print!("\n");

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
    print!("\x1b[32m  ✓ Session DB loaded\x1b[0m\n");

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
        print!("\x1b[33m  ⚠ Config not loaded: {}\x1b[0m\n", e);
    } else {
        print!("\x1b[32m  ✓ Config loaded\x1b[0m\n");
    }

    let has_key = std::env::var("MINIMAX_API_KEY").map(|k| !k.is_empty()).unwrap_or(false);
    if has_key {
        print!("\x1b[32m  ✓ API key set\x1b[0m\n");
    } else {
        print!("\x1b[31m  ✗ No MINIMAX_API_KEY env var!\x1b[0m\n");
    }
    print!("\n");

    let state = AppState {
        session_db: Arc::new(RwLock::new(session_db)),
        config: Arc::new(RwLock::new(config_loader)),
        interrupt_flag: Arc::new(AtomicBool::new(false)),
    };

    let app = create_router(state);

    let addr = format!("0.0.0.0:{}", port);
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
    let config = state.config.read().await.get().clone();
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
        let new_session = state.session_db.write().await.create_session(Some(model.clone())).await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse { error: e.to_string() })))?;
        new_session.id
    };
    let sid_short = &session_id[..8.min(session_id.len())];
    let user_msg = req.messages.last().map(|m| m.content.clone()).unwrap_or_default();
    let preview = if user_msg.len() > 80 { format!("{}...", &user_msg[..user_msg.ceil_char_boundary(80)]) } else { user_msg };
    print!("\x1b[35m[chat] POST /api/chat session={} msgs={} user=\"{}\"\x1b[0m\n", sid_short, req.messages.len(), preview);

    for msg in &req.messages {
        let role = if msg.role == "user" { "user" } else if msg.role == "assistant" { "assistant" } else { &msg.role };
        state.session_db.write().await.save_message(&session_id, role, &msg.content)
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

    match chat::run_conversation(&model, &api_url, &api_key, messages, Some(state.interrupt_flag.clone()), None).await {
        Ok(response) => {
            state.session_db.write().await.save_message(&session_id, "assistant", &response.content)
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
    let config = state.config.read().await.get().clone();
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
            let new_session = state.session_db.write().await.create_session(Some(model.clone()))
                .await
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse { error: e.to_string() })))?;
            new_session.id
        }
    };

    let sid_short = &session_id[..8.min(session_id.len())];
    let user_msg = req.messages.last().map(|m| m.content.clone()).unwrap_or_default();
    let preview = if user_msg.len() > 80 { format!("{}...", &user_msg[..user_msg.ceil_char_boundary(80)]) } else { user_msg };
    print!("\x1b[35m[stream] POST /api/chat/stream session={} msgs={} user=\"{}\"\x1b[0m\n", sid_short, req.messages.len(), preview);

    for msg in &req.messages {
        let role = if msg.role == "user" { "user" } else if msg.role == "assistant" { "assistant" } else { &msg.role };
        state.session_db.write().await.save_message(&session_id, role, &msg.content)
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

    state.interrupt_flag.store(false, Ordering::SeqCst);
    let interrupt_flag = state.interrupt_flag.clone();

    let session_id_clone = session_id.clone();
    let state_clone = state.clone();
    let sid_log = session_id[..8.min(session_id.len())].to_string();

    let (token_tx, mut token_rx) = tokio::sync::mpsc::channel::<String>(100);

    let tx_clone = tx.clone();
    let token_tx_clone = token_tx.clone();
    tokio::spawn(async move {
        let result = chat::run_conversation(&model, &api_url, &api_key, messages, Some(interrupt_flag), Some(token_tx)).await;

        match result {
            Ok(response) => {
                let full_content = response.content;
                let total_chars_count = full_content.chars().count();
                let resp_preview = if full_content.len() > 100 { format!("{}...", &full_content[..full_content.ceil_char_boundary(100)]) } else { full_content.clone() };
                print!("\x1b[32m[stream] session={} total {} chars\x1b[0m\n", sid_log, total_chars_count);

                tx_clone.send(Ok(Event::default().data(format!(r#"{{"session_id":"{}"}}"#, session_id_clone)))).ok();

                if !full_content.is_empty() {
                    state_clone.session_db.write().await
                        .save_message(&session_id_clone, "assistant", &full_content)
                        .await
                        .ok();
                }

                tx_clone.send(Ok(Event::default().data(r#"{"done":true}"#))).ok();
                print!("\x1b[32m[stream] session={} ✓ complete ({} chars)\x1b[0m\n", sid_log, total_chars_count);
            }
            Err(e) => {
                print!("\x1b[31m[stream] session={} ✗ ERROR: {}\x1b[0m\n", sid_log, e);
                let error_data = serde_json::json!({
                    "error": e.to_string()
                });
                if let Ok(event) = serde_json::to_string(&error_data) {
                    tx_clone.send(Ok(Event::default().data(event))).ok();
                }
                tx_clone.send(Ok(Event::default().data(r#"{"done":true}"#))).ok();
            }
        }
    });

    tokio::spawn(async move {
        while let Some(token) = token_rx.recv().await {
            let event_data = serde_json::json!({
                "content": token,
                "done": false
            });
            if let Ok(event) = serde_json::to_string(&event_data) {
                tx.send(Ok(Event::default().data(event))).ok();
            }
        }
    });

    let stream = UnboundedReceiverStream::new(rx).map_err(|e: anyhow::Error| anyhow::anyhow!("Channel error: {}", e));
    Ok(Sse::new(stream))
}

async fn chat_interrupt(
    State(state): State<AppState>,
) -> Json<serde_json::Value> {
    state.interrupt_flag.store(true, Ordering::SeqCst);
    print!("\x1b[33m[interrupt] Conversation interrupt requested\x1b[0m\n");
    Json(serde_json::json!({"status": "interrupted"}))
}

async fn list_sessions(
    State(state): State<AppState>,
) -> Result<Json<Vec<SessionInfo>>, (StatusCode, Json<ErrorResponse>)> {
    match state.session_db.read().await.list_sessions().await {
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
    match state.session_db.write().await.create_session(req.model).await {
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
    match state.session_db.read().await.get_session(&id).await {
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
    match state.session_db.read().await.get_messages(&id).await {
        Ok(messages) => Ok(Json(messages)),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse {
            error: e.to_string(),
        }))),
    }
}

async fn list_tools() -> Json<Vec<serde_json::Value>> {
    Json(hermes_agent::get_tool_definitions())
}

async fn delete_session(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<()>, (StatusCode, Json<ErrorResponse>)> {
    match state.session_db.write().await.delete_session(&id).await {
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
    let config = state.config.read().await.get().clone();
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
    if let Err(e) = state.config.write().await.update(update) {
        return Err((StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse {
            error: e.to_string(),
        })));
    }
    let config = state.config.read().await.get().clone();
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

#[derive(Debug, Deserialize)]
struct MemoryActionRequest {
    action: String,
    target: String,
    content: Option<String>,
    old_content: Option<String>,
}

async fn memory_read() -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let mut store = hermes_agent::MemoryStore::new();
    if let Err(e) = store.load_from_disk() {
        return Err((StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse {
            error: format!("Failed to load memory: {}", e),
        })));
    }
    let memory_content = store.execute_action("read", "memory", None, None);
    let user_content = store.execute_action("read", "user", None, None);
    Ok(Json(serde_json::json!({
        "memory": serde_json::from_str::<serde_json::Value>(&memory_content).unwrap_or_default(),
        "user": serde_json::from_str::<serde_json::Value>(&user_content).unwrap_or_default(),
    })))
}

async fn memory_action(
    Json(req): Json<MemoryActionRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let mut store = hermes_agent::MemoryStore::new();
    if let Err(e) = store.load_from_disk() {
        return Err((StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse {
            error: format!("Failed to load memory: {}", e),
        })));
    }
    let result = store.execute_action(&req.action, &req.target, req.content.as_deref(), req.old_content.as_deref());
    match serde_json::from_str::<serde_json::Value>(&result) {
        Ok(v) => Ok(Json(v)),
        Err(_) => Ok(Json(serde_json::json!({"raw": result}))),
    }
}