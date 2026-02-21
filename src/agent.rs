//! Agent execution contracts: timeout, retries, and failure codes.

use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use crate::providers::Provider;

#[derive(Debug, Clone)]
pub struct ExecutionContract {
    pub timeout_seconds: u64,
    pub retries: u32,
    pub retry_backoff_ms: u64,
}

impl Default for ExecutionContract {
    fn default() -> Self {
        Self {
            timeout_seconds: 240,
            retries: 1,
            retry_backoff_ms: 600,
        }
    }
}

impl ExecutionContract {
    pub fn for_agent(provider: &str) -> Self {
        match provider {
            "ollama" => Self {
                timeout_seconds: 420,
                retries: 1,
                retry_backoff_ms: 800,
            },
            "cline" | "claude" | "codex" | "opencode" | "grok" => Self::default(),
            _ => Self::default(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum FailureCode {
    Timeout,
    Unauthorized,
    ProviderUnavailable,
    CliMissing,
    Unknown,
}

impl std::fmt::Display for FailureCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FailureCode::Timeout => write!(f, "timeout"),
            FailureCode::Unauthorized => write!(f, "unauthorized"),
            FailureCode::ProviderUnavailable => write!(f, "provider_unavailable"),
            FailureCode::CliMissing => write!(f, "cli_missing"),
            FailureCode::Unknown => write!(f, "unknown"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ExecutionError {
    pub code: FailureCode,
    pub message: String,
}

impl std::fmt::Display for ExecutionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}] {}", self.code, self.message)
    }
}

impl std::error::Error for ExecutionError {}

pub async fn execute_with_contract(
    provider: Arc<dyn Provider>,
    prompt: &str,
    model: Option<&str>,
    working_dir: Option<&Path>,
    contract: &ExecutionContract,
) -> Result<String, ExecutionError> {
    let attempts = contract.retries + 1;
    let timeout = Duration::from_secs(contract.timeout_seconds);
    let mut last_error: Option<ExecutionError> = None;

    for attempt in 1..=attempts {
        let result = tokio::time::timeout(timeout, provider.complete(prompt, model, working_dir)).await;
        match result {
            Ok(Ok(text)) => return Ok(text),
            Ok(Err(e)) => {
                let err = classify_error(&e.to_string());
                last_error = Some(err.clone());
                tracing::warn!(
                    "Execution attempt {}/{} failed: {}",
                    attempt,
                    attempts,
                    err
                );
            }
            Err(_) => {
                let err = ExecutionError {
                    code: FailureCode::Timeout,
                    message: format!(
                        "provider completion exceeded timeout of {}s",
                        contract.timeout_seconds
                    ),
                };
                last_error = Some(err.clone());
                tracing::warn!("Execution attempt {}/{} timed out", attempt, attempts);
            }
        }

        if attempt < attempts {
            tokio::time::sleep(Duration::from_millis(contract.retry_backoff_ms)).await;
        }
    }

    Err(last_error.unwrap_or(ExecutionError {
        code: FailureCode::Unknown,
        message: "execution failed for unknown reason".to_string(),
    }))
}

fn classify_error(message: &str) -> ExecutionError {
    let m = message.to_lowercase();
    let code = if m.contains("unauthorized")
        || m.contains("auth")
        || m.contains("sign in")
        || m.contains("forbidden")
    {
        FailureCode::Unauthorized
    } else if m.contains("not found")
        || m.contains("no such file")
        || m.contains("command not found")
    {
        FailureCode::CliMissing
    } else if m.contains("not available")
        || m.contains("connection")
        || m.contains("timeout")
        || m.contains("failed to connect")
    {
        FailureCode::ProviderUnavailable
    } else {
        FailureCode::Unknown
    };

    ExecutionError {
        code,
        message: message.to_string(),
    }
}
