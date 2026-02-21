//! Telegram bot client - simple polling version.

use std::collections::HashMap;
use std::sync::OnceLock;

use teloxide::prelude::*;
use teloxide::RequestError;
use tokio::process::Command as TokioCommand;
use tokio::sync::Mutex;

use crate::config::load_settings;
use crate::error::Error;

use super::pairing::PairingManager;

#[derive(Clone)]
struct SoulTarget {
    agent_id: String,
    agent_name: String,
    soul_path: std::path::PathBuf,
}

fn pending_soul_writes() -> &'static Mutex<HashMap<String, SoulTarget>> {
    static PENDING: OnceLock<Mutex<HashMap<String, SoulTarget>>> = OnceLock::new();
    PENDING.get_or_init(|| Mutex::new(HashMap::new()))
}

fn sanitize_file_name(name: &str) -> String {
    let mut out = String::new();
    for ch in name.chars() {
        if ch.is_ascii_alphanumeric() || matches!(ch, '.' | '_' | '-') {
            out.push(ch);
        } else {
            out.push('_');
        }
    }
    if out.is_empty() {
        "file.bin".to_string()
    } else {
        out
    }
}

async fn download_telegram_file(
    file_id: &str,
    fallback_ext: &str,
    original_name: Option<&str>,
) -> std::result::Result<Option<String>, String> {
    let settings = load_settings().map_err(|e| e.to_string())?;
    let token = settings
        .channels
        .telegram
        .bot_token
        .ok_or_else(|| "No telegram token configured".to_string())?;

    let get_file_url = format!(
        "https://api.telegram.org/bot{}/getFile?file_id={}",
        token, file_id
    );
    let resp = reqwest::get(get_file_url).await.map_err(|e| e.to_string())?;
    let value: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
    let file_path = value
        .get("result")
        .and_then(|r| r.get("file_path"))
        .and_then(|p| p.as_str())
        .ok_or_else(|| "Telegram getFile returned no file_path".to_string())?;

    let download_url = format!("https://api.telegram.org/file/bot{}/{}", token, file_path);
    let bytes = reqwest::get(download_url)
        .await
        .map_err(|e| e.to_string())?
        .bytes()
        .await
        .map_err(|e| e.to_string())?;

    let home = crate::config::get_home_dir().map_err(|e| e.to_string())?;
    let files_dir = home.join("files");
    std::fs::create_dir_all(&files_dir).map_err(|e| e.to_string())?;

    let base = if let Some(name) = original_name {
        sanitize_file_name(name)
    } else {
        let suffix = if fallback_ext.starts_with('.') {
            fallback_ext.to_string()
        } else {
            format!(".{}", fallback_ext)
        };
        format!("telegram_{}{}", ulid::Ulid::new(), suffix)
    };
    let mut filename = base.clone();
    if std::path::Path::new(&filename).extension().is_none() && !fallback_ext.is_empty() {
        let ext_owned = if fallback_ext.starts_with('.') {
            fallback_ext.to_string()
        } else {
            format!(".{}", fallback_ext)
        };
        filename.push_str(&ext_owned);
    }

    let path = files_dir.join(filename);
    std::fs::write(&path, bytes).map_err(|e| e.to_string())?;
    Ok(Some(path.display().to_string()))
}

/// Run the telegram bot daemon using simple polling.
pub async fn run_telegram_daemon() -> Result<(), Error> {
    tracing::info!("Starting Telegram bot...");
    
    let settings = load_settings()?;
    
    let token = settings.channels.telegram.bot_token
        .ok_or_else(|| Error::Telegram("No bot token configured".to_string()))?;
    
    let bot = Bot::new(token);
    
    // Set up commands
    if let Err(e) = bot.set_my_commands(vec![
        teloxide::types::BotCommand::new("help", "Show help"),
        teloxide::types::BotCommand::new("agent", "List agents"),
        teloxide::types::BotCommand::new("team", "List teams"),
        teloxide::types::BotCommand::new("board", "Show board info"),
        teloxide::types::BotCommand::new("status", "Show daemon status"),
        teloxide::types::BotCommand::new("restart", "Restart TinyVegeta daemon"),
        teloxide::types::BotCommand::new("doctor", "Run remote health checks"),
        teloxide::types::BotCommand::new("provider", "Show or set provider"),
        teloxide::types::BotCommand::new("models", "Alias for provider switch"),
        teloxide::types::BotCommand::new("memory", "Quick memory ops"),
        teloxide::types::BotCommand::new("brain", "BRAIN.md quick ops"),
        teloxide::types::BotCommand::new("logs", "Tail filtered logs"),
        teloxide::types::BotCommand::new("gateway", "Gateway status/restart"),
        teloxide::types::BotCommand::new("releasecheck", "Run release checks"),
        teloxide::types::BotCommand::new("sovereign", "Control sovereign runtime"),
        teloxide::types::BotCommand::new("soul", "Edit/show SOUL.md"),
        teloxide::types::BotCommand::new("reset", "Reset conversation"),
        teloxide::types::BotCommand::new("triage", "Toggle auto-triage"),
    ]).await {
        tracing::warn!("Failed to set commands: {}", e);
    }
    
    tracing::info!("Telegram bot commands set");
    
    // Use dispatch with a simple handler
    teloxide::repl(bot, |bot, msg| async move {
        handle_message(bot, msg).await
    }).await;
    
    Ok(())
}

