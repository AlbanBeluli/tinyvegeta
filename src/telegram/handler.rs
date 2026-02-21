//! Telegram message handling.
#![allow(dead_code)]

use teloxide::prelude::*;
use teloxide::types::Message;

use crate::core::{Queue, MessageData};
use super::pairing::PairingManager;

/// Handle incoming messages.
pub async fn handle_message(bot: Bot, msg: Message) -> Result<(), teloxide::RequestError> {
    // Get sender info
    let sender = msg.from
        .as_ref()
        .map(|u| u.full_name())
        .unwrap_or_else(|| "Unknown".to_string());
    
    let sender_id = msg.from
        .as_ref()
        .map(|u| u.id.0)
        .unwrap_or_else(|| 0);
    
    // Check pairing approval
    if !PairingManager::is_approved(&sender_id.to_string()) {
        // Check if this is the first message (might need pairing)
        if PairingManager::is_pending(&sender_id.to_string()) {
            bot.send_message(
                msg.chat.id,
                "Your request is pending approval. Please wait or use the pairing code from shell."
            ).await?;
        } else {
            // New sender - generate pairing code
            match PairingManager::add_pending(&sender_id.to_string(), &sender) {
                Ok(code) => {
                    bot.send_message(
                        msg.chat.id,
                        format!("Welcome! Your pairing code is: {}\n\nApprove from shell with:\ntinyvegeta pairing approve {}", code, code)
                    ).await?;
                }
                Err(e) => {
                    tracing::warn!("Failed to add pending sender: {}", e);
                }
            }
        }
        return Ok(());
    }
    
    // Get message text
    let text = msg.text().unwrap_or("");
    
    if text.is_empty() {
        return Ok(());
    }
    
    // Parse routing
    let (target_agent, message) = parse_message_routing(text);
    
    // Create message data
    let mut message_data = MessageData::new(
        "telegram",
        &sender,
        &sender_id.to_string(),
        &message,
    );
    
    message_data.message_id = Some(msg.id.0 as i64);
    message_data.response_channel = Some("telegram".to_string());
    message_data.response_chat_id = Some(msg.chat.id.0);
    
    if let Some(ref agent) = target_agent {
        message_data.agent = Some(agent.clone());
    }
    
    // Enqueue message
    match Queue::enqueue(message_data) {
        Ok(id) => {
            tracing::info!("Enqueued message {} from {} to agent {:?}", id, sender, target_agent);
            // No status message - process and respond directly.
        }
        Err(e) => {
            tracing::error!("Failed to enqueue message: {}", e);
            bot.send_message(
                msg.chat.id,
                "Failed to process message. Please try again."
            ).await?;
        }
    }
    
    Ok(())
}

/// Parse message for routing (e.g., @agent_id message).
fn parse_message_routing(text: &str) -> (Option<String>, String) {
    // Check for @agent_id prefix
    if text.starts_with('@') {
        // Find first space
        if let Some(space_idx) = text.find(' ') {
            let agent = &text[1..space_idx];
            let message = text[space_idx + 1..].trim();
            
            // Validate agent ID (alphanumeric + underscore)
            if agent.chars().all(|c| c.is_alphanumeric() || c == '_') {
                return (Some(agent.to_string()), message.to_string());
            }
        }
    }
    
    (None, text.to_string())
}

/// Send a response message back to Telegram.
pub async fn send_response(
    bot: Bot,
    chat_id: i64,
    text: &str,
) -> Result<Message, teloxide::RequestError> {
    bot.send_message(ChatId(chat_id), text).await
}
