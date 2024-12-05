use crate::{
    protocol::{Protocol, ProtocolBuilder, RequestOptions},
    transport::Transport,
};

use anyhow::Result;

pub struct Client<T: Transport> {
    protocol: Protocol<T>,
}

impl<T: Transport> Client<T> {
    pub fn builder(transport: T) -> ClientBuilder<T> {
        ClientBuilder::new(transport)
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
