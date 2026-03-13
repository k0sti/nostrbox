use serde::{Deserialize, Serialize};

use crate::{GroupId, Pubkey, Visibility};

/// Role within a specific group.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum GroupRole {
    Member,
    Admin,
    Owner,
}

/// Join policy for a group.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JoinPolicy {
    Open,
    Request,
    InviteOnly,
    Closed,
}

impl Default for JoinPolicy {
    fn default() -> Self {
        Self::Request
    }
}

/// Group status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum GroupStatus {
    Active,
    Frozen,
    Archived,
}

impl Default for GroupStatus {
    fn default() -> Self {
        Self::Active
    }
}

/// A named group in Nostrbox.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Group {
    pub group_id: GroupId,
    pub name: String,
    #[serde(default)]
    pub description: String,
    pub visibility: Visibility,
    #[serde(default)]
    pub slug: Option<String>,
    #[serde(default)]
    pub join_policy: JoinPolicy,
    #[serde(default)]
    pub status: GroupStatus,
    #[serde(default)]
    pub members: Vec<GroupMember>,
    #[serde(default)]
    pub created_at: u64,
    #[serde(default)]
    pub updated_at: u64,
}

/// A member entry within a group.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroupMember {
    pub pubkey: Pubkey,
    pub role: GroupRole,
}
