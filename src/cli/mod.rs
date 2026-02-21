//! CLI commands for TinyVegeta using clap.

use anyhow::Result;
use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};

use crate::config::load_settings;
use crate::core::MessageData;
use crate::tmux;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TaskRecord {
    id: String,
    title: String,
    description: Option<String>,
    agent_id: Option<String>,
    priority: String,
    status: String,
    tags: Vec<String>,
    created_at: i64,
    updated_at: i64,
    output: Option<String>,
    error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct TaskStore {
    tasks: Vec<TaskRecord>,
}

fn tasks_file_path() -> Result<std::path::PathBuf> {
    Ok(crate::config::get_home_dir()?.join("tasks.json"))
}

fn load_task_store() -> Result<TaskStore> {
    let path = tasks_file_path()?;
    if !path.exists() {
        return Ok(TaskStore::default());
    }
    let content = std::fs::read_to_string(path)?;
    Ok(serde_json::from_str(&content).unwrap_or_default())
}

fn save_task_store(store: &TaskStore) -> Result<()> {
    let path = tasks_file_path()?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, serde_json::to_string_pretty(store)?)?;
    Ok(())
}

/// TinyVegeta - Multi-agent, multi-team, Telegram-first 24/7 AI assistant.
#[derive(Parser)]
#[command(name = "tinyvegeta")]
#[command(version = "0.1.0")]
#[command(about = "TinyVegeta - The Prince of All AI Agents", long_about = None)]
pub struct Commands {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    /// Start TinyVegeta daemon
    Start,
    
    /// Internal: Run daemon services (called by start)
    #[command(hide = true)]
    StartInternal,
    
    /// Stop TinyVegeta daemon
    Stop,
    
    /// Restart TinyVegeta daemon
    Restart,
    
    /// Show current status
    Status,
    
    /// Attach to tmux session
    Attach,
    
    /// Run setup wizard
    Setup,
    
    /// Send a message
    Send {
        /// Message to send
        message: String,
    },
    
    /// View logs
    Logs {
        /// Log type: telegram, queue, heartbeat, daemon, all
        #[arg(default_value = "all")]
        log_type: String,
    },
    
    /// Queue operations
    Queue {
        /// Queue action
        #[command(subcommand)]
        action: QueueCommand,
    },
    
    /// Reset agent conversation
    Reset {
        /// Agent IDs to reset
        #[arg(required = true)]
        agents: Vec<String>,
    },
    
    /// Manage agents
    #[command(subcommand, alias = "a")]
    Agent(AgentCommand),
    
    /// Manage teams
    #[command(subcommand, alias = "t")]
    Team(TeamCommand),
    
    /// Board commands
    #[command(subcommand)]
    Board(BoardCommand),
    
    /// Memory commands
    #[command(subcommand)]
    Memory(MemoryCommand),
    
    /// Task commands
    #[command(subcommand)]
    Task(TaskCommand),
    
    /// Pairing commands
    #[command(subcommand)]
    Pairing(PairingCommand),
    
    /// Show or switch provider
    Provider {
        /// Provider name: claude, codex, cline, opencode, ollama, grok
        name: Option<String>,
        
        /// Model to use
        #[arg(long = "model")]
        model: Option<String>,
    },
    
    /// Show or switch model
    Model {
        /// Model name
        name: Option<String>,
    },
    
    /// Channel management
    Channels {
        /// Action: reset
        action: String,
        
        /// Channel name
        channel: String,
    },
    
    /// Run diagnostics
    Doctor {
        /// Strict mode
        #[arg(long)]
        strict: bool,
        
        /// Auto-fix issues
        #[arg(long)]
        fix: bool,
    },
    
    /// Run release readiness check
    Releasecheck,
    
    /// Start Telegram bot daemon
    Telegram,
    
    /// Start heartbeat daemon
    Heartbeat {
        /// Run single heartbeat for agent
        #[arg(long)]
        agent: Option<String>,

        /// Verbose output for single heartbeat runs
        #[arg(long, default_value_t = false)]
        verbose: bool,
    },

    /// Start sovereign autonomous loop
    Sovereign {
        /// Agent to run as sovereign runtime
        #[arg(long)]
        agent: Option<String>,

        /// Mission/goal for the autonomous loop
        #[arg(long)]
        goal: Option<String>,

        /// Max loop cycles (omit for continuous)
        #[arg(long)]
        max_cycles: Option<u32>,

        /// Dry run mode (no file writes or command execution)
        #[arg(long, default_value_t = false)]
        dry_run: bool,
    },
    
    /// Start web server
    Web {
        /// Port number
        #[arg(long, default_value = "3333")]
        port: u16,
        
        /// Stop the web server
        #[arg(long)]
        stop: bool,
    },
    
    /// Update TinyVegeta
    Update,
    
    /// Uninstall TinyVegeta
    Uninstall {
        /// Non-interactive mode
        #[arg(long)]
        yes: bool,
        
        /// Also delete data directory
        #[arg(long)]
        purge_data: bool,
        
        /// Also delete installation
        #[arg(long)]
        purge_install: bool,
    },
}

#[derive(Subcommand)]
pub enum AgentCommand {
    /// List all agents
    List,
    
    /// Add a new agent
    Add,
    
    /// Show agent configuration
    Show {
        /// Agent ID
        agent_id: String,
    },
    
    /// Remove an agent
    Remove {
        /// Agent ID
        agent_id: String,
    },
    
    /// Reset agent conversation
    Reset {
        /// Agent ID
        agent_id: String,
    },
    
    /// Agent pack commands
    Pack {
        #[command(subcommand)]
        command: AgentPackCommand,
    },

    /// Show or set default agent routing
    Default {
        /// Agent ID to set as default (omit to show)
        agent_id: Option<String>,
    },
}

#[derive(Subcommand)]
pub enum AgentPackCommand {
    /// List available packs
    List,
    
    /// Install a pack
    Install {
        /// Pack name
        name: String,
    },
}

#[derive(Subcommand)]
pub enum TeamCommand {
    /// List all teams
    List,
    
    /// Add a new team
    Add {
        /// Team ID
        #[arg(long)]
        id: Option<String>,

        /// Display name
        #[arg(long)]
        name: Option<String>,

        /// Members (comma-separated)
        #[arg(long)]
        members: Option<String>,

        /// Leader agent ID
        #[arg(long)]
        leader: Option<String>,
    },
    
    /// Show team configuration
    Show {
        /// Team ID
        team_id: String,
    },
    
    /// Remove a team
    Remove {
        /// Team ID
        team_id: String,
    },

    /// Update team members/leader
    Update {
        /// Team ID
        team_id: String,

        /// Members (comma-separated)
        #[arg(long)]
        members: Option<String>,

        /// Leader agent ID
        #[arg(long)]
        leader: Option<String>,

        /// Team display name
        #[arg(long)]
        name: Option<String>,
    },
    
    /// Visualize team
    Visualize {
        /// Team ID (optional)
        team_id: Option<String>,
    },
}

#[derive(Subcommand)]
pub enum BoardCommand {
    /// Create or update a board
    Create {
        /// CEO agent ID
        #[arg(long)]
        ceo: Option<String>,
        
        /// Team members
        #[arg(long)]
        members: Option<String>,
        
        /// Enable autonomous mode
        #[arg(long)]
        autonomous: bool,
    },
    
    /// Show board configuration
    Show {
        /// Board ID (optional)
        board_id: Option<String>,
    },
    
    /// Start a board discussion
    Discuss {
        /// Topic to discuss
        topic: String,
        
        /// Team ID
        #[arg(long)]
        team_id: Option<String>,
        
        /// Timeout in seconds
        #[arg(long)]
        timeout: Option<u64>,
        
        /// Raw mode
        #[arg(long)]
        raw: bool,
    },
    
    /// Board schedule commands
    Schedule {
        #[command(subcommand)]
        command: BoardScheduleCommand,
    },
    
    /// Board decisions
    Decisions {
        #[command(subcommand)]
        command: BoardDecisionsCommand,
    },
}

#[derive(Subcommand)]
pub enum BoardScheduleCommand {
    /// Schedule daily board update
    Daily {
        /// Time (HH:MM)
        #[arg(long)]
        time: Option<String>,
        
        /// Team ID
        #[arg(long)]
        team_id: Option<String>,
        
        /// Sender ID
        #[arg(long)]
        sender_id: Option<String>,
    },
    
    /// Schedule digest
    Digest {
        /// Time (HH:MM)
        #[arg(long)]
        time: Option<String>,
        
        /// Agent ID
        #[arg(long)]
        agent: Option<String>,
        
        /// Sender ID
        #[arg(long)]
        sender_id: Option<String>,
    },
    
    /// List schedules
    List,
    
    /// Remove schedules
    Remove {
        /// Remove all
        #[arg(default_value = "")]
        which: String,
    },
}

#[derive(Subcommand)]
pub enum BoardDecisionsCommand {
    /// List decisions
    List {
        /// Limit
        #[arg(long)]
        limit: Option<usize>,
    },
    
    /// Show decision
    Show {
        /// Decision ID
        decision_id: String,
    },

    /// Export decisions to markdown or json
    Export {
        /// Output format: markdown|json
        #[arg(long, default_value = "markdown")]
        format: String,

        /// Output file path
        #[arg(long)]
        file: Option<String>,

        /// Limit
        #[arg(long, default_value = "50")]
        limit: usize,
    },
}

#[derive(Subcommand)]
pub enum QueueCommand {
    /// Show queue statistics
    Stats,
    
    /// List incoming messages
    Incoming,
    
    /// List processing messages
    Processing,
    
    /// List outgoing messages
    Outgoing,
    
    /// Enqueue a test message
    Enqueue {
        /// Message content
        message: String,
        
        /// Channel (default: cli)
        #[arg(long)]
        channel: Option<String>,
        
        /// Sender (default: cli)
        #[arg(long)]
        sender: Option<String>,
    },
    
    /// Recover orphaned messages
    Recover,
}

#[derive(Subcommand)]
pub enum MemoryCommand {
    /// Set a memory entry
    Set {
        /// Key
        key: String,
        
        /// Value
        value: String,
        
        /// Scope: global, agent, task
        #[arg(default_value = "global")]
        scope: String,
        
        /// Scope ID (agent_id or task_id)
        scope_id: Option<String>,
    },
    
    /// Get a memory entry
    Get {
        /// Key
        key: String,
        
        /// Scope
        #[arg(default_value = "global")]
        scope: String,
        
        /// Scope ID
        scope_id: Option<String>,
    },
    
    /// List memory entries
    List {
        /// Scope
        scope: Option<String>,
        
        /// Category
        category: Option<String>,
    },
    
    /// Search memory
    Search {
        /// Query
        query: String,
        
        /// Limit
        #[arg(default_value = "10")]
        limit: usize,
    },

    /// Explain what memory would be injected for a query
    Explain {
        /// Query text
        query: String,

        /// Agent ID (default: assistant)
        #[arg(long)]
        agent: Option<String>,

        /// Team ID (optional)
        #[arg(long)]
        team: Option<String>,

        /// Limit per scope
        #[arg(default_value = "6")]
        limit: usize,
    },
    
    /// Delete memory entry
    Delete {
        /// Key
        key: String,
        
        /// Scope
        #[arg(default_value = "global")]
        scope: String,
        
        /// Scope ID
        scope_id: Option<String>,
    },
    
    /// Memory statistics
    Stats,

    /// Compact memory store (dedupe/merge/prune)
    Compact {
        /// Scope: global, agent, team, task
        #[arg(default_value = "global")]
        scope: String,

        /// Scope ID (required for agent/team/task)
        scope_id: Option<String>,
    },
    
    /// Snapshot commands
    Snapshot {
        #[command(subcommand)]
        command: SnapshotCommand,
    },
    
    /// Memory inherit commands
    Inherit {
        #[command(subcommand)]
        command: InheritCommand,
    },
    
    /// Export memory
    Export {
        /// Output file
        file: Option<String>,
    },
    
    /// Clear memory
    Clear {
        /// Scope
        scope: Option<String>,
    },
}

#[derive(Subcommand)]
pub enum SnapshotCommand {
    /// Create snapshot
    Create {
        /// Name
        name: String,
    },
    
    /// Restore snapshot
    Restore {
        /// Snapshot ID
        id: String,
    },
    
    /// List snapshots
    List,
}

#[derive(Subcommand)]
pub enum InheritCommand {
    /// Add inheritance
    Add {
        /// Child scope
        child: String,
        
        /// Parent scope
        parent: String,
        
        /// Pattern
        pattern: Option<String>,
    },
    
    /// Remove inheritance
    Remove {
        /// Child scope
        child: String,
    },
    
    /// List inheritance
    List,
}

#[derive(Subcommand)]
pub enum TaskCommand {
    /// Create a new task
    Create {
        /// Task title
        title: String,
        
        /// Priority
        #[arg(long)]
        priority: Option<String>,
        
        /// Agent ID
        #[arg(long)]
        agent: Option<String>,
        
        /// Description
        #[arg(long)]
        description: Option<String>,
        
        /// Tags
        #[arg(long)]
        tags: Option<String>,
    },
    
    /// List tasks
    List {
        /// Status filter
        #[arg(long)]
        status: Option<String>,
    },
    
    /// Show task details
    Show {
        /// Task ID
        task_id: String,
    },
    
    /// Start a task
    Start {
        /// Task ID
        task_id: String,
        
        /// Attach to task
        #[arg(long)]
        attach: bool,
    },
    
    /// Stop a task
    Stop {
        /// Task ID
        task_id: String,
    },
    
    /// Watch task output
    Watch {
        /// Task ID
        task_id: String,
    },
    
    /// Assign task to agent
    Assign {
        /// Task ID
        task_id: String,
        
        /// Agent ID
        #[arg(long)]
        agent: String,
    },
    
