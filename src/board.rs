//! Board orchestration and default agent-pack installation.
#![allow(dead_code)]

use std::path::{Path, PathBuf};

use crate::config::{AgentConfig, Settings, TeamConfig};
use crate::core::routing::{extract_mentions, find_team_for_agent, is_teammate};
use crate::error::{Error, Result};
use crate::heartbeat::tasks::TaskSpawner;
use crate::memory::{Memory, MemoryScope};
use serde::{Deserialize, Serialize};

struct PackAgent {
    id: &'static str,
    name: &'static str,
    role_md: &'static str,
}

const DEFAULT_PACK: &[PackAgent] = &[
    PackAgent {
        id: "assistant",
        name: "Assistant (CEO)",
        role_md: include_str!("../templates/agent-pack/default/ceo.md"),
    },
    PackAgent {
        id: "coder",
        name: "Coding Lead",
        role_md: include_str!("../templates/agent-pack/default/coding.md"),
    },
    PackAgent {
        id: "security",
        name: "Security Lead",
        role_md: include_str!("../templates/agent-pack/default/security.md"),
    },
    PackAgent {
        id: "operations",
        name: "Operations Lead",
        role_md: include_str!("../templates/agent-pack/default/operations.md"),
    },
    PackAgent {
        id: "marketing",
        name: "Marketing Lead",
        role_md: include_str!("../templates/agent-pack/default/marketing.md"),
    },
    PackAgent {
        id: "seo",
        name: "SEO Lead",
        role_md: include_str!("../templates/agent-pack/default/seo.md"),
    },
    PackAgent {
        id: "sales",
        name: "Sales Lead",
        role_md: include_str!("../templates/agent-pack/default/sales.md"),
    },
];

fn default_model_for_provider(provider: &str) -> String {
    match provider {
        "claude" | "codex" | "cline" | "opencode" => "default".to_string(),
        "grok" => "grok-2".to_string(),
        "ollama" => "llama3.3".to_string(),
        _ => "default".to_string(),
    }
}

fn ensure_role_overlay(agent_dir: &Path, role_md: &str) -> Result<()> {
    let soul_path = agent_dir.join("SOUL.md");
    let heading = role_md.lines().next().unwrap_or("").trim();
    let role_block = format!("## Role Overlay\n\n{}", role_md.trim());

    if soul_path.exists() {
        let current = std::fs::read_to_string(&soul_path)?;
        if !heading.is_empty() && current.contains(heading) {
            return Ok(());
        }
        let merged = format!("{}\n\n---\n\n{}\n", current.trim_end(), role_block);
        std::fs::write(&soul_path, merged)?;
    } else {
        std::fs::write(&soul_path, format!("{}\n", role_block))?;
    }

    let memory_path = agent_dir.join("MEMORY.md");
    if !memory_path.exists() {
        std::fs::write(&memory_path, crate::context::create_default_memory())?;
    }

    Ok(())
}

/// Install default board agents from embedded templates.
pub fn install_default_pack(settings: &mut Settings, workspace_root: &Path) -> Result<()> {
    let primary_provider = settings
        .agents
        .get("assistant")
        .and_then(|a| a.provider.clone())
        .unwrap_or_else(|| settings.models.provider.clone());
    let primary_model = settings
        .agents
        .get("assistant")
        .and_then(|a| a.model.clone())
        .unwrap_or_else(|| default_model_for_provider(&primary_provider));

    for spec in DEFAULT_PACK {
        let dir = workspace_root.join(spec.id);
        std::fs::create_dir_all(&dir)?;
        crate::context::init_agent_context(spec.id, &dir)?;
        ensure_role_overlay(&dir, spec.role_md)?;

        let entry = settings
            .agents
            .entry(spec.id.to_string())
            .or_insert_with(AgentConfig::default);

        if entry.name.is_none() {
            entry.name = Some(spec.name.to_string());
        }
        if entry.provider.is_none() {
            entry.provider = Some(primary_provider.clone());
        }
        if entry.model.is_none() {
            entry.model = Some(primary_model.clone());
        }
        if entry.working_directory.is_none() {
            entry.working_directory = Some(dir);
        }
    }

    settings.teams.insert(
        "board".to_string(),
        TeamConfig {
            name: "Executive Board".to_string(),
            agents: DEFAULT_PACK.iter().map(|a| a.id.to_string()).collect(),
            leader_agent: Some("assistant".to_string()),
        },
    );

    settings.board.team_id = Some("board".to_string());
    settings.board.autonomous = Some(true);
    if settings.routing.default_agent.is_none() {
        settings.routing.default_agent = Some("assistant".to_string());
    }
    if settings.board.schedules.is_none() {
        settings.board.schedules = Some(Vec::new());
    }
    if settings.monitoring.heartbeat_interval == 0 {
        settings.monitoring.heartbeat_interval = 3600;
    }

    Ok(())
}

