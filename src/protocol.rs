use super::transport::{
    JsonRpcError, JsonRpcMessage, JsonRpcNotification, JsonRpcRequest, JsonRpcResponse, Transport,
};
use super::types::ErrorCode;
use anyhow::anyhow;
use anyhow::Result;
use async_trait::async_trait;
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::pin::Pin;
use std::sync::atomic::Ordering;
use std::time::Duration;
use std::{
    collections::HashMap,
    sync::{atomic::AtomicU64, Arc},
};
use tokio::sync::oneshot;
use tokio::sync::Mutex;
use tokio::time::timeout;
use tracing::debug;

#[derive(Clone)]
pub struct Protocol<T: Transport> {
    transport: Arc<T>,

    request_id: Arc<AtomicU64>,
    pending_requests: Arc<Mutex<HashMap<u64, oneshot::Sender<JsonRpcResponse>>>>,
    request_handlers: Arc<Mutex<HashMap<String, Box<dyn RequestHandler>>>>,
    notification_handlers: Arc<Mutex<HashMap<String, Box<dyn NotificationHandler>>>>,
}

impl<T: Transport> Protocol<T> {
    pub fn builder(transport: T) -> ProtocolBuilder<T> {
        ProtocolBuilder::new(transport)
    }

    pub async fn notify(&self, method: &str, params: Option<serde_json::Value>) -> Result<()> {
        let notification = JsonRpcNotification {
            method: method.to_string(),
            params,
            ..Default::default()
        };
        let msg = JsonRpcMessage::Notification(notification);
        self.transport.send(&msg).await?;
        Ok(())
    }

    pub async fn request(
        &self,
        method: &str,
        params: Option<serde_json::Value>,
        options: RequestOptions,
    ) -> Result<JsonRpcResponse> {
        let id = self.request_id.fetch_add(1, Ordering::SeqCst);

        // Create a oneshot channel for this request
        let (tx, rx) = oneshot::channel();

        // Store the sender
        {
            let mut pending = self.pending_requests.lock().await;
            pending.insert(id, tx);
        }

        // Send the request
        let msg = JsonRpcMessage::Request(JsonRpcRequest {
            id,
            method: method.to_string(),
            params,
            ..Default::default()
        });
        self.transport.send(&msg).await?;

        // Wait for response with timeout
        match timeout(options.timeout, rx)
            .await
            .map_err(|_| anyhow!("Request timed out"))?
        {
            Ok(response) => Ok(response),
            Err(_) => {
                // Clean up the pending request if receiver was dropped
                let mut pending = self.pending_requests.lock().await;
                pending.remove(&id);
                Err(anyhow!("Request cancelled"))
            }
        }
    }

    pub async fn listen(&self) -> Result<()> {
        debug!("Listening for requests");
        loop {
            let message = self.transport.receive().await;

            let message = match message {
                Ok(msg) => msg,
                Err(e) => {
                    tracing::error!("Failed to parse message: {:?}", e);
                    continue;
                }
            };

            // Exit loop when transport signals shutdown with None
            if message.is_none() {
                break;
            }

            match message.unwrap() {
                JsonRpcMessage::Request(request) => self.handle_request(request).await?,
                JsonRpcMessage::Response(response) => {
                    let id = response.id;
                    let mut pending = self.pending_requests.lock().await;
                    if let Some(tx) = pending.remove(&id) {
                        let _ = tx.send(response);
                    }
                }
                JsonRpcMessage::Notification(notification) => {
                    let handlers = self.notification_handlers.lock().await;
                    if let Some(handler) = handlers.get(&notification.method) {
                        handler.handle(notification).await?;
                    }
                }
            }
        }
        Ok(())
    }

    async fn handle_request(&self, request: JsonRpcRequest) -> Result<()> {
        let handlers = self.request_handlers.lock().await;
        if let Some(handler) = handlers.get(&request.method) {
            match handler.handle(request.clone()).await {
                Ok(response) => {
                    let msg = JsonRpcMessage::Response(response);
                    self.transport.send(&msg).await?;
                }
                Err(e) => {
                    let error_response = JsonRpcResponse {
                        id: request.id,
                        result: None,
                        error: Some(JsonRpcError {
                            code: ErrorCode::InternalError as i32,
                            message: e.to_string(),
                            data: None,
                        }),
                        ..Default::default()
                    };
                    let msg = JsonRpcMessage::Response(error_response);
                    self.transport.send(&msg).await?;
                }
            }
        } else {
            self.transport
                .send(&JsonRpcMessage::Response(JsonRpcResponse {
                    id: request.id,
                    error: Some(JsonRpcError {
                        code: ErrorCode::MethodNotFound as i32,
                        message: format!("Method not found: {}", request.method),
                        data: None,
                    }),
                    ..Default::default()
                }))
                .await?;
        }
        Ok(())
    }
}

/// The default request timeout, in milliseconds
pub const DEFAULT_REQUEST_TIMEOUT_MSEC: u64 = 60000;
pub struct RequestOptions {
    timeout: Duration,
}

