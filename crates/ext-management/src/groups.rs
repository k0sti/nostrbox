//! Group operation handlers.

use nostrbox_core::{Group, GroupMember, GroupRole, Visibility};

use crate::types::{ErrorCode, OperationRequest, OperationResponse};
use crate::ManagementHandler;

impl ManagementHandler<'_> {
    pub(crate) fn group_list(&self, req: &OperationRequest) -> OperationResponse {
        match self.store.list_groups() {
            Ok(groups) => {
                let role = self.resolve_caller_role(req);
                let filtered: Vec<Group> = if role.is_admin() {
                    groups
                } else {
                    let caller = req.caller.as_deref().unwrap_or_default();
                    groups
                        .into_iter()
                        .filter(|g| {
                            g.visibility == Visibility::Public
                                || g.members.iter().any(|m| m.pubkey == caller)
                        })
                        .collect()
                };
                OperationResponse::success(serde_json::to_value(filtered).unwrap())
            }
            Err(e) => OperationResponse::error(e.to_string()),
        }
    }

    pub(crate) fn group_get(&self, req: &OperationRequest) -> OperationResponse {
        let Some(group_id) = req.params.get("group_id").and_then(|v| v.as_str()) else {
            return OperationResponse::error_with_code(
                ErrorCode::ValidationError,
                "missing param: group_id",
            );
        };
        match self.store.get_group(group_id) {
            Ok(Some(group)) => {
                if group.visibility == Visibility::Public {
                    return OperationResponse::success(
                        serde_json::to_value(&group).unwrap(),
                    );
                }
                let role = self.resolve_caller_role(req);
                if role.is_admin() {
                    return OperationResponse::success(
                        serde_json::to_value(&group).unwrap(),
                    );
                }
                let caller = req.caller.as_deref().unwrap_or_default();
                if group.members.iter().any(|m| m.pubkey == caller) {
                    OperationResponse::success(serde_json::to_value(&group).unwrap())
                } else {
                    OperationResponse::error_with_code(
                        ErrorCode::Forbidden,
                        "access denied",
                    )
                }
            }
            Ok(None) => {
                OperationResponse::error_with_code(ErrorCode::NotFound, "group not found")
            }
            Err(e) => OperationResponse::error(e.to_string()),
        }
    }

    pub(crate) fn group_delete(&self, req: &OperationRequest) -> OperationResponse {
        if let Some(err) = self.require_admin(req) {
            return err;
        }
        let Some(group_id) = req.params.get("group_id").and_then(|v| v.as_str()) else {
            return OperationResponse::error_with_code(
                ErrorCode::ValidationError,
                "missing param: group_id",
            );
        };
        match self.store.delete_group(group_id) {
            Ok(true) => OperationResponse::success(serde_json::json!({"deleted": true})),
            Ok(false) => OperationResponse::error_with_code(
                ErrorCode::NotFound,
                "group not found",
            ),
            Err(e) => OperationResponse::error(e.to_string()),
        }
    }

    pub(crate) fn group_put(&self, req: &OperationRequest) -> OperationResponse {
        if let Some(err) = self.require_admin(req) {
            return err;
        }
        let group: Result<Group, _> = serde_json::from_value(req.params.clone());
        match group {
            Ok(g) => match self.store.upsert_group(&g) {
                Ok(()) => {
                    let vis = serde_json::to_value(&g.visibility)
                        .ok()
                        .and_then(|v| v.as_str().map(String::from))
                        .unwrap_or_default();
                    self.publish_event(nostrbox_nostr::build_group_event(
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

    pub(crate) fn group_add_member(&self, req: &OperationRequest) -> OperationResponse {
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
                self.publish_event(nostrbox_nostr::build_membership_event(group_id, pubkey, role_str));
                OperationResponse::success(serde_json::json!({"ok": true}))
            }
            Err(e) => OperationResponse::error(e.to_string()),
        }
    }

    pub(crate) fn group_remove_member(&self, req: &OperationRequest) -> OperationResponse {
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
}
