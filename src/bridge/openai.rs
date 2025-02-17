//! MCP to OpenAI Function Call Bridge
//! 
//! This module provides conversion between MCP tools and OpenAI function calls.
//! It allows MCP tools to be used with any LLM that supports OpenAI's function calling format.

use crate::transport::error::TransportError;
use crate::types::{Tool, ServerCapabilities};
use serde::{Deserialize, Serialize};

/// OpenAI function definition format
#[derive(Debug, Serialize, Deserialize)]
pub struct FunctionDefinition {
    pub name: String,
    pub description: Option<String>,
    pub parameters: serde_json::Value,
    #[serde(default = "default_strict")]
    pub strict: bool,
}

/// OpenAI function format
#[derive(Debug, Serialize, Deserialize)]
pub struct Function {
    #[serde(rename = "type")]
    pub function_type: String,
    pub function: FunctionDefinition,
}

fn default_strict() -> bool {
    true
}

/// OpenAI function response format
#[derive(Debug, Serialize, Deserialize)]
pub struct FunctionResponse {
    pub name: String,
    pub content: String,
}

/// Convert MCP tools to OpenAI function format
pub fn mcp_to_function(tools: &[Tool]) -> Vec<Function> {
    tools.iter().map(|tool| {
        // Get the base schema
        let mut parameters = tool.input_schema.clone();
        
        // Add additionalProperties: false if it's not already set
        if let Some(obj) = parameters.as_object_mut() {
            if !obj.contains_key("additionalProperties") {
                obj.insert("additionalProperties".to_string(), serde_json::Value::Bool(false));
            }
        }

        Function {
            function_type: "function".to_string(),
            function: FunctionDefinition {
                name: tool.name.clone(),
                description: tool.description.clone(),
                parameters,
                strict: true,
            },
        }
    }).collect()
}

/// OpenAI tool call format
#[derive(Debug, Deserialize)]
pub struct ToolCall {
    pub id: String,
    pub function: FunctionCall,
}

/// OpenAI function call format in responses
#[derive(Debug, Deserialize)]
pub struct FunctionCall {
    pub name: String,
    pub arguments: String,
}

/// Convert OpenAI tool call to MCP tool execution
pub fn tool_call_to_mcp(tool_call: &ToolCall) -> Result<ToolExecution, TransportError> {
    // Parse the arguments string as JSON
    let arguments = serde_json::from_str(&tool_call.function.arguments).map_err(|e| {
        TransportError::new(
            crate::transport::error::TransportErrorCode::InvalidMessage,
            format!("Failed to parse function arguments: {}", e)
        )
    })?;

    Ok(ToolExecution {
        name: tool_call.function.name.clone(),
        arguments,
    })
}

/// Convert MCP tool response to OpenAI function response
pub fn mcp_to_function_response(tool_name: &str, response: &ToolResponse) -> FunctionResponse {
    FunctionResponse {
        name: tool_name.to_string(),
        content: if let Some(error) = &response.error {
            format!("Error: {}", error)
        } else {
            serde_json::to_string(&response.result).unwrap_or_else(|_| "{}".to_string())
        },
    }
}

/// MCP message format for tool execution
#[derive(Debug, Serialize, Deserialize)]
pub struct ToolExecution {
    pub name: String,
    pub arguments: serde_json::Value,
}

/// MCP message format for tool response  
#[derive(Debug, Serialize, Deserialize)]
pub struct ToolResponse {
    pub result: serde_json::Value,
    pub error: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mcp_to_function() {
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

        let functions = mcp_to_function(&tools);
        assert_eq!(functions.len(), 1);
        assert_eq!(functions[0].function_type, "function");
        assert_eq!(functions[0].function.name, "test_tool");
        assert_eq!(functions[0].function.description, Some("A test tool".to_string()));
        assert!(functions[0].function.strict);
    }

    #[test]
    fn test_function_to_mcp() {
        let function = Function {
            function_type: "function".to_string(),
            function: FunctionDefinition {
                name: "test_tool".to_string(),
                description: Some("A test tool".to_string()),
                parameters: serde_json::json!({
                    "arg1": "test"
                }),
                strict: true,
            },
        };

        let execution = function_to_mcp(&function).unwrap();
        assert_eq!(execution.name, "test_tool");
        assert_eq!(execution.arguments, serde_json::json!({"arg1": "test"}));
    }

    #[test]
    fn test_mcp_to_function_response() {
        let response = ToolResponse {
            result: serde_json::json!({"output": "test"}),
            error: None,
        };

        let function_response = mcp_to_function_response("test_tool", &response);
        assert_eq!(function_response.name, "test_tool");
        assert_eq!(function_response.content, r#"{"output":"test"}"#);

        let error_response = ToolResponse {
            result: serde_json::Value::Null,
            error: Some("Test error".to_string()),
        };

        let function_response = mcp_to_function_response("test_tool", &error_response);
        assert_eq!(function_response.content, "Error: Test error");
    }
}