impl RequestOptions {
    pub fn timeout(self, timeout: Duration) -> Self {
        Self { timeout }
    }
}

impl Default for RequestOptions {
    fn default() -> Self {
        Self {
            timeout: Duration::from_millis(DEFAULT_REQUEST_TIMEOUT_MSEC),
        }
    }
}

pub struct ProtocolBuilder<T: Transport> {
    transport: T,
    request_handlers: HashMap<String, Box<dyn RequestHandler>>,
    notification_handlers: HashMap<String, Box<dyn NotificationHandler>>,
}
impl<T: Transport> ProtocolBuilder<T> {
    pub fn new(transport: T) -> Self {
        Self {
            transport,
            request_handlers: HashMap::new(),
            notification_handlers: HashMap::new(),
        }
    }
    /// Register a typed request handler
    pub fn request_handler<Req, Resp>(
        mut self,
        method: &str,
        handler: impl Fn(Req) -> Pin<Box<dyn std::future::Future<Output = Result<Resp>> + Send>>
            + Send
            + Sync
            + 'static,
    ) -> Self
    where
        Req: DeserializeOwned + Send + Sync + 'static,
        Resp: Serialize + Send + Sync + 'static,
    {
        let handler = TypedRequestHandler {
            handler: Box::new(handler),
            _phantom: std::marker::PhantomData,
        };

        self.request_handlers
            .insert(method.to_string(), Box::new(handler));
        self
    }

    pub fn has_request_handler(&self, method: &str) -> bool {
        self.request_handlers.contains_key(method)
    }

    pub fn notification_handler<N>(
        mut self,
        method: &str,
        handler: impl Fn(N) -> Pin<Box<dyn std::future::Future<Output = Result<()>> + Send>>
            + Send
            + Sync
            + 'static,
    ) -> Self
    where
        N: DeserializeOwned + Send + Sync + 'static,
    {
        self.notification_handlers.insert(
            method.to_string(),
            Box::new(TypedNotificationHandler {
                handler: Box::new(handler),
                _phantom: std::marker::PhantomData,
            }),
        );
        self
    }

    pub fn build(self) -> Protocol<T> {
        Protocol {
            transport: Arc::new(self.transport),
            request_handlers: Arc::new(Mutex::new(self.request_handlers)),
            notification_handlers: Arc::new(Mutex::new(self.notification_handlers)),
            request_id: Arc::new(AtomicU64::new(0)),
            pending_requests: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

// Update the handler traits to be async
#[async_trait]
trait RequestHandler: Send + Sync {
    async fn handle(&self, request: JsonRpcRequest) -> Result<JsonRpcResponse>;
}

#[async_trait]
trait NotificationHandler: Send + Sync {
    async fn handle(&self, notification: JsonRpcNotification) -> Result<()>;
}

// Update the TypedRequestHandler to use async handlers
struct TypedRequestHandler<Req, Resp>
where
    Req: DeserializeOwned + Send + Sync + 'static,
    Resp: Serialize + Send + Sync + 'static,
{
    handler: Box<
        dyn Fn(Req) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Resp>> + Send>>
            + Send
            + Sync,
    >,
    _phantom: std::marker::PhantomData<(Req, Resp)>,
}

#[async_trait]
impl<Req, Resp> RequestHandler for TypedRequestHandler<Req, Resp>
where
    Req: DeserializeOwned + Send + Sync + 'static,
    Resp: Serialize + Send + Sync + 'static,
{
    async fn handle(&self, request: JsonRpcRequest) -> Result<JsonRpcResponse> {
        let params: Req = if request.params.is_none() || request.params.as_ref().unwrap().is_null()
        {
            serde_json::from_value(serde_json::Value::Null)?
        } else {
            serde_json::from_value(request.params.unwrap())?
        };
        let result = (self.handler)(params).await?;
        Ok(JsonRpcResponse {
            id: request.id,
            result: Some(serde_json::to_value(result)?),
            error: None,
            ..Default::default()
        })
    }
}

struct TypedNotificationHandler<N>
where
    N: DeserializeOwned + Send + Sync + 'static,
{
    handler: Box<
        dyn Fn(N) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<()>> + Send>>
            + Send
            + Sync,
    >,
    _phantom: std::marker::PhantomData<N>,
}

#[async_trait]
impl<N> NotificationHandler for TypedNotificationHandler<N>
where
    N: DeserializeOwned + Send + Sync + 'static,
{
    async fn handle(&self, notification: JsonRpcNotification) -> Result<()> {
        let params: N =
            if notification.params.is_none() || notification.params.as_ref().unwrap().is_null() {
                serde_json::from_value(serde_json::Value::Null)?
            } else {
                match &notification.params {
                    Some(params) => {
                        let res = serde_json::from_value(params.clone());
                        match res {
                            Ok(r) => r,
                            Err(e) => {
                                tracing::warn!("Failed to parse notification params: {:?}. Params: {:?}", e, notification.params);
                                serde_json::from_value(serde_json::Value::Null)?
                            }
                        }
                    },
                    None => serde_json::from_value(serde_json::Value::Null)?,
                }
            };
        (self.handler)(params).await
    }
}
