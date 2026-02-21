//! AI Provider trait for TinyVegeta.
#![allow(dead_code)]

use async_trait::async_trait;
use std::path::Path;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ProviderError {
    #[error("Provider not available: {0}")]
    NotAvailable(String),
    
    #[error("Model not found: {0}")]
    ModelNotFound(String),
    
    #[error("API error: {0}")]
    ApiError(String),
    
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    
    #[error("HTTP error: {0}")]
    HttpError(#[from] reqwest::Error),
    
    #[error("Parse error: {0}")]
    ParseError(String),
    
    #[error("Timeout")]
    Timeout,
    
    #[error("{0}")]
    Other(String),
}

pub type Result<T> = std::result::Result<T, ProviderError>;

/// AI Provider trait.
#[async_trait]
pub trait Provider: Send + Sync {
    /// Provider name.
    fn name(&self) -> &str;
    
    /// Check if the provider is available (CLI installed or API configured).
    async fn is_available(&self) -> bool;
    
    /// List available models.
    async fn list_models(&self) -> Result<Vec<String>>;
    
    /// Complete a prompt.
    async fn complete(
        &self,
        prompt: &str,
        model: Option<&str>,
        working_dir: Option<&Path>,
    ) -> Result<String>;
    
    /// Get the default model.
    fn default_model(&self) -> Option<&str>;
}


impl ProviderError {
    pub fn other(s: impl Into<String>) -> Self {
        ProviderError::Other(s.into())
    }
}