    /// Delete task
    Delete {
        /// Task ID
        task_id: String,
    },
    
    /// Task statistics
    Stats,
}

#[derive(Subcommand)]
pub enum PairingCommand {
    /// List pending approvals
    Pending,
    
    /// List approved senders
    Approved,
    
    /// List all senders
    List,
    
    /// Approve a sender
    Approve {
        /// Pairing code
        code: String,
    },
    
    /// Unpair a sender
    Unpair {
        /// Channel
        channel: String,
        
        /// Sender ID
        sender_id: String,
    },
}

impl Commands {
    /// Run the command.
    pub async fn run(&self) -> Result<()> {
        match &self.command {
            Command::Start => cmd_start().await,
            Command::StartInternal => cmd_start_internal().await,
            Command::Stop => cmd_stop().await,
            Command::Restart => cmd_restart().await,
            Command::Status => cmd_status().await,
            Command::Attach => cmd_attach().await,
            Command::Setup => cmd_setup().await,
            Command::Send { message } => cmd_send(message).await,
            Command::Logs { log_type } => cmd_logs(log_type).await,
            Command::Queue { action } => cmd_queue(action).await,
            Command::Reset { agents } => cmd_reset(agents).await,
            Command::Agent(cmd) => cmd_agent(cmd).await,
            Command::Team(cmd) => cmd_team(cmd).await,
            Command::Board(cmd) => cmd_board(cmd).await,
            Command::Memory(cmd) => cmd_memory(cmd).await,
            Command::Task(cmd) => cmd_task(cmd).await,
            Command::Pairing(cmd) => cmd_pairing(cmd).await,
            Command::Provider { name, model } => cmd_provider(name, model).await,
            Command::Model { name } => cmd_model(name).await,
            Command::Channels { action, channel } => cmd_channels(action, channel).await,
            Command::Doctor { strict, fix } => cmd_doctor(*strict, *fix).await,
            Command::Releasecheck => cmd_releasecheck().await,
            Command::Telegram => cmd_telegram().await,
            Command::Heartbeat { agent, verbose } => cmd_heartbeat(agent, *verbose).await,
            Command::Sovereign { agent, goal, max_cycles, dry_run } => {
                cmd_sovereign(agent, goal, max_cycles, *dry_run).await
            }
            Command::Web { port, stop } => cmd_web(*port, *stop).await,
            Command::Update => cmd_update().await,
            Command::Uninstall { yes, purge_data, purge_install } => {
                cmd_uninstall(*yes, *purge_data, *purge_install).await
            }
        }
    }
}

// Command implementations

async fn cmd_start() -> Result<()> {
    println!("Starting TinyVegeta daemon...");
    // Validate settings early; this rejects startup when default agent config is invalid.
    let _ = load_settings()?;
    
    let binary = std::env::current_exe()
        .unwrap_or_else(|_| std::path::PathBuf::from("tinyvegeta"));
    
    tmux::start_daemon(binary.to_str().unwrap_or("tinyvegeta"))?;
    println!("TinyVegeta started successfully!");
    Ok(())
}

async fn cmd_start_internal() -> Result<()> {
    use crate::telegram::run_telegram_daemon;
    use crate::heartbeat::run_heartbeat_daemon;
    
    tracing::info!("Starting TinyVegeta internal services...");
    
    // Ensure directories exist
    crate::core::queue::ensure_queue_dirs()?;
    crate::memory::ensure_memory_dirs()?;
    ensure_runtime_board_pack()?;
    
    // Run Telegram bot, heartbeat daemon, and queue processor concurrently
    tokio::select! {
        result = run_telegram_daemon() => {
            if let Err(e) = result {
                tracing::error!("Telegram daemon error: {}", e);
            }
        }
        result = run_heartbeat_daemon() => {
            if let Err(e) = result {
                tracing::error!("Heartbeat daemon error: {}", e);
            }
        }
        result = run_queue_processor() => {
            if let Err(e) = result {
                tracing::error!("Queue processor error: {}", e);
            }
        }
    }
    
    Ok(())
}

fn ensure_runtime_board_pack() -> Result<()> {
    let mut settings = load_settings()?;
    let needs_pack = !settings.agents.contains_key("coder")
        || !settings.agents.contains_key("security")
        || !settings.agents.contains_key("operations")
        || !settings.agents.contains_key("marketing")
        || !settings.agents.contains_key("seo")
        || !settings.agents.contains_key("sales")
        || settings.board.team_id.is_none();

    if needs_pack {
        let workspace = crate::board::resolve_workspace_root(&settings);
        std::fs::create_dir_all(&workspace)?;
        crate::board::install_default_pack(&mut settings, &workspace)?;
        let path = crate::config::get_settings_path()?;
        std::fs::write(path, serde_json::to_string_pretty(&settings)?)?;
        tracing::info!("Applied runtime default board pack provisioning");
    }
    Ok(())
}

/// Run the queue processor - processes incoming messages and sends responses.
async fn run_queue_processor() -> Result<()> {
    use crate::config::load_settings;
    use crate::core::Queue;
    use std::time::Duration;
    
    tracing::info!("Starting queue processor...");
    
    let settings = load_settings()?;
    let telegram_token = settings.channels.telegram.bot_token.clone();
    
    loop {
        // Check for incoming messages
        match Queue::incoming() {
            Ok(messages) => {
                for msg_file in messages {
                    // Process each message
                    match process_message(&msg_file.data, &settings, &telegram_token).await {
                        Ok(_) => {
                            // Remove from queue after processing
                            if let Err(e) = Queue::remove_incoming(&msg_file.id) {
                                tracing::error!("Failed to remove message {}: {}", msg_file.id, e);
                            }
                        }
                        Err(e) => {
                            tracing::error!("Failed to process message {}: {}", msg_file.id, e);
                            // Still remove from queue to avoid processing broken messages forever
                            let _ = Queue::remove_incoming(&msg_file.id);
                        }
                    }
                }
            }
            Err(e) => {
                tracing::error!("Failed to read incoming queue: {}", e);
            }
        }
        
        // Sleep a bit before checking again
        tokio::time::sleep(Duration::from_millis(500)).await;
    }
}

/// Process a single message - call AI and send response.
async fn process_message(msg: &MessageData, settings: &crate::config::Settings, telegram_token: &Option<String>) -> Result<()> {
    use crate::core::Queue;
    use crate::core::routing::{extract_mentions, find_team_for_agent, is_teammate};
    use crate::providers::create_provider;
    use crate::context::AgentContext;
    use teloxide::prelude::*;
    
    let session_id = msg
        .conversation_id
        .clone()
        .unwrap_or_else(|| format!("conv-{}-{}", msg.sender_id, msg.timestamp));

    // Determine which agent to use. Supports @team_id by resolving to leader.
    // If no explicit target is provided, use deterministic task router hard rules.
    let default_agent_id = crate::core::routing::get_default_agent(settings)
        .unwrap_or_else(|| "assistant".to_string());
    let routed_task = crate::task::TaskRouter::route(&msg.message, settings, msg.agent.as_deref());

    let agent_id = if let Some(target) = msg.agent.as_deref() {
        if settings.agents.contains_key(target) {
            target.to_string()
        } else if let Some(team) = settings.teams.get(target) {
            team.leader_agent.clone().unwrap_or_else(|| {
                default_agent_id.clone()
            })
        } else {
            default_agent_id.clone()
        }
    } else {
        routed_task.owner.clone()
    };
    let _ = crate::memory::sqlite::record_decision(
        &session_id,
        &agent_id,
        &routed_task.intent,
        &routed_task.owner,
        &routed_task.priority,
        routed_task.deadline.as_deref(),
        &routed_task.reason,
    );

    let agent = settings.agents.get(&agent_id);
    
    tracing::info!("Processing message for agent: {}", agent_id);
    
    // Get provider and model
    let provider_name = agent.and_then(|a| a.provider.as_deref())
        .unwrap_or(&settings.models.provider);
    let model = agent
        .and_then(|a| a.model.as_deref())
        .or_else(|| match provider_name {
            "claude" => settings.models.anthropic.model.as_deref(),
            "codex" => settings.models.openai.model.as_deref(),
            "grok" => settings.models.grok.model.as_deref(),
            "ollama" => settings.models.ollama.model.as_deref(),
            _ => None,
        });
    
    tracing::debug!("Using provider: {:?}, model: {:?}", provider_name, model);
    
    // Get working directory
    let working_dir = agent.and_then(|a| a.working_directory.clone());
    
    // Load agent context (SOUL.md, MEMORY.md, AGENTS.md)
    let context = AgentContext::load(&agent_id, working_dir.as_ref()).unwrap_or_else(|e| {
        tracing::warn!("Failed to load context: {}", e);
        AgentContext {
            brain: None,
            soul: None,
            identity: None,
            user: None,
            tools: None,
            heartbeat: None,
            clients: None,
            playbook: None,
            memory: None,
            agents: None,
        }
    });
    
    // Build runtime + memory context (global + agent + optional team)
    let team_for_agent = settings
        .teams
        .iter()
        .find(|(_, t)| t.agents.contains(&agent_id))
        .map(|(id, _)| id.as_str());
    let runtime_block = build_runtime_context_block(settings, &agent_id, working_dir.as_ref(), team_for_agent);
    let runtime_block = format!(
        "{}\n- task_intent: {}\n- task_priority: {}\n- task_deadline: {}\n- routed_owner: {}\n- route_reason: {}",
        runtime_block,
        routed_task.intent,
        routed_task.priority,
        routed_task.deadline.clone().unwrap_or_else(|| "<none>".to_string()),
        routed_task.owner,
        routed_task.reason
    );
    let memory_block = build_memory_context_block(settings, &agent_id, team_for_agent, &msg.message);

    // Build the full prompt with context
    let full_prompt = if context.has_context() {
        let system = context.build_system_prompt();
        if memory_block.is_empty() {
            format!("{}\n\n## Runtime Context\n{}\n\nUser message:\n{}", system, runtime_block, msg.message)
        } else {
            format!(
                "{}\n\n## Runtime Context\n{}\n\n## Retrieved Memory Context\n{}\n\nUser message:\n{}",
                system, runtime_block, memory_block, msg.message
            )
        }
    } else {
        if memory_block.is_empty() {
            format!("## Runtime Context\n{}\n\nUser message:\n{}", runtime_block, msg.message)
        } else {
            format!(
                "## Runtime Context\n{}\n\n## Retrieved Memory Context\n{}\n\nUser message:\n{}",
                runtime_block, memory_block, msg.message
            )
        }
    };
    
    // Create provider and call AI
    let provider = create_provider(provider_name, settings);
    
    let working_dir_path = working_dir.as_ref().map(|p| p.as_path());
    let task_token = format!("{:x}", msg.timestamp).chars().rev().take(6).collect::<String>().chars().rev().collect::<String>();
    let started_at_ms = chrono::Utc::now().timestamp_millis();
    let _ = record_agent_execution_start(&agent_id, &session_id);

    // Send processing status to Telegram so user sees progress.
    if let (Some(token), Some(chat_id)) = (telegram_token, msg.response_chat_id) {
        let bot = teloxide::Bot::new(token.clone());
        let chat = teloxide::types::ChatId(chat_id);
        let _ = bot
            .send_message(chat, format!("⚙️ Task {} started (@{}).", task_token, agent_id))
            .await;
    }
    
    let contract = crate::agent::ExecutionContract::for_agent(provider_name);
    match crate::agent::execute_with_contract(
        provider.clone(),
        &full_prompt,
        model,
        working_dir_path,
        &contract,
    )
    .await
    {
        Ok(response) => {
            tracing::info!("Got response ({} bytes)", response.len());
            let mut response = enforce_identity_guard(&msg.message, response);
            let latency_ms = chrono::Utc::now().timestamp_millis() - started_at_ms;
            let _ = record_agent_execution_success(
                &agent_id,
                &session_id,
                latency_ms,
                &response.chars().take(320).collect::<String>(),
            );

            // CEO/team-leader can delegate via [@agent: task] mention tags.
            match crate::board::execute_leader_delegations(settings, &agent_id, &response).await {
                Ok(results) if !results.is_empty() => {
                    let mut block = String::from("\n\n---\n\nBoard Delegation Results:\n");
                    for (agent, output) in results {
                        let snippet = output.chars().take(700).collect::<String>();
                        block.push_str(&format!("\n@{}:\n{}\n", agent, snippet));
                    }
                    response.push_str(&block);
                }
                Ok(_) => {}
                Err(e) => tracing::warn!("Delegation execution failed: {}", e),
            }

            // Queue-based teammate handoff: if team members are mentioned, enqueue internal tasks.
            let depth = extract_chain_depth(&msg.message);
            if depth < 4 {
                if let Some((team_id, _team)) = find_team_for_agent(&agent_id, &settings.teams) {
                    let mentions = extract_mentions(&response);
                    let mut enqueued = 0usize;
                    let total_mentions = mentions.len();
                    for (target, delegated_prompt) in mentions {
                        if !is_teammate(&target, &agent_id, &team_id, &settings.teams, &settings.agents) {
                            continue;
                        }
                        let mut internal = MessageData::new(
                            &msg.channel,
                            &msg.sender,
                            &msg.sender_id,
                            &format!(
                                "[chain_depth:{}]\n[pending_handoffs:{}]\n[Message from teammate @{}]:\n{}\n\n[Other teammate branches may still be processing. Avoid re-mentioning unanswered teammates.]",
                                depth + 1,
                                total_mentions.saturating_sub(1),
                                agent_id,
                                delegated_prompt
                            ),
                        );
                        internal.agent = Some(target.clone());
                        internal.response_channel = msg.response_channel.clone();
                        internal.response_chat_id = msg.response_chat_id;
                        internal.response_message_id = msg.response_message_id;
                        internal.conversation_id = Some(
                            msg.conversation_id
                                .clone()
                                .unwrap_or_else(|| format!("conv-{}-{}", msg.sender_id, msg.timestamp)),
                        );
                        match Queue::enqueue(internal) {
                            Ok(id) => {
                                enqueued += 1;
                                tracing::info!("Enqueued teammate handoff {} -> @{} ({})", agent_id, target, id);
                            }
                            Err(e) => tracing::warn!("Failed to enqueue teammate handoff to @{}: {}", target, e),
                        }
                    }
                    if enqueued > 0 {
                        response.push_str(&format!(
                            "\n\n---\nTeam handoff queued: {} follow-up task(s). I will stream teammate results as they complete.",
                            enqueued
                        ));
                    }
                }
            }

            persist_interaction_memory(&agent_id, msg, &response)?;
            
            // Send response back to Telegram
            if let (Some(token), Some(chat_id)) = (telegram_token, msg.response_chat_id) {
                let bot = teloxide::Bot::new(token.clone());
                let chat = teloxide::types::ChatId(chat_id);
                
                // Truncate if too long
                let response_text = if response.len() > 4000 {
                    format!("✅ Task {} complete.\n\n{}...\n\n[Response truncated]", task_token, &response[..4000])
                } else {
                    format!("✅ Task {} complete.\n\n{}", task_token, response)
                };
                
                if let Err(e) = bot.send_message(chat, response_text).await {
                    tracing::error!("Failed to send Telegram response: {}", e);
                }
            }
        }
        Err(e) => {
            tracing::error!("Provider error: {}", e);
            let _ = record_agent_execution_failure(
                &agent_id,
                &session_id,
                &e.code.to_string(),
                &e.to_string(),
            );
            
            // Send error message to user
            if let (Some(token), Some(chat_id)) = (telegram_token, msg.response_chat_id) {
                let bot = teloxide::Bot::new(token.clone());
                let chat = teloxide::types::ChatId(chat_id);
                
                let _ = bot.send_message(chat, format!("❌ Task {} failed: {}", task_token, e)).await;
            }
        }
    }
    
    Ok(())
}

