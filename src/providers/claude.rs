//! Claude CLI provider.
#![allow(dead_code)]

use async_trait::async_trait;
use std::path::Path;
use std::process::Stdio;
use tokio::process::Command;

use super::provider::{Provider, ProviderError, Result};

pub struct ClaudeProvider {
    cli_path: String,
    default_model: String,
}

impl ClaudeProvider {
    pub fn new() -> Self {
        Self {
            cli_path: "claude".to_string(),
            default_model: "sonnet".to_string(),
        }
    }
    
    pub fn with_cli_path(cli_path: impl Into<String>) -> Self {
        Self {
            cli_path: cli_path.into(),
            default_model: "sonnet".to_string(),
        }
    }
}

impl Default for ClaudeProvider {
    fn default() -> Self {
        Self::new()
    }
}

fn selected_model_arg(model: Option<&str>) -> Option<String> {
    model
        .map(str::trim)
        .filter(|m| !m.is_empty() && *m != "default")
        .map(ToString::to_string)
}

#[async_trait]
impl Provider for ClaudeProvider {
    fn name(&self) -> &str {
        "claude"
    }
    
    async fn is_available(&self) -> bool {
        tokio::process::Command::new(&self.cli_path)
            .arg("--version")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map(|_| true)
            .unwrap_or(false)
    }
    
    async fn list_models(&self) -> Result<Vec<String>> {
        Ok(vec![
            "sonnet".to_string(),
            "opus".to_string(),
            "haiku".to_string(),
        ])
    }
    
    async fn complete(
        &self,
        prompt: &str,
        model: Option<&str>,
        working_dir: Option<&Path>,
    ) -> Result<String> {
        let mut cmd = Command::new(&self.cli_path);
        cmd.arg("-c")
           .arg("-p")
           .arg(prompt);

        if let Some(m) = selected_model_arg(model) {
            cmd.arg("--model").arg(m);
        }
        
        if let Some(dir) = working_dir {
            cmd.current_dir(dir);
        }
        
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());
        
        let output = cmd.output().await?;
        
        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).to_string())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(ProviderError::ApiError(stderr.to_string()))
        }
    }
    
    fn default_model(&self) -> Option<&str> {
        Some(&self.default_model)
    }
}

#[cfg(test)]
mod tests {
    use super::selected_model_arg;

    #[test]
    fn default_model_does_not_force_override() {
        assert_eq!(selected_model_arg(Some("default")), None);
        assert_eq!(selected_model_arg(Some("")), None);
        assert_eq!(selected_model_arg(Some("opus")), Some("opus".to_string()));
    }
}
