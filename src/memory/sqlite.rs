//! SQLite-backed operational memory for events, decisions, and outcomes.

use rusqlite::{params, Connection};

use crate::config::get_home_dir;
use crate::error::Error;

#[derive(Debug, Clone)]
pub struct SessionSummary {
    pub session_id: String,
    pub event_count: usize,
    pub decision_count: usize,
    pub outcome_count: usize,
    pub last_outcome: Option<String>,
}

fn db_path() -> Result<std::path::PathBuf, Error> {
    Ok(get_home_dir()?.join("memory").join("events.db"))
}

pub fn sqlite_db_path() -> Result<std::path::PathBuf, Error> {
    db_path()
}

fn connect() -> Result<Connection, Error> {
    let path = db_path()?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let conn = Connection::open(path).map_err(|e| Error::Memory(format!("sqlite open: {}", e)))?;
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS events (
            id TEXT PRIMARY KEY,
            ts INTEGER NOT NULL,
            session_id TEXT NOT NULL,
            agent_id TEXT NOT NULL,
            event_type TEXT NOT NULL,
            detail TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS decisions (
            id TEXT PRIMARY KEY,
            ts INTEGER NOT NULL,
            session_id TEXT NOT NULL,
            agent_id TEXT NOT NULL,
            intent TEXT NOT NULL,
            owner TEXT NOT NULL,
            priority TEXT NOT NULL,
            deadline TEXT,
            reason TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS outcomes (
            id TEXT PRIMARY KEY,
            ts INTEGER NOT NULL,
            session_id TEXT NOT NULL,
            agent_id TEXT NOT NULL,
            status TEXT NOT NULL,
            error_code TEXT,
            summary TEXT NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_events_session ON events(session_id, ts);
        CREATE INDEX IF NOT EXISTS idx_decisions_session ON decisions(session_id, ts);
        CREATE INDEX IF NOT EXISTS idx_outcomes_session ON outcomes(session_id, ts);
        "#,
    )
    .map_err(|e| Error::Memory(format!("sqlite init: {}", e)))?;
    Ok(conn)
}

pub fn record_event(
    session_id: &str,
    agent_id: &str,
    event_type: &str,
    detail: &str,
) -> Result<(), Error> {
    let conn = connect()?;
    conn.execute(
        "INSERT INTO events (id, ts, session_id, agent_id, event_type, detail) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            ulid::Ulid::new().to_string(),
            chrono::Utc::now().timestamp_millis(),
            session_id,
            agent_id,
            event_type,
            detail
        ],
    )
    .map_err(|e| Error::Memory(format!("sqlite insert event: {}", e)))?;
    Ok(())
}

pub fn record_decision(
    session_id: &str,
    agent_id: &str,
    intent: &str,
    owner: &str,
    priority: &str,
    deadline: Option<&str>,
    reason: &str,
) -> Result<(), Error> {
    let conn = connect()?;
    conn.execute(
        "INSERT INTO decisions (id, ts, session_id, agent_id, intent, owner, priority, deadline, reason) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        params![
            ulid::Ulid::new().to_string(),
            chrono::Utc::now().timestamp_millis(),
            session_id,
            agent_id,
            intent,
            owner,
            priority,
            deadline,
            reason
        ],
    )
    .map_err(|e| Error::Memory(format!("sqlite insert decision: {}", e)))?;
    Ok(())
}

pub fn record_outcome(
    session_id: &str,
    agent_id: &str,
    status: &str,
    error_code: Option<&str>,
    summary: &str,
) -> Result<(), Error> {
    let conn = connect()?;
    conn.execute(
        "INSERT INTO outcomes (id, ts, session_id, agent_id, status, error_code, summary) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            ulid::Ulid::new().to_string(),
            chrono::Utc::now().timestamp_millis(),
            session_id,
            agent_id,
            status,
            error_code,
            summary
        ],
    )
    .map_err(|e| Error::Memory(format!("sqlite insert outcome: {}", e)))?;
    Ok(())
}

pub fn summarize_session(session_id: &str) -> Result<SessionSummary, Error> {
    let conn = connect()?;
    let event_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM events WHERE session_id = ?1",
            params![session_id],
            |row| row.get(0),
        )
        .map_err(|e| Error::Memory(format!("sqlite count events: {}", e)))?;
    let decision_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM decisions WHERE session_id = ?1",
            params![session_id],
            |row| row.get(0),
        )
        .map_err(|e| Error::Memory(format!("sqlite count decisions: {}", e)))?;
    let outcome_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM outcomes WHERE session_id = ?1",
            params![session_id],
            |row| row.get(0),
        )
        .map_err(|e| Error::Memory(format!("sqlite count outcomes: {}", e)))?;

    let mut stmt = conn
        .prepare("SELECT summary FROM outcomes WHERE session_id = ?1 ORDER BY ts DESC LIMIT 1")
        .map_err(|e| Error::Memory(format!("sqlite prepare summary: {}", e)))?;
    let mut rows = stmt
        .query(params![session_id])
        .map_err(|e| Error::Memory(format!("sqlite query summary: {}", e)))?;
    let last_outcome = rows
        .next()
        .map_err(|e| Error::Memory(format!("sqlite read summary: {}", e)))?
        .and_then(|r| r.get::<_, String>(0).ok());

    Ok(SessionSummary {
        session_id: session_id.to_string(),
        event_count: event_count as usize,
        decision_count: decision_count as usize,
        outcome_count: outcome_count as usize,
        last_outcome,
    })
}

pub fn failed_outcomes_last_hour(agent_id: &str) -> Result<u32, Error> {
    let conn = connect()?;
    let since = chrono::Utc::now().timestamp_millis() - 3_600_000;
    let count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM outcomes WHERE agent_id = ?1 AND status = 'failed' AND ts >= ?2",
            params![agent_id, since],
            |row| row.get(0),
        )
        .map_err(|e| Error::Memory(format!("sqlite count failed outcomes: {}", e)))?;
    Ok(count as u32)
}

pub fn vacuum() -> Result<(), Error> {
    let conn = connect()?;
    conn.execute_batch("VACUUM;")
        .map_err(|e| Error::Memory(format!("sqlite vacuum: {}", e)))?;
    Ok(())
}
