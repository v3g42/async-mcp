//! Ollama Bridge Implementation
//! 
//! This module provides conversion between MCP tools and Ollama's function call format,
//! which follows OpenAI's function calling specification.

use super::{Function, FunctionDefinition, FunctionResponse, ToolCall, FunctionCall, ToolExecution, ToolResponse, mcp_to_function, tool_call_to_mcp, mcp_to_function_response};
use crate::transport::error::TransportError;
use serde::{Deserialize, Serialize};

/// Convert MCP tools to Ollama function format
pub fn convert_tools_for_ollama(tools: &[Tool]) -> serde_json::Value {
    let functions = mcp_to_function(tools);
    serde_json::json!({
        "functions": functions,
        "function_call": "auto"
    })
}

/// Parse Ollama response to extract function calls
pub fn parse_ollama_response(response: &str) -> Result<Option<ToolExecution>, TransportError> {
    // Look for function call pattern in response
    if let Some(function_call) = extract_function_call(response) {
        let tool_call = ToolCall {
            id: "0".to_string(), // Ollama doesn't provide IDs, so we use a default
            function: FunctionCall {
                name: function_call.name,
                arguments: serde_json::to_string(&function_call.arguments).unwrap_or_default(),
            },
        };
        Ok(Some(tool_call_to_mcp(&tool_call)?))
    } else {
        Ok(None)
    }
}

/// Format tool response for Ollama
pub fn format_ollama_response(tool_name: &str, response: &ToolResponse) -> String {
    let function_response = mcp_to_function_response(tool_name, response);
    serde_json::to_string(&function_response).unwrap_or_else(|_| "{}".to_string())
}

/// Helper to extract function calls from Ollama's response format
#[derive(Debug, Deserialize)]
struct OllamaFunctionCall {
    name: String,
    arguments: serde_json::Value,
}

fn extract_function_call(response: &str) -> Option<OllamaFunctionCall> {
    // This regex looks for function call patterns in Ollama's response
    // You would need to adapt this to match Ollama's actual format
    let re = regex::Regex::new(r"<function>(?P<name>[^<]+)</function>\s*<args>(?P<args>[^<]+)</args>").ok()?;
    
    if let Some(caps) = re.captures(response) {
        let name = caps.name("name")?.as_str().to_string();
        let args = caps.name("args")?.as_str();
        
        if let Ok(arguments) = serde_json::from_str(args) {
            return Some(OllamaFunctionCall { name, arguments });
        }
    }
    
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Tool;

    #[test]
    fn test_convert_tools() {
        let tools = vec![Tool {
            name: "test_tool".to_string(),
            description: Some("A test tool".to_string()),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "arg1": { "type": "string" }
                },
                "required": ["arg1"]
            }),
        }];

        let ollama_format = convert_tools_for_ollama(&tools);
        assert!(ollama_format.get("functions").is_some());
        assert_eq!(ollama_format.get("function_call").unwrap(), "auto");
    }

    #[test]
    fn test_parse_response() {
        let response = r#"<function>test_tool</function><args>{"arg1": "test"}</args>"#;
        let execution = parse_ollama_response(response).unwrap().unwrap();
        assert_eq!(execution.name, "test_tool");
        assert_eq!(execution.arguments, serde_json::json!({"arg1": "test"}));
    }
}