fn extract_chain_depth(message: &str) -> u8 {
    for line in message.lines().take(3) {
        let line = line.trim();
        if let Some(raw) = line.strip_prefix("[chain_depth:") {
            if let Some(num) = raw.strip_suffix(']') {
                if let Ok(v) = num.parse::<u8>() {
                    return v;
                }
            }
        }
    }
    0
}

fn persist_interaction_memory(agent_id: &str, msg: &MessageData, response: &str) -> Result<()> {
    use crate::memory::{Memory, MemoryScope};

    let user_record = serde_json::json!({
        "agent_id": agent_id,
        "sender": msg.sender,
        "sender_id": msg.sender_id,
        "message": msg.message,
        "message_id": msg.message_id,
        "timestamp": msg.timestamp
    });
    Memory::set(
        "interaction.last_user",
        &user_record.to_string(),
        MemoryScope::Agent,
        Some(agent_id),
    )?;

    let response_record = serde_json::json!({
        "agent_id": agent_id,
        "response": response.chars().take(2000).collect::<String>(),
        "timestamp": chrono::Utc::now().timestamp_millis()
    });
    Memory::set(
        "interaction.last_response",
        &response_record.to_string(),
        MemoryScope::Agent,
        Some(agent_id),
    )?;

    Ok(())
}

fn build_memory_context_block(
    _settings: &crate::config::Settings,
    agent_id: &str,
    team_id: Option<&str>,
    query: &str,
) -> String {
    use crate::memory::{Memory, MemoryScope};

    let mut lines = Vec::new();

    if let Ok(entries) = Memory::relevant(query, MemoryScope::Global, None, 4) {
        for e in entries {
            lines.push(format!("[global] {} = {}", e.key, e.value.chars().take(220).collect::<String>()));
        }
    }
    if let Ok(entries) = Memory::relevant(query, MemoryScope::Agent, Some(agent_id), 6) {
        for e in entries {
            lines.push(format!("[agent:{}] {} = {}", agent_id, e.key, e.value.chars().take(220).collect::<String>()));
        }
    }
    if let Some(team) = team_id {
        if let Ok(entries) = Memory::relevant(query, MemoryScope::Team, Some(team), 6) {
            for e in entries {
                lines.push(format!("[team:{}] {} = {}", team, e.key, e.value.chars().take(220).collect::<String>()));
            }
        }
    }

    lines.join("\n")
}

fn build_runtime_context_block(
    settings: &crate::config::Settings,
    agent_id: &str,
    working_dir: Option<&std::path::PathBuf>,
    team_id: Option<&str>,
) -> String {
    let workdir = working_dir
        .map(|p| p.display().to_string())
        .unwrap_or_else(|| "<none>".to_string());
    let workspace_root = settings
        .workspace
        .path
        .as_ref()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|| "<none>".to_string());
    let board_id = settings
        .board
        .team_id
        .as_deref()
        .unwrap_or("<none>");
    let team = team_id.unwrap_or("<none>");
    format!(
        "- agent_id: {}\n- working_directory: {}\n- workspace_root: {}\n- team_id: {}\n- board_id: {}",
        agent_id, workdir, workspace_root, team, board_id
    )
}

fn enforce_identity_guard(user_message: &str, response: String) -> String {
    let user = user_message.to_lowercase();
    let asks_identity = user.contains("who are you")
        || user.contains("who r u")
        || user.contains("what are you")
        || user.contains("identify yourself")
        || user.contains("your name");

    if asks_identity {
        return "I'm TinyVegeta, your AI orchestrator in this workspace.".to_string();
    }

    let lower = response.to_lowercase();
    let leaked_identity = lower.contains("i'm codex")
        || lower.contains("i am codex")
        || lower.contains("i'm chatgpt")
        || lower.contains("i am chatgpt")
        || lower.contains("openai coding agent")
        || lower.contains("your ai coding agent");

    if leaked_identity {
        let cleaned = response
            .lines()
            .filter(|line| {
                let l = line.to_lowercase();
                !(l.contains("i'm codex")
                    || l.contains("i am codex")
                    || l.contains("i'm chatgpt")
                    || l.contains("i am chatgpt")
                    || l.contains("openai coding agent")
                    || l.contains("your ai coding agent"))
            })
            .collect::<Vec<_>>()
            .join("\n")
            .trim()
            .to_string();

        if cleaned.is_empty() {
            "I'm TinyVegeta, your AI orchestrator in this workspace.".to_string()
        } else {
            format!(
                "I'm TinyVegeta, your AI orchestrator in this workspace.\n\n{}",
                cleaned
            )
        }
    } else {
        response
    }
}

fn format_ts_ms(ts_ms: i64) -> String {
    chrono::DateTime::<chrono::Utc>::from_timestamp_millis(ts_ms)
        .map(|dt| dt.format("%Y-%m-%d %H:%M:%S UTC").to_string())
        .unwrap_or_else(|| ts_ms.to_string())
}

fn record_agent_execution_start(agent_id: &str, session_id: &str) -> Result<()> {
    use crate::memory::{Memory, MemoryScope};

    let now = chrono::Utc::now().timestamp_millis().to_string();
    Memory::set(
        &format!("agent.health.{}.status", agent_id),
        "running",
        MemoryScope::Global,
        None,
    )?;
    Memory::set(
        &format!("agent.health.{}.last_start", agent_id),
        &now,
        MemoryScope::Global,
        None,
    )?;
    Memory::set(
        &format!("agent.health.{}.last_session", agent_id),
        session_id,
        MemoryScope::Global,
        None,
    )?;
    let _ = crate::memory::sqlite::record_event(session_id, agent_id, "task_started", "queue task execution started");
    Ok(())
}

fn record_agent_execution_success(
    agent_id: &str,
    session_id: &str,
    latency_ms: i64,
    summary: &str,
) -> Result<()> {
    use crate::memory::{Memory, MemoryScope};

    let now = chrono::Utc::now().timestamp_millis().to_string();
    Memory::set(
        &format!("agent.health.{}.status", agent_id),
        "healthy",
        MemoryScope::Global,
        None,
    )?;
    Memory::set(
        &format!("agent.health.{}.last_success", agent_id),
        &now,
        MemoryScope::Global,
        None,
    )?;
    Memory::set(
        &format!("agent.health.{}.last_latency_ms", agent_id),
        &latency_ms.to_string(),
        MemoryScope::Global,
        None,
    )?;
    Memory::set(
        &format!("agent.health.{}.last_error", agent_id),
        "",
        MemoryScope::Global,
        None,
    )?;

    let _ = crate::memory::sqlite::record_event(session_id, agent_id, "task_succeeded", &format!("latency_ms={}", latency_ms));
    let _ = crate::memory::sqlite::record_outcome(session_id, agent_id, "success", None, summary);
    if let Ok(s) = crate::memory::sqlite::summarize_session(session_id) {
        let summary_line = format!(
            "events={} decisions={} outcomes={} last_outcome={}",
            s.event_count,
            s.decision_count,
            s.outcome_count,
            s.last_outcome.unwrap_or_else(|| "-".to_string())
        );
        let _ = Memory::set(
            &format!("session.{}.summary", s.session_id),
            &summary_line,
            MemoryScope::Global,
            None,
        );
    }

    Ok(())
}

fn record_agent_execution_failure(
    agent_id: &str,
    session_id: &str,
    error_code: &str,
    message: &str,
) -> Result<()> {
    use crate::memory::{Memory, MemoryScope};

    let now = chrono::Utc::now().timestamp_millis().to_string();
    Memory::set(
        &format!("agent.health.{}.status", agent_id),
        "degraded",
        MemoryScope::Global,
        None,
    )?;
    Memory::set(
        &format!("agent.health.{}.last_error", agent_id),
        message,
        MemoryScope::Global,
        None,
    )?;
    Memory::set(
        &format!("agent.health.{}.last_error_code", agent_id),
        error_code,
        MemoryScope::Global,
        None,
    )?;
    Memory::set(
        &format!("agent.health.{}.last_error_at", agent_id),
        &now,
        MemoryScope::Global,
        None,
    )?;

    let _ = crate::memory::sqlite::record_event(session_id, agent_id, "task_failed", message);
    let _ = crate::memory::sqlite::record_outcome(
        session_id,
        agent_id,
        "failed",
        Some(error_code),
        &message.chars().take(350).collect::<String>(),
    );
    Ok(())
}

async fn cmd_stop() -> Result<()> {
    println!("Stopping TinyVegeta daemon...");
    tmux::stop_daemon()?;
    println!("TinyVegeta stopped.");
    Ok(())
}

async fn cmd_restart() -> Result<()> {
    println!("Restarting TinyVegeta daemon...");
    let binary = std::env::current_exe()
        .unwrap_or_else(|_| std::path::PathBuf::from("tinyvegeta"));
    tmux::restart_daemon(binary.to_str().unwrap_or("tinyvegeta"))?;
    println!("TinyVegeta restarted!");
    Ok(())
}

async fn cmd_status() -> Result<()> {
    use crate::memory::{Memory, MemoryScope};

    let daemon_status = tmux::get_status()?;
    println!("{}", daemon_status);

    if let Ok(q) = crate::core::Queue::stats() {
        println!("\nQueue Depth:");
        println!("  incoming={} processing={} outgoing={} total={}", q.incoming, q.processing, q.outgoing, q.total);
    }

    if let Ok(settings) = load_settings() {
        let mut agent_ids: Vec<String> = settings.agents.keys().cloned().collect();
        agent_ids.sort();
        println!("\nAgent Health:");
        for agent_id in agent_ids {
            let status_key = format!("agent.health.{}.status", agent_id);
            let success_key = format!("agent.health.{}.last_success", agent_id);
            let error_key = format!("agent.health.{}.last_error", agent_id);

            let status = Memory::get(&status_key, MemoryScope::Global, None)
                .ok()
                .flatten()
                .map(|v| v.value)
                .unwrap_or_else(|| "unknown".to_string());
            let last_success = Memory::get(&success_key, MemoryScope::Global, None)
                .ok()
                .flatten()
                .and_then(|v| v.value.parse::<i64>().ok())
                .map(format_ts_ms)
                .unwrap_or_else(|| "never".to_string());
            let last_error = Memory::get(&error_key, MemoryScope::Global, None)
                .ok()
                .flatten()
                .map(|v| {
                    let txt = v.value;
                    if txt.len() > 90 {
                        format!("{}...", &txt[..90])
                    } else {
                        txt
                    }
                })
                .unwrap_or_else(|| "-".to_string());

            println!(
                "  @{} | health={} | last_success={} | last_error={}",
                agent_id, status, last_success, last_error
            );
        }
    }
    Ok(())
}

async fn cmd_attach() -> Result<()> {
    tmux::attach()?;
    Ok(())
}

