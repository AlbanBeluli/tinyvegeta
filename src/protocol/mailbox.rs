//! Agent mailboxes with persistence for inter-agent communication.

use std::collections::HashMap;
use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};

use super::envelope::Envelope;
use super::types::{MessageStatus, Priority};

/// Maximum messages per mailbox before pruning.
const MAX_MAILBOX_SIZE: usize = 1000;

/// Mailbox directory name.
const MAILBOX_DIR: &str = "mailboxes";

/// An agent's mailbox containing received messages.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentMailbox {
    /// Agent ID owning this mailbox.
    pub agent_id: String,
    /// Inbox (received messages).
    pub inbox: Vec<Envelope>,
    /// Outbox (sent messages awaiting delivery).
    pub outbox: Vec<Envelope>,
    /// Archive (processed messages).
    pub archive: Vec<Envelope>,
    /// Total messages received.
    pub total_received: u64,
    /// Total messages sent.
    pub total_sent: u64,
    /// Last activity timestamp.
    pub last_activity: i64,
}

impl AgentMailbox {
    /// Create a new mailbox for an agent.
    pub fn new(agent_id: impl Into<String>) -> Self {
        Self {
            agent_id: agent_id.into(),
            inbox: Vec::new(),
            outbox: Vec::new(),
            archive: Vec::new(),
            total_received: 0,
            total_sent: 0,
            last_activity: current_timestamp(),
        }
    }

    /// Deliver an envelope to this mailbox.
    pub fn deliver(&mut self, mut envelope: Envelope) {
        envelope.message.mark_delivered();
        self.inbox.push(envelope);
        self.total_received += 1;
        self.last_activity = current_timestamp();
        self.prune_if_needed();
    }

    /// Queue an envelope for sending.
    pub fn queue_outgoing(&mut self, envelope: Envelope) {
        self.outbox.push(envelope);
        self.total_sent += 1;
        self.last_activity = current_timestamp();
    }

    /// Get unread messages (pending or delivered status).
    pub fn unread(&self) -> Vec<&Envelope> {
        self.inbox
            .iter()
            .filter(|e| {
                matches!(
                    e.message.status,
                    MessageStatus::Pending | MessageStatus::Delivered
                )
            })
            .collect()
    }

    /// Get messages by priority.
    pub fn by_priority(&self, priority: Priority) -> Vec<&Envelope> {
        self.inbox
            .iter()
            .filter(|e| e.message.priority == priority)
            .collect()
    }

    /// Get messages from a specific sender.
    pub fn from_agent(&self, agent_id: &str) -> Vec<&Envelope> {
        self.inbox
            .iter()
            .filter(|e| e.from_agent == agent_id)
            .collect()
    }

    /// Mark a message as read.
    pub fn mark_read(&mut self, message_id: &str) -> bool {
        if let Some(envelope) = self.inbox.iter_mut().find(|e| e.id == message_id) {
            envelope.message.mark_read();
            self.last_activity = current_timestamp();
            true
        } else {
            false
        }
    }

    /// Mark a message as processing.
    pub fn mark_processing(&mut self, message_id: &str) -> bool {
        if let Some(envelope) = self.inbox.iter_mut().find(|e| e.id == message_id) {
            envelope.message.mark_processing();
            self.last_activity = current_timestamp();
            true
        } else {
            false
        }
    }

    /// Complete a message and archive it.
    pub fn complete(&mut self, message_id: &str) -> bool {
        if let Some(pos) = self.inbox.iter().position(|e| e.id == message_id) {
            let mut envelope = self.inbox.remove(pos);
            envelope.message.mark_completed();
            self.archive.push(envelope);
            self.last_activity = current_timestamp();
            true
        } else {
            false
        }
    }

    /// Fail a message and archive it.
    pub fn fail(&mut self, message_id: &str, error: &str) -> bool {
        if let Some(pos) = self.inbox.iter().position(|e| e.id == message_id) {
            let mut envelope = self.inbox.remove(pos);
            envelope.message.mark_failed(error);
            self.archive.push(envelope);
            self.last_activity = current_timestamp();
            true
        } else {
            false
        }
    }

    /// Remove expired messages.
    pub fn purge_expired(&mut self) -> usize {
        let before = self.inbox.len();
        self.inbox.retain(|e| !e.is_expired());
        self.archive.retain(|e| !e.is_expired());
        before - self.inbox.len()
    }

