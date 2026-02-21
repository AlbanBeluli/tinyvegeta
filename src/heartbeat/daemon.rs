//! Heartbeat daemon for autonomous agent operations.
#![allow(dead_code)]

use std::sync::Arc;
use std::time::Duration;
use std::io::Write;
use tokio::sync::RwLock;
use tokio::time::sleep;

use crate::config::{get_home_dir, load_settings, Settings};
use crate::error::Error;
use crate::memory::{Memory, MemoryScope};

use super::scheduler::{HeartbeatSchedule, ScheduleManager};
use super::tasks::TaskSpawner;

/// Heartbeat daemon.
pub struct HeartbeatDaemon {
    settings: Arc<RwLock<Settings>>,
    schedules: Arc<RwLock<ScheduleManager>>,
    running: Arc<RwLock<bool>>,
}

impl HeartbeatDaemon {
    /// Create a new heartbeat daemon.
    pub fn new(settings: Settings) -> Self {
        let mut manager = ScheduleManager::new();
        
        // Add default heartbeat schedule
        let schedule = HeartbeatSchedule::interval(settings.monitoring.heartbeat_interval);
        manager.add(schedule);
        manager.update_next_runs();
        
        Self {
            settings: Arc::new(RwLock::new(settings)),
            schedules: Arc::new(RwLock::new(manager)),
            running: Arc::new(RwLock::new(false)),
        }
    }
    
    /// Create with custom schedules.
    pub fn with_schedules(settings: Settings, schedules: Vec<HeartbeatSchedule>) -> Self {
        let mut manager = ScheduleManager::new();
        
        for schedule in schedules {
            manager.add(schedule);
        }
        manager.update_next_runs();
        
        Self {
            settings: Arc::new(RwLock::new(settings)),
            schedules: Arc::new(RwLock::new(manager)),
            running: Arc::new(RwLock::new(false)),
        }
    }
    
    /// Start the daemon.
    pub async fn start(&self) -> Result<(), Error> {
        let mut running = self.running.write().await;
        if *running {
            return Err(Error::Other("Daemon already running".to_string()));
        }
        *running = true;
        drop(running);
        
        tracing::info!("Heartbeat daemon started");
        
        // Main loop
        loop {
            // Check if we should stop
            {
                let running = self.running.read().await;
                if !*running {
                    tracing::info!("Heartbeat daemon stopping");
                    break;
                }
            }
            
            // Check for due schedules
            {
                let schedules = self.schedules.read().await;
                let due = schedules.due();
                
                for schedule in due {
                    tracing::debug!("Processing schedule: {}", schedule.id);
                    
                    let settings = self.settings.read().await.clone();
                    
                    // Execute the schedule
                    if let Some(agent_id) = &schedule.agent_id {
                        match TaskSpawner::run_heartbeat(agent_id, &settings).await {
                            Ok(result) => {
                                tracing::info!("Heartbeat completed for {}: {} bytes", 
                                    agent_id, result.len());
                            }
                            Err(e) => {
                                tracing::error!("Heartbeat failed for {}: {}", agent_id, e);
                            }
                        }
                    }
                }
            }
            
            // Update next runs
            {
                let mut schedules = self.schedules.write().await;
                schedules.update_next_runs();
            }

            // Execute persisted board schedules and follow-ups.
            {
                let settings = self.settings.read().await.clone();
                if let Err(e) = execute_board_schedules(&settings).await {
                    tracing::warn!("Board schedule execution warning: {}", e);
                }
                if let Err(e) = run_delegation_followups(&settings).await {
                    tracing::warn!("Delegation follow-up warning: {}", e);
                }
                if let Err(e) = run_brain_proactive_checks(&settings).await {
                    tracing::warn!("BRAIN proactive check warning: {}", e);
                }
                if let Err(e) = run_system_maintenance(&settings).await {
                    tracing::warn!("System maintenance warning: {}", e);
                }
            }
            
            // Sleep for a bit
            sleep(Duration::from_secs(10)).await;
        }
        
        Ok(())
    }
    
    /// Stop the daemon.
    pub async fn stop(&self) {
        let mut running = self.running.write().await;
        *running = false;
        tracing::info!("Heartbeat daemon stopped");
    }
    
