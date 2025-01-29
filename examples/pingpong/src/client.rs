use std::time::Duration;

use anyhow::Result;
use async_mcp::{
    protocol::RequestOptions,
    transport::{ClientInMemoryTransport, ClientSseTransport, ClientStdioTransport, Transport},
};
use clap::{Parser, ValueEnum};
use pingpong::inmemory_server;
use serde_json::json;
use tracing::info;
#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Transport type to use
    #[arg(value_enum, default_value_t = TransportType::Stdio)]
    transport: TransportType,
}

#[derive(Copy, Clone, PartialEq, Eq, ValueEnum)]
enum TransportType {
    Stdio,
    InMemory,
    Sse,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .with_writer(std::io::stderr)
        .init();

    let cli = Cli::parse();

    let response = match cli.transport {
        TransportType::Stdio => {
            // Build the server first
            // cargo build --bin pingpong_server
            let transport = ClientStdioTransport::new("./target/debug/pingpong", &[])?;
            transport.open().await?;
            // Create and start client
            let client = async_mcp::client::ClientBuilder::new(transport.clone()).build();
            let client_clone = client.clone();
            let _client_handle = tokio::spawn(async move { client_clone.start().await });

            tokio::time::sleep(Duration::from_millis(100)).await;
            // Make a request
            client
                .request(
                    "tools/call",
                    Some(json!({"name": "ping", "arguments": {}})),
                    RequestOptions::default().timeout(Duration::from_secs(5)),
                )
                .await?
        }
        TransportType::Sse => {
            let transport = ClientSseTransport::new("http://localhost:3004".to_string());
            transport.open().await?;
            // Create and start client
            let client = async_mcp::client::ClientBuilder::new(transport.clone()).build();
            let client_clone = client.clone();
            let _client_handle = tokio::spawn(async move { client_clone.start().await });

            // Make a request
            client
                .request(
                    "tools/call",
                    Some(json!({"name": "ping", "arguments": {}})),
                    RequestOptions::default().timeout(Duration::from_secs(5)),
                )
                .await?
        }
        TransportType::InMemory => {
            let client_transport =
                ClientInMemoryTransport::new(|t| tokio::spawn(inmemory_server(t)));
            client_transport.open().await?;
            let client = async_mcp::client::ClientBuilder::new(client_transport.clone()).build();
            let client_clone = client.clone();
            let _client_handle = tokio::spawn(async move { client_clone.start().await });

            // Make a request
            client
                .request(
                    "tools/call",
                    Some(json!({"name": "ping", "arguments": {}})),
                    RequestOptions::default().timeout(Duration::from_secs(5)),
                )
                .await?
        }
    };
    info!("response: {response}");
    Ok(())
}