/// Handle incoming messages.
async fn handle_message(bot: Bot, msg: Message) -> Result<(), RequestError> {
    // Check if it's a command
    if let Some(text) = msg.text() {
        if text.starts_with('/') {
            let chat_id = msg.chat.id;
            let mut parts = text.split_whitespace();
            let cmd = parts.next().unwrap_or("");

            match cmd {
                "/help" => {
                    bot.send_message(chat_id, HELP_TEXT).await?;
                }
                "/agent" => {
                    cmd_agents(bot, chat_id).await?;
                }
                "/team" => {
                    cmd_teams(bot, chat_id).await?;
                }
                "/board" => {
                    let sub = parts.next();
                    if sub == Some("discuss") {
                        if !ensure_approved_sender(&bot, &msg).await? {
                            return Ok(());
                        }
                        let topic = parts.collect::<Vec<_>>().join(" ");
                        if topic.trim().is_empty() {
                            bot.send_message(chat_id, "Usage: /board discuss <topic>").await?;
                        } else {
                            cmd_board_discuss(bot, chat_id, &topic).await?;
                        }
                    } else {
                        cmd_board(bot, chat_id).await?;
                    }
                }
                "/discuss" => {
                    if !ensure_approved_sender(&bot, &msg).await? {
                        return Ok(());
                    }
                    let topic = parts.collect::<Vec<_>>().join(" ");
                    if topic.trim().is_empty() {
                        bot.send_message(chat_id, "Usage: /discuss <topic>").await?;
                    } else {
                        cmd_board_discuss(bot, chat_id, &topic).await?;
                    }
                }
                "/status" => {
                    cmd_status(bot, chat_id).await?;
                }
                "/restart" => {
                    if !ensure_approved_sender(&bot, &msg).await? {
                        return Ok(());
                    }
                    cmd_restart(bot, msg).await?;
                }
                "/doctor" => {
                    if !ensure_approved_sender(&bot, &msg).await? {
                        return Ok(());
                    }
                    cmd_doctor(bot, chat_id).await?;
                }
                "/provider" => {
                    if !ensure_approved_sender(&bot, &msg).await? {
                        return Ok(());
                    }
                    let provider = parts.next();
                    cmd_provider(bot, chat_id, provider).await?;
                }
                "/models" => {
                    if !ensure_approved_sender(&bot, &msg).await? {
                        return Ok(());
                    }
                    let provider = parts.next();
                    cmd_provider(bot, chat_id, provider).await?;
                }
                "/memory" => {
                    if !ensure_approved_sender(&bot, &msg).await? {
                        return Ok(());
                    }
                    let sub = parts.next();
                    let args = parts.collect::<Vec<_>>();
                    cmd_memory(bot, chat_id, sub, &args).await?;
                }
                "/brain" => {
                    if !ensure_approved_sender(&bot, &msg).await? {
                        return Ok(());
                    }
                    let sub = parts.next();
                    let args = parts.collect::<Vec<_>>();
                    cmd_brain(bot, chat_id, sub, &args).await?;
                }
                "/logs" => {
                    if !ensure_approved_sender(&bot, &msg).await? {
                        return Ok(());
                    }
                    let log_type = parts.next().unwrap_or("all");
                    let lines = parts.next().and_then(|n| n.parse::<usize>().ok()).unwrap_or(80);
                    cmd_logs(bot, chat_id, log_type, lines).await?;
                }
                "/gateway" => {
                    if !ensure_approved_sender(&bot, &msg).await? {
                        return Ok(());
                    }
                    match parts.next() {
                        None | Some("status") => cmd_status(bot, chat_id).await?,
                        Some("restart") => cmd_restart(bot, msg).await?,
                        _ => {
                            bot.send_message(chat_id, "Usage: /gateway [status|restart]").await?;
                        }
                    }
                }
                "/releasecheck" => {
                    if !ensure_approved_sender(&bot, &msg).await? {
                        return Ok(());
                    }
                    cmd_releasecheck(bot, chat_id).await?;
                }
                "/sovereign" => {
                    if !ensure_approved_sender(&bot, &msg).await? {
                        return Ok(());
                    }
                    let args = parts.collect::<Vec<_>>();
                    cmd_sovereign(bot, chat_id, &args).await?;
                }
                "/soul" => {
                    if !ensure_approved_sender(&bot, &msg).await? {
                        return Ok(());
                    }
                    let args = parts.collect::<Vec<_>>();
                    cmd_soul(bot, &msg, &args).await?;
                }
                "/reset" => {
                    if !ensure_approved_sender(&bot, &msg).await? {
                        return Ok(());
                    }
                    let agents = parts
                        .map(|a| a.trim_start_matches('@').to_lowercase())
                        .filter(|a| !a.is_empty())
                        .collect::<Vec<_>>();
                    if agents.is_empty() {
                        bot.send_message(chat_id, "Usage: /reset @agent_id [@agent_id2 ...]").await?;
                    } else {
                        cmd_reset_agents(bot, chat_id, &agents).await?;
                    }
                }
                "/triage" => {
                    if !ensure_approved_sender(&bot, &msg).await? {
                        return Ok(());
                    }
                    let arg = parts.next().unwrap_or("status");
                    cmd_triage(bot, chat_id, arg).await?;
                }
                _ => {
                    bot.send_message(chat_id, "Unknown command. /help for available commands.").await?;
                }
            }
            return Ok(());
        }
    }
    
    // Handle regular messages
    handle_regular_message(bot, msg).await
}

async fn ensure_approved_sender(bot: &Bot, msg: &Message) -> Result<bool, RequestError> {
    let sender = msg.from
        .as_ref()
        .map(|u| u.full_name())
        .unwrap_or_else(|| "Unknown".to_string());
    let sender_id = msg.from
        .as_ref()
        .map(|u| u.id.0.to_string())
        .unwrap_or_else(|| "0".to_string());

    if PairingManager::is_approved(&sender_id) {
        return Ok(true);
    }

    if PairingManager::is_pending(&sender_id) {
        bot.send_message(msg.chat.id, "Your request is pending approval.").await?;
    } else {
        match PairingManager::add_pending(&sender_id, &sender) {
            Ok(code) => {
                bot.send_message(
                    msg.chat.id,
                    format!("Pair first. Your code is: {}\nApprove with:\ntinyvegeta pairing approve {}", code, code),
                ).await?;
            }
            Err(e) => {
                tracing::warn!("Failed to add pending sender: {}", e);
            }
        }
    }
    Ok(false)
}

