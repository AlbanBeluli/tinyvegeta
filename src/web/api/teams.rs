//! API endpoints for teams.

use axum::{
    extract::Path,
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};

use crate::config::{load_settings, TeamConfig};

/// Team API response.
#[derive(Serialize)]
pub struct TeamResponse {
    pub id: String,
    pub name: String,
    pub agents: Vec<String>,
    pub leader_agent: Option<String>,
}

impl From<(String, TeamConfig)> for TeamResponse {
    fn from((id, team): (String, TeamConfig)) -> Self {
        Self {
            id,
            name: team.name,
            agents: team.agents,
            leader_agent: team.leader_agent,
        }
    }
}

/// Create team request.
#[derive(Deserialize)]
pub struct CreateTeamRequest {
    pub id: String,
    pub name: String,
    pub agents: Vec<String>,
    pub leader_agent: Option<String>,
}

/// List all teams.
pub async fn list_teams() -> Result<Json<Vec<TeamResponse>>, StatusCode> {
    let settings = load_settings().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    let teams: Vec<TeamResponse> = settings.teams
        .into_iter()
        .map(TeamResponse::from)
        .collect();
    
    Ok(Json(teams))
}

/// Get a single team.
pub async fn get_team(Path(id): Path<String>) -> Result<Json<TeamResponse>, StatusCode> {
    let settings = load_settings().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    let team = settings.teams.get(&id)
        .ok_or(StatusCode::NOT_FOUND)?;
    
    Ok(Json(TeamResponse::from((id, team.clone()))))
}

/// Create a new team.
pub async fn create_team(
    Json(payload): Json<CreateTeamRequest>,
) -> Result<Json<TeamResponse>, StatusCode> {
    let mut settings = load_settings().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    if settings.teams.contains_key(&payload.id) {
        return Err(StatusCode::CONFLICT);
    }
    
    let team = TeamConfig {
        name: payload.name,
        agents: payload.agents,
        leader_agent: payload.leader_agent,
    };
    
    let id = payload.id.clone();
    settings.teams.insert(id.clone(), team.clone());
    
    // Save settings
    let path = crate::config::get_settings_path().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let content = serde_json::to_string_pretty(&settings).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    std::fs::write(path, content).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    Ok(Json(TeamResponse::from((id, team))))
}

/// Delete a team.
pub async fn delete_team(Path(id): Path<String>) -> Result<StatusCode, StatusCode> {
    let mut settings = load_settings().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    if settings.teams.remove(&id).is_none() {
        return Err(StatusCode::NOT_FOUND);
    }
    
    // Save settings
    let path = crate::config::get_settings_path().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let content = serde_json::to_string_pretty(&settings).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    std::fs::write(path, content).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    Ok(StatusCode::NO_CONTENT)
}
