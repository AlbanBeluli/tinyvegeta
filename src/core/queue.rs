//! File-based message queue for TinyVegeta.
#![allow(dead_code)]
//!
//! Queue structure:
//! - incoming/  : New messages arrive here
//! - processing/: Messages being processed
//! - outgoing/  : Ready to send to channel

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::config::get_home_dir;
use crate::error::Error;

/// Queue directory names
pub const QUEUE_INCOMING: &str = "incoming";
pub const QUEUE_PROCESSING: &str = "processing";
pub const QUEUE_OUTGOING: &str = "outgoing";

/// Get the queue base directory.
pub fn get_queue_dir() -> Result<PathBuf, Error> {
    Ok(get_home_dir()?.join("queue"))
}

/// Get a specific queue subdirectory.
pub fn get_queue_subdir(subdir: &str) -> Result<PathBuf, Error> {
    Ok(get_queue_dir()?.join(subdir))
}

/// Ensure all queue directories exist.
pub fn ensure_queue_dirs() -> Result<(), Error> {
    for subdir in [QUEUE_INCOMING, QUEUE_PROCESSING, QUEUE_OUTGOING] {
        let dir = get_queue_subdir(subdir)?;
        if !dir.exists() {
            fs::create_dir_all(&dir)?;
            tracing::debug!("Created queue directory: {}", dir.display());
        }
    }
    Ok(())
}

/// Message data structure.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct MessageData {
    /// Channel (e.g., "telegram", "cli")
    pub channel: String,

    /// Sender name
    pub sender: String,

    /// Sender ID (channel-specific)
    pub sender_id: String,

    /// Message content
    pub message: String,

    /// Unix timestamp
    pub timestamp: i64,

    /// Message ID from channel (optional)
    pub message_id: Option<i64>,

    /// Target agent (for routing)
    pub agent: Option<String>,

    /// Conversation ID (for tracking)
    pub conversation_id: Option<String>,

    /// Files attached (paths)
    pub files: Option<Vec<String>>,

    /// Response target (where to send the reply)
    pub response_channel: Option<String>,
    pub response_chat_id: Option<i64>,
    pub response_message_id: Option<i64>,
}

impl MessageData {
    /// Create a new message with current timestamp.
    pub fn new(channel: &str, sender: &str, sender_id: &str, message: &str) -> Self {
        Self {
            channel: channel.to_string(),
            sender: sender.to_string(),
            sender_id: sender_id.to_string(),
            message: message.to_string(),
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_millis() as i64,
            message_id: None,
            agent: None,
            conversation_id: None,
            files: None,
            response_channel: None,
            response_chat_id: None,
            response_message_id: None,
        }
    }
}

/// Queue file wrapper.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct QueueFile {
    /// Unique ID (ULID)
    pub id: String,

    /// Message data
    pub data: MessageData,

    /// When created (unix timestamp)
    pub created_at: i64,
}

impl QueueFile {
    /// Create a new queue file.
    pub fn new(data: MessageData) -> Self {
        let id = ulid::Ulid::new().to_string();
        let created_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as i64;

        Self {
            id,
            data,
            created_at,
        }
    }
}

/// Queue operations.
pub struct Queue;

impl Queue {
    /// Enqueue a message to the incoming queue.
    pub fn enqueue(data: MessageData) -> Result<String, Error> {
        ensure_queue_dirs()?;

        let queue_file = QueueFile::new(data);
        let id = queue_file.id.clone();

        // Write to incoming directory
        let incoming_dir = get_queue_subdir(QUEUE_INCOMING)?;
        let file_path = incoming_dir.join(format!("{}.json", id));

        let content = serde_json::to_string_pretty(&queue_file)?;
        fs::write(&file_path, content)?;

        tracing::debug!("Enqueued message {} to incoming", id);
        Ok(id)
    }

    /// Move a message to processing.
    pub fn mark_processing(id: &str) -> Result<(), Error> {
        let incoming_dir = get_queue_subdir(QUEUE_INCOMING)?;
        let processing_dir = get_queue_subdir(QUEUE_PROCESSING)?;

        let src = incoming_dir.join(format!("{}.json", id));
        let dst = processing_dir.join(format!("{}.json", id));

        if !src.exists() {
            return Err(Error::Queue(format!(
                "Message {} not found in incoming",
                id
            )));
        }

        fs::rename(&src, &dst)?;
        tracing::debug!("Moved message {} to processing", id);
        Ok(())
    }

    /// Move a message to outgoing (ready to send).
    pub fn mark_outgoing(id: &str) -> Result<(), Error> {
        let processing_dir = get_queue_subdir(QUEUE_PROCESSING)?;
        let outgoing_dir = get_queue_subdir(QUEUE_OUTGOING)?;

        let src = processing_dir.join(format!("{}.json", id));
        let dst = outgoing_dir.join(format!("{}.json", id));

        if !src.exists() {
            return Err(Error::Queue(format!(
                "Message {} not found in processing",
                id
            )));
        }

        fs::rename(&src, &dst)?;
        tracing::debug!("Moved message {} to outgoing", id);
        Ok(())
    }