/// Handle regular (non-command) messages.
async fn handle_regular_message(bot: Bot, msg: Message) -> Result<(), RequestError> {
    // Get sender info
    let sender = msg.from
        .as_ref()
        .map(|u| u.full_name())
        .unwrap_or_else(|| "Unknown".to_string());
    
    let sender_id = msg.from
        .as_ref()
        .map(|u| u.id.0.to_string())
        .unwrap_or_else(|| "0".to_string());
    
    // Check pairing approval
    if !PairingManager::is_approved(&sender_id) {
        if PairingManager::is_pending(&sender_id) {
            bot.send_message(
                msg.chat.id,
                "Your request is pending approval."
            ).await?;
        } else {
            match PairingManager::add_pending(&sender_id, &sender) {
                Ok(code) => {
                    bot.send_message(
                        msg.chat.id,
                        format!("Welcome! Your pairing code is: {}\n\nApprove with:\ntinyvegeta pairing approve {}", code, code)
                    ).await?;
                }
                Err(e) => {
                    tracing::warn!("Failed to add pending sender: {}", e);
                }
            }
        }
        return Ok(());
    }
    
    // Collect text + file attachments.
    let mut text = msg.text().unwrap_or("").to_string();
    if text.is_empty() {
        text = msg.caption().unwrap_or("").to_string();
    }
    let mut downloaded_files: Vec<String> = Vec::new();

    if let Some(photos) = msg.photo() {
        if let Some(last) = photos.last() {
            if let Ok(Some(path)) = download_telegram_file(&last.file.id, ".jpg", None).await {
                downloaded_files.push(path);
            }
        }
    }
    if let Some(doc) = msg.document() {
        let ext = doc
            .file_name
            .as_deref()
            .and_then(|n| std::path::Path::new(n).extension().and_then(|e| e.to_str()))
            .unwrap_or("bin");
        if let Ok(Some(path)) = download_telegram_file(&doc.file.id, ext, doc.file_name.as_deref()).await {
            downloaded_files.push(path);
        }
    }
    if let Some(audio) = msg.audio() {
        let ext = audio
            .file_name
            .as_deref()
            .and_then(|n| std::path::Path::new(n).extension().and_then(|e| e.to_str()))
            .unwrap_or("mp3");
        if let Ok(Some(path)) = download_telegram_file(&audio.file.id, ext, audio.file_name.as_deref()).await {
            downloaded_files.push(path);
        }
    }
    if let Some(voice) = msg.voice() {
        if let Ok(Some(path)) = download_telegram_file(&voice.file.id, "ogg", None).await {
            downloaded_files.push(path);
        }
    }
    if let Some(video) = msg.video() {
        let ext = video
            .file_name
            .as_deref()
            .and_then(|n| std::path::Path::new(n).extension().and_then(|e| e.to_str()))
            .unwrap_or("mp4");
        if let Ok(Some(path)) = download_telegram_file(&video.file.id, ext, video.file_name.as_deref()).await {
            downloaded_files.push(path);
        }
    }
    if let Some(video_note) = msg.video_note() {
        if let Ok(Some(path)) = download_telegram_file(&video_note.file.id, "mp4", None).await {
            downloaded_files.push(path);
        }
    }
    if let Some(sticker) = msg.sticker() {
        if let Ok(Some(path)) = download_telegram_file(&sticker.file.id, "webp", None).await {
            downloaded_files.push(path);
            if text.trim().is_empty() {
                text = format!("[Sticker {}]", sticker.emoji.as_deref().unwrap_or("sticker"));
            }
        }
    }
    
    if text.trim().is_empty() && downloaded_files.is_empty() {
        return Ok(());
    }

    // SOUL edit mode capture: next non-command message becomes full SOUL.md content.
    if !text.trim().starts_with('/') {
        let mut pending = pending_soul_writes().lock().await;
        if let Some(target) = pending.get(&sender_id).cloned() {
            if let Err(e) = std::fs::create_dir_all(
                target
                    .soul_path
                    .parent()
                    .unwrap_or_else(|| std::path::Path::new(".")),
            ) {
                bot.send_message(msg.chat.id, format!("Failed to create SOUL directory: {}", e)).await?;
                pending.remove(&sender_id);
                return Ok(());
            }
            match std::fs::write(&target.soul_path, format!("{}\n", text.trim_end())) {
                Ok(_) => {
                    bot.send_message(
                        msg.chat.id,
                        format!(
                            "Saved SOUL.md for @{} ({})\nPath: {}",
                            target.agent_id,
                            target.agent_name,
                            target.soul_path.display()
                        ),
                    )
                    .await?;
                }
                Err(e) => {
                    bot.send_message(msg.chat.id, format!("Failed to save SOUL.md: {}", e)).await?;
                }
            }
            pending.remove(&sender_id);
            return Ok(());
        }
    }
    
    // Parse routing
    let mut routed_text = text.to_string();
    if !text.trim_start().starts_with('@') && triage_enabled() {
        if let Some(agent) = triage_agent_candidate(&text) {
            if let Ok(settings) = load_settings() {
                if settings.agents.contains_key(&agent) {
                    routed_text = format!("@{} {}", agent, text);
                    let _ = bot.send_message(msg.chat.id, format!("Auto-routed to @{}.", agent)).await;
                }
            }
        }
    }
    if !downloaded_files.is_empty() {
        let refs = downloaded_files
            .iter()
            .map(|p| format!("[file: {}]", p))
            .collect::<Vec<_>>()
            .join("\n");
        routed_text = if routed_text.trim().is_empty() {
            refs
        } else {
            format!("{}\n\n{}", routed_text, refs)
        };
    }
    let (target_agent, message) = parse_message_routing(&routed_text);
    
    // Create message data
    use crate::core::MessageData;
    let mut message_data = MessageData::new(
        "telegram",
        &sender,
        &sender_id,
        &message,
    );
    
    message_data.message_id = Some(msg.id.0 as i64);
    message_data.response_channel = Some("telegram".to_string());
    message_data.response_chat_id = Some(msg.chat.id.0);
    if !downloaded_files.is_empty() {
        message_data.files = Some(downloaded_files.clone());
    }
    
    if let Some(ref agent) = target_agent {
        message_data.agent = Some(agent.clone());
    }
    
    // Enqueue message
    match crate::core::Queue::enqueue(message_data) {
        Ok(id) => {
            tracing::info!("Enqueued message {} from {} to agent {:?}", id, sender, target_agent);
            let short_id = id.chars().take(8).collect::<String>();
            let route = target_agent.unwrap_or_else(|| "default".to_string());
            let _ = bot
                .send_message(
                    msg.chat.id,
                    format!("📥 Task {} queued for @{}. I’ll update when it starts and completes.", short_id, route),
                )
                .await;
            let _ = bot.send_chat_action(msg.chat.id, teloxide::types::ChatAction::Typing).await;
        }
        Err(e) => {
            tracing::error!("Failed to enqueue message: {}", e);
            bot.send_message(msg.chat.id, "Failed to process message.").await?;
        }
    }
    
    Ok(())
}

