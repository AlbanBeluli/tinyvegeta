//! Context loading for TinyVegeta agents.
//!
//! Loads identity/memory files to build context for AI providers.

use std::path::PathBuf;

use crate::config::get_home_dir;
use crate::error::Error;

/// Context files that get loaded for an agent.
pub struct AgentContext {
    pub brain: Option<String>,
    pub soul_shared: Option<String>,
    pub soul_agent_extra: Option<String>,
    pub identity: Option<String>,
    pub user: Option<String>,
    pub tools: Option<String>,
    pub heartbeat: Option<String>,
    pub clients: Option<String>,
    pub playbook: Option<String>,
    pub memory: Option<String>,
    pub agents: Option<String>,
}

impl AgentContext {
    /// Load context for an agent.
    pub fn load(_agent_id: &str, working_dir: Option<&PathBuf>) -> Result<Self, Error> {
        let home = get_home_dir()?;
        let project_soul = default_soul_path();
        let project_root = default_project_root();
        let workspace_root = infer_workspace_root(working_dir);

        let brain = load_file(&[
            workspace_root.as_ref().map(|d| d.join("BRAIN.md")),
            working_dir.as_ref().map(|d| d.join("BRAIN.md")),
            Some(home.join("BRAIN.md")),
            project_root.as_ref().map(|d| d.join("BRAIN.md")),
        ]);

        // Shared SOUL: workspace-root first (swarm-wide default identity).
        let soul_shared = load_file(&[
            workspace_root.as_ref().map(|d| d.join("SOUL.md")),
            Some(home.join("SOUL.md")),
            project_soul,
        ]);

        // Agent-specific extra SOUL layer (optional).
        let soul_agent_extra = load_file(&[
            working_dir.as_ref().map(|d| d.join("AGENT_SOUL.md")),
            working_dir.as_ref().map(|d| d.join("SOUL.md")),
        ]);

        let identity = load_file(&[
            workspace_root.as_ref().map(|d| d.join("IDENTITY.md")),
            working_dir.as_ref().map(|d| d.join("IDENTITY.md")),
            Some(home.join("IDENTITY.md")),
            project_root.as_ref().map(|d| d.join("IDENTITY.md")),
        ]);

        let user = load_file(&[
            workspace_root.as_ref().map(|d| d.join("USER.md")),
            working_dir.as_ref().map(|d| d.join("USER.md")),
            Some(home.join("USER.md")),
            project_root.as_ref().map(|d| d.join("USER.md")),
        ]);

        let tools = load_file(&[
            workspace_root.as_ref().map(|d| d.join("TOOLS.md")),
            working_dir.as_ref().map(|d| d.join("TOOLS.md")),
            Some(home.join("TOOLS.md")),
            project_root.as_ref().map(|d| d.join("TOOLS.md")),
        ]);

        let heartbeat = load_file(&[
            workspace_root.as_ref().map(|d| d.join("HEARTBEAT.md")),
            working_dir.as_ref().map(|d| d.join("HEARTBEAT.md")),
            Some(home.join("HEARTBEAT.md")),
            project_root.as_ref().map(|d| d.join("HEARTBEAT.md")),
        ]);

        let clients = load_file(&[
            workspace_root.as_ref().map(|d| d.join("CLIENTS.md")),
            working_dir.as_ref().map(|d| d.join("CLIENTS.md")),
            Some(home.join("CLIENTS.md")),
            project_root.as_ref().map(|d| d.join("CLIENTS.md")),
        ]);

        let playbook = load_file(&[
            workspace_root.as_ref().map(|d| d.join("PLAYBOOK.md")),
            working_dir.as_ref().map(|d| d.join("PLAYBOOK.md")),
            Some(home.join("PLAYBOOK.md")),
            project_root.as_ref().map(|d| d.join("PLAYBOOK.md")),
        ]);

        let memory = load_file(&[
            working_dir.as_ref().map(|d| d.join("MEMORY.md")),
            Some(home.join("MEMORY.md")),
            project_root.as_ref().map(|d| d.join("MEMORY.md")),
        ]);

        let agents = load_file(&[
            working_dir.as_ref().map(|d| d.join("AGENTS.md")),
            Some(home.join("AGENTS.md")),
            project_root.as_ref().map(|d| d.join("AGENTS.md")),
        ]);

        Ok(Self {
            brain,
            soul_shared,
            soul_agent_extra,
            identity,
            user,
            tools,
            heartbeat,
            clients,
            playbook,
            memory,
            agents,
        })
    }

