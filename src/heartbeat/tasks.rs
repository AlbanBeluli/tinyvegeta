//! Task spawning for heartbeat.
#![allow(dead_code)]

use std::process::Stdio;
use tokio::process::Command;
use serde_json::Value;

use crate::config::Settings;
use crate::providers::create_provider;
use crate::error::Error;

fn extract_cline_response(stdout: &str) -> String {
    let raw = stdout.trim();
    if raw.is_empty() {
        return String::new();
    }
    let mut candidate = String::new();
    for line in raw.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let Ok(v) = serde_json::from_str::<Value>(line) else {
            continue;
        };
        let typ = v.get("type").and_then(|t| t.as_str()).unwrap_or_default();
        let say = v.get("say").and_then(|s| s.as_str()).unwrap_or_default();
        if let Some(result) = v.get("result").and_then(|x| x.as_str()) {
            if !result.trim().is_empty() {
                return result.trim().to_string();
            }
        }
        if let Some(text) = v.get("text").and_then(|x| x.as_str()) {
            let text = text.trim();
            if !text.is_empty() && !(typ == "say" && (say == "task" || say == "plan")) {
                candidate = text.to_string();
            }
        }
    }
    if !candidate.is_empty() {
        candidate
    } else {
        raw.to_string()
    }
}

/// Task definition.
#[derive(Debug, Clone)]
pub struct Task {
    /// Task ID.
    pub id: String,
    
    /// Task title.
    pub title: String,
    
    /// Task description.
    pub description: Option<String>,
    
    /// Assigned agent.
    pub agent_id: Option<String>,
    
    /// Priority.
    pub priority: TaskPriority,
    
    /// Status.
    pub status: TaskStatus,
    
    /// Tags.
    pub tags: Vec<String>,
    
    /// Created at.
    pub created_at: i64,
    
    /// Updated at.
    pub updated_at: i64,
}

/// Task priority.
#[derive(Debug, Clone, PartialEq)]
pub enum TaskPriority {
    Low,
    Medium,
    High,
    Urgent,
}

impl std::fmt::Display for TaskPriority {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TaskPriority::Low => write!(f, "low"),
            TaskPriority::Medium => write!(f, "medium"),
            TaskPriority::High => write!(f, "high"),
            TaskPriority::Urgent => write!(f, "urgent"),
        }
    }
}

impl FromStr for TaskPriority {
    type Err = String;
    
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "low" => Ok(TaskPriority::Low),
            "medium" => Ok(TaskPriority::Medium),
            "high" => Ok(TaskPriority::High),
            "urgent" => Ok(TaskPriority::Urgent),
            _ => Err(format!("Unknown priority: {}", s)),
        }
    }
}

use std::str::FromStr;

/// Task status.
#[derive(Debug, Clone, PartialEq)]
pub enum TaskStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
}

impl std::fmt::Display for TaskStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TaskStatus::Pending => write!(f, "pending"),
            TaskStatus::Running => write!(f, "running"),
            TaskStatus::Completed => write!(f, "completed"),
            TaskStatus::Failed => write!(f, "failed"),
            TaskStatus::Cancelled => write!(f, "cancelled"),
        }
    }
}

impl Task {
    /// Create a new task.
    pub fn new(title: &str) -> Self {
        let now = chrono::Utc::now().timestamp_millis();
        let id = ulid::Ulid::new().to_string();
        
        Self {
            id,
            title: title.to_string(),
            description: None,
            agent_id: None,
            priority: TaskPriority::Medium,
            status: TaskStatus::Pending,
            tags: Vec::new(),
            created_at: now,
            updated_at: now,
        }
    }
    
    /// Set description.
    pub fn with_description(mut self, desc: &str) -> Self {
        self.description = Some(desc.to_string());
        self
    }
    
    /// Set agent.
    pub fn with_agent(mut self, agent_id: &str) -> Self {
        self.agent_id = Some(agent_id.to_string());
        self
    }
    
    /// Set priority.
    pub fn with_priority(mut self, priority: TaskPriority) -> Self {
        self.priority = priority;
        self
    }
    
    /// Add tag.
    pub fn with_tag(mut self, tag: &str) -> Self {
        self.tags.push(tag.to_string());
        self
    }
}

/// Task spawner.
pub struct TaskSpawner;

impl TaskSpawner {
    /// Run a heartbeat for an agent.
    pub async fn run_heartbeat(
        agent_id: &str,
        settings: &Settings,
    ) -> Result<String, Error> {
        let agent = settings.agents.get(agent_id)
            .ok_or_else(|| Error::NotFound(format!("Agent not found: {}", agent_id)))?;
        
        // Get agent heartbeat instructions if exists.
        let working_dir = agent.working_directory.clone()
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_default());
        
