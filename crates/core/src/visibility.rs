use serde::{Deserialize, Serialize};

/// Visibility level, aligned with Nomen terminology.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Visibility {
    /// Accessible to anyone, including guests.
    Public,
    /// Accessible to members of a named group.
    Group,
    /// Server/internal use only.
    Internal,
}

/// Scope is the durable boundary identifier paired with visibility.
///
/// - `public` → empty scope
/// - `group` → named group id
/// - `internal` → internal/system scope
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Scope {
    pub visibility: Visibility,
    /// The group id when visibility is `Group`, empty otherwise.
    pub group_id: Option<String>,
}

impl Scope {
    pub fn public() -> Self {
        Self {
            visibility: Visibility::Public,
            group_id: None,
        }
    }

    pub fn group(group_id: impl Into<String>) -> Self {
        Self {
            visibility: Visibility::Group,
            group_id: Some(group_id.into()),
        }
    }

    pub fn internal() -> Self {
        Self {
            visibility: Visibility::Internal,
            group_id: None,
        }
    }
}
