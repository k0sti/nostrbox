//! NostrBox server: composition root.

mod auth;
mod http;
mod ingestion;

use std::net::SocketAddr;
use std::sync::Arc;

use axum::{Router, routing::{get, post}};
use tower_http::cors::CorsLayer;
use tracing::info;
use tracing_subscriber::EnvFilter;

use nostr_sdk::ToBech32;
use nostrbox_core::Config;
use nostrbox_relay::config::{RelayAccessConfig, RelayConfig};
use nostrbox_store::StorePool;

use http::AppState;

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

    // Open store pool
    let pool = StorePool::open(&config.db_path, 4).expect("failed to open store pool");

    // Register server + owner actors
    if let Some(ref keys) = keys {
        register_actor(&pool, &keys.public_key().to_hex(), &keys.public_key().to_bech32().unwrap_or_default(),
            nostrbox_core::ActorKind::System, "nostrbox");
    }
    if let Some(ref owner_spec) = config.auth.owner_pubkey {
        if let Some(hex) = resolve_pubkey_hex(owner_spec) {
            let npub = nostr_sdk::PublicKey::parse(&hex)
                .map(|pk| pk.to_bech32().unwrap_or_default()).unwrap_or_default();
            register_actor(&pool, &hex, &npub, nostrbox_core::ActorKind::Human, "owner");
        }
    }

    // Relay
    let server_pubkey = keys.as_ref().map(|k| k.public_key().to_hex()).unwrap_or_default();
    let public_relay_url = config.public_relay_url();
    let relay_access: RelayAccessConfig = serde_json::from_value(config.relay.clone()).unwrap_or_default();
    let relay_config = RelayConfig {
        name: "nostrbox".into(),
        description: "Nostrbox community relay".into(),
        server_pubkey,
        public_relay_url: public_relay_url.clone(),
        access: relay_access,
    };
    let relay_router = nostrbox_relay::relay_routes(pool.clone(), relay_config);
    info!(url = %public_relay_url, "relay ready");

    // Email config (env override)
    let mut email_config = config.email.clone();
    if let Ok(key) = std::env::var("RESEND_API_KEY") {
        email_config.resend_api_key = key;
    }
    if email_config.public_url.is_empty() {
        if let Some(ref url) = config.public_url {
            email_config.public_url = url.clone();
        }
    }
    let email_config = Arc::new(email_config);
    info!("email login {}", if email_config.is_enabled() { "enabled (Resend)" } else { "disabled" });

    // Background tasks
    spawn_contextvm_transport(&config, &keys, &pool, &email_config);
    spawn_event_ingestion(&config, &keys, &pool);
    spawn_cleanup_job(&pool, &email_config);

    let state = AppState {
        pool,
        keys,
        public_relay_url,
        email_config,
        public_url: config.public_url.clone(),
        auth_config: config.auth.clone(),
    };

    // Router
    let app = Router::new()
        .route("/health", get(http::health))
        .route("/api/op", post(http::handle_operation))
        .route("/api/relay-info", get(http::relay_info))
        .with_state(state)
        .merge(relay_router)
        .merge(nostrbox_ext_webui::webui_routes(&config.web_dist_path))
        .layer(CorsLayer::permissive());

    info!("nostrbox server starting on {}", config.bind_address);
    let listener = tokio::net::TcpListener::bind(&config.bind_address).await.unwrap();
    axum::serve(listener, app.into_make_service_with_connect_info::<SocketAddr>()).await.unwrap();
}

// ── Helpers ─────────────────────────────────────────────────────────

fn register_actor(pool: &StorePool, pubkey: &str, npub: &str, kind: nostrbox_core::ActorKind, label: &str) {
    let store = pool.get().expect("failed to get store connection");
    let now = now_secs();
    let actor = nostrbox_core::Actor {
        pubkey: pubkey.to_string(),
        npub: npub.to_string(),
        kind,
        global_role: nostrbox_core::GlobalRole::Owner,
        status: nostrbox_core::ActorStatus::Active,
        display_name: Some(label.to_string()),
        groups: vec![],
        created_at: now,
        updated_at: now,
    };
    match store.upsert_actor(&actor) {
        Ok(()) => info!(pubkey = %pubkey, "{label} actor registered"),
        Err(e) => tracing::error!("failed to register {label} actor: {e}"),
    }
}

fn resolve_pubkey_hex(spec: &str) -> Option<String> {
    if spec.starts_with("npub1") {
        match nostr_sdk::PublicKey::parse(spec) {
            Ok(pk) => Some(pk.to_hex()),
            Err(e) => { tracing::error!("invalid owner_pubkey: {e}"); None }
        }
    } else {
        Some(spec.to_string())
    }
}

fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn spawn_contextvm_transport(
    config: &Config, keys: &Option<Arc<nostr_sdk::Keys>>,
    pool: &StorePool, email_config: &Arc<nostrbox_core::EmailConfig>,
) {
    let Some(keys) = keys.clone() else { return };
    let relay_url = if !config.relay_urls.is_empty() {
        config.relay_urls[0].clone()
    } else {
        format!("ws://{}/relay", config.bind_address)
    };
    let pool = pool.clone();
    let email_config = email_config.clone();
    tokio::spawn(async move {
        if let Err(e) = nostrbox_ext_contextvm::start_contextvm_transport(&relay_url, &keys, pool, email_config).await {
            tracing::error!("ContextVM transport failed: {e}");
        }
    });
}

fn spawn_event_ingestion(
    config: &Config, keys: &Option<Arc<nostr_sdk::Keys>>,
    pool: &StorePool,
) {
    let keys = keys.clone();
    let relay_url = if !config.relay_urls.is_empty() {
        config.relay_urls[0].clone()
    } else {
        format!("ws://{}/relay", config.bind_address)
    };
    let pool = pool.clone();
    tokio::spawn(async move {
        if let Err(e) = ingestion::start_event_ingestion(keys.as_ref().map(|v| &**v), &relay_url, pool).await {
            tracing::error!("event ingestion failed: {e}");
        }
    });
}

fn spawn_cleanup_job(pool: &StorePool, email_config: &Arc<nostrbox_core::EmailConfig>) {
    let pool = pool.clone();
    let email_config = email_config.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(300));
        loop {
            interval.tick().await;
            let pool = pool.clone();
            let ttl = email_config.abandoned_ttl();
            if let Err(e) = tokio::task::spawn_blocking(move || {
                let store = pool.get()?;
                let tokens = store.cleanup_login_tokens().unwrap_or(0);
                let emails = store.cleanup_abandoned_email_identities(ttl).unwrap_or(0);
                let audit = store.cleanup_relay_audit_log(86400 * 30).unwrap_or(0);
                if tokens > 0 || emails > 0 || audit > 0 {
                    info!(tokens, emails, audit, "cleanup completed");
                }
                Ok::<(), Box<dyn std::error::Error + Send + Sync>>(())
            }).await {
                tracing::warn!("cleanup task failed: {e}");
            }
        }
    });
}
