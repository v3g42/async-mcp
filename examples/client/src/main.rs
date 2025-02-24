use std::time::Duration;

use anyhow::Result;
use async_mcp::{
    client::ClientBuilder,
    protocol::RequestOptions,
    transport::{ClientStdioTransport, Transport},
};

#[tokio::main]
async fn main() -> Result<()> {
    #[cfg(unix)]
    {
        // Create transport connected to cat command which will stay alive
        let transport = ClientStdioTransport::new("cat", &[], None)?;

        // Open transport
        transport.open().await?;

        let client = ClientBuilder::new(transport).build();
        let client_clone = client.clone();
        tokio::spawn(async move { client_clone.start().await });
        let response = client
            .request(
                "echo",
                None,
                RequestOptions::default().timeout(Duration::from_secs(1)),
            )
            .await?;
        println!("{:?}", response);
    }
    #[cfg(windows)]
    {
        println!("Windows is not supported yet");
    }
    Ok(())
}
