use crate::sse::middleware::{AuthConfig, Claims};

use super::{Message, Transport};

use actix_web::web::Bytes;
use anyhow::{Context, Result};
use async_trait::async_trait;
use futures::StreamExt;
use jsonwebtoken::{encode, EncodingKey, Header};

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::{broadcast, mpsc, Mutex};
use tracing::debug;

#[derive(Clone)]
pub struct ServerSseTransport {
    // For receiving messages from HTTP POST requests
    message_rx: Arc<Mutex<mpsc::Receiver<Message>>>,
    message_tx: mpsc::Sender<Message>,
    // For sending messages to SSE clients
    sse_tx: broadcast::Sender<Message>,
}

impl ServerSseTransport {
    pub fn new(sse_tx: broadcast::Sender<Message>) -> Self {
        let (message_tx, message_rx) = mpsc::channel(100);
        Self {
            message_rx: Arc::new(Mutex::new(message_rx)),
            message_tx,
            sse_tx,
        }
    }

    pub async fn send_message(&self, message: Message) -> Result<()> {
        self.message_tx.send(message).await?;
        Ok(())
    }
}

#[async_trait]
impl Transport for ServerSseTransport {
    async fn receive(&self) -> Result<Option<Message>> {
        let mut rx = self.message_rx.lock().await;
        match rx.recv().await {
            Some(message) => {
                debug!("Received message from POST request: {:?}", message);
                Ok(Some(message))
            }
            None => Ok(None),
        }
    }

    async fn send(&self, message: &Message) -> Result<()> {
        self.sse_tx.send(message.clone())?;
        debug!("Sent message to SSE clients: {:?}", message);
        Ok(())
    }

    async fn open(&self) -> Result<()> {
        Ok(())
    }

    async fn close(&self) -> Result<()> {
        Ok(())
    }
}
#[derive(Debug)]
pub enum SseEvent {
    Message(Message),
    SessionId(String),
}

/// Client-side SSE transport that sends messages via HTTP POST
/// and receives responses via SSE
#[derive(Clone)]
pub struct ClientSseTransport {
    tx: mpsc::Sender<Message>,
    rx: Arc<Mutex<mpsc::Receiver<Message>>>,
    server_url: String,
    client: reqwest::Client,
    auth_config: Option<AuthConfig>,
    session_id: Arc<Mutex<Option<String>>>,
    headers: HashMap<String, String>,
}

impl ClientSseTransport {
    pub fn builder(url: String) -> ClientSseTransportBuilder {
        ClientSseTransportBuilder::new(url)
    }

    fn generate_token(&self) -> Result<String> {
        let auth_config = self
            .auth_config
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Auth config not set"))?;

        let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs() as usize;
        let claims = Claims {
            iat: now,
            exp: now + 3600, // Token expires in 1 hour
        };

        encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(auth_config.jwt_secret.as_bytes()),
        )
        .map_err(Into::into)
    }

    async fn add_auth_header(
        &self,
        request: reqwest::RequestBuilder,
    ) -> Result<reqwest::RequestBuilder> {
        if self.auth_config.is_some() {
            let token = self.generate_token()?;
            Ok(request.header("Authorization", format!("Bearer {}", token)))
        } else {
            Ok(request)
        }
    }

    fn parse_sse_message(event: &str) -> Option<SseEvent> {
        let mut event_type = None;
        let mut data = None;

        // Split by newlines and process each line
        for line in event.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            if line.starts_with("event:") {
                event_type = Some(line.trim_start_matches("event:").trim());
            } else if line.starts_with("data:") {
                data = Some(line.trim_start_matches("data:").trim());
            }
        }

        match (event_type, data) {
            (Some("endpoint"), Some(url)) => Some(SseEvent::SessionId(
                url.split("sessionId=")
                    .nth(1)
                    .unwrap_or_default()
                    .to_string(),
            )),
            // Handle case where only data is present (assume it's a message)
            (None, Some(data)) | (Some("message"), Some(data)) => {
                serde_json::from_str::<Message>(data)
                    .ok()
                    .map(SseEvent::Message)
            }
            _ => None,
        }
    }

    async fn handle_sse_chunk(
        chunk: Bytes,
        tx: &mpsc::Sender<Message>,
        session_id: &Arc<Mutex<Option<String>>>,
    ) -> Result<()> {
        let event = String::from_utf8(chunk.to_vec())?;
        let sse_event = Self::parse_sse_message(&event)
            .context(format!("sse_event is not recognised {event}"))?;
        match sse_event {
            SseEvent::Message(message) => {
                debug!("Received SSE message: {:?}", message);
                tx.send(message).await?;
            }
            SseEvent::SessionId(id) => {
                debug!("Received session ID: {}", id);
                *session_id.lock().await = Some(id);
            }
        }

        Ok(())
    }
}

