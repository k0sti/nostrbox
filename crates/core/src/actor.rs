use serde::{Deserialize, Serialize};

use crate::{GlobalRole, Pubkey};

/// The kind of actor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ActorKind {
    Human,
    Agent,
    Service,
    System,
}

/// Actor status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ActorStatus {
    Active,
    Disabled,
    Banned,
    Restricted,
}

impl Default for ActorStatus {
    fn default() -> Self {
        Self::Active
    }
}

/// An actor in Nostrbox — any pubkey-based identity.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Actor {
    pub pubkey: Pubkey,
    pub npub: String,
    pub kind: ActorKind,
    pub global_role: GlobalRole,
    pub status: ActorStatus,
    /// Display name from kind-0 metadata, if known.
    pub display_name: Option<String>,
    /// Groups this actor belongs to (group ids).
    pub groups: Vec<String>,
    pub created_at: u64,
    pub updated_at: u64,
}