        let heartbeat_upper = working_dir.join("HEARTBEAT.md");
        let heartbeat_lower = working_dir.join("heartbeat.md");
        let prompt = if heartbeat_upper.exists() {
            std::fs::read_to_string(&heartbeat_upper)
                .unwrap_or_else(|_| "Perform your heartbeat check.".to_string())
        } else if heartbeat_lower.exists() {
            std::fs::read_to_string(&heartbeat_lower)
                .unwrap_or_else(|_| "Perform your heartbeat check.".to_string())
        } else {
            "Perform your heartbeat check and report status.".to_string()
        };
        
        // Get provider
        let provider_name = agent.provider.as_deref().unwrap_or(&settings.models.provider);
        let provider = create_provider(provider_name, settings);
        
        // Run completion
        let model = agent.model.as_deref();
        let contract = crate::agent::ExecutionContract::for_agent(provider_name);
        let result = crate::agent::execute_with_contract(
            provider,
            &prompt,
            model,
            Some(&working_dir),
            &contract,
        )
        .await
        .map_err(|e| Error::Provider(e.to_string()))?;
        
        tracing::info!("Heartbeat completed for {}: {} chars", agent_id, result.len());
        
        Ok(result)
    }
    
    /// Run a task in a tmux window.
    pub async fn spawn_task(
        task: &Task,
        settings: &Settings,
    ) -> Result<String, Error> {
        let agent_id = task.agent_id.as_ref()
            .ok_or_else(|| Error::Other("Task has no assigned agent".to_string()))?;
        
        let agent = settings.agents.get(agent_id)
            .ok_or_else(|| Error::NotFound(format!("Agent not found: {}", agent_id)))?;
        
        let working_dir = agent.working_directory.clone()
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_default());
        
        // Get provider
        let provider_name = agent.provider.as_deref().unwrap_or(&settings.models.provider);
        let provider = create_provider(provider_name, settings);
        
        // Build prompt
        let prompt = if let Some(desc) = &task.description {
            format!("{}\n\n{}", task.title, desc)
        } else {
            task.title.clone()
        };
        
        // Run completion
        let model = agent.model.as_deref();
        let contract = crate::agent::ExecutionContract::for_agent(provider_name);
        let result = crate::agent::execute_with_contract(
            provider,
            &prompt,
            model,
            Some(&working_dir),
            &contract,
        )
        .await
        .map_err(|e| Error::Provider(e.to_string()))?;
        
        tracing::info!("Task {} completed by {}", task.id, agent_id);
        
        Ok(result)
    }
    
    /// Invoke agent CLI directly.
    pub async fn invoke_agent_cli(
        agent_id: &str,
        prompt: &str,
        settings: &Settings,
    ) -> Result<String, Error> {
        let agent = settings.agents.get(agent_id)
            .ok_or_else(|| Error::NotFound(format!("Agent not found: {}", agent_id)))?;
        
        let working_dir = agent.working_directory.clone()
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_default());
        
        let provider_name = agent.provider.as_deref().unwrap_or(&settings.models.provider);
        
        // Determine CLI command based on provider
        let (cli, args) = match provider_name {
            "claude" => ("claude", vec!["-c", "-p", prompt]),
            "codex" => ("codex", vec!["complete", prompt]),
            "cline" => ("cline", vec!["task", prompt, "--json"]),
            "opencode" => ("opencode", vec!["complete", prompt]),
            _ => {
                // Use provider trait for HTTP providers
                let provider = create_provider(provider_name, settings);
                let model = agent.model.as_deref();
                let contract = crate::agent::ExecutionContract::for_agent(provider_name);
                return crate::agent::execute_with_contract(
                    provider,
                    prompt,
                    model,
                    Some(&working_dir),
                    &contract,
                )
                .await
                .map_err(|e| Error::Provider(e.to_string()));
            }
        };
        
        // Run CLI
        let output = Command::new(cli)
            .args(&args)
            .current_dir(&working_dir)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await?;
        
        if output.status.success() {
            let raw = String::from_utf8_lossy(&output.stdout).to_string();
            if provider_name == "cline" {
                Ok(extract_cline_response(&raw))
            } else {
                Ok(raw)
            }
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(Error::Provider(stderr.to_string()))
        }
    }
}

/// Team replication - spawn new agent instances.
pub fn spawn_team_agents(
    team_id: &str,
    settings: &Settings,
) -> Result<Vec<String>, Error> {
    let team = settings.teams.get(team_id)
        .ok_or_else(|| Error::NotFound(format!("Team not found: {}", team_id)))?;
    
    let mut spawned = Vec::new();
    
    for agent_id in &team.agents {
        if settings.agents.contains_key(agent_id) {
            spawned.push(agent_id.clone());
        } else {
            tracing::warn!("Team {} references missing agent {}", team_id, agent_id);
        }
    }
    
    Ok(spawned)
}