    /// Add a schedule.
    pub async fn add_schedule(&self, schedule: HeartbeatSchedule) {
        let mut schedules = self.schedules.write().await;
        schedules.add(schedule);
        schedules.update_next_runs();
    }
    
    /// Remove a schedule.
    pub async fn remove_schedule(&self, id: &str) -> Option<HeartbeatSchedule> {
        let mut schedules = self.schedules.write().await;
        schedules.remove(id)
    }
    
    /// List schedules.
    pub async fn list_schedules(&self) -> Vec<HeartbeatSchedule> {
        let schedules = self.schedules.read().await;
        schedules.list().to_vec()
    }
    
    /// Run a single heartbeat for an agent.
    pub async fn run_heartbeat(agent_id: &str) -> Result<String, Error> {
        let settings = load_settings()?;
        TaskSpawner::run_heartbeat(agent_id, &settings).await
    }
}

/// Run the heartbeat daemon.
pub async fn run_heartbeat_daemon() -> Result<(), Error> {
    tracing::info!("Starting heartbeat daemon...");
    
    let settings = load_settings()?;
    let daemon = HeartbeatDaemon::new(settings);
    
    // Handle Ctrl+C
    let running = daemon.running.clone();
    tokio::spawn(async move {
        tokio::signal::ctrl_c().await.ok();
        let mut r = running.write().await;
        *r = false;
    });
    
    daemon.start().await
}

/// Run a single heartbeat for an agent.
pub async fn run_single_heartbeat(agent_id: &str) -> Result<String, Error> {
    HeartbeatDaemon::run_heartbeat(agent_id).await
}

fn should_run_schedule(id: &str, hhmm: &str, schedule_type: &str) -> bool {
    let now = chrono::Local::now().format("%H:%M").to_string();
    if hhmm != now {
        return false;
    }
    let today = chrono::Local::now().format("%Y-%m-%d").to_string();
    let last_key = format!("board.schedule.last_run.{}", id);
    match Memory::get(&last_key, MemoryScope::Global, None) {
        Ok(Some(entry)) => !(entry.value == today && schedule_type != "digest"),
        _ => true,
    }
}

fn mark_schedule_run(id: &str) {
    let today = chrono::Local::now().format("%Y-%m-%d").to_string();
    let _ = Memory::set(
        &format!("board.schedule.last_run.{}", id),
        &today,
        MemoryScope::Global,
        None,
    );
}

fn log_schedule_attempt(id: &str, ok: bool, detail: &str) {
    let ts = chrono::Utc::now().to_rfc3339();
    let key = format!("board.schedule.log.{}.{}", id, ulid::Ulid::new());
    let rec = serde_json::json!({
        "schedule_id": id,
        "success": ok,
        "timestamp": ts,
        "detail": detail
    });
    let _ = Memory::set(&key, &rec.to_string(), MemoryScope::Global, None);
}

async fn execute_board_schedules(settings: &Settings) -> Result<(), Error> {
    let Some(schedules) = settings.board.schedules.as_ref() else {
        return Ok(());
    };

    for s in schedules {
        if !s.enabled {
            continue;
        }
        let run_now = should_run_schedule(&s.id, &s.time, &s.schedule_type);
        let retry_key = format!("board.schedule.retry.{}", s.id);
        let retries = Memory::get(&retry_key, MemoryScope::Global, None)
            .ok()
            .flatten()
            .and_then(|v| v.value.parse::<u32>().ok())
            .unwrap_or(0);
        let retry_due = retries > 0 && retries < 3;

        if !run_now && !retry_due {
            continue;
        }

        let result = match s.schedule_type.as_str() {
            "daily" => {
                let team_id = s
                    .team_id
                    .as_deref()
                    .or(settings.board.team_id.as_deref())
                    .unwrap_or("board");
                let topic = format!("Daily board update for {}", chrono::Local::now().format("%Y-%m-%d"));
                crate::board::run_board_discussion(settings, team_id, &topic, Some(120))
                    .await
                    .map(|_| ())
            }
            "digest" => {
                if let Some(agent) = s.agent_id.as_deref() {
                    TaskSpawner::run_heartbeat(agent, settings).await.map(|_| ())
                } else {
                    Err(Error::Other("Digest schedule missing agent_id".to_string()))
                }
            }
            _ => Err(Error::Other(format!("Unknown board schedule type: {}", s.schedule_type))),
        };

        match result {
            Ok(_) => {
                mark_schedule_run(&s.id);
                let _ = Memory::set(
                    &format!("board.schedule.retry.{}", s.id),
                    "0",
                    MemoryScope::Global,
                    None,
                );
                log_schedule_attempt(&s.id, true, "ok");
            }
            Err(e) => {
                let next = retries.saturating_add(1);
                let _ = Memory::set(&retry_key, &next.to_string(), MemoryScope::Global, None);
                log_schedule_attempt(&s.id, false, &e.to_string());
            }
        }
    }
    Ok(())
}

