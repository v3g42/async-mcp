use crate::sse::middleware::{AuthConfig, Claims};

use super::{Message, Transport};

use actix_web::web::Bytes;
use anyhow::Result;
use async_trait::async_trait;
use futures::StreamExt;
use jsonwebtoken::{encode, EncodingKey, Header};

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::{broadcast, mpsc, Mutex};
use tracing::debug;

#[derive(Clone)]
pub struct ServerSseTransport {
    // For receiving messages from HTTP POST requests
    message_rx: Arc<Mutex<mpsc::Receiver<Message>>>,
    message_tx: mpsc::Sender<Message>,
    // For sending messages to SSE clients
    sse_tx: broadcast::Sender<Message>,
}

impl ServerSseTransport {
    pub fn new(sse_tx: broadcast::Sender<Message>) -> Self {
        let (message_tx, message_rx) = mpsc::channel(100);
        Self {
            message_rx: Arc::new(Mutex::new(message_rx)),
            message_tx,
            sse_tx,
        }
    }

    pub async fn send_message(&self, message: Message) -> Result<()> {
        self.message_tx.send(message).await?;
        Ok(())
    }

    // Helper function to chunk message into SSE format
    fn format_sse_message(message: &Message) -> Result<String> {
        const CHUNK_SIZE: usize = 16 * 1024; // 16KB chunks
        let json = serde_json::to_string(message)?;
        let mut result = String::new();

        // Add event type
        result.push_str("event: message\n");

        // If small enough, send as single chunk
        if json.len() <= CHUNK_SIZE {
            result.push_str(&format!("data: {}\n\n", json));
            return Ok(result);
        }

        // For larger messages, split at proper boundaries (commas or spaces)
        let mut start = 0;
        while start < json.len() {
            let mut end = (start + CHUNK_SIZE).min(json.len());

            // If we're not at the end, find a good split point
            if end < json.len() {
                // Look back for a comma or space to split at
                while end > start && !json[end..].starts_with([',', ' ']) {
                    end -= 1;
                }
                // If we couldn't find a good split point, just use the max size
                if end == start {
                    end = (start + CHUNK_SIZE).min(json.len());
                }
            }

            result.push_str(&format!("data: {}\n", &json[start..end]));
            start = end;
        }

        result.push('\n');
        Ok(result)
    }
}

#[async_trait]
impl Transport for ServerSseTransport {
    async fn receive(&self) -> Result<Option<Message>> {
        let mut rx = self.message_rx.lock().await;
        match rx.recv().await {
            Some(message) => {
                debug!("Received message from POST request: {:?}", message);
                Ok(Some(message))
            }
            None => Ok(None),
        }
    }

    async fn send(&self, message: &Message) -> Result<()> {
        let formatted = Self::format_sse_message(message)?;
        debug!("Sending chunked SSE message: {}", formatted);
        self.sse_tx.send(message.clone())?;
        Ok(())
    }

    async fn open(&self) -> Result<()> {
        Ok(())
    }

    async fn close(&self) -> Result<()> {
        Ok(())
    }
}

#[derive(Debug)]
pub enum SseEvent {
    Message(Message),
    SessionId(String),
}

/// Client-side SSE transport that sends messages via HTTP POST
/// and receives responses via SSE
#[derive(Clone)]
pub struct ClientSseTransport {
    tx: mpsc::Sender<Message>,
    rx: Arc<Mutex<mpsc::Receiver<Message>>>,
    server_url: String,
    client: reqwest::Client,
    auth_config: Option<AuthConfig>,
    session_id: Arc<Mutex<Option<String>>>,
    headers: HashMap<String, String>,
    buffer: Arc<Mutex<String>>, // Add buffer for partial messages
}

impl ClientSseTransport {
    pub fn builder(url: String) -> ClientSseTransportBuilder {
        ClientSseTransportBuilder::new(url)
    }