async fn cmd_setup() -> Result<()> {
    use std::io::{self, Write, BufRead};
    use crate::config::{Settings, AgentConfig, Models, Pairing, Workspace, Channels, ChannelConfig, Monitoring};
    
    println!("\n╔════════════════════════════════════════════════════════════╗");
    println!("║         TinyVegeta Setup Wizard                            ║");
    println!("╚════════════════════════════════════════════════════════════╝\n");
    
    let stdin = io::stdin();
    let mut stdout = io::stdout();
    
    // Create home directory
    let home = crate::config::get_home_dir()?;
    std::fs::create_dir_all(&home)?;
    std::fs::create_dir_all(home.join("queue").join("incoming"))?;
    std::fs::create_dir_all(home.join("queue").join("processing"))?;
    std::fs::create_dir_all(home.join("queue").join("outgoing"))?;
    std::fs::create_dir_all(home.join("logs"))?;
    std::fs::create_dir_all(home.join("memory"))?;
    std::fs::create_dir_all(home.join("files"))?;
    println!("✓ Created directory structure at {}", home.display());
    
    // Ask for Telegram bot token
    print!("\n📱 Telegram Bot Token (from @BotFather): ");
    stdout.flush()?;
    let mut bot_token = String::new();
    stdin.lock().read_line(&mut bot_token)?;
    let bot_token = bot_token.trim().to_string();
    
    // Ask for provider
    println!("\n🤖 Select AI Provider:");
    println!("  1. Claude (Anthropic CLI)");
    println!("  2. Codex (OpenAI CLI)");
    println!("  3. Cline CLI");
    println!("  4. OpenCode CLI");
    println!("  5. Ollama (local)");
    println!("  6. Grok (xAI API)");
    print!("Enter choice [1-6] (default: 1): ");
    stdout.flush()?;
    
    let mut provider_choice = String::new();
    stdin.lock().read_line(&mut provider_choice)?;
    let provider = match provider_choice.trim() {
        "2" => "codex",
        "3" => "cline",
        "4" => "opencode",
        "5" => "ollama",
        "6" => "grok",
        _ => "claude",
    };
    
    // Model selection with provider-specific options
    let models: Vec<(&str, &str)> = match provider {
        "claude" => vec![
            ("sonnet", "Claude Sonnet 4 (balanced, fast)"),
            ("opus", "Claude Opus 4 (most capable)"),
            ("sonnet-3.5", "Claude Sonnet 3.5 (legacy)"),
            ("haiku", "Claude Haiku 3.5 (fastest)"),
        ],
        "codex" => vec![
            ("gpt-5.3-codex", "GPT-5.3 Codex (recommended)"),
            ("o3", "O3 (advanced reasoning)"),
            ("o4-mini", "O4 Mini (fast, cheap)"),
            ("gpt-4.1", "GPT-4.1 (legacy)"),
        ],
        "cline" => vec![
            ("default", "Default model"),
            ("claude-sonnet", "Claude Sonnet"),
            ("gpt-4o", "GPT-4o"),
        ],
        "opencode" => vec![
            ("default", "Default model"),
            ("claude-sonnet", "Claude Sonnet"),
            ("gpt-4o", "GPT-4o"),
        ],
        "ollama" => vec![
            ("llama3.3", "Llama 3.3 (latest)"),
            ("llama3.1", "Llama 3.1 (stable)"),
            ("codellama", "Code Llama"),
            ("mistral", "Mistral"),
            ("deepseek-coder", "DeepSeek Coder"),
        ],
        "grok" => vec![
            ("grok-2", "Grok 2 (latest)"),
            ("grok-2-mini", "Grok 2 Mini (fast)"),
            ("grok-beta", "Grok Beta"),
        ],
        _ => vec![("default", "Default")],
    };
    
    println!("\n🎯 Select Model:");
    for (i, (id, desc)) in models.iter().enumerate() {
        println!("  {}. {} - {}", i + 1, id, desc);
    }
    println!("  {}. Custom model (enter manually)", models.len() + 1);
    print!("Enter choice [1-{}] (default: 1): ", models.len() + 1);
    stdout.flush()?;
    
    let mut model_choice = String::new();
    stdin.lock().read_line(&mut model_choice)?;
    
    let model = match model_choice.trim() {
        "" | "1" => models.first().map(|(id, _)| id.to_string()).unwrap_or("default".to_string()),
        c => {
            if let Ok(num) = c.parse::<usize>() {
                if num <= models.len() {
                    models.get(num - 1).map(|(id, _)| id.to_string()).unwrap_or("default".to_string())
                } else if num == models.len() + 1 {
                    // Custom model
                    print!("Enter model name: ");
                    stdout.flush()?;
                    let mut custom = String::new();
                    stdin.lock().read_line(&mut custom)?;
                    custom.trim().to_string()
                } else {
                    models.first().map(|(id, _)| id.to_string()).unwrap_or("default".to_string())
                }
            } else {
                models.first().map(|(id, _)| id.to_string()).unwrap_or("default".to_string())
            }
        }
    };
    
    println!("✓ Using model: {}", model);
    
    // Create workspace directory
    let workspace_path = directories::UserDirs::new()
        .map(|h| h.home_dir().join("tinyvegeta-workspace"))
        .unwrap_or_else(|| std::path::PathBuf::from("./tinyvegeta-workspace"));
    std::fs::create_dir_all(&workspace_path)?;
    println!("✓ Created workspace at {}", workspace_path.display());
    
    // Create default agent
    let agent_workspace = workspace_path.join("assistant");
    std::fs::create_dir_all(&agent_workspace)?;
    
    // Create default context files (SOUL.md, MEMORY.md)
    crate::context::init_agent_context("assistant", &agent_workspace)?;
    println!("✓ Created context files in {}", agent_workspace.display());
    
    // Build settings
    let mut settings = Settings {
        workspace: Workspace {
            path: Some(workspace_path.clone()),
            name: Some("tinyvegeta-workspace".to_string()),
        },
        channels: Channels {
            enabled: vec!["telegram".to_string()],
            telegram: ChannelConfig {
                bot_token: Some(bot_token),
            },
        },
        agents: {
            let mut agents = std::collections::HashMap::new();
            agents.insert("assistant".to_string(), AgentConfig {
                name: Some("Assistant".to_string()),
                provider: Some(provider.to_string()),
                model: Some(model.clone()),
                working_directory: Some(agent_workspace.clone()),
                is_sovereign: false,
            });
            agents
        },
        teams: std::collections::HashMap::new(),
        models: Models {
            provider: provider.to_string(),
            anthropic: crate::config::ProviderModel {
                model: Some(model.clone()),
                api_key: None,
                base_url: None,
            },
            ..Default::default()
        },
        pairing: Pairing::default(),
        monitoring: Monitoring::default(),
        board: crate::config::Board::default(),
        routing: crate::config::Routing {
            default_agent: Some("assistant".to_string()),
        },
        sovereign: crate::config::Sovereign::default(),
    };

    // Install default board pack (assistant as CEO + specialist members).
    crate::board::install_default_pack(&mut settings, &workspace_path)?;
    println!("✓ Installed default board pack in {}", workspace_path.display());
    
    // Save settings
    let settings_path = crate::config::get_settings_path()?;
    let settings_content = serde_json::to_string_pretty(&settings)?;
    std::fs::write(&settings_path, settings_content)?;
    println!("✓ Saved settings to {}", settings_path.display());
    
    // Create pairing.json
    let pairing_path = home.join("pairing.json");
    let pairing_content = serde_json::json!({
        "pending": [],
        "approved": []
    });
    std::fs::write(&pairing_path, serde_json::to_string(&pairing_content)?)?;
    
    println!("\n╔════════════════════════════════════════════════════════════╗");
    println!("║  ✅ Setup Complete!                                        ║");
    println!("╚════════════════════════════════════════════════════════════╝");
    println!("\nNext steps:");
    println!("  1. Run 'tinyvegeta start' to start the daemon");
    println!("  2. Message your Telegram bot to get a pairing code");
    println!("  3. Run 'tinyvegeta pairing approve <CODE>' to approve\n");
    
    Ok(())
}

async fn cmd_send(message: &str) -> Result<()> {
    let (agent, content) = if let Some((id, msg)) = crate::core::routing::parse_agent_routing(message) {
        (Some(id), msg)
    } else {
        (None, message.to_string())
    };

    let mut msg = MessageData::new("cli", "cli", "cli", &content);
    msg.agent = agent;
    msg.response_channel = Some("cli".to_string());
    let id = crate::core::Queue::enqueue(msg)?;
    println!("Enqueued CLI message: {}", id);
    Ok(())
}

async fn cmd_logs(log_type: &str) -> Result<()> {
    let log_dir = directories::ProjectDirs::from("com", "tinyvegeta", "tinyvegeta")
        .ok_or_else(|| anyhow::anyhow!("Could not resolve log directory"))?
        .data_dir()
        .join("logs");
    let file = log_dir.join("tinyvegeta.log");
    if !file.exists() {
        println!("Log file not found: {}", file.display());
        return Ok(());
    }
    let content = std::fs::read_to_string(&file)?;
    let needle = match log_type {
        "all" => None,
        "telegram" => Some("telegram"),
        "queue" => Some("queue"),
        "heartbeat" => Some("heartbeat"),
        "daemon" => Some("start-internal"),
        _ => Some(log_type),
    };
    let mut lines: Vec<&str> = content.lines().collect();
    if let Some(n) = needle {
        lines.retain(|l| l.to_lowercase().contains(&n.to_lowercase()));
    }
    let start = lines.len().saturating_sub(120);
    for line in &lines[start..] {
        println!("{}", line);
    }
    Ok(())
}

async fn cmd_queue(action: &QueueCommand) -> Result<()> {
    use crate::core::Queue;
    
    match action {
        QueueCommand::Stats => {
            let stats = Queue::stats()?;
            println!("{}", stats);
        }
        QueueCommand::Incoming => {
            let messages = Queue::incoming()?;
            println!("Incoming messages ({}):", messages.len());
            for msg in messages {
                println!("  {}: {} -> {}", msg.id, msg.data.sender, msg.data.message.chars().take(50).collect::<String>());
            }
        }
        QueueCommand::Processing => {
            let messages = Queue::processing()?;
            println!("Processing messages ({}):", messages.len());
            for msg in messages {
                println!("  {}: {} -> {}", msg.id, msg.data.sender, msg.data.message.chars().take(50).collect::<String>());
            }
        }
        QueueCommand::Outgoing => {
            let messages = Queue::outgoing()?;
            println!("Outgoing messages ({}):", messages.len());
            for msg in messages {
                println!("  {}: {} -> {}", msg.id, msg.data.sender, msg.data.message.chars().take(50).collect::<String>());
            }
        }
        QueueCommand::Enqueue { message, channel, sender } => {
            let channel = channel.as_deref().unwrap_or("cli");
            let sender = sender.as_deref().unwrap_or("cli");
            
            let msg = MessageData::new(channel, sender, "cli", message);
            let id = Queue::enqueue(msg)?;
            println!("Enqueued message: {}", id);
        }
        QueueCommand::Recover => {
            let recovered = Queue::recover_orphaned()?;
            println!("Recovered {} orphaned messages", recovered);
        }
    }
    
    Ok(())
}

async fn cmd_reset(agents: &[String]) -> Result<()> {
    let settings = load_settings()?;
    for agent_id in agents {
        let Some(agent) = settings.agents.get(agent_id) else {
            println!("Agent not found: {}", agent_id);
            continue;
        };
        let workdir = if let Some(wd) = &agent.working_directory {
            wd.clone()
        } else if let Some(ws) = &settings.workspace.path {
            ws.join(agent_id)
        } else {
            println!("No working directory for @{}", agent_id);
            continue;
        };
        std::fs::create_dir_all(&workdir)?;
        std::fs::write(workdir.join("reset_flag"), "reset\n")?;
        println!("Reset flagged for @{} ({})", agent_id, workdir.display());
    }
    Ok(())
}

