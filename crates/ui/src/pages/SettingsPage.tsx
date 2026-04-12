import { Key, Palette, Globe } from "lucide-react";
import { useState, useEffect } from "react";
import { getConfig, updateConfig } from "../api";

const THEMES = [
  { id: "official-dark", name: "Official Dark", accent: "#c9a227" },
  { id: "official-light", name: "Official Light", accent: "#c9a227" },
  { id: "classic-dark", name: "Classic Dark", accent: "#4a9eff" },
  { id: "classic-light", name: "Classic Light", accent: "#0066cc" },
  { id: "slate-dark", name: "Slate Dark", accent: "#38bdf8" },
  { id: "slate-light", name: "Slate Light", accent: "#0284c7" },
  { id: "mono-dark", name: "Mono Dark", accent: "#a3a3a3" },
  { id: "mono-light", name: "Mono Light", accent: "#404040" },
];

const PROVIDERS = [
  { id: "minimax", name: "MiniMax", url: "https://api.minimaxi.com/v1" },
  { id: "openai", name: "OpenAI", url: "https://api.openai.com/v1" },
  { id: "anthropic", name: "Anthropic", url: "https://api.anthropic.com" },
  { id: "openrouter", name: "OpenRouter", url: "https://openrouter.ai/api/v1" },
];

export function SettingsPage() {
  const [activeTab, setActiveTab] = useState("providers");
  const [apiKey, setApiKey] = useState("");
  const [selectedTheme, setSelectedTheme] = useState("official-dark");
  const [saveStatus, setSaveStatus] = useState<"idle" | "saving" | "saved" | "error">("idle");

  useEffect(() => {
    const saved = localStorage.getItem("hermes-theme") || "official-dark";
    setSelectedTheme(saved);
    document.documentElement.setAttribute("data-theme", saved);
  }, []);

  useEffect(() => {
    getConfig().then(cfg => {
      setApiKey(cfg.api_key || "");
    }).catch(() => {});
  }, []);

  const handleThemeChange = (themeId: string) => {
    setSelectedTheme(themeId);
    localStorage.setItem("hermes-theme", themeId);
    document.documentElement.setAttribute("data-theme", themeId);
    updateConfig({ skin: themeId }).catch(() => {});
  };

  const handleSaveApiKey = async () => {
    setSaveStatus("saving");
    try {
      await updateConfig({ api_key: apiKey });
      setSaveStatus("saved");
      setTimeout(() => setSaveStatus("idle"), 2000);
    } catch {
      setSaveStatus("error");
      setTimeout(() => setSaveStatus("idle"), 3000);
    }
  };

  return (
    <div className="page-container settings-page">
      <div className="page-header">
        <h1>Settings</h1>
      </div>
      <div className="settings-layout">
        <nav className="settings-nav">
          <button
            className={`settings-nav-item ${activeTab === "providers" ? "active" : ""}`}
            onClick={() => setActiveTab("providers")}
          >
            <Globe size={18} />
            <span>Providers</span>
          </button>
          <button
            className={`settings-nav-item ${activeTab === "appearance" ? "active" : ""}`}
            onClick={() => setActiveTab("appearance")}
          >
            <Palette size={18} />
            <span>Appearance</span>
          </button>
          <button
            className={`settings-nav-item ${activeTab === "api" ? "active" : ""}`}
            onClick={() => setActiveTab("api")}
          >
            <Key size={18} />
            <span>API Keys</span>
          </button>
        </nav>

        <div className="settings-content">
          {activeTab === "providers" && (
            <div className="settings-section">
              <h2>LLM Providers</h2>
              <p className="settings-description">Configure your LLM provider endpoints</p>
              <div className="provider-list">
                {PROVIDERS.map(p => (
                  <div key={p.id} className="provider-item">
                    <div className="provider-info">
                      <div className="provider-name">{p.name}</div>
                      <div className="provider-url">{p.url}</div>
                    </div>
                    <button className="provider-config-btn">Configure</button>
                  </div>
                ))}
              </div>
            </div>
          )}

          {activeTab === "appearance" && (
            <div className="settings-section">
              <h2>Theme</h2>
              <p className="settings-description">Choose from 8 preset themes</p>
              <div className="theme-grid">
                {THEMES.map(t => (
                  <button
                    key={t.id}
                    className={`theme-card ${selectedTheme === t.id ? "selected" : ""}`}
                    onClick={() => handleThemeChange(t.id)}
                  >
                    <div 
                      className="theme-preview" 
                      style={{ 
                        background: t.id.endsWith("-dark") ? "#1a1a1f" : "#ffffff",
                        border: `2px solid ${t.accent}`
                      }}
                    />
                    <div className="theme-name">{t.name}</div>
                  </button>
                ))}
              </div>
            </div>
          )}

          {activeTab === "api" && (
            <div className="settings-section">
              <h2>API Keys</h2>
              <p className="settings-description">Manage your API keys for LLM providers</p>
              <div className="api-key-form">
                <label>
                  <span>MiniMax API Key</span>
                  <input
                    type="password"
                    value={apiKey}
                    onChange={e => setApiKey(e.target.value)}
                    placeholder="sk-cp-c0SQS3..."
                  />
                </label>
                <button 
                  className={`save-btn ${saveStatus}`}
                  onClick={handleSaveApiKey}
                  disabled={saveStatus === "saving"}
                >
                  {saveStatus === "saving" ? "Saving..." : saveStatus === "saved" ? "✓ Saved" : saveStatus === "error" ? "Error" : "Save"}
                </button>
              </div>
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
