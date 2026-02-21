//! Ollama HTTP provider.
#![allow(dead_code)]

use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::path::Path;

use super::provider::{Provider, Result};

pub struct OllamaProvider {
    client: Client,
    base_url: String,
    default_model: String,
}

#[derive(Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<Message>,
    stream: bool,
}

#[derive(Serialize)]
struct Message {
    role: String,
    content: String,
}

#[derive(Deserialize)]
struct ChatResponse {
    message: ResponseMessage,
}

#[derive(Deserialize)]
struct ResponseMessage {
    content: String,
}

#[derive(Deserialize)]
struct ModelsResponse {
    models: Vec<ModelInfo>,
}

#[derive(Deserialize)]
struct ModelInfo {
    name: String,
}

impl OllamaProvider {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
            base_url: "http://localhost:11434".to_string(),
            default_model: "llama3.2".to_string(),
        }
    }
    
    pub fn with_base_url(base_url: impl Into<String>) -> Self {
        Self {
            client: Client::new(),
            base_url: base_url.into(),
            default_model: "llama3.2".to_string(),
        }
    }
    
    pub fn with_model(model: impl Into<String>) -> Self {
        Self {
            client: Client::new(),
            base_url: "http://localhost:11434".to_string(),
            default_model: model.into(),
        }
    }
}

impl Default for OllamaProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Provider for OllamaProvider {
    fn name(&self) -> &str {
        "ollama"
    }
    
    async fn is_available(&self) -> bool {
        self.client
            .get(format!("{}/api/tags", self.base_url))
            .send()
            .await
            .is_ok()
    }
    
    async fn list_models(&self) -> Result<Vec<String>> {
        let response = self.client
            .get(format!("{}/api/tags", self.base_url))
            .send()
            .await?;
        
        let models: ModelsResponse = response.json().await?;
        
        Ok(models.models.into_iter().map(|m| m.name).collect())
    }
    
    async fn complete(
        &self,
        prompt: &str,
        model: Option<&str>,
        _working_dir: Option<&Path>,
    ) -> Result<String> {
        let model = model.unwrap_or(&self.default_model);
        
        let request = ChatRequest {
            model: model.to_string(),
            messages: vec![Message {
                role: "user".to_string(),
                content: prompt.to_string(),
            }],
            stream: false,
        };
        
        let response = self.client
            .post(format!("{}/api/chat", self.base_url))
            .json(&request)
            .send()
            .await?;
        
        let chat_response: ChatResponse = response.json().await?;
        
        Ok(chat_response.message.content)
    }
    
    fn default_model(&self) -> Option<&str> {
        Some(&self.default_model)
    }
}
