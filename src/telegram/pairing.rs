//! Pairing logic for Telegram bot.
#![allow(dead_code)]

use ulid::Ulid;

use crate::config::{get_settings_path, load_settings, ApprovedSender, PendingSender};

/// Pairing mode.
#[derive(Debug, Clone, PartialEq)]
pub enum PairingMode {
    Approval,
    Open,
}

impl PairingMode {
    pub fn from_str(s: &str) -> Self {
        match s {
            "open" => PairingMode::Open,
            _ => PairingMode::Approval,
        }
    }
}

/// Pairing manager.
pub struct PairingManager;

impl PairingManager {
    /// Check if a sender is approved.
    pub fn is_approved(sender_id: &str) -> bool {
        let settings = match load_settings() {
            Ok(s) => s,
            Err(_) => return false,
        };

        let mode = PairingMode::from_str(&settings.pairing.mode);

        // Open mode allows everyone
        if mode == PairingMode::Open {
            return true;
        }

        // Check approved list
        if let Some(approved) = &settings.pairing.approved_senders {
            return approved.iter().any(|s| s.sender_id == sender_id);
        }

        false
    }

    /// Check if a sender is pending approval.
    pub fn is_pending(sender_id: &str) -> bool {
        let settings = match load_settings() {
            Ok(s) => s,
            Err(_) => return false,
        };

        if let Some(pending) = &settings.pairing.pending_senders {
            return pending.iter().any(|s| s.sender_id == sender_id);
        }

        false
    }

    /// Generate a pairing code for a new sender.
    pub fn generate_code() -> String {
        let code = Ulid::new().to_string();
        code[..8].to_uppercase()
    }

    /// Add a pending sender.
    pub fn add_pending(sender_id: &str, sender_name: &str) -> Result<String, String> {
        // Check if already approved or pending
        if Self::is_approved(sender_id) {
            return Err("Sender already approved".to_string());
        }

        if Self::is_pending(sender_id) {
            return Err("Sender already pending".to_string());
        }

        let code = Self::generate_code();

        // Load settings
        let mut settings = load_settings().map_err(|e| e.to_string())?;

        // Ensure pending_senders exists
        if settings.pairing.pending_senders.is_none() {
            settings.pairing.pending_senders = Some(Vec::new());
        }

        if let Some(pending) = &mut settings.pairing.pending_senders {
            pending.push(PendingSender {
                sender_id: sender_id.to_string(),
                sender_name: sender_name.to_string(),
                code: code.clone(),
                requested_at: chrono::Utc::now().timestamp_millis(),
            });
        }

        // Save settings
        let path = get_settings_path().map_err(|e| e.to_string())?;
        let content = serde_json::to_string_pretty(&settings).map_err(|e| e.to_string())?;
        std::fs::write(path, content).map_err(|e| e.to_string())?;

        tracing::info!("Added pending sender: {} ({})", sender_name, sender_id);

        Ok(code)
    }

    /// Approve a sender by code.
    pub fn approve_by_code(code: &str) -> Result<ApprovedSender, String> {
        let mut settings = load_settings().map_err(|e| e.to_string())?;

        // Find pending sender with this code
        let pending_sender = if let Some(pending) = &mut settings.pairing.pending_senders {
            let idx = pending.iter().position(|s| s.code == code);
            if let Some(idx) = idx {
                Some(pending.remove(idx))
            } else {
                None
            }
        } else {
            None
        };

        let pending_sender = pending_sender.ok_or_else(|| "Invalid code".to_string())?;

        // Ensure approved_senders exists
        if settings.pairing.approved_senders.is_none() {
            settings.pairing.approved_senders = Some(Vec::new());
        }

        let approved_sender = ApprovedSender {
            sender_id: pending_sender.sender_id.clone(),
            sender_name: pending_sender.sender_name.clone(),
            paired_at: chrono::Utc::now().timestamp_millis(),
        };

        if let Some(approved) = &mut settings.pairing.approved_senders {
            approved.push(approved_sender.clone());
        }

        // Save settings
        let path = get_settings_path().map_err(|e| e.to_string())?;
        let content = serde_json::to_string_pretty(&settings).map_err(|e| e.to_string())?;
        std::fs::write(path, content).map_err(|e| e.to_string())?;

        tracing::info!(
            "Approved sender: {} ({})",
            approved_sender.sender_name,
            approved_sender.sender_id
        );

        Ok(approved_sender)
    }

    /// Unpair (remove) an approved sender.
    pub fn unpair(sender_id: &str) -> Result<(), String> {
        let mut settings = load_settings().map_err(|e| e.to_string())?;

        if let Some(approved) = &mut settings.pairing.approved_senders {
            approved.retain(|s| s.sender_id != sender_id);
        }

        // Save settings
        let path = get_settings_path().map_err(|e| e.to_string())?;
        let content = serde_json::to_string_pretty(&settings).map_err(|e| e.to_string())?;
        std::fs::write(path, content).map_err(|e| e.to_string())?;

        tracing::info!("Unpaired sender: {}", sender_id);

        Ok(())
    }

    /// Check if sender is the soul owner.
    pub fn is_soul_owner(sender_id: &str) -> bool {
        let settings = match load_settings() {
            Ok(s) => s,
            Err(_) => return false,
        };

        settings.pairing.soul_owner_sender_id.as_deref() == Some(sender_id)
    }

    /// Set soul owner.
    pub fn set_soul_owner(sender_id: &str) -> Result<(), String> {
        let mut settings = load_settings().map_err(|e| e.to_string())?;

        settings.pairing.soul_owner_sender_id = Some(sender_id.to_string());

        // Save settings
        let path = get_settings_path().map_err(|e| e.to_string())?;
        let content = serde_json::to_string_pretty(&settings).map_err(|e| e.to_string())?;
        std::fs::write(path, content).map_err(|e| e.to_string())?;

        tracing::info!("Set soul owner: {}", sender_id);

        Ok(())
    }
}
