//! Management extension: actors, groups, registration, dashboard operations.

mod actors;
mod dashboard;
mod groups;
mod registration;
pub mod types;

use nostr_sdk::{PublicKey, ToBech32};
use nostrbox_core::GlobalRole;
use nostrbox_store::Store;
use tracing::info;

use types::{ErrorCode, OperationRequest, OperationResponse};

/// Resolved caller role for operation access control.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CallerRole {
    Owner,
    Admin,
    Member,
    Anonymous,
}

impl CallerRole {
    pub fn is_admin(&self) -> bool {
        matches!(self, CallerRole::Owner | CallerRole::Admin)
    }
}

/// Compute bech32 npub from hex pubkey, returning empty string on failure.
pub fn compute_npub(hex_pubkey: &str) -> String {
    PublicKey::parse(hex_pubkey)
        .ok()
        .and_then(|pk| pk.to_bech32().ok())
        .unwrap_or_default()
}

pub fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Handles management operations by dispatching to the store/core.
pub struct ManagementHandler<'a> {
    store: &'a Store,
    /// Server identity keys for signing events. None = no event publishing.
    keys: Option<&'a nostr_sdk::Keys>,
}

impl<'a> ManagementHandler<'a> {
    pub fn new(store: &'a Store) -> Self {
        Self {
            store,
            keys: None,
        }
    }

    pub fn with_keys(store: &'a Store, keys: &'a nostr_sdk::Keys) -> Self {
        Self {
            store,
            keys: Some(keys),
        }
    }

    /// Publish a signed event to the store's events table.
    fn publish_event(&self, builder: nostr_sdk::EventBuilder) {
        let Some(keys) = self.keys else { return };
        match nostrbox_nostr::sign_event(builder, keys) {
            Ok(event) => {
                let tags_json = serde_json::to_string(&event.tags).unwrap_or_default();
                if let Err(e) = self.store.store_event(
                    &event.id.to_hex(),
                    &event.pubkey.to_hex(),
                    event.kind.as_u16() as u64,
                    event.created_at.as_u64(),
                    &event.content,
                    &tags_json,
                    &event.sig.to_string(),
                ) {
                    tracing::warn!("failed to store published event: {e}");
                }
                info!(kind = event.kind.as_u16(), id = %event.id, "published event");
            }
            Err(e) => {
                tracing::warn!("failed to sign event: {e}");
            }
        }
    }

    /// Dispatch a management operation request to the appropriate handler.
    pub fn handle(&self, req: &OperationRequest) -> OperationResponse {
        info!(op = %req.op, auth = ?req.auth_source, "handling operation");
        match req.op.as_str() {
            "registration.submit" => self.registration_submit(req),
            "registration.list" => self.registration_list(req),
            "registration.get" => self.registration_get(req),
            "registration.approve" => self.registration_approve(req),
            "registration.deny" => self.registration_deny(req),
            "registration.delete" => self.registration_delete(req),
            "actor.list" => self.actor_list(req),
            "actor.get" => self.actor_get(req),
            "actor.delete" => self.actor_delete(req),
            "actor.detail" => self.actor_detail(req),
            "group.list" => self.group_list(req),
            "group.get" => self.group_get(req),
            "group.put" => self.group_put(req),
            "group.add_member" => self.group_add_member(req),
            "group.delete" => self.group_delete(req),
            "group.remove_member" => self.group_remove_member(req),
            "dashboard.get" => self.dashboard_get(req),
            _ => OperationResponse::error_with_code(
                ErrorCode::UnknownOperation,
                format!("unknown operation: {}", req.op),
            ),
        }
    }

    /// Check if caller has admin/owner role.
    pub(crate) fn require_admin(&self, req: &OperationRequest) -> Option<OperationResponse> {
        let Some(caller) = &req.caller else {
            return Some(OperationResponse::error_with_code(
                ErrorCode::Unauthorized,
                "authentication required",
            ));
        };
        match self.store.get_actor(caller) {
            Ok(Some(actor)) if actor.global_role.can_manage() => None,
            Ok(Some(_)) => Some(OperationResponse::error_with_code(
                ErrorCode::Forbidden,
                "admin or owner role required",
            )),
            Ok(None) => Some(OperationResponse::error_with_code(
                ErrorCode::Forbidden,
                "caller not found",
            )),
            Err(e) => Some(OperationResponse::error(e.to_string())),
        }
    }

    /// Require an authenticated caller.
    pub(crate) fn require_authenticated(&self, req: &OperationRequest) -> Result<String, OperationResponse> {
        req.caller.clone().ok_or_else(|| {
            OperationResponse::error_with_code(ErrorCode::Unauthorized, "authentication required")
        })
    }

    /// Resolve the caller's effective role from the actor store.
    pub(crate) fn resolve_caller_role(&self, req: &OperationRequest) -> CallerRole {
        let Some(caller) = &req.caller else {
            return CallerRole::Anonymous;
        };
        match self.store.get_actor(caller) {
            Ok(Some(actor)) => match actor.global_role {
                GlobalRole::Owner => CallerRole::Owner,
                GlobalRole::Admin => CallerRole::Admin,
                GlobalRole::Member | GlobalRole::Guest => CallerRole::Member,
            },
            _ => CallerRole::Anonymous,
        }
    }

    /// Check that the caller is either the target pubkey (self-access) or an admin.
    pub(crate) fn require_self_or_admin(
        &self,
        req: &OperationRequest,
        target_pubkey: &str,
    ) -> Option<OperationResponse> {
        let Some(caller) = &req.caller else {
            return Some(OperationResponse::error_with_code(
                ErrorCode::Unauthorized,
                "authentication required",
            ));
        };
        if caller == target_pubkey {
            return None; // self-access
        }
        match self.store.get_actor(caller) {
            Ok(Some(actor)) if actor.global_role.can_manage() => None,
            Ok(Some(_)) => Some(OperationResponse::error_with_code(
                ErrorCode::Forbidden,
                "can only access own record, or admin role required",
            )),
            Ok(None) => Some(OperationResponse::error_with_code(
                ErrorCode::Forbidden,
                "caller not found",
            )),
            Err(e) => Some(OperationResponse::error(e.to_string())),
        }
    }
}