/// Parse message for routing (e.g., @agent_id message).
fn parse_message_routing(text: &str) -> (Option<String>, String) {
    if text.starts_with('@') {
        if let Some(space_idx) = text.find(' ') {
            let agent = &text[1..space_idx];
            let message = text[space_idx + 1..].trim();
            
            if agent.chars().all(|c| c.is_alphanumeric() || c == '_') {
                return (Some(agent.to_string()), message.to_string());
            }
        }
    }
    (None, text.to_string())
}

/// Handle /agents command.
async fn cmd_agents(bot: Bot, chat_id: ChatId) -> Result<(), RequestError> {
    let settings = match load_settings() {
        Ok(s) => s,
        Err(e) => {
            bot.send_message(chat_id, format!("Error: {}", e)).await?;
            return Ok(());
        }
    };
    
    let mut response = String::from("Agents:\n");
    for (id, agent) in &settings.agents {
        let name = agent.name.as_deref().unwrap_or(id);
        let provider = agent.provider.as_deref().unwrap_or("unknown");
        response.push_str(&format!("• @{} - {} ({})\n", id, name, provider));
    }
    
    bot.send_message(chat_id, response).await?;
    Ok(())
}

/// Handle /teams command.
async fn cmd_teams(bot: Bot, chat_id: ChatId) -> Result<(), RequestError> {
    let settings = match load_settings() {
        Ok(s) => s,
        Err(e) => {
            bot.send_message(chat_id, format!("Error: {}", e)).await?;
            return Ok(());
        }
    };
    
    if settings.teams.is_empty() {
        bot.send_message(chat_id, "No teams configured.").await?;
        return Ok(());
    }
    
    let mut response = String::from("Teams:\n");
    for (id, team) in &settings.teams {
        response.push_str(&format!("• @{} - {}: {:?}\n", id, team.name, team.agents));
    }
    
    bot.send_message(chat_id, response).await?;
    Ok(())
}

/// Handle /board command.
async fn cmd_board(bot: Bot, chat_id: ChatId) -> Result<(), RequestError> {
    let settings = match load_settings() {
        Ok(s) => s,
        Err(e) => {
            bot.send_message(chat_id, format!("Error: {}", e)).await?;
            return Ok(());
        }
    };
    
    if let Some(board) = &settings.board.team_id {
        let board_config = settings.teams.get(board);
        if let Some(team) = board_config {
            let response = format!(
                "Board: @{}\nLeader: @{}\nMembers: {}",
                board,
                team.leader_agent.as_deref().unwrap_or("none"),
                team.agents.join(", ")
            );
            bot.send_message(chat_id, response).await?;
        } else {
            bot.send_message(chat_id, format!("Board team @{} not found", board)).await?;
        }
    } else {
        bot.send_message(chat_id, "No board configured.").await?;
    }
    
    Ok(())
}

/// Handle /status command.
async fn cmd_status(bot: Bot, chat_id: ChatId) -> Result<(), RequestError> {
    match crate::tmux::get_status() {
        Ok(status) => {
            bot.send_message(chat_id, status).await?;
        }
        Err(e) => {
            bot.send_message(chat_id, format!("Status check failed: {}", e)).await?;
        }
    }
    Ok(())
}

async fn cmd_doctor(bot: Bot, chat_id: ChatId) -> Result<(), RequestError> {
    let exe = std::env::current_exe()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|_| "tinyvegeta".to_string());

    let out = TokioCommand::new(exe)
        .arg("doctor")
        .output()
        .await;

    match out {
        Ok(output) => {
            let text = format!(
                "{}\n{}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
            let mut lines = Vec::new();
            for line in text.lines() {
                let l = line.trim();
                if l.starts_with("📋")
                    || l.starts_with("📡")
                    || l.starts_with("✅")
                    || l.starts_with("❌")
                    || l.starts_with("⚠")
                    || l.starts_with("🔧")
                    || l.starts_with("   ")
                {
                    lines.push(l.to_string());
                }
            }
            if lines.is_empty() {
                lines = text.lines().rev().take(25).map(|s| s.to_string()).collect();
                lines.reverse();
            }
            let mut response = format!("Doctor summary:\n{}", lines.join("\n"));
            if response.len() > 3900 {
                response.truncate(3900);
                response.push_str("\n...[truncated]");
            }
            bot.send_message(chat_id, response).await?;
        }
        Err(e) => {
            bot.send_message(chat_id, format!("Doctor failed: {}", e)).await?;
        }
    }
    Ok(())
}

async fn cmd_provider(bot: Bot, chat_id: ChatId, provider: Option<&str>) -> Result<(), RequestError> {
    if let Some(p) = provider {
        let exe = std::env::current_exe()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|_| "tinyvegeta".to_string());
        match TokioCommand::new(exe).args(["provider", p]).output().await {
            Ok(output) => {
                let text = String::from_utf8_lossy(&output.stdout).to_string();
                let err = String::from_utf8_lossy(&output.stderr).to_string();
                let reply = if output.status.success() {
                    format!("Provider updated:\n{}", text.trim())
                } else {
                    format!("Provider switch failed:\n{}", if !err.trim().is_empty() { err.trim() } else { text.trim() })
                };
                bot.send_message(chat_id, reply).await?;
            }
            Err(e) => {
                bot.send_message(chat_id, format!("Provider switch failed: {}", e)).await?;
            }
        }
    } else {
        match load_settings() {
            Ok(settings) => {
                let default_agent = crate::core::routing::get_default_agent(&settings).unwrap_or_else(|| "assistant".to_string());
                let active = settings.agents.get(&default_agent);
                let provider_name = active
                    .and_then(|a| a.provider.as_deref())
                    .unwrap_or(&settings.models.provider);
                let model = active.and_then(|a| a.model.as_deref()).unwrap_or("default");
                bot.send_message(
                    chat_id,
                    format!("Current provider: {}\nDefault agent: @{}\nAgent model: {}", provider_name, default_agent, model),
                )
                .await?;
            }
            Err(e) => {
                bot.send_message(chat_id, format!("Could not load settings: {}", e)).await?;
            }
        }
    }
    Ok(())
}

