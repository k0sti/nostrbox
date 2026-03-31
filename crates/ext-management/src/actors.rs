//! Actor operation handlers.

use crate::types::{ErrorCode, OperationRequest, OperationResponse};
use crate::ManagementHandler;

impl ManagementHandler<'_> {
    pub(crate) fn actor_list(&self, req: &OperationRequest) -> OperationResponse {
        if let Some(err) = self.require_admin(req) {
            return err;
        }
        match self.store.list_actors() {
            Ok(actors) => OperationResponse::success(serde_json::to_value(actors).unwrap()),
            Err(e) => OperationResponse::error(e.to_string()),
        }
    }

    pub(crate) fn actor_get(&self, req: &OperationRequest) -> OperationResponse {
        let Some(pubkey) = req.params.get("pubkey").and_then(|v| v.as_str()) else {
            return OperationResponse::error_with_code(
                ErrorCode::ValidationError,
                "missing param: pubkey",
            );
        };
        if let Some(err) = self.require_self_or_admin(req, pubkey) {
            return err;
        }
        match self.store.get_actor(pubkey) {
            Ok(Some(actor)) => OperationResponse::success(serde_json::to_value(actor).unwrap()),
            Ok(None) => {
                OperationResponse::error_with_code(ErrorCode::NotFound, "actor not found")
            }
            Err(e) => OperationResponse::error(e.to_string()),
        }
    }

    pub(crate) fn actor_detail(&self, req: &OperationRequest) -> OperationResponse {
        let Some(pubkey) = req.params.get("pubkey").and_then(|v| v.as_str()) else {
            return OperationResponse::error_with_code(
                ErrorCode::ValidationError,
                "missing param: pubkey",
            );
        };
        if let Some(err) = self.require_self_or_admin(req, pubkey) {
            return err;
        }
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

    pub(crate) fn actor_delete(&self, req: &OperationRequest) -> OperationResponse {
        if let Some(err) = self.require_admin(req) {
            return err;
        }
        let Some(pubkey) = req.params.get("pubkey").and_then(|v| v.as_str()) else {
            return OperationResponse::error_with_code(
                ErrorCode::ValidationError,
                "missing param: pubkey",
            );
        };
        match self.store.delete_actor(pubkey) {
            Ok(true) => OperationResponse::success(serde_json::json!({"deleted": true})),
            Ok(false) => OperationResponse::error_with_code(
                ErrorCode::NotFound,
                "actor not found",
            ),
            Err(e) => OperationResponse::error(e.to_string()),
        }
    }
}
