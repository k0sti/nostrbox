//! Nostrbox relay — purpose-built Nostr relay using axum WebSocket.
//!
//! Provides:
//! - NIP-01: EVENT, REQ, CLOSE, OK, EOSE, NOTICE
//! - NIP-11: Relay information document
//! - NIP-33: Parameterized replaceable events
//! - NIP-42: AUTH challenge/response
//! - Role-based write admission and read filtering

pub mod admission;
pub mod broadcast;
pub mod config;
pub mod nip11;
pub mod nip42;
pub mod protocol;
pub mod query;
pub mod session;

use std::sync::Arc;

use axum::{
    Router,
    extract::{State, ws::WebSocketUpgrade},
    http::HeaderMap,
    response::IntoResponse,
    routing::get,
};
use nostrbox_store::StorePool;

use crate::broadcast::Broadcaster;
use crate::config::RelayConfig;

/// Shared relay state.
#[derive(Clone)]
pub struct RelayState {
    pub pool: StorePool,
    pub config: Arc<RelayConfig>,
    pub broadcaster: Arc<Broadcaster>,
}

/// Create the relay axum routes to be merged into the main router.
///
/// The returned router handles:
/// - `GET /relay` — WebSocket upgrade (relay protocol)
/// - `GET /relay` — NIP-11 info document (when no upgrade header)
pub fn relay_routes(pool: StorePool, config: RelayConfig) -> Router {
    let state = RelayState {
        pool,
        config: Arc::new(config),
        broadcaster: Arc::new(Broadcaster::new()),
    };
    Router::new()
        .route("/relay", get(ws_handler))
        .route("/relay/info", get(nip11_handler))
        .with_state(state)
}

/// WebSocket upgrade handler for relay connections.
async fn ws_handler(
    State(state): State<RelayState>,
    ws: WebSocketUpgrade,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| session::handle_session(socket, state))
}

/// NIP-11 relay info handler (also usable as fallback from /ws when no upgrade).
async fn nip11_handler(
    headers: HeaderMap,
    State(state): State<RelayState>,
) -> impl IntoResponse {
    nip11::serve_nip11(&headers, &state)
}
