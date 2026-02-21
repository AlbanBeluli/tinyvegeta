//! Grok HTTP provider.
#![allow(dead_code)]

use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::env;
use std::path::Path;

use super::provider::{Provider, ProviderError, Result};

pub struct GrokProvider {
    client: Client,
    api_key: Option<String>,
    base_url: String,
    default_model: String,
}

#[derive(Serialize)]
struct ChatRequest {
    messages: Vec<Message>,
    model: String,
}

#[derive(Serialize)]
struct Message {
    role: String,
    content: String,
}

#[derive(Deserialize)]
struct ChatResponse {
    choices: Vec<Choice>,
}

#[derive(Deserialize)]
struct Choice {
    message: ResponseMessage,
}

#[derive(Deserialize)]
struct ResponseMessage {
    content: String,
}

impl GrokProvider {
    pub fn new() -> Self {
        let api_key = env::var("XAI_API_KEY")
            .or_else(|_| env::var("GROK_API_KEY"))
            .ok();
        
        Self {
            client: Client::new(),
            api_key,
            base_url: "https://api.x.ai/v1".to_string(),
            default_model: "grok-4".to_string(),
        }
    }
    
    pub fn with_api_key(api_key: impl Into<String>) -> Self {
        Self {
            client: Client::new(),
            api_key: Some(api_key.into()),
            base_url: "https://api.x.ai/v1".to_string(),
            default_model: "grok-4".to_string(),
        }
    }
    
    fn get_api_key(&self) -> Result<&str> {
        self.api_key
            .as_deref()
            .ok_or_else(|| ProviderError::NotAvailable("XAI_API_KEY not set".to_string()))
    }
}

impl Default for GrokProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Provider for GrokProvider {
    fn name(&self) -> &str {
        "grok"
    }
    
    async fn is_available(&self) -> bool {
        if self.api_key.is_none() {
            return false;
        }
        
        // Try a simple request to check connectivity
        self.client
            .get("https://api.x.ai")
            .send()
            .await
            .is_ok()
    }
    
    async fn list_models(&self) -> Result<Vec<String>> {
        Ok(vec![
            "grok-4".to_string(),
            "grok-4-latest".to_string(),
            "grok-2".to_string(),
            "grok-2-vision-1212".to_string(),
        ])
    }
    
    async fn complete(
        &self,
        prompt: &str,
        model: Option<&str>,
        _working_dir: Option<&Path>,
    ) -> Result<String> {
        let api_key = self.get_api_key()?;
        let model = model.unwrap_or(&self.default_model);
        
        let request = ChatRequest {
            messages: vec![Message {
                role: "user".to_string(),
                content: prompt.to_string(),
            }],
            model: model.to_string(),
        };
        
        let response = self.client
            .post(format!("{}/chat/completions", self.base_url))
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await?;
        
        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(ProviderError::ApiError(format!("HTTP {}: {}", status, text)));
        }
        
        let chat_response: ChatResponse = response.json().await?;
        
        chat_response
            .choices
            .first()
            .map(|c| c.message.content.clone())
            .ok_or_else(|| ProviderError::ApiError("No response choices".to_string()))
    }
    
    fn default_model(&self) -> Option<&str> {
        Some(&self.default_model)
    }
}
