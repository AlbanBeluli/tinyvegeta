//! Message envelopes with correlation IDs for tracking agent communication.

use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

use super::types::{AgentMessage, MessageStatus, Priority};

/// Message envelope wrapping AgentMessage with metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Envelope {
    /// Unique message ID (ULID)
    pub id: String,
    /// Correlation ID for request/response chains
    pub correlation_id: Option<String>,
    /// Reply-to message ID (for responses)
    pub reply_to: Option<String>,
    /// Sender agent ID
    pub from_agent: String,
    /// Recipient agent ID (None for broadcast)
    pub to_agent: Option<String>,
    /// Recipient agents (for multi-cast)
    pub to_agents: Vec<String>,
    /// Team ID context (optional)
    pub team_id: Option<String>,
    /// The message payload
    pub message: AgentMessage,
    /// Creation timestamp (unix ms)
    pub created_at: i64,
    /// Last updated timestamp (unix ms)
    pub updated_at: i64,
    /// Expiration timestamp (unix ms, optional)
    pub expires_at: Option<i64>,
    /// Number of delivery attempts
    pub delivery_attempts: u32,
    /// Maximum delivery attempts before marking failed
    pub max_attempts: u32,
}

impl Envelope {
    /// Create a new envelope from an agent to another agent.
    pub fn new(from_agent: impl Into<String>, to_agent: impl Into<String>, message: AgentMessage) -> Self {
        let now = current_timestamp();
        Self {
            id: generate_id(),
            correlation_id: None,
            reply_to: None,
            from_agent: from_agent.into(),
            to_agent: Some(to_agent.into()),
            to_agents: Vec::new(),
            team_id: None,
            message,
            created_at: now,
            updated_at: now,
            expires_at: None,
            delivery_attempts: 0,
            max_attempts: 3,
        }
    }

    /// Create a broadcast envelope to multiple agents.
    pub fn broadcast(from_agent: impl Into<String>, to_agents: Vec<String>, message: AgentMessage) -> Self {
        let now = current_timestamp();
        Self {
            id: generate_id(),
            correlation_id: None,
            reply_to: None,
            from_agent: from_agent.into(),
            to_agent: None,
            to_agents,
            team_id: None,
            message,
            created_at: now,
            updated_at: now,
            expires_at: None,
            delivery_attempts: 0,
            max_attempts: 3,
        }
    }

    /// Create a team-scoped envelope.
    pub fn to_team(from_agent: impl Into<String>, team_id: impl Into<String>, message: AgentMessage) -> Self {
        let now = current_timestamp();
        Self {
            id: generate_id(),
            correlation_id: None,
            reply_to: None,
            from_agent: from_agent.into(),
            to_agent: None,
            to_agents: Vec::new(),
            team_id: Some(team_id.into()),
            message,
            created_at: now,
            updated_at: now,
            expires_at: None,
            delivery_attempts: 0,
            max_attempts: 3,
        }
    }

    /// Set correlation ID for request/response tracking.
    pub fn with_correlation_id(mut self, id: impl Into<String>) -> Self {
        self.correlation_id = Some(id.into());
        self
    }

    /// Set reply-to for response chains.
    pub fn with_reply_to(mut self, id: impl Into<String>) -> Self {
        self.reply_to = Some(id.into());
        self
    }

    /// Set expiration time (seconds from now).
    pub fn with_ttl(mut self, ttl_seconds: u64) -> Self {
        self.expires_at = Some(current_timestamp() + (ttl_seconds as i64 * 1000));
        self
    }

    /// Set max delivery attempts.
    pub fn with_max_attempts(mut self, max: u32) -> Self {
        self.max_attempts = max;
        self
    }

    /// Check if envelope has expired.
    pub fn is_expired(&self) -> bool {
        if let Some(expires) = self.expires_at {
            current_timestamp() > expires
        } else {
            false
        }
    }

    /// Check if delivery attempts exhausted.
    pub fn is_exhausted(&self) -> bool {
        self.delivery_attempts >= self.max_attempts
    }

    /// Increment delivery attempt counter.
    pub fn increment_attempt(&mut self) {
        self.delivery_attempts += 1;
        self.updated_at = current_timestamp();
    }

    /// Create a response envelope.
    pub fn create_response(&self, from_agent: impl Into<String>, response_message: AgentMessage) -> Self {
        Self {
            id: generate_id(),
            correlation_id: self.correlation_id.clone(),
            reply_to: Some(self.id.clone()),
            from_agent: from_agent.into(),
            to_agent: Some(self.from_agent.clone()),
            to_agents: Vec::new(),
            team_id: self.team_id.clone(),
            message: response_message,
            created_at: current_timestamp(),
            updated_at: current_timestamp(),
            expires_at: None,
            delivery_attempts: 0,
            max_attempts: 3,
        }
    }

    /// Get all recipient agents.
    pub fn recipients(&self) -> Vec<String> {
        if let Some(to) = &self.to_agent {
            vec![to.clone()]
        } else {
            self.to_agents.clone()
        }
    }

    /// Check if this envelope is addressed to a specific agent.
    pub fn is_for(&self, agent_id: &str) -> bool {
        self.to_agent.as_deref() == Some(agent_id) || self.to_agents.iter().any(|a| a == agent_id)
    }
}

