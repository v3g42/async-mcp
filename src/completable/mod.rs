use std::future::{Future, IntoFuture};
use std::pin::Pin;
use std::sync::Arc;
use std::fmt::Debug;

// Type aliases for complex future types
type CompletionFuture<T> = Pin<Box<dyn Future<Output = Vec<T>> + Send>>;
type CompletionFn<T> = Arc<dyn Fn(&str) -> CompletionFuture<T> + Send + Sync>;

/// A trait for types that can provide completion suggestions.
/// Similar to the TypeScript SDK's Completable type.
pub trait Completable {
    /// The input type for completion suggestions
    type Input: ?Sized + Debug;
    /// The output type for completion suggestions
    type Output;

    /// Generate completion suggestions for the given input value
    fn complete(&self, value: &Self::Input) -> CompletionFuture<Self::Output>;
}

/// A completable string that uses a callback function to generate suggestions
pub struct CompletableString {
    complete_fn: CompletionFn<String>,
}

impl CompletableString {
    /// Create a new CompletableString with the given completion callback
    pub fn new<F, Fut>(complete_fn: F) -> Self 
    where
        F: Fn(&str) -> Fut + Send + Sync + 'static,
        Fut: IntoFuture<Output = Vec<String>> + Send + 'static,
        Fut::IntoFuture: Send,
    {
        Self {
            complete_fn: Arc::new(move |input| {
                let input = input.to_string();
                Box::pin(complete_fn(&input).into_future())
            }),
        }
    }
}

impl Completable for CompletableString {
    type Input = str;
    type Output = String;

    fn complete(&self, value: &Self::Input) -> CompletionFuture<Self::Output> {
        (self.complete_fn)(value)
    }
}

/// A completable type that provides fixed suggestions
pub struct FixedCompletions<T> {
    values: Vec<T>,
}

impl<T: Clone + Send + Debug + 'static> FixedCompletions<T> {
    /// Create a new FixedCompletions with the given values
    pub fn new(values: Vec<T>) -> Self {
        Self { values }
    }
}

impl<T: Clone + Send + Debug + 'static> Completable for FixedCompletions<T> {
    type Input = str;
    type Output = T;
    
    fn complete(&self, value: &Self::Input) -> CompletionFuture<Self::Output> {
        let values = self.values.clone();
        let value = value.to_string();
        
        Box::pin(async move {
            values
                .into_iter()
                .filter(|v| format!("{:?}", v).to_lowercase().contains(&value.to_lowercase()))
                .collect()
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_completable_string() {
        let completable = CompletableString::new(|input: &str| {
            let input = input.to_string();
            async move {
                vec![
                    format!("{}1", input),
                    format!("{}2", input)
                ]
            }
        });

        let suggestions = completable.complete("test").await;
        assert_eq!(suggestions, vec!["test1", "test2"]);
    }

    #[tokio::test]
    async fn test_fixed_completions() {
        let completions = FixedCompletions::new(vec!["apple", "banana", "cherry"]);
        let suggestions = completions.complete("a").await;
        assert_eq!(suggestions, vec!["apple", "banana"]);
    }
}
