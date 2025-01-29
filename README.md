# Model Context Protocol (MCP)
Minimalistic Async Implementation Of Model Context Protocol(MCP). Original sync implementation from [async-mcp](https://github.com/AntigmaLabs/async-mcp)

[![Crates.io](https://img.shields.io/crates/v/async-mcp)](https://crates.io/crates/async-mcp)


Main repo from Anthropic: [MCP](https://github.com/modelcontextprotocol)
### Examples
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
- [x] See [examples/file_system/README.md](examples/file_system/README.md) for usage examples and documentation
## Other Sdks

### Official
- [typescript-sdk](https://github.com/modelcontextprotocol/typescript-sdk)
- [python-sdk](https://github.com/modelcontextprotocol/python-sdk)

For complete feature please refer to the [MCP specification](https://spec.modelcontextprotocol.io/).
## Features
### Basic Protocol
- [x] Basic Message Types
- [ ] Error and Signal Handling
- Transport
    - [x] Stdio
    - [x] In Memory Channel 
    - [x] SSE
    - [ ] More compact serialization format (not yet supported in formal specification)
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
For now use claude desktop as client.

### Monitoring
- [ ] Logging
- [ ] Metrics
