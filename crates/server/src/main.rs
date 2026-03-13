use std::sync::Arc;

use axum::{
    Router,
    extract::State,
    extract::ws::{WebSocket, WebSocketUpgrade, Message},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Json},
    routing::{any, get, post},
};
use futures_util::{SinkExt, StreamExt};
use tower_http::cors::CorsLayer;
use tower_http::services::{ServeDir, ServeFile};
use tracing::info;
use tracing_subscriber::EnvFilter;

use nostr_sdk::ToBech32;
use nostrbox_contextvm::{OperationRequest, OperationResponse};
use nostrbox_contextvm::OperationHandler;
use nostrbox_relay::setup::{RelayConfig, start_relay};
use nostrbox_store::StorePool;

/// Server configuration.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Config {
    pub bind_address: String,
    pub db_path: String,
    pub web_dist_path: String,
    pub identity_nsec: Option<String>,
    pub relay_port: u16,
    pub relay_urls: Vec<String>,
    /// Public base URL (e.g. "https://nostrbox.atlantislabs.space").
    /// Used to derive the public relay WebSocket URL (wss://.../ws).
    pub public_url: Option<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            bind_address: "0.0.0.0:3000".into(),
            db_path: "nostrbox.db".into(),
            web_dist_path: "web/dist".into(),
            identity_nsec: None,
            relay_port: 7777,
            relay_urls: vec![],
            public_url: None,
        }
    }
}

impl Config {
    pub fn load() -> Self {
        let path = std::env::var("NOSTRBOX_CONFIG").unwrap_or_else(|_| "nostrbox.toml".into());
        match std::fs::read_to_string(&path) {
            Ok(contents) => match toml_parse::from_str(&contents) {
                Ok(config) => {
                    info!("loaded config from {path}");
                    config
                }
                Err(e) => {
                    tracing::warn!("failed to parse config {path}: {e}, using defaults");
                    Self::default()
                }
            },
            Err(_) => {
                info!("no config file found at {path}, using defaults");
                Self::default()
            }
        }
    }

    /// Derive the public relay URL from public_url config or fall back to local relay URL.
    fn public_relay_url(&self, local_relay_url: &str) -> String {
        if let Some(ref base) = self.public_url {
            let scheme = if base.starts_with("https://") { "wss" } else { "ws" };
            let host = base
                .trim_start_matches("https://")
                .trim_start_matches("http://")
                .trim_end_matches('/');
            format!("{scheme}://{host}/ws")
        } else {
            local_relay_url.to_string()
        }
    }
}

#[derive(Clone)]
struct AppState {
    pool: StorePool,
    keys: Option<Arc<nostr_sdk::Keys>>,
    /// Public-facing relay URL (for NIP-11 / client use).
    public_relay_url: String,
    /// Internal relay URL (ws://127.0.0.1:PORT) for proxying.
    local_relay_url: String,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::from_default_env().add_directive("nostrbox=info".parse().unwrap()),
        )
        .init();

    let config = Config::load();

    // Resolve identity
    let keys = if let Some(ref nsec) = config.identity_nsec {
        match nostr_sdk::SecretKey::parse(nsec) {
            Ok(sk) => {
                let keys = nostr_sdk::Keys::new(sk);
                info!(npub = %keys.public_key().to_bech32().unwrap_or_default(), "server identity loaded");
                Some(Arc::new(keys))
            }
            Err(e) => {
                tracing::error!("invalid identity_nsec in config: {e}");
                std::process::exit(1);
            }
        }
    } else {
        tracing::warn!("no identity_nsec configured — run `just init` to set up");
        None
    };

    // Open store pool (4 connections, WAL mode for concurrent reads)
    let pool = StorePool::open(&config.db_path, 4).expect("failed to open store pool");

    // Start relay
    let relay_config = RelayConfig {
        port: config.relay_port,
    };
    let relay = start_relay(relay_config, pool.clone())
        .await
        .expect("failed to start relay");
    let local_relay_url = relay.url();
    let public_relay_url = config.public_relay_url(&local_relay_url);
    info!(local = %local_relay_url, public = %public_relay_url, "relay running");

    // Start ContextVM transport (if identity is configured)
    if let Some(ref keys) = keys {
        let transport_pool = pool.clone();
        let transport_keys = keys.clone();
        let transport_relay_url = local_relay_url.clone();
        tokio::spawn(async move {
            if let Err(e) =
                start_contextvm_transport(&transport_relay_url, &transport_keys, transport_pool)
                    .await
            {
                tracing::error!("ContextVM transport failed: {e}");
            }
        });
    }

    // Start event ingestion pipeline
    {
        let ingestion_pool = pool.clone();
        let ingestion_relay_url = local_relay_url.clone();
        tokio::spawn(async move {
            if let Err(e) = start_event_ingestion(&ingestion_relay_url, ingestion_pool).await {
                tracing::error!("event ingestion failed: {e}");
            }
        });
    }

    let state = AppState {
        pool,
        keys,
        public_relay_url,
        local_relay_url,
    };

    // SPA fallback: serve index.html for non-API routes
    let spa_fallback = ServeFile::new(format!("{}/index.html", config.web_dist_path));
    let serve_dir = ServeDir::new(&config.web_dist_path).fallback(spa_fallback);

    // Build router
    let app = Router::new()
        .route("/health", get(health))
        .route("/api/op", post(handle_operation))
        .route("/api/relay-info", get(relay_info))
        .route("/ws", get(ws_upgrade))
        .fallback_service(serve_dir)
        .layer(CorsLayer::permissive())
        .with_state(state);

    info!("nostrbox server starting on {}", config.bind_address);
    let listener = tokio::net::TcpListener::bind(&config.bind_address)
        .await
        .unwrap();

    // Keep relay alive for the lifetime of the server
    let _relay = relay;
    axum::serve(listener, app).await.unwrap();
}

