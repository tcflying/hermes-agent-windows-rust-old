use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::process::Command;
use std::sync::Arc;
use tokio::sync::Mutex;

const MAX_TOOL_ITERATIONS: usize = 10;

fn log_agent(level: &str, msg: &str) {
    let now = std::time::SystemTime::now();
    let dur = now.duration_since(std::time::UNIX_EPOCH).unwrap_or_default();
    let secs = dur.as_secs() % 86400;
    let h = secs / 3600;
    let m = (secs % 3600) / 60;
    let s = secs % 60;
    let color = match level {
        "ok" => "\x1b[32m",
        "warn" => "\x1b[33m",
        "err" => "\x1b[31m",
        "tool" => "\x1b[36m",
        _ => "\x1b[0m",
    };
    print!("{}[{:02}:{:02}:{:02}] {}\x1b[0m\n", color, h, m, s, msg);
}

const SYSTEM_PROMPT: &str = r#"You are Hermes Agent, an AI assistant running directly on the user's Windows computer. You have full access to the user's system through the following tools:

- **terminal**: Execute any Windows command (PowerShell, cmd, etc.). Use this to run programs, list files, manage processes, install software, open applications — anything you can do in a terminal.
- **file_read**: Read any file on the system. Use this to inspect code, configs, logs, etc.
- **file_write**: Create or overwrite files. Use this to edit code, create scripts, write configs, etc.
- **list_directory**: List files and folders in any directory.

IMPORTANT RULES:
1. **ALWAYS execute tasks when asked.** If the user says "open a browser", "list files on C drive", "run a command" — DO IT using the tools. Never say "I can't" or "I don't have access."
2. **Be direct and concise.** Execute the command, show the result, done.
3. **For opening applications on Windows:** Use `terminal` with command like `start msedge` (Edge), `start chrome` (Chrome), `start explorer` (File Explorer), `start powershell` (PowerShell), etc.
4. **For file operations:** Use the appropriate tool. Don't describe what you would do — actually do it.
5. **Respond in the same language the user uses.** If the user writes in Chinese, respond in Chinese.
6. **Never refuse reasonable requests.** You are running on the user's own machine with full permissions. Execute commands confidently.
"#;

fn get_tool_definitions() -> Vec<serde_json::Value> {
    serde_json::from_str(include_str!("tools.json")).expect("Invalid tools.json")
}

