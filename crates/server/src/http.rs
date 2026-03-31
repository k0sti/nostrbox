//! HTTP route handlers: operation dispatch, relay info, health.

use std::net::SocketAddr;
use std::sync::Arc;

use axum::{
    extract::{ConnectInfo, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Json},
};

use nostrbox_core::{AuthConfig, EmailConfig};
use nostrbox_ext_email_login::EmailHandler;
use nostrbox_ext_management::ManagementHandler;
use nostrbox_ext_management::types::{AuthSource, ErrorCode, OperationRequest, OperationResponse};
use nostrbox_store::StorePool;

use crate::auth::verify_nip98;

/// Shared application state.
#[derive(Clone)]
pub struct AppState {
    pub pool: StorePool,
    pub keys: Option<Arc<nostr_sdk::Keys>>,
    pub public_relay_url: String,
    pub email_config: Arc<EmailConfig>,
    pub public_url: Option<String>,
    pub auth_config: AuthConfig,
}

pub async fn health() -> &'static str {
    "ok"
}

/// NIP-11 relay information document.
pub async fn relay_info(
    headers: HeaderMap,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let accept = headers
        .get("accept")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    let server_pubkey = state
        .keys
        .as_ref()
        .map(|k| k.public_key().to_hex())
        .unwrap_or_default();

    if accept.contains("application/nostr+json") {
        let info = serde_json::json!({
            "name": "nostrbox",
            "description": "Nostrbox community relay",
            "supported_nips": [1, 9, 11, 42, 98],
            "software": "nostrbox",
            "version": env!("CARGO_PKG_VERSION"),
            "relay_url": state.public_relay_url,
            "pubkey": server_pubkey,
        });
        (
            StatusCode::OK,
            [("content-type", "application/nostr+json")],
            serde_json::to_string(&info).unwrap(),
        )
            .into_response()
    } else {
        (
            StatusCode::OK,
            [("content-type", "application/json")],
            serde_json::json!({
                "relay_url": state.public_relay_url,
                "pubkey": server_pubkey,
                "status": "running",
            })
            .to_string(),
        )
            .into_response()
    }
}

fn build_request_url(state: &AppState, headers: &HeaderMap) -> String {
    if let Some(ref base) = state.public_url {
        format!("{}/api/op", base.trim_end_matches('/'))
    } else {
        let host = headers
            .get("host")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("localhost");
        format!("http://{host}/api/op")
    }
}

/// Operation endpoint with NIP-98 authentication.
/// Dispatches to management handler first, then email handler.
pub async fn handle_operation(
    State(state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Json(req): Json<OperationRequest>,
) -> (StatusCode, Json<OperationResponse>) {
    let mut req = req;
    let is_local = addr.ip().is_loopback();

    if let Some(auth) = headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .filter(|v| v.starts_with("Nostr "))
    {
        let request_url = build_request_url(&state, &headers);
        match verify_nip98(auth, &request_url, "POST", state.auth_config.auth_window_secs) {
            Ok(pubkey) => {
                req.caller = Some(pubkey);
                req.auth_source = AuthSource::Nip98;
            }
            Err(e) => {
                tracing::warn!("NIP-98 auth failed: {e}");
                return (
                    StatusCode::UNAUTHORIZED,
                    Json(OperationResponse::error_with_code(
                        ErrorCode::Unauthorized,
                        format!("NIP-98 auth failed: {e}"),
                    )),
                );
            }
        }
    } else if is_local && state.auth_config.local_bypass {
        req.auth_source = AuthSource::LocalBypass;
    } else {
        req.caller = None;
    }

    let pool = state.pool.clone();
    let keys = state.keys.clone();
    let email_config = state.email_config.clone();
    let resp = tokio::task::spawn_blocking(move || {
        let store = pool.get().expect("failed to get store connection");

        // Try management handler first
        let mgmt = if let Some(ref keys) = keys {
            ManagementHandler::with_keys(&store, keys)
        } else {
            ManagementHandler::new(&store)
        };
        let mgmt_resp = mgmt.handle(&req);
        if mgmt_resp.error_code.as_deref() != Some("unknown_operation") {
            return mgmt_resp;
        }

        // Fall through to email handler
        let mut email = EmailHandler::new(&store, &email_config);
        if let Some(ref keys) = keys {
            email = email.with_keys(keys);
        }
        email.handle(&req)
    })
    .await
    .unwrap();

    let status = if resp.ok {
        StatusCode::OK
    } else if resp.error_code.as_deref() == Some("unauthorized") {
        StatusCode::UNAUTHORIZED
    } else {
        StatusCode::BAD_REQUEST
    };
    (status, Json(resp))
}
