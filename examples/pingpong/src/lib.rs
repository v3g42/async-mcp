use async_mcp::transport::ServerInMemoryTransport;
use server::build_server;

pub mod server;
pub async fn inmemory_server(transport: ServerInMemoryTransport) {
    let server = build_server(transport.clone());
    server.listen().await.unwrap();
}
