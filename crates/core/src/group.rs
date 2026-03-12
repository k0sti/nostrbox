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

/// A named group in Nostrbox.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Group {
    pub group_id: GroupId,
    pub name: String,
    pub description: String,
    pub visibility: Visibility,
    pub members: Vec<GroupMember>,
}

/// A member entry within a group.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroupMember {
    pub pubkey: Pubkey,
    pub role: GroupRole,
}
