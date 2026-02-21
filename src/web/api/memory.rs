//! API endpoints for memory.
#![allow(dead_code)]

use axum::{
    extract::{Path, Query},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};

use crate::memory::{Memory, MemoryEntry, MemoryScope};

/// Memory API response.
#[derive(Serialize)]
pub struct MemoryResponse {
    pub key: String,
    pub value: String,
    pub scope: String,
    pub scope_id: Option<String>,
    pub category: Option<String>,
}

impl From<MemoryEntry> for MemoryResponse {
    fn from(entry: MemoryEntry) -> Self {
        Self {
            key: entry.key,
            value: entry.value,
            scope: entry.scope.to_string(),
            scope_id: entry.scope_id,
            category: entry.category,
        }
    }
}

/// Set memory request.
#[derive(Deserialize)]
pub struct SetMemoryRequest {
    pub key: String,
    pub value: String,
    pub scope: Option<String>,
    pub scope_id: Option<String>,
    pub category: Option<String>,
}

/// Memory query parameters.
#[derive(Deserialize)]
pub struct MemoryQuery {
    pub scope: Option<String>,
    pub scope_id: Option<String>,
    pub category: Option<String>,
}

/// Set a memory entry.
pub async fn set_memory(
    Json(payload): Json<SetMemoryRequest>,
) -> Result<Json<MemoryResponse>, StatusCode> {
    let scope = match payload.scope.as_deref() {
        Some("agent") => MemoryScope::Agent,
        Some("team") => MemoryScope::Team,
        Some("task") => MemoryScope::Task,
        _ => MemoryScope::Global,
    };
    
    Memory::set(&payload.key, &payload.value, scope, payload.scope_id.as_deref())
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    // Get the entry back
    let entry = Memory::get(&payload.key, scope, payload.scope_id.as_deref())
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
    
    Ok(Json(MemoryResponse::from(entry)))
}

/// Get a memory entry.
pub async fn get_memory(
    Path(key): Path<String>,
    Query(query): Query<MemoryQuery>,
) -> Result<Json<MemoryResponse>, StatusCode> {
    let scope = match query.scope.as_deref() {
        Some("agent") => MemoryScope::Agent,
        Some("team") => MemoryScope::Team,
        Some("task") => MemoryScope::Task,
        _ => MemoryScope::Global,
    };
    
    let entry = Memory::get(&key, scope, query.scope_id.as_deref())
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;
    
    Ok(Json(MemoryResponse::from(entry)))
}

/// List memory entries.
pub async fn list_memory(
    Query(query): Query<MemoryQuery>,
) -> Result<Json<Vec<MemoryResponse>>, StatusCode> {
    let scope = match query.scope.as_deref() {
        Some("agent") => MemoryScope::Agent,
        Some("team") => MemoryScope::Team,
        Some("task") => MemoryScope::Task,
        _ => MemoryScope::Global,
    };
    
    let entries = Memory::list(scope, query.scope_id.as_deref(), query.category.as_deref())
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    let responses: Vec<MemoryResponse> = entries
        .into_iter()
        .map(MemoryResponse::from)
        .collect();
    
    Ok(Json(responses))
}

/// Delete a memory entry.
pub async fn delete_memory(
    Path(key): Path<String>,
    Query(query): Query<MemoryQuery>,
) -> Result<StatusCode, StatusCode> {
    let scope = match query.scope.as_deref() {
        Some("agent") => MemoryScope::Agent,
        Some("team") => MemoryScope::Team,
        Some("task") => MemoryScope::Task,
        _ => MemoryScope::Global,
    };
    
    Memory::delete(&key, scope, query.scope_id.as_deref())
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    Ok(StatusCode::NO_CONTENT)
}

/// Search memory.
#[derive(Deserialize)]
pub struct SearchQuery {
    pub q: String,
    pub limit: Option<usize>,
}

pub async fn search_memory(
    Query(query): Query<SearchQuery>,
) -> Result<Json<Vec<MemoryResponse>>, StatusCode> {
    let limit = query.limit.unwrap_or(10);
    
    let entries = Memory::search(&query.q, limit)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    let responses: Vec<MemoryResponse> = entries
        .into_iter()
        .map(MemoryResponse::from)
        .collect();
    
    Ok(Json(responses))
}

/// Get memory stats.
#[derive(Serialize)]
pub struct MemoryStatsResponse {
    pub global: usize,
    pub agents: usize,
    pub teams: usize,
    pub tasks: usize,
    pub total: usize,
}

pub async fn memory_stats() -> Result<Json<MemoryStatsResponse>, StatusCode> {
    let stats = Memory::stats()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    Ok(Json(MemoryStatsResponse {
        global: stats.global,
        agents: stats.agents,
        teams: stats.teams,
        tasks: stats.tasks,
        total: stats.total,
    }))
}
