use serde::{Deserialize, Serialize};

/// How the caller's identity was verified.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum AuthSource {
    /// Not yet determined (default when deserialized from JSON).
    #[default]
    Unknown,
    /// NIP-98 HTTP Authorization header.
    Nip98,
    /// ContextVM Nostr relay transport (event signature).
    ContextVm,
    /// Local bypass (loopback address).
    LocalBypass,
}

/// A ContextVM operation request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationRequest {
    /// Operation name, e.g. "registration.list", "actor.get"
    pub op: String,
    /// JSON payload
    #[serde(default)]
    pub params: serde_json::Value,
    /// Caller identity (pubkey), if authenticated
    pub caller: Option<String>,
    /// How the caller's identity was verified. Set by the transport layer, not from JSON.
    #[serde(skip)]
    pub auth_source: AuthSource,
}

/// Structured error codes per Shared Spec.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCode {
    Unauthorized,
    Forbidden,
    NotFound,
    InvalidState,
    Conflict,
    ValidationError,
    Internal,
    UnknownOperation,
}

/// A ContextVM operation response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationResponse {
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_code: Option<String>,
}

impl OperationResponse {
    pub fn success(data: serde_json::Value) -> Self {
        Self {
            ok: true,
            data: Some(data),
            error: None,
            error_code: None,
        }
    }

    pub fn error(msg: impl Into<String>) -> Self {
        Self {
            ok: false,
            data: None,
            error: Some(msg.into()),
            error_code: None,
        }
    }

    pub fn error_with_code(code: ErrorCode, msg: impl Into<String>) -> Self {
        let code_str = serde_json::to_value(&code)
            .ok()
            .and_then(|v| v.as_str().map(String::from))
            .unwrap_or_else(|| "internal".into());
        Self {
            ok: false,
            data: None,
            error: Some(msg.into()),
            error_code: Some(code_str),
        }
    }
}
