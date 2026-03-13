pub mod access;
pub mod actor;
pub mod group;
pub mod registration;
pub mod role;
pub mod visibility;

/// Re-export core types for convenience.
pub use access::can_access;
pub use actor::{Actor, ActorKind, ActorStatus};
pub use group::{Group, GroupMember, GroupRole, GroupStatus, JoinPolicy};
pub use registration::{Registration, RegistrationStatus};
pub use role::GlobalRole;
pub use visibility::{Scope, Visibility};

/// A Nostr public key, represented as a hex string.
pub type Pubkey = String;

/// A unique group identifier.
pub type GroupId = String;