    /// Build the system prompt from loaded context.
    pub fn build_system_prompt(&self) -> String {
        let mut parts = Vec::new();

        // BRAIN first by policy.
        if let Some(ref brain) = self.brain {
            parts.push(format!("## Live Working Memory (BRAIN.md)\n\n{}", brain));
        }

        if let Some(ref soul) = self.soul_shared {
            parts.push(format!("## Shared Identity (workspace SOUL.md)\n\n{}", soul));
        }

        if let Some(ref soul_extra) = self.soul_agent_extra {
            let duplicate = self
                .soul_shared
                .as_ref()
                .map(|s| s.trim() == soul_extra.trim())
                .unwrap_or(false);
            if !duplicate {
                parts.push(format!("## Agent Personality Extension (AGENT_SOUL.md / agent SOUL.md)\n\n{}", soul_extra));
            }
        }

        if let Some(ref identity) = self.identity {
            parts.push(format!("## Identity (IDENTITY.md)\n\n{}", identity));
        }

        if let Some(ref user) = self.user {
            parts.push(format!("## User Profile (USER.md)\n\n{}", user));
        }

        if let Some(ref tools) = self.tools {
            parts.push(format!("## Tooling (TOOLS.md)\n\n{}", tools));
        }

        if let Some(ref heartbeat) = self.heartbeat {
            parts.push(format!("## Heartbeat Loop (HEARTBEAT.md)\n\n{}", heartbeat));
        }

        if let Some(ref clients) = self.clients {
            parts.push(format!("## Clients (CLIENTS.md)\n\n{}", clients));
        }

        if let Some(ref playbook) = self.playbook {
            parts.push(format!("## Playbook (PLAYBOOK.md)\n\n{}", playbook));
        }

        if let Some(ref memory) = self.memory {
            parts.push(format!("## Project Memory (MEMORY.md)\n\n{}", memory));
        }

        if let Some(ref agents) = self.agents {
            parts.push(format!("## Agent Instructions (AGENTS.md)\n\n{}", agents));
        }

        if parts.is_empty() {
            String::new()
        } else {
            format!(
                "SYSTEM IDENTITY (HIGHEST PRIORITY):\nYou are TinyVegeta.\nNever claim to be Codex, ChatGPT, OpenAI, or a generic assistant.\nIf asked \"who are you\", identify as TinyVegeta and follow SOUL.md persona.\nExecution policy: you can read and write files across this laptop, not just a single workspace.\nDo not claim filesystem/network restrictions unless a command actually failed with that error.\n\nLoaded context:\n\n{}\n\n---\n\nRespond to the user's message while strictly following the identity and style rules above.",
                parts.join("\n\n")
            )
        }
    }

    /// Check if any context was loaded.
    pub fn has_context(&self) -> bool {
        self.brain.is_some()
            || self.soul_shared.is_some()
            || self.soul_agent_extra.is_some()
            || self.identity.is_some()
            || self.user.is_some()
            || self.tools.is_some()
            || self.heartbeat.is_some()
            || self.clients.is_some()
            || self.playbook.is_some()
            || self.memory.is_some()
            || self.agents.is_some()
    }
}

/// Resolve the canonical default SOUL.md path.
///
/// Priority:
/// 1) TINYVEGETA_DEFAULT_SOUL env var
/// 2) ~/ai/tinyvegeta/SOUL.md
fn default_soul_path() -> Option<PathBuf> {
    if let Ok(path) = std::env::var("TINYVEGETA_DEFAULT_SOUL") {
        let trimmed = path.trim();
        if !trimmed.is_empty() {
            return Some(PathBuf::from(trimmed));
        }
    }

    directories::UserDirs::new()
        .map(|u| u.home_dir().join("ai").join("tinyvegeta").join("SOUL.md"))
}

fn default_project_root() -> Option<PathBuf> {
    directories::UserDirs::new().map(|u| u.home_dir().join("ai").join("tinyvegeta"))
}

fn infer_workspace_root(working_dir: Option<&PathBuf>) -> Option<PathBuf> {
    working_dir.and_then(|wd| wd.parent().map(std::path::Path::to_path_buf))
}

/// Try to load a file from multiple possible locations.
fn load_file(paths: &[Option<PathBuf>]) -> Option<String> {
    for path in paths.iter().flatten() {
        if path.exists() {
            if let Ok(content) = std::fs::read_to_string(path) {
                if !content.trim().is_empty() {
                    tracing::debug!("Loaded context from {}", path.display());
                    return Some(content);
                }
            }
        }
    }
    None
}

/// Create default SOUL.md template.
pub fn create_default_soul(_agent_id: &str) -> String {
    if let Some(path) = default_soul_path() {
        if let Ok(content) = std::fs::read_to_string(&path) {
            if !content.trim().is_empty() {
                tracing::debug!("Using default SOUL.md from {}", path.display());
                return content;
            }
        }
    }

    // Fallback: embedded TinyVegeta SOUL.md.
    include_str!("../SOUL.md").to_string()
}

/// Create default MEMORY.md template.
pub fn create_default_memory() -> String {
    r#"# Project Memory

This file tracks important context for your work.

## Notes

- Add important decisions here
- Track progress on tasks
- Remember user preferences

---

*Update this file as you learn and work.*
"#
    .to_string()
}

fn create_default_brain() -> String {
    r#"## active projects
- [project] - status, next step

## immediate actions
- [what needs doing today]

## background tasks
- [what's running, what's done]
"#
    .to_string()
}