    fn generate_token(&self) -> Result<String> {
        let auth_config = self
            .auth_config
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Auth config not set"))?;

        let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs() as usize;
        let claims = Claims {
            iat: now,
            exp: now + 3600, // Token expires in 1 hour
        };

        encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(auth_config.jwt_secret.as_bytes()),
        )
        .map_err(Into::into)
    }

    async fn add_auth_header(
        &self,
        request: reqwest::RequestBuilder,
    ) -> Result<reqwest::RequestBuilder> {
        if self.auth_config.is_some() {
            let token = self.generate_token()?;
            Ok(request.header("Authorization", format!("Bearer {}", token)))
        } else {
            Ok(request)
        }
    }

    fn parse_sse_message(event: &str) -> Option<SseEvent> {
        let mut event_type = None;
        let mut current_data = String::new();

        // Process each line
        for line in event.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            if line.starts_with("event:") {
                event_type = Some(line.trim_start_matches("event:").trim().to_string());
            } else if line.starts_with("data:") {
                // Strip the "data:" prefix and any leading/trailing whitespace
                let data = line["data:".len()..].trim();
                // For chunked messages, we just concatenate the data
                current_data.push_str(data);
            }
        }

        // If we have data, try to parse it
        if !current_data.is_empty() {
            let result = match (event_type.as_ref(), Some(&current_data)) {
                (Some(endpoint), Some(url)) if endpoint == "endpoint" => Some(SseEvent::SessionId(
                    url.split("sessionId=")
                        .nth(1)
                        .unwrap_or_default()
                        .to_string(),
                )),
                (None, Some(data)) | (Some(_), Some(data)) => {
                    match serde_json::from_str::<Message>(data) {
                        Ok(msg) => Some(SseEvent::Message(msg)),
                        Err(e) => {
                            debug!(
                                "Failed to parse SSE message: {}. Content preview: {}",
                                e,
                                if data.len() > 100 {
                                    format!("{}... (truncated)", &data[..100])
                                } else {
                                    data.to_string()
                                }
                            );
                            None
                        }
                    }
                }
                _ => None,
            };

            if result.is_none() {
                debug!(
                    "Unrecognized SSE event format - event_type: {:?}, data length: {}",
                    event_type,
                    current_data.len()
                );
            }

            result
        } else {
            None
        }
    }

    async fn handle_sse_chunk(
        chunk: Bytes,
        tx: &mpsc::Sender<Message>,
        session_id: &Arc<Mutex<Option<String>>>,
        buffer: &Arc<Mutex<String>>,
    ) -> Result<()> {
        let chunk_str = String::from_utf8(chunk.to_vec())?;
        let mut buffer = buffer.lock().await;

        // Append new chunk to buffer
        buffer.push_str(&chunk_str);

        // Process complete messages
        while let Some(pos) = buffer.find("\n\n") {
            let complete_event = buffer[..pos + 2].to_string();
            buffer.replace_range(..pos + 2, "");

            if let Some(sse_event) = Self::parse_sse_message(&complete_event) {
                match sse_event {
                    SseEvent::Message(message) => {
                        debug!("Received SSE message: {:?}", message);
                        tx.send(message).await?;
                    }
                    SseEvent::SessionId(id) => {
                        debug!("Received session ID: {}", id);
                        *session_id.lock().await = Some(id);
                    }
                }
            }
        }

        Ok(())
    }
}

#[derive(Default)]
pub struct ClientSseTransportBuilder {
    server_url: String,
    auth_config: Option<AuthConfig>,
    headers: HashMap<String, String>,
}

impl ClientSseTransportBuilder {
    pub fn new(server_url: String) -> Self {
        Self {
            server_url,
            auth_config: None,
            headers: HashMap::new(),
        }
    }

    pub fn with_auth(mut self, jwt_secret: String) -> Self {
        self.auth_config = Some(AuthConfig { jwt_secret });
        self
    }

    pub fn with_header(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.insert(key.into(), value.into());
        self
    }

    pub fn build(self) -> ClientSseTransport {
        let (tx, rx) = mpsc::channel(100);
        ClientSseTransport {
            tx,
            rx: Arc::new(Mutex::new(rx)),
            server_url: self.server_url,
            client: reqwest::Client::new(),
            auth_config: self.auth_config,
            session_id: Arc::new(Mutex::new(None)),
            headers: self.headers,
            buffer: Arc::new(Mutex::new(String::new())), // Initialize buffer
        }
    }
}

#[async_trait]
impl Transport for ClientSseTransport {
    async fn receive(&self) -> Result<Option<Message>> {
        let mut rx = self.rx.lock().await;
        match rx.recv().await {
            Some(message) => {
                debug!("Received SSE message: {:?}", message);
                Ok(Some(message))
            }
            None => Ok(None),
        }
    }

    async fn send(&self, message: &Message) -> Result<()> {
        let session_id = self
            .session_id
            .lock()
            .await
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("No session ID available"))?
            .clone();

        let request = self
            .client
            .post(format!(
                "{}/message?sessionId={}",
                self.server_url, session_id
            ))
            .json(message);