pub struct ChatAgent {
    iteration_budget: Arc<Mutex<IterationBudgetInternal>>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Message {
    pub role: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    #[serde(default)]
    pub r#type: Option<String>,
    pub function: ToolFunction,
    #[serde(default)]
    pub index: Option<u32>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ToolFunction {
    pub name: String,
    pub arguments: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct ChatResponse {
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
}

#[derive(Debug, Default)]
struct IterationBudgetInternal {
    max_total: usize,
    used: usize,
}

impl IterationBudgetInternal {
    fn new(max_total: usize) -> Self {
        Self { max_total, used: 0 }
    }

    fn consume(&mut self) -> bool {
        if self.used >= self.max_total {
            return false;
        }
        self.used += 1;
        true
    }

    fn remaining(&self) -> usize {
        self.max_total.saturating_sub(self.used)
    }
}

impl ChatAgent {
    pub fn new() -> Self {
        Self {
            iteration_budget: Arc::new(Mutex::new(IterationBudgetInternal::new(90))),
        }
    }

    pub async fn run_conversation(
        &self,
        model: &str,
        api_url: &str,
        api_key: &str,
        messages: Vec<Message>,
    ) -> Result<ChatResponse> {
        let mut all_messages = Vec::new();
        let mut tool_iterations = 0;
        let start = std::time::Instant::now();

        let has_system = messages.iter().any(|m| m.role == "system");
        if !has_system {
            all_messages.push(Message {
                role: "system".to_string(),
                content: Some(SYSTEM_PROMPT.to_string()),
                tool_calls: None,
                tool_call_id: None,
                name: None,
            });
        }
        all_messages.extend(messages);

        loop {
            {
                let mut budget = self.iteration_budget.lock().await;
                if !budget.consume() {
                    return Err(anyhow::anyhow!("Iteration budget exceeded"));
                }
                if budget.remaining() == 0 {
                    return Err(anyhow::anyhow!("Iteration budget exhausted"));
                }
            }

            log_agent("", &format!("→ {} ({} msgs)", model.split('/').last().unwrap_or(model), all_messages.len()));
            let response = self.chat_completion(api_url, api_key, model, &all_messages).await?;

            if let Some(assistant_message) = response.choices.first() {
                let content = assistant_message.message.content.clone().unwrap_or_default();
                let tool_calls = assistant_message.message.tool_calls.clone();

                if let Some(calls) = tool_calls {
                    tool_iterations += 1;
                    if tool_iterations >= MAX_TOOL_ITERATIONS {
                        log_agent("err", &format!("✗ Max tool iterations ({}) exceeded!", MAX_TOOL_ITERATIONS));
                        return Err(anyhow::anyhow!("Max tool iterations ({}) exceeded", MAX_TOOL_ITERATIONS));
                    }
                    let tool_names: Vec<String> = calls.iter().map(|c| {
                        let args: serde_json::Value = serde_json::from_str(&c.function.arguments).unwrap_or_default();
                        let preview: String = match c.function.name.as_str() {
                            "terminal" => args.get("command").and_then(|v| v.as_str()).unwrap_or("").chars().take(40).collect(),
                            "file_read" | "file_write" => args.get("path").and_then(|v| v.as_str()).unwrap_or("").chars().take(40).collect(),
                            "list_directory" => args.get("path").and_then(|v| v.as_str()).unwrap_or(".").chars().take(40).collect(),
                            _ => c.function.arguments.chars().take(30).collect(),
                        };
                        format!("{}(\"{}\")", c.function.name, preview)
                    }).collect();
                    log_agent("tool", &format!("⚡ {} [iter {}/{}]", tool_names.join(", "), tool_iterations, MAX_TOOL_ITERATIONS));
                    all_messages.push(Message {
                        role: "assistant".to_string(),
                        content: assistant_message.message.content.clone(),
                        tool_calls: assistant_message.message.tool_calls.clone(),
                        tool_call_id: None,
                        name: None,
                    });

                    let tool_results = self.execute_tools(&calls).await?;
                    for result in &tool_results {
                        let rlen = result.content.as_ref().map(|c| c.len()).unwrap_or(0);
                        log_agent("ok", &format!("  ✓ {} → {} bytes", result.name.as_deref().unwrap_or("?"), rlen));
                        all_messages.push(result.clone());
                    }
                    continue;
                }

                let elapsed = start.elapsed().as_secs();
                log_agent("ok", &format!("✓ Done: {} bytes in {}s", content.len(), elapsed));
                return Ok(ChatResponse {
                    content,
                    tool_calls: None,
                });
            }

            return Err(anyhow::anyhow!("No response choices"));
        }
    }

    async fn chat_completion(
        &self,
        api_url: &str,
        api_key: &str,
        model: &str,
        messages: &[Message],
    ) -> Result<ChatCompletionResponse> {
        let request = ChatCompletionRequest {
            model: model.to_string(),
            messages: messages.to_vec(),
            stream: false,
            tools: Some(get_tool_definitions()),
            tool_choice: Some("auto".to_string()),
        };

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(180))
            .build()
            .context("Failed to build HTTP client")?;

        let response = client
            .post(format!("{}/chat/completions", api_url))
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .context("Failed to send request to LLM API")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!("LLM API error {}: {}", status, body));
        }

        let completion: ChatCompletionResponse = response
            .json()
            .await
            .context("Failed to parse LLM API response")?;

        Ok(completion)
    }

