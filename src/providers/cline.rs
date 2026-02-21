//! Cline CLI provider.
#![allow(dead_code)]

use async_trait::async_trait;
use serde_json::Value;
use std::path::Path;
use std::process::Stdio;
use tokio::process::Command;

use super::provider::{Provider, ProviderError, Result};

pub struct ClineProvider {
    cli_path: String,
    default_model: String,
}

impl ClineProvider {
    pub fn new() -> Self {
        Self {
            cli_path: "cline".to_string(),
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

impl Default for ClineProvider {
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

fn extract_cline_response(stdout: &str) -> String {
    let raw = stdout.trim();
    if raw.is_empty() {
        return String::new();
    }

    let mut best: Option<(u8, String)> = None;
    for line in raw.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        let Ok(v) = serde_json::from_str::<Value>(line) else {
            continue;
        };

        let typ = v.get("type").and_then(|t| t.as_str()).unwrap_or_default();
        let say = v.get("say").and_then(|s| s.as_str()).unwrap_or_default();

        // Highest-priority structured payloads.
        if let Some(text) = v.get("result").and_then(|x| x.as_str()) {
            if !text.trim().is_empty() {
                best = Some((100, text.trim().to_string()));
                continue;
            }
        }
        if let Some(text) = v.get("message").and_then(|x| x.as_str()) {
            if !text.trim().is_empty() {
                best = Some((90, text.trim().to_string()));
                continue;
            }
        }

        // Common event-stream formats.
        if let Some(text) = v.get("text").and_then(|x| x.as_str()) {
            let text = text.trim();
            if text.is_empty() {
                continue;
            }

            // Ignore task/setup echo events that contain the injected prompt.
            if typ == "say" && (say == "task" || say == "plan") {
                continue;
            }

            let score = if typ == "assistant_message" || typ == "final" || typ == "result" {
                80
            } else if typ == "say" {
                60
            } else {
                50
            };
            best = Some((score, text.to_string()));
        }
    }

    if let Some((_, text)) = best {
        return text;
    }

    // Fallback: remove JSON event lines and return remaining text.
    let plain = raw
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('{'))
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_string();

    if plain.is_empty() {
        raw.to_string()
    } else {
        plain
    }
}

#[async_trait]
impl Provider for ClineProvider {
    fn name(&self) -> &str {
        "cline"
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
            "claude-sonnet-4-20250514".to_string(),
            "gpt-4o".to_string(),
        ])
    }
    
    async fn complete(
        &self,
        prompt: &str,
        model: Option<&str>,
        working_dir: Option<&Path>,
    ) -> Result<String> {
        let mut cmd = Command::new(&self.cli_path);
        cmd.arg("task")
           .arg(prompt)
           .arg("--json");

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
            let raw = String::from_utf8_lossy(&output.stdout).to_string();
            Ok(extract_cline_response(&raw))
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let mut msg = stderr.to_string();
            if msg.contains("Unauthorized: Please sign in to Cline") {
                msg.push_str(
                    "\nHint: run `cline auth` in the same user context as tinyvegeta, then restart tinyvegeta.",
                );
            }
            Err(ProviderError::ApiError(msg))
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
    fn default_model_does_not_override_cli_selection() {
        assert_eq!(selected_model_arg(None), None);
        assert_eq!(selected_model_arg(Some("")), None);
        assert_eq!(selected_model_arg(Some("default")), None);
        assert_eq!(
            selected_model_arg(Some("z-ai/glm-5")),
            Some("z-ai/glm-5".to_string())
        );
    }
}
