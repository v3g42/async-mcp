use anyhow::Result;
use async_mcp::{run_http_server, transport::ServerStdioTransport};
use clap::{Parser, ValueEnum};
use file_system::server::build_server;

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
    Sse,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        // needs to be stderr due to stdio transport
        .with_writer(std::io::stderr)
        .init();

    let cli = Cli::parse();

    match cli.transport {
        TransportType::Stdio => {
            let server = build_server(ServerStdioTransport);
            server
                .listen()
                .await
                .map_err(|e| anyhow::anyhow!("Server error: {}", e))?;
        }
        TransportType::Sse => {
            run_http_server(3004, None, |transport, _| async move {
                let server = build_server(transport);
                Ok(server)
            })
            .await?;
        }
    };
    Ok(())
}
