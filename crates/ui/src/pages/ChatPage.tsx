import { useState, useEffect, useRef, useCallback } from "react";
import { chatStream, listSessions, healthCheck, getSessionMessages, deleteSession, getConfig, ChatMessage, SessionInfo } from "../api";
import { MarkdownRenderer } from "../components/MarkdownRenderer";
import { ToolCallList, parseToolCalls } from "../components/ToolCallCard";
import { Plus, MessageSquare, Copy, Trash2, Check, X } from "lucide-react";

const MODELS = [
  "MiniMax-M2.7-highspeed",
  "anthropic/claude-sonnet-4-20250514",
  "anthropic/claude-4-opus-20251120",
  "openai/gpt-4o",
  "openai/gpt-4o-mini",
  "google/gemini-2.5-pro",
  "mistralai/mistral-large-2411",
  "deepseek/deepseek-chat-v3-0324",
  "NousThink/Thinker-Large",
];

const SLASH_COMMANDS = [
  { name: "/new", description: "Start a new conversation", icon: "✚" },
  { name: "/clear", description: "Clear current messages", icon: "🗑" },
  { name: "/model", description: "Switch model (e.g. /model MiniMax-M2.7-highspeed)", icon: "🤖" },
  { name: "/help", description: "Show available commands", icon: "❓" },
  { name: "/sessions", description: "Show recent sessions", icon: "📋" },
];

function formatDate(iso: string): string {
  try {
    const d = new Date(iso);
    const now = new Date();
    const diffMs = now.getTime() - d.getTime();
    const diffMins = Math.floor(diffMs / 60000);
    if (diffMins < 1) return "just now";
    if (diffMins < 60) return `${diffMins}m ago`;
    const diffHours = Math.floor(diffMins / 60);
    if (diffHours < 24) return `${diffHours}h ago`;
    const diffDays = Math.floor(diffHours / 24);
    if (diffDays < 7) return `${diffDays}d ago`;
    return d.toLocaleDateString();
  } catch {
    return "";
  }
}

