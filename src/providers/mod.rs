//! AI Providers module.
#![allow(dead_code)]

use std::sync::Arc;

pub mod provider;
pub mod claude;
pub mod codex;
pub mod cline;
pub mod opencode;
pub mod ollama;
pub mod grok;

pub use provider::{Provider, Result};

use crate::config::Settings;

/// Provider factory.
pub fn create_provider(name: &str, settings: &Settings) -> Arc<dyn Provider> {
    match name {
        "claude" => Arc::new(claude::ClaudeProvider::new()),
        "codex" => Arc::new(codex::CodexProvider::new()),
        "cline" => Arc::new(cline::ClineProvider::new()),
        "opencode" => Arc::new(opencode::OpenCodeProvider::new()),
        "ollama" => {
            if let Some(url) = &settings.models.ollama.base_url {
                Arc::new(ollama::OllamaProvider::with_base_url(url.clone()))
            } else {
                Arc::new(ollama::OllamaProvider::new())
            }
        }
        "grok" => Arc::new(grok::GrokProvider::new()),
        _ => Arc::new(cline::ClineProvider::new()),
    }
}

/// Get the current provider from settings.
pub fn get_current_provider(settings: &Settings) -> Arc<dyn Provider> {
    create_provider(&settings.models.provider, settings)
}

/// Check if a provider is available.
pub async fn is_provider_available(name: &str, settings: &Settings) -> bool {
    let provider = create_provider(name, settings);
    provider.is_available().await
}

/// Complete a prompt with the current provider.
pub async fn complete(
    prompt: &str,
    model: Option<&str>,
    working_dir: Option<&std::path::Path>,
    settings: &Settings,
) -> Result<String> {
    let provider = get_current_provider(settings);
    provider.complete(prompt, model, working_dir).await
}

/// List available models for a provider.
pub async fn list_models(name: &str, settings: &Settings) -> Result<Vec<String>> {
    let provider = create_provider(name, settings);
    provider.list_models().await
}
