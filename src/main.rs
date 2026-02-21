//! TinyVegeta - Multi-agent, multi-team, Telegram-first 24/7 AI assistant.
//!
//! This is the main entry point for the Rust rewrite.

use clap::Parser;
use std::process::ExitCode;

mod cli;
mod config;
mod agent;
mod board;
mod context;
mod core;
mod error;
mod heartbeat;
mod logging;
mod memory;
mod providers;
mod task;
mod sovereign;
mod telegram;
mod tmux;
mod web;

use cli::Commands;

#[tokio::main]
async fn main() -> ExitCode {
    // Initialize logging
    if let Err(e) = logging::init() {
        eprintln!("Failed to initialize logging: {}", e);
        return ExitCode::FAILURE;
    }

    // Parse command line arguments
    let args = Commands::parse();

    // Run the command
    match args.run().await {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            tracing::error!("{}", e);
            eprintln!("Error: {}", e);
            ExitCode::FAILURE
        }
    }
}