async fn cmd_memory(bot: Bot, chat_id: ChatId, sub: Option<&str>, args: &[&str]) -> Result<(), RequestError> {
    match sub.unwrap_or("") {
        "stats" => match crate::memory::Memory::stats() {
            Ok(stats) => {
                bot.send_message(chat_id, stats.to_string()).await?;
            }
            Err(e) => {
                bot.send_message(chat_id, format!("Memory stats failed: {}", e)).await?;
            }
        },
        "search" => {
            let query = args.join(" ").trim().to_string();
            if query.is_empty() {
                bot.send_message(chat_id, "Usage: /memory search <query>").await?;
                return Ok(());
            }
            match crate::memory::Memory::search(&query, 8) {
                Ok(results) => {
                    if results.is_empty() {
                        bot.send_message(chat_id, "No memory matches found.").await?;
                    } else {
                        let mut out = format!("Memory search: \"{}\"\n", query);
                        for entry in results {
                            out.push_str(&format!(
                                "- [{}] {} = {}\n",
                                entry.scope,
                                entry.key,
                                entry.value.chars().take(140).collect::<String>()
                            ));
                        }
                        if out.len() > 3900 {
                            out.truncate(3900);
                            out.push_str("\n...[truncated]");
                        }
                        bot.send_message(chat_id, out).await?;
                    }
                }
                Err(e) => {
                    bot.send_message(chat_id, format!("Memory search failed: {}", e)).await?;
                }
            }
        }
        _ => {
            bot.send_message(
                chat_id,
                "Usage:\n/memory stats\n/memory search <query>",
            )
            .await?;
        }
    }
    Ok(())
}

fn resolve_brain_file() -> Option<std::path::PathBuf> {
    if let Ok(raw) = std::env::var("TINYVEGETA_BRAIN_PATH") {
        let trimmed = raw.trim();
        if !trimmed.is_empty() {
            return Some(std::path::PathBuf::from(trimmed));
        }
    }
    directories::UserDirs::new().map(|u| u.home_dir().join("ai").join("tinyvegeta").join("BRAIN.md"))
}

async fn cmd_brain(bot: Bot, chat_id: ChatId, sub: Option<&str>, args: &[&str]) -> Result<(), RequestError> {
    let Some(path) = resolve_brain_file() else {
        bot.send_message(chat_id, "Could not resolve BRAIN.md path.").await?;
        return Ok(());
    };
    match sub.unwrap_or("show") {
        "show" => {
            if !path.exists() {
                bot.send_message(chat_id, format!("BRAIN.md not found at {}", path.display())).await?;
                return Ok(());
            }
            let content = std::fs::read_to_string(&path).unwrap_or_default();
            let preview = if content.len() > 3500 {
                format!("{}...\n[truncated]", &content[..3500])
            } else {
                content
            };
            bot.send_message(chat_id, format!("BRAIN.md ({})\n\n{}", path.display(), preview)).await?;
        }
        "status" => {
            let last_check = crate::memory::Memory::get("brain.last_check", crate::memory::MemoryScope::Global, None)
                .ok()
                .flatten()
                .map(|v| v.value)
                .unwrap_or_else(|| "never".to_string());
            let last_summary = crate::memory::Memory::get("brain.last_summary", crate::memory::MemoryScope::Global, None)
                .ok()
                .flatten()
                .map(|v| v.value)
                .unwrap_or_else(|| "-".to_string());
            bot.send_message(
                chat_id,
                format!("BRAIN status\nPath: {}\nLast check: {}\nLast summary: {}", path.display(), last_check, last_summary),
            )
            .await?;
        }
        "add" => {
            let text = args.join(" ").trim().to_string();
            if text.is_empty() {
                bot.send_message(chat_id, "Usage: /brain add <text>").await?;
                return Ok(());
            }
            let mut existing = if path.exists() {
                std::fs::read_to_string(&path).unwrap_or_default()
            } else {
                "## active projects\n\n## immediate actions\n\n## background tasks\n".to_string()
            };
            let ts = chrono::Local::now().format("%Y-%m-%d %H:%M").to_string();
            existing.push_str(&format!("- [{}] {}\n", ts, text));
            if let Some(parent) = path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            match std::fs::write(&path, existing) {
                Ok(_) => {
                    let _ = crate::memory::sqlite::record_event("brain-manual", "assistant", "brain_add", &text);
                    bot.send_message(chat_id, format!("Added to BRAIN.md at {}", path.display())).await?;
                }
                Err(e) => {
                    bot.send_message(chat_id, format!("Failed to update BRAIN.md: {}", e)).await?;
                }
            }
        }
        _ => {
            bot.send_message(chat_id, "Usage:\n/brain show\n/brain status\n/brain add <text>").await?;
        }
    }
    Ok(())
}

async fn cmd_board_discuss(bot: Bot, chat_id: ChatId, topic: &str) -> Result<(), RequestError> {
    let settings = match load_settings() {
        Ok(s) => s,
        Err(e) => {
            bot.send_message(chat_id, format!("Error: {}", e)).await?;
            return Ok(());
        }
    };
    let team_id = settings
        .board
        .team_id
        .clone()
        .unwrap_or_else(|| "board".to_string());
    match crate::board::run_board_discussion(&settings, &team_id, topic, None).await {
        Ok(output) => {
            let decision = output
                .split("CEO (")
                .nth(1)
                .map(|s| format!("CEO ({s}"))
                .unwrap_or_else(|| output.clone());
            let mut response = format!(
                "Board Discussion\nTeam: @{}\nTopic: {}\n\nDecision:\n{}",
                team_id,
                topic,
                decision.trim()
            );
            if response.len() > 3900 {
                response.truncate(3900);
                response.push_str("\n...[truncated]");
            }
            bot.send_message(chat_id, response).await?;
        }
        Err(e) => {
            bot.send_message(chat_id, format!("Board discussion failed: {}", e)).await?;
        }
    }
    Ok(())
}