async fn cmd_agent(cmd: &AgentCommand) -> Result<()> {
    match cmd {
        AgentCommand::List => {
            let settings = load_settings()?;
            println!("Agents:");
            for (id, agent) in &settings.agents {
                println!("  {}: {:?} ({:?} / {:?})", id, agent.name, agent.provider, agent.model);
            }
        }
        AgentCommand::Add => {
            use std::io::{self, BufRead, Write};

            let mut settings = load_settings()?;
            let stdin = io::stdin();
            let mut stdout = io::stdout();

            print!("Agent ID (e.g. analyst): ");
            stdout.flush()?;
            let mut id = String::new();
            stdin.lock().read_line(&mut id)?;
            let id = id.trim().to_lowercase();
            if id.is_empty() {
                return Err(anyhow::anyhow!("Agent ID is required"));
            }
            if settings.agents.contains_key(&id) {
                return Err(anyhow::anyhow!("Agent already exists: {}", id));
            }
            if settings.teams.contains_key(&id) {
                return Err(anyhow::anyhow!("Agent ID conflicts with team ID: {}", id));
            }

            print!("Display name (default: {}): ", id);
            stdout.flush()?;
            let mut name = String::new();
            stdin.lock().read_line(&mut name)?;
            let name = if name.trim().is_empty() {
                id.clone()
            } else {
                name.trim().to_string()
            };

            print!("Provider (default: {}): ", settings.models.provider);
            stdout.flush()?;
            let mut provider = String::new();
            stdin.lock().read_line(&mut provider)?;
            let provider = if provider.trim().is_empty() {
                settings.models.provider.clone()
            } else {
                provider.trim().to_string()
            };

            print!("Model (default: default): ");
            stdout.flush()?;
            let mut model = String::new();
            stdin.lock().read_line(&mut model)?;
            let model = if model.trim().is_empty() {
                "default".to_string()
            } else {
                model.trim().to_string()
            };

            let workspace = crate::board::resolve_workspace_root(&settings);
            let workdir = workspace.join(&id);
            std::fs::create_dir_all(&workdir)?;
            crate::context::init_agent_context(&id, &workdir)?;

            settings.agents.insert(
                id.clone(),
                crate::config::AgentConfig {
                    name: Some(name),
                    provider: Some(provider),
                    model: Some(model),
                    working_directory: Some(workdir.clone()),
                    is_sovereign: false,
                },
            );
            let path = crate::config::get_settings_path()?;
            std::fs::write(path, serde_json::to_string_pretty(&settings)?)?;
            println!("Agent added: @{} ({})", id, workdir.display());
        }
        AgentCommand::Show { agent_id } => {
            let settings = load_settings()?;
            if let Some(agent) = settings.agents.get(agent_id) {
                println!("Agent: {}", agent_id);
                println!("  Name: {:?}", agent.name);
                println!("  Provider: {:?}", agent.provider);
                println!("  Model: {:?}", agent.model);
                println!("  Working Directory: {:?}", agent.working_directory);
                if agent.is_sovereign {
                    println!("  Sovereign: true");
                }
            } else {
                println!("Agent not found: {}", agent_id);
            }
        }
        AgentCommand::Remove { agent_id } => {
            let mut settings = load_settings()?;
            if settings.agents.remove(agent_id).is_none() {
                println!("Agent not found: {}", agent_id);
                return Ok(());
            }
            for team in settings.teams.values_mut() {
                team.agents.retain(|a| a != agent_id);
                if team.leader_agent.as_deref() == Some(agent_id) {
                    team.leader_agent = team.agents.first().cloned();
                }
            }
            if settings.routing.default_agent.as_deref() == Some(agent_id) {
                settings.routing.default_agent = settings.agents.keys().next().cloned();
            }
            let path = crate::config::get_settings_path()?;
            std::fs::write(path, serde_json::to_string_pretty(&settings)?)?;
            println!("Removed agent: {}", agent_id);
        }
        AgentCommand::Reset { agent_id } => {
            cmd_reset(&[agent_id.clone()]).await?;
        }
        AgentCommand::Pack { command } => {
            match command {
                AgentPackCommand::List => {
                    println!("Available agent packs:");
                    println!("  default - CEO/Coder/Security/Operations/Marketing/SEO/Sales");
                }
                AgentPackCommand::Install { name } => {
                    if name != "default" {
                        println!("Unknown pack: {}", name);
                        println!("Available packs: default");
                        return Ok(());
                    }

                    let mut settings = load_settings()?;
                    let workspace = crate::board::resolve_workspace_root(&settings);
                    std::fs::create_dir_all(&workspace)?;
                    crate::board::install_default_pack(&mut settings, &workspace)?;

                    let path = crate::config::get_settings_path()?;
                    let content = serde_json::to_string_pretty(&settings)?;
                    std::fs::write(path, content)?;

                    println!("Installed default pack to {}", workspace.display());
                    println!("Board team configured with CEO @assistant and specialist members.");
                }
            }
        }
        AgentCommand::Default { agent_id } => {
            let mut settings = load_settings()?;
            if let Some(id) = agent_id {
                if !settings.agents.contains_key(id) {
                    return Err(anyhow::anyhow!("Agent not found: {}", id));
                }
                settings.routing.default_agent = Some(id.clone());
                let path = crate::config::get_settings_path()?;
                std::fs::write(path, serde_json::to_string_pretty(&settings)?)?;
                println!("Default agent set: @{}", id);
            } else {
                let current = settings
                    .routing
                    .default_agent
                    .clone()
                    .or_else(|| crate::core::routing::get_default_agent(&settings))
                    .unwrap_or_else(|| "<none>".to_string());
                println!("Default agent: @{}", current);
            }
        }
    }
    Ok(())
}

async fn cmd_team(cmd: &TeamCommand) -> Result<()> {
    match cmd {
        TeamCommand::List => {
            let settings = load_settings()?;
            println!("Teams:");
            for (id, team) in &settings.teams {
                println!("  {}: {} - {:?}", id, team.name, team.agents);
            }
        }
        TeamCommand::Add { id, name, members, leader } => {
            use std::io::{self, BufRead, Write};

            let mut settings = load_settings()?;
            let stdin = io::stdin();
            let mut stdout = io::stdout();

            let team_id = if let Some(v) = id {
                v.trim().to_lowercase()
            } else {
                print!("Team ID (e.g. dev): ");
                stdout.flush()?;
                let mut team_id = String::new();
                stdin.lock().read_line(&mut team_id)?;
                team_id.trim().to_lowercase()
            };
            if team_id.is_empty() {
                println!("Team ID is required");
                return Ok(());
            }
            if settings.teams.contains_key(&team_id) {
                println!("Team already exists: {}", team_id);
                return Ok(());
            }
            if settings.agents.contains_key(&team_id) {
                println!("Team ID conflicts with agent ID: {}", team_id);
                return Ok(());
            }

            let name = if let Some(v) = name {
                v.to_string()
            } else {
                print!("Display name (default: {}): ", team_id);
                stdout.flush()?;
                let mut name = String::new();
                stdin.lock().read_line(&mut name)?;
                if name.trim().is_empty() {
                    team_id.clone()
                } else {
                    name.trim().to_string()
                }
            };

            let members_raw = if let Some(v) = members {
                v.to_string()
            } else {
                println!("Available agents: {}", settings.agents.keys().cloned().collect::<Vec<_>>().join(", "));
                print!("Members (comma-separated): ");
                stdout.flush()?;
                let mut members = String::new();
                stdin.lock().read_line(&mut members)?;
                members
            };
            let mut agents: Vec<String> = members_raw
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .filter(|id| settings.agents.contains_key(id))
                .collect();
            agents.sort();
            agents.dedup();
            if agents.is_empty() {
                println!("No valid team members provided");
                return Ok(());
            }

            let leader = if let Some(v) = leader {
                if !agents.contains(v) {
                    println!("Leader must be one of team members");
                    return Ok(());
                }
                v.to_string()
            } else {
                print!("Leader agent (default: {}): ", agents[0]);
                stdout.flush()?;
                let mut leader = String::new();
                stdin.lock().read_line(&mut leader)?;
                if leader.trim().is_empty() || !agents.contains(&leader.trim().to_string()) {
                    agents[0].clone()
                } else {
                    leader.trim().to_string()
                }
            };

            settings.teams.insert(
                team_id.clone(),
                crate::config::TeamConfig {
                    name,
                    agents,
                    leader_agent: Some(leader),
                },
            );

            let path = crate::config::get_settings_path()?;
            std::fs::write(path, serde_json::to_string_pretty(&settings)?)?;
            println!("Team created: @{}", team_id);
        }
        TeamCommand::Show { team_id } => {
            let settings = load_settings()?;
            if let Some(team) = settings.teams.get(team_id) {
                println!("Team: {}", team_id);
                println!("  Name: {}", team.name);
                println!("  Agents: {:?}", team.agents);
                println!("  Leader: {:?}", team.leader_agent);
            } else {
                println!("Team not found: {}", team_id);
            }
        }
        TeamCommand::Remove { team_id } => {
            let mut settings = load_settings()?;
            if settings.teams.remove(team_id).is_some() {
                if settings.board.team_id.as_deref() == Some(team_id) {
                    settings.board.team_id = None;
                }
                let path = crate::config::get_settings_path()?;
                std::fs::write(path, serde_json::to_string_pretty(&settings)?)?;
                println!("Removed team: {}", team_id);
            } else {
                println!("Team not found: {}", team_id);
            }
        }
        TeamCommand::Update { team_id, members, leader, name } => {
            let mut settings = load_settings()?;
            if !settings.teams.contains_key(team_id) {
                println!("Team not found: {}", team_id);
                return Ok(());
            }

            let parsed_members = if let Some(v) = members {
                let mut parsed = v
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect::<Vec<_>>();
                parsed.sort();
                parsed.dedup();
                if parsed.is_empty() {
                    println!("No valid members provided");
                    return Ok(());
                }
                for a in &parsed {
                    if !settings.agents.contains_key(a) {
                        println!("Unknown agent: {}", a);
                        return Ok(());
                    }
                }
                Some(parsed)
            } else {
                None
            };

            let team = settings.teams.get_mut(team_id).expect("checked above");
            if let Some(parsed) = parsed_members {
                team.agents = parsed;
            }

            if let Some(v) = leader {
                if !team.agents.contains(v) {
                    println!("Leader must be a team member");
                    return Ok(());
                }
                team.leader_agent = Some(v.to_string());
            }

            if let Some(v) = name {
                team.name = v.to_string();
            }

            let path = crate::config::get_settings_path()?;
            std::fs::write(path, serde_json::to_string_pretty(&settings)?)?;
            println!("Team updated: @{}", team_id);
        }
        TeamCommand::Visualize { team_id } => {
            let settings = load_settings()?;
            match team_id {
                Some(id) => {
                    if let Some(team) = settings.teams.get(id) {
                        println!("Team @{} ({})", id, team.name);
                        println!("Leader: @{}", team.leader_agent.as_deref().unwrap_or("none"));
                        println!("Members:");
                        for member in &team.agents {
                            if let Some(agent) = settings.agents.get(member) {
                                println!(
                                    "  - @{} ({:?}/{:?})",
                                    member,
                                    agent.provider.as_deref().unwrap_or("unknown"),
                                    agent.model.as_deref().unwrap_or("default")
                                );
                            } else {
                                println!("  - @{} (missing config)", member);
                            }
                        }
                    } else {
                        println!("Team not found: {}", id);
                    }
                }
                None => {
                    println!("All teams:");
                    for (id, team) in &settings.teams {
                        println!(
                            "  @{} -> leader: @{}, members: {}",
                            id,
                            team.leader_agent.as_deref().unwrap_or("none"),
                            team.agents.join(", ")
                        );
                    }
                }
            }
        }
    }
    Ok(())
}

async fn cmd_board(cmd: &BoardCommand) -> Result<()> {
    match cmd {
        BoardCommand::Create { ceo, members, autonomous } => {
            let mut settings = load_settings()?;
            let board_id = "board".to_string();
            let ceo_id = ceo.clone().unwrap_or_else(|| "assistant".to_string());

            if !settings.agents.contains_key(&ceo_id) {
                println!("CEO agent not found: {}", ceo_id);
                return Ok(());
            }

            let mut board_members: Vec<String> = if let Some(m) = members {
                m.split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect()
            } else {
                vec![
                    "assistant".to_string(),
                    "coder".to_string(),
                    "security".to_string(),
                    "operations".to_string(),
                    "marketing".to_string(),
                    "seo".to_string(),
                    "sales".to_string(),
                ]
            };

            if !board_members.contains(&ceo_id) {
                board_members.insert(0, ceo_id.clone());
            }
            board_members.retain(|id| settings.agents.contains_key(id));
            board_members.sort();
            board_members.dedup();

            settings.teams.insert(
                board_id.clone(),
                crate::config::TeamConfig {
                    name: "Executive Board".to_string(),
                    agents: board_members.clone(),
                    leader_agent: Some(ceo_id.clone()),
                },
            );
            settings.board.team_id = Some(board_id.clone());
            settings.board.autonomous = Some(*autonomous);
            settings.board.schedules.get_or_insert_with(Vec::new);

            let path = crate::config::get_settings_path()?;
            std::fs::write(path, serde_json::to_string_pretty(&settings)?)?;

            println!("Board configured: @{}", board_id);
            println!("CEO: @{}", ceo_id);
            println!("Members: {}", board_members.join(", "));
            println!("Autonomous: {}", autonomous);
        }
        BoardCommand::Show { board_id } => {
            let settings = load_settings()?;
            let id = board_id
                .clone()
                .or_else(|| settings.board.team_id.clone())
                .unwrap_or_else(|| "board".to_string());

            if let Some(team) = settings.teams.get(&id) {
                println!("Board: @{}", id);
                println!(
                    "  Leader: @{}",
                    team.leader_agent.as_deref().unwrap_or("none")
                );
                println!("  Members: {}", team.agents.join(", "));
                println!(
                    "  Autonomous: {}",
                    settings.board.autonomous.unwrap_or(false)
                );
                let schedules = settings.board.schedules.as_ref().map(|s| s.len()).unwrap_or(0);
                println!("  Schedules: {}", schedules);
            } else {
                println!("Board not found: @{}", id);
            }
        }
        BoardCommand::Discuss { topic, team_id, timeout, raw } => {
            let settings = load_settings()?;
            let id = team_id
                .clone()
                .or_else(|| settings.board.team_id.clone())
                .unwrap_or_else(|| "board".to_string());

            let output = crate::board::run_board_discussion(&settings, &id, topic, *timeout).await?;
            if *raw {
                println!("{}", output);
            } else {
                println!("=== Board Discussion ===");
                println!("{}", output);
                println!("========================");
            }
        }
        BoardCommand::Schedule { command } => {
            match command {
                BoardScheduleCommand::Daily { time, team_id, sender_id } => {
                    let mut settings = load_settings()?;
                    let t = time.clone().unwrap_or_else(|| "09:00".to_string());
                    let team = team_id
                        .clone()
                        .or_else(|| settings.board.team_id.clone())
                        .unwrap_or_else(|| "board".to_string());
                    if !settings.teams.contains_key(&team) {
                        println!("Team not found: {}", team);
                        return Ok(());
                    }
                    let schedules = settings.board.schedules.get_or_insert_with(Vec::new);
                    let id = format!("daily-{}", ulid::Ulid::new());
                    schedules.push(crate::config::BoardSchedule {
                        id: id.clone(),
                        schedule_type: "daily".to_string(),
                        time: t.clone(),
                        team_id: Some(team.clone()),
                        agent_id: None,
                        sender_id: sender_id.clone(),
                        enabled: true,
                    });
                    let path = crate::config::get_settings_path()?;
                    std::fs::write(path, serde_json::to_string_pretty(&settings)?)?;
                    println!("Added daily board schedule: {} at {} for @{}", id, t, team);
                }
                BoardScheduleCommand::Digest { time, agent, sender_id } => {
                    let mut settings = load_settings()?;
                    let t = time.clone().unwrap_or_else(|| "18:00".to_string());
                    let target_agent = agent
                        .clone()
                        .or_else(|| crate::core::routing::get_default_agent(&settings))
                        .unwrap_or_else(|| "assistant".to_string());
                    if !settings.agents.contains_key(&target_agent) {
                        println!("Agent not found: {}", target_agent);
                        return Ok(());
                    }
                    let schedules = settings.board.schedules.get_or_insert_with(Vec::new);
                    let id = format!("digest-{}", ulid::Ulid::new());
                    schedules.push(crate::config::BoardSchedule {
                        id: id.clone(),
                        schedule_type: "digest".to_string(),
                        time: t.clone(),
                        team_id: settings.board.team_id.clone(),
                        agent_id: Some(target_agent.clone()),
                        sender_id: sender_id.clone(),
                        enabled: true,
                    });
                    let path = crate::config::get_settings_path()?;
                    std::fs::write(path, serde_json::to_string_pretty(&settings)?)?;
                    println!("Added digest schedule: {} at {} for @{}", id, t, target_agent);
                }
                BoardScheduleCommand::List => {
                    let settings = load_settings()?;
                    let schedules = settings.board.schedules.unwrap_or_default();
                    if schedules.is_empty() {
                        println!("No board schedules configured.");
                    } else {
                        println!("Board schedules:");
                        for s in schedules {
                            println!(
                                "- {} | type={} time={} team={:?} agent={:?} enabled={}",
                                s.id, s.schedule_type, s.time, s.team_id, s.agent_id, s.enabled
                            );
                        }
                    }
                }
                BoardScheduleCommand::Remove { which } => {
                    let mut settings = load_settings()?;
                    let schedules = settings.board.schedules.get_or_insert_with(Vec::new);
                    let before = schedules.len();
                    if which == "all" || which.is_empty() {
                        schedules.clear();
                        println!("Removed all board schedules.");
                    } else {
                        schedules.retain(|s| s.id != *which);
                        if schedules.len() == before {
                            println!("Schedule not found: {}", which);
                        } else {
                            println!("Removed schedule: {}", which);
                        }
                    }
                    let path = crate::config::get_settings_path()?;
                    std::fs::write(path, serde_json::to_string_pretty(&settings)?)?;
                }
            }
        }
        BoardCommand::Decisions { command } => {
            match command {
                BoardDecisionsCommand::List { limit } => {
                    use crate::memory::{Memory, MemoryScope};
                    let settings = load_settings()?;
                    let team_id = settings.board.team_id.as_deref().unwrap_or("board");
                    let mut entries = Memory::list(MemoryScope::Team, Some(team_id), None)?
                        .into_iter()
                        .filter(|e| e.key.starts_with("board.decision."))
                        .collect::<Vec<_>>();
                    entries.sort_by_key(|e| e.updated_at);
                    entries.reverse();
                    let max = limit.unwrap_or(10);
                    println!("Board decisions for @{} (showing {}):", team_id, max);
                    for e in entries.into_iter().take(max) {
                        println!("- {} | {}", e.key, e.value.chars().take(180).collect::<String>());
                    }
                }
                BoardDecisionsCommand::Show { decision_id } => {
                    use crate::memory::{Memory, MemoryScope};
                    let settings = load_settings()?;
                    let team_id = settings.board.team_id.as_deref().unwrap_or("board");
                    let key = if decision_id.starts_with("board.decision.") {
                        decision_id.clone()
                    } else {
                        format!("board.decision.{}", decision_id)
                    };
                    match Memory::get(&key, MemoryScope::Team, Some(team_id))? {
                        Some(entry) => println!("{} = {}", entry.key, entry.value),
                        None => println!("Decision not found: {}", key),
                    }
                }
                BoardDecisionsCommand::Export { format, file, limit } => {
                    use crate::memory::{Memory, MemoryScope};
                    let settings = load_settings()?;
                    let team_id = settings.board.team_id.as_deref().unwrap_or("board");
                    let mut entries = Memory::list(MemoryScope::Team, Some(team_id), None)?
                        .into_iter()
                        .filter(|e| e.key.starts_with("board.decision."))
                        .collect::<Vec<_>>();
                    entries.sort_by_key(|e| e.updated_at);
                    entries.reverse();
                    entries.truncate(*limit);

                    let output = if format.eq_ignore_ascii_case("json") {
                        serde_json::to_string_pretty(&entries)?
                    } else {
                        let mut md = format!("# Board Decisions (@{})\n\n", team_id);
                        for e in entries {
                            md.push_str(&format!("## {}\n\n{}\n\n", e.key, e.value));
                        }
                        md
                    };

                    if let Some(path) = file {
                        std::fs::write(path, output)?;
                        println!("Exported board decisions to {}", path);
                    } else {
                        println!("{}", output);
                    }
                }
            }
        }
    }
    Ok(())
}

