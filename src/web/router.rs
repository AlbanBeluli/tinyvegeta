//! Route definitions for web server.

use axum::{
    routing::{get, post},
    Router,
};

use super::api;

/// Create the API router.
pub fn create_api_router() -> Router {
    Router::new()
        // Agents
        .route("/agents", get(api::list_agents).post(api::create_agent))
        .route("/agents/:id", get(api::get_agent).delete(api::delete_agent))
        
        // Teams
        .route("/teams", get(api::list_teams).post(api::create_team))
        .route("/teams/:id", get(api::get_team).delete(api::delete_team))
        
        // Memory
        .route("/memory", post(api::set_memory).get(api::list_memory))
        .route("/memory/:key", get(api::get_memory).delete(api::delete_memory))
        .route("/memory/search", get(api::search_memory))
        .route("/memory/stats", get(api::memory_stats))
}

/// Create the full app router.
pub fn create_app_router() -> Router {
    Router::new()
        .nest("/api", create_api_router())
        .route("/health", get(health_check))
}

/// Health check endpoint.
async fn health_check() -> &'static str {
    "OK"
}
