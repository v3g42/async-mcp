use actix_web::middleware::Logger;
use actix_web::web::Payload;
use actix_web::web::Query;
use actix_web::{web, App, HttpResponse, HttpServer};
use anyhow::Result;
use futures::StreamExt;
use uuid::Uuid;

use crate::server::Server;
use crate::sse::middleware::{AuthConfig, JwtAuth};
use crate::transport::ServerHttpTransport;
use crate::transport::{handle_ws_connection, Message, ServerSseTransport, ServerWsTransport};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::sync::broadcast;
use tracing::{debug, error, info};

/// Server-side SSE transport that handles HTTP POST requests for incoming messages
/// and sends responses via SSE
#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub exp: usize,
    pub iat: usize,
}

#[derive(Deserialize)]
pub struct MessageQuery {
    #[serde(rename = "sessionId")]
    session_id: Option<String>,
}

#[derive(Clone)]
pub struct SessionState {
    sessions: Arc<Mutex<HashMap<String, ServerHttpTransport>>>,
    port: u16,
    build_server: Arc<
        dyn Fn(
                ServerHttpTransport,
            )
                -> futures::future::BoxFuture<'static, Result<Server<ServerHttpTransport>>>
            + Send
            + Sync,
    >,
}

/// Run a server instance with the specified transport
pub async fn run_http_server<F, Fut>(
    port: u16,
    jwt_secret: Option<String>,
    build_server: F,
) -> Result<()>
where
    F: Fn(ServerHttpTransport) -> Fut + Send + Sync + 'static,
    Fut: futures::Future<Output = Result<Server<ServerHttpTransport>>> + Send + 'static,
{
    info!("Starting server on http://127.0.0.1:{}", port);
    info!("WebSocket endpoint: ws://127.0.0.1:{}/ws", port);
    info!("SSE endpoint: http://127.0.0.1:{}/sse", port);

    let sessions = Arc::new(Mutex::new(HashMap::new()));

    // Box the future when creating the Arc
    let build_server =
        Arc::new(move |t| Box::pin(build_server(t)) as futures::future::BoxFuture<_>);

    let auth_config = jwt_secret.map(|jwt_secret| AuthConfig { jwt_secret });
    let http_server = http_server(port, sessions, auth_config, build_server);

    http_server.await?;
    Ok(())
}

pub async fn http_server(
    port: u16,
    sessions: Arc<Mutex<HashMap<String, ServerHttpTransport>>>,
    auth_config: Option<AuthConfig>,
    build_server: Arc<
        dyn Fn(
                ServerHttpTransport,
            )
                -> futures::future::BoxFuture<'static, Result<Server<ServerHttpTransport>>>
            + Send
            + Sync,
    >,
) -> std::result::Result<(), std::io::Error> {
    let session_state = SessionState {
        sessions,
        build_server,
        port,
    };

    let server = HttpServer::new(move || {
        let session_state = session_state.clone();
        App::new()
            .wrap(Logger::default())
            .wrap(JwtAuth::new(auth_config.clone()))
            .app_data(web::Data::new(session_state))
            .route("/sse", web::get().to(sse_handler))
            .route("/message", web::post().to(message_handler))
            .route("/ws", web::get().to(ws_handler))
    })
    .bind(("127.0.0.1", port))?
    .run();

    server.await
}