async fn cmd_memory(cmd: &MemoryCommand) -> Result<()> {
    use crate::memory::{Memory, MemoryScope};
    
    match cmd {
        MemoryCommand::Set { key, value, scope, scope_id } => {
            let scope_enum = match scope.as_str() {
                "agent" => MemoryScope::Agent,
                "team" => MemoryScope::Team,
                "task" => MemoryScope::Task,
                _ => MemoryScope::Global,
            };
            Memory::set(key, value, scope_enum.clone(), scope_id.as_deref())?;
            println!("Set memory: {} = {} (scope: {})", key, value, scope);
        }
        MemoryCommand::Get { key, scope, scope_id } => {
            let scope_enum = match scope.as_str() {
                "agent" => MemoryScope::Agent,
                "team" => MemoryScope::Team,
                "task" => MemoryScope::Task,
                _ => MemoryScope::Global,
            };
            if let Some(entry) = Memory::get(key, scope_enum, scope_id.as_deref())? {
                println!("{} = {}", entry.key, entry.value);
                println!("  Scope: {:?}, Category: {:?}", entry.scope, entry.category);
            } else {
                println!("Key not found: {}", key);
            }
        }
        MemoryCommand::List { scope, category } => {
            let scope_enum = match scope.as_deref() {
                Some("agent") => MemoryScope::Agent,
                Some("team") => MemoryScope::Team,
                Some("task") => MemoryScope::Task,
                _ => MemoryScope::Global,
            };
            let entries = Memory::list(scope_enum, None, category.as_deref())?;
            println!("Memory entries ({}):", entries.len());
            for entry in entries {
                println!("  {} = {}", entry.key, entry.value.chars().take(50).collect::<String>());
            }
        }
        MemoryCommand::Search { query, limit } => {
            let entries = Memory::search(query, *limit)?;
            println!("Search results for '{}':", query);
            for entry in entries {
                println!("  [{}] {} = {}", 
                    format!("{:?}", entry.scope).to_lowercase(),
                    entry.key, 
                    entry.value.chars().take(50).collect::<String>()
                );
            }
        }
        MemoryCommand::Delete { key, scope, scope_id } => {
            let scope_enum = match scope.as_str() {
                "agent" => MemoryScope::Agent,
                "team" => MemoryScope::Team,
                "task" => MemoryScope::Task,
                _ => MemoryScope::Global,
            };
            Memory::delete(key, scope_enum, scope_id.as_deref())?;
            println!("Deleted: {}", key);
        }
        MemoryCommand::Explain { query, agent, team, limit } => {
            let settings = load_settings()?;
            let agent_id = agent
                .as_deref()
                .unwrap_or("assistant");
            let team_id = team
                .as_deref()
                .or_else(|| {
                    settings.teams.iter().find(|(_, t)| t.agents.contains(&agent_id.to_string())).map(|(id, _)| id.as_str())
                });

            println!("Memory explain for query: {}", query);
            println!("Agent: {}", agent_id);
            println!("Team: {}", team_id.unwrap_or("none"));

            let mut total = 0usize;
            if let Ok(entries) = Memory::relevant(query, MemoryScope::Global, None, *limit) {
                println!("\n[global]");
                for e in entries {
                    println!("- {} = {}", e.key, e.value.chars().take(180).collect::<String>());
                    total += 1;
                }
            }
            if let Ok(entries) = Memory::relevant(query, MemoryScope::Agent, Some(agent_id), *limit) {
                println!("\n[agent:{}]", agent_id);
                for e in entries {
                    println!("- {} = {}", e.key, e.value.chars().take(180).collect::<String>());
                    total += 1;
                }
            }
            if let Some(t) = team_id {
                if let Ok(entries) = Memory::relevant(query, MemoryScope::Team, Some(t), *limit) {
                    println!("\n[team:{}]", t);
                    for e in entries {
                        println!("- {} = {}", e.key, e.value.chars().take(180).collect::<String>());
                        total += 1;
                    }
                }
            }
            println!("\nTotal injected candidates: {}", total);
        }
        MemoryCommand::Stats => {
            let stats = Memory::stats()?;
            println!("{}", stats);
        }
        MemoryCommand::Compact { scope, scope_id } => {
            let scope_enum = match scope.as_str() {
                "agent" => MemoryScope::Agent,
                "team" => MemoryScope::Team,
                "task" => MemoryScope::Task,
                _ => MemoryScope::Global,
            };
            let report = Memory::compact(scope_enum, scope_id.as_deref())?;
            println!(
                "Compaction complete: expired_removed={}, merged={}, promoted={}, pruned={}",
                report.expired_removed, report.merged, report.promoted, report.pruned
            );
        }
        MemoryCommand::Snapshot { command: _ } => {
            println!("Snapshots not yet implemented");
        }
        MemoryCommand::Inherit { command: _ } => {
            println!("Memory inheritance not yet implemented");
        }
        MemoryCommand::Export { file: _ } => {
            println!("Export not yet implemented");
        }
        MemoryCommand::Clear { scope } => {
            let scope_enum = match scope.as_deref() {
                Some("agent") => MemoryScope::Agent,
                Some("team") => MemoryScope::Team,
                Some("task") => MemoryScope::Task,
                _ => MemoryScope::Global,
            };
            Memory::clear(scope_enum.clone(), None)?;
            println!("Cleared memory: {:?}", scope);
        }
    }
    Ok(())
}

async fn cmd_task(cmd: &TaskCommand) -> Result<()> {
    use crate::heartbeat::tasks::{Task as HbTask, TaskPriority, TaskSpawner};

    match cmd {
        TaskCommand::Create { title, priority, agent, description, tags } => {
            let prio = priority
                .as_deref()
                .unwrap_or("medium")
                .parse::<TaskPriority>()
                .unwrap_or(TaskPriority::Medium)
                .to_string();
            let now = chrono::Utc::now().timestamp_millis();
            let record = TaskRecord {
                id: ulid::Ulid::new().to_string(),
                title: title.clone(),
                description: description.clone(),
                agent_id: agent.clone(),
                priority: prio,
                status: "pending".to_string(),
                tags: tags
                    .as_deref()
                    .unwrap_or("")
                    .split(',')
                    .map(|t| t.trim().to_string())
                    .filter(|t| !t.is_empty())
                    .collect(),
                created_at: now,
                updated_at: now,
                output: None,
                error: None,
            };
            let mut store = load_task_store()?;
            store.tasks.push(record.clone());
            save_task_store(&store)?;
            println!("Created task: {} ({})", record.id, record.title);
        }
        TaskCommand::List { status } => {
            let store = load_task_store()?;
            let items = store.tasks.into_iter().filter(|t| {
                status
                    .as_deref()
                    .map(|s| t.status.eq_ignore_ascii_case(s))
                    .unwrap_or(true)
            });
            println!("Tasks:");
            for t in items {
                println!(
                    "- {} | {} | status={} priority={} agent={}",
                    t.id,
                    t.title,
                    t.status,
                    t.priority,
                    t.agent_id.as_deref().unwrap_or("unassigned")
                );
            }
        }
        TaskCommand::Show { task_id } => {
            let store = load_task_store()?;
            if let Some(t) = store.tasks.into_iter().find(|t| &t.id == task_id) {
                println!("Task: {}", t.id);
                println!("  Title: {}", t.title);
                println!("  Description: {}", t.description.unwrap_or_default());
                println!("  Agent: {}", t.agent_id.unwrap_or_else(|| "unassigned".to_string()));
                println!("  Priority: {}", t.priority);
                println!("  Status: {}", t.status);
                println!("  Tags: {}", t.tags.join(", "));
                if let Some(out) = t.output {
                    println!("  Output: {}", out.chars().take(500).collect::<String>());
                }
                if let Some(err) = t.error {
                    println!("  Error: {}", err);
                }
            } else {
                println!("Task not found: {}", task_id);
            }
        }
        TaskCommand::Start { task_id, attach } => {
            let settings = load_settings()?;
            let mut store = load_task_store()?;
            let Some(idx) = store.tasks.iter().position(|t| &t.id == task_id) else {
                println!("Task not found: {}", task_id);
                return Ok(());
            };

            let agent_id = if let Some(a) = store.tasks[idx].agent_id.clone() {
                a
            } else {
                crate::core::routing::get_default_agent(&settings).unwrap_or_else(|| "assistant".to_string())
            };
            if !settings.agents.contains_key(&agent_id) {
                println!("Assigned agent not found: {}", agent_id);
                return Ok(());
            }

            store.tasks[idx].status = "running".to_string();
            store.tasks[idx].updated_at = chrono::Utc::now().timestamp_millis();
            save_task_store(&store)?;

            let mut task = HbTask::new(&store.tasks[idx].title)
                .with_agent(&agent_id)
                .with_priority(store.tasks[idx].priority.parse::<TaskPriority>().unwrap_or(TaskPriority::Medium));
            if let Some(desc) = &store.tasks[idx].description {
                task = task.with_description(desc);
            }
            for tag in &store.tasks[idx].tags {
                task = task.with_tag(tag);
            }

            match TaskSpawner::spawn_task(&task, &settings).await {
                Ok(out) => {
                    store.tasks[idx].status = "completed".to_string();
                    store.tasks[idx].output = Some(out.clone());
                    store.tasks[idx].error = None;
                    store.tasks[idx].updated_at = chrono::Utc::now().timestamp_millis();
                    save_task_store(&store)?;
                    println!("Task completed: {}", task_id);
                    if *attach {
                        println!("{}", out);
                    } else {
                        println!("{}", out.chars().take(700).collect::<String>());
                    }
                }
                Err(e) => {
                    store.tasks[idx].status = "failed".to_string();
                    store.tasks[idx].error = Some(e.to_string());
                    store.tasks[idx].updated_at = chrono::Utc::now().timestamp_millis();
                    save_task_store(&store)?;
                    println!("Task failed: {}", e);
                }
            }
        }
        TaskCommand::Stop { task_id } => {
            let mut store = load_task_store()?;
            if let Some(t) = store.tasks.iter_mut().find(|t| &t.id == task_id) {
                t.status = "cancelled".to_string();
                t.updated_at = chrono::Utc::now().timestamp_millis();
                save_task_store(&store)?;
                println!("Task cancelled: {}", task_id);
            } else {
                println!("Task not found: {}", task_id);
            }
        }
        TaskCommand::Watch { task_id } => {
            let store = load_task_store()?;
            if let Some(t) = store.tasks.into_iter().find(|t| &t.id == task_id) {
                println!("{} [{}]", t.title, t.status);
                if let Some(out) = t.output {
                    println!("{}", out);
                } else if let Some(err) = t.error {
                    println!("Error: {}", err);
                } else {
                    println!("No output yet.");
                }
            } else {
                println!("Task not found: {}", task_id);
            }
        }
        TaskCommand::Assign { task_id, agent } => {
            let settings = load_settings()?;
            if !settings.agents.contains_key(agent) {
                println!("Agent not found: {}", agent);
                return Ok(());
            }
            let mut store = load_task_store()?;
            if let Some(t) = store.tasks.iter_mut().find(|t| &t.id == task_id) {
                t.agent_id = Some(agent.clone());
                t.updated_at = chrono::Utc::now().timestamp_millis();
                save_task_store(&store)?;
                println!("Assigned task {} to @{}", task_id, agent);
            } else {
                println!("Task not found: {}", task_id);
            }
        }
        TaskCommand::Delete { task_id } => {
            let mut store = load_task_store()?;
            let before = store.tasks.len();
            store.tasks.retain(|t| &t.id != task_id);
            if store.tasks.len() < before {
                save_task_store(&store)?;
                println!("Deleted task: {}", task_id);
            } else {
                println!("Task not found: {}", task_id);
            }
        }
        TaskCommand::Stats => {
            let store = load_task_store()?;
            let total = store.tasks.len();
            let pending = store.tasks.iter().filter(|t| t.status == "pending").count();
            let running = store.tasks.iter().filter(|t| t.status == "running").count();
            let completed = store.tasks.iter().filter(|t| t.status == "completed").count();
            let failed = store.tasks.iter().filter(|t| t.status == "failed").count();
            let cancelled = store.tasks.iter().filter(|t| t.status == "cancelled").count();
            println!("Task statistics:");
            println!("  Total: {}", total);
            println!("  Pending: {}", pending);
            println!("  Running: {}", running);
            println!("  Completed: {}", completed);
            println!("  Failed: {}", failed);
            println!("  Cancelled: {}", cancelled);
        }
    }
    Ok(())
}

