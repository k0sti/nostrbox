//! Read filtering using authed_pubkey directly from session.

use nostrbox_store::StorePool;

use crate::admission::get_role;
use crate::config::RelayAccessConfig;
use crate::session::StoredFilter;

/// Check if a query (REQ) should be allowed based on the authed pubkey's role.
///
/// Returns Ok(()) if allowed, Err(reason) if denied.
pub fn check_query_admission(
    pool: &StorePool,
    authed_pubkey: Option<&str>,
    filters: &[StoredFilter],
    access_config: &RelayAccessConfig,
) -> Result<(), String> {
    let Some(pubkey) = authed_pubkey else {
        return Err("auth-required: authentication required".into());
    };

    let store = pool.get().map_err(|e| format!("error: store: {e}"))?;
    let role = get_role(&store, pubkey);
    let role_access = access_config.role_access(role);

    // If role has read_all, allow everything
    if role_access.read_all {
        return Ok(());
    }

    // Check if all filters only query allowed kinds
    for filter in filters {
        if filter.kinds.is_empty() {
            // No kind filter means requesting all kinds — denied for limited roles
            return Err(format!(
                "restricted: {:?}s have limited read access",
                role
            ));
        }
        for kind in &filter.kinds {
            if !role_access.can_read(*kind as u16) {
                return Err(format!(
                    "restricted: {:?}s cannot read kind {}",
                    role, kind
                ));
            }
        }
    }

    Ok(())
}

/// Query events from the store matching the given filters.
/// Returns a vec of (event_json, matched by filter index).
pub fn query_stored_events(
    pool: &StorePool,
    filters: &[StoredFilter],
) -> Result<Vec<serde_json::Value>, String> {
    let store = pool.get().map_err(|e| format!("store error: {e}"))?;
    let mut all_events = Vec::new();

    for filter in filters {
        let ids: Vec<String> = filter.ids.clone();
        let authors: Vec<String> = filter.authors.clone();
        let kinds: Vec<u64> = filter.kinds.clone();

        match store.query_events(&ids, &authors, &kinds, filter.since, filter.until, filter.limit) {
            Ok(events) => {
                for event in events {
                    // Deduplicate by event id
                    let event_id = event.get("id").and_then(|v| v.as_str()).unwrap_or("");
                    if !all_events
                        .iter()
                        .any(|e: &serde_json::Value| e.get("id").and_then(|v| v.as_str()).unwrap_or("") == event_id)
                    {
                        all_events.push(event);
                    }
                }
            }
            Err(e) => {
                tracing::warn!("query_events error: {e}");
            }
        }
    }

    Ok(all_events)
}
