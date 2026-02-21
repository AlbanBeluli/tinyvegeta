//! Codex CLI provider.
#![allow(dead_code)]

use async_trait::async_trait;
use std::path::Path;
use std::process::Stdio;
use tokio::process::Command;

use super::provider::{Provider, ProviderError, Result};

pub struct CodexProvider {
    cli_path: String,
    default_model: String,
}

impl CodexProvider {
    pub fn new() -> Self {
        Self {
            cli_path: "codex".to_string(),
            default_model: "gpt-5.3-codex".to_string(),
        }
    }
    
    pub fn with_cli_path(cli_path: impl Into<String>) -> Self {
        Self {
            cli_path: cli_path.into(),
            default_model: "gpt-5.3-codex".to_string(),
        }
    }
}

impl Default for CodexProvider {
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
impl Provider for CodexProvider {
    fn name(&self) -> &str {
        "codex"
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
            "gpt-5.3-codex".to_string(),
            "gpt-4o-codex".to_string(),
        ])
    }
    
    async fn complete(
        &self,
        prompt: &str,
        model: Option<&str>,
        working_dir: Option<&Path>,
    ) -> Result<String> {
        let mut cmd = Command::new(&self.cli_path);
        // Use non-interactive mode and place flags before prompt.
        cmd.arg("exec")
           .arg("--sandbox")
           .arg("danger-full-access")
           .arg("--skip-git-repo-check");

        if let Some(m) = selected_model_arg(model) {
            cmd.arg("--model").arg(m);
        }

        cmd.arg(prompt);
        
        if let Some(dir) = working_dir {
            cmd.current_dir(dir);
        }
        
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());
        
        let output = cmd.output().await?;
        
        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
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
        assert_eq!(selected_model_arg(Some("o3")), Some("o3".to_string()));
    }
}
