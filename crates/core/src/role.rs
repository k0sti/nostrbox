use serde::{Deserialize, Serialize};

/// Global role — an actor's baseline authority in Nostrbox.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum GlobalRole {
    /// Public visibility access only.
    Guest,
    /// Normal approved user.
    Member,
    /// Can manage users/groups.
    Admin,
    /// Full authority.
    Owner,
}

impl GlobalRole {
    pub fn can_manage(&self) -> bool {
        matches!(self, GlobalRole::Admin | GlobalRole::Owner)
    }
}
