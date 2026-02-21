//! Configuration loading for TinyVegeta.
#![allow(dead_code)]

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

use crate::error::Error;
pub type Result<T> = std::result::Result<T, Error>;

/// Get the TinyVegeta home directory (~/.tinyvegeta).
pub fn get_home_dir() -> Result<PathBuf> {
    let home = directories::UserDirs::new()
        .ok_or_else(|| Error::Config("Could not determine home directory".to_string()))?;

    Ok(home.home_dir().join(".tinyvegeta"))
}

/// Get the settings file path.
pub fn get_settings_path() -> Result<PathBuf> {
    Ok(get_home_dir()?.join("settings.json"))
}

/// Load settings from ~/.tinyvegeta/settings.json
pub fn load_settings() -> Result<Settings> {
    let path = get_settings_path()?;

    if !path.exists() {
        return Err(Error::Config(format!(
            "Settings file not found at {}. Run 'tinyvegeta setup' first.",
            path.display()
        )));
    }

    let content = std::fs::read_to_string(&path)?;
    let mut settings: Settings = serde_json::from_str(&content)?;

    // Self-heal minimal defaults for existing installs that predate
    // default team/board provisioning.
    if ensure_default_team_and_board(&mut settings) {
        let updated = serde_json::to_string_pretty(&settings)?;
        std::fs::write(&path, updated)?;
        tracing::info!("Applied default team/board provisioning to {}", path.display());
    }

    validate_settings(&settings)?;

    tracing::debug!("Loaded settings from {}", path.display());
    Ok(settings)
}

fn ensure_default_team_and_board(settings: &mut Settings) -> bool {
    let mut changed = false;

    let primary_agent = if settings.agents.contains_key("assistant") {
        Some("assistant".to_string())
    } else {
        settings.agents.keys().next().cloned()
    };

    if settings.teams.is_empty() {
        if let Some(agent_id) = primary_agent.clone() {
            settings.teams.insert(
                "board".to_string(),
                TeamConfig {
                    name: "Board".to_string(),
                    agents: vec![agent_id.clone()],
                    leader_agent: Some(agent_id),
                },
            );
            changed = true;
        }
    }

    if settings.board.team_id.is_none() {
        if settings.teams.contains_key("board") {
            settings.board.team_id = Some("board".to_string());
            changed = true;
        } else if let Some((team_id, _)) = settings.teams.iter().next() {
            settings.board.team_id = Some(team_id.clone());
            changed = true;
        }
    }

    if let Some(board_id) = settings.board.team_id.clone() {
        if !settings.teams.contains_key(&board_id) {
            if let Some(agent_id) = primary_agent {
                settings.teams.insert(
                    board_id,
                    TeamConfig {
                        name: "Board".to_string(),
                        agents: vec![agent_id.clone()],
                        leader_agent: Some(agent_id),
                    },
                );
                changed = true;
            }
        }
    }

    if settings.board.autonomous.is_none() {
        settings.board.autonomous = Some(false);
        changed = true;
    }

    if settings.routing.default_agent.is_none() {
        if settings.agents.contains_key("assistant") {
            settings.routing.default_agent = Some("assistant".to_string());
            changed = true;
        } else if let Some(first) = settings.agents.keys().next().cloned() {
            settings.routing.default_agent = Some(first);
            changed = true;
        }
    }

    // Sovereign defaults for unrestricted local operation.
    if !settings.sovereign.allow_tool_install {
        settings.sovereign.allow_tool_install = true;
        changed = true;
    }
    if !settings.sovereign.allow_self_modify {
        settings.sovereign.allow_self_modify = true;
        changed = true;
    }

    changed
}

fn validate_settings(settings: &Settings) -> Result<()> {
    if let Some(default_agent) = settings.routing.default_agent.as_deref() {
        if !settings.agents.contains_key(default_agent) {
            return Err(Error::Config(format!(
                "routing.default_agent '{}' not found in settings.agents",
                default_agent
            )));
        }
    }
    Ok(())
}

/// Load settings or return default if not found.
pub fn load_settings_or_default() -> Settings {
    load_settings().unwrap_or_else(|e| {
        tracing::warn!("Failed to load settings: {}, using defaults", e);
        Settings::default()
    })
}

/// Workspace configuration.
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Workspace {
    pub path: Option<PathBuf>,
    pub name: Option<String>,
}

/// Channel configuration.
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct ChannelConfig {
    pub bot_token: Option<String>,
}

/// Channels configuration.
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Channels {
    pub enabled: Vec<String>,
    #[serde(default)]
    pub telegram: ChannelConfig,
}

/// Agent configuration.
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct AgentConfig {
    pub name: Option<String>,
    pub provider: Option<String>,
    pub model: Option<String>,
    pub working_directory: Option<PathBuf>,
    #[serde(default)]
    pub is_sovereign: bool,
}

