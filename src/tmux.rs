//! Tmux session management for TinyVegeta daemon.
#![allow(dead_code)]

use std::process::Command;

use crate::error::Error;
pub type Result<T> = std::result::Result<T, Error>;

/// The tmux session name used by TinyVegeta.
pub const TMUX_SESSION: &str = "tinyvegeta";

/// Check if a tmux session exists.
pub fn session_exists() -> Result<bool> {
    let output = Command::new("tmux")
        .args(["has-session", "-t", TMUX_SESSION])
        .output()?;

    Ok(output.status.success())
}

/// Check if TinyVegeta is running (session exists and attached).
pub fn is_running() -> Result<bool> {
    if !session_exists()? {
        return Ok(false);
    }

    // Check if the session has at least one client attached
    let output = Command::new("tmux")
        .args(["list-clients", "-t", TMUX_SESSION])
        .output()?;

    // If there are clients attached, it's running
    Ok(output.status.success())
}

/// Start the TinyVegeta daemon in a tmux session.
pub fn start_daemon(binary_path: &str) -> Result<()> {
    if session_exists()? {
        return Err(Error::Tmux(format!(
            "Session '{}' already exists. Stop it first with 'tinyvegeta stop'.",
            TMUX_SESSION
        )));
    }

    // Create the session and start the daemon
    // The -d flag creates the session detached
    let _start_cmd = format!(
        "{} queue &\\; {} telegram &\\; {} heartbeat &\\; sleep infinity",
        binary_path, binary_path, binary_path
    );

    let output = Command::new("tmux")
        .args(["new-session", "-d", "-s", TMUX_SESSION, "-n", "tinyvegeta"])
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(Error::Tmux(format!(
            "Failed to create tmux session: {}",
            stderr
        )));
    }

    // Send the start commands to the session
    let daemon_cmd = format!("{} start-internal", binary_path);

    let output = Command::new("tmux")
        .args(["send-keys", "-t", TMUX_SESSION, &daemon_cmd, "Enter"])
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(Error::Tmux(format!(
            "Failed to send command to session: {}",
            stderr
        )));
    }

    tracing::info!(
        "Started TinyVegeta daemon in tmux session '{}'",
        TMUX_SESSION
    );
    Ok(())
}

/// Stop the TinyVegeta daemon.
pub fn stop_daemon() -> Result<()> {
    if !session_exists()? {
        return Err(Error::Tmux(format!(
            "No session '{}' found. Is TinyVegeta running?",
            TMUX_SESSION
        )));
    }

    // Kill the session
    let output = Command::new("tmux")
        .args(["kill-session", "-t", TMUX_SESSION])
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(Error::Tmux(format!("Failed to kill session: {}", stderr)));
    }

    tracing::info!("Stopped TinyVegeta daemon");
    Ok(())
}

/// Restart the TinyVegeta daemon.
pub fn restart_daemon(binary_path: &str) -> Result<()> {
    // Try to stop first (ignore error if not running)
    let _ = stop_daemon();

    // Start fresh
    start_daemon(binary_path)
}

/// Attach to the TinyVegeta tmux session.
pub fn attach() -> Result<()> {
    if !session_exists()? {
        return Err(Error::Tmux(
            "Session not found. Is TinyVegeta running?".to_string(),
        ));
    }

    // Detach any existing client and attach
    let output = Command::new("tmux")
        .args(["attach-session", "-t", TMUX_SESSION])
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(Error::Tmux(format!("Failed to attach: {}", stderr)));
    }

    Ok(())
}

/// Get status information about the TinyVegeta session.
pub fn get_status() -> Result<String> {
    if !session_exists()? {
        return Ok("Status: stopped".to_string());
    }

    // Get session info
    let output = Command::new("tmux")
        .args(["list-session", "-t", TMUX_SESSION, "-F", "#{session_info}"])
        .output()?;

    if output.status.success() {
        let info = String::from_utf8_lossy(&output.stdout);
        Ok(format!("Status: running\n{}", info))
    } else {
        Ok("Status: running".to_string())
    }
}
