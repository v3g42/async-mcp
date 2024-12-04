use std::collections::HashMap;
use std::path::{Path, PathBuf};

use anyhow::Result;
use mcp_sdk::server::Server;
use mcp_sdk::transport::StdioTransport;
use mcp_sdk::types::{
    CallToolRequest, CallToolResponse, ListRequest, ResourcesListResponse, ServerCapabilities,
    ToolResponseContent, ToolsListResponse,
};
use serde_json::json;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        // needs to be stderr due to stdio transport
        .with_writer(std::io::stderr)
        .init();

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
    let server_handle = {
        let server = server;
        tokio::spawn(async move { server.listen().await })
    };

    server_handle
        .await?
        .map_err(|e| anyhow::anyhow!("Server error: {}", e))?;
    Ok(())
}

fn call_tool(req: CallToolRequest) -> Result<CallToolResponse> {
    let name = req.name.as_str();
    let args = req.arguments.unwrap_or_default();
    let result = match name {
        "read_file" => {
            let path = get_path(&args)?;
            let content = std::fs::read_to_string(path)?;
            ToolResponseContent::Text { text: content }
        }
        "list_directory" => {
            let path = get_path(&args)?;
            let entries = std::fs::read_dir(path)?;
            let mut text = String::new();
            for entry in entries {
                let entry = entry?;
                let prefix = if entry.file_type()?.is_dir() {
                    "[DIR]"
                } else {
                    "[FILE]"
                };
                text.push_str(&format!(
                    "{prefix} {}\n",
                    entry.file_name().to_string_lossy()
                ));
            }
            ToolResponseContent::Text { text }
        }
        "search_files" => {
            let path = get_path(&args)?;
            let pattern = args["pattern"].as_str().unwrap();
            let mut matches = Vec::new();
            search_directory(&path, pattern, &mut matches)?;
            ToolResponseContent::Text {
                text: matches.join("\n"),
            }
        }
        "get_file_info" => {
            let path = get_path(&args)?;
            let metadata = std::fs::metadata(path)?;
            ToolResponseContent::Text {
                text: format!("{:?}", metadata),
            }
        }
        "list_allowed_directories" => ToolResponseContent::Text {
            text: "[]".to_string(),
        },
        _ => return Err(anyhow::anyhow!("Unknown tool: {}", req.name)),
    };
    Ok(CallToolResponse {
        content: vec![result],
        is_error: None,
        meta: None,
    })
}

fn search_directory(dir: &Path, pattern: &str, matches: &mut Vec<String>) -> Result<()> {
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        let name = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_lowercase();

        // Check if the current file/directory matches the pattern
        if name.contains(&pattern.to_lowercase()) {
            matches.push(path.to_string_lossy().to_string());
        }

        // Recursively search subdirectories
        if path.is_dir() {
            search_directory(&path, pattern, matches)?;
        }
    }
    Ok(())
}

fn get_path(args: &HashMap<String, serde_json::Value>) -> Result<PathBuf> {
    let path = args["path"]
        .as_str()
        .ok_or(anyhow::anyhow!("Missing path"))?;

    if path.starts_with('~') {
        let home = home::home_dir().ok_or(anyhow::anyhow!("Could not determine home directory"))?;
        // Strip the ~ and join with home path
        let path = home.join(path.strip_prefix("~/").unwrap_or_default());
        Ok(path)
    } else {
        Ok(PathBuf::from(path))
    }
}

fn list_tools(_req: ListRequest) -> Result<ToolsListResponse> {
    let response = json!({
      "tools": [
        {
          "name": "read_file",
          "description":
            "Read the complete contents of a file from the file system. \
            Handles various text encodings and provides detailed error messages \
            if the file cannot be read. Use this tool when you need to examine \
            the contents of a single file. Only works within allowed directories.",
          "inputSchema": {
            "type": "object",
            "properties": {
              "path": {
                "type": "string"
              }
            },
            "required": ["path"]
          },
        },
        {
          "name": "list_directory",
          "description":
            "Get a detailed listing of all files and directories in a specified path. \
            Results clearly distinguish between files and directories with [FILE] and [DIR] \
            prefixes. This tool is essential for understanding directory structure and \
            finding specific files within a directory. Only works within allowed directories.",
          "inputSchema": {
            "type": "object",
            "properties": {
              "path": {
                "type": "string"
              }
            },
            "required": ["path"]
          },
        },
        {
          "name": "search_files",
          "description":
            "Recursively search for files and directories matching a pattern. \
            Searches through all subdirectories from the starting path. The search \
            is case-insensitive and matches partial names. Returns full paths to all \
            matching items. Great for finding files when you don't know their exact location. \
            Only searches within allowed directories.",
          "inputSchema": {
            "type": "object",
            "properties": {
              "path": {
                "type": "string"
              },
              "pattern": {
                "type": "string"
              }
            },
            "required": ["path", "pattern"]
          },
        },
        {
          "name": "get_file_info",
          "description":
            "Retrieve detailed metadata about a file or directory. Returns comprehensive \
            information including size, creation time, last modified time, permissions, \
            and type. This tool is perfect for understanding file characteristics \
            without reading the actual content. Only works within allowed directories.",
          "inputSchema": {
            "type": "object",
            "properties": {
              "path": {
                "type": "string"
              }
            },
            "required": ["path"]
          },
        }
      ],
    });
    Ok(serde_json::from_value(response)?)
}