async fn cmd_logs(bot: Bot, chat_id: ChatId, log_type: &str, lines: usize) -> Result<(), RequestError> {
    let limit = lines.clamp(10, 400);
    let log_dir = match directories::ProjectDirs::from("com", "tinyvegeta", "tinyvegeta") {
        Some(p) => p.data_dir().join("logs"),
        None => {
            bot.send_message(chat_id, "Could not resolve log directory.").await?;
            return Ok(());
        }
    };
    let path = log_dir.join("tinyvegeta.log");
    let content = match std::fs::read_to_string(&path) {
        Ok(c) => c,
        Err(e) => {
            bot.send_message(chat_id, format!("Failed to read logs: {}", e)).await?;
            return Ok(());
        }
    };

    let needle = match log_type {
        "telegram" => Some("telegram"),
        "queue" => Some("queue"),
        "heartbeat" => Some("heartbeat"),
        "all" => None,
        _ => {
            bot.send_message(chat_id, "Usage: /logs <telegram|queue|heartbeat|all> [lines]").await?;
            return Ok(());
        }
    };

    let mut filtered: Vec<&str> = content.lines().collect();
    if let Some(n) = needle {
        filtered.retain(|line| line.to_lowercase().contains(n));
    }
    let start = filtered.len().saturating_sub(limit);
    let tail = filtered[start..].join("\n");

    let mut response = format!("Logs ({}, last {}):\n{}", log_type, limit, tail);
    if response.len() > 3900 {
        response = format!("Logs ({}, last {}):\n{}", log_type, limit, &response.chars().rev().take(3600).collect::<String>().chars().rev().collect::<String>());
    }
    if response.trim().is_empty() {
        response = format!("No {} logs found.", log_type);
    }
    bot.send_message(chat_id, response).await?;
    Ok(())
}

async fn cmd_releasecheck(bot: Bot, chat_id: ChatId) -> Result<(), RequestError> {
    let exe = std::env::current_exe()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|_| "tinyvegeta".to_string());
    match TokioCommand::new(exe).arg("releasecheck").output().await {
        Ok(out) => {
            let text = format!(
                "{}\n{}",
                String::from_utf8_lossy(&out.stdout),
                String::from_utf8_lossy(&out.stderr)
            );
            let reply = if text.trim().is_empty() {
                "releasecheck finished with no output".to_string()
            } else if text.len() > 3900 {
                format!("{}...\n[truncated]", &text[..3900])
            } else {
                text
            };
            bot.send_message(chat_id, reply).await?;
        }
        Err(e) => {
            bot.send_message(chat_id, format!("releasecheck failed: {}", e)).await?;
        }
    }
    Ok(())
}

fn sovereign_pid_key() -> &'static str {
    "sovereign.process.pid"
}

fn sovereign_meta_key() -> &'static str {
    "sovereign.process.meta"
}

fn parse_stored_pid() -> Option<u32> {
    crate::memory::Memory::get(sovereign_pid_key(), crate::memory::MemoryScope::Global, None)
        .ok()
        .flatten()
        .and_then(|v| v.value.parse::<u32>().ok())
}

fn is_pid_alive(pid: u32) -> bool {
    std::process::Command::new("kill")
        .arg("-0")
        .arg(pid.to_string())
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn clear_sovereign_state() {
    let _ = crate::memory::Memory::delete(
        sovereign_pid_key(),
        crate::memory::MemoryScope::Global,
        None,
    );
    let _ = crate::memory::Memory::delete(
        sovereign_meta_key(),
        crate::memory::MemoryScope::Global,
        None,
    );
}

async fn cmd_sovereign(bot: Bot, chat_id: ChatId, args: &[&str]) -> Result<(), RequestError> {
    let action = args.first().copied().unwrap_or("status");
    match action {
        "status" => {
            if let Some(pid) = parse_stored_pid() {
                if is_pid_alive(pid) {
                    let meta = crate::memory::Memory::get(
                        sovereign_meta_key(),
                        crate::memory::MemoryScope::Global,
                        None,
                    )
                    .ok()
                    .flatten()
                    .map(|m| m.value)
                    .unwrap_or_else(|| "no metadata".to_string());
                    bot.send_message(
                        chat_id,
                        format!("Sovereign runtime: running\nPID: {}\n{}", pid, meta),
                    )
                    .await?;
                } else {
                    clear_sovereign_state();
                    bot.send_message(chat_id, "Sovereign runtime: not running (stale PID cleared).")
                        .await?;
                }
            } else {
                bot.send_message(chat_id, "Sovereign runtime: not running.").await?;
            }
        }
        "stop" => {
            if let Some(pid) = parse_stored_pid() {
                if is_pid_alive(pid) {
                    let out = std::process::Command::new("kill")
                        .arg(pid.to_string())
                        .output();
                    match out {
                        Ok(o) if o.status.success() => {
                            clear_sovereign_state();
                            bot.send_message(chat_id, format!("Stopped sovereign runtime (PID {}).", pid))
                                .await?;
                        }
                        Ok(o) => {
                            let err = String::from_utf8_lossy(&o.stderr).to_string();
                            bot.send_message(
                                chat_id,
                                format!("Failed to stop PID {}: {}", pid, err.trim()),
                            )
                            .await?;
                        }
                        Err(e) => {
                            bot.send_message(chat_id, format!("Stop failed: {}", e)).await?;
                        }
                    }
                } else {
                    clear_sovereign_state();
                    bot.send_message(chat_id, "Sovereign runtime already stopped (stale PID cleared).")
                        .await?;
                }
            } else {
                bot.send_message(chat_id, "Sovereign runtime is not running.").await?;
            }
        }
        "start" => {
            if let Some(pid) = parse_stored_pid() {
                if is_pid_alive(pid) {
                    bot.send_message(
                        chat_id,
                        format!("Sovereign runtime already running (PID {}).", pid),
                    )
                    .await?;
                    return Ok(());
                }
                clear_sovereign_state();
            }

            let mut agent = "assistant".to_string();
            let mut dry_run = false;
            let mut goal_parts: Vec<String> = Vec::new();
            for raw in args.iter().skip(1) {
                if *raw == "--dry-run" || *raw == "dry-run" {
                    dry_run = true;
                    continue;
                }
                if let Some(stripped) = raw.strip_prefix('@') {
                    if !stripped.trim().is_empty() {
                        agent = stripped.to_lowercase();
                    }
                    continue;
                }
                goal_parts.push((*raw).to_string());
            }
            let goal = if goal_parts.is_empty() {
                "improve tinyvegeta safely".to_string()
            } else {
                goal_parts.join(" ")
            };

            let exe = std::env::current_exe()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|_| "tinyvegeta".to_string());

            let mut cmd = std::process::Command::new(exe);
            cmd.arg("sovereign")
                .arg("--agent")
                .arg(&agent)
                .arg("--goal")
                .arg(&goal)
                .stdin(std::process::Stdio::null())
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null());
            if dry_run {
                cmd.arg("--dry-run");
            }

            match cmd.spawn() {
                Ok(child) => {
                    let pid = child.id();
                    let _ = crate::memory::Memory::set(
                        sovereign_pid_key(),
                        &pid.to_string(),
                        crate::memory::MemoryScope::Global,
                        None,
                    );
                    let meta = format!(
                        "agent=@{} goal=\"{}\" dry_run={} started_at={}",
                        agent,
                        goal,
                        dry_run,
                        chrono::Utc::now().to_rfc3339()
                    );
                    let _ = crate::memory::Memory::set(
                        sovereign_meta_key(),
                        &meta,
                        crate::memory::MemoryScope::Global,
                        None,
                    );
                    bot.send_message(
                        chat_id,
                        format!("Started sovereign runtime.\nPID: {}\n{}", pid, meta),
                    )
                    .await?;
                }
                Err(e) => {
                    bot.send_message(chat_id, format!("Failed to start sovereign runtime: {}", e))
                        .await?;
                }
            }
        }
        _ => {
            bot.send_message(
                chat_id,
                "Usage:\n/sovereign status\n/sovereign start [@agent] [goal words...] [--dry-run]\n/sovereign stop",
            )
            .await?;
        }
    }
    Ok(())
}

