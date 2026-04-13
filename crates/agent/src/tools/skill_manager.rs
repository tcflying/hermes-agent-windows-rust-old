use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Skill {
    pub name: String,
    pub description: String,
    pub content: String,
    pub created_at: String,
    pub updated_at: String,
    pub source: String,
    pub enabled: bool,
}

fn get_skills_dir() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".hermes")
        .join("skills")
}

fn skill_path(name: &str) -> PathBuf {
    get_skills_dir().join(format!("{}.md", name))
}

fn meta_path(name: &str) -> PathBuf {
    get_skills_dir().join(format!("{}.json", name))
}

fn load_skill_from_file(name: &str) -> Option<Skill> {
    let content = fs::read_to_string(skill_path(name)).ok()?;
    let meta: Skill = if let Ok(meta_str) = fs::read_to_string(meta_path(name)) {
        let mut s: Skill = serde_json::from_str(&meta_str).ok()?;
        s.content = content;
        s
    } else {
        Skill {
            name: name.to_string(),
            description: String::new(),
            content,
            created_at: chrono::Utc::now().to_rfc3339(),
            updated_at: chrono::Utc::now().to_rfc3339(),
            source: "user".to_string(),
            enabled: true,
        }
    };
    Some(meta)
}

fn save_skill(skill: &Skill) -> Result<(), String> {
    let dir = get_skills_dir();
    let _ = fs::create_dir_all(&dir);

    let md_path = skill_path(&skill.name);
    fs::write(&md_path, &skill.content).map_err(|e| format!("{}", e))?;

    let mut meta = skill.clone();
    meta.content = String::new();
    let meta_json = serde_json::to_string_pretty(&meta).map_err(|e| format!("{}", e))?;
    fs::write(meta_path(&skill.name), meta_json).map_err(|e| format!("{}", e))
}

pub struct SkillManager {
    skills: HashMap<String, Skill>,
}

impl SkillManager {
    pub fn new() -> Self {
        let mut mgr = Self {
            skills: HashMap::new(),
        };
        let _ = mgr.scan_directory();
        mgr
    }

    pub fn list(&self) -> String {
        let skills: Vec<&Skill> = self.skills.values().collect();
        serde_json::json!({
            "count": skills.len(),
            "skills": skills.iter().map(|s| {
                serde_json::json!({
                    "name": s.name,
                    "description": s.description,
                    "source": s.source,
                    "enabled": s.enabled,
                    "created_at": s.created_at,
                    "updated_at": s.updated_at
                })
            }).collect::<Vec<_>>()
        })
        .to_string()
    }

    pub fn view(&self, name: &str) -> String {
        match self.skills.get(name) {
            Some(skill) => serde_json::json!({
                "name": skill.name,
                "description": skill.description,
                "content": skill.content,
                "source": skill.source,
                "enabled": skill.enabled
            })
            .to_string(),
            None => serde_json::json!({"error": format!("Skill not found: {}", name)}).to_string(),
        }
    }

    pub fn create(&mut self, name: &str, description: &str, content: &str) -> String {
        let now = chrono::Utc::now().to_rfc3339();
        let skill = Skill {
            name: name.to_string(),
            description: description.to_string(),
            content: content.to_string(),
            created_at: now.clone(),
            updated_at: now,
            source: "user".to_string(),
            enabled: true,
        };
        if let Err(e) = save_skill(&skill) {
            return serde_json::json!({"error": e}).to_string();
        }
        self.skills.insert(name.to_string(), skill.clone());
        serde_json::json!({"status": "created", "skill": skill.name}).to_string()
    }

    pub fn update(&mut self, name: &str, content: &str) -> String {
        match self.skills.get_mut(name) {
            Some(skill) => {
                skill.content = content.to_string();
                skill.updated_at = chrono::Utc::now().to_rfc3339();
                if let Err(e) = save_skill(skill) {
                    return serde_json::json!({"error": e}).to_string();
                }
                serde_json::json!({"status": "updated", "name": name}).to_string()
            }
            None => serde_json::json!({"error": format!("Skill not found: {}", name)}).to_string(),
        }
    }

    pub fn delete(&mut self, name: &str) -> String {
        match self.skills.remove(name) {
            Some(_) => {
                let _ = fs::remove_file(skill_path(name));
                let _ = fs::remove_file(meta_path(name));
                serde_json::json!({"status": "deleted", "name": name}).to_string()
            }
            None => serde_json::json!({"error": format!("Skill not found: {}", name)}).to_string(),
        }
    }

    pub fn toggle(&mut self, name: &str, enabled: bool) -> String {
        match self.skills.get_mut(name) {
            Some(skill) => {
                skill.enabled = enabled;
                skill.updated_at = chrono::Utc::now().to_rfc3339();
                let _ = save_skill(skill);
                serde_json::json!({"status": "toggled", "name": name, "enabled": enabled})
                    .to_string()
            }
            None => serde_json::json!({"error": format!("Skill not found: {}", name)}).to_string(),
        }
    }

    pub fn search(&self, query: &str) -> String {
        let query_lower = query.to_lowercase();
        let results: Vec<&Skill> = self
            .skills
            .values()
            .filter(|s| {
                s.name.to_lowercase().contains(&query_lower)
                    || s.description.to_lowercase().contains(&query_lower)
            })
            .collect();
        serde_json::json!({"count": results.len(), "results": results.iter().map(|s| &s.name).collect::<Vec<_>>()}).to_string()
    }

    pub fn scan_directory(&mut self) -> Result<usize, String> {
        let dir = get_skills_dir();
        if !dir.exists() {
            let _ = fs::create_dir_all(&dir);
            return Ok(0);
        }

        let mut found = 0;
        if let Ok(entries) = fs::read_dir(&dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if let Some(ext) = path.extension() {
                    if ext == "md" {
                        if let Some(stem) = path.file_stem() {
                            if let Some(name) = stem.to_str() {
                                if !self.skills.contains_key(name) {
                                    if let Some(skill) = load_skill_from_file(name) {
                                        self.skills.insert(name.to_string(), skill);
                                        found += 1;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        Ok(found)
    }

    pub fn get_enabled_skills_content(&self) -> String {
        let mut parts = Vec::new();
        for skill in self.skills.values().filter(|s| s.enabled) {
            parts.push(format!("## Skill: {}\n{}\n", skill.name, skill.content));
        }
        if parts.is_empty() {
            String::new()
        } else {
            format!("# Loaded Skills\n\n{}", parts.join("\n"))
        }
    }

    pub fn auto_create_from_experience(
        &mut self,
        task_description: &str,
        solution: &str,
    ) -> String {
        let name = format!("auto-{}", chrono::Utc::now().timestamp() % 100000);
        let content = format!(
            "# Auto-generated Skill\n\n## Task\n{}\n\n## Solution\n{}\n\n## Notes\nThis skill was auto-created from a completed task.",
            task_description, solution
        );
        self.create(
            &name,
            &format!(
                "Auto-skill: {}",
                task_description.chars().take(80).collect::<String>()
            ),
            &content,
        )
    }
}

impl Default for SkillManager {
    fn default() -> Self {
        Self::new()
    }
}
