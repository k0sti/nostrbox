use nostrbox_core::{GlobalRole, Group, GroupMember, GroupRole, RegistrationStatus};
use nostrbox_store::Store;
use tracing::info;

use crate::types::{DashboardSummary, OperationRequest, OperationResponse};

/// Handles ContextVM operations by dispatching to the store/core.
///
/// TODO: Replace with rust-contextvm-sdk handler trait once available.
pub struct OperationHandler<'a> {
    store: &'a Store,
}

impl<'a> OperationHandler<'a> {
    pub fn new(store: &'a Store) -> Self {
        Self { store }
    }

    /// Dispatch an operation request to the appropriate handler.
    pub fn handle(&self, req: &OperationRequest) -> OperationResponse {
        info!(op = %req.op, "handling operation");
        match req.op.as_str() {
            "registration.list" => self.registration_list(),
            "registration.get" => self.registration_get(req),
            "registration.approve" => self.registration_approve(req),
            "actor.list" => self.actor_list(),
            "actor.get" => self.actor_get(req),
            "group.list" => self.group_list(),
            "group.get" => self.group_get(req),
            "group.put" => self.group_put(req),
            "group.add_member" => self.group_add_member(req),
            "group.remove_member" => self.group_remove_member(req),
            "dashboard.get" => self.dashboard_get(),
            _ => OperationResponse::error(format!("unknown operation: {}", req.op)),
        }
    }

    // ── Registration operations ────────────────────────────────────

    fn registration_list(&self) -> OperationResponse {
        match self.store.list_registrations() {
            Ok(regs) => OperationResponse::success(serde_json::to_value(regs).unwrap()),
            Err(e) => OperationResponse::error(e.to_string()),
        }
    }

    fn registration_get(&self, req: &OperationRequest) -> OperationResponse {
        let pubkey = req.params.get("pubkey").and_then(|v| v.as_str());
        let Some(pubkey) = pubkey else {
            return OperationResponse::error("missing param: pubkey");
        };
        match self.store.get_registration(pubkey) {
            Ok(Some(reg)) => OperationResponse::success(serde_json::to_value(reg).unwrap()),
            Ok(None) => OperationResponse::error("not found"),
            Err(e) => OperationResponse::error(e.to_string()),
        }
    }

    fn registration_approve(&self, req: &OperationRequest) -> OperationResponse {
        // TODO: Check caller has admin/owner role
        let pubkey = req.params.get("pubkey").and_then(|v| v.as_str());
        let Some(pubkey) = pubkey else {
            return OperationResponse::error("missing param: pubkey");
        };
        match self.store.get_registration(pubkey) {
            Ok(Some(mut reg)) => {
                reg.status = RegistrationStatus::Approved;
                if let Err(e) = self.store.upsert_registration(&reg) {
                    return OperationResponse::error(e.to_string());
                }
                // Create actor with member role
                let actor = nostrbox_core::Actor {
                    pubkey: reg.pubkey.clone(),
                    kind: nostrbox_core::ActorKind::Human,
                    global_role: GlobalRole::Member,
                    display_name: None,
                    groups: vec![],
                };
                if let Err(e) = self.store.upsert_actor(&actor) {
                    return OperationResponse::error(e.to_string());
                }
                // TODO: Publish Nostr replaceable event for role assignment
                OperationResponse::success(serde_json::to_value(reg).unwrap())
            }
            Ok(None) => OperationResponse::error("registration not found"),
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
        let pubkey = req.params.get("pubkey").and_then(|v| v.as_str());
        let Some(pubkey) = pubkey else {
            return OperationResponse::error("missing param: pubkey");
        };
        match self.store.get_actor(pubkey) {
            Ok(Some(actor)) => OperationResponse::success(serde_json::to_value(actor).unwrap()),
            Ok(None) => OperationResponse::error("not found"),
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
        let group_id = req.params.get("group_id").and_then(|v| v.as_str());
        let Some(group_id) = group_id else {
            return OperationResponse::error("missing param: group_id");
        };
        match self.store.get_group(group_id) {
            Ok(Some(group)) => OperationResponse::success(serde_json::to_value(group).unwrap()),
            Ok(None) => OperationResponse::error("not found"),
            Err(e) => OperationResponse::error(e.to_string()),
        }
    }

    fn group_put(&self, req: &OperationRequest) -> OperationResponse {
        // TODO: Check caller has admin/owner role
        let group: Result<Group, _> = serde_json::from_value(req.params.clone());
        match group {
            Ok(g) => match self.store.upsert_group(&g) {
                Ok(()) => OperationResponse::success(serde_json::to_value(&g).unwrap()),
                Err(e) => OperationResponse::error(e.to_string()),
            },
            Err(e) => OperationResponse::error(format!("invalid group payload: {e}")),
        }
    }

    fn group_add_member(&self, req: &OperationRequest) -> OperationResponse {
        // TODO: Check caller has admin/owner role on this group
        let group_id = req.params.get("group_id").and_then(|v| v.as_str());
        let pubkey = req.params.get("pubkey").and_then(|v| v.as_str());
        let role_str = req
            .params
            .get("role")
            .and_then(|v| v.as_str())
            .unwrap_or("member");
        let (Some(group_id), Some(pubkey)) = (group_id, pubkey) else {
            return OperationResponse::error("missing param: group_id or pubkey");
        };
        let role: GroupRole =
            serde_json::from_value(serde_json::Value::String(role_str.to_string()))
                .unwrap_or(GroupRole::Member);
        let member = GroupMember {
            pubkey: pubkey.to_string(),
            role,
        };
        match self.store.add_group_member(group_id, &member) {
            Ok(()) => OperationResponse::success(serde_json::json!({"ok": true})),
            Err(e) => OperationResponse::error(e.to_string()),
        }
    }

    fn group_remove_member(&self, req: &OperationRequest) -> OperationResponse {
        // TODO: Check caller has admin/owner role on this group
        let group_id = req.params.get("group_id").and_then(|v| v.as_str());
        let pubkey = req.params.get("pubkey").and_then(|v| v.as_str());
        let (Some(group_id), Some(pubkey)) = (group_id, pubkey) else {
            return OperationResponse::error("missing param: group_id or pubkey");
        };
        match self.store.remove_group_member(group_id, pubkey) {
            Ok(()) => OperationResponse::success(serde_json::json!({"ok": true})),
            Err(e) => OperationResponse::error(e.to_string()),
        }
    }

    // ── Dashboard ──────────────────────────────────────────────────

    fn dashboard_get(&self) -> OperationResponse {
        let summary = DashboardSummary {
            pending_registrations: self.store.count_pending_registrations().unwrap_or(0),
            total_actors: self.store.count_actors().unwrap_or(0),
            total_groups: self.store.count_groups().unwrap_or(0),
        };
        OperationResponse::success(serde_json::to_value(summary).unwrap())
    }
}
