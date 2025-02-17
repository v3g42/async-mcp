use async_mcp::completable::{Completable, CompletableString, FixedCompletions};
use async_mcp::server::notifications::{Notification, CancelledParams, NotificationSender};
use async_mcp::server::prompt::PromptBuilder;
use async_mcp::server::Server;
use async_mcp::types::{Implementation, ServerCapabilities};
use async_mcp::transport::Transport;
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use futures::executor::block_on;
use serde_json;
use std::sync::Arc;
use tokio::time::sleep;
use std::time::Duration;
use async_trait::async_trait;
use tokio::runtime::Runtime;

struct MockTransport;

#[async_trait::async_trait]
impl Transport for MockTransport {
    async fn receive(&self) -> async_mcp::transport::Result<Option<async_mcp::transport::Message>> {
        Ok(None)
    }

    async fn send(&self, _message: &async_mcp::transport::Message) -> async_mcp::transport::Result<()> {
        Ok(())
    }

    async fn open(&self) -> async_mcp::transport::Result<()> {
        Ok(())
    }

    async fn close(&self) -> async_mcp::transport::Result<()> {
        Ok(())
    }
}

fn bench_completable_string(c: &mut Criterion) {
    let completable = CompletableString::new(|input: &str| {
        let input = input.to_string();
        async move {
            vec![format!("{}1", input), format!("{}2", input)]
        }
    });

    c.bench_function("completable_string", |b| {
        b.iter(|| {
            let _ = block_on(completable.complete(black_box("test")));
        })
    });
}

fn bench_fixed_completions(c: &mut Criterion) {
    let completions = FixedCompletions::new(vec!["apple", "banana", "cherry"]);

    c.bench_function("fixed_completions", |b| {
        b.iter(|| {
            let _ = block_on(completions.complete(black_box("a")));
        })
    });
}

fn bench_notification_serialization(c: &mut Criterion) {
    let notification = Notification::Cancelled(CancelledParams {
        request_id: "123".to_string(),
        reason: Some("User cancelled".to_string()),
    });

    c.bench_function("notification_serialization", |b| {
        b.iter(|| {
            let _ = serde_json::to_string(black_box(&notification));
        })
    });
}

fn bench_prompt_builder(c: &mut Criterion) {
    c.bench_function("prompt_builder", |b| {
        b.iter(|| {
            let _ = PromptBuilder::new("test")
                .description("A test prompt")
                .required_arg("arg1", Some("First argument"))
                .optional_arg("arg2", None)
                .build(|_args| async { 
                    async_mcp::server::prompt::GetPromptResult::default()
                });
        })
    });
}

fn bench_connection_setup(c: &mut Criterion) {
    c.bench_function("connection_setup", |b| {
        b.iter(|| {
            let server = Server::new(Implementation {
                name: "test".to_string(),
                version: "0.1.0".to_string(),
            });
            block_on(server.connect(MockTransport))
        })
    });
}

struct MockNotificationSender;

impl MockNotificationSender {
    async fn send_notification(&self, _notification: Notification) -> Result<(), async_mcp::server::error::ServerError> {
        // Simulate notification sending
        sleep(Duration::from_micros(100)).await;
        Ok(())
    }
}

struct AsyncNotificationSender(Arc<MockNotificationSender>);

#[async_trait]
impl NotificationSender for AsyncNotificationSender {
    async fn send(&self, notification: Notification) -> Result<(), async_mcp::server::error::ServerError> {
        self.0.send_notification(notification).await
    }
}

fn bench_notification_sending(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    
    let mut server = Server::new(Implementation {
        name: "test".to_string(),
        version: "0.1.0".to_string(),
    });
    
    let sender = AsyncNotificationSender(Arc::new(MockNotificationSender));
    server.set_notification_sender(sender);

    let notification = Notification::Initialized;

    c.bench_function("notification_sending", |b| {
        b.iter(|| {
            rt.block_on(server.send_notification(black_box(notification.clone())))
        })
    });
}

fn bench_server_capabilities_registration(c: &mut Criterion) {
    let mut server = Server::new(Implementation {
        name: "test".to_string(),
        version: "0.1.0".to_string(),
    });

    let capabilities = ServerCapabilities::default();

    c.bench_function("server_capabilities_registration", |b| {
        b.iter(|| {
            server.register_capabilities(black_box(capabilities.clone()));
        })
    });
}

criterion_group!(benches, 
    bench_completable_string,
    bench_fixed_completions,
    bench_notification_serialization,
    bench_prompt_builder,
    bench_connection_setup,
    bench_notification_sending,
    bench_server_capabilities_registration
);
criterion_main!(benches);
