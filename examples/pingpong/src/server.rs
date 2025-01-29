use anyhow::Result;
use async_mcp::server::Server;
use async_mcp::transport::Transport;
use async_mcp::types::{
    CallToolRequest, CallToolResponse, ListRequest, ResourcesListResponse, ServerCapabilities,
    ToolResponseContent, ToolsListResponse,
};
use serde_json::json;

pub fn build_server<T: Transport>(t: T) -> Server<T> {
    Server::builder(t)
        .capabilities(ServerCapabilities {
            tools: Some(json!({})),
            ..Default::default()
        })
        .request_handler("tools/list", |req: ListRequest| {
            Box::pin(async move { list_tools(req) })
        })
        .request_handler("tools/call", |req: CallToolRequest| {
            Box::pin(async move { call_tool(req) })
        })
        .request_handler("resources/list", |_req: ListRequest| {
            Box::pin(async move {
                Ok(ResourcesListResponse {
                    resources: vec![],
                    next_cursor: None,
                    meta: None,
                })
            })
        })
        .build()
}

fn list_tools(_req: ListRequest) -> Result<ToolsListResponse> {
    let response = json!({
    "tools": [
      {
        "name": "ping",
        "description": "Send a ping to get a pong response",
        "inputSchema": {
          "type": "object",
          "properties": {},
          "required": []
        },
      },
    ]});
    Ok(serde_json::from_value(response)?)
}

fn call_tool(req: CallToolRequest) -> Result<CallToolResponse> {
    let name = req.name.as_str();
    let result = match name {
        "ping" => ToolResponseContent::Text {
            text: "pong".to_string(),
        },
        _ => return Err(anyhow::anyhow!("Unknown tool: {}", req.name)),
    };
    Ok(CallToolResponse {
        content: vec![result],
        is_error: None,
        meta: None,
    })
}