/// Team configuration.
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct TeamConfig {
    pub name: String,
    pub agents: Vec<String>,
    pub leader_agent: Option<String>,
}

/// Provider model configuration.
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct ProviderModel {
    pub model: Option<String>,
    pub api_key: Option<String>,
    pub base_url: Option<String>,
}

/// Models configuration.
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Models {
    #[serde(default)]
    pub provider: String,
    #[serde(default)]
    pub openai: ProviderModel,
    #[serde(default)]
    pub anthropic: ProviderModel,
    #[serde(default)]
    pub grok: ProviderModel,
    #[serde(default)]
    pub ollama: ProviderModel,
}

/// Pairing configuration.
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Pairing {
    #[serde(default = "default_pairing_mode")]
    pub mode: String,
    pub approved_senders: Option<Vec<ApprovedSender>>,
    pub pending_senders: Option<Vec<PendingSender>>,
    pub soul_owner_sender_id: Option<String>,
}

fn default_pairing_mode() -> String {
    "approval".to_string()
}

/// Approved sender for pairing.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ApprovedSender {
    pub sender_id: String,
    pub sender_name: String,
    pub paired_at: i64,
}

/// Pending sender for pairing.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PendingSender {
    pub sender_id: String,
    pub sender_name: String,
    pub code: String,
    pub requested_at: i64,
}

/// Monitoring configuration.
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Monitoring {
    #[serde(default = "default_heartbeat_interval")]
    pub heartbeat_interval: u64,
}

fn default_heartbeat_interval() -> u64 {
    3600
}

/// Board configuration.
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Board {
    pub team_id: Option<String>,
    pub autonomous: Option<bool>,
    pub schedules: Option<Vec<BoardSchedule>>,
}

/// Routing configuration.
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Routing {
    pub default_agent: Option<String>,
}

/// Sovereign runtime configuration.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Sovereign {
    #[serde(default = "default_sovereign_enabled")]
    pub enabled: bool,
    pub constitution_path: Option<PathBuf>,
    #[serde(default)]
    pub protected_files: Vec<String>,
    #[serde(default = "default_sovereign_loop_sleep_seconds")]
    pub loop_sleep_seconds: u64,
    #[serde(default = "default_sovereign_max_actions_per_cycle")]
    pub max_actions_per_cycle: u32,
    #[serde(default = "default_sovereign_max_self_modifications_per_hour")]
    pub max_self_modifications_per_hour: u32,
    #[serde(default = "default_sovereign_allow_tool_install")]
    pub allow_tool_install: bool,
    #[serde(default = "default_sovereign_allow_self_modify")]
    pub allow_self_modify: bool,
}

fn default_sovereign_enabled() -> bool {
    false
}

fn default_sovereign_loop_sleep_seconds() -> u64 {
    20
}

fn default_sovereign_max_actions_per_cycle() -> u32 {
    3
}

fn default_sovereign_max_self_modifications_per_hour() -> u32 {
    6
}

fn default_sovereign_allow_tool_install() -> bool {
    true
}

fn default_sovereign_allow_self_modify() -> bool {
    true
}

impl Default for Sovereign {
    fn default() -> Self {
        Self {
            enabled: default_sovereign_enabled(),
            constitution_path: None,
            protected_files: Vec::new(),
            loop_sleep_seconds: default_sovereign_loop_sleep_seconds(),
            max_actions_per_cycle: default_sovereign_max_actions_per_cycle(),
            max_self_modifications_per_hour: default_sovereign_max_self_modifications_per_hour(),
            allow_tool_install: default_sovereign_allow_tool_install(),
            allow_self_modify: default_sovereign_allow_self_modify(),
        }
    }
}

/// Board schedule.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct BoardSchedule {
    pub id: String,
    pub schedule_type: String,
    pub time: String,
    pub team_id: Option<String>,
    pub agent_id: Option<String>,
    pub sender_id: Option<String>,
    pub enabled: bool,
}

/// TinyVegeta settings.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Settings {
    #[serde(default)]
    pub workspace: Workspace,

    #[serde(default)]
    pub channels: Channels,

    #[serde(default)]
    pub agents: HashMap<String, AgentConfig>,

    #[serde(default)]
    pub teams: HashMap<String, TeamConfig>,

    #[serde(default)]
    pub models: Models,

    #[serde(default)]
    pub pairing: Pairing,

    #[serde(default)]
    pub monitoring: Monitoring,

    #[serde(default)]
    pub board: Board,

    #[serde(default)]
    pub routing: Routing,

    #[serde(default)]
    pub sovereign: Sovereign,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            workspace: Workspace::default(),
            channels: Channels::default(),
            agents: HashMap::new(),
            teams: HashMap::default(),
            models: Models::default(),
            pairing: Pairing::default(),
            monitoring: Monitoring::default(),
            board: Board::default(),
            routing: Routing::default(),
            sovereign: Sovereign::default(),
        }
    }
}
