use std::sync::{Arc, RwLock};

use super::{
    protocol::{Protocol, ProtocolBuilder},
    transport::Transport,
    types::{
        ClientCapabilities, Implementation, InitializeRequest, InitializeResult,
        ServerCapabilities, LATEST_PROTOCOL_VERSION,
    },
};
use anyhow::Result;

#[derive(Clone)]
pub struct ServerState {
    client_capabilities: Option<ClientCapabilities>,
    client_info: Option<Implementation>,
    initialized: bool,
}

#[derive(Clone)]
pub struct Server<T: Transport> {
    protocol: Protocol<T>,
    state: Arc<RwLock<ServerState>>,
}
pub struct ServerOptions {
    server_info: Implementation,
    capabilities: ServerCapabilities,
}
impl Default for ServerOptions {
    fn default() -> Self {
        Self {
            server_info: Implementation {
                name: env!("CARGO_PKG_NAME").to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
            },
            capabilities: Default::default(),
        }
    }
}
impl ServerOptions {
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
}

impl<T: Transport> Server<T> {
    pub fn new(protocol: ProtocolBuilder<T>, options: ServerOptions) -> Self {
        let state = Arc::new(RwLock::new(ServerState {
            client_capabilities: None,
            client_info: None,
            initialized: false,
        }));

        // Initialize protocol with handlers
        let protocol = protocol
            .request_handler(
                "initialize",
                Self::handle_init(state.clone(), options.server_info, options.capabilities),
            )
            .notification_handler(
                "notifications/initialized",
                Self::handle_initialized(state.clone()),
            );

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
    ) -> impl Fn(InitializeRequest) -> Result<InitializeResult> {
        move |req| {
            let mut state = state
                .write()
                .map_err(|_| anyhow::anyhow!("Lock poisoned"))?;
            state.client_capabilities = Some(req.capabilities);
            state.client_info = Some(req.client_info);

            Ok(InitializeResult {
                protocol_version: LATEST_PROTOCOL_VERSION.to_string(),
                capabilities: capabilities.clone(),
                server_info: server_info.clone(),
            })
        }
    }

    // Helper function for initialized handler
    fn handle_initialized(state: Arc<RwLock<ServerState>>) -> impl Fn(()) -> Result<()> {
        move |_| {
            let mut state = state
                .write()
                .map_err(|_| anyhow::anyhow!("Lock poisoned"))?;
            state.initialized = true;
            Ok(())
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
