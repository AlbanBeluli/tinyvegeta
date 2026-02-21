//! Core module - Queue, routing, and conversation management.
//!
//! This module contains the heart of TinyVegeta's message processing:
//! - File-based message queue
//! - Agent and team routing
//! - Conversation tracking

pub mod conversation;
pub mod queue;
pub mod routing;

pub use queue::{MessageData, Queue};
