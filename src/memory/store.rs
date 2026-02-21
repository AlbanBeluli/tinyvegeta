//! Memory storage - three-layer memory system.
#![allow(dead_code)]

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::config::get_home_dir;
use crate::error::Error;

use super::lock::with_lock;

/// Memory scope.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum MemoryScope {
    Global,
    Agent,
    Team,
    Task,
}

impl Default for MemoryScope {
    fn default() -> Self {
        MemoryScope::Global
    }
}

impl std::fmt::Display for MemoryScope {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MemoryScope::Global => write!(f, "global"),
            MemoryScope::Agent => write!(f, "agent"),
            MemoryScope::Team => write!(f, "team"),
            MemoryScope::Task => write!(f, "task"),
        }
    }
}

/// Memory entry.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct MemoryEntry {
    pub key: String,
    pub value: String,
    pub scope: MemoryScope,
    pub scope_id: Option<String>,
    pub category: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
    pub expires_at: Option<i64>,
    pub importance: f32,
}

impl MemoryEntry {
    /// Create a new memory entry.
    pub fn new(key: &str, value: &str, scope: MemoryScope, scope_id: Option<String>) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as i64;

        Self {
            key: key.to_string(),
            value: value.to_string(),
            scope,
            scope_id,
            category: None,
            created_at: now,
            updated_at: now,
            expires_at: None,
            importance: 1.0,
        }
    }

    /// Check if entry has expired.
    pub fn is_expired(&self) -> bool {
        if let Some(expires_at) = self.expires_at {
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_millis() as i64;
            return now > expires_at;
        }
        false
    }
}

/// Memory store file format.
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct MemoryStore {
    pub entries: HashMap<String, MemoryEntry>,
}

const GLOBAL_LIMIT: usize = 2000;
const AGENT_LIMIT: usize = 1500;
const TEAM_LIMIT: usize = 1500;
const TASK_LIMIT: usize = 750;

impl MemoryStore {
    /// Create empty store.
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
        }
    }

    /// Get an entry.
    pub fn get(&self, key: &str) -> Option<&MemoryEntry> {
        self.entries.get(key).filter(|e| !e.is_expired())
    }

    /// Set an entry.
    pub fn set(&mut self, entry: MemoryEntry) {
        self.entries.insert(entry.key.clone(), entry);
    }

    /// Delete an entry.
    pub fn delete(&mut self, key: &str) -> Option<MemoryEntry> {
        self.entries.remove(key)
    }

    /// List entries by scope.
    pub fn list_by_scope(&self, scope: &MemoryScope, scope_id: Option<&str>) -> Vec<&MemoryEntry> {
        self.entries
            .values()
            .filter(|e| {
                e.scope == *scope
                    && scope_id.map_or(true, |id| e.scope_id.as_deref() == Some(id))
                    && !e.is_expired()
            })
            .collect()
    }

    /// List entries by category.
    pub fn list_by_category(&self, category: &str) -> Vec<&MemoryEntry> {
        self.entries
            .values()
            .filter(|e| e.category.as_deref() == Some(category) && !e.is_expired())
            .collect()
    }

    /// Search entries.
    pub fn search(&self, query: &str) -> Vec<&MemoryEntry> {
        let query_lower = query.to_lowercase();
        self.entries
            .values()
            .filter(|e| {
                !e.is_expired()
                    && (e.key.to_lowercase().contains(&query_lower)
                        || e.value.to_lowercase().contains(&query_lower))
            })
            .collect()
    }

    /// Clean expired entries.
    pub fn cleanup(&mut self) -> usize {
        let before = self.entries.len();
        self.entries.retain(|_, e| !e.is_expired());
        before - self.entries.len()
    }
}

/// Get the memory directory.
pub fn get_memory_dir() -> Result<PathBuf, Error> {
    Ok(get_home_dir()?.join("memory"))
}

