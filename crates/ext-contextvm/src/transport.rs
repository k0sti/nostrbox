//! ContextVM Nostr transport: listen for incoming JSON-RPC requests over Nostr
//! and dispatch them to the management/email handlers.

use std::sync::Arc;

use contextvm_sdk::core::types::{JsonRpcMessage, JsonRpcResponse, ServerInfo};
use contextvm_sdk::transport::server::{NostrServerTransport, NostrServerTransportConfig};
use nostrbox_core::EmailConfig;
use nostrbox_ext_email_login::EmailHandler;
use nostrbox_ext_management::ManagementHandler;
use nostrbox_ext_management::types::{AuthSource, OperationRequest};
use nostrbox_store::StorePool;
use tracing::info;

/// Start the ContextVM transport: listen for incoming JSON-RPC requests over Nostr
/// and dispatch them to the operation handlers.
pub async fn start_contextvm_transport(
    relay_url: &str,
    keys: &nostr_sdk::Keys,
    pool: StorePool,
    email_config: Arc<EmailConfig>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
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

        // Extract method and params from the JSON-RPC request
        let (id, method, params) = match &incoming.message {
            JsonRpcMessage::Request(req) => {
                (req.id.clone(), req.method.clone(), req.params.clone())
            }
            _ => continue, // skip non-requests
        };

        let client_pubkey = incoming.client_pubkey.clone();

        // Map JSON-RPC method to operation request
        let op_req = OperationRequest {
            op: method,
            params: params.unwrap_or(serde_json::json!({})),
            caller: Some(client_pubkey),
            auth_source: AuthSource::ContextVm,
        };

        // Dispatch on a blocking thread
        let op_pool = pool.clone();
        let keys_clone = keys.clone();
        let op_email_config = email_config.clone();
        let resp = tokio::task::spawn_blocking(move || {
            let store = op_pool.get().expect("failed to get store connection");
            let mgmt = ManagementHandler::with_keys(&store, &keys_clone);

            // Try management handler first
            let mgmt_resp = mgmt.handle(&op_req);
            if mgmt_resp.error_code.as_deref() != Some("unknown_operation") {
                return mgmt_resp;
            }

            // Fall through to email handler
            let email = EmailHandler::new(&store, &op_email_config)
                .with_keys(&keys_clone);
            email.handle(&op_req)
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