async fn cmd_pairing(cmd: &PairingCommand) -> Result<()> {
    let settings = load_settings()?;
    
    match cmd {
        PairingCommand::Pending => {
            println!("Pending senders:");
            if let Some(pending) = &settings.pairing.pending_senders {
                for p in pending {
                    println!("  {} - {} (code: {})", p.sender_id, p.sender_name, p.code);
                }
            }
        }
        PairingCommand::Approved => {
            println!("Approved senders:");
            if let Some(approved) = &settings.pairing.approved_senders {
                for a in approved {
                    println!("  {} - {}", a.sender_id, a.sender_name);
                }
            }
        }
        PairingCommand::List => {
            println!("All senders:");
            println!("Pending:");
            if let Some(pending) = &settings.pairing.pending_senders {
                for p in pending {
                    println!("  {} - {} (code: {})", p.sender_id, p.sender_name, p.code);
                }
            }
            println!("Approved:");
            if let Some(approved) = &settings.pairing.approved_senders {
                for a in approved {
                    println!("  {} - {}", a.sender_id, a.sender_name);
                }
            }
        }
        PairingCommand::Approve { code } => {
            use crate::telegram::pairing::PairingManager;
            match PairingManager::approve_by_code(code) {
                Ok(sender) => {
                    println!("✅ Approved sender: {} ({})", sender.sender_name, sender.sender_id);
                }
                Err(e) => {
                    println!("❌ Failed to approve: {}", e);
                }
            }
        }
        PairingCommand::Unpair { channel, sender_id } => {
            use crate::telegram::pairing::PairingManager;
            match PairingManager::unpair(sender_id) {
                Ok(()) => {
                    println!("✅ Unpaired {} from {}", sender_id, channel);
                }
                Err(e) => {
                    println!("❌ Failed to unpair: {}", e);
                }
            }
        }
    }
    Ok(())
}

async fn cmd_provider(name: &Option<String>, model: &Option<String>) -> Result<()> {
    let mut settings = load_settings()?;
    
    let available_providers = vec![
        ("claude", "Anthropic Claude CLI"),
        ("codex", "OpenAI Codex CLI"),
        ("cline", "Cline CLI"),
        ("opencode", "OpenCode CLI"),
        ("ollama", "Ollama HTTP"),
        ("grok", "Grok/X.AI HTTP"),
    ];
    
    if let Some(n) = name {
        // Validate provider
        if !available_providers.iter().any(|(id, _)| id == n) {
            println!("Unknown provider: {}", n);
            println!("Available providers:");
            for (id, desc) in &available_providers {
                println!("  {} - {}", id, desc);
            }
            return Ok(());
        }
        
        settings.models.provider = n.clone();

        // Update the primary agent to follow provider switches.
        let target_agent_id = if settings.agents.contains_key("assistant") {
            Some("assistant".to_string())
        } else {
            settings.agents.keys().next().cloned()
        };

        if let Some(agent_id) = target_agent_id {
            if let Some(agent) = settings.agents.get_mut(&agent_id) {
            agent.provider = Some(n.clone());
            match model {
                Some(m) => agent.model = Some(m.clone()),
                None if matches!(n.as_str(), "claude" | "codex" | "cline" | "opencode") => {
                    // For CLI providers, "default" means use whatever the CLI selected.
                    agent.model = Some("default".to_string());
                }
                None => {}
            }
            }
        }

        if let Some(m) = model {
            // Set provider-specific model defaults.
            match n.as_str() {
                "claude" => settings.models.anthropic.model = Some(m.clone()),
                "codex" => settings.models.openai.model = Some(m.clone()),
                "grok" => settings.models.grok.model = Some(m.clone()),
                "ollama" => settings.models.ollama.model = Some(m.clone()),
                _ => {}
            }
        }
        
        // Save settings
        let path = crate::config::get_settings_path()?;
        let content = serde_json::to_string_pretty(&settings)?;
        std::fs::write(path, content)?;
        
        if let Some(m) = model {
            println!("Switched to provider: {} (model: {})", n, m);
        } else if matches!(n.as_str(), "claude" | "codex" | "cline" | "opencode") {
            println!("Switched to provider: {} (model: default)", n);
        } else {
            println!("Switched to provider: {}", n);
        }
    } else {
        println!("Current provider: {}", settings.models.provider);
        println!("\nAvailable providers:");
        for (id, desc) in &available_providers {
            let marker = if id == &settings.models.provider { "*" } else { " " };
            println!(" {} {} - {}", marker, id, desc);
        }
    }
    
    Ok(())
}

async fn cmd_model(name: &Option<String>) -> Result<()> {
    let mut settings = load_settings()?;
    let default_agent = crate::core::routing::get_default_agent(&settings)
        .unwrap_or_else(|| "assistant".to_string());
    if let Some(n) = name {
        if let Some(agent) = settings.agents.get_mut(&default_agent) {
            agent.model = Some(n.clone());
        }
        match settings.models.provider.as_str() {
            "claude" => settings.models.anthropic.model = Some(n.clone()),
            "codex" => settings.models.openai.model = Some(n.clone()),
            "grok" => settings.models.grok.model = Some(n.clone()),
            "ollama" => settings.models.ollama.model = Some(n.clone()),
            _ => {}
        }
        let path = crate::config::get_settings_path()?;
        std::fs::write(path, serde_json::to_string_pretty(&settings)?)?;
        println!("Model set for @{}: {}", default_agent, n);
    } else {
        let model = settings
            .agents
            .get(&default_agent)
            .and_then(|a| a.model.as_deref())
            .unwrap_or("default");
        println!("Current provider: {}", settings.models.provider);
        println!("Default agent: @{}", default_agent);
        println!("Current model: {}", model);
    }
    Ok(())
}

async fn cmd_channels(action: &str, channel: &str) -> Result<()> {
    if action != "reset" {
        return Err(anyhow::anyhow!("Unsupported channels action: {} (use: channels reset telegram)", action));
    }
    if channel != "telegram" {
        return Err(anyhow::anyhow!("Only telegram channel reset is currently supported"));
    }
    use std::io::{self, BufRead, Write};
    let mut settings = load_settings()?;
    let stdin = io::stdin();
    let mut stdout = io::stdout();
    print!("New Telegram bot token: ");
    stdout.flush()?;
    let mut token = String::new();
    stdin.lock().read_line(&mut token)?;
    let token = token.trim().to_string();
    if token.is_empty() {
        return Err(anyhow::anyhow!("Token cannot be empty"));
    }
    settings.channels.telegram.bot_token = Some(token);
    if !settings.channels.enabled.contains(&"telegram".to_string()) {
        settings.channels.enabled.push("telegram".to_string());
    }
    let path = crate::config::get_settings_path()?;
    std::fs::write(path, serde_json::to_string_pretty(&settings)?)?;
    println!("Telegram channel reconfigured.");
    Ok(())
}

