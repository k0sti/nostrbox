//! Dashboard operation handler.

use crate::types::{OperationRequest, OperationResponse};
use crate::ManagementHandler;

impl ManagementHandler<'_> {
    pub(crate) fn dashboard_get(&self, req: &OperationRequest) -> OperationResponse {
        if let Some(err) = self.require_admin(req) {
            return err;
        }
        match self.store.get_dashboard_summary() {
            Ok(summary) => OperationResponse::success(serde_json::to_value(summary).unwrap()),
            Err(e) => OperationResponse::error(e.to_string()),
        }
    }
}
