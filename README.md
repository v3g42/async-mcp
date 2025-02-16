# Async MCP
The most advanced and complete implementation of the Model Context Protocol (MCP) specification. This Rust implementation goes beyond the standard specification to provide:

- **Full Specification Coverage**: Implements every feature from the latest MCP spec
- **Production-Grade Error Handling**: Comprehensive error system with recovery mechanisms
- **Advanced Transport Layer**: Robust implementations of all transport types with detailed error tracking
- **Type-Safe Architecture**: Leveraging Rust's type system for compile-time correctness
- **Real-World Ready**: Production-tested with Claude Desktop compatibility

This library sets the standard for MCP implementations with its comprehensive feature set and robust error handling.

[![Crates.io](https://img.shields.io/crates/v/async-mcp)](https://crates.io/crates/async-mcp)

> **Note**: While this implementation provides the most complete coverage of the MCP specification, including features like sampling, roots, and completion that are not yet available in other implementations, it is still under active development.

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
async-mcp = "0.0.6"
```

## Overview
This is an implementation of the [Model Context Protocol](https://github.com/modelcontextprotocol) defined by Anthropic.

## Features

### Supported Transports
- Server-Sent Events (SSE)
- Standard IO (Stdio) 
- In-Memory Channel
- Websockets

## Usage Examples

### OpenAI Function Call Bridge
The implementation provides a seamless bridge between MCP tools and LLM provider function calling formats:

#### Bridge Architecture
The bridge layer automatically handles conversion between MCP tools and provider-specific formats:
- Users only need to define tools using the MCP format
- Bridge handles all provider-specific conversions internally
- No need to know or work with provider-specific formats

#### Function Call Format
- Automatic conversion of MCP tools to OpenAI function format
- Handles function definitions, parameter validation, and response formatting
- Supports strict mode with proper JSON schema validation
- Type-safe conversion with comprehensive error handling

#### Provider Support
- **OpenAI Integration**
  - Full support for all tool choice options (auto/required/none/specific)
  - Parallel function calling support
  - Streaming support for real-time function calls
  
- **Ollama Integration**
  - Automatic adaptation to Ollama's capabilities
  - Handles Ollama's "auto-only" tool choice limitation
  - Function call extraction with regex-based parsing
  - Compatible response formatting

The bridge abstracts away provider differences, allowing you to write provider-agnostic code while the bridge handles the specific requirements and limitations of each LLM provider.

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

#### Run Http Server supporting both SSE and WS 
```rust
run_http_server(3004, None, |transport| async move {
    let server = build_server(transport);
    Ok(server)
})
.await?;
```

Local Endpoints
```
WebSocket endpoint: ws://127.0.0.1:3004/ws
SSE endpoint: http://127.0.0.1:3004/sse
```

### Client Implementation

#### Setting up Transport
```rust
// Stdio Transport
let transport = ClientStdioTransport::new("<CMD>", &[])?;

// In-Memory Transport
let transport = ClientInMemoryTransport::new(|t| tokio::spawn(inmemory_server(t)));

// SSE Transport
let transport = ClientSseTransportBuilder::new(server_url).build();

// WS Transport
let transport = async_mcp::transport::ClientWsTransportBuilder::new("ws://localhost:3004/ws".to_string()).build();
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

## Using MCP Servers

### Installing Available Servers
MCP servers can be installed and run using npm. For example:
```bash
# Install and run Brave Search MCP server
npx -y @modelcontextprotocol/server-brave-search

# Install and run GitHub MCP server
npx -y @modelcontextprotocol/server-github

# Install and run NPM Search server
npx -y npm-search-mcp-server
```

### Connecting to Servers
Once servers are running, you can connect to them using the client:

```rust
// Example: Using Brave Search server
let transport = ClientSseTransportBuilder::new("http://localhost:3000/sse").build();
let client = async_mcp::client::ClientBuilder::new(transport.clone()).build();

// Make a search request
let response = client
    .request(
        "tools/call",
        Some(json!({
            "name": "brave_web_search",
            "arguments": {
                "query": "Rust programming language",
                "count": 5
            }
        })),
        RequestOptions::default(),
    )
    .await?;
```

### Available Servers
Common MCP servers include:
- **Brave Search**: Web search capabilities
- **GitHub**: Repository management and code search
- **NPM Search**: Package search and metadata
- **File System**: Local file operations
- **Memory**: Knowledge graph and data persistence
- **Weather**: Weather data and forecasts
- **Playwright**: Browser automation and testing

Each server provides its own set of tools and resources that can be used through the MCP protocol. Check individual server documentation for specific capabilities and usage details.

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
- [x] Error and Signal Handling
  - [x] JSON-RPC Error Codes
  - [x] Error Data Support
  - [x] Graceful Shutdown
  - [x] Signal Handlers
  - [x] Transport-specific Errors
- [x] Transport Layer
  - [x] Stdio (with error handling)
  - [x] In-Memory Channel (with error handling)
  - [x] SSE (with error handling)
  - [x] Websockets (with error handling)
  - [x] Detailed Error Codes
  - [x] Error Recovery

### Server Features
- [x] Tools Support
- [x] Prompts Support
  - [x] Arguments
  - [x] Templates
  - [x] List Changed Notifications
- [x] Resources Support
  - [x] Pagination
  - [x] Templates
  - [x] Subscriptions
  - [x] Update Notifications
- [x] Completion Support
  - [x] Resource Completion
  - [x] Prompt Completion
  - [x] Argument Completion
- [x] Sampling Support
  - [x] Model Preferences
  - [x] Context Inclusion
  - [x] System Prompts
- [x] Roots Support
  - [x] URI-based Roots
  - [x] Change Notifications

### Client Features
- [x] Claude Desktop Support
  - [x] Stdio Transport
  - [x] In-Memory Channel
  - [x] SSE Support
  - [x] Websocket Support
- [x] MCP Bridge Protocol
  - [x] Tool Registration Format
  - [x] Tool Execution Format
  - [x] Tool Response Format
  - [x] Message Conversion
  - [x] Error Handling

### Notification Support
- [x] Resource Updates
- [x] Resource List Changes
- [x] Tool List Changes
- [x] Prompt List Changes
- [x] Roots List Changes
- [x] Progress Updates
- [x] Cancellation

### Monitoring
- [x] Logging Support
  - [x] Level Control
  - [x] Message Notifications
- [ ] Metrics

### Utilities
- [x] Ping Support
- [x] Cancellation Support
- [x] Progress Tracking
  - [x] Progress Notifications
  - [x] Progress Tokens
  - [x] Progress Values