#[derive(Default)]
pub struct ClientSseTransportBuilder {
    server_url: String,
    auth_config: Option<AuthConfig>,
    headers: HashMap<String, String>,
}

impl ClientSseTransportBuilder {
    pub fn new(server_url: String) -> Self {
        Self {
            server_url,
            auth_config: None,
            headers: HashMap::new(),
        }
    }

    pub fn with_auth(mut self, jwt_secret: String) -> Self {
        self.auth_config = Some(AuthConfig { jwt_secret });
        self
    }

    pub fn with_header(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.insert(key.into(), value.into());
        self
    }

    pub fn build(self) -> ClientSseTransport {
        let (tx, rx) = mpsc::channel(100);
        ClientSseTransport {
            tx,
            rx: Arc::new(Mutex::new(rx)),
            server_url: self.server_url,
            client: reqwest::Client::new(),
            auth_config: self.auth_config,
            session_id: Arc::new(Mutex::new(None)),
            headers: self.headers,
        }
    }
}

#[async_trait]
impl Transport for ClientSseTransport {
    async fn receive(&self) -> Result<Option<Message>> {
        let mut rx = self.rx.lock().await;
        match rx.recv().await {
            Some(message) => {
                debug!("Received SSE message: {:?}", message);
                Ok(Some(message))
            }
            None => Ok(None),
        }
    }

    async fn send(&self, message: &Message) -> Result<()> {
        let session_id = self
            .session_id
            .lock()
            .await
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("No session ID available"))?
            .clone();

        let request = self
            .client
            .post(format!(
                "{}/message?sessionId={}",
                self.server_url, session_id
            ))
            .json(message);

        let request = self.add_auth_header(request).await?;
        let response = request.send().await?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await?;
            return Err(anyhow::anyhow!(
                "Failed to send message, status: {status}, body: {text}",
            ));
        }

        Ok(())
    }

    async fn open(&self) -> Result<()> {
        let tx = self.tx.clone();
        let server_url = self.server_url.clone();
        let auth_config = self.auth_config.clone();
        let session_id = self.session_id.clone();
        let headers = self.headers.clone();

        let handle = tokio::spawn(async move {
            let mut request = reqwest::Client::new().get(&format!("{}/sse", server_url));

            // Add custom headers
            for (key, value) in &headers {
                request = request.header(key, value);
            }

            // Add auth header if configured
            if let Some(auth_config) = auth_config {
                let claims = Claims {
                    iat: SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs() as usize,
                    exp: SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs() as usize + 3600,
                };

                let token = encode(
                    &Header::default(),
                    &claims,
                    &EncodingKey::from_secret(auth_config.jwt_secret.as_bytes()),
                )?;

                request = request.header("Authorization", format!("Bearer {}", token));
            }

            let mut event_stream = request.send().await?.bytes_stream();

            // Handle first message to get session ID
            if let Some(first_chunk) = event_stream.next().await {
                match first_chunk {
                    Ok(bytes) => Self::handle_sse_chunk(bytes, &tx, &session_id).await?,
                    Err(e) => {
                        return Err(anyhow::anyhow!("Failed to get initial SSE message: {}", e))
                    }
                }
            } else {
                return Err(anyhow::anyhow!(
                    "SSE connection closed before receiving initial message"
                ));
            }

            // Handle remaining messages
            while let Some(chunk) = event_stream.next().await {
                if let Ok(bytes) = chunk {
                    if let Err(e) = Self::handle_sse_chunk(bytes, &tx, &session_id).await {
                        debug!("Error handling SSE message: {:?}", e);
                    }
                }
            }

            Ok::<_, anyhow::Error>(())
        });

        // Wait for the session ID to be set
        let mut attempts = 0;
        while attempts < 10 {
            if self.session_id.lock().await.is_some() {
                return Ok(());
            }
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            attempts += 1;
        }

        handle.abort();
        Err(anyhow::anyhow!("Timeout waiting for initial SSE message"))
    }

    async fn close(&self) -> Result<()> {
        Ok(())
    }
}
