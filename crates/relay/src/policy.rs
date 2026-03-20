use std::fmt;
use std::net::SocketAddr;
use std::sync::Arc;

use nostr_relay_builder::prelude::*;
use nostrbox_core::GlobalRole;
use nostrbox_store::StorePool;

use crate::admission::{check_write_admission, AdmissionResult};
use crate::config::RelayAccessConfig;

/// Write policy that checks actor permissions via the store and access config.
pub struct NostrboxWritePolicy {
    pool: StorePool,
    access_config: Arc<RelayAccessConfig>,
}

impl fmt::Debug for NostrboxWritePolicy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("NostrboxWritePolicy").finish()
    }
}

impl NostrboxWritePolicy {
    pub fn new(pool: StorePool, access_config: Arc<RelayAccessConfig>) -> Self {
        Self { pool, access_config }
    }
}

impl WritePolicy for NostrboxWritePolicy {
    fn admit_event<'a>(
        &'a self,
        event: &'a Event,
        addr: &'a SocketAddr,
    ) -> BoxedFuture<'a, WritePolicyResult> {
        Box::pin(async move {
            let kind = event.kind.as_u16();

            // Check bypass kinds (e.g., NIP-59 gift wraps for ContextVM transport).
            if self.access_config.write_bypass_kinds.contains(&kind) {
                return WritePolicyResult::Accept;
            }

            let pubkey = event.pubkey.to_hex();
            match self.pool.get() {
                Ok(store) => {
                    match check_write_admission(&store, &pubkey, kind, &self.access_config) {
                        AdmissionResult::Allow => WritePolicyResult::Accept,
                        AdmissionResult::Deny(reason) => {
                            let role = match store.get_actor(&pubkey) {
                                Ok(Some(actor)) => format!("{:?}", actor.global_role).to_lowercase(),
                                Ok(None) => "unknown".to_string(),
                                Err(_) => "unknown".to_string(),
                            };
                            tracing::debug!(pubkey = %pubkey, kind, %reason, "write denied");
                            let _ = store.log_relay_denial(
                                Some(&pubkey),
                                Some(kind),
                                "write",
                                &role,
                                &reason,
                                Some(&addr.to_string()),
                            );
                            WritePolicyResult::reject(MachineReadablePrefix::Blocked, reason)
                        }
                    }
                }
                Err(e) => WritePolicyResult::reject(MachineReadablePrefix::Error, format!("store error: {e}")),
            }
        })
    }
}

/// Query/read policy that checks actor role before allowing REQ.
pub struct NostrboxQueryPolicy {
    pool: StorePool,
    access_config: Arc<RelayAccessConfig>,
}

impl fmt::Debug for NostrboxQueryPolicy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("NostrboxQueryPolicy").finish()
    }
}

impl NostrboxQueryPolicy {
    pub fn new(pool: StorePool, access_config: Arc<RelayAccessConfig>) -> Self {
        Self { pool, access_config }
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

impl QueryPolicy for NostrboxQueryPolicy {
    fn admit_query<'a>(
        &'a self,
        query: &'a Filter,
        addr: &'a SocketAddr,
        authed_pubkey: Option<&'a PublicKey>,
    ) -> BoxedFuture<'a, QueryPolicyResult> {
        Box::pin(async move {
            let Some(pubkey) = authed_pubkey else {
                tracing::debug!("query rejected: no authed pubkey");
                if let Ok(store) = self.pool.get() {
                    let _ = store.log_relay_denial(
                        None,
                        None,
                        "read",
                        "unauthenticated",
                        "authentication required",
                        Some(&addr.to_string()),
                    );
                }
                return QueryPolicyResult::reject(
                    MachineReadablePrefix::AuthRequired,
                    "authentication required",
                );
            };

            let hex = pubkey.to_hex();
            let role = self.get_role(&hex);
            let role_access = self.access_config.role_access(role);
            tracing::debug!(pubkey = %hex, role = ?role, "query policy check");

            // If role has read_all, allow everything
            if role_access.read_all {
                return QueryPolicyResult::Accept;
            }

            // Check if querying only allowed kinds
            if let Some(ref kinds) = query.kinds {
                let all_allowed = kinds.iter().all(|k| role_access.can_read(k.as_u16()));
                if all_allowed {
                    return QueryPolicyResult::Accept;
                }
            }

            // Deny: no kind filter or disallowed kinds
            let reason = format!("{:?}s have limited read access", role);
            tracing::debug!(pubkey = %hex, role = ?role, %reason, "query denied");
            if let Ok(store) = self.pool.get() {
                let _ = store.log_relay_denial(
                    Some(&hex),
                    None,
                    "read",
                    &format!("{:?}", role).to_lowercase(),
                    &reason,
                    Some(&addr.to_string()),
                );
            }
            QueryPolicyResult::reject(
                MachineReadablePrefix::Restricted,
                reason,
            )
        })
    }
}
