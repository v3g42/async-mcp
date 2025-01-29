# Ping/Pong exmaple with various transports

This example demonstrates a simple ping/pong 

### Tools

- **ping**
  - Responds with pong


## Test locally
```
cat << 'EOF' | cargo run --bin pingpong
{"jsonrpc": "2.0", "method": "tools/call", "params": {"name": "ping"}, "id": 1}
EOF
```