/// WebSocket proxy + NIP-11 handler for /ws.
///
/// If the request is a WebSocket upgrade, proxy to the local relay.
/// Otherwise, serve NIP-11 relay information document (SDK clients
/// fetch this via plain HTTP GET on the relay URL).
/// WebSocket upgrade handler for /ws.
async fn ws_upgrade(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
) -> impl IntoResponse {
    ws.on_upgrade(move |client_ws| relay_proxy(client_ws, state.local_relay_url))
}

/// NIP-11 relay info for plain GET /ws (no upgrade header).
async fn ws_info(
    headers: HeaderMap,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let server_pubkey = state
        .keys
        .as_ref()
        .map(|k| k.public_key().to_hex())
        .unwrap_or_default();

    let accept = headers
        .get("accept")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    if accept.contains("application/nostr+json") {
        let info = serde_json::json!({
            "name": "nostrbox",
            "description": "Nostrbox community relay",
            "supported_nips": [1, 9, 11, 42, 59],
            "software": "nostrbox",
            "version": env!("CARGO_PKG_VERSION"),
            "relay_url": state.public_relay_url,
            "pubkey": server_pubkey,
        });
        (
            StatusCode::OK,
            [(axum::http::header::CONTENT_TYPE, "application/nostr+json")],
            serde_json::to_string(&info).unwrap(),
        )
            .into_response()
    } else {
        (
            StatusCode::OK,
            [(axum::http::header::CONTENT_TYPE, "application/json")],
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

async fn relay_proxy(client_ws: WebSocket, relay_url: String) {
    // Connect to the local relay
    let upstream = match tokio_tungstenite::connect_async(&relay_url).await {
        Ok((ws, _)) => ws,
        Err(e) => {
            tracing::warn!("failed to connect to local relay: {e}");
            return;
        }
    };

    let (mut client_tx, mut client_rx) = client_ws.split();
    let (mut relay_tx, mut relay_rx) = upstream.split();

    // Client → Relay
    let client_to_relay = async {
        while let Some(Ok(msg)) = client_rx.next().await {
            let tung_msg = match msg {
                Message::Text(t) => tokio_tungstenite::tungstenite::Message::Text(t.to_string().into()),
                Message::Binary(b) => tokio_tungstenite::tungstenite::Message::Binary(b),
                Message::Ping(p) => tokio_tungstenite::tungstenite::Message::Ping(p),
                Message::Pong(p) => tokio_tungstenite::tungstenite::Message::Pong(p),
                Message::Close(_) => break,
            };
            if relay_tx.send(tung_msg).await.is_err() {
                break;
            }
        }
    };

    // Relay → Client
    let relay_to_client = async {
        while let Some(Ok(msg)) = relay_rx.next().await {
            let axum_msg = match msg {
                tokio_tungstenite::tungstenite::Message::Text(t) => Message::Text(t.to_string().into()),
                tokio_tungstenite::tungstenite::Message::Binary(b) => Message::Binary(b),
                tokio_tungstenite::tungstenite::Message::Ping(p) => Message::Ping(p),
                tokio_tungstenite::tungstenite::Message::Pong(p) => Message::Pong(p),
                tokio_tungstenite::tungstenite::Message::Close(_) => break,
                _ => continue,
            };
            if client_tx.send(axum_msg).await.is_err() {
                break;
            }
        }
    };

    tokio::select! {
        _ = client_to_relay => {},
        _ = relay_to_client => {},
    }
}

/// Start the ContextVM transport: listen for incoming JSON-RPC requests over Nostr
/// and dispatch them to the OperationHandler.
async fn start_contextvm_transport(
    relay_url: &str,
    keys: &nostr_sdk::Keys,
    pool: StorePool,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    use contextvm_sdk::core::types::{JsonRpcMessage, JsonRpcResponse, ServerInfo};
    use contextvm_sdk::transport::server::{NostrServerTransport, NostrServerTransportConfig};

    let config = NostrServerTransportConfig {
        relay_urls: vec![relay_url.to_string()],
        server_info: Some(ServerInfo {
            name: Some("nostrbox".into()),
            version: Some(env!("CARGO_PKG_VERSION").into()),
            about: Some("Nostrbox community server".into()),
            ..Default::default()
        }),
        ..Default::default()
    };

    let mut transport = NostrServerTransport::new(keys.clone(), config).await?;
    transport.start().await?;
    info!("ContextVM transport started");

    let mut rx = transport
        .take_message_receiver()
        .ok_or("failed to take message receiver")?;

    while let Some(incoming) = rx.recv().await {
        let event_id = incoming.event_id.clone();
        let client_pubkey = incoming.client_pubkey.clone();

        // Extract method and params from the JSON-RPC request
        let (id, method, params) = match &incoming.message {
            JsonRpcMessage::Request(req) => {
                (req.id.clone(), req.method.clone(), req.params.clone())
            }
            _ => continue, // skip non-requests
        };

        // Map JSON-RPC method to ContextVM operation
        let op_req = OperationRequest {
            op: method,
            params: params.unwrap_or(serde_json::json!({})),
            caller: Some(client_pubkey),
        };

        // Dispatch on a blocking thread (pool hands out a connection)
        let op_pool = pool.clone();
        let keys_clone = keys.clone();
        let resp = tokio::task::spawn_blocking(move || {
            let store = op_pool.get().expect("failed to get store connection");
            let handler = OperationHandler::with_keys(&store, &keys_clone);
            handler.handle(&op_req)
        })
        .await
        .unwrap();

        // Send JSON-RPC response
        let response = JsonRpcMessage::Response(JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id,
            result: serde_json::to_value(&resp).unwrap_or_default(),
        });

        if let Err(e) = transport.send_response(&event_id, response).await {
            tracing::warn!("failed to send ContextVM response: {e}");
        }
    }

    transport.close().await?;
    Ok(())
}

/// Subscribe to the relay for app-relevant kinds and ingest events into the store.
async fn start_event_ingestion(
    relay_url: &str,
    pool: StorePool,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    use nostr_sdk::{Client, Filter, RelayPoolNotification};
    use nostrbox_nostr::kinds;

    let client = Client::default();
    client.add_relay(relay_url).await?;
    client.connect().await;

    // Subscribe to app-relevant kinds
    let filter = Filter::new().kinds(vec![
        kinds::METADATA,         // kind 0
        kinds::ACTOR_ROLE,       // 30078
        kinds::GROUP_DEFINITION, // 30080
        kinds::GROUP_MEMBERSHIP, // 30081
    ]);
    client.subscribe(filter, None).await?;

    info!("event ingestion started");

    let mut notifications = client.notifications();
    while let Ok(notification) = notifications.recv().await {
        let RelayPoolNotification::Event { event, .. } = notification else {
            continue;
        };

        let pool = pool.clone();
        tokio::task::spawn_blocking(move || {
            let store = match pool.get() {
                Ok(s) => s,
                Err(e) => {
                    tracing::warn!("failed to get store connection for ingestion: {e}");
                    return;
                }
            };

            // Store the raw event
            let tags_json = serde_json::to_string(&event.tags).unwrap_or_default();
            let _ = store.store_event(
                &event.id.to_hex(),
                &event.pubkey.to_hex(),
                event.kind.as_u16() as u64,
                event.created_at.as_u64(),
                &event.content,
                &tags_json,
                &event.sig.to_string(),
            );

            // Process by kind
            let pubkey_hex = event.pubkey.to_hex();
            match event.kind {
                k if k == kinds::METADATA => {
                    // Kind 0: update actor display_name from metadata
                    if let Ok(meta) = serde_json::from_str::<serde_json::Value>(&event.content) {
                        if let Some(name) = meta
                            .get("display_name")
                            .or_else(|| meta.get("name"))
                            .and_then(|v| v.as_str())
                        {
                            if let Err(e) = store.update_actor_display_name(&pubkey_hex, name) {
                                tracing::debug!("kind-0 display_name update skipped: {e}");
                            } else {
                                info!(pubkey = %pubkey_hex, name, "ingested kind-0 metadata");
                            }
                        }
                    }
                }
                _ => {
                    // Other app kinds: just store (already done above)
                    info!(kind = event.kind.as_u16(), id = %event.id, "ingested event");
                }
            }
        })
        .await
        .ok();
    }

    Ok(())
}

async fn health() -> &'static str {
    "ok"
}

/// NIP-11 relay information document.
async fn relay_info(
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
            "supported_nips": [1, 9, 11, 42],
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

/// ContextVM operation endpoint.
async fn handle_operation(
    State(state): State<AppState>,
    Json(req): Json<OperationRequest>,
) -> (StatusCode, Json<OperationResponse>) {
    let pool = state.pool.clone();
    let keys = state.keys.clone();
    let resp = tokio::task::spawn_blocking(move || {
        let store = pool.get().expect("failed to get store connection");
        let handler = if let Some(ref keys) = keys {
            OperationHandler::with_keys(&store, keys)
        } else {
            OperationHandler::new(&store)
        };
        handler.handle(&req)
    })
    .await
    .unwrap();
    let status = if resp.ok {
        StatusCode::OK
    } else {
        StatusCode::BAD_REQUEST
    };
    (status, Json(resp))
}
