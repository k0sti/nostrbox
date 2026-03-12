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

/// An actor in Nostrbox — any pubkey-based identity.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Actor {
    pub pubkey: Pubkey,
    pub kind: ActorKind,
    pub global_role: GlobalRole,
    /// Display name from kind-0 metadata, if known.
    pub display_name: Option<String>,
    /// Groups this actor belongs to (group ids).
    pub groups: Vec<String>,
}
