//! OpenCode CLI provider.
#![allow(dead_code)]

use async_trait::async_trait;
use std::path::Path;
use std::process::Stdio;
use tokio::process::Command;

use super::provider::{Provider, ProviderError, Result};

pub struct OpenCodeProvider {
    cli_path: String,
    default_model: String,
}

impl OpenCodeProvider {
    pub fn new() -> Self {
        Self {
            cli_path: "opencode".to_string(),
            default_model: "default".to_string(),
        }
    }
    
    pub fn with_cli_path(cli_path: impl Into<String>) -> Self {
        Self {
            cli_path: cli_path.into(),
            default_model: "default".to_string(),
        }
    }
}

impl Default for OpenCodeProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Provider for OpenCodeProvider {
    fn name(&self) -> &str {
        "opencode"
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
            "default".to_string(),
        ])
    }
    
    async fn complete(
        &self,
        prompt: &str,
        model: Option<&str>,
        working_dir: Option<&Path>,
    ) -> Result<String> {
        let _model = model.unwrap_or(&self.default_model);
        
        let mut cmd = Command::new(&self.cli_path);
        cmd.arg("complete")
           .arg(prompt);
        
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
