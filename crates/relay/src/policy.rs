use std::fmt;
use std::net::SocketAddr;

use nostr_relay_builder::prelude::*;
use nostrbox_store::StorePool;

use crate::admission::{check_write_admission, AdmissionResult};

/// Write policy that checks actor permissions via the store.
pub struct NostrboxWritePolicy {
    pool: StorePool,
}

impl fmt::Debug for NostrboxWritePolicy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("NostrboxWritePolicy").finish()
    }
}

impl NostrboxWritePolicy {
    pub fn new(pool: StorePool) -> Self {
        Self { pool }
    }
}

impl WritePolicy for NostrboxWritePolicy {
    fn admit_event<'a>(
        &'a self,
        event: &'a Event,
        _addr: &'a SocketAddr,
    ) -> BoxedFuture<'a, PolicyResult> {
        Box::pin(async move {
            // Always allow NIP-59 gift wraps (kind 1059/1060) — needed for
            // encrypted ContextVM transport. These use ephemeral sender keys
            // that won't match any registered actor.
            let kind = event.kind.as_u16();
            if kind == 1059 || kind == 1060 {
                return PolicyResult::Accept;
            }

            let pubkey = event.pubkey.to_hex();
            match self.pool.get() {
                Ok(store) => match check_write_admission(&store, &pubkey) {
                    AdmissionResult::Allow => PolicyResult::Accept,
                    AdmissionResult::Deny(msg) => PolicyResult::Reject(msg),
                },
                Err(e) => PolicyResult::Reject(format!("store error: {e}")),
            }
        })
    }
}
