use futures::StreamExt;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use thiserror::Error;
use tokio::sync::mpsc;

#[derive(Error, Debug)]
pub enum OllamaError {
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("Ollama not running at {0}")]
    NotRunning(String),
    #[error("Model not found: {0}")]
    ModelNotFound(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Model {
    pub name: String,
    pub size: u64,
    pub digest: String,
    #[serde(default)]
    pub details: Option<ModelDetails>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelDetails {
    pub parameter_size: Option<String>,
    pub quantization_level: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<ChatMessage>,
    stream: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ChatResponse {
    pub message: Option<ChatMessage>,
    pub done: bool,
    // These fields are returned by Ollama but we don't use them yet
    #[serde(default)]
    #[allow(dead_code)]
    total_duration: Option<u64>,
    #[serde(default)]
    #[allow(dead_code)]
    eval_count: Option<u64>,
}

#[derive(Debug, Clone, Deserialize)]
struct ModelsResponse {
    models: Vec<Model>,
}

/// Client for communicating with Ollama's HTTP API
#[derive(Clone)]
pub struct OllamaClient {
    client: Client,
    base_url: String,
}

impl OllamaClient {
    pub fn new(base_url: &str) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(300)) // 5 min timeout for slow generations
            .build()
            .expect("Failed to create HTTP client");

        Self {
            client,
            base_url: base_url.trim_end_matches('/').to_string(),
        }
    }

    /// Check if Ollama is running
    pub async fn health_check(&self) -> Result<bool, OllamaError> {
        let url = format!("{}/api/tags", self.base_url);
        match self.client.get(&url).send().await {
            Ok(resp) => Ok(resp.status().is_success()),
            Err(_) => Ok(false),
        }
    }

    /// List available models
    pub async fn list_models(&self) -> Result<Vec<Model>, OllamaError> {
        let url = format!("{}/api/tags", self.base_url);
        let resp = self.client.get(&url).send().await?;

        if !resp.status().is_success() {
            return Err(OllamaError::NotRunning(self.base_url.clone()));
        }

        let models_resp: ModelsResponse = resp.json().await?;
        Ok(models_resp.models)
    }

    /// Send a chat message and stream the response
    pub async fn chat_stream(
        &self,
        model: &str,
        messages: Vec<ChatMessage>,
        tx: mpsc::UnboundedSender<String>,
    ) -> Result<ChatResponse, OllamaError> {
        let url = format!("{}/api/chat", self.base_url);

        let request = ChatRequest {
            model: model.to_string(),
            messages,
            stream: true,
        };

        let resp = self.client.post(&url).json(&request).send().await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            if text.contains("model") && text.contains("not found") {
                return Err(OllamaError::ModelNotFound(model.to_string()));
            }
            return Err(OllamaError::NotRunning(format!("HTTP {}: {}", status, text)));
        }

        let mut stream = resp.bytes_stream();
        let mut final_response = ChatResponse {
            message: None,
            done: false,
            total_duration: None,
            eval_count: None,
        };
        let mut full_content = String::new();

        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;
            let text = String::from_utf8_lossy(&chunk);

            // Each line is a JSON object
            for line in text.lines() {
                if line.trim().is_empty() {
                    continue;
                }

                if let Ok(response) = serde_json::from_str::<ChatResponse>(line) {
                    if let Some(ref msg) = response.message {
                        full_content.push_str(&msg.content);
                        let _ = tx.send(msg.content.clone());
                    }

                    if response.done {
                        final_response = response;
                        final_response.message = Some(ChatMessage {
                            role: "assistant".to_string(),
                            content: full_content.clone(),
                        });
                    }
                }
            }
        }

        Ok(final_response)
    }

    /// Send a chat message (non-streaming) - kept for potential future use
    #[allow(dead_code)]
    pub async fn chat(
        &self,
        model: &str,
        messages: Vec<ChatMessage>,
    ) -> Result<ChatResponse, OllamaError> {
        let url = format!("{}/api/chat", self.base_url);

        let request = ChatRequest {
            model: model.to_string(),
            messages,
            stream: false,
        };

        let resp = self.client.post(&url).json(&request).send().await?;

        if !resp.status().is_success() {
            return Err(OllamaError::NotRunning(self.base_url.clone()));
        }

        let response: ChatResponse = resp.json().await?;
        Ok(response)
    }
}

impl Default for OllamaClient {
    fn default() -> Self {
        Self::new("http://127.0.0.1:11434")
    }
}

