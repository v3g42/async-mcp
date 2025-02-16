use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use crate::completable::Completable;
use crate::types::{GetPromptResult, Prompt, PromptArgument};

/// A registered prompt with metadata and callbacks
pub(crate) struct RegisteredPrompt {
    /// The prompt metadata
    pub metadata: Prompt,
    /// Optional argument completions
    pub argument_completions: HashMap<String, Arc<dyn Completable<Input = str, Output = String>>>,
    /// The callback to execute the prompt
    pub execute_callback: Arc<dyn PromptCallback>,
}

/// A callback that can execute a prompt
pub trait PromptCallback: Send + Sync {
    fn call(
        &self,
        args: Option<HashMap<String, String>>,
    ) -> Pin<Box<dyn Future<Output = GetPromptResult> + Send>>;
}

struct PromptCallbackFn(
    Box<
        dyn Fn(Option<HashMap<String, String>>) -> Pin<Box<dyn Future<Output = GetPromptResult> + Send>>
            + Send
            + Sync,
    >,
);

impl PromptCallback for PromptCallbackFn {
    fn call(
        &self,
        args: Option<HashMap<String, String>>,
    ) -> Pin<Box<dyn Future<Output = GetPromptResult> + Send>> {
        (self.0)(args)
    }
}

/// Builder for creating prompts with arguments and completions
pub struct PromptBuilder {
    name: String,
    description: Option<String>,
    arguments: Vec<PromptArgument>,
    argument_completions: HashMap<String, Arc<dyn Completable<Input = str, Output = String>>>,
}

impl PromptBuilder {
    /// Create a new prompt builder with the given name
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: None,
            arguments: Vec::new(),
            argument_completions: HashMap::new(),
        }
    }

    /// Add a description to the prompt
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Add a required argument to the prompt
    pub fn required_arg(
        mut self,
        name: impl Into<String>,
        description: Option<impl Into<String>>,
    ) -> Self {
        self.arguments.push(PromptArgument {
            name: name.into(),
            description: description.map(|d| d.into()),
            required: Some(true),
        });
        self
    }

    /// Add an optional argument to the prompt
    pub fn optional_arg(
        mut self,
        name: impl Into<String>,
        description: Option<impl Into<String>>,
    ) -> Self {
        self.arguments.push(PromptArgument {
            name: name.into(),
            description: description.map(|d| d.into()),
            required: Some(false),
        });
        self
    }

    /// Add a completion callback for an argument
    pub fn with_completion(
        mut self,
        arg_name: impl Into<String>,
        completable: impl Completable<Input = str, Output = String> + 'static,
    ) -> Self {
        self.argument_completions
            .insert(arg_name.into(), Arc::new(completable));
        self
    }

    /// Build the prompt with the given execution callback
    pub fn build<F, Fut>(
        self,
        callback: F,
    ) -> (Prompt, RegisteredPrompt)
    where
        F: Fn(Option<HashMap<String, String>>) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = GetPromptResult> + Send + 'static,
    {
        let metadata = Prompt {
            name: self.name.clone(),
            description: self.description.clone(),
            arguments: if self.arguments.is_empty() {
                None
            } else {
                Some(self.arguments.clone())
            },
        };

        let registered = RegisteredPrompt {
            metadata: metadata.clone(),
            argument_completions: self.argument_completions,
            execute_callback: Arc::new(PromptCallbackFn(Box::new(move |args| {
                Box::pin(callback(args))
            }))),
        };

        (metadata, registered)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::completable::CompletableString;
    use crate::types::{PromptMessage, TextContent};

    #[tokio::test]
    async fn test_prompt_builder() {
        let (metadata, registered) = PromptBuilder::new("test")
            .description("A test prompt")
            .required_arg("arg1", Some("First argument"))
            .optional_arg("arg2", None)
            .with_completion(
                "arg1",
                CompletableString::new(|input| async move { vec![format!("{}_completed", input)] }),
            )
            .build(|_args| async move {
                GetPromptResult {
                    description: None,
                    messages: vec![PromptMessage {
                        role: "assistant".to_string(),
                        content: TextContent {
                            r#type: "text".to_string(),
                            text: "Test response".to_string(),
                        },
                    }],
                }
            });

        assert_eq!(metadata.name, "test");
        assert_eq!(metadata.description, Some("A test prompt".to_string()));
        assert_eq!(metadata.arguments.as_ref().unwrap().len(), 2);

        assert!(registered.argument_completions.contains_key("arg1"));
        assert!(!registered.argument_completions.contains_key("arg2"));

        let result = registered
            .execute_callback
            .call(Some(HashMap::new()))
            .await;
        assert_eq!(result.messages[0].content.text, "Test response");
    }
}
