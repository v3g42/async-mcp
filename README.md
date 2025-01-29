# Async MCP
A minimalistic async Rust implementation of the Model Context Protocol (MCP). This library extends the synchronous implementation from [mcp-sdk](https://github.com/AntigmaLabs/mcp-sdk) to support async operations and implements additional transports. Due to significant code changes, it is released as a separate crate.

[![Crates.io](https://img.shields.io/crates/v/async-mcp)](https://crates.io/crates/async-mcp)

> **Note**: This project is still early in development.

## Overview
This is an implementation of the [Model Context Protocol](https://github.com/modelcontextprotocol) defined by Anthropic.

## Features

### Supported Transports
- Server-Sent Events (SSE)
- Standard IO (Stdio) 
- In-Memory Channel

## Usage Examples

### Server Implementation

#### Using Stdio Transport
```rust
let server = Server::builder(StdioTransport)
    .capabilities(ServerCapabilities {
        tools: Some(json!({})),
        ..Default::default()
    })
    .request_handler("tools/list", list_tools)
    .request_handler("tools/call", call_tool)
    .request_handler("resources/list", |_req: ListRequest| {
        Ok(ResourcesListResponse {
            resources: vec![],
            next_cursor: None,
            meta: None,
        })
    })
    .build();
```

#### Using SSE Transport
```rust
run_sse_server(3004, None, |transport| async move {
    let server = build_server(transport);
    Ok(server)
})
.await?;
```

### Client Implementation

#### Setting up Transport
```rust
// Stdio Transport
let transport = ClientStdioTransport::new("<CMD>", &[])?;

// In-Memory Transport
let transport = ClientInMemoryTransport::new(|t| tokio::spawn(inmemory_server(t)));

// SSE Transport
let transport = ClientSseTransport::new(server_url);
```

#### Making Requests
```rust
// Initialize transport
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
```

## Complete Examples
For full working examples, check out:
- [Ping Pong Example](./examples/pingpong/)
- [File System Example](examples/file_system/README.md)
- [Knowledge Graph Memory Example](examples/knowledge_graph_memory/README.md)

## Related SDKs

### Official
- [TypeScript SDK](https://github.com/modelcontextprotocol/typescript-sdk)
- [Python SDK](https://github.com/modelcontextprotocol/python-sdk)

### Community
- [Go SDK](https://github.com/mark3labs/mcp-go)

For the complete feature set, please refer to the [MCP specification](https://spec.modelcontextprotocol.io/).

## Implementation Status

### Core Protocol Features
- [x] Basic Message Types
- [ ] Error and Signal Handling
- [x] Transport Layer
  - [x] Stdio
  - [x] In-Memory Channel
  - [x] SSE

### Server Features
- [x] Tools Support
- [ ] Prompts
- [ ] Resources
  - [x] Pagination
  - [x] Completion

### Client Features
Compatible with Claude Desktop:
- [x] Stdio Support
- [x] In-Memory Channel
- [x] SSE Support

### Monitoring
- [ ] Logging
- [ ] Metrics

### Utilities
- [ ] Ping
- [ ] Cancellation
- [ ] Progress Tracking
