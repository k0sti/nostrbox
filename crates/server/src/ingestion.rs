//! Event ingestion: subscribe to relay for app-relevant kinds and ingest into store.

use nostr_sdk::{Client, ClientBuilder, Filter, RelayPoolNotification};
use nostrbox_nostr::kinds;
use nostrbox_store::StorePool;
use tracing::info;

/// Subscribe to the relay for app-relevant kinds and ingest events into the store.
pub async fn start_event_ingestion(
    keys: Option<&nostr_sdk::Keys>,
    relay_url: &str,
    pool: StorePool,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let client = if let Some(keys) = keys {
        ClientBuilder::new().signer(keys.clone()).build()
    } else {
        Client::default()
    };
    client.add_relay(relay_url).await?;
    client.connect().await;

    let filter = Filter::new().kinds(vec![
        kinds::METADATA,
        kinds::ACTOR_ROLE,
        kinds::GROUP_DEFINITION,
        kinds::GROUP_MEMBERSHIP,
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

            let pubkey_hex = event.pubkey.to_hex();
            match event.kind {
                k if k == kinds::METADATA => {
                    if let Ok(meta) =
                        serde_json::from_str::<serde_json::Value>(&event.content)
                    {
                        if let Some(name) = meta
                            .get("display_name")
                            .or_else(|| meta.get("name"))
                            .and_then(|v| v.as_str())
                        {
                            if let Err(e) =
                                store.update_actor_display_name(&pubkey_hex, name)
                            {
                                tracing::debug!(
                                    "kind-0 display_name update skipped: {e}"
                                );
                            } else {
                                info!(pubkey = %pubkey_hex, name, "ingested kind-0 metadata");
                            }
                        }
                    }
                }
                _ => {
                    info!(kind = event.kind.as_u16(), id = %event.id, "ingested event");
                }
            }
        })
        .await
        .ok();
    }

    Ok(())
}
