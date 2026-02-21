//! Web server using Axum.

use std::net::SocketAddr;
use tower_http::cors::{Any, CorsLayer};

use super::router::create_app_router;

/// Web server configuration.
pub struct WebServerConfig {
    pub port: u16,
    pub host: String,
}

impl Default for WebServerConfig {
    fn default() -> Self {
        Self {
            port: 3333,
            host: "0.0.0.0".to_string(),
        }
    }
}

/// Run the web server.
pub async fn run_server(config: WebServerConfig) -> Result<(), Box<dyn std::error::Error>> {
    let app = create_app_router()
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any),
        );
    
    let addr: SocketAddr = format!("{}:{}", config.host, config.port)
        .parse()
        .map_err(|e| format!("Invalid address: {}", e))?;
    
    tracing::info!("Starting web server on {}", addr);
    
    let listener = tokio::net::TcpListener::bind(addr).await?;
    
    axum::serve(listener, app).await?;
    
    Ok(())
}

/// Run the web server with default config.
pub async fn run_web_server(port: u16) -> Result<(), Box<dyn std::error::Error>> {
    let config = WebServerConfig {
        port,
        ..Default::default()
    };
    
    run_server(config).await
}