    /// Get the next pending message (highest priority first).
    pub fn next_pending(&self) -> Option<&Envelope> {
        self.inbox
            .iter()
            .filter(|e| matches!(e.message.status, MessageStatus::Pending | MessageStatus::Delivered))
            .max_by(|a, b| a.message.priority.cmp(&b.message.priority))
    }

    /// Prune old messages if mailbox exceeds max size.
    fn prune_if_needed(&mut self) {
        if self.inbox.len() > MAX_MAILBOX_SIZE {
            // Sort by priority (keep high priority) and age (keep newer)
            self.inbox.sort_by(|a, b| {
                a.message.priority.cmp(&b.message.priority)
                    .then_with(|| b.created_at.cmp(&a.created_at))
            });
            self.inbox.truncate(MAX_MAILBOX_SIZE);
        }
    }

    /// Get mailbox statistics.
    pub fn stats(&self) -> MailboxStats {
        MailboxStats {
            agent_id: self.agent_id.clone(),
            inbox_count: self.inbox.len(),
            outbox_count: self.outbox.len(),
            archive_count: self.archive.len(),
            unread_count: self.unread().len(),
            total_received: self.total_received,
            total_sent: self.total_sent,
            last_activity: self.last_activity,
        }
    }
}

/// Mailbox statistics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MailboxStats {
    pub agent_id: String,
    pub inbox_count: usize,
    pub outbox_count: usize,
    pub archive_count: usize,
    pub unread_count: usize,
    pub total_received: u64,
    pub total_sent: u64,
    pub last_activity: i64,
}

/// Store for all agent mailboxes with persistence.
#[derive(Debug)]
pub struct MailboxStore {
    /// Base path for mailbox storage.
    base_path: PathBuf,
    /// In-memory cache of mailboxes.
    cache: Arc<Mutex<HashMap<String, AgentMailbox>>>,
}