/// Run a board discussion and return the synthesized decision.
pub async fn run_board_discussion(
    settings: &Settings,
    team_id: &str,
    topic: &str,
    _timeout_secs: Option<u64>,
) -> Result<String> {
    let team = settings
        .teams
        .get(team_id)
        .ok_or_else(|| Error::NotFound(format!("Team not found: {}", team_id)))?;

    let ceo = team
        .leader_agent
        .clone()
        .or_else(|| team.agents.first().cloned())
        .ok_or_else(|| Error::Other(format!("Team {} has no members", team_id)))?;

    let mut member_inputs = Vec::new();
    for member in &team.agents {
        if member == &ceo {
            continue;
        }
        if !settings.agents.contains_key(member) {
            continue;
        }

        let prompt = format!(
            "You are @{} in the {} board.\n\nTopic:\n{}\n\nGive your expert recommendation in 5-8 bullets: risks, opportunities, and next action.",
            member, team_id, topic
        );

        let response = TaskSpawner::invoke_agent_cli(member, &prompt, settings)
            .await
            .unwrap_or_else(|e| format!("Error from @{}: {}", member, e));

        member_inputs.push((member.clone(), response.trim().to_string()));
    }

    let mut synthesis = String::new();
    for (member, input) in &member_inputs {
        synthesis.push_str(&format!("@{} input:\n{}\n\n", member, input));
    }

    let ceo_prompt = format!(
        "You are @{} and lead board @{}.\n\nTopic:\n{}\n\nRecent team memory:\n{}\n\nBoard inputs:\n{}\nProvide final decision with:\nDECISION\nRATIONALE\nNEXT STEPS with @owner.",
        ceo,
        team_id,
        topic,
        render_recent_team_memory(team_id, topic),
        synthesis
    );

    let ceo_decision = TaskSpawner::invoke_agent_cli(&ceo, &ceo_prompt, settings)
        .await
        .unwrap_or_else(|e| format!("CEO synthesis failed: {}", e));

    let output = format!(
        "Board @{} discussion on: {}\n\n{}\nCEO (@{}) decision:\n{}",
        team_id,
        topic,
        synthesis.trim(),
        ceo,
        ceo_decision.trim()
    );

    persist_board_decision(team_id, topic, ceo_decision.trim())?;
    Ok(output)
}

