//! Conversation tracking for TinyVegeta.
#![allow(dead_code)]
//!
//! Handles:
//! - Tracking active conversations
//! - Pending mentions within conversations
//! - Conversation completion detection

use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

/// A conversation tracks messages and pending mentions.
#[derive(Debug, Clone)]
pub struct Conversation {
    /// Unique conversation ID
    pub id: String,

    /// Original sender ID
    pub sender_id: String,

    /// Channel (telegram, cli, etc.)
    pub channel: String,

    /// Original message
    pub original_message: String,

    /// Agent that handled the initial message
    pub primary_agent: Option<String>,

    /// All agents involved in this conversation
    pub participants: Vec<String>,

    /// Pending mentions (agent_id -> message)
    pub pending_mentions: HashMap<String, String>,

    /// When the conversation started
    pub created_at: i64,

    /// Last activity timestamp
    pub updated_at: i64,

    /// Whether the conversation is complete
    pub completed: bool,
}

impl Conversation {
    /// Create a new conversation.
    pub fn new(id: &str, sender_id: &str, channel: &str, message: &str) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as i64;

        Self {
            id: id.to_string(),
            sender_id: sender_id.to_string(),
            channel: channel.to_string(),
            original_message: message.to_string(),
            primary_agent: None,
            participants: Vec::new(),
            pending_mentions: HashMap::new(),
            created_at: now,
            updated_at: now,
            completed: false,
        }
    }

    /// Add a participant agent.
    pub fn add_participant(&mut self, agent_id: &str) {
        if !self.participants.contains(&agent_id.to_string()) {
            self.participants.push(agent_id.to_string());
        }
        self.updated_at = now_timestamp();
    }

    /// Set the primary agent.
    pub fn set_primary_agent(&mut self, agent_id: &str) {
        if self.primary_agent.is_none() {
            self.primary_agent = Some(agent_id.to_string());
        }
        self.add_participant(agent_id);
    }

    /// Add a pending mention.
    pub fn add_pending_mention(&mut self, agent_id: &str, message: &str) {
        self.pending_mentions
            .insert(agent_id.to_string(), message.to_string());
        self.add_participant(agent_id);
        self.updated_at = now_timestamp();
    }

    /// Remove a pending mention (agent has responded).
    pub fn complete_mention(&mut self, agent_id: &str) -> Option<String> {
        let msg = self.pending_mentions.remove(agent_id);
        if msg.is_some() {
            self.updated_at = now_timestamp();
        }
        msg
    }

    /// Check if conversation has pending mentions.
    pub fn has_pending(&self) -> bool {
        !self.pending_mentions.is_empty()
    }

    /// Mark conversation as complete.
    pub fn complete(&mut self) {
        self.completed = true;
        self.updated_at = now_timestamp();
    }

    /// Check if conversation is complete (no pending mentions).
    pub fn is_complete(&self) -> bool {
        self.pending_mentions.is_empty()
    }
}

/// Get current timestamp.
fn now_timestamp() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64
}

/// Conversation manager.
pub struct ConversationManager {
    conversations: HashMap<String, Conversation>,
}

impl ConversationManager {
    /// Create a new conversation manager.
    pub fn new() -> Self {
        Self {
            conversations: HashMap::new(),
        }
    }

    /// Create or get a conversation by ID.
    pub fn get_or_create(&mut self, id: &str) -> &mut Conversation {
        self.conversations
            .entry(id.to_string())
            .or_insert_with(|| Conversation::new(id, "", "", ""))
    }

    /// Get a conversation by ID.
    pub fn get(&self, id: &str) -> Option<&Conversation> {
        self.conversations.get(id)
    }

    /// Get a conversation by ID (mutable).
    pub fn get_mut(&mut self, id: &str) -> Option<&mut Conversation> {
        self.conversations.get_mut(id)
    }

    /// Create a new conversation.
    pub fn create(
        &mut self,
        id: &str,
        sender_id: &str,
        channel: &str,
        message: &str,
    ) -> &mut Conversation {
        self.conversations
            .entry(id.to_string())
            .or_insert_with(|| Conversation::new(id, sender_id, channel, message))
    }

    /// Remove a completed conversation.
    pub fn remove(&mut self, id: &str) -> Option<Conversation> {
        self.conversations.remove(id)
    }

    /// List all active conversations.
    pub fn list_active(&self) -> Vec<&Conversation> {
        self.conversations
            .values()
            .filter(|c| !c.completed)
            .collect()
    }

    /// Cleanup old completed conversations.
    pub fn cleanup(&mut self, max_age_ms: i64) -> usize {
        let now = now_timestamp();
        let mut removed = 0;

        self.conversations.retain(|_id, conv| {
            let should_remove = conv.completed && (now - conv.updated_at) > max_age_ms;
            if should_remove {
                removed += 1;
            }
            !should_remove
        });

        removed
    }
}

impl Default for ConversationManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Conversation state for persistence.
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct ConversationState {
    pub id: String,
    pub sender_id: String,
    pub channel: String,
    pub primary_agent: Option<String>,
    pub participants: Vec<String>,
    pub pending_mentions: HashMap<String, String>,
    pub completed: bool,
    pub created_at: i64,
    pub updated_at: i64,
}

impl From<&Conversation> for ConversationState {
    fn from(conv: &Conversation) -> Self {
        Self {
            id: conv.id.clone(),
            sender_id: conv.sender_id.clone(),
            channel: conv.channel.clone(),
            primary_agent: conv.primary_agent.clone(),
            participants: conv.participants.clone(),
            pending_mentions: conv.pending_mentions.clone(),
            completed: conv.completed,
            created_at: conv.created_at,
            updated_at: conv.updated_at,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_conversation() {
        let mut conv = Conversation::new("conv1", "user123", "telegram", "Hello");

        assert!(!conv.has_pending());

        conv.set_primary_agent("coder");
        assert_eq!(conv.primary_agent, Some("coder".to_string()));

        conv.add_pending_mention("reviewer", "Please review");
        assert!(conv.has_pending());

        conv.complete_mention("reviewer");
        assert!(!conv.has_pending());
    }

    #[test]
    fn test_conversation_manager() {
        let mut mgr = ConversationManager::new();

        let conv = mgr.create("conv1", "user1", "telegram", "Hello");
        assert_eq!(conv.id, "conv1");

        let conv = mgr.get("conv1").unwrap();
        assert_eq!(conv.sender_id, "user1");

        // Cleanup old completed
        mgr.cleanup(60000); // 1 minute
    }
}