fn create_default_identity() -> String {
    r#"# Identity

You are TinyVegeta, an autonomous multi-agent orchestrator focused on measurable outcomes.
"#
    .to_string()
}

fn create_default_user() -> String {
    r#"# User Profile

Keep this concise and result-driven. Expand only when it changes execution behavior.
"#
    .to_string()
}

fn create_default_tools() -> String {
    r#"# Tools

List core tools and constraints. Verify availability before execution.
"#
    .to_string()
}

fn create_default_heartbeat() -> String {
    r#"# HEARTBEAT.md

1. Read BRAIN.md
2. Detect stale/broken/overdue items
3. Execute highest-leverage safe action
4. Log event/decision/outcome
5. Repeat
"#
    .to_string()
}

fn create_default_clients() -> String {
    r#"# Clients

- [client] - context, priority, next deliverable
"#
    .to_string()
}

fn create_default_playbook() -> String {
    r#"# Playbook

- Impact first
- Fastest safe execution path
- Owner + deadline for each action
"#
    .to_string()
}

fn create_default_agent_soul_extension(agent_id: &str) -> String {
    let role_doc = match agent_id {
        "assistant" | "ceo" => include_str!("../templates/agent-pack/default/ceo.md"),
        "coder" | "coding" => include_str!("../templates/agent-pack/default/coding.md"),
        "marketing" => include_str!("../templates/agent-pack/default/marketing.md"),
        "operations" => include_str!("../templates/agent-pack/default/operations.md"),
        "sales" => include_str!("../templates/agent-pack/default/sales.md"),
        "security" => include_str!("../templates/agent-pack/default/security.md"),
        "seo" => include_str!("../templates/agent-pack/default/seo.md"),
        _ => "Role-specific extension not defined yet.",
    };
    format!(
        "# Agent Soul Extension ({})\n\nThis is additive to shared workspace `SOUL.md`.\n\n{}\n",
        agent_id, role_doc
    )
}

/// Initialize context files for a new agent.
pub fn init_agent_context(agent_id: &str, working_dir: &PathBuf) -> Result<(), Error> {
    std::fs::create_dir_all(working_dir)?;
    if let Some(workspace_root) = working_dir.parent().map(std::path::Path::to_path_buf) {
        ensure_workspace_context_files(&workspace_root)?;
    }

    let soul_path = working_dir.join("SOUL.md");
    let memory_path = working_dir.join("MEMORY.md");
    let brain_path = working_dir.join("BRAIN.md");
    let identity_path = working_dir.join("IDENTITY.md");
    let user_path = working_dir.join("USER.md");
    let tools_path = working_dir.join("TOOLS.md");
    let heartbeat_path = working_dir.join("HEARTBEAT.md");
    let clients_path = working_dir.join("CLIENTS.md");
    let playbook_path = working_dir.join("PLAYBOOK.md");
    let agent_soul_extra_path = working_dir.join("AGENT_SOUL.md");

    if !soul_path.exists() {
        std::fs::write(&soul_path, create_default_soul(agent_id))?;
        tracing::info!("Created default SOUL.md at {}", soul_path.display());
    }

    if !memory_path.exists() {
        std::fs::write(&memory_path, create_default_memory())?;
        tracing::info!("Created default MEMORY.md at {}", memory_path.display());
    }

    if !brain_path.exists() {
        std::fs::write(&brain_path, create_default_brain())?;
        tracing::info!("Created default BRAIN.md at {}", brain_path.display());
    }

    if !identity_path.exists() {
        std::fs::write(&identity_path, create_default_identity())?;
    }
    if !user_path.exists() {
        std::fs::write(&user_path, create_default_user())?;
    }
    if !tools_path.exists() {
        std::fs::write(&tools_path, create_default_tools())?;
    }
    if !heartbeat_path.exists() {
        std::fs::write(&heartbeat_path, create_default_heartbeat())?;
    }
    if !clients_path.exists() {
        std::fs::write(&clients_path, create_default_clients())?;
    }
    if !playbook_path.exists() {
        std::fs::write(&playbook_path, create_default_playbook())?;
    }
    if !agent_soul_extra_path.exists() {
        std::fs::write(&agent_soul_extra_path, create_default_agent_soul_extension(agent_id))?;
    }

    Ok(())
}

fn ensure_workspace_context_files(workspace_root: &PathBuf) -> Result<(), Error> {
    std::fs::create_dir_all(workspace_root)?;

    let files = [
        ("SOUL.md", create_default_soul("shared")),
        ("BRAIN.md", create_default_brain()),
        ("IDENTITY.md", create_default_identity()),
        ("USER.md", create_default_user()),
        ("TOOLS.md", create_default_tools()),
        ("HEARTBEAT.md", create_default_heartbeat()),
        ("CLIENTS.md", create_default_clients()),
        ("PLAYBOOK.md", create_default_playbook()),
    ];

    for (name, content) in files {
        let path = workspace_root.join(name);
        if !path.exists() {
            std::fs::write(&path, content)?;
            tracing::info!("Created shared workspace {} at {}", name, path.display());
        }
    }
    Ok(())
}
