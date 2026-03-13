use nostr_sdk::{PublicKey, ToBech32};
use nostrbox_core::{ActorStatus, GlobalRole, Group, GroupMember, GroupRole, RegistrationStatus};
use nostrbox_store::Store;
use tracing::info;

use crate::events;
use crate::types::{ErrorCode, OperationRequest, OperationResponse};

/// Compute bech32 npub from hex pubkey, returning empty string on failure.
fn compute_npub(hex_pubkey: &str) -> String {
    PublicKey::parse(hex_pubkey)
        .ok()
        .and_then(|pk| pk.to_bech32().ok())
        .unwrap_or_default()
}

/// Handles ContextVM operations by dispatching to the store/core.
pub struct OperationHandler<'a> {
    store: &'a Store,
    /// Server identity keys for signing events. None = no event publishing.
    keys: Option<&'a nostr_sdk::Keys>,
}

impl<'a> OperationHandler<'a> {
    pub fn new(store: &'a Store) -> Self {
        Self { store, keys: None }
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
        match events::sign_event(builder, keys) {
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

    /// Dispatch an operation request to the appropriate handler.
    pub fn handle(&self, req: &OperationRequest) -> OperationResponse {
        info!(op = %req.op, "handling operation");
        match req.op.as_str() {
            "registration.submit" => self.registration_submit(req),
            "registration.list" => self.registration_list(),
            "registration.get" => self.registration_get(req),
            "registration.approve" => self.registration_approve(req),
            "registration.deny" => self.registration_deny(req),
            "actor.list" => self.actor_list(),
            "actor.get" => self.actor_get(req),
            "actor.detail" => self.actor_detail(req),
            "group.list" => self.group_list(),
            "group.get" => self.group_get(req),
            "group.put" => self.group_put(req),
            "group.add_member" => self.group_add_member(req),
            "group.remove_member" => self.group_remove_member(req),
            "dashboard.get" => self.dashboard_get(),
            _ => OperationResponse::error_with_code(
                ErrorCode::UnknownOperation,
                format!("unknown operation: {}", req.op),
            ),
        }
    }

    /// Check if caller has admin/owner role. Returns None if authorized, or an error response.
    fn require_admin(&self, req: &OperationRequest) -> Option<OperationResponse> {
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

    // ── Registration operations ────────────────────────────────────

    fn registration_submit(&self, req: &OperationRequest) -> OperationResponse {
        let Some(pubkey) = req
            .caller
            .as_deref()
            .or_else(|| req.params.get("pubkey").and_then(|v| v.as_str()))
        else {
            return OperationResponse::error_with_code(
                ErrorCode::ValidationError,
                "missing caller or pubkey",
            );
        };
        let message = req
            .params
            .get("message")
            .and_then(|v| v.as_str())
            .map(String::from);
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let reg = nostrbox_core::Registration {
            pubkey: pubkey.to_string(),
            message,
            timestamp: now,
            status: nostrbox_core::RegistrationStatus::Pending,
        };
        match self.store.upsert_registration(&reg) {
            Ok(()) => OperationResponse::success(serde_json::to_value(reg).unwrap()),
            Err(e) => OperationResponse::error(e.to_string()),
        }
    }

    fn registration_list(&self) -> OperationResponse {
        match self.store.list_registrations() {
            Ok(regs) => OperationResponse::success(serde_json::to_value(regs).unwrap()),
            Err(e) => OperationResponse::error(e.to_string()),
        }
    }

    fn registration_get(&self, req: &OperationRequest) -> OperationResponse {
        let Some(pubkey) = req.params.get("pubkey").and_then(|v| v.as_str()) else {
            return OperationResponse::error_with_code(
                ErrorCode::ValidationError,
                "missing param: pubkey",
            );
        };
        match self.store.get_registration(pubkey) {
            Ok(Some(reg)) => OperationResponse::success(serde_json::to_value(reg).unwrap()),
            Ok(None) => OperationResponse::error_with_code(
                ErrorCode::NotFound,
                "registration not found",
            ),
            Err(e) => OperationResponse::error(e.to_string()),
        }
    }

    fn registration_approve(&self, req: &OperationRequest) -> OperationResponse {
        if let Some(err) = self.require_admin(req) {
            return err;
        }
        let Some(pubkey) = req.params.get("pubkey").and_then(|v| v.as_str()) else {
            return OperationResponse::error_with_code(
                ErrorCode::ValidationError,
                "missing param: pubkey",
            );
        };
        match self.store.get_registration(pubkey) {
            Ok(Some(mut reg)) => {
                reg.status = RegistrationStatus::Approved;
                if let Err(e) = self.store.upsert_registration(&reg) {
                    return OperationResponse::error(e.to_string());
                }
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_secs())
                    .unwrap_or(0);
                let actor = nostrbox_core::Actor {
                    pubkey: reg.pubkey.clone(),
                    npub: compute_npub(&reg.pubkey),
                    kind: nostrbox_core::ActorKind::Human,
                    global_role: GlobalRole::Member,
                    status: ActorStatus::Active,
                    display_name: None,
                    groups: vec![],
                    created_at: now,
                    updated_at: now,
                };
                if let Err(e) = self.store.upsert_actor(&actor) {
                    return OperationResponse::error(e.to_string());
                }

                // Publish role assignment event
                self.publish_event(events::build_role_event(&reg.pubkey, "member"));

                OperationResponse::success(serde_json::to_value(reg).unwrap())
            }
            Ok(None) => OperationResponse::error_with_code(
                ErrorCode::NotFound,
                "registration not found",
            ),
            Err(e) => OperationResponse::error(e.to_string()),
        }
    }

    fn registration_deny(&self, req: &OperationRequest) -> OperationResponse {
        if let Some(err) = self.require_admin(req) {
            return err;
        }
        let Some(pubkey) = req.params.get("pubkey").and_then(|v| v.as_str()) else {
            return OperationResponse::error_with_code(
                ErrorCode::ValidationError,
                "missing param: pubkey",
            );
        };
        match self.store.get_registration(pubkey) {
            Ok(Some(mut reg)) => {
                reg.status = RegistrationStatus::Denied;
                if let Err(e) = self.store.upsert_registration(&reg) {
                    return OperationResponse::error(e.to_string());
                }
                OperationResponse::success(serde_json::to_value(reg).unwrap())
            }
            Ok(None) => OperationResponse::error_with_code(
                ErrorCode::NotFound,
                "registration not found",
            ),
            Err(e) => OperationResponse::error(e.to_string()),
        }
    }

    // ── Actor operations ───────────────────────────────────────────

    fn actor_list(&self) -> OperationResponse {
        match self.store.list_actors() {
            Ok(actors) => OperationResponse::success(serde_json::to_value(actors).unwrap()),
            Err(e) => OperationResponse::error(e.to_string()),
        }
    }

    fn actor_get(&self, req: &OperationRequest) -> OperationResponse {
        let Some(pubkey) = req.params.get("pubkey").and_then(|v| v.as_str()) else {
            return OperationResponse::error_with_code(
                ErrorCode::ValidationError,
                "missing param: pubkey",
            );
        };
        match self.store.get_actor(pubkey) {
            Ok(Some(actor)) => OperationResponse::success(serde_json::to_value(actor).unwrap()),
            Ok(None) => {
                OperationResponse::error_with_code(ErrorCode::NotFound, "actor not found")
            }
            Err(e) => OperationResponse::error(e.to_string()),
        }
    }

    fn actor_detail(&self, req: &OperationRequest) -> OperationResponse {
        let Some(pubkey) = req.params.get("pubkey").and_then(|v| v.as_str()) else {
            return OperationResponse::error_with_code(
                ErrorCode::ValidationError,
                "missing param: pubkey",
            );
        };
        match self.store.get_actor_detail(pubkey) {
            Ok(Some(detail)) => {
                OperationResponse::success(serde_json::to_value(detail).unwrap())
            }
            Ok(None) => {
                OperationResponse::error_with_code(ErrorCode::NotFound, "actor not found")
            }
            Err(e) => OperationResponse::error(e.to_string()),
        }
    }

    // ── Group operations ───────────────────────────────────────────

    fn group_list(&self) -> OperationResponse {
        match self.store.list_groups() {
            Ok(groups) => OperationResponse::success(serde_json::to_value(groups).unwrap()),
            Err(e) => OperationResponse::error(e.to_string()),
        }
    }

    fn group_get(&self, req: &OperationRequest) -> OperationResponse {
        let Some(group_id) = req.params.get("group_id").and_then(|v| v.as_str()) else {
            return OperationResponse::error_with_code(
                ErrorCode::ValidationError,
                "missing param: group_id",
            );
        };
        match self.store.get_group(group_id) {
            Ok(Some(group)) => OperationResponse::success(serde_json::to_value(group).unwrap()),
            Ok(None) => {
                OperationResponse::error_with_code(ErrorCode::NotFound, "group not found")
            }
            Err(e) => OperationResponse::error(e.to_string()),
        }
    }

    fn group_put(&self, req: &OperationRequest) -> OperationResponse {
        if let Some(err) = self.require_admin(req) {
            return err;
        }
        let group: Result<Group, _> = serde_json::from_value(req.params.clone());
        match group {
            Ok(g) => match self.store.upsert_group(&g) {
                Ok(()) => {
                    // Publish group definition event
                    let vis = serde_json::to_value(&g.visibility)
                        .ok()
                        .and_then(|v| v.as_str().map(String::from))
                        .unwrap_or_default();
                    self.publish_event(events::build_group_event(
                        &g.group_id,
                        &g.name,
                        &g.description,
                        &vis,
                    ));
                    OperationResponse::success(serde_json::to_value(&g).unwrap())
                }
                Err(e) => OperationResponse::error(e.to_string()),
            },
            Err(e) => OperationResponse::error_with_code(
                ErrorCode::ValidationError,
                format!("invalid group payload: {e}"),
            ),
        }
    }

    fn group_add_member(&self, req: &OperationRequest) -> OperationResponse {
        if let Some(err) = self.require_admin(req) {
            return err;
        }
        let group_id = req.params.get("group_id").and_then(|v| v.as_str());
        let pubkey = req.params.get("pubkey").and_then(|v| v.as_str());
        let role_str = req
            .params
            .get("role")
            .and_then(|v| v.as_str())
            .unwrap_or("member");
        let (Some(group_id), Some(pubkey)) = (group_id, pubkey) else {
            return OperationResponse::error_with_code(
                ErrorCode::ValidationError,
                "missing param: group_id or pubkey",
            );
        };
        let role: GroupRole =
            serde_json::from_value(serde_json::Value::String(role_str.to_string()))
                .unwrap_or(GroupRole::Member);
        let member = GroupMember {
            pubkey: pubkey.to_string(),
            role,
        };
        match self.store.add_group_member(group_id, &member) {
            Ok(()) => {
                // Publish membership event
                self.publish_event(events::build_membership_event(group_id, pubkey, role_str));
                OperationResponse::success(serde_json::json!({"ok": true}))
            }
            Err(e) => OperationResponse::error(e.to_string()),
        }
    }

    fn group_remove_member(&self, req: &OperationRequest) -> OperationResponse {
        if let Some(err) = self.require_admin(req) {
            return err;
        }
        let group_id = req.params.get("group_id").and_then(|v| v.as_str());
        let pubkey = req.params.get("pubkey").and_then(|v| v.as_str());
        let (Some(group_id), Some(pubkey)) = (group_id, pubkey) else {
            return OperationResponse::error_with_code(
                ErrorCode::ValidationError,
                "missing param: group_id or pubkey",
            );
        };
        match self.store.remove_group_member(group_id, pubkey) {
            Ok(()) => OperationResponse::success(serde_json::json!({"ok": true})),
            Err(e) => OperationResponse::error(e.to_string()),
        }
    }

    // ── Dashboard ──────────────────────────────────────────────────

    fn dashboard_get(&self) -> OperationResponse {
        match self.store.get_dashboard_summary() {
            Ok(summary) => OperationResponse::success(serde_json::to_value(summary).unwrap()),
            Err(e) => OperationResponse::error(e.to_string()),
        }
    }
}
