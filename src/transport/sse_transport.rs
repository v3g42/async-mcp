//! Server-sent events (SSE) transport implementation using actix-web-lab
//! This module provides a transport layer for server-sent events using the actix-web-lab crate.

use std::time::Duration;
use async_trait::async_trait;
use actix_web_lab::sse;
use bytestring::ByteString;
use serde::Serialize;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;

use crate::transport::{Message, Result, Transport};

/// Server-side SSE transport implementation
#[derive(Debug, Clone)]
pub struct ServerSseTransport {
    sender: mpsc::Sender<Result<sse::Event>>,
}

impl ServerSseTransport {
    /// Creates a new SSE transport with the given channel capacity
    pub fn new(capacity: usize) -> Self {
        let (tx, _) = mpsc::channel(capacity);
        Self { sender: tx }
    }

    /// Creates a new SSE transport with the given channel capacity and returns the transport and responder
    pub fn new_with_responder(capacity: usize) -> (Self, impl actix_web::Responder) {
        let (tx, rx) = mpsc::channel(capacity);
        let transport = Self { sender: tx };
        let responder = sse::Sse::from_stream(ReceiverStream::new(rx))
            .with_keep_alive(Duration::from_secs(15));
        (transport, responder)
    }

    /// Sends a message through the SSE channel
    pub async fn send_message(&self, message: Message) -> Result<()> {
        let json = serde_json::to_string(&message)?;
        self.sender
            .send(Ok(sse::Event::Data(sse::Data::new(json))))
            .await
            .map_err(|e| e.into())
    }

    /// Sends a data message through the SSE channel
    pub async fn send_data(&self, data: impl Into<String>) -> Result<()> {
        let data = ByteString::from(data.into());
        self.sender
            .send(Ok(sse::Event::Data(sse::Data::new(data))))
            .await
            .map_err(|e| e.into())
    }

    /// Sends a JSON-serialized data message through the SSE channel
    pub async fn send_json<T: Serialize>(&self, data: T) -> Result<()> {
        let json = serde_json::to_string(&data)?;
        let data = ByteString::from(json);
        self.sender
            .send(Ok(sse::Event::Data(sse::Data::new(data))))
            .await
            .map_err(|e| e.into())
    }

    /// Sends a named event with data through the SSE channel
    pub async fn send_event(&self, event: impl Into<String>, data: impl Into<String>) -> Result<()> {
        let data = ByteString::from(data.into());
        let event = ByteString::from(event.into());
        self.sender
            .send(Ok(sse::Event::Data(sse::Data::new(data).event(event))))
            .await
            .map_err(|e| e.into())
    }

    /// Sends a comment through the SSE channel
    pub async fn send_comment(&self, comment: impl Into<String>) -> Result<()> {
        let comment = ByteString::from(comment.into());
        self.sender
            .send(Ok(sse::Event::Comment(comment)))
            .await
            .map_err(|e| e.into())
    }
}

#[async_trait]
impl Transport for ServerSseTransport {
    async fn send(&self, message: &Message) -> Result<()> {
        self.send_message(message.clone()).await
    }

    async fn receive(&self) -> Result<Option<Message>> {
        // SSE is unidirectional, server to client only
        Ok(None)
    }

    async fn open(&self) -> Result<()> {
        Ok(())
    }

    async fn close(&self) -> Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    

