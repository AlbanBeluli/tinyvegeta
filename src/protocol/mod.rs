//! Agent Communication Protocol for TinyVegeta.
//!
//! This module defines a structured protocol for inter-agent communication:
//! - Message envelopes with correlation IDs
//! - Typed message types (request, response, broadcast, delegation)
//! - Agent mailboxes with persistence
//! - Communication audit trail

pub mod envelope;
pub mod mailbox;
pub mod types;

pub use envelope::{Envelope, EnvelopeBuilder};
pub use mailbox::{AgentMailbox, MailboxStore};
pub use types::{MessageType, Priority, MessageStatus, AgentMessage};