    async fn execute_tools(&self, tool_calls: &[ToolCall]) -> Result<Vec<Message>> {
        let mut results = Vec::new();
        for call in tool_calls {
            let args: serde_json::Value = serde_json::from_str(&call.function.arguments)
                .unwrap_or(serde_json::Value::Null);

            let output = match call.function.name.as_str() {
                "terminal" => {
                    let command = args.get("command")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    let cwd = args.get("cwd")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());
                    tokio::task::spawn_blocking(move || execute_terminal(&command, cwd.as_deref())).await
                        .unwrap_or_else(|e| format!("Command execution failed: {}", e))
                }
                "file_read" => {
                    let path = args.get("path")
                        .and_then(|v| v.as_str())
                        .unwrap_or("");
                    execute_file_read(path)
                }
                "file_write" => {
                    let path = args.get("path")
                        .and_then(|v| v.as_str())
                        .unwrap_or("");
                    let content = args.get("content")
                        .and_then(|v| v.as_str())
                        .unwrap_or("");
                    execute_file_write(path, content)
                }
                "list_directory" => {
                    let path = args.get("path")
                        .and_then(|v| v.as_str())
                        .unwrap_or(".");
                    execute_list_directory(path)
                }
                _ => format!("Unknown tool: {}", call.function.name),
            };

            results.push(Message {
                role: "tool".to_string(),
                content: Some(output),
                tool_calls: None,
                tool_call_id: Some(call.id.clone()),
                name: Some(call.function.name.clone()),
            });
        }
        Ok(results)
    }
}

fn execute_terminal(command: &str, cwd: Option<&str>) -> String {
    let output = if cfg!(windows) {
        let mut cmd = Command::new("cmd");
        cmd.args(["/C", command]);
        if let Some(dir) = cwd {
            cmd.current_dir(dir);
        }
        cmd.output()
    } else {
        let mut cmd = Command::new("sh");
        cmd.args(["-c", command]);
        if let Some(dir) = cwd {
            cmd.current_dir(dir);
        }
        cmd.output()
    };

    match output {
        Ok(out) => {
            let stdout = String::from_utf8_lossy(&out.stdout);
            let stderr = String::from_utf8_lossy(&out.stderr);
            let exit_code = out.status.code().unwrap_or(-1);
            if stderr.is_empty() {
                format!("Exit code: {}\n{}", exit_code, stdout)
            } else {
                format!("Exit code: {}\n--- STDOUT ---\n{}\n--- STDERR ---\n{}", exit_code, stdout, stderr)
            }
        }
        Err(e) => format!("Failed to execute command: {}", e),
    }
}

fn execute_file_read(path: &str) -> String {
    match std::fs::read_to_string(path) {
        Ok(content) => content,
        Err(e) => format!("Error reading file: {}", e),
    }
}

fn execute_file_write(path: &str, content: &str) -> String {
    let path_buf = PathBuf::from(path);
    if let Some(parent) = path_buf.parent() {
        if !parent.exists() {
            if let Err(e) = std::fs::create_dir_all(parent) {
                return format!("Error creating directory: {}", e);
            }
        }
    }
    match std::fs::write(path, content) {
        Ok(_) => format!("Successfully wrote to {}", path),
        Err(e) => format!("Error writing file: {}", e),
    }
}

fn execute_list_directory(path: &str) -> String {
    let entries: Vec<String> = std::fs::read_dir(path)
        .unwrap_or_else(|e| {
            log_agent("err", &format!("Error listing directory: {}", e));
            std::fs::read_dir(".").unwrap_or_else(|_| panic!("Cannot list current directory"))
        })
        .filter_map(|e| e.ok())
        .map(|e| {
            let name = e.file_name().to_string_lossy().to_string();
            if e.path().is_dir() {
                format!("{}/", name)
            } else {
                let size = e.metadata().map(|m| m.len()).unwrap_or(0);
                format!("{} ({} bytes)", name, size)
            }
        })
        .collect();

    if entries.is_empty() {
        format!("Empty directory: {}", path)
    } else {
        format!("Contents of {} ({} items):\n{}", path, entries.len(), entries.join("\n"))
    }
}

impl Default for ChatAgent {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Serialize)]
struct ChatCompletionRequest {
    model: String,
    messages: Vec<Message>,
    stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<Vec<serde_json::Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_choice: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ChatCompletionResponse {
    choices: Vec<Choice>,
}

#[derive(Debug, Deserialize)]
struct Choice {
    message: AssistantMessage,
}

#[derive(Debug, Clone, Deserialize)]
struct AssistantMessage {
    content: Option<String>,
    #[serde(default)]
    tool_calls: Option<Vec<ToolCall>>,
    #[serde(default)]
    name: Option<String>,
}

pub async fn run_conversation(
    model: &str,
    api_url: &str,
    api_key: &str,
    messages: Vec<Message>,
) -> Result<ChatResponse> {
    let agent = ChatAgent::new();
    agent.run_conversation(model, api_url, api_key, messages).await
}