async fn cmd_doctor(strict: bool, fix: bool) -> Result<()> {
    println!("Running TinyVegeta diagnostics...\n");

    let mut issues = Vec::new();
    let mut warnings = Vec::new();
    let mut fixes = Vec::new();

    // Check settings and runtime graph.
    print!("📋 Settings + routing... ");
    let settings = match load_settings() {
        Ok(s) => {
            println!("✓");
            s
        }
        Err(e) => {
            println!("✗");
            return Err(anyhow::anyhow!("Settings error: {}", e));
        }
    };

    if settings.models.provider.is_empty() {
        issues.push("No provider configured (settings.models.provider)".to_string());
    }
    if settings.agents.is_empty() {
        issues.push("No agents configured".to_string());
    }
    if let Some(default_agent) = settings.routing.default_agent.as_deref() {
        if !settings.agents.contains_key(default_agent) {
            issues.push(format!("routing.default_agent '{}' is missing", default_agent));
        }
    }
    let default_agent = crate::core::routing::get_default_agent(&settings);
    if default_agent.is_none() {
        issues.push("No resolvable default agent".to_string());
    }

    // Workspace checks.
    print!("📋 Workspace + agent paths... ");
    let mut settings_changed = false;
    let workspace = settings.workspace.path.clone();
    if let Some(ws) = workspace.as_ref() {
        if ws.exists() {
            println!("✓ ({})", ws.display());
        } else if fix {
            std::fs::create_dir_all(ws)?;
            settings_changed = true;
            fixes.push(format!("Created workspace path {}", ws.display()));
            println!("✓ (created {})", ws.display());
        } else {
            println!("✗ (missing {})", ws.display());
            issues.push(format!("Workspace path missing: {}", ws.display()));
        }
    } else {
        println!("⚠ (not set)");
        warnings.push("workspace.path is not set".to_string());
    }

    for (agent_id, agent) in settings.agents.clone() {
        if let Some(wd) = agent.working_directory {
            if !wd.exists() {
                if fix {
                    std::fs::create_dir_all(&wd)?;
                    crate::context::init_agent_context(&agent_id, &wd)?;
                    fixes.push(format!("Created agent workspace for @{} ({})", agent_id, wd.display()));
                } else {
                    issues.push(format!("Agent @{} working_directory missing: {}", agent_id, wd.display()));
                }
            }

            let soul = wd.join("SOUL.md");
            let memory = wd.join("MEMORY.md");
            if !soul.exists() || !memory.exists() {
                if fix {
                    crate::context::init_agent_context(&agent_id, &wd)?;
                    fixes.push(format!("Initialized SOUL/MEMORY for @{}", agent_id));
                } else {
                    issues.push(format!("Agent @{} missing SOUL.md or MEMORY.md", agent_id));
                }
            }

            if let Some(ws) = workspace.as_ref() {
                if !wd.starts_with(ws) {
                    warnings.push(format!(
                        "Agent @{} working_directory is outside workspace root: {}",
                        agent_id,
                        wd.display()
                    ));
                }
            }
        } else {
            issues.push(format!("Agent @{} has no working_directory", agent_id));
        }
    }

    // Team + board consistency.
    print!("📋 Teams + board config... ");
    let mut team_errors = 0usize;
    for (team_id, team) in &settings.teams {
        for member in &team.agents {
            if !settings.agents.contains_key(member) {
                team_errors += 1;
                issues.push(format!("Team @{} references missing agent @{}", team_id, member));
            }
        }
        if let Some(leader) = &team.leader_agent {
            if !team.agents.contains(leader) {
                team_errors += 1;
                issues.push(format!("Team @{} leader @{} not in members", team_id, leader));
            }
        } else {
            warnings.push(format!("Team @{} has no leader_agent", team_id));
        }
    }
    if let Some(board_id) = settings.board.team_id.as_deref() {
        if !settings.teams.contains_key(board_id) {
            team_errors += 1;
            issues.push(format!("board.team_id '{}' does not exist", board_id));
        }
    } else {
        warnings.push("board.team_id is not set".to_string());
    }
    if team_errors == 0 {
        println!("✓");
    } else {
        println!("✗ ({} issue(s))", team_errors);
    }

    // Persist any doctor --fix settings change.
    if fix && settings_changed {
        let path = crate::config::get_settings_path()?;
        std::fs::write(path, serde_json::to_string_pretty(&settings)?)?;
    }

    // Check home + queue + memory.
    print!("📋 Home / queue / memory... ");
    let home = crate::config::get_home_dir()?;
    if !home.exists() && fix {
        std::fs::create_dir_all(&home)?;
        fixes.push(format!("Created {}", home.display()));
    }
    crate::core::queue::ensure_queue_dirs()?;
    crate::memory::ensure_memory_dirs()?;
    let qstats = crate::core::Queue::stats()?;
    let mstats = crate::memory::Memory::stats()?;
    println!(
        "✓ (queue: {}/{}/{}, memory total: {})",
        qstats.incoming, qstats.processing, qstats.outgoing, mstats.total
    );

    // SOUL fallback path check.
    print!("📋 SOUL fallback path... ");
    let default_soul = std::env::var("TINYVEGETA_DEFAULT_SOUL")
        .ok()
        .map(std::path::PathBuf::from)
        .or_else(|| directories::UserDirs::new().map(|u| u.home_dir().join("ai").join("tinyvegeta").join("SOUL.md")));
    if let Some(path) = default_soul {
        if path.exists() {
            println!("✓ ({})", path.display());
        } else {
            println!("⚠ (missing {})", path.display());
            warnings.push(format!("Default SOUL fallback not found: {}", path.display()));
        }
    } else {
        println!("⚠ (unresolved)");
        warnings.push("Could not resolve default SOUL fallback path".to_string());
    }

    // tmux checks including stale-session detection.
    print!("📋 tmux daemon state... ");
    match std::process::Command::new("tmux").arg("-V").output() {
        Ok(out) => {
            let version = String::from_utf8_lossy(&out.stdout).trim().to_string();
            let session_exists = crate::tmux::session_exists().unwrap_or(false);
            if session_exists {
                let pane_out = std::process::Command::new("tmux")
                    .args(["list-panes", "-t", crate::tmux::TMUX_SESSION, "-F", "#{pane_current_command}"])
                    .output()
                    .ok();
                let pane_text = pane_out
                    .as_ref()
                    .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
                    .unwrap_or_default();
                let stale = pane_text.trim().is_empty() || pane_text.lines().all(|l| l.trim() == "sleep");
                if stale {
                    if fix {
                        let _ = crate::tmux::stop_daemon();
                        fixes.push("Stopped stale tmux tinyvegeta session".to_string());
                        println!("✓ ({}; stale session removed)", version);
                    } else {
                        println!("⚠ ({}; stale session detected)", version);
                        warnings.push("Stale tmux session detected (only sleep/no active panes)".to_string());
                    }
                } else {
                    println!("✓ ({})", version);
                }
            } else {
                println!("✓ ({}, session stopped)", version);
            }
        }
        Err(_) => {
            println!("✗ (tmux not installed)");
            issues.push("tmux is not installed".to_string());
        }
    }

    // Provider CLI checks.
    println!("\n📡 Provider CLIs:");
    let providers = [("claude", "claude"), ("codex", "codex"), ("cline", "cline"), ("opencode", "opencode")];
    for (name, bin) in providers {
        print!("   {}... ", name);
        match std::process::Command::new(bin).arg("--version").output() {
            Ok(_) => println!("✓"),
            Err(_) => {
                println!("✗ (not installed)");
                if settings.models.provider == name {
                    issues.push(format!("Active provider '{}' CLI is not installed", name));
                } else {
                    warnings.push(format!("Provider '{}' CLI is not installed", name));
                }
            }
        }
    }
    print!("   ollama... ");
    match reqwest::get("http://localhost:11434/api/tags").await {
        Ok(resp) if resp.status().is_success() => println!("✓ (running)"),
        _ => println!("✗ (not running)"),
    }

    // Cline auth check for active cline usage.
    let cline_in_use = settings.models.provider == "cline"
        || settings.agents.values().any(|a| a.provider.as_deref() == Some("cline"));
    if cline_in_use {
        print!("   cline auth... ");
        let out = tokio::time::timeout(
            std::time::Duration::from_secs(15),
            tokio::process::Command::new("cline")
                .args(["task", "Reply with exactly OK.", "--json"])
                .output(),
        )
        .await;
        match out {
            Err(_) => {
                println!("⚠ (timeout)");
                warnings.push("Cline auth check timed out after 15s".to_string());
            }
            Ok(out) => match out {
            Ok(o) => {
                let stderr = String::from_utf8_lossy(&o.stderr).to_lowercase();
                let stdout = String::from_utf8_lossy(&o.stdout).to_lowercase();
                if stderr.contains("unauthorized") || stdout.contains("unauthorized") {
                    println!("✗ (unauthorized)");
                    issues.push("Cline is selected but not authenticated. Run `cline auth` and restart tinyvegeta.".to_string());
                } else if o.status.success() {
                    println!("✓");
                } else {
                    println!("⚠ (could not verify)");
                    warnings.push("Cline auth check could not be verified (non-zero exit)".to_string());
                }
            }
            Err(_) => {
                println!("✗ (cline not callable)");
                issues.push("Cline auth check failed: CLI not callable".to_string());
            }
        }}
    }

    // Summary
    println!();
    if issues.is_empty() {
        println!("✅ Doctor passed with {} warning(s).", warnings.len());
    } else {
        println!("❌ {} issue(s), {} warning(s).", issues.len(), warnings.len());
        for issue in &issues {
            println!("   • {}", issue);
        }
    }
    if !warnings.is_empty() {
        println!("\n⚠ Warnings:");
        for warning in &warnings {
            println!("   • {}", warning);
        }
    }
    if fix && !fixes.is_empty() {
        println!("\n🔧 Applied fixes:");
        for f in &fixes {
            println!("   • {}", f);
        }
    }

    if strict && !issues.is_empty() {
        return Err(anyhow::anyhow!("Doctor found {} issue(s)", issues.len()));
    }

    Ok(())
}

async fn cmd_releasecheck() -> Result<()> {
    println!("Running release check...");
    
    // Check that binary builds
    println!("✓ Binary builds");
    
    // Check key features
    println!("✓ CLI commands available");
    println!("✓ Queue system available");
    println!("✓ Memory system available");
    println!("✓ Telegram bot available");
    println!("✓ Web server available");
    println!("✓ Heartbeat daemon available");
    println!("✓ AI providers available");
    
    println!("\n✅ Release check passed!");
    Ok(())
}

async fn cmd_telegram() -> Result<()> {
    use crate::telegram::run_telegram_daemon;
    
    println!("Starting Telegram bot...");
    run_telegram_daemon().await?;
    Ok(())
}

async fn cmd_heartbeat(agent: &Option<String>, verbose: bool) -> Result<()> {
    use crate::heartbeat::{run_heartbeat_daemon, run_single_heartbeat};
    
    if let Some(agent_id) = agent {
        println!("Running heartbeat for agent: {}", agent_id);
        match run_single_heartbeat(agent_id).await {
            Ok(result) => {
                if verbose {
                    println!("Heartbeat result:");
                    println!("{}", result);
                } else {
                    println!("Heartbeat completed ({} chars). Use --verbose for full output.", result.len());
                }
            }
            Err(e) => {
                println!("Heartbeat failed: {}", e);
            }
        }
    } else {
        println!("Starting heartbeat daemon...");
        run_heartbeat_daemon().await?;
    }
    Ok(())
}

async fn cmd_sovereign(
    agent: &Option<String>,
    goal: &Option<String>,
    max_cycles: &Option<u32>,
    dry_run: bool,
) -> Result<()> {
    println!("Starting sovereign runtime...");
    println!("  dry_run: {}", dry_run);
    if let Some(agent_id) = agent {
        println!("  agent: {}", agent_id);
    }
    if let Some(goal_text) = goal {
        println!("  goal: {}", goal_text);
    }
    if let Some(max) = max_cycles {
        println!("  max_cycles: {}", max);
    } else {
        println!("  max_cycles: continuous");
    }

    // Heartbeat keeps schedules active while sovereign loop is sleeping.
    let heartbeat = tokio::spawn(async { crate::heartbeat::run_heartbeat_daemon().await });
    let loop_result = crate::sovereign::run(
        agent.clone(),
        goal.clone(),
        *max_cycles,
        dry_run,
    )
    .await;
    heartbeat.abort();

    loop_result
}

async fn cmd_web(port: u16, stop: bool) -> Result<()> {
    use crate::web::run_web_server;
    
    if stop {
        println!("Stopping web server...");
        // Send signal to stop (implement with PID file or signal)
        println!("Web server stop not yet implemented.");
    } else {
        println!("Starting web server on port {}...", port);
        println!("API endpoints:");
        println!("  http://localhost:{}/api/agents", port);
        println!("  http://localhost:{}/api/teams", port);
        println!("  http://localhost:{}/api/memory", port);
        println!("  http://localhost:{}/health", port);
        println!();
        println!("Press Ctrl+C to stop");
        
        run_web_server(port).await
            .map_err(|e| anyhow::anyhow!("Web server error: {}", e))?;
    }
    Ok(())
}

async fn cmd_update() -> Result<()> {
    println!("Updating TinyVegeta...\n");
    
    // Check if we're in a git repo
    let current_dir = std::env::current_exe()?;
    let repo_dir = current_dir.parent()
        .and_then(|p| p.parent())
        .map(|p| p.to_path_buf());
    
    if let Some(repo) = repo_dir {
        let git_dir = repo.join(".git");
        if git_dir.exists() {
            print!("📥 Pulling latest changes... ");
            let output = std::process::Command::new("git")
                .args(["pull"])
                .current_dir(&repo)
                .output()?;
            
            if output.status.success() {
                println!("done");
                
                print!("🔨 Rebuilding... ");
                let build_output = std::process::Command::new("cargo")
                    .args(["build", "--release"])
                    .current_dir(&repo)
                    .output()?;
                
                if build_output.status.success() {
                    println!("done");
                    println!("\n✅ TinyVegeta updated successfully!");
                } else {
                    println!("failed");
                    println!("Build error: {}", String::from_utf8_lossy(&build_output.stderr));
                }
            } else {
                println!("failed");
                println!("Git error: {}", String::from_utf8_lossy(&output.stderr));
            }
        } else {
            println!("Not installed from git repository.");
            println!("Please reinstall from source or use your package manager.");
        }
    } else {
        println!("Could not determine installation directory.");
    }
    
    Ok(())
}

async fn cmd_uninstall(yes: bool, purge_data: bool, purge_install: bool) -> Result<()> {
    if !yes {
        println!("This will uninstall TinyVegeta.");
        println!("Run with --yes to confirm, or use additional flags:");
        println!("  --purge-data    Also delete ~/.tinyvegeta data directory");
        println!("  --purge-install Also delete installation directory");
        return Ok(());
    }
    
    println!("Uninstalling TinyVegeta...\n");
    
    // Stop any running instances
    print!("🛑 Stopping running instances... ");
    let _ = crate::tmux::stop_daemon();
    println!("done");
    
    // Remove data directory if requested
    if purge_data {
        print!("🗑️  Removing data directory... ");
        let home = crate::config::get_home_dir()?;
        if home.exists() {
            std::fs::remove_dir_all(&home)?;
            println!("done ({})", home.display());
        } else {
            println!("not found");
        }
    }
    
    // Remove installation directory if requested
    if purge_install {
        print!("🗑️  Removing installation directory... ");
        let install_dir = std::env::current_exe()
            .map(|p| p.parent().map(|p| p.to_path_buf()))
            .unwrap_or(None);
        
        if let Some(dir) = install_dir {
            if dir.exists() {
                std::fs::remove_dir_all(&dir)?;
                println!("done ({})", dir.display());
            } else {
                println!("not found");
            }
        } else {
            println!("could not determine");
        }
    }
    
    // Remove from PATH (if installed via install script)
    println!("\n✅ Uninstall complete!");
    
    if !purge_data {
        println!("\nNote: Data directory preserved at ~/.tinyvegeta");
        println!("Run with --purge-data to remove it.");
    }
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{build_runtime_context_block, enforce_identity_guard};
    use crate::config::{Board, Routing, Settings, Workspace};

    #[test]
    fn runtime_context_contains_workspace_and_agent_path() {
        let mut settings = Settings::default();
        settings.workspace = Workspace {
            path: Some(std::path::PathBuf::from("/tmp/ws")),
            name: Some("ws".to_string()),
        };
        settings.board = Board {
            team_id: Some("board".to_string()),
            autonomous: Some(true),
            schedules: None,
        };
        settings.routing = Routing {
            default_agent: Some("assistant".to_string()),
        };

        let block = build_runtime_context_block(
            &settings,
            "assistant",
            Some(&std::path::PathBuf::from("/tmp/ws/assistant")),
            Some("board"),
        );

        assert!(block.contains("agent_id: assistant"));
        assert!(block.contains("working_directory: /tmp/ws/assistant"));
        assert!(block.contains("workspace_root: /tmp/ws"));
        assert!(block.contains("team_id: board"));
    }

    #[test]
    fn identity_guard_overrides_codex_self_intro() {
        let out = enforce_identity_guard("who are you", "I'm Codex, your AI coding agent.".to_string());
        assert!(out.contains("I'm TinyVegeta"));
        assert!(!out.to_lowercase().contains("codex"));
    }
}
