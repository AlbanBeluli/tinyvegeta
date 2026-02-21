//! Error types for TinyVegeta.
#![allow(dead_code)]

use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Configuration error: {0}")]
    Config(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Tmux error: {0}")]
    Tmux(String),

    #[error("Queue error: {0}")]
    Queue(String),

    #[error("Memory error: {0}")]
    Memory(String),

    #[error("Telegram error: {0}")]
    Telegram(String),

    #[error("Provider error: {0}")]
    Provider(String),

    #[error("Web error: {0}")]
    Web(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("{0}")]
    Other(String),
}