async fn cmd_reset_agents(bot: Bot, chat_id: ChatId, agent_ids: &[String]) -> Result<(), RequestError> {
    let settings = match load_settings() {
        Ok(s) => s,
        Err(e) => {
            bot.send_message(chat_id, format!("Failed to load settings: {}", e)).await?;
            return Ok(());
        }
    };
    let mut lines = Vec::new();
    for agent_id in agent_ids {
        let Some(agent) = settings.agents.get(agent_id) else {
            lines.push(format!("Agent not found: @{}", agent_id));
            continue;
        };
        let Some(wd) = agent.working_directory.as_ref() else {
            lines.push(format!("No working directory for @{}", agent_id));
            continue;
        };
        if let Err(e) = std::fs::create_dir_all(wd) {
            lines.push(format!("Failed to create dir for @{}: {}", agent_id, e));
            continue;
        }
        match std::fs::write(wd.join("reset_flag"), "reset\n") {
            Ok(_) => lines.push(format!("Reset flagged for @{}", agent_id)),
            Err(e) => lines.push(format!("Failed to reset @{}: {}", agent_id, e)),
        }
    }
    bot.send_message(chat_id, lines.join("\n")).await?;
    Ok(())
}

fn triage_enabled() -> bool {
    use crate::memory::{Memory, MemoryScope};
    Memory::get("triage.enabled", MemoryScope::Global, None)
        .ok()
        .flatten()
        .map(|v| v.value == "true")
        .unwrap_or(false)
}

fn set_triage_enabled(enabled: bool) {
    use crate::memory::{Memory, MemoryScope};
    let _ = Memory::set("triage.enabled", if enabled { "true" } else { "false" }, MemoryScope::Global, None);
}

fn triage_agent_candidate(message: &str) -> Option<String> {
    let m = message.to_lowercase();
    let picks = [
        ("security", &["vulnerability", "security", "auth", "xss", "csrf", "token"][..]),
        ("operations", &["deploy", "docker", "infra", "latency", "incident", "uptime"][..]),
        ("marketing", &["campaign", "brand", "launch", "positioning"][..]),
        ("seo", &["seo", "keywords", "ranking", "serp"][..]),
        ("sales", &["lead", "pipeline", "deal", "prospect", "pricing"][..]),
        ("coder", &["bug", "code", "refactor", "test", "build", "rust", "api"][..]),
    ];
    for (agent, terms) in picks {
        if terms.iter().any(|t| m.contains(t)) {
            return Some(agent.to_string());
        }
    }
    None
}

async fn cmd_triage(bot: Bot, chat_id: ChatId, arg: &str) -> Result<(), RequestError> {
    match arg {
        "on" | "enable" | "enabled" => {
            set_triage_enabled(true);
            bot.send_message(chat_id, "Auto-triage enabled.").await?;
        }
        "off" | "disable" | "disabled" => {
            set_triage_enabled(false);
            bot.send_message(chat_id, "Auto-triage disabled.").await?;
        }
        _ => {
            let status = if triage_enabled() { "enabled" } else { "disabled" };
            bot.send_message(chat_id, format!("Auto-triage is {}.", status)).await?;
        }
    }
    Ok(())
}

fn ensure_soul_authorized(sender_id: &str) -> std::result::Result<bool, String> {
    let settings = load_settings().map_err(|e| e.to_string())?;
    if let Some(owner) = settings.pairing.soul_owner_sender_id.as_deref() {
        if owner != sender_id {
            return Err(format!(
                "Only SOUL owner can use /soul. Allowed sender: {}",
                owner
            ));
        }
        return Ok(false);
    }
    PairingManager::set_soul_owner(sender_id)?;
    Ok(true)
}

