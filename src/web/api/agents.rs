//! API endpoints for agents.

use axum::{
    extract::Path,
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};

use crate::config::{load_settings, AgentConfig};

/// Agent API response.
#[derive(Serialize)]
pub struct AgentResponse {
    pub id: String,
    pub name: String,
    pub provider: String,
    pub model: Option<String>,
    pub working_directory: Option<String>,
    pub is_sovereign: bool,
}

impl From<(String, AgentConfig)> for AgentResponse {
    fn from((id, agent): (String, AgentConfig)) -> Self {
        Self {
            id,
            name: agent.name.unwrap_or_else(|| "Unknown".to_string()),
            provider: agent.provider.unwrap_or_else(|| "unknown".to_string()),
            model: agent.model,
            working_directory: agent.working_directory.map(|p| p.to_string_lossy().to_string()),
            is_sovereign: agent.is_sovereign,
        }
    }
}

/// Create agent request.
#[derive(Deserialize)]
pub struct CreateAgentRequest {
    pub id: String,
    pub name: String,
    pub provider: String,
    pub model: Option<String>,
    pub working_directory: Option<String>,
}

/// List all agents.
pub async fn list_agents() -> Result<Json<Vec<AgentResponse>>, StatusCode> {
    let settings = load_settings().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    let agents: Vec<AgentResponse> = settings.agents
        .into_iter()
        .map(AgentResponse::from)
        .collect();
    
    Ok(Json(agents))
}

/// Get a single agent.
pub async fn get_agent(Path(id): Path<String>) -> Result<Json<AgentResponse>, StatusCode> {
    let settings = load_settings().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    let agent = settings.agents.get(&id)
        .ok_or(StatusCode::NOT_FOUND)?;
    
    Ok(Json(AgentResponse::from((id, agent.clone()))))
}

/// Create a new agent.
pub async fn create_agent(
    Json(payload): Json<CreateAgentRequest>,
) -> Result<Json<AgentResponse>, StatusCode> {
    let mut settings = load_settings().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    if settings.agents.contains_key(&payload.id) {
        return Err(StatusCode::CONFLICT);
    }
    
    let agent = AgentConfig {
        name: Some(payload.name),
        provider: Some(payload.provider),
        model: payload.model,
        working_directory: payload.working_directory.map(|p| p.into()),
        is_sovereign: false,
    };
    
    let id = payload.id.clone();
    settings.agents.insert(id.clone(), agent.clone());
    
    // Save settings
    let path = crate::config::get_settings_path().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let content = serde_json::to_string_pretty(&settings).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    std::fs::write(path, content).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    Ok(Json(AgentResponse::from((id, agent))))
}

/// Delete an agent.
pub async fn delete_agent(Path(id): Path<String>) -> Result<StatusCode, StatusCode> {
    let mut settings = load_settings().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    if settings.agents.remove(&id).is_none() {
        return Err(StatusCode::NOT_FOUND);
    }
    
    // Save settings
    let path = crate::config::get_settings_path().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let content = serde_json::to_string_pretty(&settings).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    std::fs::write(path, content).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    Ok(StatusCode::NO_CONTENT)
}
