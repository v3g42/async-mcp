use crate::types::{CallToolRequest, CallToolResponse, Tool, ToolResponseContent};
use anyhow::Result;
use std::collections::HashMap;

pub struct Tools {
    tool_handlers: HashMap<String, ToolHandler>,
}

impl Tools {
    pub(crate) fn new(map: HashMap<String, ToolHandler>) -> Self {
        Self { tool_handlers: map }
    }

    pub fn get_tool(&self, name: &str) -> Option<Tool> {
        self.tool_handlers
            .get(name)
            .map(|tool_handler| tool_handler.tool.clone())
    }

    pub fn call_tool(&self, request: CallToolRequest) -> CallToolResponse {
        let request_name = request.name.clone();
        let handler = self.tool_handlers.get(&request_name);
        if handler.is_none() {
            return CallToolResponse {
                content: vec![ToolResponseContent::Text {
                    text: format!("Tool {} not found", request_name),
                }],
                is_error: Some(true),
                meta: None,
            };
        }

        let result = (handler.unwrap().f)(request);
        if result.is_err() {
            return CallToolResponse {
                content: vec![ToolResponseContent::Text {
                    text: format!(
                        "Error calling tool {}: {}",
                        request_name,
                        result.err().unwrap()
                    ),
                }],
                is_error: Some(true),
                meta: None,
            };
        }
        result.unwrap()
    }

    pub fn list_tools(&self) -> Vec<Tool> {
        self.tool_handlers
            .values()
            .map(|tool_handler| tool_handler.tool.clone())
            .collect()
    }
}

pub(crate) struct ToolHandler {
    pub tool: Tool,
    pub f: Box<dyn Fn(CallToolRequest) -> Result<CallToolResponse> + Send + Sync + 'static>,
}
