use serde::{Deserialize, Serialize};

/// Minimal Nostr event representation.
///
/// TODO: Replace with types from chosen nostr library crate once selected.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NostrEvent {
    pub id: String,
    pub pubkey: String,
    pub created_at: u64,
    pub kind: u32,
    pub tags: Vec<Vec<String>>,
    pub content: String,
    pub sig: String,
}

/// Well-known event kinds relevant to Nostrbox.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NostrEventKind {
    /// NIP-01 kind 0: user metadata
    Metadata,
    /// Replaceable event for actor role assignment
    /// TODO: Define custom kind number or use existing NIP
    ActorRole,
    /// Replaceable event for registration request
    RegistrationRequest,
    /// Replaceable event for group definition
    GroupDefinition,
    /// Replaceable event for group membership
    GroupMembership,
    /// Other / unknown
    Other(u32),
}

impl NostrEventKind {
    pub fn from_kind(kind: u32) -> Self {
        match kind {
            0 => Self::Metadata,
            // TODO: Assign concrete kind numbers for custom events
            _ => Self::Other(kind),
        }
    }

    pub fn to_kind(&self) -> u32 {
        match self {
            Self::Metadata => 0,
            Self::ActorRole => 30_000, // TODO: placeholder
            Self::RegistrationRequest => 30_001, // TODO: placeholder
            Self::GroupDefinition => 30_002, // TODO: placeholder
            Self::GroupMembership => 30_003, // TODO: placeholder
            Self::Other(k) => *k,
        }
    }
}
