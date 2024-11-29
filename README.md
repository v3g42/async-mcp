# Model Context Protocol (MCP)
Minimalistic Rust Implementation Of Model Context Protocol(MCP).
Main repo from Anthropic: [MCP](https://github.com/modelcontextprotocol)

## Minimalistic approach
Given it is still very early stage of MCP adoption, the goal is to remain agile and easy to understand.
This implementation favors simplicity and ease of understanding to capture the core idea of MCP while maintaining compatibility with Claude Desktop.
Some guidelines:
- use primitive building blocks and avoid framework if possible
- keep it simple and stupid

## Other Sdks

- [typescript-sdk](https://github.com/modelcontextprotocol/typescript-sdk)
- [python-sdk](https://github.com/modelcontextprotocol/python-sdk)

For complete feature please refer to the [MCP specification](https://spec.modelcontextprotocol.io/).
## Features
### Basic Protocol
- [x] Basic Message Types
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