pub async fn sse_handler(
    req: actix_web::HttpRequest,
    session_state: web::Data<SessionState>,
) -> HttpResponse {
    let client_ip = req
        .peer_addr()
        .map(|addr| addr.ip().to_string())
        .unwrap_or_else(|| "unknown".to_string());

    info!("New SSE connection request from {}", client_ip);

    // Create new session
    let session_id = Uuid::new_v4().to_string();

    // Create channel for SSE messages
    let (sse_tx, sse_rx) = broadcast::channel(100);

    // Create new transport for this session
    let transport = ServerHttpTransport::Sse(ServerSseTransport::new(sse_tx.clone()));

    // Store transport in sessions map
    session_state
        .sessions
        .lock()
        .unwrap()
        .insert(session_id.clone(), transport.clone());

    info!(
        "SSE connection established for {} with session_id {}",
        client_ip, session_id
    );
    let port = session_state.port;
    // Create initial endpoint info event
    let endpoint_info = format!(
        "event: endpoint\ndata: http://127.0.0.1:{port}/message?sessionId={session_id}\n\n",
    );

    let stream = futures::stream::once(async move {
        Ok::<_, std::convert::Infallible>(web::Bytes::from(endpoint_info))
    })
    .chain(futures::stream::unfold(sse_rx, move |mut rx| {
        let client_ip = client_ip.clone();
        async move {
            match rx.recv().await {
                Ok(msg) => {
                    debug!("Sending SSE message to {}: {:?}", client_ip, msg);
                    let json = serde_json::to_string(&msg).unwrap();
                    let sse_data = format!("data: {}\n\n", json);
                    Some((
                        Ok::<_, std::convert::Infallible>(web::Bytes::from(sse_data)),
                        rx,
                    ))
                }
                _ => None,
            }
        }
    }));

    // Create and start server instance for this session
    let transport_clone = transport.clone();
    let build_server = session_state.build_server.clone();
    tokio::spawn(async move {
        match build_server(transport_clone).await {
            Ok(server) => {
                if let Err(e) = server.listen().await {
                    error!("Server error: {:?}", e);
                }
            }
            Err(e) => {
                error!("Failed to build server: {:?}", e);
            }
        }
    });

    HttpResponse::Ok()
        .append_header(("X-Session-Id", session_id))
        .content_type("text/event-stream")
        .streaming(stream)
}

async fn message_handler(
    query: Query<MessageQuery>,
    message: web::Json<Message>,
    session_state: web::Data<SessionState>,
) -> HttpResponse {
    if let Some(session_id) = &query.session_id {
        let sessions = session_state.sessions.lock().unwrap();
        if let Some(transport) = sessions.get(session_id) {
            match transport {
                ServerHttpTransport::Sse(sse) => match sse.send_message(message.into_inner()).await
                {
                    Ok(_) => {
                        debug!("Successfully sent message to session {}", session_id);
                        HttpResponse::Accepted().finish()
                    }
                    Err(e) => {
                        error!("Failed to send message to session {}: {:?}", session_id, e);
                        HttpResponse::InternalServerError().finish()
                    }
                },
                ServerHttpTransport::Ws(_) => HttpResponse::BadRequest()
                    .body("Cannot send message to WebSocket connection through HTTP endpoint"),
            }
        } else {
            HttpResponse::NotFound().body(format!("Session {} not found", session_id))
        }
    } else {
        HttpResponse::BadRequest().body("Session ID not specified")
    }
}

async fn ws_handler(
    req: actix_web::HttpRequest,
    body: Payload,
    session_state: web::Data<SessionState>,
) -> Result<HttpResponse, actix_web::Error> {
    let (response, session, msg_stream) = actix_ws::handle(&req, body)?;

    let client_ip = req
        .peer_addr()
        .map(|addr| addr.ip().to_string())
        .unwrap_or_else(|| "unknown".to_string());

    info!("New WebSocket connection from {}", client_ip);

    // Create channels for message passing
    let (tx, rx) = broadcast::channel(100);
    let transport =
        ServerHttpTransport::Ws(ServerWsTransport::new(session.clone(), rx.resubscribe()));

    // Store transport in sessions map
    let session_id = Uuid::new_v4().to_string();
    session_state
        .sessions
        .lock()
        .unwrap()
        .insert(session_id, transport.clone());

    // Start WebSocket handling in the background
    actix_web::rt::spawn(async move {
        let _ = handle_ws_connection(session, msg_stream, tx.clone(), rx.resubscribe()).await;
    });

    // Spawn server instance
    let build_server = session_state.build_server.clone();
    actix_web::rt::spawn(async move {
        if let Ok(server) = build_server(transport).await {
            let _ = server.listen().await;
        }
    });

    Ok(response)
}
