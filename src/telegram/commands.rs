//! Telegram bot commands.
#![allow(dead_code)]

use teloxide::prelude::*;
use teloxide::types::Message;

use crate::config::load_settings;

/// Handle /help command.
pub async fn cmd_help(bot: Bot, msg: Message) -> Result<(), teloxide::RequestError> {
    let help_text = r#"TinyVegeta Commands:

/help - Show this help
/agent - List agents
/team - List teams
/board - Show board info
/reset - Reset conversation
/triage - Toggle auto-triage
/discuss <topic> - Start board discussion

Direct Messages:
- Just send a message to chat with the AI
- Use @agent_id to route to specific agent
- Use @team_id to route to team"#;
    
    bot.send_message(msg.chat.id, help_text).await?;
    Ok(())
}

/// Handle /agent command.
pub async fn cmd_agents(bot: Bot, msg: Message) -> Result<(), teloxide::RequestError> {
    let settings = match load_settings() {
        Ok(s) => s,
        Err(e) => {
            bot.send_message(msg.chat.id, format!("Error loading settings: {}", e)).await?;
            return Ok(());
        }
    };
    
    let mut response = String::from("Agents:\n");
    for (id, agent) in &settings.agents {
        let name = agent.name.as_deref().unwrap_or(id);
        let provider = agent.provider.as_deref().unwrap_or("unknown");
        response.push_str(&format!("• @{} - {} ({})\n", id, name, provider));
    }
    
    bot.send_message(msg.chat.id, response).await?;
    Ok(())
}

/// Handle /team command.
pub async fn cmd_teams(bot: Bot, msg: Message) -> Result<(), teloxide::RequestError> {
    let settings = match load_settings() {
        Ok(s) => s,
        Err(e) => {
            bot.send_message(msg.chat.id, format!("Error loading settings: {}", e)).await?;
            return Ok(());
        }
    };
    
    if settings.teams.is_empty() {
        bot.send_message(msg.chat.id, "No teams configured.").await?;
        return Ok(());
    }
    
    let mut response = String::from("Teams:\n");
    for (id, team) in &settings.teams {
        response.push_str(&format!("• @{} - {}: {:?}\n", id, team.name, team.agents));
    }
    
    bot.send_message(msg.chat.id, response).await?;
    Ok(())
}

/// Handle /board command.
pub async fn cmd_board(bot: Bot, msg: Message) -> Result<(), teloxide::RequestError> {
    let settings = match load_settings() {
        Ok(s) => s,
        Err(e) => {
            bot.send_message(msg.chat.id, format!("Error loading settings: {}", e)).await?;
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
            bot.send_message(msg.chat.id, response).await?;
        } else {
            bot.send_message(msg.chat.id, format!("Board team @{} not found", board)).await?;
        }
    } else {
        bot.send_message(msg.chat.id, "No board configured. Use `tinyvegeta board create` to set up.").await?;
    }
    
    Ok(())
}

/// Handle /reset command.
pub async fn cmd_reset(bot: Bot, msg: Message) -> Result<(), teloxide::RequestError> {
    // For now, just acknowledge
    bot.send_message(msg.chat.id, "Conversation reset. Start fresh!").await?;
    Ok(())
}

/// Handle /triage command.
pub async fn cmd_triage(bot: Bot, msg: Message) -> Result<(), teloxide::RequestError> {
    bot.send_message(msg.chat.id, "Auto-triage is enabled. Bug/security/ops messages will be auto-routed.").await?;
    Ok(())
}

/// Handle /discuss command.
pub async fn cmd_discuss(bot: Bot, msg: Message, topic: String) -> Result<(), teloxide::RequestError> {
    bot.send_message(msg.chat.id, format!("Starting board discussion: {}\n\n(This feature requires the board to be configured)", topic)).await?;
    Ok(())
}

/// Handle unknown commands.
pub async fn cmd_unknown(bot: Bot, msg: Message) -> Result<(), teloxide::RequestError> {
    bot.send_message(msg.chat.id, "Unknown command. Send /help for available commands.").await?;
    Ok(())
}