    #[actix_web::test]
    async fn test_sse_transport() {
        let (transport, _responder) = ServerSseTransport::new_with_responder(100);

        // Test thinking message
        transport.send_event("thinking", "<thinking>Analyzing the snapshot implementation...</thinking>").await.unwrap();

        // Test tool use message
        transport.send_event("tool_use", r#"{
            "tool": "read_file",
            "params": {
                "path": "src/snapshot/manager.rs"
            }
        }"#).await.unwrap();

        // Test large code analysis in chunks
        transport.send_event("analysis", "Let me explain the snapshot implementation in detail.").await.unwrap();
        
        transport.send_event("analysis", r#"
First, let's look at the directory creation process:
1. The code creates directories in the snapshot for directory entries
2. It ensures parent directories exist for all files
3. This is crucial for hard link creation, as target_path must be valid
"#).await.unwrap();

        transport.send_event("analysis", r#"
Performance considerations:
- Testing with large projects (36k files, 756MB)
- Creation time is minimal due to hard links
- Restoration uses fs::copy, which takes longer but is manageable
"#).await.unwrap();

        transport.send_event("analysis", r#"
The exclusion system is comprehensive:
1. Uses global ignore patterns
2. Supports per-directory .gitignore files
3. The ignore crate handles .gitignore automatically
4. walk_builder.git_ignore(true) enables this feature
5. add_global_ignore handles additional exclusions
"#).await.unwrap();

        // Test code snippet with implementation details
        transport.send_event("code", r#"
impl SnapshotManager {
    pub fn create_snapshot(&self, path: &Path) -> Result<()> {
        let walker = WalkBuilder::new(path)
            .git_ignore(true)
            .build();
        
        for entry in walker {
            let entry = entry?;
            if entry.file_type()?.is_dir() {
                self.create_directory(&entry)?;
            } else {
                self.create_hard_link(&entry)?;
            }
        }
        Ok(())
    }
}
"#).await.unwrap();

        transport.send_event("analysis", r#"
The restoration process:
1. Uses fs::copy for independence
2. Time-consuming but necessary
3. Could use hard links for space efficiency
4. But requires same filesystem
5. Most users need independent copies
"#).await.unwrap();

        // Test streaming completion
        transport.send_event("completion", "This implementation provides several benefits:").await.unwrap();
        transport.send_event("completion", "\n1. Efficient storage through hard links").await.unwrap();
        transport.send_event("completion", "\n2. Proper handling of exclusions").await.unwrap();
        transport.send_event("completion", "\n3. Flexible restoration options").await.unwrap();

        // Test error handling
        transport.send_event("error", "Warning: Restoration to a different filesystem will use more space").await.unwrap();

        // Test final completion with summary
        transport.send_event("completion_end", r#"{
            "status": "success",
            "message": "Analysis complete. The snapshot system provides efficient storage with proper exclusion handling and flexible restoration options."
        }"#).await.unwrap();

        // Test keep-alive during long analysis
        transport.send_comment("keep-alive").await.unwrap();
    }

    #[actix_web::test]
    async fn test_sse_structured_response() {
        let (transport, _responder) = ServerSseTransport::new_with_responder(100);

        // Send structured analysis response
        transport.send_event("analysis_start", "Beginning code analysis...").await.unwrap();

        transport.send_json(serde_json::json!({
            "response": {
                "metadata": {
                    "model": "code-llm-v1",
                    "timestamp": "2024-02-19T10:30:00Z",
                    "request_id": "req_789xyz",
                    "confidence_score": 0.92,
                    "processing_time_ms": 150
                },
                "context": {
                    "language": "python",
                    "file_type": "source_code",
                    "relevant_symbols": [
                        "UserRepository",
                        "authenticate_user",
                        "hash_password"
                    ],
                    "imports_detected": [
                        "bcrypt",
                        "sqlalchemy"
                    ]
                },
                "analysis": {
                    "code_quality": {
                        "score": 0.85,
                        "issues": [
                            {
                                "type": "security",
                                "severity": "medium",
                                "description": "Password hashing should use work factor of 12 or higher",
                                "line_number": 45,
                                "suggested_fix": "bcrypt.hashpw(password, bcrypt.gensalt(12))"
                            }
                        ]
                    },
                    "performance_insights": [
                        {
                            "type": "database",
                            "description": "Consider adding index on frequently queried user_email column",
                            "impact": "high",
                            "recommendation": "CREATE INDEX idx_user_email ON users(email);"
                        }
                    ]
                },
                "suggestions": {
                    "code_completions": [
                        {
                            "snippet": "def validate_password(password: str) -> bool:\n    return len(password) >= 8 and any(c.isupper() for c in password)",
                            "confidence": 0.88,
                            "context": "password validation helper function",
                            "tags": ["security", "validation", "user-input"]
                        }
                    ],
                    "refactoring_options": [
                        {
                            "type": "extract_method",
                            "description": "Extract password validation logic into separate function",
                            "before": "if len(password) >= 8 and any(c.isupper() for c in password):",
                            "after": "if validate_password(password):",
                            "benefit": "Improves code reusability and testability"
                        }
                    ]
                },
                "references": {
                    "documentation": [
                        {
                            "title": "BCrypt Best Practices",
                            "url": "https://example.com/bcrypt-guide",
                            "relevance_score": 0.95
                        }
                    ],
                    "similar_code_patterns": [
                        {
                            "repository": "auth-service",
                            "file": "auth/security.py",
                            "similarity_score": 0.82,
                            "matched_lines": [42, 43, 44]
                        }
                    ]
                },
                "execution_context": {
                    "memory_usage_mb": 245,
                    "tokens_processed": 1024,
                    "cache_hit_ratio": 0.76,
                    "embeddings_generated": 12
                }
            }
        })).await.unwrap();

        // Send follow-up events to demonstrate streaming with structured data
        transport.send_event("progress", "Generating code improvements...").await.unwrap();
        
        transport.send_event("suggestion", r#"{
            "type": "immediate_fix",
            "code": "bcrypt.hashpw(password, bcrypt.gensalt(12))",
            "priority": "high",
            "apply_to_line": 45
        }"#).await.unwrap();

        transport.send_event("analysis_complete", r#"{
            "summary": "Analysis complete. Found 1 security issue and 1 performance improvement.",
            "total_suggestions": 2,
            "execution_time_ms": 150
        }"#).await.unwrap();

        // Test keep-alive during long analysis
        transport.send_comment("keep-alive").await.unwrap();
    }
}