async fn run_delegation_followups(settings: &Settings) -> Result<(), Error> {
    let team_id = settings.board.team_id.as_deref().unwrap_or("board");
    let overdue = crate::board::run_delegation_followup(team_id, 24)?;
    if overdue.is_empty() {
        return Ok(());
    }
    let leader = settings
        .teams
        .get(team_id)
        .and_then(|t| t.leader_agent.as_deref())
        .unwrap_or("assistant");
    let prompt = format!(
        "These delegation items are overdue. Send concise follow-up actions and update status:\n{}",
        overdue.join("\n")
    );
    let out = TaskSpawner::invoke_agent_cli(leader, &prompt, settings)
        .await
        .unwrap_or_else(|e| format!("Follow-up failed: {}", e));
    let key = format!("board.followup.{}", ulid::Ulid::new());
    let rec = serde_json::json!({
        "team_id": team_id,
        "overdue_count": overdue.len(),
        "items": overdue,
        "leader": leader,
        "result": out.chars().take(1200).collect::<String>(),
        "timestamp": chrono::Utc::now().to_rfc3339()
    });
    Memory::set(&key, &rec.to_string(), MemoryScope::Team, Some(team_id))?;
    Ok(())
}

async fn run_brain_proactive_checks(settings: &Settings) -> Result<(), Error> {
    let Some(path) = resolve_brain_path(settings) else {
        return Ok(());
    };
    if !path.exists() {
        return Ok(());
    }

    let content = std::fs::read_to_string(&path).unwrap_or_default();
    let today = chrono::Local::now().format("%Y-%m-%d").to_string();
    let check_marker = format!("[auto-check {}]", today);
    if content.contains(&check_marker) {
        return Ok(());
    }

    let issues = detect_brain_issues(&content);
    let mut updated = content;
    let summary = if issues.is_empty() {
        format!("{} no stale/broken/overdue items detected", check_marker)
    } else {
        format!("{} {}", check_marker, issues.join(" | "))
    };
    updated.push_str(&format!(
        "\n- {}{}\n",
        summary,
        if issues.is_empty() { "" } else { " -> auto-followup created" }
    ));

    std::fs::write(&path, updated).map_err(|e| Error::Other(format!("write BRAIN.md: {}", e)))?;

    let session_id = format!("brain-{}", today);
    let _ = crate::memory::sqlite::record_event(&session_id, "assistant", "brain_check", &summary);
    let _ = crate::memory::sqlite::record_decision(
        &session_id,
        "assistant",
        "proactive_maintenance",
        "assistant",
        "high",
        Some(&today),
        "heartbeat proactive scan of BRAIN.md",
    );
    let _ = crate::memory::sqlite::record_outcome(
        &session_id,
        "assistant",
        "success",
        None,
        &summary,
    );
    let _ = Memory::set("brain.last_check", &today, MemoryScope::Global, None);
    let _ = Memory::set("brain.last_summary", &summary, MemoryScope::Global, None);
    Ok(())
}

fn resolve_brain_path(settings: &Settings) -> Option<std::path::PathBuf> {
    if let Ok(raw) = std::env::var("TINYVEGETA_BRAIN_PATH") {
        let trimmed = raw.trim();
        if !trimmed.is_empty() {
            return Some(std::path::PathBuf::from(trimmed));
        }
    }

    if let Some(agent) = settings.agents.get("assistant") {
        if let Some(wd) = agent.working_directory.as_ref() {
            let p = wd.join("BRAIN.md");
            if p.exists() {
                return Some(p);
            }
        }
    }

    directories::UserDirs::new().map(|u| u.home_dir().join("ai").join("tinyvegeta").join("BRAIN.md"))
}