/// Builder for creating envelopes with fluent API.
pub struct EnvelopeBuilder {
    from_agent: String,
    to_agent: Option<String>,
    to_agents: Vec<String>,
    team_id: Option<String>,
    message: Option<AgentMessage>,
    correlation_id: Option<String>,
    reply_to: Option<String>,
    ttl_seconds: Option<u64>,
    max_attempts: u32,
    priority: Priority,
}

impl EnvelopeBuilder {
    /// Start building an envelope from an agent.
    pub fn from(agent_id: impl Into<String>) -> Self {
        Self {
            from_agent: agent_id.into(),
            to_agent: None,
            to_agents: Vec::new(),
            team_id: None,
            message: None,
            correlation_id: None,
            reply_to: None,
            ttl_seconds: None,
            max_attempts: 3,
            priority: Priority::Normal,
        }
    }

    /// Address to a single agent.
    pub fn to(mut self, agent_id: impl Into<String>) -> Self {
        self.to_agent = Some(agent_id.into());
        self
    }

    /// Address to multiple agents.
    pub fn to_many(mut self, agent_ids: Vec<String>) -> Self {
        self.to_agents = agent_ids;
        self
    }

    /// Address to a team.
    pub fn to_team(mut self, team_id: impl Into<String>) -> Self {
        self.team_id = Some(team_id.into());
        self
    }

    /// Set the message.
    pub fn message(mut self, message: AgentMessage) -> Self {
        self.priority = message.priority.clone();
        self.message = Some(message);
        self
    }

    /// Set correlation ID.
    pub fn correlation_id(mut self, id: impl Into<String>) -> Self {
        self.correlation_id = Some(id.into());
        self
    }

    /// Set reply-to.
    pub fn reply_to(mut self, id: impl Into<String>) -> Self {
        self.reply_to = Some(id.into());
        self
    }

    /// Set TTL.
    pub fn ttl(mut self, seconds: u64) -> Self {
        self.ttl_seconds = Some(seconds);
        self
    }

    /// Set max attempts.
    pub fn max_attempts(mut self, max: u32) -> Self {
        self.max_attempts = max;
        self
    }

    /// Build the envelope.
    pub fn build(self) -> Result<Envelope, &'static str> {
        let message = self.message.ok_or("Message is required")?;

        let now = current_timestamp();
        let mut envelope = Envelope {
            id: generate_id(),
            correlation_id: self.correlation_id,
            reply_to: self.reply_to,
            from_agent: self.from_agent,
            to_agent: self.to_agent,
            to_agents: self.to_agents,
            team_id: self.team_id,
            message,
            created_at: now,
            updated_at: now,
            expires_at: self.ttl_seconds.map(|ttl| now + (ttl as i64 * 1000)),
            delivery_attempts: 0,
            max_attempts: self.max_attempts,
        };

        // Mark as expired if already past TTL
        if envelope.is_expired() {
            envelope.message.status = MessageStatus::Expired;
        }

        Ok(envelope)
    }
}

fn generate_id() -> String {
    ulid::Ulid::new().to_string()
}

fn current_timestamp() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::types::MessageType;

    #[test]
    fn test_envelope_creation() {
        let msg = AgentMessage::request("Fix the bug");
        let envelope = Envelope::new("assistant", "coder", msg);

        assert_eq!(envelope.from_agent, "assistant");
        assert_eq!(envelope.to_agent, Some("coder".to_string()));
        assert!(!envelope.id.is_empty());
        assert!(envelope.correlation_id.is_none());
    }

    #[test]
    fn test_envelope_builder() {
        let envelope = EnvelopeBuilder::from("assistant")
            .to("coder")
            .message(AgentMessage::delegation("Review this PR"))
            .ttl(3600)
            .max_attempts(5)
            .build()
            .unwrap();

        assert_eq!(envelope.from_agent, "assistant");
        assert_eq!(envelope.to_agent, Some("coder".to_string()));
        assert!(envelope.expires_at.is_some());
        assert_eq!(envelope.max_attempts, 5);
    }

    #[test]
    fn test_response_creation() {
        let request = Envelope::new("assistant", "coder", AgentMessage::request("Fix bug"));
        let response = request.create_response("coder", AgentMessage::response("Bug fixed"));

        assert_eq!(response.reply_to, Some(request.id.clone()));
        assert_eq!(response.to_agent, Some("assistant".to_string()));
        assert_eq!(response.from_agent, "coder");
    }

    #[test]
    fn test_expiration() {
        let envelope = EnvelopeBuilder::from("assistant")
            .to("coder")
            .message(AgentMessage::request("Urgent"))
            .ttl(0) // Already expired
            .build()
            .unwrap();

        assert!(envelope.is_expired());
    }

    #[test]
    fn test_broadcast() {
        let envelope = Envelope::broadcast(
            "assistant",
            vec!["coder".to_string(), "reviewer".to_string()],
            AgentMessage::broadcast("Team meeting"),
        );

        assert!(envelope.to_agent.is_none());
        assert_eq!(envelope.to_agents.len(), 2);
        assert!(envelope.is_for("coder"));
        assert!(envelope.is_for("reviewer"));
    }
}