export function ChatPage() {
  const [messages, setMessages] = useState<ChatMessage[]>([]);
  const [input, setInput] = useState("");
  const [model, setModel] = useState(MODELS[0]);
  const [isLoading, setIsLoading] = useState(false);
  const [sessions, setSessions] = useState<SessionInfo[]>([]);
  const [activeSession, setActiveSession] = useState<string | null>(null);
  const [sessionId, setSessionId] = useState<string | null>(null);
  const [backendUp, setBackendUp] = useState(false);
  const [showSidebar, setShowSidebar] = useState(true);
  const [copiedIdx, setCopiedIdx] = useState<number | null>(null);
  const [showSlashMenu, setShowSlashMenu] = useState(false);
  const [slashFilter, setSlashFilter] = useState("");
  const [selectedCmdIdx, setSelectedCmdIdx] = useState(0);
  const [apiKey, setApiKey] = useState("");
  const messagesEndRef = useRef<HTMLDivElement>(null);
  const textareaRef = useRef<HTMLTextAreaElement>(null);
  const slashMenuRef = useRef<HTMLDivElement>(null);

  const scrollToBottom = () => {
    messagesEndRef.current?.scrollIntoView({ behavior: "smooth" });
  };

  useEffect(() => { scrollToBottom(); }, [messages]);

  const loadSessions = useCallback(async () => {
    try {
      const s = await listSessions();
      setSessions(s);
    } catch { }
  }, []);

  useEffect(() => {
    const check = async () => {
      try { await healthCheck(); setBackendUp(true); } catch { setBackendUp(false); }
    };
    check();
    loadSessions();
    getConfig().then(cfg => {
      setApiKey(cfg.api_key || "");
      if (cfg.model) setModel(cfg.model);
    }).catch(() => {});
    const interval = setInterval(check, 10000);
    return () => clearInterval(interval);
  }, [loadSessions]);

  const filteredCommands = SLASH_COMMANDS.filter(cmd =>
    cmd.name.toLowerCase().includes(slashFilter.toLowerCase())
  );

  const executeCommand = (cmd: string) => {
    setShowSlashMenu(false);
    setSlashFilter("");
    if (cmd === "/new") {
      setMessages([]);
      setActiveSession(null);
      setSessionId(null);
    } else if (cmd === "/clear") {
      setMessages([]);
    } else if (cmd.startsWith("/model ")) {
      const newModel = cmd.slice(7).trim();
      if (MODELS.includes(newModel)) {
        setModel(newModel);
      }
    } else if (cmd === "/help") {
      const helpText = SLASH_COMMANDS.map(c => `${c.name} — ${c.description}`).join("\n");
      setMessages(prev => [...prev,
        { role: "user", content: "/help" },
        { role: "assistant", content: `**Available Commands:**\n${helpText}` },
      ]);
    } else if (cmd === "/sessions") {
      if (sessions.length === 0) {
        setMessages(prev => [...prev,
          { role: "user", content: "/sessions" },
          { role: "assistant", content: "No sessions found." },
        ]);
      } else {
        const sessionList = sessions.slice(0, 5).map((s, i) => 
          `${i + 1}. **${s.model || "Chat"}** — ${formatDate(s.updated_at)}`
        ).join("\n");
        setMessages(prev => [...prev,
          { role: "user", content: "/sessions" },
          { role: "assistant", content: `**Recent Sessions:**\n${sessionList}` },
        ]);
      }
    }
  };

  const handleSend = async () => {
    if (!input.trim() || isLoading) return;
    if (input.startsWith("/") && filteredCommands.length > 0 && !input.includes(" ")) {
      executeCommand(filteredCommands[0].name);
      return;
    }
    const userMsg: ChatMessage = { role: "user", content: input.trim() };
    const newMessages = [...messages, userMsg];
    setMessages(newMessages);
    setInput("");
    setIsLoading(true);
    setShowSlashMenu(false);

    const assistantMsgId = newMessages.length;
    setMessages(prev => [...prev, { role: "assistant", content: "" }]);

    let accumulatedContent = "";

    try {
      const returnedSessionId = await chatStream(
        { model, messages: newMessages, api_key: apiKey, session_id: sessionId ?? undefined },
        {
          onChunk: (chunk) => {
            if (chunk.done) return;
            accumulatedContent += chunk.content;
            setMessages(prev => {
              const updated = [...prev];
              if (updated[assistantMsgId]) {
                updated[assistantMsgId] = { ...updated[assistantMsgId], content: accumulatedContent };
              }
              return updated;
            });
          },
          onError: (err) => {
            setMessages(prev => {
              const updated = [...prev];
              if (updated[assistantMsgId]) {
                updated[assistantMsgId] = { ...updated[assistantMsgId], content: `Error: ${err.message}` };
              }
              return updated;
            });
          },
          onDone: () => {
            loadSessions();
          },
        }
      );
      if (returnedSessionId) {
        setSessionId(returnedSessionId);
      }
    } catch (e) {
      setMessages(prev => {
        const updated = [...prev];
        if (updated[assistantMsgId]) {
          updated[assistantMsgId] = { ...updated[assistantMsgId], content: `Error: ${e instanceof Error ? e.message : String(e)}` };
        }
        return updated;
      });
    } finally {
      setIsLoading(false);
    }
  };

  const handleNewChat = () => {
    setMessages([]);
    setActiveSession(null);
    setSessionId(null);
  };

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (showSlashMenu && filteredCommands.length > 0) {
      if (e.key === "ArrowDown") {
        e.preventDefault();
        setSelectedCmdIdx(i => Math.min(i + 1, filteredCommands.length - 1));
        return;
      }
      if (e.key === "ArrowUp") {
        e.preventDefault();
        setSelectedCmdIdx(i => Math.max(i - 1, 0));
        return;
      }
      if (e.key === "Enter") {
        e.preventDefault();
        executeCommand(filteredCommands[selectedCmdIdx].name);
        return;
      }
      if (e.key === "Escape") {
        setShowSlashMenu(false);
        return;
      }
    }
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      handleSend();
    }
  };

  const handleInputChange = (e: React.ChangeEvent<HTMLTextAreaElement>) => {
    const val = e.target.value;
    setInput(val);
    if (val.startsWith("/")) {
      setSlashFilter(val.slice(1));
      setShowSlashMenu(true);
      setSelectedCmdIdx(0);
    } else {
      setShowSlashMenu(false);
    }
    const ta = textareaRef.current;
    if (ta) {
      ta.style.height = "auto";
      ta.style.height = Math.min(ta.scrollHeight, 120) + "px";
    }
  };

  const handleCopy = (content: string, idx: number) => {
    navigator.clipboard.writeText(content).then(() => {
      setCopiedIdx(idx);
      setTimeout(() => setCopiedIdx(null), 2000);
    });
  };

  const handleDelete = (idx: number) => {
    setMessages(prev => prev.filter((_, i) => i !== idx));
  };

  const handleDeleteSession = async (e: React.MouseEvent, sid: string) => {
    e.stopPropagation();
    try {
      await deleteSession(sid);
      setSessions(prev => prev.filter(s => s.id !== sid));
      if (activeSession === sid) {
        setActiveSession(null);
        setSessionId(null);
        setMessages([]);
      }
    } catch { }
  };

  return (
    <div className="chat-page">
      <header className="chat-header">
        <div className="header-title">
          <span>☤</span>
          <span>Hermes</span>
        </div>
        <div className="header-controls">
          <div className="model-selector">
            <select value={model} onChange={e => setModel(e.target.value)}>
              {MODELS.map(m => <option key={m} value={m}>{m.split("/")[1]}</option>)}
            </select>
          </div>
          <button className="header-btn" onClick={() => setShowSidebar(s => !s)} title="Toggle sidebar">
            ☰
          </button>
        </div>
      </header>

      <div className="chat-container">
        {showSidebar && (
          <aside className="chat-sidebar">
            <div className="sidebar-section">
              <button className="new-chat-btn" onClick={handleNewChat}>
                <Plus size={16} /> New Chat
              </button>
            </div>
            <div className="sidebar-section">
              <div className="sidebar-section-title">Recent Sessions</div>
              {sessions.length === 0 && (
                <div className="empty-sessions">No sessions yet</div>
              )}
              {sessions.map(s => (
                <div
                  key={s.id}
                  className={`session-item ${activeSession === s.id ? "active" : ""}`}
                  onClick={async () => {
                    setActiveSession(s.id);
                    setSessionId(s.id);
                    setMessages([]);
                    try {
                      const msgs = await getSessionMessages(s.id);
                      setMessages(msgs.map(m => ({ role: m.role as "user" | "assistant", content: m.content })));
                    } catch { }
                  }}
                >
                  <MessageSquare size={14} />
                  <div className="session-info">
                    <div className="session-item-title">{s.model || "Chat"}</div>
                    <div className="session-item-date">{formatDate(s.updated_at)}</div>
                  </div>
                  <button
                    className="session-delete-btn"
                    onClick={(e) => handleDeleteSession(e, s.id)}
                    title="Delete session"
                  >
                    <X size={12} />
                  </button>
                </div>
              ))}
            </div>
          </aside>
        )}

        <main className="chat-main">
          {messages.length === 0 ? (
            <div className="empty-state">
              <div className="empty-state-icon">☤</div>
              <div className="empty-state-text">Start a conversation with Hermes</div>
              <div className="empty-state-hint">Type <kbd>/help</kbd> for available commands</div>
            </div>
          ) : (
            <div className="chat-messages">
              {messages.map((msg, i) => {
                const toolCalls = msg.role === "assistant" ? parseToolCalls(msg.content) : null;
                const displayContent = msg.role === "assistant" 
                  ? msg.content.replace(/<tool_calls>[\s\S]*?<\/tool_calls>/g, "").trim()
                  : msg.content;
                
                return (
                  <div key={i} className={`message ${msg.role}`}>
                    <div className="message-header">
                      <div className="message-role">{msg.role === "user" ? "You" : "Hermes"}</div>
                      <div className="message-actions">
                        <button 
                          className="message-action-btn" 
                          onClick={() => handleCopy(msg.content, i)}
                          title="Copy"
                        >
                          {copiedIdx === i ? <Check size={14} /> : <Copy size={14} />}
                        </button>
                        <button 
                          className="message-action-btn" 
                          onClick={() => handleDelete(i)}
                          title="Delete"
                        >
                          <Trash2 size={14} />
                        </button>
                      </div>
                    </div>
                    {displayContent && (
                      <div className="message-content">
                        <MarkdownRenderer content={displayContent} />
                      </div>
                    )}
                    {toolCalls && toolCalls.length > 0 && (
                      <div className="message-tool-calls">
                        <ToolCallList toolCalls={toolCalls} />
                      </div>
                    )}
                  </div>
                );
              })}
              {isLoading && (
                <div className="message assistant">
                  <div className="message-header">
                    <div className="message-role">Hermes</div>
                  </div>
                  <div className="message-content">
                    <span className="loading-dots"><span/><span/><span/></span>
                  </div>
                </div>
              )}
              <div ref={messagesEndRef} />
            </div>
          )}

          <div className="chat-input-area">
            {showSlashMenu && filteredCommands.length > 0 && (
              <div className="slash-menu" ref={slashMenuRef}>
                {filteredCommands.map((cmd, i) => (
                  <div
                    key={cmd.name}
                    className={`slash-cmd ${i === selectedCmdIdx ? "selected" : ""}`}
                    onClick={() => executeCommand(cmd.name)}
                    onMouseEnter={() => setSelectedCmdIdx(i)}
                  >
                    <span className="slash-cmd-icon">{cmd.icon}</span>
                    <div className="slash-cmd-info">
                      <span className="slash-cmd-name">{cmd.name}</span>
                      <span className="slash-cmd-desc">{cmd.description}</span>
                    </div>
                  </div>
                ))}
              </div>
            )}
            <div className="chat-input-wrapper">
              <textarea
                ref={textareaRef}
                className="chat-input"
                value={input}
                onChange={handleInputChange}
                onKeyDown={handleKeyDown}
                placeholder="Message Hermes... (type / for commands)"
                rows={1}
              />
              <button className="send-btn" onClick={handleSend} disabled={isLoading || !input.trim()}>
                Send
              </button>
            </div>
          </div>
        </main>
      </div>

      <div className="status-bar">
        <div className="status-indicator">
          <div className={`status-dot ${backendUp ? "" : "error"}`} />
          <span>{backendUp ? "Backend connected" : "Backend offline"}</span>
        </div>
        <div>{messages.length} messages · {model.split("/")[1]}</div>
      </div>
    </div>
  );
}