/// Execute mention-based delegations from team leader response.
pub async fn execute_leader_delegations(
    settings: &Settings,
    current_agent_id: &str,
    response: &str,
) -> Result<Vec<(String, String)>> {
    let (team_id, team) = match find_team_for_agent(current_agent_id, &settings.teams) {
        Some(v) => v,
        None => return Ok(Vec::new()),
    };

    if team.leader_agent.as_deref() != Some(current_agent_id) {
        return Ok(Vec::new());
    }

    let mentions = extract_mentions(response);
    if mentions.is_empty() {
        return Ok(Vec::new());
    }

    let mut results = Vec::new();
    for (target, delegated_prompt) in mentions {
        if !is_teammate(
            &target,
            current_agent_id,
            &team_id,
            &settings.teams,
            &settings.agents,
        ) {
            continue;
        }

        let delegation_id = ulid::Ulid::new().to_string();
        persist_delegation_result(
            &team_id,
            &delegation_id,
            current_agent_id,
            &target,
            &delegated_prompt,
            "open",
            "",
        )?;
        persist_delegation_result(
            &team_id,
            &delegation_id,
            current_agent_id,
            &target,
            &delegated_prompt,
            "in_progress",
            "",
        )?;

        let out = TaskSpawner::invoke_agent_cli(&target, &delegated_prompt, settings)
            .await
            .unwrap_or_else(|e| format!("Delegation failed for @{}: {}", target, e));
        let status = if out.to_lowercase().contains("failed") || out.to_lowercase().contains("error") {
            "blocked"
        } else {
            "done"
        };
        persist_delegation_result(
            &team_id,
            &delegation_id,
            current_agent_id,
            &target,
            &delegated_prompt,
            status,
            &out,
        )?;
        results.push((target, out.trim().to_string()));
    }

    Ok(results)
}

fn render_recent_team_memory(team_id: &str, query: &str) -> String {
    match Memory::relevant(query, MemoryScope::Team, Some(team_id), 8) {
        Ok(entries) if !entries.is_empty() => entries
            .iter()
            .map(|e| format!("- {}: {}", e.key, e.value.chars().take(220).collect::<String>()))
            .collect::<Vec<_>>()
            .join("\n"),
        _ => "None".to_string(),
    }
}

fn persist_board_decision(team_id: &str, topic: &str, decision_text: &str) -> Result<()> {
    let id = ulid::Ulid::new().to_string();
    let key = format!("board.decision.{}", id);
    let structured = parse_board_decision(decision_text);
    let record = serde_json::json!({
        "decision_id": id,
        "topic": topic,
        "decision": structured.decision,
        "owners": structured.owners,
        "deadlines": structured.deadlines,
        "risks": structured.risks,
        "raw": decision_text,
        "created_at": chrono::Utc::now().to_rfc3339(),
        "confidence": "medium"
    });
    validate_decision_schema(&record)?;
    Memory::set(
        &key,
        &record.to_string(),
        MemoryScope::Team,
        Some(team_id),
    )?;
    // Also keep a short pointer for fast retrieval.
    Memory::set(
        "board.last_decision",
        &format!("{} | {}", topic, decision_text.chars().take(240).collect::<String>()),
        MemoryScope::Team,
        Some(team_id),
    )?;
    Ok(())
}

