use axum::{
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};

use crate::RelayState;

/// Serve NIP-11 relay information document (called from lib.rs ws_handler).
pub fn serve_nip11(headers: &HeaderMap, state: &RelayState) -> Response {
    let accept = headers
        .get("accept")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    if accept.contains("application/nostr+json") {
        let info = serde_json::json!({
            "name": state.config.name,
            "description": state.config.description,
            "supported_nips": [1, 9, 11, 33, 42],
            "software": "nostrbox",
            "version": env!("CARGO_PKG_VERSION"),
            "relay_url": state.config.public_relay_url,
            "pubkey": state.config.server_pubkey,
        });
        (
            StatusCode::OK,
            [(axum::http::header::CONTENT_TYPE, "application/nostr+json")],
            serde_json::to_string(&info).unwrap_or_default(),
        )
            .into_response()
    } else {
        let info = serde_json::json!({
            "relay_url": state.config.public_relay_url,
            "pubkey": state.config.server_pubkey,
            "status": "running",
        });
        (
            StatusCode::OK,
            [(axum::http::header::CONTENT_TYPE, "application/json")],
            serde_json::to_string(&info).unwrap_or_default(),
        )
            .into_response()
    }
}
