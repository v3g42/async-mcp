use super::{Message, Transport};
use super::Result;
use super::error::{TransportError, TransportErrorCode};
use actix_ws::{Message as WsMessage, Session};
use async_trait::async_trait;
use futures::{SinkExt, StreamExt};
use reqwest::header::{HeaderName, HeaderValue};
use std::sync::Arc;
use std::{collections::HashMap, str::FromStr};
use tokio::sync::{broadcast, Mutex};
use tokio_tungstenite::tungstenite::{client::IntoClientRequest, Message as TungsteniteMessage};
use tracing::{debug, info};

#[derive(Clone)]
pub struct ServerWsTransport {
    session: Arc<Mutex<Option<Session>>>,
    rx: Arc<Mutex<Option<broadcast::Receiver<Message>>>>,
}

impl ServerWsTransport {
    pub fn new(session: Session, rx: broadcast::Receiver<Message>) -> Self {
        Self {
            session: Arc::new(Mutex::new(Some(session))),
            rx: Arc::new(Mutex::new(Some(rx))),
        }
    }
}

#[derive(Clone)]
pub struct ClientWsTransport {
    ws_tx: Arc<Mutex<Option<broadcast::Sender<Message>>>>,
    ws_rx: Arc<Mutex<Option<broadcast::Receiver<Message>>>>,
    url: String,
    headers: HashMap<String, String>,
    ws_write: Arc<
        Mutex<
            Option<
                futures::stream::SplitSink<
                    tokio_tungstenite::WebSocketStream<
                        tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
                    >,
                    TungsteniteMessage,
                >,
            >,
        >,
    >,
}

impl ClientWsTransport {
    pub fn builder(url: String) -> ClientWsTransportBuilder {
        ClientWsTransportBuilder::new(url)
    }
}

#[derive(Default)]
pub struct ClientWsTransportBuilder {
    url: String,
    headers: HashMap<String, String>,
}

impl ClientWsTransportBuilder {
    pub fn new(url: String) -> Self {
        Self {
            url,
            headers: HashMap::new(),
        }
    }

    pub fn with_header(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.insert(key.into(), value.into());
        self
    }

    pub fn build(self) -> ClientWsTransport {
        let (tx, rx) = broadcast::channel(100);
        ClientWsTransport {
            ws_tx: Arc::new(Mutex::new(Some(tx))),
            ws_rx: Arc::new(Mutex::new(Some(rx))),
            url: self.url,
            headers: self.headers,
            ws_write: Arc::new(Mutex::new(None)),
        }
    }
}

#[async_trait]
impl Transport for ServerWsTransport {
    async fn receive(&self) -> Result<Option<Message>> {
        if let Some(rx) = self.rx.lock().await.as_mut() {
            match rx.recv().await {
                Ok(msg) => {
                    debug!("Server received message: {:?}", msg);
                    Ok(Some(msg))
                }
                Err(e) => {
                    debug!("Server receive error: {}", e);
                    Ok(None)
                }
            }
        } else {
            debug!("Server receive called but receiver is None");
            Ok(None)
        }
    }

    async fn send(&self, message: &Message) -> Result<()> {
        let text = serde_json::to_string(message).map_err(|e| 
            TransportError::new(TransportErrorCode::InvalidMessage, format!("Failed to serialize message: {}", e)))?;
        if let Some(session) = self.session.lock().await.as_mut() {
            debug!("Server sending message: {}", text);
            session.text(text).await.map_err(|e| 
                TransportError::new(TransportErrorCode::MessageSendFailed, format!("Failed to send WebSocket message: {}", e)))?;
        } else {
            debug!("Server send called but session is None");
            return Err(TransportError::new(TransportErrorCode::InvalidState, "WebSocket session not initialized"));
        }
        Ok(())
    }

    async fn open(&self) -> Result<()> {
        Ok(())
    }

    async fn close(&self) -> Result<()> {
        info!("Server WebSocket connection closing");
        if let Some(session) = self.session.lock().await.take() {
            session.close(None).await.map_err(|e| 
                TransportError::new(TransportErrorCode::ConnectionClosed, format!("Failed to close WebSocket connection: {}", e)))?;
        }
        Ok(())
    }
}

#[async_trait]
impl Transport for ClientWsTransport {
    async fn receive(&self) -> Result<Option<Message>> {
        if let Some(rx) = self.ws_rx.lock().await.as_mut() {
            match rx.recv().await {
                Ok(msg) => {
                    debug!("Client received message: {:?}", msg);
                    Ok(Some(msg))
                }
                Err(e) => {
                    debug!("Client receive error: {}", e);
                    Ok(None)
                }
            }
        } else {
            debug!("Client receive called but receiver is None");
            Ok(None)
        }
    }

