//! Registration operation handlers.

use nostrbox_core::{ActorStatus, GlobalRole, RegistrationStatus};

use crate::types::{ErrorCode, OperationRequest, OperationResponse};
use crate::{ManagementHandler, compute_npub};

impl ManagementHandler<'_> {
    pub(crate) fn registration_submit(&self, req: &OperationRequest) -> OperationResponse {
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
        let now = crate::now_secs();
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

    pub(crate) fn registration_list(&self, req: &OperationRequest) -> OperationResponse {
        if let Some(err) = self.require_admin(req) {
            return err;
        }
        match self.store.list_registrations() {
            Ok(regs) => OperationResponse::success(serde_json::to_value(regs).unwrap()),
            Err(e) => OperationResponse::error(e.to_string()),
        }
    }

    pub(crate) fn registration_get(&self, req: &OperationRequest) -> OperationResponse {
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

    pub(crate) fn registration_approve(&self, req: &OperationRequest) -> OperationResponse {
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
                let now = crate::now_secs();
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
                self.publish_event(nostrbox_nostr::build_role_event(&reg.pubkey, "member"));

                OperationResponse::success(serde_json::to_value(reg).unwrap())
            }
            Ok(None) => OperationResponse::error_with_code(
                ErrorCode::NotFound,
                "registration not found",
            ),
            Err(e) => OperationResponse::error(e.to_string()),
        }
    }

    pub(crate) fn registration_delete(&self, req: &OperationRequest) -> OperationResponse {
        if let Some(err) = self.require_admin(req) {
            return err;
        }
        let Some(pubkey) = req.params.get("pubkey").and_then(|v| v.as_str()) else {
            return OperationResponse::error_with_code(
                ErrorCode::ValidationError,
                "missing param: pubkey",
            );
        };
        match self.store.delete_registration(pubkey) {
            Ok(true) => OperationResponse::success(serde_json::json!({"deleted": true})),
            Ok(false) => OperationResponse::error_with_code(
                ErrorCode::NotFound,
                "registration not found",
            ),
            Err(e) => OperationResponse::error(e.to_string()),
        }
    }

    pub(crate) fn registration_deny(&self, req: &OperationRequest) -> OperationResponse {
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
}
