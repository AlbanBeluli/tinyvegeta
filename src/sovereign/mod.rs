//! Sovereign runtime: continuous think -> act -> observe loop with safety rails.

use anyhow::{anyhow, Result};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use tokio::process::Command;

use crate::config::{get_home_dir, get_settings_path, load_settings, BoardSchedule, Settings};
use crate::memory::Memory;
use crate::providers::create_provider;

const DEFAULT_CONSTITUTION: &str = include_str!("../../constitution/LAWS.md");

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SovereignPlan {
    thought: String,
    #[serde(default)]
    actions: Vec<SovereignAction>,
    #[serde(default)]
    sleep_seconds: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum SovereignAction {
    Shell { cmd: String, reason: Option<String> },
    WriteFile {
        path: String,
        content: String,
        #[serde(default)]
        append: bool,
    },
    MemorySet {
        key: String,
        value: String,
        #[serde(default)]
        scope: Option<String>,
        scope_id: Option<String>,
    },
    ScheduleSet {
        schedule_type: String,
        time: String,
        team_id: Option<String>,
        agent_id: Option<String>,
    },
    SkillCreate { name: String, content: String },
    ReplicateAgent {
        new_agent_id: String,
        provider: Option<String>,
        model: Option<String>,
    },
}

#[derive(Debug, Serialize)]
struct AuditEntry {
    ts: String,
    agent_id: String,
    cycle: u64,
    action: String,
    status: String,
    detail: String,
}

#[derive(Debug, Default)]
struct SelfModifyWindow {
    seen: VecDeque<i64>,
}

impl SelfModifyWindow {
    fn allow(&mut self, limit: usize) -> bool {
        let now = Utc::now().timestamp();
        while let Some(front) = self.seen.front() {
            if now - *front > 3600 {
                let _ = self.seen.pop_front();
            } else {
                break;
            }
        }
        if self.seen.len() >= limit {
            return false;
        }
        self.seen.push_back(now);
        true
    }
}

pub async fn run(
    agent_id: Option<String>,
    goal: Option<String>,
    max_cycles: Option<u32>,
    dry_run: bool,
) -> Result<()> {
    let mut settings = load_settings().map_err(|e| anyhow!(e.to_string()))?;
    let resolved_agent = resolve_agent(&settings, agent_id)?;
    let agent_cfg = settings
        .agents
        .get(&resolved_agent)
        .ok_or_else(|| anyhow!("Agent '{}' not found", resolved_agent))?
        .clone();
    let working_dir = agent_cfg
        .working_directory
        .clone()
        .unwrap_or(std::env::current_dir()?);
    let provider_name = agent_cfg
        .provider
        .as_deref()
        .unwrap_or(&settings.models.provider)
        .to_string();
    let model = agent_cfg.model.clone();
    let constitution = load_constitution(&settings)?;
    let loop_sleep_default = settings.sovereign.loop_sleep_seconds.max(5);
    let max_actions = settings.sovereign.max_actions_per_cycle.max(1) as usize;
    let mut cycle: u64 = 0;
    let mut mod_window = SelfModifyWindow::default();

    loop {
        cycle += 1;
        if let Some(max) = max_cycles {
            if cycle > max as u64 {
                break;
            }
        }

        let prompt = build_prompt(
            &constitution,
            &resolved_agent,
            &working_dir,
            &settings,
            goal.as_deref().unwrap_or("Improve TinyVegeta safely and measurably."),
            max_actions,
        );
        let provider = create_provider(&provider_name, &settings);
        let reply = provider
            .complete(&prompt, model.as_deref(), Some(&working_dir))
            .await
            .map_err(|e| anyhow!("Provider error: {}", e))?;
        let plan = parse_plan(&reply).unwrap_or(SovereignPlan {
            thought: "No valid plan produced; observing and waiting.".to_string(),
            actions: Vec::new(),
            sleep_seconds: Some(loop_sleep_default),
        });

        append_audit(AuditEntry {
            ts: Utc::now().to_rfc3339(),
            agent_id: resolved_agent.clone(),
            cycle,
            action: "thought".to_string(),
            status: "ok".to_string(),
            detail: plan.thought.clone(),
        })?;

        for action in plan.actions.into_iter().take(max_actions) {
            let action_name = action_name(&action).to_string();
            let execution = execute_action(
                &mut settings,
                &resolved_agent,
                &working_dir,
                action,
                dry_run,
                &mut mod_window,
            )
            .await;
            let (status, detail) = match execution {
                Ok(d) => ("ok".to_string(), d),
                Err(e) => ("blocked".to_string(), e.to_string()),
            };
            append_audit(AuditEntry {
                ts: Utc::now().to_rfc3339(),
                agent_id: resolved_agent.clone(),
                cycle,
                action: action_name,
                status: status.clone(),
                detail: detail.clone(),
            })?;
            let key = format!("sovereign.cycle.{}.{}", cycle, Utc::now().timestamp_millis());
            let val = serde_json::json!({ "status": status, "detail": detail }).to_string();
            let _ = Memory::set(&key, &val, crate::memory::MemoryScope::Global, None);
        }

        let sleep_for = plan.sleep_seconds.unwrap_or(loop_sleep_default).max(5);
        tokio::time::sleep(std::time::Duration::from_secs(sleep_for)).await;
    }

    Ok(())
}

fn resolve_agent(settings: &Settings, agent_id: Option<String>) -> Result<String> {
    if let Some(agent_id) = agent_id {
        return Ok(agent_id);
    }
    if let Some(id) = settings.routing.default_agent.as_ref() {
        return Ok(id.clone());
    }
    if settings.agents.contains_key("assistant") {
        return Ok("assistant".to_string());
    }
    settings
        .agents
        .keys()
        .next()
        .cloned()
        .ok_or_else(|| anyhow!("No agents configured"))
}

fn load_constitution(settings: &Settings) -> Result<String> {
    if let Some(path) = settings.sovereign.constitution_path.as_ref() {
        if path.exists() {
            return Ok(std::fs::read_to_string(path)?);
        }
    }
    Ok(DEFAULT_CONSTITUTION.to_string())
}

fn build_prompt(
    constitution: &str,
    agent_id: &str,
    working_dir: &Path,
    settings: &Settings,
    goal: &str,
    max_actions: usize,
) -> String {
    format!(
        "SYSTEM: You are TinyVegeta sovereign runtime.\n\
         Constitution is immutable and highest priority:\n{}\n\n\
         Runtime context:\n- agent_id: {}\n- working_directory: {}\n- workspace_root: {}\n- board_team_id: {}\n\n\
         Goal:\n{}\n\n\
         Return JSON only with this schema:\n\
         {{\"thought\":\"...\",\"actions\":[...],\"sleep_seconds\":20}}\n\
         Allowed action types: shell, write_file, memory_set, schedule_set, skill_create, replicate_agent.\n\
         Hard limits: max {} actions. Do not request harmful, deceptive, or unauthorized actions.",
        constitution,
        agent_id,
        working_dir.display(),
        settings
            .workspace
            .path
            .clone()
            .unwrap_or_else(|| working_dir.to_path_buf())
            .display(),
        settings.board.team_id.clone().unwrap_or_else(|| "none".to_string()),
        goal,
        max_actions
    )
}

fn parse_plan(reply: &str) -> Option<SovereignPlan> {
    serde_json::from_str::<SovereignPlan>(reply)
        .ok()
        .or_else(|| {
            let start = reply.find('{')?;
            let end = reply.rfind('}')?;
            if end <= start {
                return None;
            }
            serde_json::from_str::<SovereignPlan>(&reply[start..=end]).ok()
        })
}

async fn execute_action(
    settings: &mut Settings,
    agent_id: &str,
    working_dir: &Path,
    action: SovereignAction,
    dry_run: bool,
    mod_window: &mut SelfModifyWindow,
) -> Result<String> {
    match action {
        SovereignAction::Shell { cmd, reason: _ } => {
            guard_shell(&cmd)?;
            if !settings.sovereign.allow_tool_install && looks_like_tool_install(&cmd) {
                return Err(anyhow!("tool install blocked by policy"));
            }
            if dry_run {
                return Ok(format!("dry-run shell: {}", cmd));
            }
            let output = Command::new("zsh")
                .arg("-lc")
                .arg(&cmd)
                .current_dir(working_dir)
                .output()
                .await?;
            let status = output.status.code().unwrap_or(-1);
            let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            Ok(format!("exit={} stdout='{}' stderr='{}'", status, stdout, stderr))
        }
        SovereignAction::WriteFile {
            path,
            content,
            append,
        } => {
            let target = normalize_path(working_dir, &path)?;
            guard_file_write(settings, &target)?;
            if !settings.sovereign.allow_self_modify {
                return Err(anyhow!("self-modifying file writes are disabled by policy"));
            }
            if !mod_window.allow(settings.sovereign.max_self_modifications_per_hour as usize) {
                return Err(anyhow!("self-modification rate limit reached"));
            }
            if dry_run {
                return Ok(format!("dry-run write: {}", target.display()));
            }
            if let Some(parent) = target.parent() {
                std::fs::create_dir_all(parent)?;
            }
            if append {
                use std::io::Write;
                let mut file = std::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(&target)?;
                file.write_all(content.as_bytes())?;
            } else {
                std::fs::write(&target, content)?;
            }
            Ok(format!("wrote {}", target.display()))
        }
        SovereignAction::MemorySet {
            key,
            value,
            scope,
            scope_id,
        } => {
            if dry_run {
                return Ok(format!("dry-run memory set: {}", key));
            }
            let parsed_scope = parse_scope(scope.as_deref().unwrap_or("global"))?;
            Memory::set(&key, &value, parsed_scope, scope_id.as_deref())?;
            Ok(format!("memory set {}", key))
        }
        SovereignAction::ScheduleSet {
            schedule_type,
            time,
            team_id,
            agent_id: target_agent,
        } => {
            if dry_run {
                return Ok(format!("dry-run schedule {} {}", schedule_type, time));
            }
            let mut schedules = settings.board.schedules.clone().unwrap_or_default();
            let schedule = BoardSchedule {
                id: ulid::Ulid::new().to_string(),
                schedule_type,
                time,
                team_id,
                agent_id: target_agent.or_else(|| Some(agent_id.to_string())),
                sender_id: None,
                enabled: true,
            };
            schedules.push(schedule);
            settings.board.schedules = Some(schedules);
            save_settings(settings)?;
            Ok("schedule added".to_string())
        }
        SovereignAction::SkillCreate { name, content } => {
            if dry_run {
                return Ok(format!("dry-run skill create: {}", name));
            }
            let skill_dir = get_home_dir()?.join("skills").join(&name);
            std::fs::create_dir_all(&skill_dir)?;
            std::fs::write(skill_dir.join("SKILL.md"), content)?;
            Ok(format!("skill created: {}", name))
        }
        SovereignAction::ReplicateAgent {
            new_agent_id,
            provider,
            model,
        } => {
            if dry_run {
                return Ok(format!("dry-run replicate agent: {}", new_agent_id));
            }
            if settings.agents.contains_key(&new_agent_id) {
                return Err(anyhow!("agent '{}' already exists", new_agent_id));
            }
            let workspace_root = settings
                .workspace
                .path
                .clone()
                .unwrap_or_else(|| working_dir.to_path_buf());
            let agent_dir = workspace_root.join(&new_agent_id);
            std::fs::create_dir_all(&agent_dir)?;
            std::fs::write(agent_dir.join("SOUL.md"), format!("# {} SOUL\n", new_agent_id))?;
            std::fs::write(agent_dir.join("MEMORY.md"), "# Memory\n")?;
            settings.agents.insert(
                new_agent_id.clone(),
                crate::config::AgentConfig {
                    name: Some(new_agent_id.clone()),
                    provider,
                    model,
                    working_directory: Some(agent_dir),
                    is_sovereign: true,
                },
            );
            save_settings(settings)?;
            Ok(format!("replicated new agent {}", new_agent_id))
        }
    }
}

fn normalize_path(base: &Path, requested: &str) -> Result<PathBuf> {
    let p = PathBuf::from(requested);
    let full = if p.is_absolute() { p } else { base.join(p) };
    Ok(full.canonicalize().unwrap_or(full))
}

fn parse_scope(scope: &str) -> Result<crate::memory::MemoryScope> {
    match scope {
        "global" => Ok(crate::memory::MemoryScope::Global),
        "agent" => Ok(crate::memory::MemoryScope::Agent),
        "team" => Ok(crate::memory::MemoryScope::Team),
        "task" => Ok(crate::memory::MemoryScope::Task),
        _ => Err(anyhow!("invalid memory scope: {}", scope)),
    }
}

fn guard_shell(cmd: &str) -> Result<()> {
    let blocked = ["rm -rf /", ":(){:|:&};:", "mkfs", "dd if=", "shutdown", "reboot"];
    if blocked.iter().any(|x| cmd.contains(x)) {
        return Err(anyhow!("blocked shell command by sovereign guard"));
    }
    Ok(())
}

fn looks_like_tool_install(cmd: &str) -> bool {
    ["brew install", "apt install", "cargo install", "npm i -g", "pip install"]
        .iter()
        .any(|x| cmd.contains(x))
}

fn guard_file_write(settings: &Settings, path: &Path) -> Result<()> {
    let protected: Vec<PathBuf> = settings
        .sovereign
        .protected_files
        .iter()
        .map(PathBuf::from)
        .collect();

    for p in protected {
        if path.ends_with(&p) {
            return Err(anyhow!("write blocked for protected file '{}'", p.display()));
        }
    }
    Ok(())
}

fn save_settings(settings: &Settings) -> Result<()> {
    let path = get_settings_path()?;
    std::fs::write(path, serde_json::to_string_pretty(settings)?)?;
    Ok(())
}

fn action_name(action: &SovereignAction) -> &'static str {
    match action {
        SovereignAction::Shell { .. } => "shell",
        SovereignAction::WriteFile { .. } => "write_file",
        SovereignAction::MemorySet { .. } => "memory_set",
        SovereignAction::ScheduleSet { .. } => "schedule_set",
        SovereignAction::SkillCreate { .. } => "skill_create",
        SovereignAction::ReplicateAgent { .. } => "replicate_agent",
    }
}

fn append_audit(entry: AuditEntry) -> Result<()> {
    let dir = get_home_dir()?.join("audit");
    std::fs::create_dir_all(&dir)?;
    let line = serde_json::to_string(&entry)?;
    use std::io::Write;
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(dir.join("sovereign.jsonl"))?;
    writeln!(file, "{}", line)?;
    Ok(())
}
