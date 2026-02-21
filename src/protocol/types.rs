//! Message types for agent communication protocol.

use serde::{Deserialize, Serialize};

/// Message type classification.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MessageType {
    /// Request expecting a response
    Request,
    /// Response to a request
    Response,
    /// Broadcast to multiple agents (no response expected)
    Broadcast,
    /// Task delegation from one agent to another
    Delegation,
    /// Status update / notification
    Notification,
    /// Error report
    Error,
}

/// Message priority levels.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum Priority {
    Low = 0,
    Normal = 1,
    High = 2,
    Urgent = 3,
}

impl Default for Priority {
    fn default() -> Self {
        Self::Normal
    }
}

/// Message delivery status.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MessageStatus {
    /// Message created, not yet delivered
    Pending,
    /// Delivered to recipient's mailbox
    Delivered,
    /// Recipient has read the message
    Read,
    /// Recipient is processing
    Processing,
    /// Successfully completed
    Completed,
    /// Failed to process
    Failed,
    /// Expired without delivery
    Expired,
}

/// Core agent message structure.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentMessage {
    /// Message type
    pub message_type: MessageType,
    /// Priority level
    pub priority: Priority,
    /// Current status
    pub status: MessageStatus,
    /// Message subject/topic (optional)
    pub subject: Option<String>,
    /// Message body content
    pub body: String,
    /// Attached data (JSON string for flexibility)
    pub payload: Option<String>,
    /// Tags for categorization
    pub tags: Vec<String>,
    /// Error message if status is Failed
    pub error: Option<String>,
}

impl AgentMessage {
    /// Create a new request message.
    pub fn request(body: impl Into<String>) -> Self {
        Self {
            message_type: MessageType::Request,
            priority: Priority::Normal,
            status: MessageStatus::Pending,
            subject: None,
            body: body.into(),
            payload: None,
            tags: Vec::new(),
            error: None,
        }
    }

    /// Create a response message.
    pub fn response(body: impl Into<String>) -> Self {
        Self {
            message_type: MessageType::Response,
            priority: Priority::Normal,
            status: MessageStatus::Pending,
            subject: None,
            body: body.into(),
            payload: None,
            tags: Vec::new(),
            error: None,
        }
    }

    /// Create a broadcast message.
    pub fn broadcast(body: impl Into<String>) -> Self {
        Self {
            message_type: MessageType::Broadcast,
            priority: Priority::Normal,
            status: MessageStatus::Pending,
            subject: None,
            body: body.into(),
            payload: None,
            tags: Vec::new(),
            error: None,
        }
    }

    /// Create a delegation message.
    pub fn delegation(body: impl Into<String>) -> Self {
        Self {
            message_type: MessageType::Delegation,
            priority: Priority::High,
            status: MessageStatus::Pending,
            subject: None,
            body: body.into(),
            payload: None,
            tags: Vec::new(),
            error: None,
        }
    }

    /// Create a notification message.
    pub fn notification(body: impl Into<String>) -> Self {
        Self {
            message_type: MessageType::Notification,
            priority: Priority::Low,
            status: MessageStatus::Pending,
            subject: None,
            body: body.into(),
            payload: None,
            tags: Vec::new(),
            error: None,
        }
    }

    /// Create an error message.
    pub fn error(body: impl Into<String>, error: impl Into<String>) -> Self {
        Self {
            message_type: MessageType::Error,
            priority: Priority::High,
            status: MessageStatus::Failed,
            subject: None,
            body: body.into(),
            payload: None,
            tags: Vec::new(),
            error: Some(error.into()),
        }
    }

    /// Set the subject.
    pub fn with_subject(mut self, subject: impl Into<String>) -> Self {
        self.subject = Some(subject.into());
        self
    }

    /// Set the priority.
    pub fn with_priority(mut self, priority: Priority) -> Self {
        self.priority = priority;
        self
    }

    /// Set the payload (JSON data).
    pub fn with_payload(mut self, payload: impl Into<String>) -> Self {
        self.payload = Some(payload.into());
        self
    }

    /// Add a tag.
    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
        self
    }

    /// Mark as delivered.
    pub fn mark_delivered(&mut self) {
        self.status = MessageStatus::Delivered;
    }

    /// Mark as read.
    pub fn mark_read(&mut self) {
        self.status = MessageStatus::Read;
    }

    /// Mark as processing.
    pub fn mark_processing(&mut self) {
        self.status = MessageStatus::Processing;
    }

    /// Mark as completed.
    pub fn mark_completed(&mut self) {
        self.status = MessageStatus::Completed;
    }

    /// Mark as failed.
    pub fn mark_failed(&mut self, error: impl Into<String>) {
        self.status = MessageStatus::Failed;
        self.error = Some(error.into());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_creation() {
        let msg = AgentMessage::request("Fix the bug").with_subject("Bug Report");
        assert_eq!(msg.message_type, MessageType::Request);
        assert_eq!(msg.subject, Some("Bug Report".to_string()));
        assert_eq!(msg.body, "Fix the bug");
    }

    #[test]
    fn test_message_status_transitions() {
        let mut msg = AgentMessage::delegation("Review PR");
        assert_eq!(msg.status, MessageStatus::Pending);
        
        msg.mark_delivered();
        assert_eq!(msg.status, MessageStatus::Delivered);
        
        msg.mark_read();
        assert_eq!(msg.status, MessageStatus::Read);
        
        msg.mark_processing();
        assert_eq!(msg.status, MessageStatus::Processing);
        
        msg.mark_completed();
        assert_eq!(msg.status, MessageStatus::Completed);
    }

    #[test]
    fn test_priority_ordering() {
        assert!(Priority::Urgent > Priority::High);
        assert!(Priority::High > Priority::Normal);
        assert!(Priority::Normal > Priority::Low);
    }
}