    async fn send(&self, message: &Message) -> Result<()> {
        let text = serde_json::to_string(message).map_err(|e| 
            TransportError::new(TransportErrorCode::InvalidMessage, format!("Failed to serialize message: {}", e)))?;
        if let Some(write) = self.ws_write.lock().await.as_mut() {
            debug!("Client sending message: {}", text);
            write.send(TungsteniteMessage::Text(text.into())).await.map_err(|e| 
                TransportError::new(TransportErrorCode::MessageSendFailed, format!("Failed to send WebSocket message: {}", e)))?;
        } else {
            debug!("Client send called but writer is None");
            return Err(TransportError::new(TransportErrorCode::InvalidState, "WebSocket writer not initialized"));
        }
        Ok(())
    }

    async fn open(&self) -> Result<()> {
        info!("Opening WebSocket connection to {}", self.url);

        let mut request = self.url.clone().into_client_request().map_err(|e| 
            TransportError::new(TransportErrorCode::WebSocketProtocolError, format!("Invalid WebSocket URL: {}", e)))?;
        
        // MCP servers seem to be expecting this as protocol
        request.headers_mut().insert(
            "Sec-WebSocket-Protocol",
            HeaderValue::from_str("mcp").map_err(|e| 
                TransportError::new(TransportErrorCode::WebSocketProtocolError, format!("Invalid protocol header: {}", e)))?,
        );
        
        for (k, v) in &self.headers {
            request.headers_mut().insert(
                HeaderName::from_str(k).map_err(|e| 
                    TransportError::new(TransportErrorCode::WebSocketProtocolError, format!("Invalid header name {}: {}", k, e)))?,
                HeaderValue::from_str(v).map_err(|e| 
                    TransportError::new(TransportErrorCode::WebSocketProtocolError, format!("Invalid header value {}: {}", v, e)))?,
            );
        }
        
        let (ws_stream, response) = tokio_tungstenite::connect_async(request).await
            .map_err(|e| TransportError::new(TransportErrorCode::WebSocketUpgradeFailed, format!("Failed to establish WebSocket connection: {}", e)))?;

        info!(
            "WebSocket connection established. Response status: {}",
            response.status()
        );
        debug!("WebSocket response headers: {:?}", response.headers());

        let (write, read) = ws_stream.split();
        *self.ws_write.lock().await = Some(write);

        // Get channels for WebSocket communication
        let ws_tx = self
            .ws_tx
            .lock()
            .await
            .as_ref()
            .ok_or_else(|| TransportError::new(TransportErrorCode::InvalidState, "WebSocket sender not initialized"))?
            .clone();

        // Handle receiving messages from WebSocket
        tokio::spawn(async move {
            let mut read = read;
            while let Some(result) = read.next().await {
                match result {
                    Ok(msg) => {
                        if let TungsteniteMessage::Text(text) = msg {
                            match serde_json::from_str::<Message>(&text) {
                                Ok(message) => {
                                    debug!("Received WebSocket message: {:?}", message);
                                    // Send to the broadcast channel for the transport to receive
                                    if let Err(e) = ws_tx.send(message) {
                                        debug!("Failed to forward WebSocket message: {}", e);
                                    }
                                }
                                Err(e) => debug!("Failed to parse WebSocket message: {}", e),
                            }
                        }
                    }
                    Err(e) => {
                        info!("WebSocket read error: {}", e);
                        break;
                    }
                }
            }
            info!("WebSocket read loop terminated");
        });

        Ok(())
    }

    async fn close(&self) -> Result<()> {
        info!("Closing WebSocket connection");
        self.ws_tx.lock().await.take();
        self.ws_rx.lock().await.take();
        Ok(())
    }
}

pub async fn handle_ws_connection(
    mut session: Session,
    mut stream: actix_ws::MessageStream,
    tx: broadcast::Sender<Message>,
    mut rx: broadcast::Receiver<Message>,
) -> Result<()> {
    info!("New WebSocket connection established");

    loop {
        tokio::select! {
            Some(Ok(msg)) = stream.next() => {
                if let WsMessage::Text(text) = msg {
                    match serde_json::from_str::<Message>(&text) {
                        Ok(message) => {
                            debug!("Handler received message: {:?}", message);
                            tx.send(message).map_err(|e| 
                                TransportError::new(TransportErrorCode::MessageSendFailed, format!("Failed to forward message: {}", e)))?;
                        }
                        Err(e) => debug!("Failed to parse message in handler: {}", e),
                    }
                }
            }
            Ok(message) = rx.recv() => {
                debug!("Handler sending message: {:?}", message);
                let text = serde_json::to_string(&message).map_err(|e| 
                    TransportError::new(TransportErrorCode::InvalidMessage, format!("Failed to serialize message: {}", e)))?;
                session.text(text).await.map_err(|e| 
                    TransportError::new(TransportErrorCode::MessageSendFailed, format!("Failed to send WebSocket message: {}", e)))?;
            }
            else => {
                info!("WebSocket connection terminated");
                break
            }
        }
    }
    Ok(())
}
