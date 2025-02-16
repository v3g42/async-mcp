use std::{
    collections::HashMap,
    sync::{atomic::{AtomicU64, Ordering}, Arc, RwLock},
};

use crate::{
    registry::{ToolHandler, Tools},
    types::{CallToolRequest, CallToolResponse, ListRequest, Tool, ToolsListResponse},
};

use super::{
    protocol::{Protocol, ProtocolBuilder},
    transport::Transport,
    types::{
        ClientCapabilities, Implementation, InitializeRequest, InitializeResponse,
        Progress, ProgressValue, ServerCapabilities, LATEST_PROTOCOL_VERSION,
    },
};
use anyhow::Result;
use serde::{de::DeserializeOwned, Serialize};
use std::future::Future;
use std::pin::Pin;

#[derive(Clone)]
pub struct ServerState {
    client_capabilities: Option<ClientCapabilities>,
    client_info: Option<Implementation>,
    initialized: bool,
    progress_counter: Arc<AtomicU64>,
}

#[derive(Clone)]
pub struct Server<T: Transport> {
    protocol: Protocol<T>,
    state: Arc<RwLock<ServerState>>,
}

pub struct ServerBuilder<T: Transport> {
    protocol: ProtocolBuilder<T>,
    server_info: Implementation,
    capabilities: ServerCapabilities,
    tools: HashMap<String, ToolHandler>,
}

impl<T: Transport> ServerBuilder<T> {
    pub fn name<S: Into<String>>(mut self, name: S) -> Self {
        self.server_info.name = name.into();
        self
    }

    pub fn version<S: Into<String>>(mut self, version: S) -> Self {
        self.server_info.version = version.into();
        self
    }

    pub fn capabilities(mut self, capabilities: ServerCapabilities) -> Self {
        self.capabilities = capabilities;
        self
    }

    /// Register a typed request handler
    /// for higher-level api use add tool
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
        self.protocol = self.protocol.request_handler(method, handler);
        self
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
        self.protocol = self.protocol.notification_handler(method, handler);
        self
    }

    pub fn register_tool(
        &mut self,
        tool: Tool,
        f: impl Fn(CallToolRequest) -> Pin<Box<dyn Future<Output = Result<CallToolResponse>> + Send>>
            + Send
            + Sync
            + 'static,
    ) {
        self.tools.insert(
            tool.name.clone(),
            ToolHandler {
                tool,
                f: Box::new(f),
            },
        );
    }

    pub fn build(self) -> Server<T> {
        Server::new(self)
    }
}

impl<T: Transport> Server<T> {
    // Progress support methods
    pub async fn create_progress(&self, value: ProgressValue) -> Result<String> {
        let token = format!(
            "progress-{}",
            self.state
                .read()
                .map_err(|_| anyhow::anyhow!("Lock poisoned"))?
                .progress_counter
                .fetch_add(1, Ordering::SeqCst)
        );

        let progress = Progress {
            token: token.clone(),
            value,
            meta: None,
        };

        self.protocol
            .notify("$/progress", Some(serde_json::to_value(progress)?))
            .await?;

        Ok(token)
    }

    pub async fn update_progress(&self, token: String, value: ProgressValue) -> Result<()> {
        let progress = Progress {
            token,
            value,
            meta: None,
        };

        self.protocol
            .notify("$/progress", Some(serde_json::to_value(progress)?))
            .await
    }

    pub fn builder(transport: T) -> ServerBuilder<T> {
        ServerBuilder {
            protocol: Protocol::builder(transport),
            server_info: Implementation {
                name: env!("CARGO_PKG_NAME").to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
            },
            capabilities: Default::default(),
            tools: HashMap::new(),
        }
    }

    fn new(builder: ServerBuilder<T>) -> Self {
        let state = Arc::new(RwLock::new(ServerState {
            client_capabilities: None,
            client_info: None,
            initialized: false,
            progress_counter: Arc::new(AtomicU64::new(0)),
        }));

        // Initialize protocol with handlers
        let mut protocol = builder
            .protocol
            .request_handler(
                "initialize",
                Self::handle_init(state.clone(), builder.server_info, builder.capabilities),
            )
            .notification_handler(
                "notifications/initialized",
                Self::handle_initialized(state.clone()),
            );

        // Add tools handlers if not already present
        if !protocol.has_request_handler("tools/list") {
            let tools = Arc::new(Tools::new(builder.tools));
            let tools_clone = tools.clone();
            let tools_list = tools.clone();
            let tools_call = tools_clone.clone();

            protocol = protocol
                .request_handler("tools/list", move |_req: ListRequest| {
                    let tools = tools_list.clone();
                    Box::pin(async move {
                        Ok(ToolsListResponse {
                            tools: tools.list_tools(),
                            next_cursor: None,
                            meta: None,
                        })
                    })
                })
                .request_handler("tools/call", move |req: CallToolRequest| {
                    let tools = tools_call.clone();
                    Box::pin(async move { tools.call_tool(req).await })
                });
        }

        Server {
            protocol: protocol.build(),
            state,
        }
    }

    // Helper function for initialize handler
    fn handle_init(
        state: Arc<RwLock<ServerState>>,
        server_info: Implementation,
        capabilities: ServerCapabilities,
    ) -> impl Fn(
        InitializeRequest,
    )
        -> Pin<Box<dyn std::future::Future<Output = Result<InitializeResponse>> + Send>> {
        move |req| {
            let state = state.clone();
            let server_info = server_info.clone();
            let capabilities = capabilities.clone();

            Box::pin(async move {
                let mut state = state
                    .write()
                    .map_err(|_| anyhow::anyhow!("Lock poisoned"))?;
                state.client_capabilities = Some(req.capabilities);
                state.client_info = Some(req.client_info);

                Ok(InitializeResponse {
                    protocol_version: LATEST_PROTOCOL_VERSION.to_string(),
                    capabilities,
                    server_info,
                })
            })
        }
    }

    // Helper function for initialized handler
    fn handle_initialized(
        state: Arc<RwLock<ServerState>>,
    ) -> impl Fn(()) -> Pin<Box<dyn std::future::Future<Output = Result<()>> + Send>> {
        move |_| {
            let state = state.clone();
            Box::pin(async move {
                let mut state = state
                    .write()
                    .map_err(|_| anyhow::anyhow!("Lock poisoned"))?;
                state.initialized = true;
                Ok(())
            })
        }
    }

    pub fn get_client_capabilities(&self) -> Option<ClientCapabilities> {
        self.state.read().ok()?.client_capabilities.clone()
    }

    pub fn get_client_info(&self) -> Option<Implementation> {
        self.state.read().ok()?.client_info.clone()
    }

    pub fn is_initialized(&self) -> bool {
        self.state
            .read()
            .ok()
            .map(|state| state.initialized)
            .unwrap_or(false)
    }

    pub async fn listen(&self) -> Result<()> {
        self.protocol.listen().await
    }
}