/// Get memory file path for a scope.
pub fn get_memory_file(scope: &MemoryScope, scope_id: Option<&str>) -> Result<PathBuf, Error> {
    let mem_dir = get_memory_dir()?;

    match scope {
        MemoryScope::Global => Ok(mem_dir.join("global.json")),
        MemoryScope::Agent => {
            let id = scope_id
                .ok_or_else(|| Error::Memory("Agent scope requires scope_id".to_string()))?;
            Ok(mem_dir.join("agents").join(format!("{}.json", id)))
        }
        MemoryScope::Team => {
            let id = scope_id
                .ok_or_else(|| Error::Memory("Team scope requires scope_id".to_string()))?;
            Ok(mem_dir.join("teams").join(format!("{}.json", id)))
        }
        MemoryScope::Task => {
            let id = scope_id
                .ok_or_else(|| Error::Memory("Task scope requires scope_id".to_string()))?;
            Ok(mem_dir.join("tasks").join(format!("{}.json", id)))
        }
    }
}

/// Ensure memory directories exist.
pub fn ensure_memory_dirs() -> Result<(), Error> {
    let mem_dir = get_memory_dir()?;
    std::fs::create_dir_all(&mem_dir)?;
    std::fs::create_dir_all(mem_dir.join("agents"))?;
    std::fs::create_dir_all(mem_dir.join("teams"))?;
    std::fs::create_dir_all(mem_dir.join("tasks"))?;
    std::fs::create_dir_all(mem_dir.join("snapshots"))?;
    Ok(())
}

/// Load memory store from file.
pub fn load_store(scope: &MemoryScope, scope_id: Option<&str>) -> Result<MemoryStore, Error> {
    let path = get_memory_file(scope, scope_id)?;

    if !path.exists() {
        return Ok(MemoryStore::new());
    }

    let content = std::fs::read_to_string(&path)?;
    let store: MemoryStore = serde_json::from_str(&content)?;
    Ok(store)
}

/// Save memory store to file.
pub fn save_store(
    scope: &MemoryScope,
    scope_id: Option<&str>,
    store: &MemoryStore,
) -> Result<(), Error> {
    ensure_memory_dirs()?;

    let path = get_memory_file(scope, scope_id)?;

    // Ensure parent directory exists
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let content = serde_json::to_string_pretty(store)?;
    std::fs::write(&path, content)?;
    Ok(())
}

/// Memory operations.
pub struct Memory;

impl Memory {
    /// Set a memory entry.
    pub fn set(
        key: &str,
        value: &str,
        scope: MemoryScope,
        scope_id: Option<&str>,
    ) -> Result<(), Error> {
        ensure_memory_dirs()?;

        let path = get_memory_file(&scope, scope_id)?;

        with_lock(&path, || {
            let mut store = load_store(&scope, scope_id).unwrap_or_default();

            let mut entry = MemoryEntry::new(key, value, scope.clone(), scope_id.map(String::from));

            // Preserve category if updating
            if let Some(existing) = store.get(key) {
                entry.category = existing.category.clone();
            }

            store.set(entry);
            prune_store(&mut store, scope, scope_id);
            save_store(&scope, scope_id, &store)?;

            tracing::debug!(
                "Set memory: {} = {} (scope: {:?}, id: {:?})",
                key,
                value,
                scope,
                scope_id
            );
            Ok(())
        })
    }

    /// Get a memory entry.
    pub fn get(
        key: &str,
        scope: MemoryScope,
        scope_id: Option<&str>,
    ) -> Result<Option<MemoryEntry>, Error> {
        let path = get_memory_file(&scope, scope_id)?;

        if !path.exists() {
            return Ok(None);
        }

        let store = load_store(&scope, scope_id)?;
        Ok(store.get(key).cloned())
    }

    /// Delete a memory entry.
    pub fn delete(key: &str, scope: MemoryScope, scope_id: Option<&str>) -> Result<(), Error> {
        let path = get_memory_file(&scope, scope_id)?;

        if !path.exists() {
            return Ok(());
        }

        with_lock(&path, || {
            let mut store = load_store(&scope, scope_id).unwrap_or_default();
            store.delete(key);
            save_store(&scope, scope_id, &store)?;
            tracing::debug!(
                "Deleted memory: {} (scope: {:?}, id: {:?})",
                key,
                scope,
                scope_id
            );
            Ok(())
        })
    }

