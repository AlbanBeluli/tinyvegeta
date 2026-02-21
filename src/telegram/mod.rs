//! Telegram bot integration.

pub mod pairing;
pub mod commands;
pub mod handler;
pub mod client;

pub use client::run_telegram_daemon;
