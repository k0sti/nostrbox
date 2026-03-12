use nostrbox_core::{GlobalRole, Pubkey, Visibility};
use nostrbox_store::Store;

/// Result of an admission check.
#[derive(Debug)]
pub enum AdmissionResult {
    Allow,
    Deny(String),
}

/// Check if a pubkey is allowed to write events.
pub fn check_write_admission(store: &Store, pubkey: &Pubkey) -> AdmissionResult {
    match store.get_actor(pubkey) {
        Ok(Some(actor)) => {
            if actor.global_role == GlobalRole::Guest {
                AdmissionResult::Deny("guests cannot write".into())
            } else {
                AdmissionResult::Allow
            }
        }
        Ok(None) => AdmissionResult::Deny("unknown actor".into()),
        Err(e) => AdmissionResult::Deny(format!("store error: {e}")),
    }
}

/// Check if a pubkey can read content with the given visibility.
pub fn check_read_admission(
    store: &Store,
    pubkey: &Pubkey,
    visibility: Visibility,
    group_id: Option<&str>,
) -> AdmissionResult {
    match visibility {
        Visibility::Public => AdmissionResult::Allow,
        Visibility::Internal => AdmissionResult::Deny("internal only".into()),
        Visibility::Group => {
            let Some(gid) = group_id else {
                return AdmissionResult::Deny("group visibility requires group_id".into());
            };
            match store.get_group(gid) {
                Ok(Some(group)) => {
                    if group.members.iter().any(|m| m.pubkey == *pubkey) {
                        AdmissionResult::Allow
                    } else {
                        AdmissionResult::Deny("not a group member".into())
                    }
                }
                Ok(None) => AdmissionResult::Deny("group not found".into()),
                Err(e) => AdmissionResult::Deny(format!("store error: {e}")),
            }
        }
    }
}
