use nostr_relay_builder::prelude::*;
use tracing::info;

use nostrbox_store::StorePool;

use crate::policy::{NostrboxWritePolicy, NostrboxQueryPolicy};

/// Configuration for the Nostrbox relay.
pub struct RelayConfig {
    pub port: u16,
}

impl Default for RelayConfig {
    fn default() -> Self {
        Self { port: 7777 }
    }
}

/// Start the Nostrbox relay with admission policies.
///
/// Returns the running LocalRelay. The relay auto-shuts down when dropped.
pub async fn start_relay(
    config: RelayConfig,
    pool: StorePool,
) -> Result<LocalRelay, nostr_relay_builder::Error> {
    let write_policy = NostrboxWritePolicy::new(pool.clone());
    let query_policy = NostrboxQueryPolicy::new(pool);

    let relay = LocalRelay::builder()
        .port(config.port)
        .nip42(LocalRelayBuilderNip42::default())
        .write_policy(write_policy)
        .query_policy(query_policy)
        .build()?;

    relay.run().await?;
    let url = relay.url().await;
    info!(url = %url, "nostrbox relay started");
    Ok(relay)
}