fn detect_brain_issues(content: &str) -> Vec<String> {
    let mut issues = Vec::new();
    let lower = content.to_lowercase();
    if lower.contains("[project]") {
        issues.push("placeholder project entry still present".to_string());
    }
    if lower.contains("[what needs doing today]") {
        issues.push("immediate actions placeholder unresolved".to_string());
    }
    if lower.contains("[what's running, what's done]") {
        issues.push("background task placeholder unresolved".to_string());
    }

    let re = regex::Regex::new(r"(?i)due:\s*(20\d{2}-\d{2}-\d{2})").ok();
    if let Some(re) = re {
        let today = chrono::Local::now().date_naive();
        for cap in re.captures_iter(content) {
            if let Some(m) = cap.get(1) {
                if let Ok(due) = chrono::NaiveDate::parse_from_str(m.as_str(), "%Y-%m-%d") {
                    if due < today {
                        issues.push(format!("overdue item detected (due:{})", due));
                    }
                }
            }
        }
    }

    issues
}

async fn run_system_maintenance(settings: &Settings) -> Result<(), Error> {
    let mut actions: Vec<String> = Vec::new();
    let mut warnings: Vec<String> = Vec::new();
    let mut score: i32 = 100;

    run_doctor_fix_if_due(&mut actions, &mut warnings, &mut score)?;
    check_queue_pressure(&mut actions, &mut warnings, &mut score)?;
    check_tmux_state(&mut actions, &mut warnings, &mut score)?;
    check_agent_freshness_and_failures(settings, &mut actions, &mut warnings, &mut score)?;
    check_provider_health(settings, &mut actions, &mut warnings, &mut score).await?;
    check_disk_space(&mut actions, &mut warnings, &mut score)?;
    check_sqlite_health(&mut actions, &mut warnings, &mut score)?;
    check_sovereign_runtime(settings, &mut actions, &mut warnings, &mut score)?;
    cleanup_stale_pairing_requests(&mut actions, &mut warnings)?;
    suggest_memory_compaction(&mut actions, &mut warnings)?;

    if score < 0 {
        score = 0;
    }
    let ts = chrono::Utc::now().to_rfc3339();
    let action_line = if actions.is_empty() {
        "none".to_string()
    } else {
        actions.join(" | ")
    };
    let warn_line = if warnings.is_empty() {
        "none".to_string()
    } else {
        warnings.join(" | ")
    };
    let summary = format!(
        "health_score={} actions={} warnings={}",
        score, action_line, warn_line
    );

    Memory::set("heartbeat.last_timestamp", &ts, MemoryScope::Global, None)?;
    Memory::set("heartbeat.health_score", &score.to_string(), MemoryScope::Global, None)?;
    Memory::set("heartbeat.last_actions", &action_line, MemoryScope::Global, None)?;
    Memory::set("heartbeat.last_warnings", &warn_line, MemoryScope::Global, None)?;

    let _ = crate::memory::sqlite::record_event("heartbeat", "assistant", "heartbeat_cycle", &summary);
    let _ = crate::memory::sqlite::record_outcome("heartbeat", "assistant", "success", None, &summary);
    append_heartbeat_audit(&ts, score, &actions, &warnings)?;
    Ok(())
}

fn run_doctor_fix_if_due(actions: &mut Vec<String>, warnings: &mut Vec<String>, score: &mut i32) -> Result<(), Error> {
    let now = chrono::Utc::now().timestamp_millis();
    let key = "heartbeat.doctor.last_run_ms";
    let last = Memory::get(key, MemoryScope::Global, None)
        .ok()
        .flatten()
        .and_then(|v| v.value.parse::<i64>().ok())
        .unwrap_or(0);
    if now - last < 3_600_000 {
        return Ok(());
    }

    let exe = std::env::current_exe().map_err(|e| Error::Other(format!("current_exe: {}", e)))?;
    let output = std::process::Command::new(exe)
        .arg("doctor")
        .arg("--fix")
        .output()
        .map_err(|e| Error::Other(format!("doctor --fix failed: {}", e)))?;
    if output.status.success() {
        actions.push("doctor --fix".to_string());
    } else {
        warnings.push("doctor --fix reported issues".to_string());
        *score -= 8;
    }
    Memory::set(key, &now.to_string(), MemoryScope::Global, None)?;
    Ok(())
}

