use nostr_relay_builder::prelude::*;
use tracing::info;

use nostrbox_store::StorePool;

use crate::policy::NostrboxWritePolicy;

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
    let write_policy = NostrboxWritePolicy::new(pool);

    let builder = RelayBuilder::default()
        .port(config.port)
        .write_policy(write_policy);

    let relay = LocalRelay::run(builder).await?;
    info!(url = %relay.url(), "nostrbox relay started");
    Ok(relay)
}