    /// List memory entries.
    pub fn list(
        scope: MemoryScope,
        scope_id: Option<&str>,
        category: Option<&str>,
    ) -> Result<Vec<MemoryEntry>, Error> {
        let path = get_memory_file(&scope, scope_id)?;

        if !path.exists() {
            return Ok(vec![]);
        }

        let store = load_store(&scope, scope_id)?;

        let entries = if let Some(cat) = category {
            store.list_by_category(cat)
        } else {
            store.list_by_scope(&scope, scope_id)
        };

        Ok(entries.into_iter().cloned().collect())
    }

    /// Search memory.
    pub fn search(query: &str, limit: usize) -> Result<Vec<MemoryEntry>, Error> {
        ensure_memory_dirs()?;

        let mut results = Vec::new();

        // Search global
        let global_path = get_memory_file(&MemoryScope::Global, None)?;
        if global_path.exists() {
            let store = load_store(&MemoryScope::Global, None)?;
            for entry in store.search(query) {
                results.push(entry.clone());
            }
        }

        // Search agents
        let agents_dir = get_memory_dir()?.join("agents");
        if agents_dir.exists() {
            for entry in std::fs::read_dir(agents_dir)? {
                let entry = entry?;
                if entry.path().extension().map_or(false, |e| e == "json") {
                    let content = std::fs::read_to_string(entry.path())?;
                    if let Ok(store) = serde_json::from_str::<MemoryStore>(&content) {
                        for e in store.search(query) {
                            results.push(e.clone());
                        }
                    }
                }
            }
        }

        // Search teams
        let teams_dir = get_memory_dir()?.join("teams");
        if teams_dir.exists() {
            for entry in std::fs::read_dir(teams_dir)? {
                let entry = entry?;
                if entry.path().extension().map_or(false, |e| e == "json") {
                    let content = std::fs::read_to_string(entry.path())?;
                    if let Ok(store) = serde_json::from_str::<MemoryStore>(&content) {
                        for e in store.search(query) {
                            results.push(e.clone());
                        }
                    }
                }
            }
        }

        // Limit results
        results.truncate(limit);
        Ok(results)
    }

    /// Retrieve relevant memory entries for prompt context.
    pub fn relevant(
        query: &str,
        scope: MemoryScope,
        scope_id: Option<&str>,
        limit: usize,
    ) -> Result<Vec<MemoryEntry>, Error> {
        let path = get_memory_file(&scope, scope_id)?;
        if !path.exists() {
            return Ok(Vec::new());
        }
        let store = load_store(&scope, scope_id)?;
        let q = query.to_lowercase();
        let mut entries: Vec<MemoryEntry> = store
            .entries
            .values()
            .filter(|e| !e.is_expired())
            .map(|e| {
                let mut c = e.clone();
                let mut score = c.importance;
                let kl = c.key.to_lowercase();
                let vl = c.value.to_lowercase();
                if !q.is_empty() {
                    if kl.contains(&q) || vl.contains(&q) {
                        score += 4.0;
                    }
                    for token in q.split_whitespace() {
                        if token.len() < 3 {
                            continue;
                        }
                        if kl.contains(token) || vl.contains(token) {
                            score += 0.8;
                        }
                    }
                    // Lightweight semantic ranking via hashed-token embedding similarity.
                    score += cosine_sim(&text_embedding(&q), &text_embedding(&format!("{} {}", kl, vl))) * 3.0;
                }
                // recency bias
                score += (c.updated_at as f32) / 1_500_000_000_000.0;
                c.importance = score;
                c
            })
            .collect();

        entries.sort_by(|a, b| b.importance.partial_cmp(&a.importance).unwrap_or(std::cmp::Ordering::Equal));
        entries.truncate(limit);
        Ok(entries)
    }