fn resolve_soul_target(agent_hint: Option<&str>) -> std::result::Result<SoulTarget, String> {
    let settings = load_settings().map_err(|e| e.to_string())?;
    let agent_id = if let Some(raw) = agent_hint {
        raw.trim_start_matches('@').trim().to_lowercase()
    } else {
        crate::core::routing::get_default_agent(&settings).unwrap_or_else(|| "assistant".to_string())
    };
    let agent = settings
        .agents
        .get(&agent_id)
        .ok_or_else(|| format!("Agent not found: {}", agent_id))?;
    let workdir = agent
        .working_directory
        .clone()
        .ok_or_else(|| format!("No working directory for @{}", agent_id))?;
    Ok(SoulTarget {
        agent_id: agent_id.clone(),
        agent_name: agent.name.clone().unwrap_or(agent_id),
        soul_path: workdir.join("SOUL.md"),
    })
}

async fn cmd_soul(bot: Bot, msg: &Message, args: &[&str]) -> Result<(), RequestError> {
    let sender_id = msg
        .from
        .as_ref()
        .map(|u| u.id.0.to_string())
        .unwrap_or_else(|| "0".to_string());
    let claimed = match ensure_soul_authorized(&sender_id) {
        Ok(c) => c,
        Err(reason) => {
            bot.send_message(msg.chat.id, reason).await?;
            return Ok(());
        }
    };

    if args.first().map(|s| s.eq_ignore_ascii_case("cancel")).unwrap_or(false) {
        pending_soul_writes().lock().await.remove(&sender_id);
        bot.send_message(msg.chat.id, "SOUL edit canceled.").await?;
        return Ok(());
    }

    if args.first().map(|s| s.eq_ignore_ascii_case("show")).unwrap_or(false) {
        let target = match resolve_soul_target(args.get(1).copied()) {
            Ok(t) => t,
            Err(e) => {
                bot.send_message(msg.chat.id, e).await?;
                return Ok(());
            }
        };
        if !target.soul_path.exists() {
            bot.send_message(msg.chat.id, format!("No SOUL.md yet for @{}.", target.agent_id)).await?;
            return Ok(());
        }
        let content = std::fs::read_to_string(&target.soul_path).unwrap_or_default();
        let preview = if content.len() > 3500 {
            format!("{}...\n[truncated]", &content[..3500])
        } else {
            content
        };
        bot.send_message(msg.chat.id, format!("SOUL.md for @{}:\n\n{}", target.agent_id, preview)).await?;
        return Ok(());
    }

    let target = match resolve_soul_target(args.first().copied()) {
        Ok(t) => t,
        Err(e) => {
            bot.send_message(msg.chat.id, format!("{}\nUsage: /soul [@agent]\n/soul show [@agent]\n/soul cancel", e)).await?;
            return Ok(());
        }
    };

    pending_soul_writes()
        .lock()
        .await
        .insert(sender_id, target.clone());
    let ownership = if claimed {
        "\nSOUL owner locked to this sender."
    } else {
        ""
    };
    bot.send_message(
        msg.chat.id,
        format!(
            "SOUL edit mode enabled for @{} ({}).\nSend full SOUL.md content in your next message.\nUse /soul cancel to abort.{}",
            target.agent_id, target.agent_name, ownership
        ),
    )
    .await?;
    Ok(())
}

/// Handle /restart command.
async fn cmd_restart(bot: Bot, msg: Message) -> Result<(), RequestError> {
    let sender = msg.from
        .as_ref()
        .map(|u| u.full_name())
        .unwrap_or_else(|| "Unknown".to_string());
    let sender_id = msg.from
        .as_ref()
        .map(|u| u.id.0.to_string())
        .unwrap_or_else(|| "0".to_string());

    if !PairingManager::is_approved(&sender_id) {
        if PairingManager::is_pending(&sender_id) {
            bot.send_message(msg.chat.id, "Your request is pending approval.").await?;
        } else {
            match PairingManager::add_pending(&sender_id, &sender) {
                Ok(code) => {
                    bot.send_message(
                        msg.chat.id,
                        format!("Pair first. Your code: {}\nApprove with:\ntinyvegeta pairing approve {}", code, code),
                    ).await?;
                }
                Err(e) => {
                    tracing::warn!("Failed to add pending sender for /restart: {}", e);
                }
            }
        }
        return Ok(());
    }

    bot.send_message(msg.chat.id, "Restarting TinyVegeta daemon...").await?;

    let exe = std::env::current_exe()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|_| "tinyvegeta".to_string());

    let spawn_result = std::process::Command::new("nohup")
        .arg(exe)
        .arg("restart")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn();

    if let Err(e) = spawn_result {
        tracing::error!("Failed to spawn restart command: {}", e);
        bot.send_message(msg.chat.id, format!("Failed to restart: {}", e)).await?;
    }

    Ok(())
}

const HELP_TEXT: &str = r#"TinyVegeta Commands:

/help - Show this help
/agent - List agents
/team - List teams
/board - Show board info
/board discuss <topic> - Run board discussion
/status - Show daemon status
/restart - Restart TinyVegeta daemon
/doctor - Run health checks
/provider [name] - Show or switch provider
/memory stats - Memory statistics
/memory search <query> - Search memory
/brain show - Show BRAIN.md
/brain status - Show proactive brain status
/brain add <text> - Append note/action to BRAIN.md
/logs <telegram|queue|heartbeat|all> [lines] - Tail logs
/gateway [status|restart] - Gateway controls
/releasecheck - Run release checks
/sovereign [start|stop|status] - Control autonomous sovereign loop
/reset @agent [@agent2...] - Reset specific agents
/triage [on|off|status] - Auto-triage controls
/soul [@agent] - Start SOUL edit mode
/soul show [@agent] - Preview SOUL.md
/soul cancel - Cancel SOUL edit mode
/discuss <topic> - Start board discussion

Direct Messages:
- Just send a message to chat with the AI
- Use @agent_id to route to specific agent
- Use @team_id to route to team
"#;
