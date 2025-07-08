use axum::{
    extract::{ws::{Message, WebSocket}, State, WebSocketUpgrade},
    http::HeaderValue,
    response::IntoResponse,
    routing::get,
    Router,
};
use futures_util::{SinkExt, StreamExt};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tokio::sync::broadcast::{self, Sender};
use tower_http::cors::{Any, CorsLayer};

#[derive(Clone)]
struct AppState {
    tx: Sender<String>,
    user_count: Arc<AtomicUsize>,
}

#[tokio::main]
async fn main() {
    let (tx, _) = broadcast::channel(100);
    let app = app(tx);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

fn app(tx: Sender<String>) -> Router {
    let cors_layer = CorsLayer::new()
        .allow_methods(Any)
        .allow_origin("http://127.0.0.1:8080".parse::<HeaderValue>().unwrap());

    let state = AppState {
        tx,
        user_count: Arc::new(AtomicUsize::new(0)),
    };

    Router::new()
        .route("/", get(|| async { "Home" }))
        .route("/chat", get(chat_handler))
        .with_state(state)
        .layer(cors_layer)
}

async fn chat_handler(
    State(state): State<AppState>,
    ws: WebSocketUpgrade,
) -> impl IntoResponse {
    ws.on_upgrade(|websocket| handle_websocket(state, websocket))
}

async fn handle_websocket(state: AppState, websocket: WebSocket) {
    let (mut sender, mut receiver) = websocket.split();

    // Update user count
    state.user_count.fetch_add(1, Ordering::SeqCst);
    let count = state.user_count.load(Ordering::SeqCst);
    let _ = state.tx.send(format!("[系统] 当前在线人数: {}", count));

    let mut rx = state.tx.subscribe();
    tokio::spawn(async move {
        while let Ok(msg) = rx.recv().await {
            let _ = sender.send(Message::Text(msg.into())).await;
        }
    });

    while let Some(Ok(msg)) = receiver.next().await {
        if let Message::Text(content) = msg {
            let _ = state.tx.send(content.to_string());
        }
    }

    // Decrease user count when connection closes
    state.user_count.fetch_sub(1, Ordering::SeqCst);
    let count = state.user_count.load(Ordering::SeqCst);
    let _ = state.tx.send(format!("[系统] 当前在线人数: {}", count));
}