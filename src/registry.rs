use crate::types::{CallToolRequest, CallToolResponse, Tool};
use anyhow::Result;
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;

// Type alias for tool handler map
type ToolHandlerMap = HashMap<String, ToolHandler>;

pub struct Tools {
    tool_handlers: ToolHandlerMap,
}

impl Tools {

    pub fn get_tool(&self, name: &str) -> Option<Tool> {
        self.tool_handlers
            .get(name)
            .map(|tool_handler| tool_handler.tool.clone())
    }

    pub async fn call_tool(&self, req: CallToolRequest) -> Result<CallToolResponse> {
        let handler = self
            .tool_handlers
            .get(&req.name)
            .ok_or_else(|| anyhow::anyhow!("Tool not found: {}", req.name))?;

        (handler.f)(req).await
    }

    pub fn list_tools(&self) -> Vec<Tool> {
        self.tool_handlers
            .values()
            .map(|tool_handler| tool_handler.tool.clone())
            .collect()
    }
}

// Type aliases for complex future and handler types
type ToolFuture = Pin<Box<dyn Future<Output = Result<CallToolResponse>> + Send>>;
type ToolHandlerFn = Box<dyn Fn(CallToolRequest) -> ToolFuture + Send + Sync>;

// Struct for storing tool handlers
pub(crate) struct ToolHandler {
    pub tool: Tool,
    pub f: ToolHandlerFn,
}