fn check_queue_pressure(actions: &mut Vec<String>, warnings: &mut Vec<String>, score: &mut i32) -> Result<(), Error> {
    let stats = crate::core::Queue::stats()?;
    Memory::set("heartbeat.queue.depth", &stats.total.to_string(), MemoryScope::Global, None)?;
    if stats.total > 50 {
        warnings.push(format!("queue pressure high ({})", stats.total));
        *score -= 12;
    } else {
        actions.push(format!("queue ok ({})", stats.total));
    }
    Ok(())
}

fn check_tmux_state(actions: &mut Vec<String>, warnings: &mut Vec<String>, score: &mut i32) -> Result<(), Error> {
    if !crate::tmux::session_exists()? {
        let exe = std::env::current_exe()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|_| "tinyvegeta".to_string());
        if crate::tmux::start_daemon(&exe).is_ok() {
            actions.push("tmux recovered via restart".to_string());
        } else {
            warnings.push("tmux session missing and restart failed".to_string());
            *score -= 15;
        }
    } else {
        actions.push("tmux alive".to_string());
    }
    Ok(())
}

fn check_agent_freshness_and_failures(
    settings: &Settings,
    actions: &mut Vec<String>,
    warnings: &mut Vec<String>,
    score: &mut i32,
) -> Result<(), Error> {
    let now = chrono::Utc::now().timestamp_millis();
    for agent_id in settings.agents.keys() {
        let success_key = format!("agent.health.{}.last_success", agent_id);
        let last_success = Memory::get(&success_key, MemoryScope::Global, None)
            .ok()
            .flatten()
            .and_then(|v| v.value.parse::<i64>().ok())
            .unwrap_or(0);
        if now - last_success > 30 * 60 * 1000 {
            warnings.push(format!("@{} stale (>30m without success)", agent_id));
            *score -= 4;
        }

        let fail_count = crate::memory::sqlite::failed_outcomes_last_hour(agent_id).unwrap_or(0);
        if fail_count > 3 {
            let reset_key = format!("agent.health.{}.auto_reset", agent_id);
            let _ = Memory::set(&reset_key, &now.to_string(), MemoryScope::Global, None);
            warnings.push(format!("@{} >3 failures/hour ({}), reset flagged", agent_id, fail_count));
            *score -= 8;
        }
    }
    actions.push("agent freshness/failure scan".to_string());
    Ok(())
}

async fn check_provider_health(
    settings: &Settings,
    actions: &mut Vec<String>,
    warnings: &mut Vec<String>,
    score: &mut i32,
) -> Result<(), Error> {
    let mut checked: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    for agent in settings.agents.values() {
        let provider_name = agent
            .provider
            .clone()
            .unwrap_or_else(|| settings.models.provider.clone());
        checked.insert(provider_name);
    }
    for provider_name in checked {
        let provider = crate::providers::create_provider(&provider_name, settings);
        let ok = provider.is_available().await;
        if ok {
            actions.push(format!("provider {} ok", provider_name));
        } else {
            warnings.push(format!("provider {} unavailable", provider_name));
            *score -= 8;
        }
    }
    Ok(())
}

fn check_disk_space(actions: &mut Vec<String>, warnings: &mut Vec<String>, score: &mut i32) -> Result<(), Error> {
    let home = get_home_dir()?;
    let output = std::process::Command::new("df")
        .args(["-k", home.to_string_lossy().as_ref()])
        .output()
        .map_err(|e| Error::Other(format!("df failed: {}", e)))?;
    let text = String::from_utf8_lossy(&output.stdout);
    let mut low = false;
    for line in text.lines().skip(1) {
        let cols = line.split_whitespace().collect::<Vec<_>>();
        if cols.len() >= 4 {
            if let Ok(avail_kb) = cols[3].parse::<u64>() {
                if avail_kb < 2_000_000 {
                    low = true;
                }
            }
        }
    }
    if low {
        warnings.push("low disk space (<2GB free)".to_string());
        *score -= 10;
    } else {
        actions.push("disk space ok".to_string());
    }
    Ok(())
}

