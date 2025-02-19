use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use crate::server::error::ServerError;

type Result<T> = std::result::Result<T, ServerError>;

/// Message role in a sampling conversation
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MessageRole {
    User,
    Assistant,
}

/// Content type for a message
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum MessageContent {
    Text { text: String },
    Image { data: String, mime_type: Option<String> },
}

/// A message in a sampling conversation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: MessageRole,
    pub content: MessageContent,
}

/// Model selection preferences
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelPreferences {
    pub hints: Option<Vec<ModelHint>>,
    pub cost_priority: Option<f32>,
    pub speed_priority: Option<f32>,
    pub intelligence_priority: Option<f32>,
}

/// A hint for model selection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelHint {
    pub name: Option<String>,
}

/// Context inclusion level for sampling
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ContextInclusion {
    None,
    ThisServer,
    AllServers,
}

/// Parameters for a sampling request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SamplingRequest {
    pub messages: Vec<Message>,
    pub model_preferences: Option<ModelPreferences>,
    pub system_prompt: Option<String>,
    pub include_context: Option<ContextInclusion>,
    pub temperature: Option<f32>,
    pub max_tokens: u32,
    pub stop_sequences: Option<Vec<String>>,
    pub metadata: Option<HashMap<String, Value>>,
}

/// Stop reason for a sampling completion
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum StopReason {
    EndTurn,
    StopSequence,
    MaxTokens,
    Unknown,
    #[serde(other)]
    Other,
}

/// Result of a sampling request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SamplingResult {
    pub model: String,
    pub stop_reason: Option<StopReason>,
    pub role: MessageRole,
    pub content: MessageContent,
}

/// A callback that can handle sampling requests
pub trait SamplingCallback: Send + Sync {
    fn call(
        &self,
        request: SamplingRequest,
    ) -> Pin<Box<dyn Future<Output = Result<SamplingResult>> + Send + 'static>>;
}

impl<F, Fut> SamplingCallback for F
where
    F: Fn(SamplingRequest) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Result<SamplingResult>> + Send + 'static,
{
    fn call(
        &self,
        request: SamplingRequest,
    ) -> Pin<Box<dyn Future<Output = Result<SamplingResult>> + Send + 'static>> {
        Box::pin(self(request))
    }
}

// Type aliases for complex future and callback types
type SamplingFuture = Pin<Box<dyn Future<Output = Result<SamplingResult>> + Send + 'static>>;
type SamplingCallbackFunc = Arc<dyn Fn(SamplingRequest) -> SamplingFuture + Send + Sync>;

/// A registered sampling handler
pub(crate) struct RegisteredSampling {
    /// The callback to handle sampling requests
    #[allow(dead_code)]
    pub callback: SamplingCallbackFunc,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_sampling_request() {
        let request = SamplingRequest {
            messages: vec![Message {
                role: MessageRole::User,
                content: MessageContent::Text {
                    text: "Hello".to_string(),
                },
            }],
            model_preferences: Some(ModelPreferences {
                hints: Some(vec![ModelHint {
                    name: Some("claude-3".to_string()),
                }]),
                cost_priority: Some(0.5),
                speed_priority: Some(0.8),
                intelligence_priority: Some(0.9),
            }),
            system_prompt: Some("You are a helpful assistant.".to_string()),
            include_context: Some(ContextInclusion::ThisServer),
            temperature: Some(0.7),
            max_tokens: 100,
            stop_sequences: Some(vec!["END".to_string()]),
            metadata: None,
        };

        let callback = |_req: SamplingRequest| {
            Box::pin(async move {
                Ok(SamplingResult {
                    model: "claude-3".to_string(),
                    stop_reason: Some(StopReason::EndTurn),
                    role: MessageRole::Assistant,
                    content: MessageContent::Text {
                        text: "Hi there!".to_string(),
                    },
                })
            }) as Pin<Box<dyn Future<Output = Result<SamplingResult>> + Send>>
        };

        let result = callback(request).await.unwrap();
        assert_eq!(result.model, "claude-3");
        if let MessageContent::Text { text } = result.content {
            assert_eq!(text, "Hi there!");
        } else {
            panic!("Expected text content");
        }
    }
}
