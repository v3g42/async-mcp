use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use url::Url;

use crate::server::{
    completion::{CompletionCallback, RegisteredCompletion},
    error::ServerError,
    prompt::{PromptBuilder, RegisteredPrompt},
    resource::{RegisteredResource, RegisteredResourceTemplate, ResourceTemplate, ReadResourceResult, ReadResourceCallbackFn},
    roots::{RegisteredRoots, Root},
    sampling::{RegisteredSampling, SamplingRequest, SamplingResult},
    tool::{RegisteredTool, ToolBuilder},
    Server,
};
use crate::transport::Transport;
use crate::types::{Implementation, Prompt, Resource, ServerCapabilities, Tool};

type Result<T> = std::result::Result<T, ServerError>;

/// High-level MCP server that provides a simpler API for working with resources, tools, and prompts.
pub struct McpServer {
    /// The underlying Server instance
    pub server: Server,

    registered_resources: HashMap<String, RegisteredResource>,
    registered_resource_templates: HashMap<String, RegisteredResourceTemplate>,
    registered_tools: HashMap<String, RegisteredTool>,
    registered_prompts: HashMap<String, RegisteredPrompt>,
    registered_sampling: Option<RegisteredSampling>,
    registered_roots: Option<RegisteredRoots>,
    registered_completion: Option<RegisteredCompletion>,
}

impl McpServer {
    /// Create a new MCP server with the given implementation info
    pub fn new(server_info: Implementation) -> Self {
        Self {
            server: Server::new(server_info),
            registered_resources: HashMap::new(),
            registered_resource_templates: HashMap::new(),
            registered_tools: HashMap::new(),
            registered_prompts: HashMap::new(),
            registered_sampling: None,
            registered_roots: None,
            registered_completion: None,
        }
    }

    /// Register a completion handler
    pub fn register_completion(&mut self, handler: impl CompletionCallback + 'static) {
        self.registered_completion = Some(RegisteredCompletion {
            callback: Box::new(handler),
        });

        // Register completion capability
        self.server.register_capabilities(ServerCapabilities {
            completion: Some(Default::default()),
            ..Default::default()
        });
    }

    /// Register a sampling handler
    pub fn register_sampling(
        &mut self,
        callback: impl Fn(SamplingRequest) -> Pin<Box<dyn Future<Output = Result<SamplingResult>> + Send + 'static>>
            + Send
            + Sync
            + 'static,
    ) {
        self.registered_sampling = Some(RegisteredSampling {
            callback: Arc::new(callback),
        });

        // Register sampling capability
        self.server.register_capabilities(ServerCapabilities {
            sampling: Some(Default::default()),
            ..Default::default()
        });
    }

    /// Register a roots handler
    pub fn register_roots(
        &mut self,
        list_callback: impl Fn() -> Pin<Box<dyn Future<Output = Result<Vec<Root>>> + Send>>
            + Send
            + Sync
            + 'static,
        supports_change_notifications: bool,
    ) {
        let wrapped_callback = move || -> Pin<Box<dyn Future<Output = anyhow::Result<Vec<Root>>> + Send>> {
            let fut = list_callback();
            Box::pin(async move { fut.await.map_err(|e| anyhow::anyhow!("{}", e)) })
        };

        self.registered_roots = Some(RegisteredRoots::new(
            wrapped_callback,
            supports_change_notifications,
        ));

        // Register roots capability
        self.server.register_capabilities(ServerCapabilities {
            roots: Some(Default::default()),
            ..Default::default()
        });
    }

    /// Connect to the given transport and start listening for messages
    pub async fn connect(&self, _transport: impl Transport) -> Result<()> {
        self.server.connect(_transport).await
    }

    /// Register a resource at a fixed URI
    pub fn resource(
        &mut self,
        name: impl Into<String>,
        uri: impl Into<String>,
        metadata: Option<Resource>,
        read_callback: impl Fn(&Url) -> Pin<Box<dyn Future<Output = ReadResourceResult> + Send + 'static>>
            + Send 
            + Sync
            + 'static,
    ) {
        let uri = uri.into();
        let name = name.into();

        let metadata = metadata.unwrap_or_else(|| Resource {
            uri: Url::parse(&uri).unwrap_or_else(|e| {
                eprintln!("Warning: Invalid URI '{}': {}", uri, e);
                Url::parse("about:invalid").unwrap()
            }),
            name: name.clone(),
            description: None,
            mime_type: None,
        });

        self.registered_resources.insert(
            uri.clone(),
            RegisteredResource::new(
                metadata,
                read_callback,
                false,
            ),
        );

        // Register capabilities if this is the first resource
        if self.registered_resources.len() == 1 {
            self.server.register_capabilities(ServerCapabilities {
                resources: Some(Default::default()),
                ..Default::default()
            });
        }
    }

    /// Register a resource template
    pub fn resource_template(
        &mut self,
        name: impl Into<String>,
        template: ResourceTemplate,
        metadata: Option<Resource>,
        read_callback: impl Fn(&Url) -> Pin<Box<dyn Future<Output = ReadResourceResult> + Send + 'static>>
            + Send
            + Sync
            + 'static,
    ) {
        let name = name.into();

        let metadata = metadata.unwrap_or_else(|| Resource {
            uri: Url::parse(&template.uri_template()).unwrap_or_else(|e| {
                eprintln!("Warning: Invalid URI template: {}", e);
                Url::parse("about:invalid").unwrap()
            }),
            name: name.clone(),
            description: None,
            mime_type: None,
        });

        self.registered_resource_templates.insert(
            name,
            RegisteredResourceTemplate {
                template,
                metadata,
                read_callback: Arc::new(ReadResourceCallbackFn(Box::new(read_callback))),
            },
        );

        // Register capabilities if this is the first resource template
        if self.registered_resource_templates.len() == 1 {
            self.server.register_capabilities(ServerCapabilities {
                resources: Some(Default::default()),
                ..Default::default()
            });
        }
    }

    /// Create a new prompt builder
    pub fn prompt_builder(&self, name: impl Into<String>) -> PromptBuilder {
        PromptBuilder::new(name)
    }

    /// Register a prompt
    pub fn register_prompt(&mut self, metadata: impl Into<Prompt>, registered: RegisteredPrompt) {
        let metadata = metadata.into();
        self.registered_prompts
            .insert(metadata.name.clone(), registered);

        // Register capabilities if this is the first prompt
        if self.registered_prompts.len() == 1 {
            self.server.register_capabilities(ServerCapabilities {
                prompts: Some(Default::default()),
                ..Default::default()
            });
        }
    }

    /// Create a new tool builder
    pub fn tool_builder(&self, name: impl Into<String>) -> ToolBuilder {
        ToolBuilder::new(name)
    }

    /// Register a tool
    pub fn register_tool(&mut self, metadata: impl Into<Tool>, registered: RegisteredTool) {
        let metadata = metadata.into();
        self.registered_tools.insert(metadata.name.clone(), registered);

        // Register capabilities if this is the first tool
        if self.registered_tools.len() == 1 {
            self.server.register_capabilities(ServerCapabilities {
                tools: Some(Default::default()),
                ..Default::default()
            });
        }
    }
}
