//! Write admission checks — simplified from original, no trait adapters.

use nostrbox_core::{GlobalRole, Pubkey};
use nostrbox_store::Store;

use crate::config::RelayAccessConfig;

/// Result of an admission check.
#[derive(Debug)]
pub enum AdmissionResult {
    Allow,
    Deny(String),
}

/// Check if a pubkey is allowed to write an event of the given kind.
pub fn check_write_admission(
    store: &Store,
    pubkey: &Pubkey,
    kind: u16,
    access_config: &RelayAccessConfig,
) -> AdmissionResult {
    // Check bypass kinds first (e.g., NIP-59 gift wraps for ContextVM transport).
    if access_config.write_bypass_kinds.contains(&kind) {
        return AdmissionResult::Allow;
    }

    match store.get_actor(pubkey) {
        Ok(Some(actor)) => {
            let role_access = access_config.role_access(actor.global_role);
            if role_access.can_write(kind) {
                AdmissionResult::Allow
            } else {
                AdmissionResult::Deny(format!(
                    "{:?} cannot write kind {}",
                    actor.global_role, kind
                ))
            }
        }
        Ok(None) => AdmissionResult::Deny("unknown actor".into()),
        Err(e) => AdmissionResult::Deny(format!("store error: {e}")),
    }
}

/// Get the role of a pubkey, defaulting to Guest if unknown.
pub fn get_role(store: &Store, pubkey: &str) -> GlobalRole {
    match store.get_actor(pubkey) {
        Ok(Some(actor)) => actor.global_role,
        _ => GlobalRole::Guest,
    }
}
