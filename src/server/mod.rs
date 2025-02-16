mod mcp;
pub mod completion;
pub mod error;
pub mod notifications;
pub mod prompt;
pub mod requests;
pub mod resource;
pub mod roots;
pub mod sampling;
pub mod tool;

pub use mcp::McpServer;
pub use error::{ErrorCode, JsonRpcError, SignalHandler};
pub use notifications::{Notification, NotificationHandler, NotificationSender};
pub use requests::{Request, RequestHandler};

use crate::types::{Implementation, ServerCapabilities};
use crate::Transport;
use anyhow::Result;
use std::sync::Arc;

/// A server that implements the Model Context Protocol
pub struct Server {
    server_info: Implementation,
    capabilities: ServerCapabilities,
    request_handler: Option<Arc<dyn RequestHandler>>,
    notification_handler: Option<Arc<dyn NotificationHandler>>,
    notification_sender: Option<Arc<dyn NotificationSender>>,
    signal_handler: Option<Arc<dyn SignalHandler>>,
}

impl Server {
    /// Create a new server with the given implementation info
    pub fn new(server_info: Implementation) -> Self {
        Self {
            server_info,
            capabilities: Default::default(),
            request_handler: None,
            notification_handler: None,
            notification_sender: None,
            signal_handler: None,
        }
    }

    /// Set the signal handler
    pub fn set_signal_handler(&mut self, handler: impl SignalHandler + 'static) {
        self.signal_handler = Some(Arc::new(handler));
    }

    /// Handle a shutdown signal
    pub fn handle_shutdown(&self) -> anyhow::Result<()> {
        if let Some(handler) = &self.signal_handler {
            handler.handle_shutdown()?;
        }
        Ok(())
    }

    /// Connect to the given transport and start listening for messages
    pub async fn connect(&self, transport: impl Transport) -> Result<()> {
        // TODO: Implement actual server connection logic
        Ok(())
    }

    /// Register new capabilities
    pub fn register_capabilities(&mut self, capabilities: ServerCapabilities) {
        self.capabilities = capabilities;
    }

    /// Set the request handler
    pub fn set_request_handler(&mut self, handler: impl RequestHandler + 'static) {
        self.request_handler = Some(Arc::new(handler));
    }

    /// Set the notification handler
    pub fn set_notification_handler(&mut self, handler: impl NotificationHandler + 'static) {
        self.notification_handler = Some(Arc::new(handler));
    }

    /// Set the notification sender
    pub fn set_notification_sender(&mut self, sender: impl NotificationSender + 'static) {
        self.notification_sender = Some(Arc::new(sender));
    }

    /// Send a notification to clients
    pub fn send_notification(&self, notification: Notification) -> Result<()> {
        if let Some(sender) = &self.notification_sender {
            sender.send(notification)?;
        }
        Ok(())
    }
}
