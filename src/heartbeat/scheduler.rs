//! Cron scheduling for heartbeat.
#![allow(dead_code)]

use cron::Schedule;
use std::str::FromStr;
use tokio::time::sleep;
use chrono::{DateTime, Utc};

/// Heartbeat schedule.
#[derive(Debug, Clone)]
pub struct HeartbeatSchedule {
    /// Schedule ID.
    pub id: String,
    
    /// Cron expression.
    pub cron: String,
    
    /// Schedule type.
    pub schedule_type: ScheduleType,
    
    /// Target agent ID.
    pub agent_id: Option<String>,
    
    /// Target team ID.
    pub team_id: Option<String>,
    
    /// Sender ID for responses.
    pub sender_id: Option<String>,
    
    /// Enabled.
    pub enabled: bool,
    
    /// Last run time.
    pub last_run: Option<DateTime<Utc>>,
    
    /// Next run time.
    pub next_run: Option<DateTime<Utc>>,
}

/// Schedule type.
#[derive(Debug, Clone, PartialEq)]
pub enum ScheduleType {
    Heartbeat,
    Daily,
    Digest,
    Task,
}

impl FromStr for ScheduleType {
    type Err = String;
    
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "heartbeat" => Ok(ScheduleType::Heartbeat),
            "daily" => Ok(ScheduleType::Daily),
            "digest" => Ok(ScheduleType::Digest),
            "task" => Ok(ScheduleType::Task),
            _ => Err(format!("Unknown schedule type: {}", s)),
        }
    }
}

impl HeartbeatSchedule {
    /// Create a new heartbeat schedule.
    pub fn new(id: &str, cron: &str, schedule_type: ScheduleType) -> Self {
        Self {
            id: id.to_string(),
            cron: cron.to_string(),
            schedule_type,
            agent_id: None,
            team_id: None,
            sender_id: None,
            enabled: true,
            last_run: None,
            next_run: None,
        }
    }
    
    /// Create a daily schedule at a specific time.
    pub fn daily(time: &str) -> Result<Self, String> {
        // Parse time (HH:MM format)
        let parts: Vec<&str> = time.split(':').collect();
        if parts.len() != 2 {
            return Err("Invalid time format. Use HH:MM".to_string());
        }
        
        let hour: u32 = parts[0].parse().map_err(|_| "Invalid hour")?;
        let minute: u32 = parts[1].parse().map_err(|_| "Invalid minute")?;
        
        if hour > 23 || minute > 59 {
            return Err("Invalid time. Hour must be 0-23, minute 0-59".to_string());
        }
        
        // Create cron expression: "MM HH * * * *"
        let cron = format!("{} {} * * * *", minute, hour);
        
        Ok(Self::new(&format!("daily_{}{}", hour, minute), &cron, ScheduleType::Daily))
    }
    
    /// Create an interval schedule (every N seconds).
    pub fn interval(seconds: u64) -> Self {
        // For intervals, we use a simple approach
        let cron = format!("*/{} * * * * *", seconds);
        Self::new(&format!("interval_{}s", seconds), &cron, ScheduleType::Heartbeat)
    }
    
    /// Parse and get the cron schedule.
    pub fn get_schedule(&self) -> Result<Schedule, String> {
        Schedule::from_str(&self.cron)
            .map_err(|e| format!("Invalid cron expression: {}", e))
    }
    
    /// Calculate next run time.
    pub fn calculate_next_run(&mut self) -> Result<DateTime<Utc>, String> {
        let schedule = self.get_schedule()?;
        
        let _now = Utc::now();
        let next = schedule.upcoming(Utc).next()
            .ok_or_else(|| "No upcoming schedule".to_string())?;
        
        self.next_run = Some(next);
        Ok(next)
    }
    
    /// Mark as run.
    pub fn mark_run(&mut self) {
        self.last_run = Some(Utc::now());
    }
    
    /// Set agent target.
    pub fn with_agent(mut self, agent_id: &str) -> Self {
        self.agent_id = Some(agent_id.to_string());
        self
    }
    
    /// Set team target.
    pub fn with_team(mut self, team_id: &str) -> Self {
        self.team_id = Some(team_id.to_string());
        self
    }
    
    /// Set sender.
    pub fn with_sender(mut self, sender_id: &str) -> Self {
        self.sender_id = Some(sender_id.to_string());
        self
    }
}

/// Schedule manager.
pub struct ScheduleManager {
    schedules: Vec<HeartbeatSchedule>,
}

impl ScheduleManager {
    /// Create a new schedule manager.
    pub fn new() -> Self {
        Self {
            schedules: Vec::new(),
        }
    }
    
    /// Add a schedule.
    pub fn add(&mut self, schedule: HeartbeatSchedule) {
        self.schedules.push(schedule);
    }
    
    /// Remove a schedule.
    pub fn remove(&mut self, id: &str) -> Option<HeartbeatSchedule> {
        let idx = self.schedules.iter().position(|s| s.id == id)?;
        Some(self.schedules.remove(idx))
    }
    
    /// Get all schedules.
    pub fn list(&self) -> &[HeartbeatSchedule] {
        &self.schedules
    }
    
    /// Get enabled schedules.
    pub fn enabled(&self) -> Vec<&HeartbeatSchedule> {
        self.schedules.iter().filter(|s| s.enabled).collect()
    }
    
    /// Get schedules due for execution.
    pub fn due(&self) -> Vec<&HeartbeatSchedule> {
        let now = Utc::now();
        self.schedules.iter()
            .filter(|s| {
                s.enabled && s.next_run.map_or(true, |next| next <= now)
            })
            .collect()
    }
    
    /// Update next run times.
    pub fn update_next_runs(&mut self) {
        for schedule in &mut self.schedules {
            if let Err(e) = schedule.calculate_next_run() {
                tracing::warn!("Failed to calculate next run for {}: {}", schedule.id, e);
            }
        }
    }
}

impl Default for ScheduleManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Wait until the next scheduled time.
pub async fn wait_until_next(schedule: &HeartbeatSchedule) -> Result<(), String> {
    let next = schedule.next_run.ok_or("No next run time")?;
    let now = Utc::now();
    
    if next > now {
        let duration = (next - now).to_std()
            .map_err(|_| "Invalid duration")?;
        sleep(duration).await;
    }
    
    Ok(())
}

/// Common schedule presets.
pub fn default_heartbeat_schedule() -> HeartbeatSchedule {
    HeartbeatSchedule::interval(3600) // Every hour
}

pub fn default_daily_schedule(time: &str) -> Result<HeartbeatSchedule, String> {
    HeartbeatSchedule::daily(time)
}
