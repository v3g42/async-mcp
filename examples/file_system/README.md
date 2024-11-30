# Simple Example of A Read-Only File System

This example demonstrates a simple read-only file system. It allows you to list the contents of a directory and read the contents of a file.

similar to the [Typescript Example](https://github.com/modelcontextprotocol/servers/tree/main/src/filesystem) example, but with a read-only file system.

### Tools

- **read_file**
  - Read complete contents of a file
  - Input: `path` (string)
  - Reads complete file contents with UTF-8 encoding

- **list_directory**
  - List directory contents with [FILE] or [DIR] prefixes
  - Input: `path` (string)

- **search_files**
  - Recursively search for files/directories
  - Inputs:
    - `path` (string): Starting directory
    - `pattern` (string): Search pattern
  - Case-insensitive matching
  - Returns full paths to matches

- **get_file_info**
  - Get detailed file/directory metadata
  - Input: `path` (string)
  - Returns: metadata of the file or directory

## How to Build and Run Example Locally

### Prerequisites
- macOS (will handle Windows in the future)
- The latest version of Claude Desktop installed
- Install [Rust](https://www.rust-lang.org/tools/install)

### Build and Install Binary
```bash
cd mcp-sdk/examples/file_system
cargo install --path .
```
This will build the binary and install it to your local cargo bin directory. Later you will need to configure Claude Desktop to use this binary.
### Configure Claude Desktop

If you are using macOS, open the `claude_desktop_config.json` file in a text editor:
```bash
code ~/Library/Application\ Support/Claude/claude_desktop_config.json
```

Modify the `claude_desktop_config.json` file to include the following configuration:
(replace YOUR_USERNAME with your actual username):
```json
{
  "mcpServers": {
    "mcp_example_file_system": {
      "command": "/Users/YOUR_USERNAME/.cargo/bin/file_system"
    }
  }
}
```
Save the file, and restart Claude Desktop.
## What will it look like
<img width="546" alt="Screenshot 2024-11-30 at 12 44 19 PM" src="https://github.com/user-attachments/assets/24a9f249-1d79-4c34-ba65-59aa59705a2b">
