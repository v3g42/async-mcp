use crate::{
    protocol::{Protocol, ProtocolBuilder, RequestOptions},
    transport::Transport,
    types::{
        ClientCapabilities, Implementation, InitializeRequest, InitializeResponse,
        LATEST_PROTOCOL_VERSION,
    },
};

use anyhow::Result;
use tracing::debug;

#[derive(Clone)]
pub struct Client<T: Transport> {
    protocol: Protocol<T>,
}

impl<T: Transport> Client<T> {
    pub fn builder(transport: T) -> ClientBuilder<T> {
        ClientBuilder::new(transport)
    }

    pub async fn initialize(&self, client_info: Implementation) -> Result<InitializeResponse> {
        let request = InitializeRequest {
            protocol_version: LATEST_PROTOCOL_VERSION.to_string(),
            capabilities: ClientCapabilities::default(),
            client_info,
        };
        let response = self
            .request(
                "initialize",
                Some(serde_json::to_value(request)?),
                RequestOptions::default(),
            )
            .await?;
        let response: InitializeResponse = serde_json::from_value(response)
            .map_err(|e| anyhow::anyhow!("Failed to parse response: {}", e))?;

        if response.protocol_version != LATEST_PROTOCOL_VERSION {
            return Err(anyhow::anyhow!(
                "Unsupported protocol version: {}",
                response.protocol_version
            ));
        }

        debug!(
            "Initialized with protocol version: {}",
            response.protocol_version
        );
        self.protocol.notify("notifications/initialized", None)?;
        Ok(response)
    }

    pub async fn request(
        &self,
        method: &str,
        params: Option<serde_json::Value>,
        options: RequestOptions,
    ) -> Result<serde_json::Value> {
        let response = self.protocol.request(method, params, options).await?;
        response
            .result
            .ok_or_else(|| anyhow::anyhow!("Request failed: {:?}", response.error))
    }

    pub async fn start(&self) -> Result<()> {
        self.protocol.listen().await
    }
}

pub struct ClientBuilder<T: Transport> {
    protocol: ProtocolBuilder<T>,
}

impl<T: Transport> ClientBuilder<T> {
    pub fn new(transport: T) -> Self {
        Self {
            protocol: ProtocolBuilder::new(transport),
        }
    }

    pub fn build(self) -> Client<T> {
        Client {
            protocol: self.protocol.build(),
        }
    }
}
