import { Sparkles, ExternalLink, Search } from "lucide-react";
import { useState, useEffect } from "react";
import { listDir } from "../api";

interface Skill {
  name: string;
  description: string;
  path: string;
}

const MARKETPLACE_SKILLS = [
  { name: "git-master", description: "Git operations, branching, rebasing, and history analysis", category: "Development" },
  { name: "docker", description: "Container management, Docker Compose, and image operations", category: "Infrastructure" },
  { name: "web-search", description: "Web search using Tavily's LLM-optimized API", category: "Research" },
  { name: "self-improving", description: "Self-reflection and learning from experience", category: "AI" },
  { name: "superpowers", description: "Spec-first, TDD, subagent-driven development workflow", category: "Workflow" },
  { name: "playwright", description: "Browser automation and testing with Playwright", category: "Testing" },
  { name: "rust", description: "Idiomatic Rust with ownership and borrow checker patterns", category: "Development" },
  { name: "frontend-ui-ux", description: "Designer-turned-developer for stunning UI/UX", category: "Design" },
];

export function SkillsPage() {
  const [skills, setSkills] = useState<Skill[]>([]);
  const [search, setSearch] = useState("");
  const [activeTab, setActiveTab] = useState<"installed" | "marketplace">("installed");

  useEffect(() => {
    listDir("G:\\opencode-project\\hermes-agent").then(entries => {
      const skillDirs = entries
        .filter(e => e.is_dir && !e.name.startsWith(".") && e.name !== "node_modules")
        .map(e => ({
          name: e.name,
          description: `Local project: ${e.name}`,
          path: e.path,
        }));
      setSkills(skillDirs);
    }).catch(() => setSkills([]));
  }, []);

  const filteredMarketplace = MARKETPLACE_SKILLS.filter(s =>
    s.name.toLowerCase().includes(search.toLowerCase()) ||
    s.description.toLowerCase().includes(search.toLowerCase())
  );

  return (
    <div className="page-container skills-page">
      <div className="page-header">
        <h1>Skills Hub</h1>
      </div>
      <div className="skills-tabs">
        <button
          className={`tab-btn ${activeTab === "installed" ? "active" : ""}`}
          onClick={() => setActiveTab("installed")}
        >
          Installed ({skills.length})
        </button>
        <button
          className={`tab-btn ${activeTab === "marketplace" ? "active" : ""}`}
          onClick={() => setActiveTab("marketplace")}
        >
          Marketplace
        </button>
      </div>

      {activeTab === "marketplace" && (
        <div className="skills-search">
          <Search size={16} />
          <input
            type="text"
            placeholder="Search skills..."
            value={search}
            onChange={e => setSearch(e.target.value)}
          />
        </div>
      )}

      <div className="skills-content">
        {activeTab === "installed" && (
          <>
            {skills.length === 0 ? (
              <div className="skills-placeholder">
                <Sparkles size={48} />
                <h2>No Skills Installed</h2>
                <p>Skills are stored in your ~/.hermes/skills/ directory</p>
              </div>
            ) : (
              <div className="skills-grid">
                {skills.map(skill => (
                  <div key={skill.name} className="skill-card">
                    <div className="skill-header">
                      <Sparkles size={20} />
                      <h3>{skill.name}</h3>
                    </div>
                    <p className="skill-desc">{skill.description}</p>
                    <div className="skill-path">{skill.path}</div>
                  </div>
                ))}
              </div>
            )}
          </>
        )}

        {activeTab === "marketplace" && (
          <div className="skills-grid">
            {filteredMarketplace.map(skill => (
              <div key={skill.name} className="skill-card">
                <div className="skill-header">
                  <Sparkles size={20} />
                  <h3>{skill.name}</h3>
                  <span className="skill-category">{skill.category}</span>
                </div>
                <p className="skill-desc">{skill.description}</p>
                <button className="skill-install-btn" disabled>
                  <ExternalLink size={14} /> Install
                </button>
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}
