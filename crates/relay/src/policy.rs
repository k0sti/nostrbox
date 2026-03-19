use std::fmt;
use std::net::SocketAddr;

use nostr_relay_builder::prelude::*;
use nostrbox_core::GlobalRole;
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
    ) -> BoxedFuture<'a, WritePolicyResult> {
        Box::pin(async move {
            // Always allow NIP-59 gift wraps (kind 1059/1060) — needed for
            // encrypted ContextVM transport. These use ephemeral sender keys
            // that won't match any registered actor.
            let kind = event.kind.as_u16();
            if kind == 1059 || kind == 1060 {
                return WritePolicyResult::Accept;
            }

            let pubkey = event.pubkey.to_hex();
            match self.pool.get() {
                Ok(store) => match check_write_admission(&store, &pubkey) {
                    AdmissionResult::Allow => WritePolicyResult::Accept,
                    AdmissionResult::Deny(msg) => WritePolicyResult::reject(MachineReadablePrefix::Blocked, msg),
                },
                Err(e) => WritePolicyResult::reject(MachineReadablePrefix::Error, format!("store error: {e}")),
            }
        })
    }
}

/// Query/read policy that checks actor role before allowing REQ.
///
/// - Unauthenticated: denied (NIP-42 should block this, but defense in depth)
/// - Guest: can only query kind 9021 (join requests) and kind 0 (metadata)
/// - Member+: can query all kinds
pub struct NostrboxQueryPolicy {
    pool: StorePool,
}

impl fmt::Debug for NostrboxQueryPolicy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("NostrboxQueryPolicy").finish()
    }
}

impl NostrboxQueryPolicy {
    pub fn new(pool: StorePool) -> Self {
        Self { pool }
    }

    fn get_role(&self, pubkey: &str) -> GlobalRole {
        match self.pool.get() {
            Ok(store) => match store.get_actor(pubkey) {
                Ok(Some(actor)) => actor.global_role,
                _ => GlobalRole::Guest,
            },
            Err(_) => GlobalRole::Guest,
        }
    }
}

/// Kinds that guests are allowed to query.
const GUEST_READ_KINDS: &[u16] = &[
    0,     // Metadata (needed for profile display)
    9021,  // Join request (so they can check their own request status)
];

impl QueryPolicy for NostrboxQueryPolicy {
    fn admit_query<'a>(
        &'a self,
        query: &'a Filter,
        _addr: &'a SocketAddr,
        authed_pubkey: Option<&'a PublicKey>,
    ) -> BoxedFuture<'a, QueryPolicyResult> {
        Box::pin(async move {
            let Some(pubkey) = authed_pubkey else {
                tracing::debug!("query rejected: no authed pubkey");
                return QueryPolicyResult::reject(
                    MachineReadablePrefix::AuthRequired,
                    "authentication required",
                );
            };

            let hex = pubkey.to_hex();
            let role = self.get_role(&hex);
            tracing::debug!(pubkey = %hex, role = ?role, "query policy check");

            // Members, admins, owners can read everything
            if role != GlobalRole::Guest {
                return QueryPolicyResult::Accept;
            }

            // Guests: check if querying only allowed kinds
            if let Some(ref kinds) = query.kinds {
                let all_allowed = kinds.iter().all(|k| GUEST_READ_KINDS.contains(&k.as_u16()));
                if all_allowed {
                    return QueryPolicyResult::Accept;
                }
            }

            // Guest with no kind filter or disallowed kinds → deny
            QueryPolicyResult::reject(
                MachineReadablePrefix::Restricted,
                "guests have limited read access",
            )
        })
    }
}
