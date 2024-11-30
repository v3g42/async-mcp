# Model Context Protocol (MCP)
Minimalistic Rust Implementation Of Model Context Protocol(MCP).

Main repo from Anthropic: [MCP](https://github.com/modelcontextprotocol)

## Minimalistic approach
Given it is still very early stage of MCP adoption, the goal is to remain agile and easy to understand.
This implementation aims to capture the core idea of MCP while maintaining compatibility with Claude Desktop.
Many optional features are not implemented yet.

Some guidelines:
- use primitive building blocks and avoid framework if possible
- keep it simple and stupid

## Other Sdks

### Official
- [typescript-sdk](https://github.com/modelcontextprotocol/typescript-sdk)
- [python-sdk](https://github.com/modelcontextprotocol/python-sdk)

### Community
- [go-sdk](https://github.com/mark3labs/mcp-go)

For complete feature please refer to the [MCP specification](https://spec.modelcontextprotocol.io/).
## Features
### Basic Protocol
- [x] Basic Message Types
- [ ] Error and Signal Handling
- Transport
    - [x] Stdio
    - [ ] In Memory Channel (not yet supported in formal specification)
    - [ ] SSE
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
- [ ] Examples
### Client
For now use claude desktop as client.

### Monitoring
- [ ] Logging
- [ ] Metrics

### Examples
- [x] See [examples/file_system/README.md](examples/file_system/README.md) for usage examples and documentation