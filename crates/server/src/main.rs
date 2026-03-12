use std::sync::{Arc, Mutex};

use axum::{
    Router,
    extract::State,
    http::StatusCode,
    response::Json,
    routing::{get, post},
};
use tower_http::cors::CorsLayer;
use tracing::info;
use tracing_subscriber::EnvFilter;

use nostrbox_contextvm::types::{OperationRequest, OperationResponse};
use nostrbox_contextvm::OperationHandler;
use nostrbox_store::Store;

#[derive(Clone)]
struct AppState {
    store: Arc<Mutex<Store>>,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::from_default_env().add_directive("nostrbox=info".parse().unwrap()),
        )
        .init();

    // Open store
    let store = Store::open("nostrbox.db").expect("failed to open store");
    let state = AppState {
        store: Arc::new(Mutex::new(store)),
    };

    // Build router
    let app = Router::new()
        .route("/health", get(health))
        .route("/api/op", post(handle_operation))
        // TODO: Serve static web UI files from web/dist/
        // .nest_service("/", tower_http::services::ServeDir::new("web/dist"))
        .layer(CorsLayer::permissive())
        .with_state(state);

    let bind = "0.0.0.0:3000";
    info!("nostrbox server starting on {bind}");
    let listener = tokio::net::TcpListener::bind(bind).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn health() -> &'static str {
    "ok"
}

/// ContextVM operation endpoint.
///
/// TODO: Replace with proper ContextVM transport (websocket or SDK handler)
/// once rust-contextvm-sdk is available. This HTTP POST endpoint is a
/// placeholder that mirrors the operation dispatch pattern.
async fn handle_operation(
    State(state): State<AppState>,
    Json(req): Json<OperationRequest>,
) -> (StatusCode, Json<OperationResponse>) {
    let store = state.store.lock().unwrap();
    let handler = OperationHandler::new(&store);
    let resp = handler.handle(&req);
    let status = if resp.ok {
        StatusCode::OK
    } else {
        StatusCode::BAD_REQUEST
    };
    (status, Json(resp))
}