fn check_sqlite_health(actions: &mut Vec<String>, warnings: &mut Vec<String>, score: &mut i32) -> Result<(), Error> {
    let path = crate::memory::sqlite::sqlite_db_path()?;
    if !path.exists() {
        actions.push("sqlite db not created yet".to_string());
        return Ok(());
    }
    let meta = std::fs::metadata(&path)?;
    let size_mb = meta.len() / (1024 * 1024);
    Memory::set("heartbeat.sqlite.size_mb", &size_mb.to_string(), MemoryScope::Global, None)?;
    if size_mb > 100 {
        match crate::memory::sqlite::vacuum() {
            Ok(_) => actions.push(format!("sqlite vacuum ran ({}MB)", size_mb)),
            Err(e) => {
                warnings.push(format!("sqlite vacuum failed: {}", e));
                *score -= 6;
            }
        }
    } else {
        actions.push(format!("sqlite size {}MB", size_mb));
    }
    Ok(())
}

fn check_sovereign_runtime(
    settings: &Settings,
    actions: &mut Vec<String>,
    warnings: &mut Vec<String>,
    score: &mut i32,
) -> Result<(), Error> {
    if !settings.sovereign.enabled {
        actions.push("sovereign disabled".to_string());
        return Ok(());
    }
    let pid = Memory::get("sovereign.process.pid", MemoryScope::Global, None)
        .ok()
        .flatten()
        .and_then(|v| v.value.parse::<u32>().ok());
    let Some(pid) = pid else {
        warnings.push("sovereign enabled but no pid tracked".to_string());
        *score -= 8;
        return Ok(());
    };
    let alive = std::process::Command::new("kill")
        .arg("-0")
        .arg(pid.to_string())
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);
    if alive {
        actions.push(format!("sovereign alive pid={}", pid));
    } else {
        warnings.push(format!("sovereign pid {} not alive", pid));
        *score -= 8;
    }
    Ok(())
}

fn cleanup_stale_pairing_requests(actions: &mut Vec<String>, warnings: &mut Vec<String>) -> Result<(), Error> {
    let mut settings = crate::config::load_settings()?;
    let now = chrono::Utc::now().timestamp_millis();
    let cutoff = now - 24 * 60 * 60 * 1000;
    let mut removed = 0usize;
    if let Some(pending) = settings.pairing.pending_senders.as_mut() {
        let before = pending.len();
        pending.retain(|p| p.requested_at >= cutoff);
        removed = before.saturating_sub(pending.len());
    }
    if removed > 0 {
        let path = crate::config::get_settings_path()?;
        std::fs::write(path, serde_json::to_string_pretty(&settings)?)?;
        actions.push(format!("auto-rejected {} stale pairing requests", removed));
    } else {
        warnings.push("no stale pairing requests".to_string());
    }
    Ok(())
}

fn suggest_memory_compaction(actions: &mut Vec<String>, warnings: &mut Vec<String>) -> Result<(), Error> {
    let key = "heartbeat.memory.compact.last_day";
    let today = chrono::Local::now().format("%Y-%m-%d").to_string();
    let last = Memory::get(key, MemoryScope::Global, None)
        .ok()
        .flatten()
        .map(|v| v.value)
        .unwrap_or_default();
    if last == today {
        return Ok(());
    }
    match crate::memory::Memory::compact(MemoryScope::Global, None) {
        Ok(report) => {
            // CompactReport fields: merged, expired_removed, promoted, pruned.
            actions.push(format!(
                "memory compact global merged={} removed={} promoted={} pruned={}",
                report.merged, report.expired_removed, report.promoted, report.pruned
            ));
            Memory::set(key, &today, MemoryScope::Global, None)?;
        }
        Err(e) => warnings.push(format!("memory compact failed: {}", e)),
    }
    Ok(())
}

fn append_heartbeat_audit(
    ts: &str,
    health_score: i32,
    actions: &[String],
    warnings: &[String],
) -> Result<(), Error> {
    let path = get_home_dir()?.join("audit").join("heartbeat.jsonl");
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let rec = serde_json::json!({
        "timestamp": ts,
        "health_score": health_score,
        "actions": actions,
        "warnings": warnings,
    });
    let mut f = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;
    writeln!(f, "{}", rec)?;
    Ok(())
}