fn persist_delegation_result(
    team_id: &str,
    delegation_id: &str,
    owner: &str,
    target: &str,
    task: &str,
    status: &str,
    output: &str,
) -> Result<()> {
    let key = format!("delegation.{}", delegation_id);
    let record = serde_json::json!({
        "delegation_id": delegation_id,
        "owner": owner,
        "target": target,
        "task": task,
        "status": status,
        "updated_at": chrono::Utc::now().to_rfc3339(),
        "output": output.chars().take(1500).collect::<String>()
    });
    Memory::set(&key, &record.to_string(), MemoryScope::Team, Some(team_id))?;
    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ParsedDecision {
    decision: String,
    owners: Vec<String>,
    deadlines: Vec<String>,
    risks: Vec<String>,
}

fn parse_board_decision(text: &str) -> ParsedDecision {
    let mut decision = String::new();
    let mut owners = Vec::new();
    let mut deadlines = Vec::new();
    let mut risks = Vec::new();

    for line in text.lines() {
        let l = line.trim();
        if l.is_empty() {
            continue;
        }
        let lower = l.to_lowercase();

        if decision.is_empty() && (lower.starts_with("decision:") || lower.starts_with("decision ")) {
            decision = l.split_once(':').map(|(_, v)| v.trim().to_string()).unwrap_or_else(|| l.to_string());
        }
        if l.contains("@") {
            for token in l.split_whitespace() {
                if let Some(owner) = token.strip_prefix('@') {
                    let owner = owner.trim_matches(|c: char| !c.is_ascii_alphanumeric() && c != '_' && c != '-');
                    if !owner.is_empty() && !owners.contains(&owner.to_string()) {
                        owners.push(owner.to_string());
                    }
                }
            }
        }
        if lower.contains("deadline") || lower.contains("due") || lower.contains("eta") {
            deadlines.push(l.to_string());
        }
        if lower.contains("risk") || lower.contains("blocker") || lower.contains("concern") {
            risks.push(l.to_string());
        }
    }

    if decision.is_empty() {
        decision = text.lines().take(4).collect::<Vec<_>>().join(" ").trim().to_string();
    }
    if owners.is_empty() {
        owners.push("assistant".to_string());
    }

    ParsedDecision {
        decision,
        owners,
        deadlines,
        risks,
    }
}

fn validate_decision_schema(record: &serde_json::Value) -> Result<()> {
    let decision_ok = record.get("decision").and_then(|v| v.as_str()).map(|s| !s.trim().is_empty()).unwrap_or(false);
    let owners_ok = record
        .get("owners")
        .and_then(|v| v.as_array())
        .map(|a| !a.is_empty())
        .unwrap_or(false);
    if !decision_ok || !owners_ok {
        return Err(Error::Other("Board decision schema validation failed".to_string()));
    }
    Ok(())
}

pub fn run_delegation_followup(team_id: &str, max_age_hours: i64) -> Result<Vec<String>> {
    let now = chrono::Utc::now();
    let entries = Memory::list(MemoryScope::Team, Some(team_id), None)?
        .into_iter()
        .filter(|e| e.key.starts_with("delegation."))
        .collect::<Vec<_>>();
    let mut overdue = Vec::new();
    for e in entries {
        let Ok(v) = serde_json::from_str::<serde_json::Value>(&e.value) else {
            continue;
        };
        let status = v.get("status").and_then(|s| s.as_str()).unwrap_or("unknown");
        if !matches!(status, "open" | "in_progress" | "blocked") {
            continue;
        }
        let updated = v.get("updated_at").and_then(|s| s.as_str()).unwrap_or_default();
        let Ok(ts) = chrono::DateTime::parse_from_rfc3339(updated) else {
            continue;
        };
        let age = now.signed_duration_since(ts.with_timezone(&chrono::Utc)).num_hours();
        if age >= max_age_hours {
            let target = v.get("target").and_then(|s| s.as_str()).unwrap_or("unknown");
            let task = v.get("task").and_then(|s| s.as_str()).unwrap_or("");
            overdue.push(format!("@{} overdue {}h: {}", target, age, task));
        }
    }
    Ok(overdue)
}

/// Determine workspace root from settings or fallback to ~/tinyvegeta-workspace.
pub fn resolve_workspace_root(settings: &Settings) -> PathBuf {
    settings
        .workspace
        .path
        .clone()
        .or_else(|| directories::UserDirs::new().map(|u| u.home_dir().join("tinyvegeta-workspace")))
        .unwrap_or_else(|| PathBuf::from("./tinyvegeta-workspace"))
}

#[cfg(test)]
mod tests {
    use super::{parse_board_decision, validate_decision_schema};

    #[test]
    fn parses_decision_fields() {
        let text = "DECISION: Ship v1 now\nOwner: @assistant @coder\nRisk: auth regression\nDeadline: today 18:00";
        let parsed = parse_board_decision(text);
        assert!(parsed.decision.to_lowercase().contains("ship v1"));
        assert!(parsed.owners.contains(&"assistant".to_string()));
        assert!(!parsed.risks.is_empty());
        assert!(!parsed.deadlines.is_empty());
    }

    #[test]
    fn validates_decision_schema() {
        let record = serde_json::json!({
            "decision": "Ship",
            "owners": ["assistant"]
        });
        assert!(validate_decision_schema(&record).is_ok());
    }
}
