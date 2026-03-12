use serde::{Deserialize, Serialize};

/// A ContextVM operation request.
///
/// TODO: Align with rust-contextvm-sdk types once available.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationRequest {
    /// Operation name, e.g. "registration.list", "actor.get"
    pub op: String,
    /// JSON payload
    #[serde(default)]
    pub params: serde_json::Value,
    /// Caller identity (pubkey), if authenticated
    pub caller: Option<String>,
}

/// A ContextVM operation response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationResponse {
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl OperationResponse {
    pub fn success(data: serde_json::Value) -> Self {
        Self {
            ok: true,
            data: Some(data),
            error: None,
        }
    }

    pub fn error(msg: impl Into<String>) -> Self {
        Self {
            ok: false,
            data: None,
            error: Some(msg.into()),
        }
    }
}

/// Dashboard summary data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardSummary {
    pub pending_registrations: u64,
    pub total_actors: u64,
    pub total_groups: u64,
}
