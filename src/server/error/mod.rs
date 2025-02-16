use serde::{Deserialize, Serialize};
use std::fmt;

/// Standard JSON-RPC error codes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCode {
    // Standard JSON-RPC error codes
    ParseError = -32700,
    InvalidRequest = -32600,
    MethodNotFound = -32601,
    InvalidParams = -32602,
    InternalError = -32603,

    // MCP-specific error codes
    ConnectionClosed = -1,
    RequestTimeout = -2,
}

impl fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ErrorCode::ParseError => write!(f, "Parse error"),
            ErrorCode::InvalidRequest => write!(f, "Invalid request"),
            ErrorCode::MethodNotFound => write!(f, "Method not found"),
            ErrorCode::InvalidParams => write!(f, "Invalid parameters"),
            ErrorCode::InternalError => write!(f, "Internal error"),
            ErrorCode::ConnectionClosed => write!(f, "Connection closed"),
            ErrorCode::RequestTimeout => write!(f, "Request timeout"),
        }
    }
}

/// A JSON-RPC error response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcError {
    /// The error code
    pub code: i32,
    /// A short description of the error
    pub message: String,
    /// Additional information about the error
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

impl JsonRpcError {
    /// Create a new JSON-RPC error
    pub fn new(code: ErrorCode, message: impl Into<String>) -> Self {
        Self {
            code: code as i32,
            message: message.into(),
            data: None,
        }
    }

    /// Create a new JSON-RPC error with additional data
    pub fn with_data(
        code: ErrorCode,
        message: impl Into<String>,
        data: impl Into<serde_json::Value>,
    ) -> Self {
        Self {
            code: code as i32,
            message: message.into(),
            data: Some(data.into()),
        }
    }
}

/// A signal handler for graceful shutdown
pub trait SignalHandler: Send + Sync {
    /// Handle a shutdown signal
    fn handle_shutdown(&self) -> anyhow::Result<()>;
}

/// A registered signal handler
pub(crate) struct RegisteredSignalHandler {
    /// The handler for shutdown signals
    pub handler: Box<dyn SignalHandler>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_codes() {
        assert_eq!(ErrorCode::ParseError as i32, -32700);
        assert_eq!(ErrorCode::InvalidRequest as i32, -32600);
        assert_eq!(ErrorCode::MethodNotFound as i32, -32601);
        assert_eq!(ErrorCode::InvalidParams as i32, -32602);
        assert_eq!(ErrorCode::InternalError as i32, -32603);
    }

    #[test]
    fn test_error_display() {
        assert_eq!(ErrorCode::ParseError.to_string(), "Parse error");
        assert_eq!(ErrorCode::InvalidRequest.to_string(), "Invalid request");
        assert_eq!(ErrorCode::MethodNotFound.to_string(), "Method not found");
        assert_eq!(ErrorCode::InvalidParams.to_string(), "Invalid parameters");
        assert_eq!(ErrorCode::InternalError.to_string(), "Internal error");
    }

    #[test]
    fn test_json_rpc_error() {
        let error = JsonRpcError::new(ErrorCode::ParseError, "Failed to parse JSON");
        assert_eq!(error.code, -32700);
        assert_eq!(error.message, "Failed to parse JSON");
        assert!(error.data.is_none());

        let error = JsonRpcError::with_data(
            ErrorCode::InvalidParams,
            "Invalid parameters",
            serde_json::json!({
                "missing": ["param1", "param2"]
            }),
        );
        assert_eq!(error.code, -32602);
        assert_eq!(error.message, "Invalid parameters");
        assert!(error.data.is_some());
    }
}