    /// Get memory statistics.
    pub fn stats() -> Result<MemoryStats, Error> {
        ensure_memory_dirs()?;

        let mut global_count = 0;
        let mut agent_count = 0;
        let mut team_count = 0;
        let mut task_count = 0;

        // Global
        let global_path = get_memory_file(&MemoryScope::Global, None)?;
        if global_path.exists() {
            let store = load_store(&MemoryScope::Global, None)?;
            global_count = store.entries.len();
        }

        // Agents
        let agents_dir = get_memory_dir()?.join("agents");
        if agents_dir.exists() {
            for entry in std::fs::read_dir(agents_dir)? {
                let entry = entry?;
                if entry.path().extension().map_or(false, |e| e == "json") {
                    let content = std::fs::read_to_string(entry.path())?;
                    if let Ok(store) = serde_json::from_str::<MemoryStore>(&content) {
                        agent_count += store.entries.len();
                    }
                }
            }
        }

        // Tasks
        let tasks_dir = get_memory_dir()?.join("tasks");
        if tasks_dir.exists() {
            for entry in std::fs::read_dir(tasks_dir)? {
                let entry = entry?;
                if entry.path().extension().map_or(false, |e| e == "json") {
                    let content = std::fs::read_to_string(entry.path())?;
                    if let Ok(store) = serde_json::from_str::<MemoryStore>(&content) {
                        task_count += store.entries.len();
                    }
                }
            }
        }

        // Teams
        let teams_dir = get_memory_dir()?.join("teams");
        if teams_dir.exists() {
            for entry in std::fs::read_dir(teams_dir)? {
                let entry = entry?;
                if entry.path().extension().map_or(false, |e| e == "json") {
                    let content = std::fs::read_to_string(entry.path())?;
                    if let Ok(store) = serde_json::from_str::<MemoryStore>(&content) {
                        team_count += store.entries.len();
                    }
                }
            }
        }

        Ok(MemoryStats {
            global: global_count,
            agents: agent_count,
            teams: team_count,
            tasks: task_count,
            total: global_count + agent_count + team_count + task_count,
        })
    }

    /// Clear memory for a scope.
    pub fn clear(scope: MemoryScope, scope_id: Option<&str>) -> Result<(), Error> {
        let path = get_memory_file(&scope, scope_id)?;

        if path.exists() {
            std::fs::remove_file(&path)?;
            tracing::info!("Cleared memory: scope: {:?}, id: {:?}", scope, scope_id);
        }

        Ok(())
    }

    /// Compact memory: dedupe, merge similar, cleanup expired, promote high-signal.
    pub fn compact(scope: MemoryScope, scope_id: Option<&str>) -> Result<CompactReport, Error> {
        let path = get_memory_file(&scope, scope_id)?;
        if !path.exists() {
            return Ok(CompactReport::default());
        }

        with_lock(&path, || {
            let mut store = load_store(&scope, scope_id).unwrap_or_default();
            let mut report = CompactReport::default();

            report.expired_removed = store.cleanup();

            // Merge near-duplicate values into earliest key.
            let mut keys: Vec<String> = store.entries.keys().cloned().collect();
            keys.sort();
            for i in 0..keys.len() {
                for j in (i + 1)..keys.len() {
                    let Some(a) = store.entries.get(&keys[i]).cloned() else { continue };
                    let Some(b) = store.entries.get(&keys[j]).cloned() else { continue };
                    if normalized(&a.value) == normalized(&b.value) || cosine_sim(&text_embedding(&a.value), &text_embedding(&b.value)) > 0.95 {
                        if let Some(entry) = store.entries.get_mut(&keys[i]) {
                            entry.updated_at = entry.updated_at.max(b.updated_at);
                            entry.importance = entry.importance.max(b.importance) + 0.2;
                        }
                        store.entries.remove(&keys[j]);
                        report.merged += 1;
                    }
                }
            }

            // Promote high-signal keys.
            for entry in store.entries.values_mut() {
                let k = entry.key.to_lowercase();
                if k.contains("decision") || k.contains("owner") || k.contains("workspace") || k.contains("incident") {
                    entry.importance += 0.3;
                    report.promoted += 1;
                }
            }

            let before = store.entries.len();
            prune_store(&mut store, scope, scope_id);
            report.pruned = before.saturating_sub(store.entries.len());
            save_store(&scope, scope_id, &store)?;
            Ok(report)
        })
    }
}

