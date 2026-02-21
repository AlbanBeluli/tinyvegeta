//! Web server module (Axum + API).

pub mod api;
pub mod auth;
pub mod router;
pub mod server;

pub use server::run_web_server;