        let request = self.add_auth_header(request).await?;
        let response = request.send().await?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await?;
            return Err(anyhow::anyhow!(
                "Failed to send message, status: {status}, body: {text}",
            ));
        }

        Ok(())
    }

    async fn open(&self) -> Result<()> {
        let tx = self.tx.clone();
        let server_url = self.server_url.clone();
        let auth_config = self.auth_config.clone();
        let session_id = self.session_id.clone();
        let headers = self.headers.clone();
        let buffer = self.buffer.clone();

        let handle = tokio::spawn(async move {
            let mut request = reqwest::Client::new().get(format!("{}/sse", server_url));

            // Add custom headers
            for (key, value) in &headers {
                request = request.header(key, value);
            }

            // Add auth header if configured
            if let Some(auth_config) = auth_config {
                let claims = Claims {
                    iat: SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs() as usize,
                    exp: SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs() as usize + 3600,
                };

                let token = encode(
                    &Header::default(),
                    &claims,
                    &EncodingKey::from_secret(auth_config.jwt_secret.as_bytes()),
                )?;

                request = request.header("Authorization", format!("Bearer {}", token));
            }

            let mut event_stream = request.send().await?.bytes_stream();

            // Handle first message to get session ID
            if let Some(first_chunk) = event_stream.next().await {
                match first_chunk {
                    Ok(bytes) => Self::handle_sse_chunk(bytes, &tx, &session_id, &buffer).await?,
                    Err(e) => {
                        return Err(anyhow::anyhow!("Failed to get initial SSE message: {}", e))
                    }
                }
            } else {
                return Err(anyhow::anyhow!(
                    "SSE connection closed before receiving initial message"
                ));
            }

            // Handle remaining messages
            while let Some(chunk) = event_stream.next().await {
                if let Ok(bytes) = chunk {
                    if let Err(e) = Self::handle_sse_chunk(bytes, &tx, &session_id, &buffer).await {
                        debug!("Error handling SSE message: {:?}", e);
                    }
                }
            }

            Ok::<_, anyhow::Error>(())
        });

        // Wait for the session ID to be set
        let mut attempts = 0;
        while attempts < 10 {
            if self.session_id.lock().await.is_some() {
                return Ok(());
            }
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            attempts += 1;
        }

        handle.abort();
        Err(anyhow::anyhow!("Timeout waiting for initial SSE message"))
    }

    async fn close(&self) -> Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_large_sse_message() {
        // This is the problematic message format we're seeing
        let large_json = r#"{"id":0,"result":{"tools":[{"description":"A powerful web search tool that provides comprehensive, real-time results using Tavily's AI search engine. Returns relevant web content with customizable parameters for result count, content type, and domain filtering. Ideal for gathering current information, news, and detailed web content analysis.","inputSchema":{"properties":{"days":{"default":3,"description":"The number of days back from the current date to include in the search results. This specifies the time frame of data to be retrieved. Please note that this feature is only available when using the 'news' search topic","type":"number"}}},"name":"tavily-search"}]},"jsonrpc":"2.0"}"#;

        // Format it as an SSE message with multiple data chunks
        let mut sse_message = String::new();
        sse_message.push_str("event: message\n");

        // Split the JSON into smaller chunks (simulating what the server does)
        let chunk_size = 100;
        for chunk in large_json.as_bytes().chunks(chunk_size) {
            if let Ok(chunk_str) = std::str::from_utf8(chunk) {
                sse_message.push_str(&format!("data: {}\n", chunk_str));
            }
        }
        sse_message.push('\n');

        // Try to parse it
        let result = ClientSseTransport::parse_sse_message(&sse_message);
        assert!(result.is_some(), "Failed to parse SSE message");

        if let Some(SseEvent::Message(msg)) = result {
            // Verify the parsed message matches the original
            let parsed_json = serde_json::to_string(&msg).unwrap();
            assert_eq!(parsed_json, large_json);
        } else {
            panic!("Expected Message event");
        }
    }

    #[test]
    fn test_parse_real_sse_message() {
        // The actual message that's failing, but properly formatted
        let sse_message = concat!(
            "data: {\"id\":0,\"result\":{\"tools\":[{\"description\":\"A powerful web search tool that provides comprehensive, real-time results using Tavily's AI search engine. Returns relevant web content with customizable parameters for result count, content type, and domain filtering. Ideal for gathering current information, news, and detailed web content analysis.\",\"inputSchema\":{\"properties\":{\"days\":{\"default\":3,\"description\":\"The number of days back from the current date to include in the search results. This specifies the time frame of data to be retrieved. Please note that this feature is only available when using the 'news' search topic\",\"type\":\"number\"},\"exclude_domains\":{\"default\":[],\"description\":\"List of domains to specifically exclude, if the user asks to exclude a domain set this to the domain of the site\",\"items\":{\"type\":\"string\"},\"type\":\"array\"},\"include_domains\":{\"default\":[],\"description\":\"A list of domains to specifically include in the search results, if the user asks to search on specific sites set this to the domain of the site\",\"items\":{\"type\":\"string\"},\"type\":\"array\"},\"include_image_descriptions\":{\"default\":false,\"description\":\"Include a list of query-related images and their descriptions in the response\",\"type\":\"boolean\"},\"include_images\":{\"default\":false,\"description\":\"Include a list of query-related images in the response\",\"type\":\"boolean\"},\"include_raw_content\":{\"default\":false,\"description\":\"Include the cleaned and parsed HTML content of each search result\",\"type\":\"boolean\"},\"max_results\":{\"default\":10,\"description\":\"The maximum number of search results to return\",\"maximum\":20,\"minimum\":5,\"type\":\"number\"},\"query\":{\"description\":\"Search query\",\"type\":\"string\"},\"search_depth\":{\"default\":\"basic\",\"description\":\"The depth of the search. It can be 'basic' or 'advanced'\",\"enum\":[\"basic\",\"advanced\"],\"type\":\"string\"},\"time_range\":{\"description\":\"The time range back from the current date to include in the search results. This feature is available for both 'general' and 'news' search topics\",\"enum\":[\"day\",\"week\",\"month\",\"year\",\"d\",\"w\",\"m\",\"y\"],\"type\":\"string\"},\"topic\":{\"default\":\"general\",\"description\":\"The category of the search. This will determine which of our agents will be used for the search\",\"enum\":[\"general\",\"news\"],\"type\":\"string\"}},\"required\":[\"query\"],\"type\":\"object\"},\"name\":\"tavily-search\"},{\"description\":\"A powerful web content extraction tool that retrieves and processes raw content from specified URLs, ideal for data collection, content analysis, and research tasks.\",\"inputSchema\":{\"properties\":{\"extract_depth\":{\"default\":\"basic\",\"description\":\"Depth of extraction - 'basic' or 'advanced', if usrls are linkedin use 'advanced' or if explicitly told to use advanced\",\"enum\":[\"basic\",\"advanced\"],\"type\":\"string\"},\"include_images\":{\"default\":false,\"description\":\"Include a list of images extracted from the urls in the response\",\"type\":\"boolean\"},\"urls\":{\"description\":\"List of URLs to extract content from\",\"items\":{\"type\":\"string\"},\"type\":\"array\"}},\"required\":[\"urls\"],\"type\":\"object\"},\"name\":\"tavily-extract\"},{\"description\":\"Read the complete contents of a file from the file system. Handles various text encodings and provides detailed error messages if the file cannot be read. Use this tool when you need to examine the contents of a single file. Only works within allowed directories.\",\"inputSchema\":{\"$schema\":\"http://json-schema.org/draft-07/schema#\",\"additionalProperties\":false,\"properties\":{\"path\":{\"type\":\"string\"}},\"required\":[\"path\"],\"type\":\"object\"},\"name\":\"read_file\"},{\"description\":\"Read the contents of multiple files simultaneously. This is more efficient than reading files one by one when you need to analyze or compare multiple files. Each file's content is returned with its path as a reference. Failed reads for individual files won't stop the entire operation. Only works within allowed directories.\",\"inputSchema\":{\"$schema\":\"http://json-schema.org/draft-07/schema#\",\"additionalProperties\":false,\"properties\":{\"paths\":{\"items\":{\"type\":\"string\"},\"type\":\"array\"}},\"required\":[\"paths\"],\"type\":\"object\"},\"name\":\"read_multiple_files\"},{\"description\":\"Create a new file or completely overwrite an existing file with new content. Use with caution as it will overwrite existing files without warning. Handles text content with proper encoding. Only works within allowed directories.\",\"inputSchema\":{\"$schema\":\"http://json-schema.org/draft-07/schema#\",\"additionalProperties\":false,\"properties\":{\"content\":{\"type\":\"string\"},\"path\":{\"type\":\"string\"}},\"required\":[\"path\",\"content\"],\"type\":\"object\"},\"name\":\"write_file\"},{\"description\":\"Make line-based edits to a text file. Each edit replaces exact line sequences with new content. Returns a git-style diff showing the changes made. Only works within allowed directories.\",\"inputSchema\":{\"$schema\":\"http://json-schema.org/draft-07/schema#\",\"additionalProperties\":false,\"properties\":{\"dryRun\":{\"default\":false,\"description\":\"Preview changes using git-style diff format\",\"type\":\"boolean\"},\"edits\":{\"items\":{\"additionalProperties\":false,\"properties\":{\"newText\":{\"description\":\"Text to replace with\",\"type\":\"string\"},\"oldText\":{\"description\":\"Text to search for - must match exactly\",\"type\":\"string\"}},\"required\":[\"oldText\",\"newText\"],\"type\":\"object\"},\"type\":\"array\"},\"path\":{\"type\":\"string\"}},\"required\":[\"path\",\"edits\"],\"type\":\"object\"},\"name\":\"edit_file\"},{\"description\":\"Create a new directory or ensure a directory exists. Can create multiple nested directories in one operation. If the directory already exists, this operation will succeed silently. Perfect for setting up directory structures for projects or ensuring required paths exist. Only works within allowed directories.\",\"inputSchema\":{\"$schema\":\"http://json-schema.org/draft-07/schema#\",\"additionalProperties\":false,\"properties\":{\"path\":{\"type\":\"string\"}},\"required\":[\"path\"],\"type\":\"object\"},\"name\":\"create_directory\"},{\"description\":\"Get a detailed listing of all files and directories in a specified path. Results clearly distinguish between files and directories with [FILE] and [DIR] prefixes. This tool is essential for understanding directory structure and finding specific files within a directory. Only works within allowed directories.\",\"inputSchema\":{\"$schema\":\"http://json-schema.org/draft-07/schema#\",\"additionalProperties\":false,\"properties\":{\"path\":{\"type\":\"string\"}},\"required\":[\"path\"],\"type\":\"object\"},\"name\":\"list_directory\"},{\"description\":\"Get a recursive tree view of files and directories as a JSON structure. Each entry includes 'name', 'type' (file/directory), and 'children' for directories. Files have no children array, while directories always have a children array (which may be empty). The output is formatted with 2-space indentation for readability. Only works within allowed directories.\",\"inputSchema\":{\"$schema\":\"http://json-schema.org/draft-07/schema#\",\"additionalProperties\":false,\"properties\":{\"path\":{\"type\":\"string\"}},\"required\":[\"path\"],\"type\":\"object\"},\"name\":\"directory_tree\"},{\"description\":\"Move or rename files and directories. Can move files between directories and rename them in a single operation. If the destination exists, the operation will fail. Works across different directories and can be used for simple renaming within the same directory. Both source and destination must be within allowed directories.\",\"inputSchema\":{\"$schema\":\"http://json-schema.org/draft-07/schema#\",\"additionalProperties\":false,\"properties\":{\"destination\":{\"type\":\"string\"},\"source\":{\"type\":\"string\"}},\"required\":[\"source\",\"destination\"],\"type\":\"object\"},\"name\":\"move_file\"},{\"description\":\"Recursively search for files and directories matching a pattern. Searches through all subdirectories from the starting path. The search is case-insensitive and matches partial names. Returns full paths to all matching items. Great for finding files when you don't know their exact location. Only searches within allowed directories.\",\"inputSchema\":{\"$schema\":\"http://json-schema.org/draft-07/schema#\",\"additionalProperties\":false,\"properties\":{\"excludePatterns\":{\"default\":[],\"items\":{\"type\":\"string\"},\"type\":\"array\"},\"path\":{\"type\":\"string\"},\"pattern\":{\"type\":\"string\"}},\"requ",
            "data: ired\":[\"path\",\"pattern\"],\"type\":\"object\"},\"name\":\"search_files\"},{\"description\":\"Retrieve detailed metadata about a file or directory. Returns comprehensive information including size, creation time, last modified time, permissions, and type. This tool is perfect for understanding file characteristics without reading the actual content. Only works within allowed directories.\",\"inputSchema\":{\"$schema\":\"http: //json-schema.org/draft-07/schema#\",\"additionalProperties\":false,\"properties\":{\"path\":{\"type\":\"string\"}},\"required\":[\"path\"],\"type\":\"object\"},\"name\":\"get_file_info\"},{\"description\":\"Returns the list of directories that this server is allowed to access. Use this to understand which directories are available before trying to access files.\",\"inputSchema\":{\"properties\":{},\"required\":[],\"type\":\"object\"},\"name\":\"list_allowed_directories\"}]},\"jsonrpc\":\"2.0\"}"
        );

        let result = ClientSseTransport::parse_sse_message(sse_message);
        assert!(result.is_some(), "Failed to parse real SSE message");

        // Verify we can parse the message into valid JSON
        if let Some(SseEvent::Message(msg)) = result {
            let json = serde_json::to_string(&msg).unwrap();
            assert!(json.contains("\"description\":\"A powerful web search tool"));
        } else {
            panic!("Expected Message event");
        }
    }
}