#[derive(Debug, Clone, Default)]
pub struct CompactReport {
    pub expired_removed: usize,
    pub merged: usize,
    pub promoted: usize,
    pub pruned: usize,
}

fn scope_limit(scope: MemoryScope, _scope_id: Option<&str>) -> usize {
    match scope {
        MemoryScope::Global => GLOBAL_LIMIT,
        MemoryScope::Agent => AGENT_LIMIT,
        MemoryScope::Team => TEAM_LIMIT,
        MemoryScope::Task => TASK_LIMIT,
    }
}

fn prune_store(store: &mut MemoryStore, scope: MemoryScope, scope_id: Option<&str>) {
    let limit = scope_limit(scope, scope_id);
    if store.entries.len() <= limit {
        return;
    }
    let mut entries: Vec<MemoryEntry> = store.entries.values().cloned().collect();
    entries.sort_by(|a, b| {
        let sa = a.importance * 10.0 + (a.updated_at as f32 / 1_000_000_000_000.0);
        let sb = b.importance * 10.0 + (b.updated_at as f32 / 1_000_000_000_000.0);
        sa.partial_cmp(&sb).unwrap_or(std::cmp::Ordering::Equal)
    });
    let remove_count = store.entries.len().saturating_sub(limit);
    for e in entries.into_iter().take(remove_count) {
        store.entries.remove(&e.key);
    }
}

fn normalized(s: &str) -> String {
    s.to_lowercase()
        .replace(|c: char| !c.is_ascii_alphanumeric() && !c.is_ascii_whitespace(), " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn text_embedding(text: &str) -> [f32; 64] {
    let mut v = [0.0_f32; 64];
    for tok in normalized(text).split_whitespace() {
        let mut h: u64 = 1469598103934665603;
        for b in tok.as_bytes() {
            h ^= *b as u64;
            h = h.wrapping_mul(1099511628211);
        }
        let idx = (h as usize) % 64;
        v[idx] += 1.0;
    }
    // L2 normalize.
    let norm = v.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm > 0.0 {
        for x in &mut v {
            *x /= norm;
        }
    }
    v
}

fn cosine_sim(a: &[f32; 64], b: &[f32; 64]) -> f32 {
    let mut dot = 0.0;
    for i in 0..64 {
        dot += a[i] * b[i];
    }
    dot
}

/// Memory statistics.
#[derive(Debug, Clone)]
pub struct MemoryStats {
    pub global: usize,
    pub agents: usize,
    pub teams: usize,
    pub tasks: usize,
    pub total: usize,
}

impl std::fmt::Display for MemoryStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Memory Stats:\n")?;
        write!(f, "  Global: {}\n", self.global)?;
        write!(f, "  Agents:  {}\n", self.agents)?;
        write!(f, "  Teams:   {}\n", self.teams)?;
        write!(f, "  Tasks:   {}\n", self.tasks)?;
        write!(f, "  Total:   {}", self.total)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_entry() {
        let entry = MemoryEntry::new("test_key", "test_value", MemoryScope::Global, None);

        assert_eq!(entry.key, "test_key");
        assert_eq!(entry.value, "test_value");
        assert!(!entry.is_expired());
    }

    #[test]
    fn test_memory_store() {
        let mut store = MemoryStore::new();

        let entry = MemoryEntry::new("key1", "value1", MemoryScope::Global, None);
        store.set(entry);

        assert!(store.get("key1").is_some());
        assert!(store.get("key2").is_none());

        store.delete("key1");
        assert!(store.get("key1").is_none());
    }
}
