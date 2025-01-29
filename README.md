# Model Context Protocol (MCP)
Minimalistic Async Rust Implementation Of Model Context Protocol(MCP). 
This extends `sync` implementation of [mcp-sdk](https://github.com/AntigmaLabs/mcp-sdk) to `async` and implements additional transports required for our use. As this ended up changing the code significantly releasing it as a different crate. 

[![Crates.io](https://img.shields.io/crates/v/async-mcp)](https://crates.io/crates/async-mcp)


MCP protocol defined by Anthropic: [MCP](https://github.com/modelcontextprotocol)

| Note: Still early in development. 

### Features
Supported Transports
- SSE
- Stdio
- InMemory

## Examples
Refer to [pinpong](./examples/pingpong/) for a full working example.

### Building a Server with Stdio Transport
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
### Building a Server with SSE Transport
```rust
run_sse_server(3004, None, |transport| async move {
    // Similar to the above example except here we use SSE Transport
    let server = build_server(transport);
    Ok(server)
})
.await?;
```

## Client 
```rust
// Create transport 

// Stdio
let transport = ClientStdioTransport::new("<CMD>", &[])?;

// InMemory
let transport = ClientInMemoryTransport::new(|t| tokio::spawn(inmemory_server(t)));

//SSE
let transport = ClientSseTransport::new(server_url);
```

Request tools/call
```rust
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

For complete examples, see:
- [File System Example](examples/file_system/README.md)
- [Knowledge Graph Memory Example](examples/knowledge_graph_memory/README.md)

## Other SDKs
### Official
- [typescript-sdk](https://github.com/modelcontextprotocol/typescript-sdk)
- [python-sdk](https://github.com/modelcontextprotocol/python-sdk)

### Community
- [go-sdk](https://github.com/mark3labs/mcp-go)

For complete feature set, please refer to the [MCP specification](https://spec.modelcontextprotocol.io/).

## Features
### Basic Protocol
- [x] Basic Message Types
- [ ] Error and Signal Handling
- Transport
    - [x] Stdio
    - [x] In Memory Channel 
    - [x] SSE
- Utilities 
    - [ ] Ping
    - [ ] Cancellation
    - [ ] Progress
### Server
- [x] Tools
- [ ] Prompts
- [ ] Resources
    - [x] Pagination
    - [x] Completion
### Client
Compatible with Claude Desktop.
- [x] Stdio
- [x] In Memory Channel 
- [x] SSE

### Monitoring
- [ ] Logging
- [ ] Metrics