    /// Complete a message (remove from queue).
    pub fn complete(id: &str) -> Result<(), Error> {
        let outgoing_dir = get_queue_subdir(QUEUE_OUTGOING)?;
        let file_path = outgoing_dir.join(format!("{}.json", id));

        if file_path.exists() {
            fs::remove_file(&file_path)?;
            tracing::debug!("Completed and removed message {}", id);
        }

        Ok(())
    }

    /// Remove a message from incoming queue directly.
    pub fn remove_incoming(id: &str) -> Result<(), Error> {
        let incoming_dir = get_queue_subdir(QUEUE_INCOMING)?;
        let file_path = incoming_dir.join(format!("{}.json", id));

        if file_path.exists() {
            fs::remove_file(&file_path)?;
            tracing::debug!("Removed message {} from incoming", id);
        }

        Ok(())
    }

    /// Get a message by ID from any queue.
    pub fn get(id: &str) -> Result<Option<QueueFile>, Error> {
        for subdir in [QUEUE_INCOMING, QUEUE_PROCESSING, QUEUE_OUTGOING] {
            let dir = get_queue_subdir(subdir)?;
            let file_path = dir.join(format!("{}.json", id));

            if file_path.exists() {
                let content = fs::read_to_string(&file_path)?;
                let queue_file: QueueFile = serde_json::from_str(&content)?;
                return Ok(Some(queue_file));
            }
        }

        Ok(None)
    }

    /// List all messages in a queue directory.
    pub fn list(subdir: &str) -> Result<Vec<QueueFile>, Error> {
        let dir = get_queue_subdir(subdir)?;

        if !dir.exists() {
            return Ok(vec![]);
        }

        let mut files = Vec::new();

        for entry in fs::read_dir(&dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.extension().map_or(false, |ext| ext == "json") {
                if let Ok(content) = fs::read_to_string(&path) {
                    if let Ok(queue_file) = serde_json::from_str::<QueueFile>(&content) {
                        files.push(queue_file);
                    }
                }
            }
        }

        // Sort by created_at
        files.sort_by_key(|f| f.created_at);

        Ok(files)
    }

    /// Get incoming messages.
    pub fn incoming() -> Result<Vec<QueueFile>, Error> {
        Self::list(QUEUE_INCOMING)
    }

    /// Get processing messages.
    pub fn processing() -> Result<Vec<QueueFile>, Error> {
        Self::list(QUEUE_PROCESSING)
    }

    /// Get outgoing messages.
    pub fn outgoing() -> Result<Vec<QueueFile>, Error> {
        Self::list(QUEUE_OUTGOING)
    }

    /// Get queue statistics.
    pub fn stats() -> Result<QueueStats, Error> {
        ensure_queue_dirs()?;

        let incoming = Self::incoming()?.len();
        let processing = Self::processing()?.len();
        let outgoing = Self::outgoing()?.len();

        Ok(QueueStats {
            incoming,
            processing,
            outgoing,
            total: incoming + processing + outgoing,
        })
    }

    /// Recover orphaned messages from processing on startup.
    pub fn recover_orphaned() -> Result<usize, Error> {
        ensure_queue_dirs()?;

        let processing_dir = get_queue_subdir(QUEUE_PROCESSING)?;
        let incoming_dir = get_queue_subdir(QUEUE_INCOMING)?;

        let mut recovered = 0;

        if processing_dir.exists() {
            for entry in fs::read_dir(&processing_dir)? {
                let entry = entry?;
                let path = entry.path();

                if path.extension().map_or(false, |ext| ext == "json") {
                    let filename = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");

                    let dst = incoming_dir.join(path.file_name().unwrap());

                    if fs::rename(&path, &dst).is_ok() {
                        tracing::info!("Recovered orphaned message: {}", filename);
                        recovered += 1;
                    }
                }
            }
        }

        Ok(recovered)
    }
}

/// Queue statistics.
#[derive(Debug, Clone)]
pub struct QueueStats {
    pub incoming: usize,
    pub processing: usize,
    pub outgoing: usize,
    pub total: usize,
}

impl std::fmt::Display for QueueStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Queue Stats:\n")?;
        write!(f, "  Incoming:  {}\n", self.incoming)?;
        write!(f, "  Processing: {}\n", self.processing)?;
        write!(f, "  Outgoing:  {}\n", self.outgoing)?;
        write!(f, "  Total:     {}", self.total)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_data() {
        let msg = MessageData::new("telegram", "Alice", "12345", "Hello world");

        assert_eq!(msg.channel, "telegram");
        assert_eq!(msg.sender, "Alice");
        assert_eq!(msg.sender_id, "12345");
        assert_eq!(msg.message, "Hello world");
        assert!(msg.timestamp > 0);
    }

    #[test]
    fn test_queue_file() {
        let msg = MessageData::new("telegram", "Alice", "12345", "Hello");
        let qf = QueueFile::new(msg);

        assert!(!qf.id.is_empty());
        assert!(qf.created_at > 0);
    }
}