impl MailboxStore {
    /// Create a new mailbox store.
    pub fn new(base_path: impl AsRef<Path>) -> Self {
        let base_path = base_path.as_ref().join(MAILBOX_DIR);
        let _ = fs::create_dir_all(&base_path);

        Self {
            base_path,
            cache: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Get or create a mailbox for an agent.
    pub fn get_mailbox(&self, agent_id: &str) -> AgentMailbox {
        // Check cache first
        {
            let cache = self.cache.lock().unwrap();
            if let Some(mailbox) = cache.get(agent_id) {
                return mailbox.clone();
            }
        }

        // Try to load from disk
        if let Some(mailbox) = self.load_mailbox(agent_id) {
            let mut cache = self.cache.lock().unwrap();
            cache.insert(agent_id.to_string(), mailbox.clone());
            return mailbox;
        }

        // Create new mailbox
        let mailbox = AgentMailbox::new(agent_id);
        let mut cache = self.cache.lock().unwrap();
        cache.insert(agent_id.to_string(), mailbox.clone());
        mailbox
    }

    /// Deliver an envelope to an agent's mailbox.
    pub fn deliver(&self, agent_id: &str, envelope: Envelope) {
        let mailbox_path = self.mailbox_path(agent_id);
        let mut mailbox = self.get_mailbox(agent_id);
        mailbox.deliver(envelope);
        self.save_mailbox(&mailbox);
        
        let mut cache = self.cache.lock().unwrap();
        cache.insert(agent_id.to_string(), mailbox);
    }

    /// Queue an outgoing message.
    pub fn queue_outgoing(&self, agent_id: &str, envelope: Envelope) {
        let mut mailbox = self.get_mailbox(agent_id);
        mailbox.queue_outgoing(envelope);
        self.save_mailbox(&mailbox);
        
        let mut cache = self.cache.lock().unwrap();
        cache.insert(agent_id.to_string(), mailbox);
    }

    /// Update a mailbox after processing.
    pub fn update(&self, mailbox: &AgentMailbox) {
        self.save_mailbox(mailbox);
        let mut cache = self.cache.lock().unwrap();
        cache.insert(mailbox.agent_id.clone(), mailbox.clone());
    }

    /// Get all pending outgoing messages across all mailboxes.
    pub fn pending_outgoing(&self) -> Vec<(String, Envelope)> {
        let cache = self.cache.lock().unwrap();
        let mut result = Vec::new();
        
        for (agent_id, mailbox) in cache.iter() {
            for envelope in mailbox.outbox.iter() {
                if !envelope.is_exhausted() {
                    result.push((agent_id.clone(), envelope.clone()));
                }
            }
        }
        
        result
    }

    /// Get statistics for all mailboxes.
    pub fn all_stats(&self) -> Vec<MailboxStats> {
        let cache = self.cache.lock().unwrap();
        cache.values().map(|m| m.stats()).collect()
    }

    /// Purge expired messages from all mailboxes.
    pub fn purge_all_expired(&self) -> usize {
        let mut total = 0;
        let mut cache = self.cache.lock().unwrap();
        
        for mailbox in cache.values_mut() {
            total += mailbox.purge_expired();
        }
        
        total
    }

    /// Get the path for a mailbox file.
    fn mailbox_path(&self, agent_id: &str) -> PathBuf {
        self.base_path.join(format!("{}.jsonl", agent_id))
    }

    /// Load a mailbox from disk.
    fn load_mailbox(&self, agent_id: &str) -> Option<AgentMailbox> {
        let path = self.mailbox_path(agent_id);
        if !path.exists() {
            return None;
        }

        let file = File::open(&path).ok()?;
        let reader = BufReader::new(file);
        
        // Read the last line (most recent state)
        let last_line = reader.lines().last()?.ok()?;
        serde_json::from_str(&last_line).ok()
    }

    /// Save a mailbox to disk (append-only log).
    fn save_mailbox(&self, mailbox: &AgentMailbox) {
        let path = self.mailbox_path(&mailbox.agent_id);
        
        if let Ok(mut file) = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
        {
            let json = serde_json::to_string(mailbox).unwrap_or_default();
            let _ = writeln!(file, "{}", json);
        }
    }

    /// Compact mailbox storage (rewrite to single state).
    pub fn compact(&self, agent_id: &str) {
        let mailbox = self.get_mailbox(agent_id);
        let path = self.mailbox_path(agent_id);
        
        if let Ok(mut file) = OpenOptions::new()
            .write(true)
            .truncate(true)
            .create(true)
            .open(&path)
        {
            let json = serde_json::to_string(&mailbox).unwrap_or_default();
            let _ = writeln!(file, "{}", json);
        }
    }
}

fn current_timestamp() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::types::AgentMessage;

    #[test]
    fn test_mailbox_creation() {
        let mailbox = AgentMailbox::new("assistant");
        assert_eq!(mailbox.agent_id, "assistant");
        assert!(mailbox.inbox.is_empty());
        assert!(mailbox.outbox.is_empty());
    }

    #[test]
    fn test_deliver_message() {
        let mut mailbox = AgentMailbox::new("coder");
        let envelope = Envelope::new("assistant", "coder", AgentMessage::request("Fix bug"));
        
        mailbox.deliver(envelope);
        
        assert_eq!(mailbox.inbox.len(), 1);
        assert_eq!(mailbox.total_received, 1);
        assert_eq!(mailbox.unread().len(), 1);
    }

    #[test]
    fn test_message_lifecycle() {
        let mut mailbox = AgentMailbox::new("coder");
        let envelope = Envelope::new("assistant", "coder", AgentMessage::request("Fix bug"));
        let msg_id = envelope.id.clone();
        
        mailbox.deliver(envelope);
        assert!(mailbox.mark_read(&msg_id));
        assert!(mailbox.mark_processing(&msg_id));
        assert!(mailbox.complete(&msg_id));
        
        assert!(mailbox.inbox.is_empty());
        assert_eq!(mailbox.archive.len(), 1);
    }

    #[test]
    fn test_priority_ordering() {
        let mut mailbox = AgentMailbox::new("coder");
        
        let low = Envelope::new("assistant", "coder", 
            AgentMessage::request("Low priority").with_priority(Priority::Low));
        let high = Envelope::new("assistant", "coder",
            AgentMessage::request("High priority").with_priority(Priority::High));
        let urgent = Envelope::new("assistant", "coder",
            AgentMessage::request("Urgent").with_priority(Priority::Urgent));
        
        mailbox.deliver(low);
        mailbox.deliver(high);
        mailbox.deliver(urgent);
        
        let next = mailbox.next_pending().unwrap();
        assert_eq!(next.message.priority, Priority::Urgent);
    }

    #[test]
    fn test_mailbox_store() {
        let temp_dir = tempfile::tempdir().unwrap();
        let store = MailboxStore::new(temp_dir.path());
        
        let envelope = Envelope::new("assistant", "coder", AgentMessage::request("Fix bug"));
        store.deliver("coder", envelope);
        
        let mailbox = store.get_mailbox("coder");
        assert_eq!(mailbox.inbox.len(), 1);
    }
}